//! Bounded `/dev/video*` device-holder reconciliation and sustained-access
//! classification.
//!
//! Direct device access is weaker evidence than a linked PipeWire capture
//! stream: a holder only counts as active after it has stayed present for a
//! debounce window. Holder IDs remain internal and may feed metadata-only
//! enrichment; they never enter the public status schema.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};

use crate::process;
use crate::status::{State, Status};

/// A holder must persist this long before it counts as active access.
pub const DEBOUNCE: Duration = Duration::from_millis(750);
/// Minimum spacing between two reconciliations.
pub const MIN_INTERVAL: Duration = Duration::from_millis(1000);
/// Discovery/reconciliation fallback for hotplug and lost watchers.
pub const FALLBACK_INTERVAL: Duration = Duration::from_millis(5000);
/// Timeout for one bounded holder lookup command.
pub const HOLDER_TIMEOUT: Duration = Duration::from_millis(2000);
/// Timeout for reaping a terminated holder-command child.
pub const REAP_TIMEOUT: Duration = Duration::from_millis(2000);

pub const EV_ACCESSED: &str = "Camera device accessed";
pub const EV_NOT_SUSTAINED: &str = "Camera device access not sustained";
pub const EV_NO_HOLDERS: &str = "No camera device holders observed";
pub const EV_LOOKUP_UNAVAILABLE: &str = "Camera device holder lookup unavailable";

pub const LIM_NOT_STREAMING: &str = "Direct device access is not confirmed streaming";
pub const LIM_HIDDEN_HOLDERS: &str = "Holders invisible to this user are not observed";
pub const LIM_BROWSER: &str = "Internal browser-tab capture is not supported";
pub const DEVICE_LIMITATIONS: [&str; 3] = [LIM_NOT_STREAMING, LIM_HIDDEN_HOLDERS, LIM_BROWSER];

const VIDEO_DIR: &str = "/dev";

/// Immutable holder snapshot; `stable_since` is a monotonic process-local time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessSnapshot {
    pub holders: BTreeSet<u32>,
    pub stable_since: Option<Duration>,
    pub available: bool,
}

impl AccessSnapshot {
    /// Nothing observed yet, lookup available.
    pub fn empty() -> AccessSnapshot {
        AccessSnapshot {
            holders: BTreeSet::new(),
            stable_since: None,
            available: true,
        }
    }

    /// Lookup failed: no claim of inactivity.
    pub fn unavailable() -> AccessSnapshot {
        AccessSnapshot {
            holders: BTreeSet::new(),
            stable_since: None,
            available: false,
        }
    }
}

/// Fold a new holder observation into the previous snapshot.
///
/// A changed holder set, or a changed lookup availability, restarts the
/// stability clock; an unchanged observation leaves it running.
pub fn advance(
    previous: &AccessSnapshot,
    holders: BTreeSet<u32>,
    available: bool,
    now: Duration,
) -> AccessSnapshot {
    if holders == previous.holders && available == previous.available {
        return previous.clone();
    }
    let stable_since = if available && !holders.is_empty() {
        Some(now)
    } else {
        None
    };
    AccessSnapshot {
        holders,
        stable_since,
        available,
    }
}

/// Classify a holder snapshot at `now`.
pub fn classify(snapshot: &AccessSnapshot, now: Duration) -> Status {
    if !snapshot.available {
        return Status::new(State::Unknown, EV_LOOKUP_UNAVAILABLE, &DEVICE_LIMITATIONS);
    }
    if snapshot.holders.is_empty() {
        return Status::new(State::Unobserved, EV_NO_HOLDERS, &DEVICE_LIMITATIONS);
    }
    match snapshot.stable_since {
        Some(since) if now.saturating_sub(since) >= DEBOUNCE => {
            Status::new(State::Active, EV_ACCESSED, &DEVICE_LIMITATIONS)
        }
        _ => Status::new(State::Unobserved, EV_NOT_SUSTAINED, &DEVICE_LIMITATIONS),
    }
}

/// A normalized inotify event kind for device watches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchEventKind {
    Open,
    Close,
    Created,
    Removed,
    Overflow,
    WatchLost,
    Other,
}

impl WatchEventKind {
    pub fn from_mask(mask: EventMask) -> WatchEventKind {
        if mask.contains(EventMask::Q_OVERFLOW) {
            WatchEventKind::Overflow
        } else if mask.contains(EventMask::IGNORED) {
            WatchEventKind::WatchLost
        } else if mask.contains(EventMask::DELETE_SELF) || mask.contains(EventMask::MOVE_SELF) {
            WatchEventKind::Removed
        } else if mask.contains(EventMask::CREATE)
            || mask.contains(EventMask::DELETE)
            || mask.contains(EventMask::MOVED_TO)
            || mask.contains(EventMask::MOVED_FROM)
        {
            WatchEventKind::Created
        } else if mask.contains(EventMask::OPEN) {
            WatchEventKind::Open
        } else if mask.contains(EventMask::CLOSE_WRITE) || mask.contains(EventMask::CLOSE_NOWRITE) {
            WatchEventKind::Close
        } else {
            WatchEventKind::Other
        }
    }

    /// Whether this event should trigger a bounded reconciliation.
    pub fn should_reconcile(self) -> bool {
        !matches!(self, WatchEventKind::Other)
    }
}

/// Whether a directory entry name is a V4L2 video device node.
pub fn is_video_device(name: &str) -> bool {
    match name.strip_prefix("video") {
        Some(rest) => !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit()),
        None => false,
    }
}

/// List sorted device paths, preserving discovery failure as unavailable.
pub fn try_discover_devices(dir: &Path) -> Option<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        if let Some(name) = entry.file_name().to_str()
            && is_video_device(name)
        {
            found.push(entry.path());
        }
    }
    found.sort();
    Some(found)
}

/// Parse holder process ids out of a `fuser`-style whitespace-separated output.
pub fn parse_holder_ids(output: &str) -> BTreeSet<u32> {
    output
        .split_whitespace()
        .filter_map(|token| token.parse::<u32>().ok())
        .collect()
}

/// Look up current holders of `devices`, or `None` when lookup is unavailable.
///
/// `fuser` return codes 0 and 1 are both valid answers (holders / none).
pub fn lookup_holders(devices: &[PathBuf]) -> Option<BTreeSet<u32>> {
    if devices.is_empty() {
        return Some(BTreeSet::new());
    }
    let mut argv: Vec<&str> = Vec::with_capacity(devices.len() + 1);
    argv.push("fuser");
    for device in devices {
        argv.push(device.to_str()?);
    }
    let output = process::run_capture_with_timeout(&argv, HOLDER_TIMEOUT)?;
    if output.status != 0 && output.status != 1 {
        return None;
    }
    Some(parse_holder_ids(&output.stdout))
}

/// Scheduling and hold state for the bounded reconciliation loop.
#[derive(Debug, Clone)]
pub struct DeviceReconciler {
    pub debounce: Duration,
    pub min_interval: Duration,
    pub fallback_interval: Duration,
    devices: Vec<PathBuf>,
    snapshot: AccessSnapshot,
    last_reconcile: Option<Duration>,
    next_reconcile: Option<Duration>,
    next_fallback: Option<Duration>,
}

impl Default for DeviceReconciler {
    fn default() -> Self {
        DeviceReconciler {
            debounce: DEBOUNCE,
            min_interval: MIN_INTERVAL,
            fallback_interval: FALLBACK_INTERVAL,
            devices: Vec::new(),
            snapshot: AccessSnapshot::empty(),
            last_reconcile: None,
            // Due immediately so the first poll publishes a coherent state.
            next_reconcile: Some(Duration::ZERO),
            next_fallback: None,
        }
    }
}

impl DeviceReconciler {
    pub fn new() -> DeviceReconciler {
        DeviceReconciler::default()
    }

    pub fn snapshot(&self) -> &AccessSnapshot {
        &self.snapshot
    }

    pub fn devices(&self) -> &[PathBuf] {
        &self.devices
    }

    /// Schedule a reconciliation, respecting the minimum spacing.
    pub fn request(&mut self, now: Duration) {
        if self.last_reconcile.is_some_and(|last| now > last)
            && !self.snapshot.holders.is_empty()
            && classify(&self.snapshot, self.last_reconcile.unwrap_or(now)).state != State::Active
        {
            self.snapshot = AccessSnapshot {
                stable_since: Some(now),
                ..self.snapshot.clone()
            };
        }
        let earliest = match self.last_reconcile {
            None => now,
            Some(last) => now.max(last + self.min_interval),
        };
        if self.next_reconcile.is_none_or(|next| earliest < next) {
            self.next_reconcile = Some(earliest);
        }
    }

    /// Earliest absolute time the loop must wake for, or `None`.
    pub fn next_deadline(&self, _now: Duration) -> Option<Duration> {
        min_opt(self.next_reconcile, self.next_fallback)
    }

    pub fn status(&self, now: Duration) -> Status {
        // Elapsed wall time alone cannot confirm a holder is still present.
        classify(&self.snapshot, self.last_reconcile.unwrap_or(now))
    }

    /// Run any due work. Returns whether a reconciliation ran.
    pub fn poll(
        &mut self,
        now: Duration,
        discover: &mut dyn FnMut() -> Option<Vec<PathBuf>>,
        lookup: &mut dyn FnMut(&[PathBuf]) -> Option<BTreeSet<u32>>,
    ) -> bool {
        let due_reconcile = self.next_reconcile.is_some_and(|next| now >= next);
        let due_fallback = self.next_fallback.is_some_and(|next| now >= next);
        if !due_reconcile && !due_fallback {
            return false;
        }

        self.last_reconcile = Some(now);
        self.next_reconcile = None;
        let discovered = discover();
        let available = discovered.is_some();
        self.devices = discovered.unwrap_or_default();

        let holders = if !available {
            None
        } else if self.devices.is_empty() {
            Some(BTreeSet::new())
        } else {
            lookup(&self.devices)
        };
        let (holders, available) = match holders {
            Some(holders) => (holders, true),
            None => (BTreeSet::new(), false),
        };
        self.snapshot = advance(&self.snapshot, holders, available, now);
        self.next_fallback = Some(now + self.fallback_interval);
        if self
            .snapshot
            .stable_since
            .is_some_and(|since| now.saturating_sub(since) < self.debounce)
        {
            self.request(now);
        }
        true
    }
}

fn min_opt(a: Option<Duration>, b: Option<Duration>) -> Option<Duration> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

/// Native inotify watcher for camera device open/close and hotplug events.
///
/// The watcher is best-effort: when inotify is unavailable the reconciliation
/// fallback still covers discovery, and every failed check stays `unknown`.
pub struct DeviceWatch {
    root: PathBuf,
    inotify: Option<Inotify>,
    buffer: Vec<u8>,
    watched: BTreeMap<PathBuf, WatchDescriptor>,
    directory: Option<(PathBuf, WatchDescriptor)>,
    resync: bool,
}

impl Default for DeviceWatch {
    fn default() -> Self {
        Self::for_directory(Path::new(VIDEO_DIR))
    }
}

impl DeviceWatch {
    pub fn for_directory(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            inotify: Inotify::init().ok(),
            buffer: vec![0; 16 * 1024],
            watched: BTreeMap::new(),
            directory: None,
            resync: false,
        }
    }

    pub fn new() -> DeviceWatch {
        DeviceWatch::default()
    }

    /// Add or remove watches so they match the current device list.
    pub fn sync(&mut self, devices: &[PathBuf]) {
        let Some(inotify) = self.inotify.as_ref() else {
            return;
        };
        let mut watches = inotify.watches();

        if self.resync {
            for wd in std::mem::take(&mut self.watched).into_values() {
                let _ = watches.remove(wd);
            }
            self.resync = false;
        }

        let desired: BTreeSet<&PathBuf> = devices.iter().collect();
        let stale: Vec<PathBuf> = self
            .watched
            .keys()
            .filter(|path| !desired.contains(path))
            .cloned()
            .collect();
        for path in stale {
            if let Some(wd) = self.watched.remove(&path) {
                let _ = watches.remove(wd);
            }
        }

        let mask = WatchMask::OPEN
            | WatchMask::CLOSE_WRITE
            | WatchMask::CLOSE_NOWRITE
            | WatchMask::DELETE_SELF
            | WatchMask::MOVE_SELF;
        for path in devices {
            if !self.watched.contains_key(path)
                && let Ok(wd) = watches.add(path, mask)
            {
                self.watched.insert(path.clone(), wd);
            }
        }

        if self.directory.is_none() {
            let dir = self.root.clone();
            if let Ok(wd) = watches.add(&dir, WatchMask::CREATE | WatchMask::DELETE) {
                self.directory = Some((dir, wd));
            }
        }
    }

    /// Drain all pending events without blocking.
    pub fn drain(&mut self) -> Vec<WatchEventKind> {
        let Some(inotify) = self.inotify.as_mut() else {
            return Vec::new();
        };
        let mut kinds = Vec::new();
        match inotify.read_events(&mut self.buffer) {
            Ok(events) => {
                for event in events {
                    let kind = WatchEventKind::from_mask(event.mask);
                    if kind == WatchEventKind::Overflow {
                        self.resync = true;
                    }
                    if kind == WatchEventKind::WatchLost {
                        self.watched.retain(|_, wd| *wd != event.wd);
                        if self
                            .directory
                            .as_ref()
                            .is_some_and(|(_, wd)| *wd == event.wd)
                        {
                            self.directory = None;
                        }
                    }
                    kinds.push(kind);
                }
            }
            Err(error) if is_would_block(&error) => {}
            Err(_) => {}
        }
        kinds
    }
}

fn is_would_block(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::WouldBlock
}
