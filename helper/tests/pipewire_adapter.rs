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
