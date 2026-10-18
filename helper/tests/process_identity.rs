mod support;

use std::cell::Cell;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

use glint_helper::attribution::{ApplicationSource, DeviceIdentity};
use glint_helper::device_metadata::{FsMetadataIo, MetadataIo, read_device};
use glint_helper::process_identity::{observe_holder, observe_holder_with, parse_start_time};
use support::Fixture;

fn devices(f: &Fixture) -> Vec<DeviceIdentity> {
    ["video0", "video1"]
        .iter()
        .filter_map(|name| read_device(&f.0.join("sys"), &f.0.join("dev"), name))
        .collect()
}

fn observe(f: &Fixture, pid: u32) -> glint_helper::process_identity::HolderAttribution {
    observe_holder(
        &f.0.join("proc"),
        pid,
        &devices(f),
        Instant::now() + Duration::from_secs(1),
    )
}

#[test]
fn parses_field_22_after_the_final_parenthesis_and_checks_pid() {
    assert_eq!(parse_start_time(&support::stat(42, 999), 42), Some(999));
    assert_eq!(parse_start_time(&support::stat(43, 999), 42), None);
    for invalid in ["", "42 (truncated)", "42 no parentheses", "42 (x) S x"] {
        assert_eq!(parse_start_time(invalid, 42), None);
    }
}

#[test]
fn device_metadata_is_character_only_logical_and_sanitized() {
    let f = Fixture::new();
    f.device("video0", "/dev/null");
    let d = read_device(&f.0.join("sys"), &f.0.join("dev"), "video0").unwrap();
    assert_eq!(d.path, "/dev/video0");
    assert_eq!(d.name.as_deref(), Some("Fixture Camera"));
    assert_ne!(d.device_id, 0);
    fs::write(f.0.join("dev/video1"), "regular file").unwrap();
    for name in [
        "video1",
        "../video0",
        "video",
        "video0x",
        &format!("video{}", "1".repeat(60)),
    ] {
        assert!(read_device(&f.0.join("sys"), &f.0.join("dev"), name).is_none());
    }
    fs::write(
        f.0.join("sys/class/video4linux/video0/name"),
        "a".repeat(4097),
    )
    .unwrap();
    assert_eq!(devices(&f)[0].name, None);
    fs::remove_file(f.0.join("sys/class/video4linux/video0/name")).unwrap();
    assert_eq!(devices(&f)[0].name, None);
}

#[test]
fn holder_matches_only_current_character_fds_and_can_hold_two_devices() {
    let f = Fixture::new();
    f.device("video0", "/dev/null");
    f.device("video1", "/dev/zero");
    f.holder(42, " Browser\u{202e}\n", &["/dev/null"]);
    f.holder(43, "Meeting", &["/dev/zero"]);
    let a = observe(&f, 42);
    let b = observe(&f, 43);
    assert_eq!(a.observations.len(), 1);
    assert_eq!(a.observations[0].application.as_deref(), Some("Browser"));
    assert_eq!(
        a.observations[0].device_path.as_deref(),
        Some("/dev/video0")
    );
    assert_eq!(
        b.observations[0].device_path.as_deref(),
        Some("/dev/video1")
    );
    std::os::unix::fs::symlink("/dev/zero", f.0.join("proc/42/fd/3")).unwrap();
    assert_eq!(observe(&f, 42).observations.len(), 2);
}

#[test]
fn no_matching_fd_never_labels_a_reused_pid_even_with_stable_stat() {
    let f = Fixture::new();
    f.device("video0", "/dev/null");
    f.holder(42, "Replacement", &["/dev/zero"]);
    let result = observe(&f, 42);
    assert_eq!(result.observations.len(), 1);
    assert_eq!(
        result.observations[0].application_source,
        ApplicationSource::Unknown
    );
    assert!(result.observations[0].application.is_none());
    assert!(result.observations[0].device_path.is_none());
}

#[test]
fn failed_comm_or_stat_preserves_only_verified_device() {
    for missing in ["comm", "stat"] {
        let f = Fixture::new();
        f.device("video0", "/dev/null");
        f.holder(42, "Browser", &["/dev/null"]);
        fs::remove_file(f.0.join(format!("proc/42/{missing}"))).unwrap();
        let result = observe(&f, 42);
        assert_eq!(
            result.observations[0].device_path.as_deref(),
            Some("/dev/video0")
        );
        assert!(result.observations[0].application.is_none());
        assert_eq!(
            result.observations[0].application_source,
            ApplicationSource::Unknown
        );
    }
}

struct ChangingIo {
    stat_reads: Cell<usize>,
    expire: Cell<bool>,
    mode: &'static str,
}

#[test]
fn io_never_consults_another_metadata_path_after_an_operation_exhausts_the_budget() {
    struct ExpiringIo {
        expired: Cell<bool>,
        calls: Cell<usize>,
    }
    impl MetadataIo for ExpiringIo {
        fn now(&self) -> Instant {
            Instant::now()
                + if self.expired.get() {
                    Duration::from_secs(1)
                } else {
                    Duration::ZERO
                }
        }
        fn read_text(&self, _: &Path, _: Instant) -> io::Result<String> {
            self.calls.set(self.calls.get() + 1);
            assert!(!self.expired.get());
            Ok("Camera".into())
        }
        fn character_device(&self, _: &Path, _: Instant) -> io::Result<Option<u64>> {
            self.calls.set(self.calls.get() + 1);
            self.expired.set(true);
            Ok(Some(1))
        }
    }
    let io = ExpiringIo {
        expired: Cell::new(false),
        calls: Cell::new(0),
    };
    let d = glint_helper::device_metadata::read_device_with(
        &io,
        Path::new("/unused"),
        Path::new("/unused"),
        "video0",
        Instant::now() + Duration::from_millis(100),
    );
    assert!(d.is_none());
    assert_eq!(io.calls.get(), 1);
}

#[test]
fn fd_scan_stops_at_256_even_when_an_injected_reader_returns_more() {
    struct ExcessIo {
        metadata_calls: Cell<usize>,
    }
    impl MetadataIo for ExcessIo {
        fn read_text(&self, path: &Path, _: Instant) -> io::Result<String> {
            Ok(if path.ends_with("stat") {
                support::stat(42, 1)
            } else {
                "Browser".into()
            })
        }
        fn character_device(&self, _: &Path, _: Instant) -> io::Result<Option<u64>> {
            self.metadata_calls.set(self.metadata_calls.get() + 1);
            Ok(Some(1))
        }
        fn fd_entries(
            &self,
            _: &Path,
            _: Instant,
        ) -> io::Result<glint_helper::device_metadata::FdEntries> {
            Ok(glint_helper::device_metadata::FdEntries {
                paths: (0..300).map(|n| format!("/unused/{n}").into()).collect(),
                truncated: false,
            })
        }
    }
    let io = ExcessIo {
        metadata_calls: Cell::new(0),
    };
    let d = DeviceIdentity {
        path: "/dev/video0".into(),
        device_id: 1,
        name: None,
    };
    let result = observe_holder_with(
        &io,
        Path::new("/unused"),
        42,
        &[d],
        Instant::now() + Duration::from_secs(1),
    );
    assert_eq!(io.metadata_calls.get(), 256);
    assert!(result.truncated);
}

impl MetadataIo for ChangingIo {
    fn now(&self) -> Instant {
        Instant::now()
            + if self.expire.get() {
                Duration::from_secs(10)
            } else {
                Duration::ZERO
            }
    }

    fn read_text(&self, path: &Path, deadline: Instant) -> io::Result<String> {
        if path.ends_with("stat") {
            let count = self.stat_reads.get();
            self.stat_reads.set(count + 1);
            if self.mode == "reuse" && count > 0 {
                return Ok(support::stat(42, 200));
            }
            if self.mode == "expire" {
                self.expire.set(true);
            }
        }
        FsMetadataIo.read_text(path, deadline)
    }

    fn character_device(&self, path: &Path, deadline: Instant) -> io::Result<Option<u64>> {
        if self.mode == "vanish" || self.mode == "denied" {
            return Err(io::Error::from(if self.mode == "vanish" {
                io::ErrorKind::NotFound
            } else {
                io::ErrorKind::PermissionDenied
            }));
        }
        FsMetadataIo.character_device(path, deadline)
    }
}

#[test]
fn deterministic_pid_replacement_fd_races_permissions_and_deadlines_are_unknown() {
    for mode in ["reuse", "vanish", "denied", "expire"] {
        let f = Fixture::new();
        f.device("video0", "/dev/null");
        f.holder(42, "Browser", &["/dev/null"]);
        let io = ChangingIo {
            stat_reads: Cell::new(0),
            expire: Cell::new(false),
            mode,
        };
        let result = observe_holder_with(
            &io,
            &f.0.join("proc"),
            42,
            &devices(&f),
            Instant::now() + Duration::from_secs(1),
        );
        assert!(
            result
                .observations
                .iter()
                .all(|row| row.application.is_none()),
            "{mode}"
        );
        if mode != "reuse" {
            assert!(
                result
                    .observations
                    .iter()
                    .all(|row| row.device_path.is_none())
            );
        }
    }
}

#[test]
fn fd_scan_and_metadata_files_are_bounded() {
    let f = Fixture::new();
    f.device("video0", "/dev/null");
    f.holder(42, &"x".repeat(4097), &["/dev/null"]);
    for n in 1..300 {
        std::os::unix::fs::symlink("/dev/null", f.0.join(format!("proc/42/fd/{n}"))).unwrap();
    }
    let result = observe(&f, 42);
    assert!(result.truncated);
    assert!(
        result
            .observations
            .iter()
            .all(|row| row.application.is_none())
    );
    let expired = observe_holder(&f.0.join("proc"), 42, &devices(&f), Instant::now());
    assert!(expired.truncated);
    assert!(expired.observations[0].application.is_none());
}
