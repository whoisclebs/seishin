use std::fmt;

use seishin_render::RenderError;

#[derive(Debug)]
pub enum DesktopRuntimeError {
    Engine(seishin_core::EngineError),
    Render(RenderError),
    #[cfg(any(feature = "desktop", feature = "web"))]
    EventLoop(winit::error::EventLoopError),
    #[cfg(any(feature = "desktop", feature = "web"))]
    Os(winit::error::OsError),
}

impl fmt::Display for DesktopRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Engine(error) => write!(f, "engine error: {error}"),
            Self::Render(error) => write!(f, "render error: {error}"),
            #[cfg(any(feature = "desktop", feature = "web"))]
            Self::EventLoop(error) => write!(f, "event loop error: {error}"),
            #[cfg(any(feature = "desktop", feature = "web"))]
            Self::Os(error) => write!(f, "os error: {error}"),
        }
    }
}

impl std::error::Error for DesktopRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Engine(error) => Some(error),
            Self::Render(error) => Some(error),
            #[cfg(any(feature = "desktop", feature = "web"))]
            Self::EventLoop(error) => Some(error),
            #[cfg(any(feature = "desktop", feature = "web"))]
            Self::Os(error) => Some(error),
        }
    }
}

impl From<seishin_core::EngineError> for DesktopRuntimeError {
    fn from(value: seishin_core::EngineError) -> Self {
        Self::Engine(value)
    }
}

impl From<RenderError> for DesktopRuntimeError {
    fn from(value: RenderError) -> Self {
        Self::Render(value)
    }
}

#[cfg(any(feature = "desktop", feature = "web"))]
impl From<winit::error::EventLoopError> for DesktopRuntimeError {
    fn from(value: winit::error::EventLoopError) -> Self {
        Self::EventLoop(value)
    }
}

#[cfg(any(feature = "desktop", feature = "web"))]
impl From<winit::error::OsError> for DesktopRuntimeError {
    fn from(value: winit::error::OsError) -> Self {
        Self::Os(value)
    }
}
