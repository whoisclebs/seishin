use std::path::PathBuf;

#[cfg(all(not(target_arch = "wasm32"), feature = "kira-backend"))]
use std::collections::HashMap;

#[cfg(all(not(target_arch = "wasm32"), feature = "kira-backend"))]
use kira::{
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween,
};

use crate::{
    AudioError, AudioSkipReason, PlaybackControlResult, PlaybackHandle, PlaybackResult,
    PlaybackSettings,
};

#[cfg(all(not(target_arch = "wasm32"), feature = "kira-backend"))]
pub(crate) struct AudioBackend {
    manager: AudioManager<DefaultBackend>,
    sounds: HashMap<u64, StaticSoundData>,
    playbacks: HashMap<u64, StaticSoundHandle>,
}

#[cfg(any(target_arch = "wasm32", not(feature = "kira-backend")))]
pub(crate) struct AudioBackend;

#[cfg(all(not(target_arch = "wasm32"), feature = "kira-backend"))]
impl AudioBackend {
    pub(crate) fn new() -> Result<Self, String> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|error| error.to_string())?;

        Ok(Self {
            manager,
            sounds: HashMap::new(),
            playbacks: HashMap::new(),
        })
    }

    pub(crate) fn load_sound(&mut self, id: u64, path: PathBuf) -> Result<(), AudioError> {
        let sound = StaticSoundData::from_file(&path).map_err(|error| AudioError::Decode {
            path,
            reason: error.to_string(),
        })?;

        self.sounds.insert(id, sound);
        Ok(())
    }

    pub(crate) fn play_sound(
        &mut self,
        id: u64,
        playback: PlaybackHandle,
        settings: PlaybackSettings,
    ) -> PlaybackResult {
        let Some(sound) = self.sounds.get(&id) else {
            return PlaybackResult::Skipped(AudioSkipReason::PlaybackFailed(
                "sound data was not loaded".to_string(),
            ));
        };

        let mut sound = sound.volume(linear_volume_to_decibels(settings.volume));
        if settings.looping {
            sound = sound.loop_region(..);
        }

        match self.manager.play(sound) {
            Ok(handle) => {
                self.playbacks.insert(playback.id(), handle);
                PlaybackResult::Started(playback)
            }
            Err(error) => {
                PlaybackResult::Skipped(AudioSkipReason::PlaybackFailed(error.to_string()))
            }
        }
    }

    pub(crate) fn pause_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        let Some(handle) = self.playbacks.get_mut(&playback.id()) else {
            return PlaybackControlResult::Missing(playback);
        };

        handle.pause(Tween::default());
        PlaybackControlResult::Applied
    }

    pub(crate) fn resume_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        let Some(handle) = self.playbacks.get_mut(&playback.id()) else {
            return PlaybackControlResult::Missing(playback);
        };

        handle.resume(Tween::default());
        PlaybackControlResult::Applied
    }

    pub(crate) fn stop_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        let Some(mut handle) = self.playbacks.remove(&playback.id()) else {
            return PlaybackControlResult::Missing(playback);
        };

        handle.stop(Tween::default());
        PlaybackControlResult::Applied
    }

    pub(crate) fn set_playback_volume(
        &mut self,
        playback: PlaybackHandle,
        volume: f32,
    ) -> PlaybackControlResult {
        let Some(handle) = self.playbacks.get_mut(&playback.id()) else {
            return PlaybackControlResult::Missing(playback);
        };

        handle.set_volume(linear_volume_to_decibels(volume), Tween::default());
        PlaybackControlResult::Applied
    }

    pub(crate) fn set_master_volume(&mut self, volume: f32) -> PlaybackControlResult {
        self.manager
            .main_track()
            .set_volume(linear_volume_to_decibels(volume), Tween::default());
        PlaybackControlResult::Applied
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "kira-backend"))]
fn linear_volume_to_decibels(volume: f32) -> Decibels {
    if volume <= 0.0 {
        return Decibels::SILENCE;
    }

    Decibels(20.0 * volume.log10())
}

#[cfg(any(target_arch = "wasm32", not(feature = "kira-backend")))]
impl AudioBackend {
    pub(crate) fn new() -> Result<Self, String> {
        Err("audio backend feature is disabled for this target".to_string())
    }

    pub(crate) fn load_sound(&mut self, _id: u64, _path: PathBuf) -> Result<(), AudioError> {
        Ok(())
    }

    pub(crate) fn play_sound(
        &mut self,
        _id: u64,
        _playback: PlaybackHandle,
        _settings: PlaybackSettings,
    ) -> PlaybackResult {
        PlaybackResult::Skipped(AudioSkipReason::BackendUnavailable(
            "audio backend is not available on wasm yet".to_string(),
        ))
    }

    pub(crate) fn pause_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        PlaybackControlResult::Missing(playback)
    }

    pub(crate) fn resume_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        PlaybackControlResult::Missing(playback)
    }

    pub(crate) fn stop_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        PlaybackControlResult::Missing(playback)
    }

    pub(crate) fn set_playback_volume(
        &mut self,
        playback: PlaybackHandle,
        _volume: f32,
    ) -> PlaybackControlResult {
        PlaybackControlResult::Missing(playback)
    }

    pub(crate) fn set_master_volume(&mut self, _volume: f32) -> PlaybackControlResult {
        PlaybackControlResult::Applied
    }
}
