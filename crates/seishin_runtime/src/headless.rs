use seishin_core::{Engine, EngineResult, Game};

use crate::FixedTimestep;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadlessRunConfig {
    pub frames: u64,
}

pub fn run_headless<G: Game>(
    engine: &mut Engine,
    game: &mut G,
    run: HeadlessRunConfig,
    timestep: FixedTimestep,
) -> EngineResult<()> {
    engine.run_for_frames(game, run.frames, timestep.delta_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use seishin_core::{EngineConfig, UpdateContext};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        Ready,
        Update,
        Shutdown,
    }

    struct CountingGame {
        events: Vec<Event>,
        updates: u64,
    }

    impl Game for CountingGame {
        fn ready(&mut self, _engine: &mut Engine) -> EngineResult<()> {
            self.events.push(Event::Ready);
            Ok(())
        }

        fn update(&mut self, _engine: &mut Engine, _context: UpdateContext) -> EngineResult<()> {
            self.events.push(Event::Update);
            self.updates += 1;
            Ok(())
        }

        fn shutdown(&mut self, _engine: &mut Engine) -> EngineResult<()> {
            self.events.push(Event::Shutdown);
            Ok(())
        }
    }

    struct ContextGame {
        events: Vec<Event>,
        frames: Vec<u64>,
        deltas: Vec<f32>,
    }

    impl Game for ContextGame {
        fn ready(&mut self, _engine: &mut Engine) -> EngineResult<()> {
            self.events.push(Event::Ready);
            Ok(())
        }

        fn update(&mut self, _engine: &mut Engine, context: UpdateContext) -> EngineResult<()> {
            self.events.push(Event::Update);
            self.frames.push(context.frame);
            self.deltas.push(context.delta_seconds);
            Ok(())
        }

        fn shutdown(&mut self, _engine: &mut Engine) -> EngineResult<()> {
            self.events.push(Event::Shutdown);
            Ok(())
        }
    }

    #[test]
    fn headless_runtime_advances_expected_frames() {
        let mut engine = Engine::new(EngineConfig::default()).unwrap();
        let mut game = CountingGame {
            events: Vec::new(),
            updates: 0,
        };

        run_headless(
            &mut engine,
            &mut game,
            HeadlessRunConfig { frames: 3 },
            FixedTimestep::from_fps(60),
        )
        .unwrap();

        assert_eq!(engine.frame(), 3);
        assert_eq!(game.updates, 3);
        assert_eq!(
            game.events,
            vec![
                Event::Ready,
                Event::Update,
                Event::Update,
                Event::Update,
                Event::Shutdown
            ]
        );
    }

    #[test]
    fn zero_frame_headless_run_is_ready_then_shutdown_only() {
        let mut engine = Engine::new(EngineConfig::default()).unwrap();
        let mut game = CountingGame {
            events: Vec::new(),
            updates: 0,
        };

        run_headless(
            &mut engine,
            &mut game,
            HeadlessRunConfig { frames: 0 },
            FixedTimestep::from_fps(60),
        )
        .unwrap();

        assert_eq!(engine.frame(), 0);
        assert_eq!(game.updates, 0);
        assert_eq!(game.events, vec![Event::Ready, Event::Shutdown]);
    }

    #[test]
    fn headless_runtime_produces_deterministic_update_contexts() {
        let mut engine = Engine::new(EngineConfig::default()).unwrap();
        let mut game = ContextGame {
            events: Vec::new(),
            frames: Vec::new(),
            deltas: Vec::new(),
        };

        run_headless(
            &mut engine,
            &mut game,
            HeadlessRunConfig { frames: 3 },
            FixedTimestep::from_fps(120),
        )
        .unwrap();

        assert_eq!(
            game.events,
            vec![
                Event::Ready,
                Event::Update,
                Event::Update,
                Event::Update,
                Event::Shutdown
            ]
        );
        assert_eq!(game.frames, vec![1, 2, 3]);
        assert_eq!(game.deltas, vec![1.0 / 120.0; 3]);
    }
}
