use std::path::{Path, PathBuf};

use seishin_core::{Engine, EngineResult, Game};
use seishin_input::InputState;

use crate::FixedTimestep;

#[derive(Debug, Clone, PartialEq)]
pub struct MobileRunConfig {
    timestep: FixedTimestep,
    user_data_root: Option<PathBuf>,
}

impl MobileRunConfig {
    pub fn new(timestep: FixedTimestep) -> Self {
        Self {
            timestep,
            user_data_root: None,
        }
    }

    pub fn with_user_data_root(mut self, path: impl Into<PathBuf>) -> Self {
        self.user_data_root = Some(path.into());
        self
    }

    pub fn timestep(&self) -> FixedTimestep {
        self.timestep
    }

    pub fn user_data_root(&self) -> Option<&Path> {
        self.user_data_root.as_deref()
    }
}

impl Default for MobileRunConfig {
    fn default() -> Self {
        Self::new(FixedTimestep::from_fps(60))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileRuntimeState {
    Paused,
    Running,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MobileSurface {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
}

impl MobileSurface {
    pub const fn new(width: u32, height: u32, scale_factor: f32) -> Self {
        Self {
            width,
            height,
            scale_factor,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileTouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MobileTouchEvent {
    pub id: u64,
    pub phase: MobileTouchPhase,
    pub x: f32,
    pub y: f32,
}

impl MobileTouchEvent {
    pub const fn new(id: u64, phase: MobileTouchPhase, x: f32, y: f32) -> Self {
        Self { id, phase, x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MobileLifecycleEvent {
    Resumed,
    Paused,
    SurfaceAvailable(MobileSurface),
    SurfaceLost,
    Touch(MobileTouchEvent),
    AudioInterrupted,
    AudioResumed,
}

#[derive(Debug)]
pub struct MobileRuntime<G> {
    engine: Engine,
    game: G,
    config: MobileRunConfig,
    input: InputState,
    state: MobileRuntimeState,
    surface: Option<MobileSurface>,
    audio_interrupted: bool,
    shutdown: bool,
}

impl<G: Game> MobileRuntime<G> {
    pub fn new(mut engine: Engine, mut game: G, config: MobileRunConfig) -> EngineResult<Self> {
        game.ready(&mut engine)?;

        Ok(Self {
            engine,
            game,
            config,
            input: InputState::default(),
            state: MobileRuntimeState::Paused,
            surface: None,
            audio_interrupted: false,
            shutdown: false,
        })
    }

    pub fn handle_event(&mut self, event: MobileLifecycleEvent) {
        match event {
            MobileLifecycleEvent::Resumed => {
                if self.state != MobileRuntimeState::Stopped {
                    self.state = MobileRuntimeState::Running;
                }
            }
            MobileLifecycleEvent::Paused => {
                if self.state != MobileRuntimeState::Stopped {
                    self.state = MobileRuntimeState::Paused;
                }
            }
            MobileLifecycleEvent::SurfaceAvailable(surface) => {
                self.surface = Some(surface);
            }
            MobileLifecycleEvent::SurfaceLost => {
                self.surface = None;
            }
            MobileLifecycleEvent::Touch(touch) => self.apply_touch(touch),
            MobileLifecycleEvent::AudioInterrupted => {
                self.audio_interrupted = true;
            }
            MobileLifecycleEvent::AudioResumed => {
                self.audio_interrupted = false;
            }
        }
    }

    pub fn update_frame(&mut self) -> EngineResult<()> {
        if self.state != MobileRuntimeState::Running {
            return Ok(());
        }

        let context = self.engine.tick(self.config.timestep.delta_seconds)?;
        self.game.update(&mut self.engine, context)?;
        self.input.end_frame();
        Ok(())
    }

    pub fn shutdown(&mut self) -> EngineResult<()> {
        if !self.shutdown {
            self.game.shutdown(&mut self.engine)?;
            self.shutdown = true;
        }

        self.state = MobileRuntimeState::Stopped;
        Ok(())
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn game(&self) -> &G {
        &self.game
    }

    pub fn config(&self) -> &MobileRunConfig {
        &self.config
    }

    pub fn input_state(&self) -> &InputState {
        &self.input
    }

    pub fn state(&self) -> MobileRuntimeState {
        self.state
    }

    pub fn surface(&self) -> Option<&MobileSurface> {
        self.surface.as_ref()
    }

    pub fn audio_interrupted(&self) -> bool {
        self.audio_interrupted
    }

    fn apply_touch(&mut self, touch: MobileTouchEvent) {
        match touch.phase {
            MobileTouchPhase::Started => self.input.touch_start(touch.id, touch.x, touch.y),
            MobileTouchPhase::Moved => self.input.touch_move(touch.id, touch.x, touch.y),
            MobileTouchPhase::Ended => self.input.touch_end(touch.id),
            MobileTouchPhase::Cancelled => self.input.touch_cancel(touch.id),
        }
    }
}
