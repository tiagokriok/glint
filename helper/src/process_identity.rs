//! Attribution for already-known holder PIDs; no process discovery or FD opens.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Instant;

use crate::attribution::{
    ApplicationSource, DeviceIdentity, EvidenceKind, Observation, normalize_label, project,
};
use crate::device_metadata::{FsMetadataIo, MAX_FDS, MetadataIo};

#[derive(Debug)]
pub struct HolderAttribution {
    pub observations: Vec<Observation>,
    pub truncated: bool,
}

/// `comm` may contain spaces and parentheses. The suffix after its final `)`
/// starts at field 3; starttime is field 22 (suffix index 19).
pub fn parse_start_time(stat: &str, pid: u32) -> Option<u64> {
    let (prefix, suffix) = stat.rsplit_once(')')?;
    let (id, _) = prefix.split_once(" (")?;
    if id.parse::<u32>().ok()? != pid {
        return None;
    }
    suffix.split_whitespace().nth(19)?.parse().ok()
}

pub fn observe_holder(
    proc_root: &Path,
    pid: u32,
    devices: &[DeviceIdentity],
    deadline: Instant,
) -> HolderAttribution {
    observe_holder_with(&FsMetadataIo, proc_root, pid, devices, deadline)
}

pub fn observe_holder_with(
    io: &dyn MetadataIo,
    proc_root: &Path,
    pid: u32,
    devices: &[DeviceIdentity],
    deadline: Instant,
) -> HolderAttribution {
    if devices.is_empty() || io.now() >= deadline {
        return unknown(io.now() >= deadline);
    }
    let root = proc_root.join(pid.to_string());
    let start = read_start(io, &root, pid, deadline);
    let application = if io.now() < deadline {
        io.read_text(&root.join("comm"), deadline)
            .ok()
            .as_deref()
            .and_then(normalize_label)
    } else {
        None
    };
    if io.now() >= deadline {
        return unknown(true);
    }
    let Ok(entries) = io.fd_entries(&root.join("fd"), deadline) else {
        return unknown(true);
    };
    let mut matched = BTreeSet::new();
    let mut truncated = entries.truncated || entries.paths.len() > MAX_FDS;
    for path in entries.paths.into_iter().take(MAX_FDS) {
        if io.now() >= deadline {
            truncated = true;
            break;
        }
        match io.character_device(&path, deadline) {
            Ok(Some(id)) => {
                matched.extend(
                    devices
                        .iter()
                        .enumerate()
                        .filter_map(|(index, device)| (device.device_id == id).then_some(index)),
                );
            }
            Ok(None) => {}
            Err(_) => truncated = true,
        }
    }
    let end = read_start(io, &root, pid, deadline);
    let expired = io.now() >= deadline;
    let application = application.filter(|_| start.is_some() && start == end && !expired);
    if matched.is_empty() {
        // A stable reused PID without a current matching FD is still unknown.
        return unknown(truncated || expired);
    }
    let rows: Vec<_> = matched
        .into_iter()
        .map(|index| {
            let device = &devices[index];
            Observation {
                application: application.clone(),
                application_source: if application.is_some() {
                    ApplicationSource::Process
                } else {
                    ApplicationSource::Unknown
                },
                device: device.name.clone(),
                device_path: Some(device.path.clone()),
                evidence: EvidenceKind::DeviceAccess,
            }
        })
        .collect();
    let details = project(&rows);
    HolderAttribution {
        observations: details.observations,
        truncated: truncated || expired || details.truncated,
    }
}

fn read_start(io: &dyn MetadataIo, root: &Path, pid: u32, deadline: Instant) -> Option<u64> {
    if io.now() >= deadline {
        return None;
    }
    parse_start_time(&io.read_text(&root.join("stat"), deadline).ok()?, pid)
}

pub(crate) fn unknown_row() -> Observation {
    Observation {
        application: None,
        application_source: ApplicationSource::Unknown,
        device: None,
        device_path: None,
        evidence: EvidenceKind::DeviceAccess,
    }
}

fn unknown(truncated: bool) -> HolderAttribution {
    HolderAttribution {
        observations: vec![unknown_row()],
        truncated,
    }
}
