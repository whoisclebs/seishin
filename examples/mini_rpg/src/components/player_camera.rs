use seishin::prelude::*;

#[derive(Debug)]
pub struct PlayerCamera {
    camera: Camera2DHandle,
    zoom: f32,
    follow_speed: f32,
    position: Option<Vec2>,
}

impl PlayerCamera {
    pub const DEFAULT_ZOOM: f32 = 1.0;
    pub const DEFAULT_FOLLOW_SPEED: f32 = 7.0;

    pub fn new(camera: Camera2DHandle, zoom: f32, follow_speed: f32) -> Self {
        let zoom = if zoom > 0.0 { zoom } else { Self::DEFAULT_ZOOM };
        let follow_speed = if follow_speed.is_finite() && follow_speed >= 0.0 {
            follow_speed
        } else {
            Self::DEFAULT_FOLLOW_SPEED
        };

        Self {
            camera,
            zoom,
            follow_speed,
            position: None,
        }
    }

    fn next_camera_position(&mut self, target: Vec2, delta_seconds: f32) -> Vec2 {
        if self.position.is_none() || self.follow_speed <= f32::EPSILON {
            self.position = Some(target);
            return target;
        }

        let current = self.position.expect("position initialized above");
        let delta_seconds = delta_seconds.max(0.0);
        let blend = (1.0 - (-self.follow_speed * delta_seconds).exp()).clamp(0.0, 1.0);
        let next = Vec2::new(
            current.x + (target.x - current.x) * blend,
            current.y + (target.y - current.y) * blend,
        );
        self.position = Some(next);
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_camera_update_snaps_then_following_frames_smooth_toward_target() {
        let handle = Camera2DHandle::default();
        let mut camera = PlayerCamera::new(handle.clone(), 1.0, 6.0);

        let first = camera.next_camera_position(Vec2::new(96.0, 96.0), 1.0 / 60.0);
        assert_eq!(first, Vec2::new(96.0, 96.0));

        let second = camera.next_camera_position(Vec2::new(144.0, 96.0), 1.0 / 60.0);
        assert!(second.x > 96.0);
        assert!(second.x < 144.0);
        assert_eq!(second.y, 96.0);
    }

    #[test]
    fn zero_follow_speed_keeps_instant_camera_follow() {
        let handle = Camera2DHandle::default();
        let mut camera = PlayerCamera::new(handle, 1.0, 0.0);

        assert_eq!(
            camera.next_camera_position(Vec2::new(10.0, 20.0), 1.0 / 60.0),
            Vec2::new(10.0, 20.0)
        );
        assert_eq!(
            camera.next_camera_position(Vec2::new(30.0, 40.0), 1.0 / 60.0),
            Vec2::new(30.0, 40.0)
        );
    }
}

impl Component for PlayerCamera {
    fn update(&mut self, entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()> {
        let Some(transform) = context.world().transform(entity) else {
            return Ok(());
        };

        let position =
            self.next_camera_position(Vec2::new(transform.x, transform.y), context.delta_seconds());

        self.camera.set(Camera2D {
            x: position.x,
            y: position.y,
            zoom: self.zoom,
        })?;

        Ok(())
    }
}

pub fn player_camera_factory(
    config: &toml::Value,
    camera: Camera2DHandle,
) -> GameResult<Box<dyn Component>> {
    let zoom = config
        .get("zoom")
        .and_then(|value| value.as_float())
        .unwrap_or(PlayerCamera::DEFAULT_ZOOM as f64) as f32;
    let follow_speed = config
        .get("follow_speed")
        .and_then(|value| value.as_float())
        .unwrap_or(PlayerCamera::DEFAULT_FOLLOW_SPEED as f64) as f32;

    Ok(Box::new(PlayerCamera::new(camera, zoom, follow_speed)))
}

pub fn new() -> impl ComponentDefinition {
    |app: &mut StartupContext| {
        let camera = app
            .resource::<Camera2DHandle>()
            .cloned()
            .unwrap_or_default();

        if app.resource::<Camera2DHandle>().is_none() {
            app.insert_resource(camera.clone());
        }

        app.register_component_factory("PlayerCamera", move |config| {
            player_camera_factory(config, camera.clone())
        })?;
        Ok(())
    }
}
