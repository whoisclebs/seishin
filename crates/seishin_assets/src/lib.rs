mod error;
mod handle;
mod image;
mod loader;
mod path;
mod platform;

pub use error::AssetError;
pub use handle::{AssetHandle, ImageAsset};
pub use image::ImageData;
pub use loader::AssetLoader;
pub use path::{AssetPath, AssetRoot};

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub use platform::preload_web_assets;

pub(crate) use image::decode_image;
