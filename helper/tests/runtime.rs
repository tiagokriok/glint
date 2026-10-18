use glint_helper::attribution::{ApplicationSource, CameraDetails, EvidenceKind, Observation};
use glint_helper::runtime::StatusStream;
use glint_helper::status::{State, Status};

#[test]
fn details_changes_emit_without_coalescing_stops_or_restarts() {
    let detailed = |app: &str| Status {
        details: Some(CameraDetails {
            observations: vec![Observation {
                application: Some(app.into()),
                application_source: ApplicationSource::Process,
                device: None,
                device_path: None,
                evidence: EvidenceKind::PipewireCapture,
            }],
            truncated: false,
        }),
        ..status(State::Active)
    };
    let mut stream = StatusStream::new();
    stream.device_update(status(State::Unobserved));
    let lines = stream.pipewire_updates([
        detailed("First"),
        detailed("Second"),
        detailed("Second"),
        status(State::Unobserved),
        detailed("First"),
    ]);
    let values: Vec<serde_json::Value> = lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(values.len(), 4);
    assert_eq!(
        values[0]["camera"]["details"]["observations"][0]["application"],
        "First"
    );
    assert_eq!(
        values[1]["camera"]["details"]["observations"][0]["application"],
        "Second"
    );
    assert_eq!(values[2]["camera"]["state"], "unobserved");
    assert!(values[2]["camera"].get("details").is_none());
    assert_eq!(values[3], values[0]);
}

fn status(state: State) -> Status {
    Status::new(state, "fixture", &[])
}

#[test]
fn queued_stop_then_start_emits_the_reset_between_sessions() {
    let mut stream = StatusStream::new();
    stream.device_update(status(State::Unobserved));
    let lines = stream.pipewire_updates([
        status(State::Active),
        status(State::Unobserved),
        status(State::Active),
    ]);
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("\"state\":\"active\""));
    assert!(lines[1].contains("\"state\":\"unobserved\""));
    assert!(lines[2].contains("\"state\":\"active\""));
}

#[test]
fn positive_device_path_survives_pipewire_stop_and_duplicate_updates_are_silent() {
    let mut stream = StatusStream::new();
    stream.device_update(status(State::Active));
    let lines = stream.pipewire_updates([status(State::Unknown), status(State::Unobserved)]);
    assert!(
        lines
            .iter()
            .all(|line| line.contains("\"state\":\"active\""))
    );
    assert!(
        stream
            .pipewire_updates([status(State::Unobserved)])
            .is_empty()
    );
    assert!(
        stream
            .device_update(status(State::Unobserved))
            .unwrap()
            .contains("\"state\":\"unobserved\"")
    );
}

fn graph_status(app: &str) -> Status {
    use glint_helper::camera::*;
    let source = GraphNode {
        video_path: Some("/dev/video2".into()),
        ..GraphNode::new(2, "Video/Source", NodeState::Running).with_device_api("v4l2")
    };
    classify(
        &CameraGraph::new(true)
            .node(source)
            .node(
                GraphNode::new(10, "Stream/Input/Video", NodeState::Running)
                    .with_owner("call", app),
            )
            .link(GraphLink::new(100, 2, 10)),
    )
}

#[test]
fn pipewire_enrichment_is_optional_and_stale_results_cannot_cross_sessions() {
    use glint_helper::attribution_worker::AttributionResult;
    use glint_helper::runtime::PipewireDetails;
    let mut session = PipewireDetails::default();
    let a = session.update(graph_status("A")).unwrap();
    assert_eq!(session.status().state, State::Active);
    assert_eq!(
        session.status().details.unwrap().observations[0].device,
        None
    );
    assert!(session.update(graph_status("A")).is_none());
    let enriched = CameraDetails {
        observations: vec![Observation {
            application: None,
            application_source: ApplicationSource::Unknown,
            device: Some("Verified camera".into()),
            device_path: Some("/dev/video2".into()),
            evidence: EvidenceKind::PipewireCapture,
        }],
        truncated: false,
    };
    assert!(session.accept(AttributionResult {
        generation: a.generation,
        details: enriched.clone()
    }));
    assert_eq!(
        session.status().details.unwrap().observations[0]
            .device
            .as_deref(),
        Some("Verified camera")
    );
    session.update(status(State::Unobserved));
    assert!(!session.accept(AttributionResult {
        generation: a.generation,
        details: enriched.clone()
    }));
    let b = session.update(graph_status("B")).unwrap();
    assert!(!session.accept(AttributionResult {
        generation: a.generation,
        details: enriched.clone()
    }));
    assert!(session.accept(AttributionResult {
        generation: b.generation,
        details: enriched
    }));
    assert_eq!(
        session.status().details.unwrap().observations[0]
            .application
            .as_deref(),
        Some("B")
    );
}

#[test]
fn graph_stop_start_and_disconnect_clear_only_pipewire_rows() {
    let mut stream = StatusStream::new();
    let device = Status {
        details: Some(CameraDetails {
            observations: vec![Observation {
                application: Some("Direct holder".into()),
                application_source: ApplicationSource::Process,
                device: None,
                device_path: None,
                evidence: EvidenceKind::DeviceAccess,
            }],
            truncated: false,
        }),
        ..status(State::Active)
    };
    stream.device_update(device);
    let lines = stream.pipewire_updates([
        graph_status("A"),
        status(State::Unobserved),
        graph_status("B"),
        glint_helper::observer::unavailable_status(),
    ]);
    assert_eq!(lines.len(), 4);
    let values: Vec<serde_json::Value> = lines
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for value in &values {
        assert_eq!(value["camera"]["state"], "active");
    }
    assert!(!lines[2].contains("\"A\""));
    for index in [1, 3] {
        let rows = values[index]["camera"]["details"]["observations"]
            .as_array()
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["application"], "Direct holder");
    }
}

#[test]
fn sysfs_fallback_requires_valid_path_character_device_and_matching_sysfs_identity() {
    use glint_helper::attribution_worker::AttributionJob;
    use glint_helper::device_metadata::MetadataIo;
    use glint_helper::runtime::enrich_pipewire_with;
    use std::{io, path::Path, time::Instant};
    struct Io {
        character: bool,
        sys_id: &'static str,
    }
    impl MetadataIo for Io {
        fn read_text(&self, path: &Path, _: Instant) -> io::Result<String> {
            match path.file_name().and_then(|s| s.to_str()) {
                Some("name") => Ok(" Verified\n camera \u{202e}".into()),
                Some("dev") => Ok(self.sys_id.into()),
                _ => panic!("unexpected metadata path"),
            }
        }
        fn character_device(&self, path: &Path, _: Instant) -> io::Result<Option<u64>> {
            assert_eq!(path, Path::new("/fixture-dev/video2"));
            Ok(self.character.then_some(libc::makedev(81, 2)))
        }
    }
    let job = AttributionJob {
        generation: 1,
        holders: Default::default(),
        devices: vec![
            "/dev/video2".into(),
            "video2".into(),
            "/dev/video2/../video0".into(),
        ],
    };
    for (character, sys_id, expected) in [
        (true, "81:2\n", 1),
        (false, "81:2", 0),
        (true, "81:3", 0),
        (true, "invalid", 0),
    ] {
        let rows = enrich_pipewire_with(
            &Io { character, sys_id },
            Path::new("/fixture-sys"),
            Path::new("/fixture-dev"),
            &job,
        )
        .observations;
        assert_eq!(rows.len(), expected);
        if expected == 1 {
            assert_eq!(rows[0].device.as_deref(), Some("Verified camera"));
        }
    }
}

#[test]
fn pipewire_metadata_failure_or_replacement_keeps_capture_and_clears_prior_fallback() {
    use glint_helper::{attribution_worker::AttributionResult, runtime::PipewireDetails};
    let mut session = PipewireDetails::default();
    let job = session.update(graph_status("A")).unwrap();
    assert!(session.accept(AttributionResult {
        generation: job.generation,
        details: CameraDetails {
            observations: vec![],
            truncated: false
        }
    }));
    assert_eq!(session.status(), graph_status("A"));
    let reported = Status {
        details: Some(CameraDetails {
            observations: vec![Observation {
                device: Some("Reported camera".into()),
                ..graph_status("A").details.unwrap().observations.remove(0)
            }],
            truncated: false,
        }),
        ..graph_status("A")
    };
    assert!(session.update(reported.clone()).is_none());
    assert!(!session.accept(AttributionResult {
        generation: job.generation,
        details: CameraDetails {
            observations: vec![],
            truncated: false
        }
    }));
    assert_eq!(session.status(), reported);
    assert!(session.update(status(State::Active)).is_none());
    assert!(!session.accept(AttributionResult {
        generation: job.generation + 2,
        details: CameraDetails {
            observations: vec![],
            truncated: false
        }
    }));
}

#[test]
fn pipewire_metadata_jobs_are_device_capped_and_deadline_bounded() {
    use glint_helper::{
        attribution_worker::AttributionJob, device_metadata::MetadataIo,
        runtime::enrich_pipewire_with,
    };
    use std::{
        cell::Cell,
        io,
        path::Path,
        time::{Duration, Instant},
    };
    struct Io {
        now: Instant,
        expire: bool,
        ticks: Cell<u32>,
        stats: Cell<u32>,
    }
    impl MetadataIo for Io {
        fn now(&self) -> Instant {
            let tick = self.ticks.get();
            self.ticks.set(tick + 1);
            self.now
                + if self.expire && tick > 0 {
                    Duration::from_secs(1)
                } else {
                    Duration::ZERO
                }
        }
        fn read_text(&self, _: &Path, _: Instant) -> io::Result<String> {
            panic!("absent character device must skip sysfs");
        }
        fn character_device(&self, _: &Path, _: Instant) -> io::Result<Option<u64>> {
            self.stats.set(self.stats.get() + 1);
            Ok(None)
        }
    }
    let job = AttributionJob {
        generation: 1,
        holders: Default::default(),
        devices: (0..40).map(|id| format!("/dev/video{id}").into()).collect(),
    };
    for (expire, expected) in [(false, 32), (true, 0)] {
        let io = Io {
            now: Instant::now(),
            expire,
            ticks: Cell::new(0),
            stats: Cell::new(0),
        };
        let details = enrich_pipewire_with(
            &io,
            Path::new("/fixture-sys"),
            Path::new("/fixture-dev"),
            &job,
        );
        assert!(details.truncated);
        assert!(details.observations.is_empty());
        assert_eq!(io.stats.get(), expected);
    }
}

#[test]
fn production_pipewire_publisher_sends_evidence_before_submit_and_rejects_late_results() {
    use glint_helper::{
        attribution_worker::{AttributionJob, AttributionResult},
        device_runtime::AttributionQueue,
        runtime::PipewirePublisher,
    };
    use std::{cell::RefCell, rc::Rc, sync::mpsc};
    struct Queue {
        receiver: Rc<mpsc::Receiver<Status>>,
        jobs: Rc<RefCell<Vec<AttributionJob>>>,
        result: Rc<RefCell<Option<AttributionResult>>>,
    }
    impl AttributionQueue for Queue {
        fn submit(&self, job: AttributionJob) {
            let published = self
                .receiver
                .try_recv()
                .expect("capture must be sent before worker submission");
            assert_eq!(published.state, State::Active);
            assert!(published.details.unwrap().observations[0].device.is_none());
            self.jobs.borrow_mut().push(job);
        }
        fn clear_pending(&self) {
            self.result.borrow_mut().take();
        }
        fn take_result(&self) -> Option<AttributionResult> {
            self.result.borrow_mut().take()
        }
    }
    let (sink, receiver) = mpsc::sync_channel(8);
    let receiver = Rc::new(receiver);
    let jobs: Rc<RefCell<Vec<AttributionJob>>> = Rc::default();
    let result: Rc<RefCell<Option<AttributionResult>>> = Rc::default();
    let worker = Queue {
        receiver: receiver.clone(),
        jobs: jobs.clone(),
        result: result.clone(),
    };
    let mut publisher = PipewirePublisher::new(sink, Some(worker));
    publisher.publish(graph_status("A"));
    let generation = jobs.borrow()[0].generation;
    publisher.publish(status(State::Unobserved));
    assert_eq!(receiver.try_recv().unwrap().state, State::Unobserved);
    publisher.publish(graph_status("B"));
    *result.borrow_mut() = Some(AttributionResult {
        generation,
        details: CameraDetails {
            observations: vec![],
            truncated: false,
        },
    });
    publisher.poll();
    assert!(receiver.try_recv().is_err());
    *result.borrow_mut() = Some(AttributionResult {
        generation: jobs.borrow()[1].generation,
        details: CameraDetails {
            observations: vec![],
            truncated: false,
        },
    });
    publisher.poll();
    assert_eq!(receiver.try_recv().unwrap(), graph_status("B"));
}

#[test]
fn missing_pipewire_worker_still_publishes_capture_and_closed_sink_skips_work() {
    use glint_helper::{
        attribution_worker::{AttributionJob, AttributionResult, AttributionWorker},
        device_runtime::AttributionQueue,
        runtime::PipewirePublisher,
    };
    use std::sync::mpsc;
    let (sink, receiver) = mpsc::sync_channel(2);
    let mut publisher = PipewirePublisher::<AttributionWorker>::new(sink, None);
    publisher.publish(graph_status("A"));
    publisher.poll();
    assert_eq!(receiver.try_recv().unwrap(), graph_status("A"));
    struct NoWork;
    impl AttributionQueue for NoWork {
        fn submit(&self, _: AttributionJob) {
            panic!("closed sink must not schedule metadata work");
        }
        fn clear_pending(&self) {
            panic!("closed sink must not schedule metadata work");
        }
        fn take_result(&self) -> Option<AttributionResult> {
            None
        }
    }
    let (sink, receiver) = mpsc::sync_channel(1);
    drop(receiver);
    PipewirePublisher::new(sink, Some(NoWork)).publish(graph_status("A"));
}
