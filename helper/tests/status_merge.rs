//! Merge precedence and JSON-lines serialization.

use glint_helper::attribution::{
    ApplicationSource, CameraDetails, DeviceIdentity, EvidenceKind, Observation,
};
use glint_helper::status::{MAX_LINE_BYTES, State, Status, bounded_json_line, merge};
use serde_json::{Value, json};

fn with_details(mut status: Status, app: &str, evidence: EvidenceKind) -> Status {
    status.details = Some(CameraDetails {
        observations: vec![Observation {
            application: Some(app.into()),
            application_source: ApplicationSource::Process,
            device: Some("Camera".into()),
            device_path: Some("/dev/video0".into()),
            evidence,
        }],
        truncated: false,
    });
    status
}

#[test]
fn additive_details_contract_preserves_base_fields_and_escapes_labels() {
    let base = active_capture();
    assert_eq!(base.details, None);
    let status = with_details(base.clone(), "App \\\"", EvidenceKind::PipewireCapture);
    let value: Value = serde_json::from_str(&status.to_json_line()).unwrap();
    let old: Value = serde_json::from_str(&base.to_json_line()).unwrap();
    assert_eq!(value["version"], 1);
    for key in ["state", "evidence", "limitations"] {
        assert_eq!(value["camera"][key], old["camera"][key]);
    }
    assert_eq!(
        value["camera"]["details"],
        json!({
            "observations": [{"application": "App \\\"", "applicationSource": "process", "device": "Camera", "devicePath": "/dev/video0", "evidence": "pipewire-capture"}],
            "truncated": false
        })
    );
    let identity = DeviceIdentity {
        path: "/dev/video0".into(),
        device_id: 42,
        name: Some("Camera".into()),
    };
    assert_eq!(identity.device_id, 42);
    assert_eq!(value["camera"].as_object().unwrap().len(), 4);
    assert_eq!(
        value["camera"]["details"]["observations"][0]
            .as_object()
            .unwrap()
            .len(),
        5
    );
}

#[test]
fn all_public_enum_values_and_nulls_are_serialized() {
    for (source, name) in [
        (ApplicationSource::Process, "process"),
        (ApplicationSource::PipewireReported, "pipewire-reported"),
        (ApplicationSource::Unknown, "unknown"),
    ] {
        let mut status = with_details(active_device(), "App", EvidenceKind::DeviceAccess);
        status.details = Some(CameraDetails {
            observations: vec![Observation {
                application: None,
                application_source: source,
                device: None,
                device_path: None,
                evidence: EvidenceKind::DeviceAccess,
            }],
            truncated: false,
        });
        let value: Value = serde_json::from_str(&status.to_json_line()).unwrap();
        assert_eq!(
            value["camera"]["details"]["observations"][0],
            json!({"application": null, "applicationSource": name, "device": null, "devicePath": null, "evidence": "device-access"})
        );
    }
}

#[test]
fn worst_case_escaped_details_fit_the_line_budget() {
    let mut status = active_capture();
    status.details = Some(CameraDetails {
        observations: (0..32)
            .map(|n| Observation {
                application: Some(format!("{n:02}{}", "\\\"".repeat(39))),
                application_source: ApplicationSource::PipewireReported,
                device: Some("\\".repeat(80)),
                device_path: Some(format!("/dev/video{}", "1".repeat(54))),
                evidence: EvidenceKind::PipewireCapture,
            })
            .collect(),
        truncated: false,
    });
    let line = bounded_json_line(&status);
    assert!(line.len() <= MAX_LINE_BYTES);
    let value: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(value["camera"]["state"], "active");
    assert!(
        value["camera"]["details"]["observations"]
            .as_array()
            .unwrap()
            .len()
            <= 32
    );
}

#[test]
fn metadata_budget_never_downgrades_active() {
    let base = Status::new(
        State::Active,
        &"x".repeat(MAX_LINE_BYTES - 250),
        &["original limitation"],
    );
    let status = with_details(
        base.clone(),
        &"\\".repeat(MAX_LINE_BYTES * 2),
        EvidenceKind::DeviceAccess,
    );
    assert!(status.to_json_line().len() > MAX_LINE_BYTES);
    let line = bounded_json_line(&status);
    assert!(line.len() <= MAX_LINE_BYTES);
    let value: Value = serde_json::from_str(&line).unwrap();
    let original: Value = serde_json::from_str(&base.to_json_line()).unwrap();
    for key in ["state", "evidence", "limitations"] {
        assert_eq!(value["camera"][key], original["camera"][key]);
    }
    assert_eq!(value["camera"]["details"]["truncated"], true);
    assert_eq!(value["camera"]["details"]["observations"], json!([]));
}

#[test]
fn metadata_is_omitted_when_even_an_empty_details_wrapper_exceeds_budget() {
    let overhead = Status::new(State::Active, "", &[]).to_json_line().len();
    let base = Status::new(State::Active, &"x".repeat(MAX_LINE_BYTES - overhead), &[]);
    let status = with_details(base.clone(), "App", EvidenceKind::DeviceAccess);
    assert_eq!(bounded_json_line(&status), base.to_json_line());
    assert_eq!(bounded_json_line(&status).len(), MAX_LINE_BYTES);
}

#[test]
fn row_budget_preserves_a_deterministic_prefix_and_reports_omission() {
    let base = Status::new(State::Active, &"x".repeat(MAX_LINE_BYTES - 900), &[]);
    let first = with_details(base.clone(), "A", EvidenceKind::DeviceAccess)
        .details
        .unwrap()
        .observations[0]
        .clone();
    let rows = (0..33)
        .map(|n| Observation {
            application: Some(format!("App {n:02}")),
            ..first.clone()
        })
        .collect();
    let status = Status {
        details: Some(CameraDetails {
            observations: rows,
            truncated: false,
        }),
        ..base
    };
    let line = bounded_json_line(&status);
    let value: Value = serde_json::from_str(&line).unwrap();
    let observations = value["camera"]["details"]["observations"]
        .as_array()
        .unwrap();
    assert!(!observations.is_empty());
    assert!(observations.len() < 32);
    assert_eq!(observations[0]["application"], "App 00");
    assert_eq!(value["camera"]["details"]["truncated"], true);
    assert_eq!(value["camera"]["state"], "active");
    assert!(line.len() <= MAX_LINE_BYTES);
}

#[test]
fn merge_includes_only_active_source_details_and_preserves_omission() {
    let capture = with_details(active_capture(), "PW", EvidenceKind::PipewireCapture);
    let device = with_details(active_device(), "Device", EvidenceKind::DeviceAccess);
    let both = merge(&capture, &device);
    assert_eq!(both.details.as_ref().unwrap().observations.len(), 2);
    for state in [State::Unknown, State::Unobserved] {
        let stale = with_details(
            Status::new(state, "stale", &[]),
            "Stale",
            EvidenceKind::DeviceAccess,
        );
        assert_eq!(merge(&capture, &stale).details, capture.details);
        assert_eq!(merge(&stale, &device).details, device.details);
        assert_eq!(merge(&stale, &stale).details, None);
    }
    let truncated = Status {
        details: Some(CameraDetails {
            observations: vec![],
            truncated: true,
        }),
        ..active_capture()
    };
    assert!(merge(&truncated, &device).details.unwrap().truncated);
    assert_eq!(merge(&active_capture(), &active_device()).details, None);
}

fn active_capture() -> Status {
    Status::new(State::Active, "Camera capture observed", &["lim pipewire"])
}

fn active_device() -> Status {
    Status::new(State::Active, "Camera device accessed", &["lim device"])
}

fn unknown_pipewire() -> Status {
    Status::new(State::Unknown, "PipeWire graph unavailable", &["lim down"])
}

fn unobserved() -> Status {
    Status::new(State::Unobserved, "No capture observed", &["lim none"])
}

#[test]
fn active_beats_unknown_and_unobserved() {
    let a = merge(&unknown_pipewire(), &active_device());
    assert_eq!(a.state, State::Active);
    assert_eq!(a.evidence, "Camera device accessed");

    let b = merge(&active_capture(), &unobserved());
    assert_eq!(b.state, State::Active);
    assert_eq!(b.evidence, "Camera capture observed");

    let c = merge(&unobserved(), &active_capture());
    assert_eq!(c.state, State::Active);
}

#[test]
fn unknown_beats_unobserved() {
    assert_eq!(
        merge(&unknown_pipewire(), &unobserved()).state,
        State::Unknown
    );
    assert_eq!(
        merge(&unobserved(), &unknown_pipewire()).state,
        State::Unknown
    );
}

#[test]
fn limitations_from_both_paths_are_preserved() {
    let merged = merge(&active_capture(), &unobserved());
    assert!(merged.limitations.contains(&"lim pipewire".to_string()));
    assert!(merged.limitations.contains(&"lim none".to_string()));

    let merged = merge(&unknown_pipewire(), &active_device());
    assert!(merged.limitations.contains(&"lim down".to_string()));
    assert!(merged.limitations.contains(&"lim device".to_string()));
}

#[test]
fn pipewire_wording_wins_when_both_active() {
    let merged = merge(&active_capture(), &active_device());
    assert_eq!(merged.state, State::Active);
    assert_eq!(merged.evidence, "Camera capture observed");
}

#[test]
fn pipewire_disconnect_does_not_erase_sustained_device_access() {
    let merged = merge(&unknown_pipewire(), &active_device());
    assert_eq!(merged.state, State::Active);
    assert_eq!(merged.evidence, "Camera device accessed");
}

#[test]
fn json_line_has_version_and_camera_fields() {
    let status = Status::new(
        State::Active,
        "Camera capture observed",
        &["Only PipeWire-visible streams are observed"],
    );
    let line = status.to_json_line();
    assert_eq!(
        line,
        "{\"version\":1,\"camera\":{\"state\":\"active\",\
         \"evidence\":\"Camera capture observed\",\
         \"limitations\":[\"Only PipeWire-visible streams are observed\"]}}"
    );
}

#[test]
fn state_strings_match_the_contract() {
    assert_eq!(State::Active.as_str(), "active");
    assert_eq!(State::Unobserved.as_str(), "unobserved");
    assert_eq!(State::Unknown.as_str(), "unknown");
}

#[test]
fn evidence_is_json_escaped_and_stays_one_line() {
    let status = Status::new(State::Unknown, "say \"hi\"\n\t\\ done", &[]);
    let line = status.to_json_line();
    assert!(line.contains("\\\"hi\\\""));
    assert!(line.contains("\\n"));
    assert!(line.contains("\\t"));
    assert!(line.contains("\\\\"));
    assert!(!line.contains('\n'));
    assert_eq!(line.matches('{').count(), 2);
}

#[test]
fn oversized_status_falls_back_to_bounded_unknown() {
    let huge = "x".repeat(MAX_LINE_BYTES * 2);
    let status = Status::new(State::Active, &huge, &[]);
    let line = bounded_json_line(&status);
    assert!(line.len() <= MAX_LINE_BYTES);
    assert!(line.contains("\"state\":\"unknown\""));
    assert!(!line.contains('\n'));
}

#[test]
fn normal_status_passes_through_bounded_serialization() {
    let status = active_capture();
    assert_eq!(bounded_json_line(&status), status.to_json_line());
}
