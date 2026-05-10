use seishin_assets::{AssetHandle, AssetPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundAsset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlaybackHandle(u64);

impl PlaybackHandle {
    pub const fn from_id(id: u64) -> Self {
        Self(id)
    }

    pub const fn id(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackSettings {
    pub volume: f32,
    pub looping: bool,
}

impl PlaybackSettings {
    pub const fn new() -> Self {
        Self {
            volume: 1.0,
            looping: false,
        }
    }

    pub fn with_volume(self, volume: f32) -> Self {
        Self {
            volume: clamp_volume(volume),
            ..self
        }
    }

    pub const fn looping(self, looping: bool) -> Self {
        Self { looping, ..self }
    }
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackInfo {
    pub sound: AssetHandle<SoundAsset>,
    pub settings: PlaybackSettings,
    pub state: PlaybackState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AudioCommand {
    LoadSound {
        path: AssetPath,
    },
    PlaySound {
        sound: AssetHandle<SoundAsset>,
        settings: PlaybackSettings,
    },
    StopPlayback {
        playback: PlaybackHandle,
    },
    PausePlayback {
        playback: PlaybackHandle,
    },
    ResumePlayback {
        playback: PlaybackHandle,
    },
    SetMasterVolume {
        volume: f32,
    },
    SetPlaybackVolume {
        playback: PlaybackHandle,
        volume: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackResult {
    Started(PlaybackHandle),
    Skipped(AudioSkipReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackControlResult {
    Applied,
    Missing(PlaybackHandle),
    BackendUnavailable(String),
    PlaybackFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioSkipReason {
    BackendUnavailable(String),
    SoundNotLoaded(AssetHandle<SoundAsset>),
    PlaybackFailed(String),
}

pub(crate) fn clamp_volume(volume: f32) -> f32 {
    if volume.is_nan() {
        return 0.0;
    }

    volume.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_command_can_reference_loaded_sound_handle() {
        let command = AudioCommand::PlaySound {
            sound: AssetHandle::from_id(7),
            settings: PlaybackSettings::default(),
        };

        assert_eq!(
            command,
            AudioCommand::PlaySound {
                sound: AssetHandle::from_id(7),
                settings: PlaybackSettings::default(),
            }
        );
    }

    #[test]
    fn playback_settings_clamp_volume_and_request_looping() {
        let settings = PlaybackSettings::default().with_volume(1.25).looping(true);

        assert_eq!(settings.volume, 1.0);
        assert!(settings.looping);
        assert_eq!(settings.with_volume(-0.5).volume, 0.0);
        assert_eq!(settings.with_volume(f32::NAN).volume, 0.0);
    }

    #[test]
    fn playback_handles_are_stable_ids() {
        let playback = PlaybackHandle::from_id(42);

        assert_eq!(playback.id(), 42);
        assert_eq!(playback, PlaybackHandle::from_id(42));
    }
}
