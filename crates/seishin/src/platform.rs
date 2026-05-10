#![cfg_attr(not(feature = "logging"), allow(dead_code))]

use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::Once,
};

type PlatformResult<T> = Result<T, Box<dyn Error>>;

#[cfg(not(target_arch = "wasm32"))]
pub fn project_path(path: &Path) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(path)
}

#[cfg(target_arch = "wasm32")]
pub fn project_path(path: &Path) -> std::io::Result<PathBuf> {
    Ok(path.to_path_buf())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn discover_project_file() -> PlatformResult<PathBuf> {
    let current_dir = std::env::current_dir()?;

    for directory in current_dir.ancestors() {
        let candidate = directory.join("Seishin.toml");

        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err("Seishin.toml not found. Expected a Seishin.toml file in the current directory or a parent directory. Use App::from_project(path) for an explicit project path.".into())
}

#[cfg(target_arch = "wasm32")]
pub fn discover_project_file() -> PlatformResult<PathBuf> {
    Ok(PathBuf::from("Seishin.toml"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_to_string(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

#[cfg(target_arch = "wasm32")]
pub fn read_to_string(path: &Path) -> std::io::Result<String> {
    let key = web_path(path);
    WEB_RESOURCE_CACHE.with(|cache| {
        cache
            .borrow()
            .get(&key)
            .cloned()
            .ok_or_else(|| std::io::ErrorKind::NotFound.into())
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_string(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, contents)
}

#[cfg(target_arch = "wasm32")]
pub fn write_string(_path: &Path, _contents: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "writing user data is not available on wasm yet",
    ))
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub async fn preload_web_resources(paths: &[String]) -> Result<(), wasm_bindgen::JsValue> {
    let mut resources = Vec::with_capacity(paths.len());

    for path in paths {
        let text = fetch_text(path).await?;
        resources.push((path.to_string(), text));
    }

    for (path, text) in resources {
        WEB_RESOURCE_CACHE.with(|cache| {
            cache.borrow_mut().insert(path, text);
        });
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static WEB_RESOURCE_CACHE: std::cell::RefCell<std::collections::HashMap<String, String>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
async fn fetch_text(path: &str) -> Result<String, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window =
        web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("window unavailable"))?;
    let response = JsFuture::from(window.fetch_with_str(path)).await?;
    let response = response.dyn_into::<web_sys::Response>()?;
    if !response.ok() {
        return Err(wasm_bindgen::JsValue::from_str(&format!(
            "failed to fetch {path}: HTTP {}",
            response.status()
        )));
    }

    let text = JsFuture::from(response.text()?).await?;
    text.as_string()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("fetch response was not text"))
}

#[cfg(target_arch = "wasm32")]
fn web_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn ensure_readable_file(path: &Path) -> std::io::Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(std::io::ErrorKind::NotFound.into())
    }
}

#[cfg(target_arch = "wasm32")]
pub fn ensure_readable_file(path: &Path) -> std::io::Result<()> {
    read_to_string(path).map(|_| ())
}

pub fn install_logging(init: &'static Once, default_filter: String) {
    #[cfg(feature = "logging")]
    init.call_once(move || {
        use tracing_subscriber::EnvFilter;

        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

        #[cfg(target_arch = "wasm32")]
        let subscriber = tracing_subscriber::fmt().without_time();

        #[cfg(not(target_arch = "wasm32"))]
        let subscriber = tracing_subscriber::fmt();

        let _ = subscriber.with_env_filter(env_filter).try_init();
    });

    #[cfg(not(feature = "logging"))]
    {
        let _ = init;
        let _ = default_filter;
    }
}
