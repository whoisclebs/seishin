use std::path::{Component, Path, PathBuf};

use crate::{platform, AssetError};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPath(String);

impl AssetPath {
    pub fn new(path: impl AsRef<str>) -> Result<Self, AssetError> {
        let raw = path.as_ref();
        let path = Path::new(raw);

        if path.is_absolute() {
            return Err(AssetError::AbsolutePathRejected(raw.to_string()));
        }

        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                Component::Normal(part) => normalized.push(part),
                Component::CurDir => {}
                Component::ParentDir => {
                    return Err(AssetError::PathTraversalRejected(raw.to_string()));
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(AssetError::AbsolutePathRejected(raw.to_string()));
                }
            }
        }

        if normalized.as_os_str().is_empty() {
            normalized.push(".");
        }

        Ok(Self(normalized.to_string_lossy().replace('\\', "/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetRoot {
    path: PathBuf,
}

impl AssetRoot {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        Ok(Self {
            path: platform::asset_root(path.as_ref())?,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn resolve(&self, asset_path: &AssetPath) -> PathBuf {
        self.path.join(asset_path.as_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn asset_paths_are_normalized_without_leaving_root() {
        let path = AssetPath::new("sprites/./player.png").expect("valid asset path");

        assert_eq!(path.as_str(), "sprites/player.png");
    }

    #[test]
    fn asset_paths_reject_parent_directory_traversal() {
        let error = AssetPath::new("../secrets.png").expect_err("parent traversal must fail");

        assert_eq!(
            error,
            AssetError::PathTraversalRejected("../secrets.png".to_string())
        );
    }

    #[test]
    fn asset_paths_reject_absolute_paths() {
        let absolute_path = unique_test_dir().join("outside.png");
        let absolute_path = absolute_path.to_string_lossy().into_owned();

        let error = AssetPath::new(&absolute_path).expect_err("absolute paths must fail");

        assert_eq!(error, AssetError::AbsolutePathRejected(absolute_path));
    }

    fn unique_test_dir() -> PathBuf {
        let unique = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid clock")
            .as_nanos();
        std::env::temp_dir().join(format!("seishin_assets_test_{nanos}_{unique}"))
    }
}
