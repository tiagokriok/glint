//! Pure camera-graph classification. No camera is opened; all graphs are fixtures.

use glint_helper::attribution::{ApplicationSource, EvidenceKind};
use glint_helper::camera::{
    CAMERA_LIMITATIONS, CameraGraph, EV_CAPTURE, EV_PIPEWIRE_DOWN, EV_UNCLASSIFIED, EV_UNOBSERVED,
    GraphLink, GraphNode, GraphOwner, NodeState, classify,
};
use glint_helper::status::State;

fn attributed_graph() -> CameraGraph {
    let source = GraphNode {
        description: Some(" Camera description ".into()),
        nick: Some("Camera nick".into()),
        video_path: Some("/dev/video2".into()),
        ..v4l2_source(2, NodeState::Running).with_owner("wireplumber", "WirePlumber")
    };
    CameraGraph::new(true)
        .node(source)
        .node(video_capture_stream(10, NodeState::Running).with_client(7))
        .client(7, GraphOwner::new("/usr/bin/call", "Example Call"))
        .link(GraphLink::new(100, 2, 10))
}

#[test]
fn capture_details_use_consumer_not_source_owner_and_description_before_nick() {
    let status = classify(&attributed_graph());
    let rows = status.details.unwrap().observations;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].application.as_deref(), Some("Example Call"));
    assert_eq!(
        rows[0].application_source,
        ApplicationSource::PipewireReported
    );
    assert_eq!(rows[0].device.as_deref(), Some("Camera description"));
    assert_eq!(rows[0].device_path.as_deref(), Some("/dev/video2"));
    assert_eq!(rows[0].evidence, EvidenceKind::PipewireCapture);
}

#[test]
fn reported_labels_are_not_guessed_and_binary_fallback_is_a_basename() {
    for (owner, expected) in [
        (
            GraphOwner::new("browser", "xdg-desktop-portal"),
            Some("xdg-desktop-portal"),
        ),
        (
            GraphOwner::new("/usr/bin/example-call", "\n\u{202e}"),
            Some("example-call"),
        ),
        (GraphOwner::default(), None),
    ] {
        let graph = CameraGraph {
            clients: [(7, owner)].into(),
            ..attributed_graph()
        };
        let row = classify(&graph).details.unwrap().observations.remove(0);
        assert_eq!(row.application.as_deref(), expected);
        assert_eq!(
            row.application_source,
            if expected.is_some() {
                ApplicationSource::PipewireReported
            } else {
                ApplicationSource::Unknown
            }
        );
    }
}

#[test]
fn missing_client_does_not_reuse_node_or_source_identity() {
    let mut graph = attributed_graph();
    graph.clients.clear();
    graph.nodes[1].owner = GraphOwner::new("stale", "Stale client");
    assert_eq!(
        classify(&graph).details.unwrap().observations[0].application,
        None
    );
    graph.nodes[1].client_id = None;
    assert_eq!(
        classify(&graph).details.unwrap().observations[0]
            .application
            .as_deref(),
        Some("Stale client")
    );
}

#[test]
fn device_fallback_and_path_validation_never_guess_from_node_name() {
    for (description, nick, path, expected_device, expected_path) in [
        (
            Some("\n"),
            Some(" Nick "),
            "/dev/video0",
            Some("Nick"),
            Some("/dev/video0"),
        ),
        (None, None, "/dev/video9", None, Some("/dev/video9")),
        (None, None, "/dev/video", None, None),
        (None, None, "/dev/video0/../video1", None, None),
        (None, None, "video0", None, None),
    ] {
        let mut graph = attributed_graph();
        graph.nodes[0].description = description.map(str::to_owned);
        graph.nodes[0].nick = nick.map(str::to_owned);
        graph.nodes[0].video_path = Some(path.into());
        graph.nodes[0].node_name = "/dev/video123".into();
        let row = classify(&graph).details.unwrap().observations.remove(0);
        assert_eq!(row.device.as_deref(), expected_device);
        assert_eq!(row.device_path.as_deref(), expected_path);
    }
}

#[test]
fn non_capture_graphs_never_have_details() {
    for mode in 0..6 {
        let mut graph = attributed_graph();
        match mode {
            0 => graph.nodes[0].state = NodeState::Suspended,
            1 => graph.nodes[1].state = NodeState::Suspended,
            2 => graph.links.clear(),
            3 => graph.links[0].active = false,
            4 => {
                graph.nodes.remove(1);
            }
            _ => {
                graph.nodes[0].device_api = None;
                graph.nodes[0].owner = GraphOwner::new("xdg-desktop-portal", "portal");
            }
        };
        assert!(classify(&graph).details.is_none());
    }
}

#[test]
fn active_pair_projection_caps_rows_and_sanitizes_labels_without_cross_pairing() {
    let source_a = GraphNode {
        description: Some(format!("\u{202e}{}\n", "é".repeat(60))),
        video_path: Some("/dev/video1".into()),
        ..v4l2_source(1, NodeState::Running)
    };
    let source_b = GraphNode {
        description: Some("Second camera".into()),
        video_path: Some("/dev/video2".into()),
        ..v4l2_source(2, NodeState::Running)
    };
    let mut graph = CameraGraph::new(true).node(source_a).node(source_b);
    for id in 10..45 {
        graph = graph
            .node(
                video_capture_stream(id, NodeState::Running)
                    .with_owner("call", &format!("Call {id}")),
            )
            .link(GraphLink::new(
                id + 100,
                if id.is_multiple_of(2) { 1 } else { 2 },
                id,
            ));
    }
    graph = graph.link(GraphLink::new(999, 1, 10));
    let details = classify(&graph).details.unwrap();
    assert!(details.truncated);
    assert_eq!(details.observations.len(), 32);
    for row in details.observations {
        let id: u32 = row
            .application
            .unwrap()
            .strip_prefix("Call ")
            .unwrap()
            .parse()
            .unwrap();
        if id.is_multiple_of(2) {
            assert_eq!(row.device_path.as_deref(), Some("/dev/video1"));
            assert_eq!(row.device.as_deref(), Some("é".repeat(40).as_str()));
        } else {
            assert_eq!(row.device_path.as_deref(), Some("/dev/video2"));
            assert_eq!(row.device.as_deref(), Some("Second camera"));
        }
    }
}

fn v4l2_source(id: u32, state: NodeState) -> GraphNode {
    GraphNode::new(id, "Video/Source", state).with_device_api("v4l2")
}

fn video_capture_stream(id: u32, state: NodeState) -> GraphNode {
    GraphNode::new(id, "Stream/Input/Video", state)
}

#[test]
fn running_v4l2_camera_linked_to_capture_consumer_is_active() {
    let graph = CameraGraph::new(true)
        .node(v4l2_source(2, NodeState::Running))
        .node(video_capture_stream(10, NodeState::Running))
        .link(GraphLink::new(100, 2, 10));

    let status = classify(&graph);
    assert_eq!(status.state, State::Active);
    assert_eq!(status.evidence, EV_CAPTURE);
    assert_eq!(
        status.limitations,
        CAMERA_LIMITATIONS.map(String::from).to_vec()
    );
}

#[test]
fn hardware_presence_without_a_stream_is_not_active() {
    let graph = CameraGraph::new(true).node(v4l2_source(2, NodeState::Suspended));
    let status = classify(&graph);
    assert_ne!(status.state, State::Active);
    assert_eq!(status.state, State::Unobserved);
}

#[test]
fn suspended_source_is_not_active() {
    let graph = CameraGraph::new(true)
        .node(v4l2_source(2, NodeState::Suspended))
        .node(video_capture_stream(10, NodeState::Suspended))
        .link(GraphLink::new(100, 2, 10));

    assert_ne!(classify(&graph).state, State::Active);
}

#[test]
fn playback_output_stream_is_not_active() {
    let graph = CameraGraph::new(true)
        .node(v4l2_source(2, NodeState::Running))
        .node(GraphNode::new(
            10,
            "Stream/Output/Video",
            NodeState::Running,
        ))
        .link(GraphLink::new(100, 2, 10));

    assert_eq!(classify(&graph).state, State::Unobserved);
}

#[test]
fn portal_screen_source_is_not_active() {
    let graph = CameraGraph::new(true)
        .node(
            GraphNode::new(2, "Video/Source", NodeState::Running)
                .with_client(7)
                .with_node_name("xdph-streaming-1"),
        )
        .node(video_capture_stream(10, NodeState::Running))
        .link(GraphLink::new(100, 2, 10))
        .client(
            7,
            GraphOwner::new("xdg-desktop-portal-hyprland", "xdg-desktop-portal-hyprland"),
        );

    assert_eq!(classify(&graph).state, State::Unobserved);
}

#[test]
fn unlinked_capture_stream_is_not_active() {
    let graph = CameraGraph::new(true).node(video_capture_stream(10, NodeState::Running));
    let status = classify(&graph);
    assert_ne!(status.state, State::Active);
    assert_eq!(status.state, State::Unknown);
    assert_eq!(status.evidence, EV_UNCLASSIFIED);
}

#[test]
fn incomplete_metadata_is_unknown() {
    // A link references a producer node that was never reported.
    let graph = CameraGraph::new(true)
        .node(video_capture_stream(10, NodeState::Running))
        .link(GraphLink::new(100, 999, 10));

    let status = classify(&graph);
    assert_eq!(status.state, State::Unknown);
    assert_eq!(status.evidence, EV_UNCLASSIFIED);
}

#[test]
fn unavailable_graph_is_unknown() {
    let graph = CameraGraph::new(false);
    let status = classify(&graph);
    assert_eq!(status.state, State::Unknown);
    assert_eq!(status.evidence, EV_PIPEWIRE_DOWN);
}

#[test]
fn empty_available_graph_is_unobserved() {
    let status = classify(&CameraGraph::new(true));
    assert_eq!(status.state, State::Unobserved);
    assert_eq!(status.evidence, EV_UNOBSERVED);
}

#[test]
fn suspended_producer_cannot_prove_capture_even_when_consumer_runs() {
    let graph = CameraGraph::new(true)
        .node(v4l2_source(2, NodeState::Suspended))
        .node(video_capture_stream(10, NodeState::Running))
        .link(GraphLink::new(100, 2, 10));
    assert_ne!(classify(&graph).state, State::Active);
}

#[test]
fn inactive_link_cannot_prove_capture() {
    let graph = CameraGraph::new(true)
        .node(v4l2_source(2, NodeState::Running))
        .node(video_capture_stream(10, NodeState::Running))
        .link(GraphLink {
            active: false,
            ..GraphLink::new(100, 2, 10)
        });
    assert_ne!(classify(&graph).state, State::Active);
}
