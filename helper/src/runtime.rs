use crate::status::{self, State, Status};

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
