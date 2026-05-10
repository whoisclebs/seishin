use seishin::prelude::*;

#[derive(Default)]
pub struct PlayerController {
    speed: Option<f32>,
}

impl PlayerController {
    pub const DEFAULT_SPEED: f32 = 180.0;

    pub fn with_speed(speed: f32) -> Self {
        Self { speed: Some(speed) }
    }

    fn speed(&self) -> f32 {
        self.speed.unwrap_or(Self::DEFAULT_SPEED)
    }
}

impl Component for PlayerController {
    fn update(&mut self, entity: Entity, ctx: &mut FrameContext<'_>) -> GameResult<()> {
        let speed = self.speed();
        let movement = ctx.input().axis2d("move");
        let displacement = movement * speed * ctx.delta_seconds();

        ctx.world().translate(entity, displacement);

        Ok(())
    }
}
