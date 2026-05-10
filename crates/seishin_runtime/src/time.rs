#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedTimestep {
    pub delta_seconds: f32,
}

impl FixedTimestep {
    pub fn from_fps(fps: u32) -> Self {
        Self {
            delta_seconds: 1.0 / fps as f32,
        }
    }
}
