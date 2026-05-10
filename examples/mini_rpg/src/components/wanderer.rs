use seishin::prelude::*;

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
        self.direction_timer += context.delta_seconds();
        if self.direction_timer >= 2.0 {
            self.direction_timer = 0.0;
            self.direction *= -1.0;
        }

        let delta_x = self.direction * self.speed * context.delta_seconds();
        context.world().translate(entity, Vec2::new(delta_x, 0.0));

        Ok(())
    }
}
