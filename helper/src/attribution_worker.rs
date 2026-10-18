//! One enrichment thread, one replaceable pending job and one result slot.
//! Generations and holder identities remain internal, never public status data.

use std::collections::BTreeSet;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::attribution::{CameraDetails, project};
use crate::device_access::{DeviceReconciler, EV_ACCESSED};
use crate::device_metadata::{FsMetadataIo, JOB_BUDGET, MetadataIo, read_device_with};
use crate::process_identity::{observe_holder_with, unknown_row};
use crate::status::{State, Status};

pub const MAX_HOLDERS: usize = 16;

#[derive(Debug, Clone)]
pub struct AttributionJob {
    pub generation: u64,
    pub holders: BTreeSet<u32>,
    /// Already-discovered paths, not pre-read identities: all I/O is on worker.
    pub devices: Vec<PathBuf>,
}

pub struct AttributionResult {
    pub generation: u64,
    pub details: CameraDetails,
}

pub fn enrich_job(proc_root: &Path, sys_root: &Path, job: &AttributionJob) -> CameraDetails {
    enrich_job_with(&FsMetadataIo, proc_root, sys_root, job)
}

pub fn enrich_job_with(
    io: &dyn MetadataIo,
    proc_root: &Path,
    sys_root: &Path,
    job: &AttributionJob,
) -> CameraDetails {
    let deadline = io.now() + JOB_BUDGET;
    let mut devices = Vec::new();
    let mut truncated = job.holders.len() > MAX_HOLDERS || job.devices.len() > 32;
    for path in job.devices.iter().take(32) {
        if io.now() >= deadline {
            truncated = true;
            break;
        }
        let device = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| read_device_with(io, sys_root, path.parent()?, name, deadline));
        if let Some(device) = device {
            devices.push(device);
        }
    }
    let mut rows = Vec::new();
    for pid in job.holders.iter().take(MAX_HOLDERS) {
        if io.now() >= deadline {
            truncated = true;
            rows.push(unknown_row());
            break;
        }
        let holder = observe_holder_with(io, proc_root, *pid, &devices, deadline);
        rows.extend(holder.observations);
        truncated |= holder.truncated;
    }
    if rows.is_empty() {
        rows.push(unknown_row());
    }
    let details = project(&rows);
    CameraDetails {
        truncated: truncated || details.truncated || io.now() >= deadline,
        ..details
    }
}

#[derive(Default)]
struct Slots {
    pending: Option<AttributionJob>,
    result: Option<AttributionResult>,
    stopped: bool,
}

pub struct AttributionWorker {
    shared: Arc<(Mutex<Slots>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl AttributionWorker {
    pub fn new(
        mut enrich: impl FnMut(AttributionJob) -> CameraDetails + Send + 'static,
    ) -> io::Result<Self> {
        let shared = Arc::new((Mutex::new(Slots::default()), Condvar::new()));
        let worker = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("camera-attribution".into())
            .spawn(move || {
                loop {
                    let job = {
                        let (lock, ready) = &*worker;
                        let Ok(slots) = lock.lock() else {
                            return;
                        };
                        let Ok(mut slots) =
                            ready.wait_while(slots, |s| !s.stopped && s.pending.is_none())
                        else {
                            return;
                        };
                        if slots.stopped {
                            return;
                        }
                        slots.pending.take().expect("pending job after wake")
                    };
                    let generation = job.generation;
                    let details = catch_unwind(AssertUnwindSafe(|| enrich(job)))
                        .unwrap_or_else(|_| project(&[unknown_row()]));
                    let Ok(mut slots) = worker.0.lock() else {
                        return;
                    };
                    if slots.stopped {
                        return;
                    }
                    slots.result = Some(AttributionResult {
                        generation,
                        details,
                    });
                }
            })?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }

    pub fn submit(&self, job: AttributionJob) {
        if let Ok(mut slots) = self.shared.0.lock()
            && !slots.stopped
        {
            slots.pending = Some(job);
            self.shared.1.notify_one();
        }
    }

    pub fn clear_pending(&self) {
        if let Ok(mut slots) = self.shared.0.lock() {
            slots.pending = None;
            slots.result = None;
        }
    }

    pub fn take_result(&self) -> Option<AttributionResult> {
        self.shared.0.lock().ok()?.result.take()
    }
}

impl Drop for AttributionWorker {
    fn drop(&mut self) {
        if let Ok(mut slots) = self.shared.0.lock() {
            slots.stopped = true;
            slots.pending = None;
            slots.result = None;
            self.shared.1.notify_one();
        }
        // Filesystem calls are not cancellable. Never indefinitely join one;
        // a detached thread cannot outlive the helper process/lifetime guard.
        if let Some(thread) = self.thread.take()
            && thread.is_finished()
        {
            let _ = thread.join();
        }
    }
}

type SourceKey = (BTreeSet<u32>, Vec<PathBuf>);

pub struct AttributionSession {
    generation: u64,
    key: Option<SourceKey>,
    status: Status,
}

impl Default for AttributionSession {
    fn default() -> Self {
        Self {
            generation: 0,
            key: None,
            status: DeviceReconciler::new().status(Duration::ZERO),
        }
    }
}

impl AttributionSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pure reconciliation: callers must publish `status()` before submitting
    /// the returned job. Refresh generations even for unchanged holder/path sets.
    pub fn reconcile(
        &mut self,
        reconciler: &DeviceReconciler,
        now: Duration,
        refreshed: bool,
    ) -> Option<AttributionJob> {
        let status = reconciler.status(now);
        let key = (status.state == State::Active
            && status.evidence == EV_ACCESSED
            && !reconciler.snapshot().holders.is_empty())
        .then(|| {
            (
                reconciler.snapshot().holders.clone(),
                reconciler.devices().to_vec(),
            )
        });
        let changed = self.key != key;
        let details = if !changed && key.is_some() {
            self.status.details.clone()
        } else {
            None
        };
        self.status = Status { details, ..status };
        if !changed && !refreshed {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.key = key;
        self.key.as_ref().map(|(holders, devices)| AttributionJob {
            generation: self.generation,
            holders: holders.clone(),
            devices: devices.clone(),
        })
    }

    pub fn status(&self) -> Status {
        self.status.clone()
    }

    pub fn accept(&mut self, result: AttributionResult) -> bool {
        if result.generation != self.generation
            || self.key.is_none()
            || self.status.state != State::Active
            || self.status.evidence != EV_ACCESSED
        {
            return false;
        }
        let details = project(&result.details.observations);
        self.status.details = Some(CameraDetails {
            truncated: details.truncated || result.details.truncated,
            ..details
        });
        true
    }
}
