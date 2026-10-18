//! Pure classification of a normalized PipeWire camera graph.
//!
//! A camera is `active` only when a running video capture stream is actively
//! linked to a producer that is a V4L2 device. Hardware presence, suspended
//! sources, playback, portal screen sources, and unlinked nodes are never
//! active; incomplete metadata is `unknown`.

use std::collections::BTreeMap;

use crate::attribution::{
    ApplicationSource, EvidenceKind, Observation, normalize_label, project, valid_path,
};
use crate::status::{State, Status};

pub const EV_CAPTURE: &str = "Camera capture observed";
pub const EV_UNOBSERVED: &str = "No capture observed on supported paths";
pub const EV_UNCLASSIFIED: &str = "Capture state could not be classified";
pub const EV_PIPEWIRE_DOWN: &str = "PipeWire graph unavailable";
pub const EV_INITIALIZING: &str = "PipeWire graph initializing";

pub const LIM_PIPEWIRE: &str = "Only PipeWire-visible streams are observed";
pub const LIM_BROWSER: &str = "Internal browser-tab capture is not supported";
pub const LIM_OTHER_USERS: &str = "Other-user and restricted processes are not observed";
pub const CAMERA_LIMITATIONS: [&str; 3] = [LIM_PIPEWIRE, LIM_BROWSER, LIM_OTHER_USERS];

/// Normalized state of a PipeWire node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Running,
    Suspended,
    Idle,
    Creating,
    Error,
    Unknown,
}

/// PipeWire-reported identity, not verified process or end-user app identity.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphOwner {
    pub binary: String,
    pub name: String,
}

impl GraphOwner {
    pub fn new(binary: &str, name: &str) -> GraphOwner {
        GraphOwner {
            binary: binary.to_string(),
            name: name.to_string(),
        }
    }
}

/// One normalized PipeWire node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    pub id: u32,
    pub media_class: String,
    pub state: NodeState,
    pub device_api: Option<String>,
    pub node_name: String,
    pub monitor: bool,
    pub client_id: Option<u32>,
    pub owner: GraphOwner,
    pub description: Option<String>,
    pub nick: Option<String>,
    pub video_path: Option<String>,
}

impl GraphNode {
    pub fn new(id: u32, media_class: &str, state: NodeState) -> GraphNode {
        GraphNode {
            id,
            media_class: media_class.to_string(),
            state,
            device_api: None,
            node_name: String::new(),
            monitor: false,
            client_id: None,
            owner: GraphOwner::default(),
            description: None,
            nick: None,
            video_path: None,
        }
    }

    #[must_use]
    pub fn with_device_api(mut self, api: &str) -> Self {
        self.device_api = Some(api.to_string());
        self
    }

    #[must_use]
    pub fn with_owner(mut self, binary: &str, name: &str) -> Self {
        self.owner = GraphOwner::new(binary, name);
        self
    }

    #[must_use]
    pub fn with_client(mut self, client_id: u32) -> Self {
        self.client_id = Some(client_id);
        self
    }

    #[must_use]
    pub fn with_node_name(mut self, name: &str) -> Self {
        self.node_name = name.to_string();
        self
    }

    #[must_use]
    pub fn with_monitor(mut self, monitor: bool) -> Self {
        self.monitor = monitor;
        self
    }
}

/// One active-link edge: `output_node` feeds `input_node`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphLink {
    pub active: bool,
    pub id: u32,
    pub output_node: u32,
    pub input_node: u32,
}

impl GraphLink {
    pub fn new(id: u32, output_node: u32, input_node: u32) -> GraphLink {
        GraphLink {
            active: true,
            id,
            output_node,
            input_node,
        }
    }
}

/// A normalized, coherent snapshot of the interesting part of the graph.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CameraGraph {
    pub available: bool,
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
    pub clients: BTreeMap<u32, GraphOwner>,
}

impl CameraGraph {
    pub fn new(available: bool) -> CameraGraph {
        CameraGraph {
            available,
            ..CameraGraph::default()
        }
    }

    #[must_use]
    pub fn node(mut self, node: GraphNode) -> Self {
        self.nodes.push(node);
        self
    }

    #[must_use]
    pub fn link(mut self, link: GraphLink) -> Self {
        self.links.push(link);
        self
    }

    #[must_use]
    pub fn client(mut self, id: u32, owner: GraphOwner) -> Self {
        self.clients.insert(id, owner);
        self
    }
}

/// Classify a camera graph snapshot into a single camera `Status`.
///
/// Only a `running` `Stream/Input/Video` linked to a `v4l2` producer is
/// `active`. Screen sources, playback, suspended nodes, and hardware presence
/// are not active; an unresolvable producer is `unknown`, never `active`.
pub fn classify(graph: &CameraGraph) -> Status {
    if !graph.available {
        return Status::new(State::Unknown, EV_PIPEWIRE_DOWN, &CAMERA_LIMITATIONS);
    }

    let mut has_camera = false;
    let mut ambiguous = false;
    let mut observations = Vec::new();

    for stream in &graph.nodes {
        if stream.media_class != CAPTURE_MEDIA_CLASS || stream.state != NodeState::Running {
            continue;
        }

        let mut producers = graph
            .links
            .iter()
            .filter(|link| link.active && link.input_node == stream.id)
            .filter_map(|link| graph.nodes.iter().find(|node| node.id == link.output_node))
            .peekable();

        if producers.peek().is_none() {
            // A running capture stream we cannot trace to a source: not inactive.
            ambiguous = true;
            continue;
        }

        for producer in producers {
            match classify_video(producer, &graph.clients) {
                VideoVerdict::Camera => {
                    has_camera = true;
                    observations.push(capture_observation(stream, producer, &graph.clients));
                }
                VideoVerdict::Screen => {}
                VideoVerdict::Ambiguous => ambiguous = true,
            }
        }
    }

    if has_camera {
        Status {
            details: Some(project(&observations)),
            ..Status::new(State::Active, EV_CAPTURE, &CAMERA_LIMITATIONS)
        }
    } else if ambiguous {
        Status::new(State::Unknown, EV_UNCLASSIFIED, &CAMERA_LIMITATIONS)
    } else {
        Status::new(State::Unobserved, EV_UNOBSERVED, &CAMERA_LIMITATIONS)
    }
}

fn capture_observation(
    stream: &GraphNode,
    producer: &GraphNode,
    clients: &BTreeMap<u32, GraphOwner>,
) -> Observation {
    // A removed referenced client is unknown; do not resurrect cached node identity.
    let owner = match stream.client_id {
        Some(id) => clients.get(&id),
        None => Some(&stream.owner),
    };
    let application = owner.and_then(|owner| {
        normalize_label(&owner.name).or_else(|| {
            std::path::Path::new(&owner.binary)
                .file_name()?
                .to_str()
                .and_then(normalize_label)
        })
    });
    Observation {
        application_source: if application.is_some() {
            ApplicationSource::PipewireReported
        } else {
            ApplicationSource::Unknown
        },
        application,
        device: producer
            .description
            .as_deref()
            .and_then(normalize_label)
            .or_else(|| producer.nick.as_deref().and_then(normalize_label)),
        device_path: producer
            .video_path
            .as_ref()
            .filter(|path| valid_path(path))
            .cloned(),
        evidence: EvidenceKind::PipewireCapture,
    }
}

/// Media class of a capture input stream; playback output streams are ignored.
const CAPTURE_MEDIA_CLASS: &str = "Stream/Input/Video";

enum VideoVerdict {
    Camera,
    Screen,
    Ambiguous,
}

fn classify_video(producer: &GraphNode, clients: &BTreeMap<u32, GraphOwner>) -> VideoVerdict {
    if producer.state != NodeState::Running {
        return VideoVerdict::Ambiguous;
    }
    if producer.device_api.as_deref() == Some("v4l2") {
        return VideoVerdict::Camera;
    }

    let owner = producer
        .client_id
        .and_then(|id| clients.get(&id))
        .unwrap_or(&producer.owner);
    let binary = owner.binary.to_lowercase();
    let name = owner.name.to_lowercase();
    if binary.starts_with("xdg-desktop-portal") || name.starts_with("xdg-desktop-portal") {
        VideoVerdict::Screen
    } else {
        VideoVerdict::Ambiguous
    }
}
