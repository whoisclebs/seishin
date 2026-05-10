#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Collider2D {
    pub width: f32,
    pub height: f32,
}

impl Collider2D {
    pub fn rectangle(width: f32, height: f32) -> Self {
        Self { width, height }
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
