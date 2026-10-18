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

fn labeled_source(id: u32, label: &str) -> GraphNode {
    GraphNode {
        description: Some(label.into()),
        nick: Some(format!("{label} nick")),
        video_path: Some(format!("/dev/video{id}")),
        ..v4l2(id)
    }
}

#[test]
fn metadata_masks_preserve_unrelated_fields_and_explicit_removal_clears_them() {
    let mut store = seeded_store();
    let source = labeled_source(2, "Camera A");
    store.upsert_node(source.clone());
    store.apply_node_update(GraphNode::new(2, "", NodeState::Unknown), false, false);
    assert_eq!(store.graph().nodes[0], source);
    store.apply_node_update(GraphNode::new(2, "", NodeState::Running), true, false);
    assert_eq!(store.graph().nodes[0], source);
    store.apply_node_update(v4l2(2), false, true);
    let cleared = store.graph().nodes[0].clone();
    assert_eq!(cleared.state, NodeState::Running);
    assert_eq!(
        (cleared.description, cleared.nick, cleared.video_path),
        (None, None, None)
    );
    store.upsert_client(7, GraphOwner::new("call", "Call A"));
    store.apply_client_update(7, GraphOwner::default(), false);
    assert_eq!(store.graph().clients[&7].name, "Call A");
    store.apply_client_update(7, GraphOwner::default(), true);
    assert_eq!(store.graph().clients[&7], GraphOwner::default());
    store.apply_client_update(8, GraphOwner::new("ignored", "Ignored"), false);
    assert!(!store.graph().clients.contains_key(&8));
}

#[test]
fn individual_property_removals_step_from_description_to_nick_to_unknown() {
    let mut store = seeded_store();
    store.upsert_node(labeled_source(2, "Camera A"));
    let without_description = GraphNode {
        description: None,
        ..labeled_source(2, "Camera A")
    };
    store.apply_node_update(without_description.clone(), false, true);
    assert_eq!(
        store.classify().details.unwrap().observations[0]
            .device
            .as_deref(),
        Some("Camera A nick")
    );
    store.apply_node_update(
        GraphNode {
            nick: None,
            ..without_description
        },
        false,
        true,
    );
    let row = store.classify().details.unwrap().observations.remove(0);
    assert_eq!(row.device, None);
    assert_eq!(row.device_path.as_deref(), Some("/dev/video2"));
    store.apply_node_update(v4l2(2), false, true);
    assert_eq!(
        store.classify().details.unwrap().observations[0].device_path,
        None
    );
}

#[test]
fn concurrent_pairs_and_removals_never_retain_old_labels() {
    let mut store = seeded_store();
    store.upsert_node(labeled_source(2, "Camera A"));
    store.upsert_node(stream(10).with_client(7));
    store.upsert_client(7, GraphOwner::new("a", "Call A"));
    store.upsert_node(labeled_source(3, "Camera B"));
    store.upsert_node(stream(11).with_client(8));
    store.upsert_client(8, GraphOwner::new("b", "Call B"));
    store.upsert_link(GraphLink::new(101, 3, 11));
    let rows = store.classify().details.unwrap().observations;
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (rows[0].application.as_deref(), rows[0].device.as_deref()),
        (Some("Call A"), Some("Camera A"))
    );
    assert_eq!(
        (rows[1].application.as_deref(), rows[1].device.as_deref()),
        (Some("Call B"), Some("Camera B"))
    );
    store.remove_client(7);
    assert_eq!(
        store.classify().details.unwrap().observations[0].application,
        None
    );
    store.remove_node(10);
    assert_eq!(store.classify().details.unwrap().observations.len(), 1);
    store.remove_link(101);
    assert!(store.classify().details.is_none());
    store.upsert_link(GraphLink::new(101, 3, 11));
    store.remove_node(3);
    assert!(store.classify().details.is_none());
    store.disconnect();
    assert!(store.classify().details.is_none());
    store.connect();
    store.mark_synced();
    assert!(store.classify().details.is_none());
}
