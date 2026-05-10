use std::{io, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetError {
    InvalidAssetRoot(PathBuf),
    AbsolutePathRejected(String),
    PathTraversalRejected(String),
    NotFound(PathBuf),
    PathOutsideRoot(PathBuf),
    Io { path: PathBuf, kind: io::ErrorKind },
    ImageDecode(PathBuf),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAssetRoot(path) => write!(f, "invalid asset root: {}", path.display()),
            Self::AbsolutePathRejected(path) => {
                write!(f, "absolute asset paths are not allowed: {path}")
            }
            Self::PathTraversalRejected(path) => {
                write!(f, "asset path traversal is not allowed: {path}")
            }
            Self::NotFound(path) => write!(f, "asset file was not found: {}", path.display()),
            Self::PathOutsideRoot(path) => {
                write!(
                    f,
                    "resolved asset path escaped the approved root: {}",
                    path.display()
                )
            }
            Self::Io { path, kind } => {
                write!(f, "failed to access asset file {}: {kind}", path.display())
            }
            Self::ImageDecode(path) => write!(f, "failed to decode image: {}", path.display()),
        }
    }
}

impl std::error::Error for AssetError {}
