use std::collections::{HashMap, HashSet};

use seishin_assets::{AssetHandle, AssetPath, AssetRoot};

use crate::{
    backend::AudioBackend, platform, types::clamp_volume, AudioError, AudioSkipReason,
    PlaybackControlResult, PlaybackHandle, PlaybackInfo, PlaybackResult, PlaybackSettings,
    PlaybackState, SoundAsset,
};

pub struct AudioSystem {
    backend: Option<AudioBackend>,
    backend_error: Option<String>,
    next_sound_id: u64,
    next_playback_id: u64,
    loaded_sounds: HashSet<u64>,
    active_playbacks: HashMap<PlaybackHandle, PlaybackInfo>,
    master_volume: f32,
}

impl AudioSystem {
    pub fn new() -> Self {
        match AudioBackend::new() {
            Ok(backend) => Self {
                backend: Some(backend),
                backend_error: None,
                next_sound_id: 1,
                next_playback_id: 1,
                loaded_sounds: HashSet::new(),
                active_playbacks: HashMap::new(),
                master_volume: 1.0,
            },
            Err(error) => Self::without_backend(error),
        }
    }

    pub fn without_backend(reason: impl Into<String>) -> Self {
        Self {
            backend: None,
            backend_error: Some(reason.into()),
            next_sound_id: 1,
            next_playback_id: 1,
            loaded_sounds: HashSet::new(),
            active_playbacks: HashMap::new(),
            master_volume: 1.0,
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

    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    pub fn set_master_volume(&mut self, volume: f32) -> PlaybackControlResult {
        self.master_volume = clamp_volume(volume);

        match &mut self.backend {
            Some(backend) => backend.set_master_volume(self.master_volume),
            None => PlaybackControlResult::Applied,
        }
    }

    pub fn active_playback_count(&self) -> usize {
        self.active_playbacks.len()
    }

    pub fn playback(&self, playback: PlaybackHandle) -> Option<PlaybackInfo> {
        self.active_playbacks.get(&playback).copied()
    }

    pub fn play_sound(&mut self, sound: AssetHandle<SoundAsset>) -> PlaybackResult {
        self.play_sound_with(sound, PlaybackSettings::default())
    }

    pub fn play_sound_with(
        &mut self,
        sound: AssetHandle<SoundAsset>,
        settings: PlaybackSettings,
    ) -> PlaybackResult {
        if !self.loaded_sounds.contains(&sound.id()) {
            return PlaybackResult::Skipped(AudioSkipReason::SoundNotLoaded(sound));
        }

        if self.backend.is_none() {
            return PlaybackResult::Skipped(AudioSkipReason::BackendUnavailable(
                self.backend_error
                    .clone()
                    .unwrap_or_else(|| "audio backend is unavailable".to_string()),
            ));
        }

        let playback = self.next_playback_handle();
        let settings = settings.with_volume(settings.volume);
        let backend = self.backend.as_mut().expect("audio backend checked");
        let result = backend.play_sound(sound.id(), playback, settings);
        if let PlaybackResult::Started(playback) = result {
            self.active_playbacks.insert(
                playback,
                PlaybackInfo {
                    sound,
                    settings,
                    state: PlaybackState::Playing,
                },
            );
        }

        result
    }

    pub fn pause_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        if !self.active_playbacks.contains_key(&playback) {
            return PlaybackControlResult::Missing(playback);
        }

        let result = match &mut self.backend {
            Some(backend) => backend.pause_playback(playback),
            None => self.backend_unavailable_control(),
        };

        if result == PlaybackControlResult::Applied {
            if let Some(info) = self.active_playbacks.get_mut(&playback) {
                info.state = PlaybackState::Paused;
            }
        }

        result
    }

    pub fn resume_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        if !self.active_playbacks.contains_key(&playback) {
            return PlaybackControlResult::Missing(playback);
        }

        let result = match &mut self.backend {
            Some(backend) => backend.resume_playback(playback),
            None => self.backend_unavailable_control(),
        };

        if result == PlaybackControlResult::Applied {
            if let Some(info) = self.active_playbacks.get_mut(&playback) {
                info.state = PlaybackState::Playing;
            }
        }

        result
    }

    pub fn stop_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        if !self.active_playbacks.contains_key(&playback) {
            return PlaybackControlResult::Missing(playback);
        }

        let result = match &mut self.backend {
            Some(backend) => backend.stop_playback(playback),
            None => self.backend_unavailable_control(),
        };

        if result == PlaybackControlResult::Applied {
            self.active_playbacks.remove(&playback);
        }

        result
    }

    pub fn set_playback_volume(
        &mut self,
        playback: PlaybackHandle,
        volume: f32,
    ) -> PlaybackControlResult {
        if !self.active_playbacks.contains_key(&playback) {
            return PlaybackControlResult::Missing(playback);
        }

        let volume = clamp_volume(volume);
        let result = match &mut self.backend {
            Some(backend) => backend.set_playback_volume(playback, volume),
            None => self.backend_unavailable_control(),
        };

        if result == PlaybackControlResult::Applied {
            if let Some(info) = self.active_playbacks.get_mut(&playback) {
                info.settings = info.settings.with_volume(volume);
            }
        }

        result
    }

    fn next_playback_handle(&mut self) -> PlaybackHandle {
        let id = self.next_playback_id;
        self.next_playback_id += 1;
        PlaybackHandle::from_id(id)
    }

    fn backend_unavailable_control(&self) -> PlaybackControlResult {
        PlaybackControlResult::BackendUnavailable(
            self.backend_error
                .clone()
                .unwrap_or_else(|| "audio backend is unavailable".to_string()),
        )
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

    #[test]
    fn mixer_volume_is_backend_free_and_clamped() {
        let mut audio = AudioSystem::without_backend("test backend disabled");

        assert_eq!(audio.master_volume(), 1.0);
        assert_eq!(audio.set_master_volume(1.5), PlaybackControlResult::Applied);
        assert_eq!(audio.master_volume(), 1.0);
        assert_eq!(
            audio.set_master_volume(-0.25),
            PlaybackControlResult::Applied
        );
        assert_eq!(audio.master_volume(), 0.0);
        assert_eq!(
            audio.set_master_volume(f32::NAN),
            PlaybackControlResult::Applied
        );
        assert_eq!(audio.master_volume(), 0.0);
    }

    #[test]
    fn disabled_backend_skips_controlled_playback_without_allocating_handles() {
        let root_dir = unique_test_dir();
        let sound_path = root_dir.join("audio").join("beep.wav");
        fs::create_dir_all(sound_path.parent().expect("sound parent")).expect("create asset tree");
        fs::write(&sound_path, b"not decoded because backend is disabled").expect("write sound");

        let root = AssetRoot::new(&root_dir).expect("asset root");
        let path = AssetPath::new("audio/beep.wav").expect("asset path");
        let mut audio = AudioSystem::without_backend("test backend disabled");
        let sound = audio.load_sound(&root, &path).expect("sound registered");

        assert_eq!(
            audio.play_sound_with(sound, PlaybackSettings::default().looping(true)),
            PlaybackResult::Skipped(AudioSkipReason::BackendUnavailable(
                "test backend disabled".to_string()
            ))
        );
        assert_eq!(audio.active_playback_count(), 0);

        cleanup_test_dir(root_dir);
    }

    #[test]
    fn missing_playback_controls_are_safe() {
        let mut audio = AudioSystem::without_backend("test backend disabled");
        let playback = PlaybackHandle::from_id(77);

        assert_eq!(
            audio.pause_playback(playback),
            PlaybackControlResult::Missing(playback)
        );
        assert_eq!(
            audio.resume_playback(playback),
            PlaybackControlResult::Missing(playback)
        );
        assert_eq!(
            audio.stop_playback(playback),
            PlaybackControlResult::Missing(playback)
        );
        assert_eq!(
            audio.set_playback_volume(playback, 0.4),
            PlaybackControlResult::Missing(playback)
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
