#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Aabb {
    #[must_use]
    pub const fn from_min_max(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    #[must_use]
    pub const fn from_min_size(min_x: f32, min_y: f32, width: f32, height: f32) -> Self {
        Self::from_min_max(min_x, min_y, min_x + width, min_y + height)
    }

    #[must_use]
    pub const fn from_center_size(center_x: f32, center_y: f32, width: f32, height: f32) -> Self {
        let half_width = width * 0.5;
        let half_height = height * 0.5;

        Self::from_min_max(
            center_x - half_width,
            center_y - half_height,
            center_x + half_width,
            center_y + half_height,
        )
    }

    #[must_use]
    pub const fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    #[must_use]
    pub const fn height(self) -> f32 {
        self.max_y - self.min_y
    }

    #[must_use]
    pub const fn center(self) -> (f32, f32) {
        (
            self.min_x + self.width() * 0.5,
            self.min_y + self.height() * 0.5,
        )
    }

    #[must_use]
    pub const fn moved_by(self, delta_x: f32, delta_y: f32) -> Self {
        Self::from_min_max(
            self.min_x + delta_x,
            self.min_y + delta_y,
            self.max_x + delta_x,
            self.max_y + delta_y,
        )
    }

    #[must_use]
    pub const fn expanded_by(self, amount_x: f32, amount_y: f32) -> Self {
        Self::from_min_max(
            self.min_x - amount_x,
            self.min_y - amount_y,
            self.max_x + amount_x,
            self.max_y + amount_y,
        )
    }

    #[must_use]
    pub fn swept_by(self, delta_x: f32, delta_y: f32) -> Self {
        let moved = self.moved_by(delta_x, delta_y);

        Self::from_min_max(
            self.min_x.min(moved.min_x),
            self.min_y.min(moved.min_y),
            self.max_x.max(moved.max_x),
            self.max_y.max(moved.max_y),
        )
    }

    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min_x < other.max_x
            && self.max_x > other.min_x
            && self.min_y < other.max_y
            && self.max_y > other.min_y
    }

    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }

        Some(Self::from_min_max(
            self.min_x.max(other.min_x),
            self.min_y.max(other.min_y),
            self.max_x.min(other.max_x),
            self.max_y.min(other.max_y),
        ))
    }
}
