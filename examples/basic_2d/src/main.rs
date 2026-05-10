use seishin::prelude::*;

mod components;

use components::PlayerController;

struct Game;

impl Game2D for Game {
    fn new(ctx: &mut StartupContext) -> GameResult<Self> {
        ctx.components()
            .register::<PlayerController>("PlayerController")?;

        Ok(Self)
    }
}

seishin::seishin_main!(Game);
