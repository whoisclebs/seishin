use std::path::PathBuf;

use seishin_assets::AssetError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioError {
    Asset(AssetError),
    Decode { path: PathBuf, reason: String },
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Asset(error) => write!(f, "{error}"),
            Self::Decode { path, reason } => {
                write!(f, "failed to decode audio {}: {reason}", path.display())
            }
        }
    }
}

impl std::error::Error for AudioError {}

impl From<AssetError> for AudioError {
    fn from(error: AssetError) -> Self {
        Self::Asset(error)
    }
}
