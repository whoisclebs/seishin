use seishin::prelude::*;

mod components;

use components::PlayerController;

struct Game;

impl Game2D for Game {
    fn new(ctx: &mut StartupContext) -> GameResult<Self> {
        ctx.components()
            .register_factory("PlayerController", |config| {
                let speed = config
                    .get("speed")
                    .and_then(|value| value.as_float())
                    .unwrap_or(PlayerController::DEFAULT_SPEED as f64)
                    as f32;

                Ok(Box::new(PlayerController::with_speed(speed)))
            })?;

        Ok(Self)
    }
}

seishin::seishin_main!(Game);
