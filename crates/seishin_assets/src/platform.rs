use std::path::PathBuf;

use crate::AssetError;

#[cfg(not(target_arch = "wasm32"))]
pub fn asset_root(path: &std::path::Path) -> Result<PathBuf, AssetError> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|_| AssetError::InvalidAssetRoot(path.to_path_buf()))?;

    if !canonical.is_dir() {
        return Err(AssetError::InvalidAssetRoot(canonical));
    }

    Ok(canonical)
}

#[cfg(target_arch = "wasm32")]
pub fn asset_root(path: &std::path::Path) -> Result<PathBuf, AssetError> {
    Ok(path.to_path_buf())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_bytes(path: &std::path::Path) -> Result<Vec<u8>, AssetError> {
    std::fs::read(path).map_err(|error| map_io_error(path.to_path_buf(), error))
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub fn read_bytes(path: &std::path::Path) -> Result<Vec<u8>, AssetError> {
    let key = web_path(path);
    WEB_ASSET_CACHE.with(|cache| {
        cache
            .borrow()
            .get(&key)
            .cloned()
            .ok_or_else(|| AssetError::NotFound(path.to_path_buf()))
    })
}

#[cfg(all(target_arch = "wasm32", not(feature = "web")))]
pub fn read_bytes(path: &std::path::Path) -> Result<Vec<u8>, AssetError> {
    Err(AssetError::NotFound(path.to_path_buf()))
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub async fn preload_web_assets(paths: &[String]) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen_futures::JsFuture;

    let assets = futures_util::future::try_join_all(paths.iter().map(|path| async move {
        let response = fetch_response(path).await?;
        let buffer = JsFuture::from(response.array_buffer()?).await?;
        let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
        Ok::<_, wasm_bindgen::JsValue>((path.to_string(), bytes))
    }))
    .await?;

    for (path, bytes) in assets {
        WEB_ASSET_CACHE.with(|cache| {
            cache.borrow_mut().insert(path, bytes);
        });
    }

    Ok(())
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
thread_local! {
    static WEB_ASSET_CACHE: std::cell::RefCell<std::collections::HashMap<String, Vec<u8>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
async fn fetch_response(path: &str) -> Result<web_sys::Response, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window =
        web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("window unavailable"))?;
    let response = JsFuture::from(window.fetch_with_str(path)).await?;
    let response = response.dyn_into::<web_sys::Response>()?;
    if response.ok() {
        Ok(response)
    } else {
        Err(wasm_bindgen::JsValue::from_str(&format!(
            "failed to fetch {path}: HTTP {}",
            response.status()
        )))
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn web_path(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn canonical_asset_path(
    root: &std::path::Path,
    joined: PathBuf,
) -> Result<PathBuf, AssetError> {
    if !joined.exists() {
        return Err(AssetError::NotFound(joined));
    }

    let canonical = std::fs::canonicalize(&joined).map_err(|error| map_io_error(joined, error))?;

    if !canonical.starts_with(root) {
        return Err(AssetError::PathOutsideRoot(canonical));
    }

    Ok(canonical)
}

#[cfg(target_arch = "wasm32")]
pub fn canonical_asset_path(
    _root: &std::path::Path,
    joined: PathBuf,
) -> Result<PathBuf, AssetError> {
    Ok(joined)
}

#[cfg(not(target_arch = "wasm32"))]
fn map_io_error(path: PathBuf, error: std::io::Error) -> AssetError {
    match error.kind() {
        std::io::ErrorKind::NotFound => AssetError::NotFound(path),
        kind => AssetError::Io { path, kind },
    }
}
