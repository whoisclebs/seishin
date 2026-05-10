use std::path::PathBuf;

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
use std::path::Path;

#[cfg(any(
    all(not(target_arch = "wasm32"), feature = "kira-backend"),
    all(target_arch = "wasm32", feature = "web")
))]
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

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub(crate) struct AudioBackend {
    sounds: HashMap<u64, String>,
    playbacks: HashMap<u64, WebPlayback>,
    master_volume: f32,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
struct WebPlayback {
    element: web_sys::HtmlAudioElement,
    volume: f32,
}

#[cfg(any(
    all(target_arch = "wasm32", not(feature = "web")),
    all(not(target_arch = "wasm32"), not(feature = "kira-backend"))
))]
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

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl AudioBackend {
    pub(crate) fn new() -> Result<Self, String> {
        Ok(Self {
            sounds: HashMap::new(),
            playbacks: HashMap::new(),
            master_volume: 1.0,
        })
    }

    pub(crate) fn load_sound(&mut self, id: u64, path: PathBuf) -> Result<(), AudioError> {
        if !is_supported_web_audio_path(&path) {
            return Err(AudioError::Decode {
                path,
                reason: "unsupported web audio format".to_string(),
            });
        }

        self.sounds.insert(id, web_audio_url(&path));
        Ok(())
    }

    pub(crate) fn play_sound(
        &mut self,
        id: u64,
        playback: PlaybackHandle,
        settings: PlaybackSettings,
    ) -> PlaybackResult {
        let Some(source) = self.sounds.get(&id) else {
            return PlaybackResult::Skipped(AudioSkipReason::PlaybackFailed(
                "sound source was not loaded".to_string(),
            ));
        };

        let element = match web_sys::HtmlAudioElement::new_with_src(source) {
            Ok(element) => element,
            Err(error) => {
                return PlaybackResult::Skipped(AudioSkipReason::PlaybackFailed(
                    js_value_to_string(error),
                ));
            }
        };

        element.set_preload("auto");
        element.set_loop(settings.looping);
        element.set_volume(effective_web_volume(settings.volume, self.master_volume));

        match element.play() {
            Ok(_) => {
                self.playbacks.insert(
                    playback.id(),
                    WebPlayback {
                        element,
                        volume: settings.volume,
                    },
                );
                PlaybackResult::Started(playback)
            }
            Err(error) => PlaybackResult::Skipped(AudioSkipReason::PlaybackFailed(format!(
                "{}; call play from a browser user gesture when autoplay is blocked",
                js_value_to_string(error)
            ))),
        }
    }

    pub(crate) fn pause_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        let Some(playback_state) = self.playbacks.get(&playback.id()) else {
            return PlaybackControlResult::Missing(playback);
        };

        match playback_state.element.pause() {
            Ok(()) => PlaybackControlResult::Applied,
            Err(error) => PlaybackControlResult::PlaybackFailed(js_value_to_string(error)),
        }
    }

    pub(crate) fn resume_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        let Some(playback_state) = self.playbacks.get(&playback.id()) else {
            return PlaybackControlResult::Missing(playback);
        };

        match playback_state.element.play() {
            Ok(_) => PlaybackControlResult::Applied,
            Err(error) => PlaybackControlResult::PlaybackFailed(format!(
                "{}; call resume from a browser user gesture when autoplay is blocked",
                js_value_to_string(error)
            )),
        }
    }

    pub(crate) fn stop_playback(&mut self, playback: PlaybackHandle) -> PlaybackControlResult {
        let Some(playback_state) = self.playbacks.remove(&playback.id()) else {
            return PlaybackControlResult::Missing(playback);
        };

        if let Err(error) = playback_state.element.pause() {
            return PlaybackControlResult::PlaybackFailed(js_value_to_string(error));
        }

        playback_state.element.set_current_time(0.0);
        PlaybackControlResult::Applied
    }

    pub(crate) fn set_playback_volume(
        &mut self,
        playback: PlaybackHandle,
        volume: f32,
    ) -> PlaybackControlResult {
        let Some(playback_state) = self.playbacks.get_mut(&playback.id()) else {
            return PlaybackControlResult::Missing(playback);
        };

        playback_state.volume = volume;
        playback_state
            .element
            .set_volume(effective_web_volume(volume, self.master_volume));
        PlaybackControlResult::Applied
    }

    pub(crate) fn set_master_volume(&mut self, volume: f32) -> PlaybackControlResult {
        self.master_volume = volume;
        for playback in self.playbacks.values() {
            playback
                .element
                .set_volume(effective_web_volume(playback.volume, self.master_volume));
        }
        PlaybackControlResult::Applied
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn web_audio_url(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn effective_web_volume(playback_volume: f32, master_volume: f32) -> f64 {
    f64::from((playback_volume * master_volume).clamp(0.0, 1.0))
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn js_value_to_string(value: wasm_bindgen::JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "browser audio operation failed".to_string())
}

#[cfg(any(
    all(target_arch = "wasm32", not(feature = "web")),
    all(not(target_arch = "wasm32"), not(feature = "kira-backend"))
))]
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

#[cfg(any(test, all(target_arch = "wasm32", feature = "web")))]
fn is_supported_web_audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "aac" | "flac" | "m4a" | "mp3" | "oga" | "ogg" | "opus" | "wav" | "webm"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_audio_extension_filter_accepts_browser_formats_and_rejects_unknown() {
        assert!(is_supported_web_audio_path(Path::new("audio/beep.wav")));
        assert!(is_supported_web_audio_path(Path::new("audio/music.ogg")));
        assert!(is_supported_web_audio_path(Path::new("audio/theme.mp3")));
        assert!(!is_supported_web_audio_path(Path::new("audio/raw.bin")));
        assert!(!is_supported_web_audio_path(Path::new(
            "audio/no_extension"
        )));
    }
}
