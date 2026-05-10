use std::sync::Arc;

use seishin_core::Transform2D;

use crate::RenderError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClearColor {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl ClearColor {
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const CORNFLOWER: Self = Self::rgb(0.392, 0.584, 0.929);

    pub const fn rgb(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    #[cfg(feature = "wgpu-backend")]
    pub(crate) fn to_wgpu(self) -> wgpu::Color {
        wgpu::Color {
            r: self.red as f64,
            g: self.green as f64,
            b: self.blue as f64,
            a: self.alpha as f64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderSize {
    pub width: u32,
    pub height: u32,
}

impl RenderSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn is_zero(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera2D {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl Camera2D {
    pub fn world_to_ndc(self, world_x: f32, world_y: f32, viewport: RenderSize) -> [f32; 2] {
        if viewport.is_zero() {
            return [0.0, 0.0];
        }

        let half_width = viewport.width as f32 * 0.5;
        let half_height = viewport.height as f32 * 0.5;
        let camera_space_x = (world_x - self.x) * self.zoom;
        let camera_space_y = (world_y - self.y) * self.zoom;

        [camera_space_x / half_width, -camera_space_y / half_height]
    }
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(u64);

impl TextureId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureData {
    id: TextureId,
    width: u32,
    height: u32,
    pixels_rgba8: Arc<[u8]>,
}

impl TextureData {
    pub fn rgba8(
        id: TextureId,
        width: u32,
        height: u32,
        pixels_rgba8: Vec<u8>,
    ) -> Result<Self, RenderError> {
        let expected_len = width
            .checked_mul(height)
            .and_then(|pixel_count| pixel_count.checked_mul(4))
            .ok_or(RenderError::InvalidTextureData {
                id,
                reason: "texture dimensions overflowed RGBA byte count".to_string(),
            })? as usize;

        if width == 0 || height == 0 {
            return Err(RenderError::InvalidTextureData {
                id,
                reason: "texture dimensions must be greater than zero".to_string(),
            });
        }

        if pixels_rgba8.len() != expected_len {
            return Err(RenderError::InvalidTextureData {
                id,
                reason: format!(
                    "expected {expected_len} RGBA bytes, got {}",
                    pixels_rgba8.len()
                ),
            });
        }

        Ok(Self {
            id,
            width,
            height,
            pixels_rgba8: pixels_rgba8.into(),
        })
    }

    pub fn id(&self) -> TextureId {
        self.id
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels_rgba8(&self) -> &[u8] {
        self.pixels_rgba8.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    pub texture_id: TextureId,
    pub transform: Transform2D,
    pub width: f32,
    pub height: f32,
}

impl Sprite {
    pub fn new(texture_id: TextureId, transform: Transform2D, width: f32, height: f32) -> Self {
        Self {
            texture_id,
            transform,
            width,
            height,
        }
    }

    #[cfg(feature = "wgpu-backend")]
    pub(crate) fn corners(self) -> [(f32, f32); 4] {
        let half_width = self.width * self.transform.scale_x * 0.5;
        let half_height = self.height * self.transform.scale_y * 0.5;

        [
            (
                self.transform.x - half_width,
                self.transform.y - half_height,
            ),
            (
                self.transform.x + half_width,
                self.transform.y - half_height,
            ),
            (
                self.transform.x + half_width,
                self.transform.y + half_height,
            ),
            (
                self.transform.x - half_width,
                self.transform.y + half_height,
            ),
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderState<'a> {
    pub clear_color: ClearColor,
    pub camera: Camera2D,
    pub textures: &'a [TextureData],
    pub sprites: &'a [Sprite],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_defaults_to_origin_with_unit_zoom() {
        assert_eq!(
            Camera2D::default(),
            Camera2D {
                x: 0.0,
                y: 0.0,
                zoom: 1.0,
            }
        );
    }

    #[test]
    fn camera_projects_world_origin_to_screen_center() {
        let position = Camera2D::default().world_to_ndc(0.0, 0.0, RenderSize::new(800, 600));

        assert_eq!(position, [0.0, -0.0]);
    }

    #[test]
    fn camera_translation_and_zoom_affect_ndc_projection() {
        let camera = Camera2D {
            x: 100.0,
            y: 50.0,
            zoom: 2.0,
        };

        let position = camera.world_to_ndc(300.0, 125.0, RenderSize::new(800, 400));

        assert_eq!(position, [1.0, -0.75]);
    }

    #[test]
    fn texture_data_validates_expected_rgba_size() {
        let error = TextureData::rgba8(TextureId::new(7), 2, 2, vec![255; 15])
            .expect_err("invalid byte count must fail");

        assert_eq!(
            error,
            RenderError::InvalidTextureData {
                id: TextureId::new(7),
                reason: "expected 16 RGBA bytes, got 15".to_string(),
            }
        );
    }

    #[test]
    fn zero_sized_viewport_is_detected() {
        assert!(RenderSize::new(0, 10).is_zero());
        assert!(RenderSize::new(10, 0).is_zero());
        assert!(!RenderSize::new(10, 10).is_zero());
    }
}
