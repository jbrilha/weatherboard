use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Lisbon,
    Caparica,
}

impl Location {
    pub const fn coords(self) -> (f32, f32) {
        match self {
            Location::Lisbon => (38.71667, -9.13333),
            Location::Caparica => (38.66667, -9.2),
        }
    }
}
