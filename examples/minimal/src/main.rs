use seishin_core::{Engine, EngineConfig, EngineResult, Game, UpdateContext};

struct MinimalGame;

impl Game for MinimalGame {
    fn ready(&mut self, engine: &mut Engine) -> EngineResult<()> {
        println!("starting {}", engine.config().app_name);
        Ok(())
    }

    fn update(&mut self, _engine: &mut Engine, context: UpdateContext) -> EngineResult<()> {
        println!("frame {} dt {}", context.frame, context.delta_seconds);
        Ok(())
    }
}

fn main() -> EngineResult<()> {
    let mut engine = Engine::new(EngineConfig {
        app_name: "minimal seishin".to_string(),
        ..EngineConfig::default()
    })?;

    engine.run_for_frames(&mut MinimalGame, 3, 1.0 / 60.0)
}
