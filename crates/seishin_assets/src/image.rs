use std::path::Path;

use crate::AssetError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageData {
    width: u32,
    height: u32,
    pixels_rgba8: Vec<u8>,
}

impl ImageData {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels_rgba8(&self) -> &[u8] {
        &self.pixels_rgba8
    }
}

pub(crate) fn decode_image(path: &Path, bytes: &[u8]) -> Result<ImageData, AssetError> {
    #[cfg(feature = "png")]
    {
        let image = image::load_from_memory(bytes)
            .map_err(|_| AssetError::ImageDecode(path.to_path_buf()))?;
        let rgba = image.into_rgba8();

        Ok(ImageData {
            width: rgba.width(),
            height: rgba.height(),
            pixels_rgba8: rgba.into_raw(),
        })
    }

    #[cfg(not(feature = "png"))]
    {
        let _ = bytes;
        Err(AssetError::ImageDecode(path.to_path_buf()))
    }
}
