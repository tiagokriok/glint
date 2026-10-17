//! Coherent graph-store transitions: removal, multiple consumers, startup with
//! existing activity, and disconnect clearing prior positive evidence.

use glint_helper::camera::{
    EV_INITIALIZING, EV_PIPEWIRE_DOWN, GraphLink, GraphNode, GraphOwner, NodeState,
};
use glint_helper::observer::GraphStore;
use glint_helper::status::State;

fn v4l2(id: u32) -> GraphNode {
    GraphNode::new(id, "Video/Source", NodeState::Running).with_device_api("v4l2")
}

fn stream(id: u32) -> GraphNode {
    GraphNode::new(id, "Stream/Input/Video", NodeState::Running)
}

fn seeded_store() -> GraphStore {
    let mut store = GraphStore::new();
    store.connect();
    store.upsert_node(v4l2(2));
    store.upsert_node(stream(10));
    store.upsert_link(GraphLink::new(100, 2, 10));
    store.mark_synced();
    store
}

#[test]
fn startup_with_existing_activity_is_active_on_first_classify() {
    let store = seeded_store();
    assert_eq!(store.classify().state, State::Active);
}

#[test]
fn state_only_node_update_preserves_camera_properties() {
    let mut store = seeded_store();
    store.apply_node_update(GraphNode::new(2, "", NodeState::Suspended), true, false);
    assert_ne!(store.classify().state, State::Active);
    store.apply_node_update(GraphNode::new(2, "", NodeState::Running), true, false);
    assert_eq!(store.classify().state, State::Active);
}

#[test]
fn property_only_node_update_preserves_running_state() {
    let mut store = seeded_store();
    store.apply_node_update(
        GraphNode::new(2, "Video/Source", NodeState::Unknown).with_device_api("v4l2"),
        false,
        true,
    );
    assert_eq!(store.classify().state, State::Active);
    store.apply_node_update(
        GraphNode::new(2, "Video/Source", NodeState::Unknown),
        false,
        true,
    );
    assert_ne!(store.classify().state, State::Active);
}

#[test]
fn connected_but_unsynced_is_unknown_not_inactivity() {
    let mut store = GraphStore::new();
    store.connect();
    store.upsert_node(v4l2(2));
    store.upsert_node(stream(10));
    store.upsert_link(GraphLink::new(100, 2, 10));

    let status = store.classify();
    assert_eq!(status.state, State::Unknown);
    assert_eq!(status.evidence, EV_INITIALIZING);
}

#[test]
fn removal_of_the_capture_stream_clears_activity() {
    let mut store = seeded_store();
    store.remove_link(100);
    store.remove_node(10);
    assert_eq!(store.classify().state, State::Unobserved);
}

#[test]
fn removing_only_the_producer_is_unknown_not_active() {
    let mut store = seeded_store();
    store.remove_node(2);
    let status = store.classify();
    assert_ne!(status.state, State::Active);
    assert_eq!(status.state, State::Unknown);
}

#[test]
fn multiple_consumers_are_active() {
    let mut store = seeded_store();
    store.upsert_node(stream(11));
    store.upsert_link(GraphLink::new(101, 2, 11));
    assert_eq!(store.classify().state, State::Active);
}

#[test]
fn one_consumer_stopping_while_another_remains_stays_active() {
    let mut store = seeded_store();
    store.upsert_node(stream(11));
    store.upsert_link(GraphLink::new(101, 2, 11));

    store.remove_link(100);
    store.remove_node(10);

    assert_eq!(store.classify().state, State::Active);
}

#[test]
fn disconnect_clears_prior_positive_evidence() {
    let mut store = seeded_store();
    assert_eq!(store.classify().state, State::Active);

    store.disconnect();
    let lost = store.classify();
    assert_eq!(lost.state, State::Unknown);
    assert_eq!(lost.evidence, EV_PIPEWIRE_DOWN);
}

#[test]
fn reconnect_does_not_reuse_stale_graph_state() {
    let mut store = seeded_store();
    assert_eq!(store.classify().state, State::Active);

    store.disconnect();
    store.connect();
    store.mark_synced();

    // Fresh connection with no reported globals: unobserved, never stale active.
    assert_eq!(store.classify().state, State::Unobserved);
}

#[test]
fn client_ownership_resolves_to_screen_source() {
    let mut store = GraphStore::new();
    store.connect();
    store.upsert_client(
        7,
        GraphOwner::new("xdg-desktop-portal-hyprland", "xdg-desktop-portal-hyprland"),
    );
    store.upsert_node(GraphNode::new(2, "Video/Source", NodeState::Running).with_client(7));
    store.upsert_node(stream(10));
    store.upsert_link(GraphLink::new(100, 2, 10));
    store.mark_synced();

    assert_eq!(store.classify().state, State::Unobserved);
}
