//! Camera status values, merge precedence, and JSON-lines serialization.

use crate::attribution::{CameraDetails, project};

/// One alternative of the `camera.state` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Active,
    Unobserved,
    Unknown,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Active => "active",
            State::Unobserved => "unobserved",
            State::Unknown => "unknown",
        }
    }

    /// Merge rank: positive evidence beats an explicit gap beats nothing seen.
    pub fn rank(self) -> u8 {
        match self {
            State::Unobserved => 0,
            State::Unknown => 1,
            State::Active => 2,
        }
    }
}

/// An observation status: one state, plain-text evidence, coverage limitations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub state: State,
    pub evidence: String,
    pub limitations: Vec<String>,
    pub details: Option<CameraDetails>,
}

impl Status {
    pub fn new(state: State, evidence: &str, limitations: &[&str]) -> Status {
        Status {
            state,
            evidence: evidence.to_string(),
            limitations: limitations.iter().map(|s| s.to_string()).collect(),
            details: None,
        }
    }
}

/// Maximum serialized line length in bytes, excluding the trailing newline.
pub const MAX_LINE_BYTES: usize = 16 * 1024;

/// Combine the PipeWire path and the device-access path.
///
/// Precedence is `active > unknown > unobserved`; on a tie the PipeWire path
/// wins so confirmed capture wording is preferred. Limitations from both
/// paths are retained. A PipeWire gap never erases positive device evidence.
pub fn merge(pipewire: &Status, device: &Status) -> Status {
    let (base, extra) = if pipewire.state.rank() >= device.state.rank() {
        (pipewire, device)
    } else {
        (device, pipewire)
    };

    let mut limitations = base.limitations.clone();
    for item in &extra.limitations {
        if !limitations.contains(item) {
            limitations.push(item.clone());
        }
    }

    let active_details: Vec<_> = [pipewire, device]
        .into_iter()
        .filter(|status| status.state == State::Active)
        .filter_map(|status| status.details.as_ref())
        .collect();
    let details = if active_details.is_empty() {
        None
    } else {
        let rows: Vec<_> = active_details
            .iter()
            .flat_map(|details| details.observations.iter().cloned())
            .collect();
        let projected = project(&rows);
        Some(CameraDetails {
            truncated: projected.truncated
                || active_details.iter().any(|details| details.truncated),
            ..projected
        })
    };

    Status {
        state: base.state,
        evidence: base.evidence.clone(),
        limitations,
        details,
    }
}

impl Status {
    /// Serialize one complete JSON object line (no trailing newline).
    pub fn to_json_line(&self) -> String {
        let mut out = String::with_capacity(128);
        out.push_str("{\"version\":1,\"camera\":{\"state\":\"");
        out.push_str(self.state.as_str());
        out.push_str("\",\"evidence\":");
        write_json_string(&mut out, &self.evidence);
        out.push_str(",\"limitations\":[");
        for (index, limitation) in self.limitations.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            write_json_string(&mut out, limitation);
        }
        out.push(']');
        if let Some(details) = &self.details {
            let projected = project(&details.observations);
            out.push_str(",\"details\":{\"observations\":[");
            for (index, row) in projected.observations.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str("{\"application\":");
                write_optional_string(&mut out, row.application.as_deref());
                out.push_str(",\"applicationSource\":");
                write_json_string(&mut out, row.application_source.as_str());
                out.push_str(",\"device\":");
                write_optional_string(&mut out, row.device.as_deref());
                out.push_str(",\"devicePath\":");
                write_optional_string(&mut out, row.device_path.as_deref());
                out.push_str(",\"evidence\":");
                write_json_string(&mut out, row.evidence.as_str());
                out.push('}');
            }
            out.push_str("],\"truncated\":");
            out.push_str(if details.truncated || projected.truncated {
                "true"
            } else {
                "false"
            });
            out.push('}');
        }
        out.push_str("}}");
        out
    }
}

/// Serialize `status`, budgeting optional details before any base fallback.
///
/// The helper never emits a line above [`MAX_LINE_BYTES`]; the fixed wording
/// keeps the framing safe even if an evidence string ever grows.
pub fn bounded_json_line(status: &Status) -> String {
    let line = status.to_json_line();
    if line.len() <= MAX_LINE_BYTES {
        return line;
    }
    if let Some(details) = &status.details {
        let projected = project(&details.observations);
        for count in (0..projected.observations.len()).rev() {
            let candidate = Status {
                details: Some(CameraDetails {
                    observations: projected.observations[..count].to_vec(),
                    truncated: true,
                }),
                ..status.clone()
            }
            .to_json_line();
            if candidate.len() <= MAX_LINE_BYTES {
                return candidate;
            }
        }
        let base = Status {
            details: None,
            ..status.clone()
        }
        .to_json_line();
        if base.len() <= MAX_LINE_BYTES {
            return base;
        }
    }
    Status::new(State::Unknown, "Capture state could not be classified", &[]).to_json_line()
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if (character as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
}

fn write_optional_string(out: &mut String, value: Option<&str>) {
    if let Some(value) = value {
        write_json_string(out, value);
    } else {
        out.push_str("null");
    }
}
