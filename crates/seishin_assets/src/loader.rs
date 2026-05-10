use crate::{decode_image, platform, AssetError, AssetPath, AssetRoot, ImageData};

#[derive(Debug, Clone)]
pub struct AssetLoader {
    root: AssetRoot,
}

impl AssetLoader {
    pub fn new(root: AssetRoot) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &AssetRoot {
        &self.root
    }

    pub fn load_image(&self, asset_path: &AssetPath) -> Result<ImageData, AssetError> {
        let path = self.resolve_readable_path(asset_path)?;
        let bytes = platform::read_bytes(&path)?;

        decode_image(&path, &bytes)
    }

    fn resolve_readable_path(
        &self,
        asset_path: &AssetPath,
    ) -> Result<std::path::PathBuf, AssetError> {
        let joined = self.root.resolve(asset_path);
        platform::canonical_asset_path(self.root.path(), joined)
    }
}

#[cfg(test)]
#[cfg(feature = "png")]
mod tests {
    use super::*;
    use image::{codecs::png::PngEncoder, ColorType, ImageEncoder};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn valid_image_file_under_root_loads_without_gpu_dependencies() {
        let root_dir = unique_test_dir();
        let sprite_path = root_dir.join("sprites").join("player.png");

        fs::create_dir_all(sprite_path.parent().expect("sprite parent"))
            .expect("create asset tree");
        fs::write(&sprite_path, valid_png_bytes()).expect("write image fixture");

        let loader = AssetLoader::new(AssetRoot::new(&root_dir).expect("asset root"));
        let asset_path = AssetPath::new("sprites/player.png").expect("asset path");

        let image = loader.load_image(&asset_path).expect("image should load");

        assert_eq!(image.width(), 1);
        assert_eq!(image.height(), 1);
        assert_eq!(image.pixels_rgba8().len(), 4);

        cleanup_test_dir(root_dir);
    }

    #[test]
    fn asset_bytes_can_be_loaded_without_image_decoding() {
        let root_dir = unique_test_dir();
        let audio_path = root_dir.join("audio").join("beep.wav");

        fs::create_dir_all(audio_path.parent().expect("audio parent")).expect("create asset tree");
        fs::write(&audio_path, b"audio bytes").expect("write asset fixture");

        let root = AssetRoot::new(&root_dir).expect("asset root");
        let asset_path = AssetPath::new("audio/beep.wav").expect("asset path");

        let bytes = crate::read_asset_bytes(&root, &asset_path).expect("read asset bytes");

        assert_eq!(bytes, b"audio bytes");

        cleanup_test_dir(root_dir);
    }

    #[test]
    fn missing_files_return_controlled_error() {
        let root_dir = unique_test_dir();
        fs::create_dir_all(&root_dir).expect("create root");

        let loader = AssetLoader::new(AssetRoot::new(&root_dir).expect("asset root"));
        let asset_path = AssetPath::new("sprites/missing.png").expect("asset path");
        let expected_path = loader.root().path().join("sprites").join("missing.png");

        let error = loader
            .load_image(&asset_path)
            .expect_err("missing file must fail");

        assert_eq!(error, AssetError::NotFound(expected_path));

        cleanup_test_dir(root_dir);
    }

    #[test]
    fn invalid_image_bytes_return_controlled_decode_error() {
        let root_dir = unique_test_dir();
        let sprite_path = root_dir.join("sprites").join("corrupt.png");

        fs::create_dir_all(sprite_path.parent().expect("sprite parent"))
            .expect("create asset tree");
        fs::write(&sprite_path, b"not a png").expect("write corrupt image");

        let loader = AssetLoader::new(AssetRoot::new(&root_dir).expect("asset root"));
        let asset_path = AssetPath::new("sprites/corrupt.png").expect("asset path");
        let expected_path = loader.root().path().join("sprites").join("corrupt.png");

        let error = loader
            .load_image(&asset_path)
            .expect_err("decode must fail");

        assert_eq!(error, AssetError::ImageDecode(expected_path));

        cleanup_test_dir(root_dir);
    }

    #[test]
    fn asset_bundle_loads_bytes_and_images_without_filesystem_paths() {
        let asset_path = AssetPath::new("sprites/player.png").expect("asset path");
        let mut bundle = crate::AssetBundle::new();

        bundle.insert(asset_path.clone(), valid_png_bytes());

        let image = bundle.load_image(&asset_path).expect("image from bundle");

        assert_eq!(image.width(), 1);
        assert_eq!(image.height(), 1);
        assert!(!bundle
            .read_bytes(&asset_path)
            .expect("raw bytes")
            .is_empty());
    }

    fn unique_test_dir() -> PathBuf {
        let unique = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("valid clock")
            .as_nanos();
        std::env::temp_dir().join(format!("seishin_assets_test_{nanos}_{unique}"))
    }

    fn cleanup_test_dir(path: PathBuf) {
        let _ = fs::remove_dir_all(path);
    }

    fn valid_png_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(&[255, 0, 0, 255], 1, 1, ColorType::Rgba8)
            .expect("encode png fixture");
        bytes
    }
}
