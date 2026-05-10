use std::{collections::BTreeMap, path::PathBuf};

use crate::{decode_image, AssetError, AssetPath, ImageData};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AssetBundle {
    bytes: BTreeMap<String, Vec<u8>>,
}

impl AssetBundle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, asset_path: AssetPath, bytes: Vec<u8>) -> Option<Vec<u8>> {
        self.bytes.insert(asset_path.as_str().to_string(), bytes)
    }

    pub fn contains(&self, asset_path: &AssetPath) -> bool {
        self.bytes.contains_key(asset_path.as_str())
    }

    pub fn read_bytes(&self, asset_path: &AssetPath) -> Result<&[u8], AssetError> {
        self.bytes
            .get(asset_path.as_str())
            .map(Vec::as_slice)
            .ok_or_else(|| AssetError::NotFound(bundle_path(asset_path)))
    }

    pub fn load_image(&self, asset_path: &AssetPath) -> Result<ImageData, AssetError> {
        decode_image(&bundle_path(asset_path), self.read_bytes(asset_path)?)
    }
}

fn bundle_path(asset_path: &AssetPath) -> PathBuf {
    PathBuf::from(asset_path.as_str())
}
