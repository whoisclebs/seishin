#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
mod desktop;
mod error;
mod headless;
mod time;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
mod web;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
pub use desktop::{run_desktop, DesktopGame, DesktopRunConfig, WindowConfig, WindowSize};
pub use error::DesktopRuntimeError;
pub use headless::{run_headless, HeadlessRunConfig};
pub use time::FixedTimestep;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub use web::{run_web as run_desktop, DesktopGame, DesktopRunConfig, WindowConfig, WindowSize};
