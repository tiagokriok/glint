//! Camera status values, merge precedence, and JSON-lines serialization.

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
}

impl Status {
    pub fn new(state: State, evidence: &str, limitations: &[&str]) -> Status {
        Status {
            state,
            evidence: evidence.to_string(),
            limitations: limitations.iter().map(|s| s.to_string()).collect(),
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

    Status {
        state: base.state,
        evidence: base.evidence.clone(),
        limitations,
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
        out.push_str("]}}");
        out
    }
}

/// Serialize `status`, falling back to a bounded unknown line when oversized.
///
/// The helper never emits a line above [`MAX_LINE_BYTES`]; the fixed wording
/// keeps the framing safe even if an evidence string ever grows.
pub fn bounded_json_line(status: &Status) -> String {
    let line = status.to_json_line();
    if line.len() <= MAX_LINE_BYTES {
        return line;
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
