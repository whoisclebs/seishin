use crate::{EngineError, EngineResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub app_name: String,
    pub target_fps: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            app_name: "seishin game".to_string(),
            target_fps: 60,
        }
    }
}

impl EngineConfig {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            ..Self::default()
        }
    }

    pub fn with_target_fps(mut self, target_fps: u32) -> Self {
        self.target_fps = target_fps;
        self
    }

    pub fn validate(&self) -> EngineResult<()> {
        if self.app_name.trim().is_empty() {
            return Err(EngineError::InvalidConfig(
                "app_name cannot be empty".to_string(),
            ));
        }

        if self.target_fps == 0 {
            return Err(EngineError::InvalidConfig(
                "target_fps must be greater than zero".to_string(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UpdateContext {
    pub delta_seconds: f32,
    pub frame: u64,
}

pub trait Game {
    fn ready(&mut self, _engine: &mut Engine) -> EngineResult<()> {
        Ok(())
    }

    fn update(&mut self, _engine: &mut Engine, _context: UpdateContext) -> EngineResult<()> {
        Ok(())
    }

    fn shutdown(&mut self, _engine: &mut Engine) -> EngineResult<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Engine {
    config: EngineConfig,
    frame: u64,
}

impl Engine {
    pub fn new(config: EngineConfig) -> EngineResult<Self> {
        config.validate()?;

        Ok(Self { config, frame: 0 })
    }

    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn tick(&mut self, delta_seconds: f32) -> EngineResult<UpdateContext> {
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(EngineError::InvalidDeltaTime);
        }

        self.frame += 1;

        Ok(UpdateContext {
            delta_seconds,
            frame: self.frame,
        })
    }

    pub fn run_for_frames<G: Game>(
        &mut self,
        game: &mut G,
        frames: u64,
        delta_seconds: f32,
    ) -> EngineResult<()> {
        game.ready(self)?;

        for _ in 0..frames {
            let context = self.tick(delta_seconds)?;
            game.update(self, context)?;
        }

        game.shutdown(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_advances_frames() {
        let mut engine = Engine::new(EngineConfig::default()).unwrap();

        let context = engine.tick(1.0 / 60.0).unwrap();

        assert_eq!(engine.frame(), 1);
        assert_eq!(context.frame, 1);
        assert_eq!(context.delta_seconds, 1.0 / 60.0);
    }

    #[test]
    fn config_rejects_empty_app_name() {
        let result = Engine::new(EngineConfig {
            app_name: " ".to_string(),
            target_fps: 60,
        });

        assert!(matches!(result, Err(EngineError::InvalidConfig(_))));
    }

    #[test]
    fn tick_rejects_invalid_delta_time() {
        let mut engine = Engine::new(EngineConfig::default()).unwrap();

        assert_eq!(engine.tick(f32::NAN), Err(EngineError::InvalidDeltaTime));
        assert_eq!(engine.tick(-0.1), Err(EngineError::InvalidDeltaTime));
    }
}
