use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use glint_helper::attribution::{ApplicationSource, CameraDetails, EvidenceKind, Observation};
use glint_helper::attribution_worker::{AttributionJob, AttributionResult, AttributionWorker};
use glint_helper::device_runtime::{AttributionQueue, DeviceRuntime};
use glint_helper::status::{State, Status};

type Events = Arc<Mutex<Vec<&'static str>>>;

struct RecordingWriter {
    events: Events,
    bytes: Vec<u8>,
    fail_flush: bool,
}

impl RecordingWriter {
    fn new(events: &Events) -> Self {
        Self {
            events: Arc::clone(events),
            bytes: Vec::new(),
            fail_flush: false,
        }
    }

    fn lines(&self) -> Vec<serde_json::Value> {
        String::from_utf8(self.bytes.clone())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

impl Write for RecordingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.events.lock().unwrap().push("write");
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.events.lock().unwrap().push("flush");
        if self.fail_flush {
            Err(io::Error::other("fixture flush failure"))
        } else {
            Ok(())
        }
    }
}

struct RecordingQueue {
    events: Events,
    worker: AttributionWorker,
}

impl AttributionQueue for RecordingQueue {
    fn submit(&self, job: AttributionJob) {
        self.events.lock().unwrap().push("submit");
        self.worker.submit(job);
    }

    fn clear_pending(&self) {
        self.events.lock().unwrap().push("clear");
        self.worker.clear_pending();
    }

    fn take_result(&self) -> Option<AttributionResult> {
        self.events.lock().unwrap().push("take");
        self.worker.take_result()
    }
}

fn details() -> CameraDetails {
    CameraDetails {
        observations: vec![Observation {
            application: Some("Old caller".into()),
            application_source: ApplicationSource::Process,
            device: None,
            device_path: Some("/dev/video0".into()),
            evidence: EvidenceKind::DeviceAccess,
        }],
        truncated: false,
    }
}

fn poll<W: AttributionQueue>(
    runtime: &mut DeviceRuntime<W>,
    writer: &mut RecordingWriter,
    ms: u64,
    holders: &[u32],
) -> io::Result<()> {
    let events = Arc::clone(&writer.events);
    runtime.poll(
        Duration::from_millis(ms),
        writer,
        &mut || {
            events.lock().unwrap().push("discover");
            Some(vec![PathBuf::from("/fixture/video0")])
        },
        &mut |paths| {
            assert_eq!(paths, &[PathBuf::from("/fixture/video0")]);
            events.lock().unwrap().push("lookup");
            Some(holders.iter().copied().collect::<BTreeSet<_>>())
        },
        &mut |_| events.lock().unwrap().push("sync"),
    )
}

#[test]
fn production_path_flushes_before_submit_and_stays_responsive_with_blocked_worker() {
    let events = Events::default();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = AttributionWorker::new(move |_| {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        details()
    })
    .unwrap();
    let queue = RecordingQueue {
        events: Arc::clone(&events),
        worker,
    };
    let mut runtime = DeviceRuntime::new(Some(queue));
    let mut writer = RecordingWriter::new(&events);
    runtime
        .pipewire_updates(
            &mut writer,
            [Status::new(State::Unobserved, "fixture", &[])],
        )
        .unwrap();
    poll(&mut runtime, &mut writer, 0, &[42]).unwrap();
    assert!(
        !events.lock().unwrap().contains(&"submit"),
        "unconfirmed probe queued"
    );
    events.lock().unwrap().clear();
    writer.bytes.clear();

    poll(&mut runtime, &mut writer, 1000, &[42]).unwrap();
    started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let trace = events.lock().unwrap().clone();
    let flush = trace.iter().position(|event| *event == "flush").unwrap();
    let submit = trace.iter().position(|event| *event == "submit").unwrap();
    assert!(trace[..flush].contains(&"write"));
    assert!(flush < submit, "{trace:?}");
    let lines = writer.lines();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["camera"]["state"], "active");
    assert!(lines[0]["camera"].get("details").is_none());

    runtime.request(Duration::from_secs(2));
    poll(&mut runtime, &mut writer, 2000, &[]).unwrap();
    assert_eq!(
        writer.lines().last().unwrap()["camera"]["state"],
        "unobserved"
    );
    let (stopped_tx, stopped_rx) = mpsc::channel();
    let shutdown = std::thread::spawn(move || {
        drop(runtime);
        stopped_tx.send(()).unwrap();
    });
    let stopped = stopped_rx.recv_timeout(Duration::from_millis(100));
    release_tx.send(()).unwrap();
    shutdown.join().unwrap();
    assert!(stopped.is_ok(), "runtime shutdown waited for enrichment");
}

// Hold an actual worker result outside its slot so readiness is deterministic.
// Deliver it even after cancellation to exercise generation rejection, as when
// running work completes after clear_pending. No synthetic result/session calls.
struct ReadyQueue {
    events: Events,
    worker: Arc<AttributionWorker>,
    ready: Arc<Mutex<Option<AttributionResult>>>,
}

impl AttributionQueue for ReadyQueue {
    fn submit(&self, job: AttributionJob) {
        self.worker.submit(job);
    }
    fn clear_pending(&self) {
        self.events.lock().unwrap().push("clear");
        self.worker.clear_pending();
    }
    fn take_result(&self) -> Option<AttributionResult> {
        self.events.lock().unwrap().push("take");
        self.ready.lock().unwrap().take()
    }
}

fn rejects_ready_old_result(next_holders: &[u32]) {
    let events = Events::default();
    let worker = Arc::new(AttributionWorker::new(|_| details()).unwrap());
    let ready = Arc::new(Mutex::new(None));
    let queue = ReadyQueue {
        events: Arc::clone(&events),
        worker: Arc::clone(&worker),
        ready: Arc::clone(&ready),
    };
    let mut runtime = DeviceRuntime::new(Some(queue));
    let mut writer = RecordingWriter::new(&events);
    runtime
        .pipewire_updates(
            &mut writer,
            [Status::new(State::Unobserved, "fixture", &[])],
        )
        .unwrap();
    poll(&mut runtime, &mut writer, 0, &[42]).unwrap();
    poll(&mut runtime, &mut writer, 1000, &[42]).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    let result = loop {
        if let Some(result) = worker.take_result() {
            break result;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    };
    *ready.lock().unwrap() = Some(result);
    events.lock().unwrap().clear();
    writer.bytes.clear();
    runtime.request(Duration::from_secs(2));
    poll(&mut runtime, &mut writer, 2000, next_holders).unwrap();
    let lines = writer.lines();
    assert_eq!(
        lines.len(),
        1,
        "old result emitted before invalidation: {lines:?}"
    );
    assert_eq!(lines[0]["camera"]["state"], "unobserved");
    assert!(lines[0]["camera"].get("details").is_none());
    let trace = events.lock().unwrap().clone();
    let lookup = trace.iter().position(|event| *event == "lookup").unwrap();
    let flush = trace.iter().position(|event| *event == "flush").unwrap();
    let clear = trace.iter().position(|event| *event == "clear").unwrap();
    let take = trace.iter().position(|event| *event == "take").unwrap();
    assert!(lookup < flush && flush < clear && clear < take, "{trace:?}");
    assert!(ready.lock().unwrap().is_none());
}

#[test]
fn production_path_observes_stop_before_consuming_ready_old_result() {
    rejects_ready_old_result(&[]);
}

#[test]
fn production_path_observes_changed_holders_before_consuming_ready_old_result() {
    rejects_ready_old_result(&[43]);
}

#[test]
fn production_path_never_submits_when_publication_flush_fails() {
    let events = Events::default();
    let worker = AttributionWorker::new(|_| details()).unwrap();
    let queue = RecordingQueue {
        events: Arc::clone(&events),
        worker,
    };
    let mut runtime = DeviceRuntime::new(Some(queue));
    let mut writer = RecordingWriter::new(&events);
    runtime
        .pipewire_updates(
            &mut writer,
            [Status::new(State::Unobserved, "fixture", &[])],
        )
        .unwrap();
    poll(&mut runtime, &mut writer, 0, &[42]).unwrap();
    events.lock().unwrap().clear();
    writer.fail_flush = true;
    assert!(poll(&mut runtime, &mut writer, 1000, &[42]).is_err());
    assert!(!events.lock().unwrap().contains(&"submit"));
    assert!(!events.lock().unwrap().contains(&"take"));
}
