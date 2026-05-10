#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(u64);

impl EntityId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    pub x: f32,
    pub y: f32,
    pub rotation_radians: f32,
    pub scale_x: f32,
    pub scale_y: f32,
}

impl Transform2D {
    pub const IDENTITY: Self = Self {
        x: 0.0,
        y: 0.0,
        rotation_radians: 0.0,
        scale_x: 1.0,
        scale_y: 1.0,
    };

    pub const fn from_translation(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            ..Self::IDENTITY
        }
    }

    pub const fn translated(self, delta_x: f32, delta_y: f32) -> Self {
        Self {
            x: self.x + delta_x,
            y: self.y + delta_y,
            ..self
        }
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_round_trips_raw_value() {
        let entity = EntityId::new(42);

        assert_eq!(entity.raw(), 42);
    }

    #[test]
    fn transform2d_identity_is_default() {
        assert_eq!(Transform2D::default(), Transform2D::IDENTITY);
    }

    #[test]
    fn transform2d_can_translate_without_backend_types() {
        let transform = Transform2D::from_translation(10.0, -4.0).translated(2.5, 3.0);

        assert_eq!(transform.x, 12.5);
        assert_eq!(transform.y, -1.0);
        assert_eq!(transform.rotation_radians, 0.0);
        assert_eq!(transform.scale_x, 1.0);
        assert_eq!(transform.scale_y, 1.0);
    }
}
