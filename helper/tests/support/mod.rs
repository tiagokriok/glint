use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Fixture(pub PathBuf);

impl Fixture {
    pub fn new() -> Self {
        let path = PathBuf::from(format!(
            "/tmp/opencode/glint-attribution-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn device(&self, name: &str, target: &str) {
        fs::create_dir_all(self.0.join("dev")).unwrap();
        symlink(target, self.0.join("dev").join(name)).unwrap();
        let sys = self.0.join("sys/class/video4linux").join(name);
        fs::create_dir_all(&sys).unwrap();
        fs::write(sys.join("name"), " Fixture Camera\n").unwrap();
    }

    pub fn holder(&self, pid: u32, name: &str, targets: &[&str]) {
        let root = self.0.join(format!("proc/{pid}"));
        fs::create_dir_all(root.join("fd")).unwrap();
        fs::write(root.join("stat"), stat(pid, 100)).unwrap();
        fs::write(root.join("comm"), name).unwrap();
        for (fd, target) in targets.iter().enumerate() {
            symlink(target, root.join(format!("fd/{fd}"))).unwrap();
        }
    }
}

pub fn stat(pid: u32, start: u64) -> String {
    format!(
        "{pid} (odd ) process (name) S {} {start} 0\n",
        vec!["0"; 18].join(" ")
    )
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
