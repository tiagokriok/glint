use crate::attribution::{
    ApplicationSource, CameraDetails, EvidenceKind, Observation, project, valid_path,
};
use crate::attribution_worker::{AttributionJob, AttributionResult};
use crate::device_metadata::{JOB_BUDGET, MetadataIo, read_device_with};
use crate::device_runtime::AttributionQueue;
use crate::status::{self, State, Status};
use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;

/// Ordered production publication seam. Optional enrichment cannot delay the
/// initial capture status; all worker results pass the pure generation gate.
pub struct PipewirePublisher<Q> {
    sink: SyncSender<Status>,
    details: PipewireDetails,
    worker: Option<Q>,
}

impl<Q: AttributionQueue> PipewirePublisher<Q> {
    pub fn new(sink: SyncSender<Status>, worker: Option<Q>) -> Self {
        Self {
            sink,
            details: PipewireDetails::default(),
            worker,
        }
    }

    pub fn publish(&mut self, status: Status) {
        let job = self.details.update(status.clone());
        if self.sink.send(self.details.status()).is_err() {
            return;
        }
        if let Some(worker) = &self.worker {
            if let Some(job) = job {
                worker.submit(job);
            } else if status.state != State::Active {
                worker.clear_pending();
            }
        }
    }

    pub fn poll(&mut self) {
        if let Some(result) = self.worker.as_ref().and_then(|worker| worker.take_result())
            && self.details.accept(result)
        {
            let _ = self.sink.send(self.details.status());
        }
    }
}

/// Pure generation gate for optional PipeWire device-name enrichment.
pub struct PipewireDetails {
    generation: u64,
    base: Status,
    enriched: Status,
}

impl Default for PipewireDetails {
    fn default() -> Self {
        let base = crate::observer::unavailable_status();
        Self {
            generation: 0,
            enriched: base.clone(),
            base,
        }
    }
}

impl PipewireDetails {
    pub fn update(&mut self, status: Status) -> Option<AttributionJob> {
        if self.base == status {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        self.enriched = status.clone();
        self.base = status;
        let devices: std::collections::BTreeSet<_> = self
            .base
            .details
            .iter()
            .filter(|_| self.base.state == State::Active)
            .flat_map(|details| &details.observations)
            .filter(|row| row.device.is_none())
            .filter_map(|row| row.device_path.as_deref())
            .filter(|path| valid_path(path))
            .map(PathBuf::from)
            .collect();
        (!devices.is_empty()).then(|| AttributionJob {
            generation: self.generation,
            holders: Default::default(),
            devices: devices.into_iter().collect(),
        })
    }

    pub fn status(&self) -> Status {
        self.enriched.clone()
    }

    pub fn accept(&mut self, result: AttributionResult) -> bool {
        if result.generation != self.generation || self.base.state != State::Active {
            return false;
        }
        let Some(details) = &self.base.details else {
            return false;
        };
        let rows: Vec<_> = details
            .observations
            .iter()
            .map(|row| Observation {
                device: row.device.clone().or_else(|| {
                    result
                        .details
                        .observations
                        .iter()
                        .find(|candidate| {
                            candidate.device_path.is_some()
                                && candidate.device_path == row.device_path
                        })
                        .and_then(|candidate| candidate.device.clone())
                }),
                ..row.clone()
            })
            .collect();
        self.enriched = Status {
            details: Some(CameraDetails {
                truncated: details.truncated || result.details.truncated,
                ..project(&rows)
            }),
            ..self.base.clone()
        };
        true
    }
}

/// Worker-only metadata I/O: at most 32 devices, 4 KiB/read and 100 ms/job.
/// Stat, never open, the device. Require its major/minor to match sysfs before
/// trusting a name; local filesystem syscalls have no hard cancellation.
pub fn enrich_pipewire_with(
    io: &dyn MetadataIo,
    sys_root: &Path,
    dev_root: &Path,
    job: &AttributionJob,
) -> CameraDetails {
    let deadline = io.now() + JOB_BUDGET;
    let mut rows = Vec::new();
    let mut truncated = job.devices.len() > 32;
    for path in job.devices.iter().take(32) {
        if io.now() >= deadline {
            truncated = true;
            break;
        }
        let Some(path) = path.to_str().filter(|path| valid_path(path)) else {
            continue;
        };
        let name = &path["/dev/".len()..];
        let Some(device) = read_device_with(io, sys_root, dev_root, name, deadline) else {
            continue;
        };
        let expected = format!(
            "{}:{}",
            libc::major(device.device_id),
            libc::minor(device.device_id)
        );
        let identity = io.read_text(
            &sys_root.join("class/video4linux").join(name).join("dev"),
            deadline,
        );
        if !identity.is_ok_and(|id| id.trim() == expected) || io.now() >= deadline {
            continue;
        }
        if let Some(label) = device.name {
            rows.push(Observation {
                application: None,
                application_source: ApplicationSource::Unknown,
                device: Some(label),
                device_path: Some(device.path),
                evidence: EvidenceKind::PipewireCapture,
            });
        }
    }
    CameraDetails {
        truncated: truncated || io.now() >= deadline,
        ..project(&rows)
    }
}

pub struct StatusStream {
    pipewire: Status,
    device: Status,
    last_line: Option<String>,
}

impl Default for StatusStream {
    fn default() -> Self {
        Self {
            pipewire: crate::observer::unavailable_status(),
            device: Status::new(
                State::Unknown,
                "Camera device holder lookup unavailable",
                &[],
            ),
            last_line: None,
        }
    }
}

impl StatusStream {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn device_update(&mut self, status: Status) -> Option<String> {
        self.device = status;
        self.line()
    }
    pub fn pipewire_updates(&mut self, updates: impl IntoIterator<Item = Status>) -> Vec<String> {
        updates
            .into_iter()
            .filter_map(|status| {
                self.pipewire = status;
                self.line()
            })
            .collect()
    }
    fn line(&mut self) -> Option<String> {
        let line = status::bounded_json_line(&status::merge(&self.pipewire, &self.device));
        if self.last_line.as_ref() == Some(&line) {
            return None;
        }
        self.last_line = Some(line.clone());
        Some(line)
    }
}
