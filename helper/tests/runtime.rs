use glint_helper::runtime::StatusStream;
use glint_helper::status::{State, Status};

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
