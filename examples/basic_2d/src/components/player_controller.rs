use seishin::prelude::*;

#[derive(Default)]
pub struct PlayerController {
    speed: Option<f32>,
}

impl PlayerController {
    pub const DEFAULT_SPEED: f32 = 180.0;

    fn speed(&mut self, ctx: &FrameContext<'_>) -> GameResult<f32> {
        if let Some(speed) = self.speed {
            return Ok(speed);
        }

        let config = ctx
            .resources()
            .toml("res://data/components/player_controller.toml")?;
        let speed = config.f32("speed").unwrap_or(Self::DEFAULT_SPEED);

        self.speed = Some(speed);
        Ok(speed)
    }
}

impl Component2D for PlayerController {
    fn update(&mut self, entity: Entity, ctx: &mut FrameContext<'_>) -> GameResult<()> {
        let speed = self.speed(ctx)?;
        let movement = ctx.input().axis2d("move");
        let displacement = movement * speed * ctx.delta_seconds();

        ctx.world().translate(entity, displacement);

        Ok(())
    }
}
