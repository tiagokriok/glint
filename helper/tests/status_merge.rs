//! Merge precedence and JSON-lines serialization.

use glint_helper::status::{MAX_LINE_BYTES, State, Status, bounded_json_line, merge};

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
