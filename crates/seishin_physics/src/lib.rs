mod aabb;
mod collider;

pub use aabb::Aabb;
pub use collider::Collider2D;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aabb_from_min_max_reports_size_and_center() {
        let aabb = Aabb::from_min_max(2.0, 4.0, 12.0, 20.0);

        assert_eq!(aabb.width(), 10.0);
        assert_eq!(aabb.height(), 16.0);
        assert_eq!(aabb.center(), (7.0, 12.0));
    }

    #[test]
    fn aabb_from_center_size_places_bounds_around_center() {
        let aabb = Aabb::from_center_size(8.0, 10.0, 6.0, 4.0);

        assert_eq!(aabb, Aabb::from_min_max(5.0, 8.0, 11.0, 12.0));
    }

    #[test]
    fn aabb_intersects_only_when_areas_overlap() {
        let aabb = Aabb::from_min_size(0.0, 0.0, 10.0, 10.0);

        assert!(aabb.intersects(&Aabb::from_min_size(9.0, 4.0, 4.0, 4.0)));
        assert!(!aabb.intersects(&Aabb::from_min_size(10.0, 0.0, 4.0, 4.0)));
        assert!(!aabb.intersects(&Aabb::from_min_size(11.0, 0.0, 4.0, 4.0)));
    }

    #[test]
    fn aabb_intersection_returns_overlap_region() {
        let aabb = Aabb::from_min_size(0.0, 0.0, 10.0, 8.0);
        let other = Aabb::from_min_size(6.0, 3.0, 8.0, 7.0);

        assert_eq!(
            aabb.intersection(&other),
            Some(Aabb::from_min_max(6.0, 3.0, 10.0, 8.0))
        );
        assert_eq!(
            aabb.intersection(&Aabb::from_min_size(10.0, 0.0, 4.0, 4.0)),
            None
        );
    }

    #[test]
    fn moved_and_swept_bounds_support_collision_queries() {
        let start = Aabb::from_min_size(10.0, 10.0, 5.0, 5.0);

        assert_eq!(
            start.moved_by(-4.0, 8.0),
            Aabb::from_min_max(6.0, 18.0, 11.0, 23.0)
        );
        assert_eq!(
            start.swept_by(-4.0, 8.0),
            Aabb::from_min_max(6.0, 10.0, 15.0, 23.0)
        );
    }

    #[test]
    fn expanded_bounds_grow_evenly_on_each_axis() {
        let aabb = Aabb::from_min_size(10.0, 20.0, 8.0, 6.0);

        assert_eq!(
            aabb.expanded_by(2.0, 3.0),
            Aabb::from_min_max(8.0, 17.0, 20.0, 29.0)
        );
    }

    #[test]
    fn collider_can_create_centered_aabb() {
        let collider = Collider2D::rectangle(6.0, 10.0);

        assert_eq!(
            collider.aabb_centered_at(20.0, 30.0),
            Aabb::from_min_max(17.0, 25.0, 23.0, 35.0)
        );
    }
}
