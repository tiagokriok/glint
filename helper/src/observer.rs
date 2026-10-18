//! Coherent PipeWire graph snapshots and the event-driven subscription runtime.
//!
//! The pure [`GraphStore`] folds registry updates into a normalized camera
//! graph. [`run_pipewire`] is the real event-driven adapter (registry +
//! listeners), mirroring `pipewire-rs` `pw-mon`; it is not exercised by the
//! fixture tests, which never require a live graph.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender as Sender;
use std::time::{Duration, Instant};

use pipewire as pw;
use pw::proxy::{Listener, ProxyT};

use crate::attribution_worker::AttributionWorker;
use crate::camera::{CameraGraph, GraphLink, GraphNode, GraphOwner, NodeState as CameraNodeState};
use crate::device_metadata::FsMetadataIo;
use crate::runtime::{PipewirePublisher, enrich_pipewire_with};
use crate::status::{State, Status};
use std::path::Path;

/// Folds PipeWire registry updates into a coherent camera graph.
#[derive(Debug, Default, Clone)]
pub struct GraphStore {
    connected: bool,
    synced: bool,
    nodes: BTreeMap<u32, GraphNode>,
    links: BTreeMap<u32, GraphLink>,
    clients: BTreeMap<u32, GraphOwner>,
}

impl GraphStore {
    pub fn new() -> GraphStore {
        GraphStore::default()
    }

    /// Start a fresh connection: stale state must never survive a reconnect.
    pub fn connect(&mut self) {
        *self = GraphStore {
            connected: true,
            ..GraphStore::default()
        };
    }

    /// Observer disconnect: drop all state and report unavailable.
    pub fn disconnect(&mut self) {
        *self = GraphStore::default();
    }

    /// Initial enumeration finished; the graph is now coherent.
    pub fn mark_synced(&mut self) {
        self.synced = true;
    }

    pub fn upsert_node(&mut self, node: GraphNode) {
        self.nodes.insert(node.id, node);
    }

    pub fn apply_node_update(&mut self, node: GraphNode, state_changed: bool, props_changed: bool) {
        let previous = self
            .nodes
            .get(&node.id)
            .cloned()
            .unwrap_or_else(|| GraphNode::new(node.id, "", CameraNodeState::Unknown));
        let next_state = if state_changed {
            node.state
        } else {
            previous.state
        };
        let properties = if props_changed { node } else { previous };
        self.upsert_node(GraphNode {
            state: next_state,
            ..properties
        });
    }

    pub fn remove_node(&mut self, id: u32) {
        self.nodes.remove(&id);
    }

    pub fn upsert_link(&mut self, link: GraphLink) {
        self.links.insert(link.id, link);
    }

    pub fn remove_link(&mut self, id: u32) {
        self.links.remove(&id);
    }

    pub fn upsert_client(&mut self, id: u32, owner: GraphOwner) {
        self.clients.insert(id, owner);
    }

    pub fn apply_client_update(&mut self, id: u32, owner: GraphOwner, props_changed: bool) {
        if props_changed {
            self.upsert_client(id, owner);
        }
    }

    pub fn remove_client(&mut self, id: u32) {
        self.clients.remove(&id);
    }

    pub fn graph(&self) -> CameraGraph {
        CameraGraph {
            available: true,
            nodes: self.nodes.values().cloned().collect(),
            links: self.links.values().copied().collect(),
            clients: self.clients.clone(),
        }
    }

    /// Classify the current graph. A partial (unsynced) graph is `unknown`,
    /// never a claim of inactivity.
    pub fn classify(&self) -> Status {
        if !self.connected {
            return unavailable_status();
        }
        if !self.synced {
            return initializing_status();
        }
        crate::camera::classify(&self.graph())
    }
}

/// The PipeWire path could not be observed.
pub fn unavailable_status() -> Status {
    Status::new(
        State::Unknown,
        crate::camera::EV_PIPEWIRE_DOWN,
        &crate::camera::CAMERA_LIMITATIONS,
    )
}

/// The initial registry enumeration has not completed yet.
pub fn initializing_status() -> Status {
    Status::new(
        State::Unknown,
        crate::camera::EV_INITIALIZING,
        &crate::camera::CAMERA_LIMITATIONS,
    )
}

/// Subscribe to PipeWire graph changes until `stop` is set.
///
/// Reconnects with bounded backoff; a drop clears stale graph state so a
/// reconnect never reuses prior positive evidence.
pub fn run_pipewire(sink: Sender<Status>, stop: Arc<AtomicBool>) {
    pw::init();
    let sink = Rc::new(RefCell::new(PipewirePublisher::new(
        sink,
        AttributionWorker::new(|job| {
            enrich_pipewire_with(&FsMetadataIo, Path::new("/sys"), Path::new("/dev"), &job)
        })
        .ok(),
    )));
    let mut backoff = Duration::from_millis(250);
    while !stop.load(Ordering::Relaxed) {
        if let Err(error) = pipewire_session(&sink, &stop) {
            eprintln!("glint-helper: pipewire session error: {error}");
        }
        if stop.load(Ordering::Relaxed) {
            break;
        }
        publish(&sink, &unavailable_status());
        sleep_interruptible(backoff, &stop);
        backoff = (backoff * 2).min(Duration::from_secs(4));
    }
    // SAFETY: paired with `pw::init`; no PipeWire objects remain alive here.
    unsafe { pw::deinit() };
}

fn pipewire_session(sink: &Rc<Publisher>, stop: &Arc<AtomicBool>) -> Result<(), pw::Error> {
    let main_loop = pw::main_loop::MainLoopRc::new(None)?;

    // Programmatic stop fallback; signals below also quit the loop.
    let weak = main_loop.downgrade();
    let stop_timer = stop.clone();
    let sink_timer = sink.clone();
    let timer = main_loop.loop_().add_timer(move |_| {
        sink_timer.borrow_mut().poll();
        if stop_timer.load(Ordering::Relaxed)
            && let Some(main_loop) = weak.upgrade()
        {
            main_loop.quit();
        }
    });
    let _ = timer.update_timer(
        Some(Duration::from_millis(250)),
        Some(Duration::from_millis(250)),
    );

    let weak = main_loop.downgrade();
    let stop_term = stop.clone();
    let _sig_term = main_loop
        .loop_()
        .add_signal_local(pw::loop_::Signal::TERM, move || {
            stop_term.store(true, Ordering::Relaxed);
            if let Some(main_loop) = weak.upgrade() {
                main_loop.quit();
            }
        });
    let weak = main_loop.downgrade();
    let stop_int = stop.clone();
    let _sig_int = main_loop
        .loop_()
        .add_signal_local(pw::loop_::Signal::INT, move || {
            stop_int.store(true, Ordering::Relaxed);
            if let Some(main_loop) = weak.upgrade() {
                main_loop.quit();
            }
        });

    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;

    let store = Rc::new(RefCell::new(GraphStore::new()));
    store.borrow_mut().connect();
    publish(sink, &initializing_status());

    let synced = Rc::new(Cell::new(false));
    let pending = Rc::new(Cell::new(None));
    let pending_done = pending.clone();
    let core_done = core.downgrade();
    let store_sync = store.clone();
    let sink_sync = sink.clone();
    let weak_core = main_loop.downgrade();
    let _core_listener = core
        .add_listener_local()
        .done(move |id, seq| {
            if id != 0 || pending_done.get() != Some(seq) {
                return;
            }
            if !synced.replace(true) {
                // Registry enumeration queued bind requests; wait for their info too.
                if let Some(core) = core_done.upgrade() {
                    pending_done.set(core.sync(0).ok());
                }
            } else {
                let mut state = store_sync.borrow_mut();
                state.mark_synced();
                let status = state.classify();
                drop(state);
                publish(&sink_sync, &status);
            }
        })
        .error(move |id, _seq, _res, message| {
            eprintln!("glint-helper: pipewire core error id={id}: {message}");
            if let Some(main_loop) = weak_core.upgrade() {
                main_loop.quit();
            }
        })
        .register();

    let registry = core.get_registry_rc()?;
    let proxies = Rc::new(RefCell::new(Proxies::new()));

    let store_global = store.clone();
    let sink_global = sink.clone();
    let proxies_global = proxies.clone();
    let weak_registry = registry.downgrade();

    let store_remove = store.clone();
    let sink_remove = sink.clone();
    let proxies_remove = proxies.clone();

    let _registry_listener = registry
        .add_listener_local()
        .global(move |obj| {
            let Some(registry) = weak_registry.upgrade() else {
                return;
            };
            match obj.type_ {
                pw::types::ObjectType::Node => {
                    let Ok(node) = registry.bind::<pw::node::Node, _>(obj) else {
                        return;
                    };
                    let store_info = store_global.clone();
                    let sink_info = sink_global.clone();
                    let listener = node
                        .add_listener_local()
                        .info(move |info| {
                            let mut state = store_info.borrow_mut();
                            state.apply_node_update(
                                node_from_info(info),
                                info.change_mask().contains(pw::node::NodeChangeMask::STATE),
                                info.change_mask().contains(pw::node::NodeChangeMask::PROPS),
                            );
                            let status = state.classify();
                            drop(state);
                            publish(&sink_info, &status);
                        })
                        .register();
                    let proxy: Box<dyn ProxyT> = Box::new(node);
                    let id = obj.id;
                    proxies_global
                        .borrow_mut()
                        .insert(id, proxy, Box::new(listener));
                }
                pw::types::ObjectType::Link => {
                    let Ok(link) = registry.bind::<pw::link::Link, _>(obj) else {
                        return;
                    };
                    let store_info = store_global.clone();
                    let sink_info = sink_global.clone();
                    let listener = link
                        .add_listener_local()
                        .info(move |info| {
                            let mut state = store_info.borrow_mut();
                            state.upsert_link(GraphLink {
                                active: matches!(info.state(), pw::link::LinkState::Active),
                                ..GraphLink::new(
                                    info.id(),
                                    info.output_node_id(),
                                    info.input_node_id(),
                                )
                            });
                            let status = state.classify();
                            drop(state);
                            publish(&sink_info, &status);
                        })
                        .register();
                    let proxy: Box<dyn ProxyT> = Box::new(link);
                    let id = obj.id;
                    proxies_global
                        .borrow_mut()
                        .insert(id, proxy, Box::new(listener));
                }
                pw::types::ObjectType::Client => {
                    let Ok(client) = registry.bind::<pw::client::Client, _>(obj) else {
                        return;
                    };
                    let store_info = store_global.clone();
                    let sink_info = sink_global.clone();
                    let listener = client
                        .add_listener_local()
                        .info(move |info| {
                            let mut state = store_info.borrow_mut();
                            state.apply_client_update(
                                info.id(),
                                client_from_info(info),
                                info.change_mask()
                                    .contains(pw::client::ClientChangeMask::PROPS),
                            );
                            let status = state.classify();
                            drop(state);
                            publish(&sink_info, &status);
                        })
                        .register();
                    let proxy: Box<dyn ProxyT> = Box::new(client);
                    let id = obj.id;
                    proxies_global
                        .borrow_mut()
                        .insert(id, proxy, Box::new(listener));
                }
                _ => {}
            }
        })
        .global_remove(move |id| {
            let mut state = store_remove.borrow_mut();
            state.remove_node(id);
            state.remove_link(id);
            state.remove_client(id);
            let status = state.classify();
            drop(state);
            publish(&sink_remove, &status);
            proxies_remove.borrow_mut().remove(id);
        })
        .register();

    pending.set(Some(core.sync(0)?));
    main_loop.run();
    Ok(())
}

struct Proxies {
    proxies: HashMap<u32, Box<dyn ProxyT>>,
    listeners: HashMap<u32, Vec<Box<dyn Listener>>>,
}

impl Proxies {
    fn new() -> Proxies {
        Proxies {
            proxies: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    fn insert(&mut self, id: u32, proxy: Box<dyn ProxyT>, listener: Box<dyn Listener>) {
        self.proxies.insert(id, proxy);
        self.listeners.entry(id).or_default().push(listener);
    }

    fn remove(&mut self, id: u32) {
        self.proxies.remove(&id);
        self.listeners.remove(&id);
    }
}

type Publisher = RefCell<PipewirePublisher<AttributionWorker>>;

fn publish(sink: &Publisher, status: &Status) {
    sink.borrow_mut().publish(status.clone());
}

fn sleep_interruptible(total: Duration, stop: &AtomicBool) {
    let deadline = Instant::now() + total;
    while Instant::now() < deadline {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Project native node metadata without filesystem access. Masks are applied by GraphStore.
pub fn node_from_info(info: &pw::node::NodeInfoRef) -> GraphNode {
    let props = info.props();
    GraphNode {
        id: info.id(),
        media_class: dict_get(props, "media.class"),
        state: map_node_state(info.state()),
        device_api: nonempty(dict_get(props, "device.api")),
        node_name: dict_get(props, "node.name"),
        monitor: dict_get(props, "monitor") == "true",
        client_id: dict_get(props, "client.id").parse::<u32>().ok(),
        owner: GraphOwner::new(
            &dict_get(props, "application.process.binary"),
            &dict_get(props, "application.name"),
        ),
        description: nonempty(dict_get(props, "node.description")),
        nick: nonempty(dict_get(props, "node.nick")),
        video_path: nonempty(dict_get(props, "api.v4l2.path"))
            .filter(|path| crate::attribution::valid_path(path)),
    }
}

/// Project only the reported application name and executable label.
pub fn client_from_info(info: &pw::client::ClientInfoRef) -> GraphOwner {
    let props = info.props();
    GraphOwner::new(
        &dict_get(props, "application.process.binary"),
        &dict_get(props, "application.name"),
    )
}

fn dict_get(props: Option<&pw::spa::utils::dict::DictRef>, key: &str) -> String {
    props
        .and_then(|props| props.get(key))
        .unwrap_or_default()
        .to_string()
}

fn nonempty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn map_node_state(state: pw::node::NodeState<'_>) -> CameraNodeState {
    match state {
        pw::node::NodeState::Running => CameraNodeState::Running,
        pw::node::NodeState::Suspended => CameraNodeState::Suspended,
        pw::node::NodeState::Idle => CameraNodeState::Idle,
        pw::node::NodeState::Creating => CameraNodeState::Creating,
        pw::node::NodeState::Error(_) => CameraNodeState::Error,
    }
}
