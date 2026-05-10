use std::fmt;

use crate::TextureId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    SurfaceCreation(String),
    AdapterUnavailable,
    DeviceRequest(String),
    NoSurfaceFormat,
    InvalidTextureData { id: TextureId, reason: String },
    MissingTexture(TextureId),
    SurfaceTimeout,
    SurfaceOutdated,
    SurfaceLost,
    SurfaceOutOfMemory,
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SurfaceCreation(message) => {
                write!(f, "failed to create render surface: {message}")
            }
            Self::AdapterUnavailable => write!(f, "no compatible GPU adapter was found"),
            Self::DeviceRequest(message) => write!(f, "failed to request GPU device: {message}"),
            Self::NoSurfaceFormat => write!(f, "surface did not report a supported texture format"),
            Self::InvalidTextureData { id, reason } => {
                write!(f, "invalid texture data for texture {}: {reason}", id.raw())
            }
            Self::MissingTexture(id) => write!(f, "sprite referenced unknown texture {}", id.raw()),
            Self::SurfaceTimeout => write!(f, "timed out while acquiring the next surface texture"),
            Self::SurfaceOutdated => write!(f, "surface became outdated and needs reconfiguration"),
            Self::SurfaceLost => write!(f, "surface was lost and needs reconfiguration"),
            Self::SurfaceOutOfMemory => write!(f, "renderer ran out of GPU memory"),
        }
    }
}

impl std::error::Error for RenderError {}
