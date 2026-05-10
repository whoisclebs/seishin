use std::collections::HashSet;

use seishin_assets::{AssetHandle, AssetPath, AssetRoot};

use crate::{
    backend::AudioBackend, platform, AudioError, AudioSkipReason, PlaybackResult, SoundAsset,
};

pub struct AudioSystem {
    backend: Option<AudioBackend>,
    backend_error: Option<String>,
    next_sound_id: u64,
    loaded_sounds: HashSet<u64>,
}

impl AudioSystem {
    pub fn new() -> Self {
        match AudioBackend::new() {
            Ok(backend) => Self {
                backend: Some(backend),
                backend_error: None,
                next_sound_id: 1,
                loaded_sounds: HashSet::new(),
            },
            Err(error) => Self::without_backend(error),
        }
    }

    pub fn without_backend(reason: impl Into<String>) -> Self {
        Self {
            backend: None,
            backend_error: Some(reason.into()),
            next_sound_id: 1,
            loaded_sounds: HashSet::new(),
        }
    }

    pub fn is_backend_available(&self) -> bool {
        self.backend.is_some()
    }

    pub fn backend_error(&self) -> Option<&str> {
        self.backend_error.as_deref()
    }

    pub fn load_sound(
        &mut self,
        root: &AssetRoot,
        path: &AssetPath,
    ) -> Result<AssetHandle<SoundAsset>, AudioError> {
        let disk_path = platform::resolve_sound_asset(root, path)?;
        let id = self.next_sound_id;
        self.next_sound_id += 1;
        let handle = AssetHandle::from_id(id);

        if let Some(backend) = &mut self.backend {
            backend.load_sound(id, disk_path)?;
        }

        self.loaded_sounds.insert(id);
        Ok(handle)
    }

    pub fn play_sound(&mut self, sound: AssetHandle<SoundAsset>) -> PlaybackResult {
        if !self.loaded_sounds.contains(&sound.id()) {
            return PlaybackResult::Skipped(AudioSkipReason::SoundNotLoaded(sound));
        }

        match &mut self.backend {
            Some(backend) => backend.play_sound(sound.id()),
            None => PlaybackResult::Skipped(AudioSkipReason::BackendUnavailable(
                self.backend_error
                    .clone()
                    .unwrap_or_else(|| "audio backend is unavailable".to_string()),
            )),
        }
    }
}

impl Default for AudioSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seishin_assets::AssetError;
    use std::{fs, path::PathBuf};

    #[test]
    fn disabled_audio_loads_existing_asset_but_skips_playback() {
        let root_dir = unique_test_dir();
        let sound_path = root_dir.join("audio").join("beep.wav");
        fs::create_dir_all(sound_path.parent().expect("sound parent")).expect("create asset tree");
        fs::write(&sound_path, b"not decoded because backend is disabled").expect("write sound");

        let root = AssetRoot::new(&root_dir).expect("asset root");
        let path = AssetPath::new("audio/beep.wav").expect("asset path");
        let mut audio = AudioSystem::without_backend("no audio device");

        let sound = audio.load_sound(&root, &path).expect("sound registered");

        assert_eq!(sound.id(), 1);
        assert_eq!(
            audio.play_sound(sound),
            PlaybackResult::Skipped(AudioSkipReason::BackendUnavailable(
                "no audio device".to_string()
            ))
        );

        cleanup_test_dir(root_dir);
    }

    #[test]
    fn missing_sound_asset_returns_controlled_error() {
        let root_dir = unique_test_dir();
        fs::create_dir_all(&root_dir).expect("create root");

        let root = AssetRoot::new(&root_dir).expect("asset root");
        let path = AssetPath::new("audio/missing.wav").expect("asset path");
        let expected_path = root.path().join("audio").join("missing.wav");
        let mut audio = AudioSystem::without_backend("test backend disabled");

        let error = audio
            .load_sound(&root, &path)
            .expect_err("missing file fails");

        assert_eq!(
            error,
            AudioError::Asset(AssetError::NotFound(expected_path))
        );

        cleanup_test_dir(root_dir);
    }

    #[test]
    fn unloaded_sound_playback_is_skipped() {
        let mut audio = AudioSystem::without_backend("test backend disabled");
        let sound = AssetHandle::from_id(99);

        assert_eq!(
            audio.play_sound(sound),
            PlaybackResult::Skipped(AudioSkipReason::SoundNotLoaded(sound))
        );
    }

    fn unique_test_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("valid clock")
            .as_nanos();
        std::env::temp_dir().join(format!("seishin_audio_test_{nanos}"))
    }

    fn cleanup_test_dir(path: PathBuf) {
        let _ = fs::remove_dir_all(path);
    }
}
