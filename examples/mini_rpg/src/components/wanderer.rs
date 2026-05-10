use seishin::prelude::*;

use super::map_state::TileMapState;

#[derive(Debug)]
pub struct WandererController {
    speed: f32,
    direction: f32,
    direction_timer: f32,
}

impl WandererController {
    pub const DEFAULT_SPEED: f32 = 80.0;

    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            direction: 1.0,
            direction_timer: 0.0,
        }
    }
}

impl Component for WandererController {
    fn update(&mut self, entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()> {
        let map = match TileMapState::from_world(&context.world()) {
            Some(map) => map,
            None => {
                self.direction_timer += context.delta_seconds();
                if self.direction_timer >= 2.0 {
                    self.direction_timer = 0.0;
                    self.direction *= -1.0;
                }

                let delta_x = self.direction * self.speed * context.delta_seconds();
                context.world().translate(entity, Vec2::new(delta_x, 0.0));
                return Ok(());
            }
        };

        self.direction_timer += context.delta_seconds();
        if self.direction_timer >= 2.0 {
            self.direction_timer = 0.0;
            self.direction *= -1.0;
        }

        let delta_x = self.direction * self.speed * context.delta_seconds();
        let current = context
            .world()
            .transform(entity)
            .map(|transform| Vec2::new(transform.x, transform.y))
            .unwrap_or(Vec2::ZERO);
        let candidate = Vec2::new(current.x + delta_x, current.y);

        if map.is_walkable(candidate.x, current.y) {
            context.world().set_position(entity, candidate.x, current.y);
        }

        Ok(())
    }
}

pub fn wanderer_controller_factory(config: &toml::Value) -> GameResult<Box<dyn Component>> {
    let speed = config
        .get("speed")
        .and_then(|value| value.as_float())
        .unwrap_or(WandererController::DEFAULT_SPEED as f64) as f32;

    Ok(Box::new(WandererController::new(speed)))
}

pub fn new() -> impl ComponentDefinition {
    component_factory("WandererController", wanderer_controller_factory)
}
