//! Pure, bounded public camera attribution; no discovery or filesystem access.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationSource {
    Process,
    PipewireReported,
    Unknown,
}

impl ApplicationSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::PipewireReported => "pipewire-reported",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceKind {
    DeviceAccess,
    PipewireCapture,
}

impl EvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeviceAccess => "device-access",
            Self::PipewireCapture => "pipewire-capture",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub application: Option<String>,
    pub application_source: ApplicationSource,
    pub device: Option<String>,
    pub device_path: Option<String>,
    pub evidence: EvidenceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraDetails {
    pub observations: Vec<Observation>,
    pub truncated: bool,
}

/// Internal lookup identity, deliberately separate from public observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub path: String,
    pub device_id: u64,
    pub name: Option<String>,
}

/// Strip control/bidi formatting characters, trim, and cap at 80 UTF-8 bytes.
pub fn normalize_label(raw: &str) -> Option<String> {
    let clean: String = raw.chars().filter(|c| {
        !c.is_control() && !matches!(*c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
    }).collect();
    let trimmed = clean.trim();
    let end = trimmed
        .char_indices()
        .map(|(i, c)| i + c.len_utf8())
        .take_while(|end| *end <= 80)
        .last()
        .unwrap_or(0);
    let label = trimmed[..end].trim_end();
    (!label.is_empty()).then(|| label.to_owned())
}

/// Normalize, exact-deduplicate and deterministically sort before capping rows.
/// Wire enum strings order lexically; device labels break otherwise equal keys.
pub fn project(rows: &[Observation]) -> CameraDetails {
    let observations: BTreeMap<_, _> = rows
        .iter()
        .map(|row| Observation {
            application: row.application.as_deref().and_then(normalize_label),
            application_source: row.application_source,
            device: row.device.as_deref().and_then(normalize_label),
            device_path: row
                .device_path
                .as_ref()
                .filter(|path| valid_path(path))
                .cloned(),
            evidence: row.evidence,
        })
        .map(|row| {
            let key = (
                row.device_path.clone(),
                row.application.clone(),
                row.application_source.as_str(),
                row.evidence.as_str(),
                row.device.clone(),
            );
            (key, row)
        })
        .collect();
    let truncated = observations.len() > 32;
    CameraDetails {
        observations: observations.into_values().take(32).collect(),
        truncated,
    }
}

pub fn valid_path(path: &str) -> bool {
    path.len() <= 64
        && path.strip_prefix("/dev/video").is_some_and(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
        })
}
