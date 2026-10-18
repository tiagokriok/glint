//! Isolated metadata-only daemon: no hardware modules, devices, or real session.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

const CONFIG: &str = r#"
context.properties = { core.daemon = true core.name = glint-fixture support.dbus = false }
context.spa-libs = { support.* = support/libspa-support }
context.modules = [
  { name = libpipewire-module-protocol-native }
  { name = libpipewire-module-spa-node-factory }
  { name = libpipewire-module-client-node }
  { name = libpipewire-module-metadata }
  { name = libpipewire-module-link-factory }
  { name = libpipewire-module-access args = { access.force = unrestricted } }
]
context.objects = [
  { factory = spa-node-factory args = { factory.name = support.node.driver node.name = fixture-driver } }
]
"#;

struct Daemon {
    child: Child,
    dir: PathBuf,
}
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn isolated_pipewire_registry_callbacks() {
    let dir = PathBuf::from(format!(
        "/tmp/opencode/glint-pw-fixture-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("fixture.conf"), CONFIG).unwrap();
    let log = std::fs::File::create(dir.join("daemon.log")).unwrap();
    let child = Command::new("pipewire")
        .args(["-c", "fixture.conf"])
        .env("PIPEWIRE_CONFIG_DIR", &dir)
        .env("PIPEWIRE_RUNTIME_DIR", &dir)
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()
        .expect("installed pipewire daemon");
    let mut daemon = Daemon { child, dir };
    let deadline = Instant::now() + Duration::from_secs(3);
    while !daemon.dir.join("glint-fixture").exists() {
        assert!(Instant::now() < deadline, "isolated daemon did not start");
        assert!(
            daemon.child.try_wait().unwrap().is_none(),
            "daemon failed: {}",
            std::fs::read_to_string(daemon.dir.join("daemon.log")).unwrap()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let result = Command::new("timeout")
        .arg("8s")
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", "adapter_fixture", "--nocapture"])
        .env("GLINT_PW_FIXTURE_DIR", &daemon.dir)
        .env("PIPEWIRE_RUNTIME_DIR", &daemon.dir)
        .env("PIPEWIRE_REMOTE", "glint-fixture")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn adapter_fixture() {
    let Ok(dir) = std::env::var("GLINT_PW_FIXTURE_DIR") else {
        return;
    };
    assert!(Path::new(&dir).join("glint-fixture").exists());
    std::thread::Builder::new()
        .name("main".into())
        .spawn(native_metadata_projection)
        .unwrap()
        .join()
        .unwrap();
    let (sender, receiver) = mpsc::sync_channel(256);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    // pipewire-rs requires its owning thread to be named main.
    let worker = std::thread::Builder::new()
        .name("main".into())
        .spawn(move || glint_helper::observer::run_pipewire(sender, worker_stop))
        .unwrap();
    let mut initialized = false;
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        let status = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        if status.state == glint_helper::status::State::Unobserved {
            initialized = true;
            break;
        }
    }
    assert!(initialized, "no coherent empty registry snapshot");
    let mut cli = Command::new("pw-cli")
        .args(["-r", "glint-fixture"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    cli.stdin.as_mut().unwrap().write_all(b"create-node spa-node-factory { factory.name=support.node.driver node.name=fixture-extra object.linger=false }\nsync\n").unwrap();
    std::thread::sleep(Duration::from_millis(100));
    cli.stdin.as_mut().unwrap().write_all(b"quit\n").unwrap();
    drop(cli.stdin.take());
    assert!(cli.wait().unwrap().success());
    // Wait for registry/client/node callbacks, then stop without touching real sockets.
    let next = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_ne!(next.state, glint_helper::status::State::Active);
    stop.store(true, Ordering::Relaxed);
    worker.join().unwrap();
}

fn native_metadata_projection() {
    use pipewire as pw;
    use pw::proxy::{Listener, ProxyT};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    pw::init();
    let main_loop = pw::main_loop::MainLoopRc::new(None).unwrap();
    let context = pw::context::ContextRc::new(&main_loop, None).unwrap();
    let core = context
        .connect_rc(Some(pw::properties::properties! {
            "application.name" => "Fixture Call",
            "application.process.binary" => "/fixture/call",
        }))
        .unwrap();
    let registry = core.get_registry_rc().unwrap();
    let seen = Rc::new(Cell::new(0_u8));
    let proxies: Rc<RefCell<Vec<Box<dyn ProxyT>>>> = Rc::default();
    let listeners: Rc<RefCell<Vec<Box<dyn Listener>>>> = Rc::default();
    let registry_weak = registry.downgrade();
    let seen_global = seen.clone();
    let _listener = registry
        .add_listener_local()
        .global(move |obj| {
            let registry = registry_weak.upgrade().unwrap();
            match obj.type_ {
                pw::types::ObjectType::Node => {
                    let node = registry.bind::<pw::node::Node, _>(obj).unwrap();
                    let seen = seen_global.clone();
                    let listener = node
                        .add_listener_local()
                        .info(move |info| {
                            let node = glint_helper::observer::node_from_info(info);
                            if node.node_name == "metadata-valid" {
                                assert_eq!(node.description.as_deref(), Some("Fixture Camera"));
                                assert_eq!(node.nick.as_deref(), Some("Fixture Nick"));
                                assert_eq!(node.video_path.as_deref(), Some("/dev/video42"));
                                assert_eq!(node.device_api.as_deref(), Some("v4l2"));
                                seen.set(seen.get() | 1);
                            } else if node.node_name == "metadata-invalid" {
                                assert_eq!(node.video_path, None);
                                assert_eq!(node.description, None);
                                assert_eq!(node.nick, None);
                                seen.set(seen.get() | 2);
                            }
                        })
                        .register();
                    proxies.borrow_mut().push(Box::new(node));
                    listeners.borrow_mut().push(Box::new(listener));
                }
                pw::types::ObjectType::Client => {
                    let client = registry.bind::<pw::client::Client, _>(obj).unwrap();
                    let seen = seen_global.clone();
                    let listener = client
                        .add_listener_local()
                        .info(move |info| {
                            let owner = glint_helper::observer::client_from_info(info);
                            if owner.name == "Fixture Call" {
                                assert_eq!(owner.binary, "/fixture/call");
                                seen.set(seen.get() | 4);
                            }
                        })
                        .register();
                    proxies.borrow_mut().push(Box::new(client));
                    listeners.borrow_mut().push(Box::new(listener));
                }
                _ => {}
            }
        })
        .register();
    let _valid = core
        .create_object::<pw::node::Node>(
            "spa-node-factory",
            &pw::properties::properties! {
                "factory.name" => "support.node.driver", "node.name" => "metadata-valid",
                "node.description" => "Fixture Camera", "node.nick" => "Fixture Nick",
                "api.v4l2.path" => "/dev/video42", "device.api" => "v4l2",
            },
        )
        .unwrap();
    let _invalid = core
        .create_object::<pw::node::Node>(
            "spa-node-factory",
            &pw::properties::properties! {
                "factory.name" => "support.node.driver", "node.name" => "metadata-invalid",
                "api.v4l2.path" => "/dev/video42/../video0",
            },
        )
        .unwrap();
    let weak = main_loop.downgrade();
    let complete = seen.clone();
    let deadline = Instant::now() + Duration::from_secs(2);
    let timer = main_loop.loop_().add_timer(move |_| {
        if complete.get() == 7 || Instant::now() >= deadline {
            weak.upgrade().unwrap().quit();
        }
    });
    let _ = timer.update_timer(
        Some(Duration::from_millis(10)),
        Some(Duration::from_millis(10)),
    );
    main_loop.run();
    assert_eq!(
        seen.get(),
        7,
        "native node/client metadata callbacks missing"
    );
}
