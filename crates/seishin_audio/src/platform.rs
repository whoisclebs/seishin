use std::path::PathBuf;

use seishin_assets::{AssetError, AssetPath, AssetRoot};

#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_sound_asset(
    root: &AssetRoot,
    asset_path: &AssetPath,
) -> Result<PathBuf, AssetError> {
    let joined = root.resolve(asset_path);

    if !joined.exists() {
        return Err(AssetError::NotFound(joined));
    }

    let canonical = std::fs::canonicalize(&joined).map_err(|error| map_io_error(joined, error))?;

    if !canonical.starts_with(root.path()) {
        return Err(AssetError::PathOutsideRoot(canonical));
    }

    Ok(canonical)
}

#[cfg(target_arch = "wasm32")]
pub fn resolve_sound_asset(
    root: &AssetRoot,
    asset_path: &AssetPath,
) -> Result<PathBuf, AssetError> {
    Ok(root.resolve(asset_path))
}

#[cfg(not(target_arch = "wasm32"))]
fn map_io_error(path: PathBuf, error: std::io::Error) -> AssetError {
    match error.kind() {
        std::io::ErrorKind::NotFound => AssetError::NotFound(path),
        kind => AssetError::Io { path, kind },
    }
}
