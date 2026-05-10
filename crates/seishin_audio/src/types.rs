use seishin_assets::{AssetHandle, AssetPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundAsset;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioCommand {
    LoadSound { path: AssetPath },
    PlaySound { sound: AssetHandle<SoundAsset> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackResult {
    Started,
    Skipped(AudioSkipReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioSkipReason {
    BackendUnavailable(String),
    SoundNotLoaded(AssetHandle<SoundAsset>),
    PlaybackFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_command_can_reference_loaded_sound_handle() {
        let command = AudioCommand::PlaySound {
            sound: AssetHandle::from_id(7),
        };

        assert_eq!(
            command,
            AudioCommand::PlaySound {
                sound: AssetHandle::from_id(7)
            }
        );
    }
}
