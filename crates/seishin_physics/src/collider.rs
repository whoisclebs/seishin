use crate::Aabb;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collider2D {
    pub width: f32,
    pub height: f32,
}

impl Collider2D {
    pub fn rectangle(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn aabb_centered_at(self, center_x: f32, center_y: f32) -> Aabb {
        Aabb::from_center_size(center_x, center_y, self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_collider_stores_size() {
        assert_eq!(
            Collider2D::rectangle(16.0, 24.0),
            Collider2D {
                width: 16.0,
                height: 24.0
            }
        );
    }
}
