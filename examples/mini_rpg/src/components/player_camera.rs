use seishin::prelude::*;

#[derive(Debug)]
pub struct PlayerCamera {
    camera: Camera2DHandle,
    zoom: f32,
}

impl PlayerCamera {
    pub const DEFAULT_ZOOM: f32 = 1.0;

    pub fn new(camera: Camera2DHandle, zoom: f32) -> Self {
        let zoom = if zoom > 0.0 { zoom } else { Self::DEFAULT_ZOOM };
        Self { camera, zoom }
    }
}

impl Component for PlayerCamera {
    fn update(&mut self, entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()> {
        let Some(transform) = context.world().transform(entity) else {
            return Ok(());
        };

        self.camera.set(Camera2D {
            x: transform.x,
            y: transform.y,
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

    Ok(Box::new(PlayerCamera::new(camera, zoom)))
}

pub struct PlayerCameraDefinition;

impl ComponentDefinition for PlayerCameraDefinition {
    fn build(&self, app: &mut StartupContext) -> GameResult<()> {
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

pub fn new() -> PlayerCameraDefinition {
    PlayerCameraDefinition
}
