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

pub fn read_asset_bytes(root: &AssetRoot, asset_path: &AssetPath) -> Result<Vec<u8>, AssetError> {
    let joined = root.resolve(asset_path);
    let path = platform::canonical_asset_path(root.path(), joined)?;
    platform::read_bytes(&path)
}
