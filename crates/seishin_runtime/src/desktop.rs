use std::rc::Rc;
use std::{cell::RefCell, mem};

use pollster::block_on;
use seishin_core::{Engine, Game};
use seishin_input::{InputState, KeyCode};
use seishin_render::{RenderError, RenderSize, RenderState, Renderer};
use winit::dpi::{LogicalSize, Size};
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode as WinitKeyCode, PhysicalKey};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::WindowBuilder;

use crate::{DesktopRuntimeError, FixedTimestep};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl WindowSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowConfig {
    pub title: String,
    pub size: WindowSize,
}

impl WindowConfig {
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            size: WindowSize::new(width, height),
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "seishin".to_string(),
            size: WindowSize::new(1280, 720),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesktopRunConfig {
    pub window: WindowConfig,
    pub timestep: FixedTimestep,
}

impl DesktopRunConfig {
    pub fn new(window: WindowConfig) -> Self {
        Self {
            window,
            ..Self::default()
        }
    }

    pub fn with_timestep(mut self, timestep: FixedTimestep) -> Self {
        self.timestep = timestep;
        self
    }
}

impl Default for DesktopRunConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            timestep: FixedTimestep::from_fps(60),
        }
    }
}

pub trait DesktopGame: Game {
    fn input_state(&mut self) -> &mut InputState;

    fn render_state(&self) -> RenderState<'_>;
}

pub fn run_desktop<G: DesktopGame>(
    mut engine: Engine,
    mut game: G,
    config: DesktopRunConfig,
) -> Result<(), DesktopRuntimeError> {
    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title(config.window.title.clone())
        .with_inner_size(Size::Logical(LogicalSize::new(
            config.window.size.width as f64,
            config.window.size.height as f64,
        )))
        .build(&event_loop)?;
    let window_id = window.id();
    let initial_size = window.inner_size();
    let raw_display_handle = window
        .display_handle()
        .map_err(|error| {
            DesktopRuntimeError::Render(RenderError::SurfaceCreation(error.to_string()))
        })?
        .as_raw();
    let raw_window_handle = window
        .window_handle()
        .map_err(|error| {
            DesktopRuntimeError::Render(RenderError::SurfaceCreation(error.to_string()))
        })?
        .as_raw();
    let mut renderer = unsafe {
        block_on(Renderer::new(
            raw_display_handle,
            raw_window_handle,
            RenderSize::new(initial_size.width, initial_size.height),
        ))
    }?;

    game.ready(&mut engine)?;

    let runtime_error = Rc::new(RefCell::new(None));
    let shared_runtime_error = Rc::clone(&runtime_error);
    let mut input = DesktopInputFrame::default();
    let mut shutdown = false;
    let mut exit_requested = false;

    event_loop.run(move |event, event_loop| {
        event_loop.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent {
                window_id: current_window_id,
                event,
            } if current_window_id == window_id => match event {
                WindowEvent::CloseRequested => {
                    exit_requested = true;
                }
                WindowEvent::Resized(size) => {
                    renderer.resize(RenderSize::new(size.width, size.height));
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    input.apply_keyboard_input(event.physical_key, event.state);
                }
                _ => {}
            },
            Event::AboutToWait => {
                let escape_requested = input.begin_game_frame(game.input_state());

                if exit_requested || escape_requested {
                    if !shutdown {
                        if let Err(error) = game.shutdown(&mut engine) {
                            *shared_runtime_error.borrow_mut() = Some(error.into());
                        }

                        shutdown = true;
                    }

                    event_loop.exit();
                    return;
                }

                let update_result = engine
                    .tick(config.timestep.delta_seconds)
                    .and_then(|context| game.update(&mut engine, context));

                match update_result {
                    Ok(()) => match renderer.render(game.render_state()) {
                        Ok(()) => {
                            input.end_game_frame(game.input_state());
                        }
                        Err(error) => {
                            *shared_runtime_error.borrow_mut() = Some(error.into());
                            shutdown_after_error(
                                &mut game,
                                &mut engine,
                                &mut shared_runtime_error.borrow_mut(),
                                &mut shutdown,
                            );
                            event_loop.exit();
                        }
                    },
                    Err(error) => {
                        *shared_runtime_error.borrow_mut() = Some(error.into());
                        shutdown_after_error(
                            &mut game,
                            &mut engine,
                            &mut shared_runtime_error.borrow_mut(),
                            &mut shutdown,
                        );
                        event_loop.exit();
                    }
                }
            }
            Event::LoopExiting if !shutdown => {
                if let Err(error) = game.shutdown(&mut engine) {
                    let error_slot = &mut *shared_runtime_error.borrow_mut();

                    if error_slot.is_none() {
                        *error_slot = Some(error.into());
                    }
                }

                shutdown = true;
            }
            _ => {}
        }
    })?;

    let result = match mem::take(&mut *runtime_error.borrow_mut()) {
        Some(error) => Err(error),
        None => Ok(()),
    };

    result
}

fn shutdown_after_error<G: DesktopGame>(
    game: &mut G,
    engine: &mut Engine,
    error_slot: &mut Option<DesktopRuntimeError>,
    shutdown: &mut bool,
) {
    if *shutdown {
        return;
    }

    if let Err(shutdown_error) = game.shutdown(engine) {
        if error_slot.is_none() {
            *error_slot = Some(shutdown_error.into());
        }
    }

    *shutdown = true;
}

fn map_winit_key_code(key: PhysicalKey) -> Option<KeyCode> {
    match key {
        PhysicalKey::Code(WinitKeyCode::ArrowUp) => Some(KeyCode::ArrowUp),
        PhysicalKey::Code(WinitKeyCode::ArrowDown) => Some(KeyCode::ArrowDown),
        PhysicalKey::Code(WinitKeyCode::ArrowLeft) => Some(KeyCode::ArrowLeft),
        PhysicalKey::Code(WinitKeyCode::ArrowRight) => Some(KeyCode::ArrowRight),
        PhysicalKey::Code(WinitKeyCode::KeyW) => Some(KeyCode::KeyW),
        PhysicalKey::Code(WinitKeyCode::KeyA) => Some(KeyCode::KeyA),
        PhysicalKey::Code(WinitKeyCode::KeyS) => Some(KeyCode::KeyS),
        PhysicalKey::Code(WinitKeyCode::KeyD) => Some(KeyCode::KeyD),
        PhysicalKey::Code(WinitKeyCode::Space) => Some(KeyCode::Space),
        PhysicalKey::Code(WinitKeyCode::Enter) => Some(KeyCode::Enter),
        PhysicalKey::Code(WinitKeyCode::Escape) => Some(KeyCode::Escape),
        _ => None,
    }
}

#[derive(Debug, Default, Clone)]
struct DesktopInputFrame {
    state: InputState,
    escape_requested: bool,
}

impl DesktopInputFrame {
    fn apply_keyboard_input(&mut self, physical_key: PhysicalKey, element_state: ElementState) {
        let Some(key) = map_winit_key_code(physical_key) else {
            return;
        };

        match element_state {
            ElementState::Pressed => {
                self.state.press(key);

                if key == KeyCode::Escape {
                    self.escape_requested = true;
                }
            }
            ElementState::Released => {
                self.state.release(key);
            }
        }
    }

    fn begin_game_frame(&mut self, game_input: &mut InputState) -> bool {
        *game_input = self.state.clone();
        self.escape_requested
    }

    fn end_game_frame(&mut self, game_input: &InputState) {
        self.state = game_input.clone();
        self.state.end_frame();
        self.escape_requested = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_input_frame_tracks_key_transitions_across_frame_boundary() {
        let mut frame = DesktopInputFrame::default();
        let mut input = InputState::default();

        frame.apply_keyboard_input(
            PhysicalKey::Code(WinitKeyCode::ArrowRight),
            ElementState::Pressed,
        );

        assert!(!frame.begin_game_frame(&mut input));
        assert!(input.pressed(KeyCode::ArrowRight));
        assert!(input.just_pressed(KeyCode::ArrowRight));

        frame.end_game_frame(&input);

        assert!(frame.state.pressed(KeyCode::ArrowRight));
        assert!(!frame.state.just_pressed(KeyCode::ArrowRight));
        assert!(!frame.state.just_released(KeyCode::ArrowRight));

        frame.apply_keyboard_input(
            PhysicalKey::Code(WinitKeyCode::ArrowRight),
            ElementState::Released,
        );
        frame.begin_game_frame(&mut input);

        assert!(!input.pressed(KeyCode::ArrowRight));
        assert!(input.just_released(KeyCode::ArrowRight));
    }

    #[test]
    fn desktop_input_frame_requests_exit_on_escape_press() {
        let mut frame = DesktopInputFrame::default();
        let mut input = InputState::default();

        frame.apply_keyboard_input(
            PhysicalKey::Code(WinitKeyCode::Escape),
            ElementState::Pressed,
        );

        assert!(frame.begin_game_frame(&mut input));
        assert!(input.pressed(KeyCode::Escape));
        assert!(input.just_pressed(KeyCode::Escape));

        frame.end_game_frame(&input);

        assert!(!frame.escape_requested);
        assert!(frame.state.pressed(KeyCode::Escape));
        assert!(!frame.state.just_pressed(KeyCode::Escape));
    }

    #[test]
    fn wasd_keys_are_mapped_for_input_actions() {
        let mut frame = DesktopInputFrame::default();
        let mut input = InputState::default();

        frame.apply_keyboard_input(PhysicalKey::Code(WinitKeyCode::KeyA), ElementState::Pressed);

        assert!(!frame.begin_game_frame(&mut input));
        assert!(input.pressed(KeyCode::KeyA));
        assert!(input.just_pressed(KeyCode::KeyA));
    }

    #[test]
    fn unsupported_winit_keys_are_ignored() {
        let mut frame = DesktopInputFrame::default();
        let mut input = InputState::default();

        frame.apply_keyboard_input(PhysicalKey::Code(WinitKeyCode::F1), ElementState::Pressed);

        assert!(!frame.begin_game_frame(&mut input));
        assert!(!input.pressed(KeyCode::ArrowRight));
        assert!(!input.pressed(KeyCode::Escape));
        assert!(!input.pressed(KeyCode::KeyA));
    }
}
