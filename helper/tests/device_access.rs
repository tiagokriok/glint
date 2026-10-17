//! Bounded device-holder reconciliation with fake clocks and fake results.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use glint_helper::device_access::{
    AccessSnapshot, DEBOUNCE, DeviceReconciler, EV_ACCESSED, EV_LOOKUP_UNAVAILABLE, EV_NO_HOLDERS,
    EV_NOT_SUSTAINED, FALLBACK_INTERVAL, MIN_INTERVAL, WatchEventKind, advance, classify,
    is_video_device, try_discover_devices,
};
use glint_helper::status::State;

fn holders(ids: &[u32]) -> BTreeSet<u32> {
    ids.iter().copied().collect()
}

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

#[test]
fn no_holders_is_unobserved() {
    let snapshot = advance(&AccessSnapshot::unavailable(), holders(&[]), true, ms(0));
    let status = classify(&snapshot, ms(0));
    assert_eq!(status.state, State::Unobserved);
    assert_eq!(status.evidence, EV_NO_HOLDERS);
}

#[test]
fn lookup_failure_is_unknown() {
    let snapshot = advance(&AccessSnapshot::empty(), holders(&[]), false, ms(0));
    let status = classify(&snapshot, ms(100));
    assert_eq!(status.state, State::Unknown);
    assert_eq!(status.evidence, EV_LOOKUP_UNAVAILABLE);
}

#[test]
fn stable_holders_become_active_at_the_debounce_boundary() {
    let snapshot = advance(&AccessSnapshot::empty(), holders(&[42]), true, ms(10_000));

    assert_eq!(snapshot.stable_since, Some(ms(10_000)));
    assert_eq!(classify(&snapshot, ms(10_749)).state, State::Unobserved);
    assert_eq!(classify(&snapshot, ms(10_749)).evidence, EV_NOT_SUSTAINED);
    assert_eq!(classify(&snapshot, ms(10_750)).state, State::Active);
    assert_eq!(classify(&snapshot, ms(10_750)).evidence, EV_ACCESSED);
}

#[test]
fn short_probes_never_activate() {
    let first = advance(&AccessSnapshot::empty(), holders(&[42]), true, ms(0));
    let gone = advance(&first, holders(&[]), true, ms(400));
    assert_eq!(gone.stable_since, None);
    assert_ne!(classify(&gone, ms(749)).state, State::Active);
}

#[test]
fn holder_set_change_restarts_the_debounce() {
    let first = advance(&AccessSnapshot::empty(), holders(&[42]), true, ms(0));
    let changed = advance(&first, holders(&[42, 43]), true, ms(500));

    assert_eq!(changed.stable_since, Some(ms(500)));
    assert_eq!(classify(&changed, ms(1_249)).state, State::Unobserved);
    assert_eq!(classify(&changed, ms(1_250)).state, State::Active);
}

#[test]
fn unchanged_holders_do_not_restart_the_debounce() {
    let first = advance(&AccessSnapshot::empty(), holders(&[42]), true, ms(0));
    let same = advance(&first, holders(&[42]), true, ms(500));
    assert_eq!(same.stable_since, Some(ms(0)));
    assert_eq!(classify(&same, ms(750)).state, State::Active);
}

#[test]
fn disappearance_clears_access() {
    let active = advance(&AccessSnapshot::empty(), holders(&[42]), true, ms(0));
    assert_eq!(classify(&active, ms(750)).state, State::Active);

    let gone = advance(&active, holders(&[]), true, ms(800));
    assert_eq!(gone.stable_since, None);
    assert_eq!(classify(&gone, ms(800)).state, State::Unobserved);
}

#[test]
fn constants_match_the_verified_hushlight_values() {
    assert_eq!(DEBOUNCE, ms(750));
    assert_eq!(MIN_INTERVAL, ms(1000));
    assert_eq!(FALLBACK_INTERVAL, ms(5000));
    assert_eq!(glint_helper::device_access::HOLDER_TIMEOUT, ms(2000));
    assert_eq!(glint_helper::device_access::REAP_TIMEOUT, ms(2000));
}

#[test]
fn watch_events_map_and_trigger_reconciliation() {
    use inotify::EventMask;

    assert_eq!(
        WatchEventKind::from_mask(EventMask::OPEN),
        WatchEventKind::Open
    );
    assert_eq!(
        WatchEventKind::from_mask(EventMask::CLOSE_WRITE),
        WatchEventKind::Close
    );
    assert_eq!(
        WatchEventKind::from_mask(EventMask::CREATE),
        WatchEventKind::Created
    );
    assert_eq!(
        WatchEventKind::from_mask(EventMask::DELETE_SELF),
        WatchEventKind::Removed
    );
    assert_eq!(
        WatchEventKind::from_mask(EventMask::Q_OVERFLOW),
        WatchEventKind::Overflow
    );
    assert_eq!(
        WatchEventKind::from_mask(EventMask::IGNORED),
        WatchEventKind::WatchLost
    );

    for kind in [
        WatchEventKind::Open,
        WatchEventKind::Close,
        WatchEventKind::Created,
        WatchEventKind::Removed,
        WatchEventKind::Overflow,
        WatchEventKind::WatchLost,
    ] {
        assert!(kind.should_reconcile(), "{kind:?} must trigger a reconcile");
    }
    assert!(!WatchEventKind::Other.should_reconcile());
}

#[test]
fn fuser_output_parses_to_holder_ids() {
    use glint_helper::device_access::parse_holder_ids;

    let ids = parse_holder_ids("  42\n43  \nnotapid\n");
    assert_eq!(ids, holders(&[42, 43]));
    assert_eq!(parse_holder_ids(""), holders(&[]));
}

#[test]
fn video_device_name_detection() {
    assert!(is_video_device("video0"));
    assert!(is_video_device("video10"));
    assert!(!is_video_device("video"));
    assert!(!is_video_device("video0x"));
    assert!(!is_video_device("v4l"));
    assert!(!is_video_device("card0"));
}

#[test]
fn discovery_lists_only_video_nodes_sorted() {
    let dir = unique_dir("discovery");
    fs::create_dir_all(&dir).unwrap();
    for name in ["video2", "video0", "card0", "video10"] {
        fs::write(dir.join(name), b"").unwrap();
    }

    let found = try_discover_devices(&dir).unwrap();
    assert_eq!(
        found,
        vec![dir.join("video0"), dir.join("video10"), dir.join("video2")]
    );

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn missing_discovery_directory_is_not_reported_as_no_camera() {
    assert!(
        glint_helper::device_access::try_discover_devices(&unique_dir("missing-directory"))
            .is_none()
    );
}

#[test]
fn reconciler_runs_initial_reconcile_and_schedules_fallback() {
    let mut reconciler = DeviceReconciler::new();
    let mut discover = || Some(vec![PathBuf::from("/dev/video0")]);
    let mut lookup = |_devices: &[PathBuf]| Some(holders(&[]));

    assert!(reconciler.poll(ms(0), &mut discover, &mut lookup));
    assert_eq!(reconciler.devices().len(), 1);
    assert_eq!(reconciler.next_deadline(ms(0)), Some(FALLBACK_INTERVAL));
}

#[test]
fn request_respects_the_minimum_interval() {
    let mut reconciler = DeviceReconciler::new();
    let mut discover = || Some(vec![PathBuf::from("/dev/video0")]);
    let mut lookup = |_devices: &[PathBuf]| Some(holders(&[]));

    assert!(reconciler.poll(ms(0), &mut discover, &mut lookup));
    reconciler.request(ms(100));
    assert_eq!(reconciler.next_deadline(ms(0)), Some(MIN_INTERVAL));
}

#[test]
fn debounce_deadline_is_scheduled_for_an_active_candidate() {
    let mut reconciler = DeviceReconciler::new();
    let mut discover = || Some(vec![PathBuf::from("/dev/video0")]);
    let mut lookup = |_devices: &[PathBuf]| Some(holders(&[7]));

    assert!(reconciler.poll(ms(0), &mut discover, &mut lookup));
    // A confirming lookup must also respect the 1s minimum interval.
    assert_eq!(reconciler.next_deadline(ms(0)), Some(MIN_INTERVAL));
    assert!(reconciler.poll(ms(1000), &mut discover, &mut lookup));
    assert_eq!(reconciler.status(ms(1000)).state, State::Active);
    assert_eq!(
        reconciler.next_deadline(ms(1000)),
        Some(FALLBACK_INTERVAL + ms(1000))
    );
}

#[test]
fn stale_single_sample_never_promotes_a_short_device_probe() {
    let mut reconciler = DeviceReconciler::new();
    let mut discover = || Some(vec![PathBuf::from("/dev/video0")]);
    reconciler.poll(ms(0), &mut discover, &mut |_| Some(holders(&[42])));
    reconciler.request(ms(400));
    assert_ne!(reconciler.status(ms(750)).state, State::Active);
    reconciler.poll(ms(1000), &mut discover, &mut |_| Some(holders(&[])));
    assert_eq!(reconciler.status(ms(1000)).state, State::Unobserved);
}

#[test]
fn absent_notifications_still_require_a_confirming_sample() {
    let mut reconciler = DeviceReconciler::new();
    let mut discover = || Some(vec![PathBuf::from("/dev/video0")]);
    reconciler.poll(ms(0), &mut discover, &mut |_| Some(holders(&[42])));
    assert_ne!(reconciler.status(ms(5000)).state, State::Active);
    assert_eq!(reconciler.next_deadline(ms(0)), Some(ms(1000)));
}

#[test]
fn notification_during_candidate_window_restarts_confirmation() {
    let mut reconciler = DeviceReconciler::new();
    let mut discover = || Some(vec![PathBuf::from("/dev/video0")]);
    reconciler.poll(ms(0), &mut discover, &mut |_| Some(holders(&[42])));
    reconciler.request(ms(400));
    reconciler.poll(ms(1000), &mut discover, &mut |_| Some(holders(&[42])));
    assert_ne!(reconciler.status(ms(1000)).state, State::Active);
    reconciler.poll(ms(2000), &mut discover, &mut |_| Some(holders(&[42])));
    assert_eq!(reconciler.status(ms(2000)).state, State::Active);
}

#[test]
fn lookup_failure_is_unknown_then_recovers_on_fallback() {
    let mut reconciler = DeviceReconciler::new();
    let mut discover = || Some(vec![PathBuf::from("/dev/video0")]);
    let mut fail = |_devices: &[PathBuf]| None;

    assert!(reconciler.poll(ms(0), &mut discover, &mut fail));
    assert_eq!(reconciler.status(ms(0)).state, State::Unknown);

    // The fallback deadline still fires a fresh lookup; a good answer recovers.
    let mut ok = |_devices: &[PathBuf]| Some(holders(&[]));
    assert!(reconciler.poll(FALLBACK_INTERVAL, &mut discover, &mut ok));
    assert_eq!(
        reconciler.status(FALLBACK_INTERVAL).state,
        State::Unobserved
    );
}

#[test]
fn hotplug_device_set_change_is_reconciled() {
    let mut reconciler = DeviceReconciler::new();
    let devices = std::cell::RefCell::new(vec![PathBuf::from("/dev/video0")]);
    let mut discover = || Some(devices.borrow().clone());
    let mut lookup = |_devices: &[PathBuf]| Some(holders(&[]));

    assert!(reconciler.poll(ms(0), &mut discover, &mut lookup));
    assert_eq!(reconciler.devices().len(), 1);

    devices.borrow_mut().push(PathBuf::from("/dev/video2"));
    assert!(reconciler.poll(FALLBACK_INTERVAL, &mut discover, &mut lookup));
    assert_eq!(reconciler.devices().len(), 2);
}

fn unique_dir(tag: &str) -> PathBuf {
    let mut dir = PathBuf::from("/tmp/opencode");
    dir.push(format!(
        "glint-helper-test-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    dir
}

#[test]
fn native_watcher_observes_only_our_fixture_file_and_recovers_removed_watches() {
    use glint_helper::device_access::DeviceWatch;
    let dir = unique_dir("watch");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("video0");
    fs::write(&path, b"fixture").unwrap();
    let mut watch = DeviceWatch::for_directory(&dir);
    watch.sync(std::slice::from_ref(&path));
    drop(fs::File::open(&path).unwrap());
    let events = watch.drain();
    assert!(events.contains(&WatchEventKind::Open));
    assert!(events.contains(&WatchEventKind::Close));
    fs::write(dir.join("video1"), b"fixture").unwrap();
    assert!(watch.drain().contains(&WatchEventKind::Created));
    watch.sync(&[]);
    watch.drain();
    watch.sync(&[]);
    assert!(
        watch.drain().is_empty(),
        "removed watches must not create a resync loop"
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn holder_lookup_uses_only_a_regular_fixture_file() {
    use glint_helper::device_access::lookup_holders;
    let dir = unique_dir("fuser");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("video0");
    fs::write(&path, b"fixture").unwrap();
    let handle = fs::File::open(&path).unwrap();
    assert!(
        lookup_holders(std::slice::from_ref(&path))
            .unwrap()
            .contains(&std::process::id())
    );
    drop(handle);
    assert!(
        !lookup_holders(std::slice::from_ref(&path))
            .unwrap()
            .contains(&std::process::id())
    );
    assert!(lookup_holders(&[]).unwrap().is_empty());
    fs::remove_dir_all(dir).unwrap();
}
