mod support;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use glint_helper::attribution::{ApplicationSource, CameraDetails, EvidenceKind, Observation};
use glint_helper::attribution_worker::{
    AttributionJob, AttributionResult, AttributionSession, AttributionWorker, enrich_job,
};
use glint_helper::device_access::DeviceReconciler;
use glint_helper::runtime::StatusStream;
use glint_helper::status::State;

fn details(name: &str) -> CameraDetails {
    CameraDetails {
        observations: vec![Observation {
            application: Some(name.into()),
            application_source: ApplicationSource::Process,
            device: None,
            device_path: Some("/dev/video0".into()),
            evidence: EvidenceKind::DeviceAccess,
        }],
        truncated: false,
    }
}

fn job(generation: u64) -> AttributionJob {
    AttributionJob {
        generation,
        holders: BTreeSet::from([42]),
        devices: vec![PathBuf::from("/dev/video0")],
    }
}

fn poll(r: &mut DeviceReconciler, ms: u64, holders: Option<&[u32]>, device: &str) -> bool {
    r.poll(
        Duration::from_millis(ms),
        &mut || Some(vec![device.into()]),
        &mut |_| holders.map(|ids| ids.iter().copied().collect()),
    )
}

fn confirmed() -> (DeviceReconciler, AttributionSession, AttributionJob) {
    let mut r = DeviceReconciler::new();
    let mut s = AttributionSession::new();
    poll(&mut r, 0, Some(&[42]), "/dev/video0");
    assert!(s.reconcile(&r, Duration::ZERO, true).is_none());
    poll(&mut r, 1000, Some(&[42]), "/dev/video0");
    let j = s.reconcile(&r, Duration::from_secs(1), true).unwrap();
    (r, s, j)
}

#[test]
fn pending_two_is_replaced_by_three_and_results_have_one_slot() {
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = AttributionWorker::new(move |j| {
        started_tx.send(j.generation).unwrap();
        if j.generation == 1 {
            release_rx.recv().unwrap();
        }
        details(&j.generation.to_string())
    })
    .unwrap();
    worker.submit(job(1));
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
    worker.submit(job(2));
    worker.submit(job(3));
    release_tx.send(()).unwrap();
    assert_eq!(started_rx.recv_timeout(Duration::from_secs(1)).unwrap(), 3);
    let deadline = Instant::now() + Duration::from_secs(1);
    let result = loop {
        if let Some(result) = worker.take_result()
            && result.generation == 3
        {
            break result;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    assert_eq!(
        result.details.observations[0].application.as_deref(),
        Some("3")
    );
    assert!(worker.take_result().is_none());
    assert!(started_rx.try_recv().is_err());
}

#[test]
fn blocked_enrichment_does_not_delay_active_publication_or_shutdown() {
    let (_r, s, j) = confirmed();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = AttributionWorker::new(move |_| {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        details("Browser")
    })
    .unwrap();
    worker.submit(j);
    started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let mut stream = StatusStream::new();
    let line = stream.device_update(s.status()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(json["camera"]["state"], "active");
    assert!(json["camera"].get("details").is_none());
    let start = Instant::now();
    drop(worker);
    assert!(start.elapsed() < Duration::from_millis(100));
    release_tx.send(()).unwrap();
}

#[test]
fn stop_new_holder_device_change_and_lookup_failure_reject_late_results() {
    for change in ["stop", "holder", "device", "failure"] {
        let (mut r, mut s, j) = confirmed();
        assert!(s.accept(AttributionResult {
            generation: j.generation,
            details: details("Old")
        }));
        r.request(Duration::from_secs(2));
        let ids = match change {
            "stop" => Some(&[][..]),
            "holder" => Some(&[43][..]),
            "failure" => None,
            _ => Some(&[42][..]),
        };
        poll(
            &mut r,
            2000,
            ids,
            if change == "device" {
                "/dev/video1"
            } else {
                "/dev/video0"
            },
        );
        s.reconcile(&r, Duration::from_secs(2), true);
        assert!(s.status().details.is_none(), "{change}");
        assert!(
            !s.accept(AttributionResult {
                generation: j.generation,
                details: details("Late")
            }),
            "{change}"
        );
    }
    let (mut r, mut s, j) = confirmed();
    r.request(Duration::from_secs(2));
    poll(&mut r, 2000, Some(&[]), "/dev/video0");
    s.reconcile(&r, Duration::from_secs(2), true);
    r.request(Duration::from_secs(3));
    poll(&mut r, 3000, Some(&[42]), "/dev/video0");
    s.reconcile(&r, Duration::from_secs(3), true);
    poll(&mut r, 4000, Some(&[42]), "/dev/video0");
    assert!(s.reconcile(&r, Duration::from_secs(4), true).is_some());
    assert!(!s.accept(AttributionResult {
        generation: j.generation,
        details: details("Previous session")
    }));
}

#[test]
fn short_probes_never_queue_and_fresh_reconciliation_refreshes_without_state_reset() {
    let mut r = DeviceReconciler::new();
    let mut s = AttributionSession::new();
    poll(&mut r, 0, Some(&[42]), "/dev/video0");
    assert!(s.reconcile(&r, Duration::ZERO, true).is_none());
    r.request(Duration::from_millis(400));
    assert!(s.reconcile(&r, Duration::from_millis(750), false).is_none());
    poll(&mut r, 1000, Some(&[]), "/dev/video0");
    assert!(s.reconcile(&r, Duration::from_secs(1), true).is_none());
    let (mut r, mut s, j) = confirmed();
    assert!(s.accept(AttributionResult {
        generation: j.generation,
        details: details("First")
    }));
    assert!(s.reconcile(&r, Duration::from_secs(2), false).is_none());
    assert!(s.status().details.is_some());
    poll(&mut r, 6000, Some(&[42]), "/dev/video0");
    let fresh = s.reconcile(&r, Duration::from_secs(6), true).unwrap();
    assert_ne!(fresh.generation, j.generation);
    assert!(!s.accept(AttributionResult {
        generation: j.generation,
        details: details("Stale")
    }));
    let state = s.status().state;
    assert!(s.accept(AttributionResult {
        generation: fresh.generation,
        details: details("Renamed")
    }));
    assert_eq!(s.status().state, state);
    assert_eq!(state, State::Active);
    assert_eq!(
        s.status().details.unwrap().observations[0]
            .application
            .as_deref(),
        Some("Renamed")
    );
}

#[test]
fn worker_reads_only_supplied_paths_and_holder_pids_and_caps_holders() {
    let f = support::Fixture::new();
    f.device("video0", "/dev/null");
    f.device("video1", "/dev/zero");
    for pid in 1..=17 {
        f.holder(pid, &format!("App{pid}"), &["/dev/null"]);
    }
    let j = AttributionJob {
        generation: 9,
        holders: (1..=17).collect(),
        devices: vec![f.0.join("dev/video0")],
    };
    let d = enrich_job(&f.0.join("proc"), &f.0.join("sys"), &j);
    assert!(d.truncated);
    assert_eq!(d.observations.len(), 16);
    assert!(
        d.observations
            .iter()
            .all(|row| row.device_path.as_deref() == Some("/dev/video0"))
    );
    assert!(
        d.observations
            .iter()
            .all(|row| row.application.as_deref() != Some("App17"))
    );
}

#[test]
fn enrichment_failure_clears_labels_without_altering_detection_and_worker_recovers() {
    let (_r, mut s, j) = confirmed();
    assert!(s.accept(AttributionResult {
        generation: j.generation,
        details: details("Old")
    }));
    let worker = AttributionWorker::new(|j| {
        if j.generation == 1 {
            panic!("fixture failure");
        }
        details("Recovered")
    })
    .unwrap();
    worker.submit(job(1));
    let deadline = Instant::now() + Duration::from_secs(1);
    let result = loop {
        if let Some(result) = worker.take_result() {
            break result;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    assert!(result.details.observations[0].application.is_none());
    assert!(s.accept(AttributionResult {
        generation: j.generation,
        details: result.details
    }));
    assert_eq!(s.status().state, State::Active);
    assert!(
        s.status().details.unwrap().observations[0]
            .application
            .is_none()
    );
    worker.submit(job(2));
    loop {
        if let Some(result) = worker.take_result() {
            assert_eq!(
                result.details.observations[0].application.as_deref(),
                Some("Recovered")
            );
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
fn clearing_pending_work_cancels_unstarted_job_without_waiting_for_running_io() {
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = AttributionWorker::new(move |j| {
        started_tx.send(j.generation).unwrap();
        if j.generation == 1 {
            release_rx.recv().unwrap();
        }
        details("Fixture")
    })
    .unwrap();
    worker.submit(job(1));
    started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    worker.submit(job(2));
    worker.clear_pending();
    release_tx.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if worker.take_result().is_some() {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(started_rx.recv_timeout(Duration::from_millis(20)).is_err());
}

#[test]
fn device_and_process_metadata_share_one_100ms_job_budget() {
    use glint_helper::attribution_worker::enrich_job_with;
    use glint_helper::device_metadata::{FdEntries, MetadataIo};
    use std::cell::Cell;
    use std::io;
    use std::path::Path;

    struct BudgetIo {
        start: Instant,
        elapsed: Cell<Duration>,
        calls: Cell<usize>,
    }
    impl BudgetIo {
        fn step(&self, deadline: Instant) {
            assert_eq!(deadline, self.start + Duration::from_millis(100));
            assert!(self.now() < deadline);
            self.calls.set(self.calls.get() + 1);
            self.elapsed
                .set(self.elapsed.get() + Duration::from_millis(30));
        }
    }
    impl MetadataIo for BudgetIo {
        fn now(&self) -> Instant {
            self.start + self.elapsed.get()
        }
        fn read_text(&self, path: &Path, deadline: Instant) -> io::Result<String> {
            self.step(deadline);
            Ok(if path.ends_with("stat") {
                support::stat(42, 1)
            } else {
                "Label".into()
            })
        }
        fn character_device(&self, _: &Path, deadline: Instant) -> io::Result<Option<u64>> {
            self.step(deadline);
            Ok(Some(1))
        }
        fn fd_entries(&self, _: &Path, _: Instant) -> io::Result<FdEntries> {
            panic!("no FD scan after budget expires")
        }
    }
    let io = BudgetIo {
        start: Instant::now(),
        elapsed: Cell::new(Duration::ZERO),
        calls: Cell::new(0),
    };
    let d = enrich_job_with(
        &io,
        Path::new("/fixture/proc"),
        Path::new("/fixture/sys"),
        &job(1),
    );
    assert_eq!(io.calls.get(), 4);
    assert!(d.truncated);
    assert!(d.observations[0].application.is_none());
    assert!(d.observations[0].device_path.is_none());
}

#[test]
fn fresh_jobs_observe_renames_device_replacement_and_missing_metadata() {
    let f = support::Fixture::new();
    f.device("video0", "/dev/null");
    f.holder(42, "First", &["/dev/null"]);
    let j = AttributionJob {
        devices: vec![f.0.join("dev/video0")],
        ..job(1)
    };
    let enrich = || enrich_job(&f.0.join("proc"), &f.0.join("sys"), &j);
    assert_eq!(
        enrich().observations[0].application.as_deref(),
        Some("First")
    );
    std::fs::write(f.0.join("proc/42/comm"), "Renamed").unwrap();
    std::fs::write(f.0.join("sys/class/video4linux/video0/name"), "New Camera").unwrap();
    let updated = enrich();
    assert_eq!(
        updated.observations[0].application.as_deref(),
        Some("Renamed")
    );
    assert_eq!(
        updated.observations[0].device.as_deref(),
        Some("New Camera")
    );
    std::fs::remove_file(f.0.join("dev/video0")).unwrap();
    std::os::unix::fs::symlink("/dev/zero", f.0.join("dev/video0")).unwrap();
    let replaced = enrich();
    assert!(replaced.observations[0].application.is_none());
    assert!(replaced.observations[0].device_path.is_none());
    std::fs::remove_file(f.0.join("dev/video0")).unwrap();
    let missing = enrich();
    assert!(missing.observations[0].application.is_none());
    assert!(missing.observations[0].device_path.is_none());
}
