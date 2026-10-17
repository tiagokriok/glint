//! Pure camera-graph classification. No camera is opened; all graphs are fixtures.

use glint_helper::camera::{
    CAMERA_LIMITATIONS, CameraGraph, EV_CAPTURE, EV_PIPEWIRE_DOWN, EV_UNCLASSIFIED, EV_UNOBSERVED,
    GraphLink, GraphNode, GraphOwner, NodeState, classify,
};
use glint_helper::status::State;

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
