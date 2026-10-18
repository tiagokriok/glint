//! Bounded metadata-only I/O. Device and FD paths are statted, never opened.

use std::fs::{self, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::attribution::{DeviceIdentity, normalize_label};
use crate::device_access::is_video_device;

pub const JOB_BUDGET: Duration = Duration::from_millis(100);
pub const MAX_FILE_BYTES: usize = 4096;
pub const MAX_FDS: usize = 256;

pub struct FdEntries {
    pub paths: Vec<PathBuf>,
    pub truncated: bool,
}

/// Injection seam for deterministic races, permission failures and clocks.
/// Implementations must preserve the byte/entry limits and check the deadline
/// between operations. Individual local filesystem calls have no hard timeout.
pub trait MetadataIo {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn read_text(&self, path: &Path, deadline: Instant) -> io::Result<String>;

    fn character_device(&self, path: &Path, deadline: Instant) -> io::Result<Option<u64>>;

    fn fd_entries(&self, path: &Path, deadline: Instant) -> io::Result<FdEntries> {
        check_deadline(self.now(), deadline)?;
        let mut entries = fs::read_dir(path)?;
        let mut paths = Vec::new();
        for _ in 0..MAX_FDS {
            check_deadline(self.now(), deadline)?;
            let Some(entry) = entries.next() else {
                return Ok(FdEntries {
                    paths,
                    truncated: false,
                });
            };
            let entry = entry?;
            if entry.file_name().to_str().is_some_and(|name| {
                !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit())
            }) {
                paths.push(entry.path());
            }
        }
        // Do not consume a 257th entry even to check whether one exists.
        Ok(FdEntries {
            paths,
            truncated: true,
        })
    }
}

pub struct FsMetadataIo;

impl MetadataIo for FsMetadataIo {
    fn read_text(&self, path: &Path, deadline: Instant) -> io::Result<String> {
        check_deadline(self.now(), deadline)?;
        if !fs::symlink_metadata(path)?.is_file() {
            return Err(io::Error::other("not a metadata file"));
        }
        check_deadline(self.now(), deadline)?;
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)?;
        check_deadline(self.now(), deadline)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::other("not a metadata file"));
        }
        let mut buffer = [0; MAX_FILE_BYTES];
        let mut used = 0;
        loop {
            check_deadline(self.now(), deadline)?;
            let count = file.read(&mut buffer[used..])?;
            if count == 0 {
                break;
            }
            used += count;
            if used == MAX_FILE_BYTES {
                return Err(io::Error::other("metadata limit reached"));
            }
        }
        String::from_utf8(buffer[..used].to_vec())
            .map_err(|_| io::Error::other("invalid metadata encoding"))
    }

    fn character_device(&self, path: &Path, deadline: Instant) -> io::Result<Option<u64>> {
        check_deadline(self.now(), deadline)?;
        let metadata = fs::metadata(path)?;
        Ok(metadata
            .file_type()
            .is_char_device()
            .then(|| metadata.rdev()))
    }
}

fn check_deadline(now: Instant, deadline: Instant) -> io::Result<()> {
    if now >= deadline {
        Err(io::Error::from(io::ErrorKind::TimedOut))
    } else {
        Ok(())
    }
}

pub fn read_device(sys_root: &Path, dev_root: &Path, video_name: &str) -> Option<DeviceIdentity> {
    read_device_with(
        &FsMetadataIo,
        sys_root,
        dev_root,
        video_name,
        Instant::now() + JOB_BUDGET,
    )
}

pub fn read_device_with(
    io: &dyn MetadataIo,
    sys_root: &Path,
    dev_root: &Path,
    video_name: &str,
    deadline: Instant,
) -> Option<DeviceIdentity> {
    let path = format!("/dev/{video_name}");
    if path.len() > 64 || !is_video_device(video_name) || io.now() >= deadline {
        return None;
    }
    let device_id = io
        .character_device(&dev_root.join(video_name), deadline)
        .ok()??;
    if io.now() >= deadline {
        return None;
    }
    let name = io
        .read_text(
            &sys_root
                .join("class/video4linux")
                .join(video_name)
                .join("name"),
            deadline,
        )
        .ok()
        .as_deref()
        .and_then(normalize_label);
    (io.now() < deadline).then_some(DeviceIdentity {
        path,
        device_id,
        name,
    })
}
