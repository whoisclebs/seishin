mod app;
mod platform;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub use platform::preload_web_resources;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub use seishin_assets::preload_web_assets;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub use wasm_bindgen_futures::spawn_local;

pub use app::{
    run, ActiveDialogue, App, Assets, CharacterData, CharacterDialogueData, Component,
    ComponentFactory, ComponentRegistry, DialogueData, DialogueState, Entity, EntityMut,
    FrameContext, FrameWorld, Game2D, GameResult, GameplayInput, InputActions, InputQuery,
    LogLevel, RenderContext, ResourceToml, Resources, SpriteBuilder, SpriteBundle, SpriteRenderer,
    StartupContext, Texture, UiElement, Vec2, WorldComponentExt,
};
pub use seishin_render_graph::{NodeLabel, RenderGraph, RenderGraphError, RenderGraphRunner};
pub use seishin_world::{
    AudioRef, CustomComponentDocument, CustomComponentRef, EntityRecord, InstanceSource,
    PrefabDocument, ResolveError, ResolvedEntity, SceneAudioDocument, SceneChange, SceneDiff,
    SceneDiffError, SceneDiffSide, SceneDocument, SceneDocumentBuilder, SceneDocumentExport,
    SceneEntityBuilder, SceneEntityDocument, SceneExportOmission, SceneInstance,
    SceneInstanceDocument, SceneReloadError, SceneReloadQueue, SceneReloadReport,
    SceneReloadRequest, SceneReloadResult, SceneReloadUpdate, SceneSpriteDocument,
    SceneTransformDocument, SceneUiDocument, SceneUiImageDocument, SceneUiInteractionDocument,
    SceneUiLayoutDocument, SceneUiTextDocument, SpriteRef, TagsDocument, UiAnchor, UiImageRef,
    UiInteractionRef, UiLayoutRef, UiRef, UiTextRef, World, WorldError,
};

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[derive(Debug, serde::Deserialize)]
pub struct WebManifest {
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub assets: Vec<String>,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub async fn fetch_web_manifest(path: &str) -> Result<WebManifest, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window =
        web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("window unavailable"))?;
    let response = JsFuture::from(window.fetch_with_str(path)).await?;
    let response = response.dyn_into::<web_sys::Response>()?;
    if !response.ok() {
        return Err(wasm_bindgen::JsValue::from_str(&format!(
            "failed to fetch {path}: HTTP {}",
            response.status()
        )));
    }

    let text = JsFuture::from(response.text()?).await?;
    let text = text
        .as_string()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("web manifest response was not text"))?;
    serde_json::from_str::<WebManifest>(&text)
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}

pub mod assets {
    pub use seishin_assets::*;
}

pub mod audio {
    pub use seishin_audio::*;
}

pub mod core {
    pub use seishin_core::*;
}

pub mod input {
    pub use seishin_input::*;
}

pub mod physics {
    pub use seishin_physics::*;
}

pub mod render {
    pub use seishin_render::{
        Camera2D, ClearColor, RenderError, RenderSize, RenderState, Sprite, TextureData, TextureId,
    };
}

pub mod render_graph {
    pub use seishin_render_graph::*;
}

pub mod runtime {
    pub use seishin_runtime::*;
}

pub mod world {
    pub use seishin_world::*;
}

pub mod prelude {
    pub use crate::{
        run, ActiveDialogue, App, Assets, CharacterData, CharacterDialogueData, Component,
        ComponentFactory, ComponentRegistry, DialogueData, DialogueState, Entity, EntityMut,
        FrameContext, FrameWorld, Game2D, GameResult, GameplayInput, InputActions, InputQuery,
        LogLevel, RenderContext, ResourceToml, Resources, SpriteBuilder, SpriteBundle,
        SpriteRenderer, StartupContext, Texture, UiElement, Vec2, World, WorldComponentExt,
        WorldError,
    };
    pub use seishin_assets::{AssetHandle, AssetLoader, AssetPath, AssetRoot};
    pub use seishin_audio::{AudioSkipReason, AudioSystem, PlaybackResult, SoundAsset};
    pub use seishin_core::{
        Engine, EngineConfig, EngineError, EngineResult, EntityId, Game, Transform2D, UpdateContext,
    };
    pub use seishin_input::{InputState, KeyCode};
    pub use seishin_physics::Collider2D;
    pub use seishin_render::{
        Camera2D, ClearColor, RenderError, RenderSize, RenderState, Sprite, TextureData, TextureId,
    };
    pub use seishin_render_graph::{NodeLabel, RenderGraph, RenderGraphError, RenderGraphRunner};
    #[cfg(any(
        all(not(target_arch = "wasm32"), feature = "desktop"),
        all(target_arch = "wasm32", feature = "web")
    ))]
    pub use seishin_runtime::{
        run_desktop, run_headless, DesktopGame, DesktopRunConfig, DesktopRuntimeError,
        FixedTimestep, HeadlessRunConfig, WindowConfig, WindowSize,
    };
    #[cfg(not(any(
        all(not(target_arch = "wasm32"), feature = "desktop"),
        all(target_arch = "wasm32", feature = "web")
    )))]
    pub use seishin_runtime::{
        run_headless, DesktopRuntimeError, FixedTimestep, HeadlessRunConfig,
    };
    pub use seishin_world::{
        AudioRef, CustomComponentDocument, CustomComponentRef, EntityRecord, InstanceSource,
        PrefabDocument, ResolveError, ResolvedEntity, SceneAudioDocument, SceneChange, SceneDiff,
        SceneDiffError, SceneDiffSide, SceneDocument, SceneDocumentBuilder, SceneDocumentExport,
        SceneEntityBuilder, SceneEntityDocument, SceneExportOmission, SceneInstance,
        SceneInstanceDocument, SceneReloadError, SceneReloadQueue, SceneReloadReport,
        SceneReloadRequest, SceneReloadResult, SceneReloadUpdate, SceneSpriteDocument,
        SceneTransformDocument, SceneUiDocument, SceneUiImageDocument, SceneUiInteractionDocument,
        SceneUiLayoutDocument, SceneUiTextDocument, SpriteRef, TagsDocument, UiAnchor, UiImageRef,
        UiInteractionRef, UiLayoutRef, UiRef, UiTextRef,
    };
}

#[cfg(not(all(target_arch = "wasm32", feature = "web")))]
#[macro_export]
macro_rules! seishin_main {
    ($game:ty) => {
        #[cfg(not(target_arch = "wasm32"))]
        fn main() -> $crate::GameResult<()> {
            $crate::run::<$game>()
        }

        #[cfg(target_arch = "wasm32")]
        fn main() {}
    };
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
#[macro_export]
macro_rules! seishin_main {
    ($game:ty) => {
        fn main() {}

        #[wasm_bindgen::prelude::wasm_bindgen(start)]
        pub fn wasm_start() {
            $crate::spawn_local(async {
                let manifest = match $crate::fetch_web_manifest("web-manifest.json").await {
                    Ok(manifest) => manifest,
                    Err(error) => {
                        report_web_startup_error(&format!("seishin web manifest preload failed: {error:?}"));
                        return;
                    }
                };

                if let Err(error) = $crate::preload_web_resources(&manifest.resources).await {
                    report_web_startup_error(&format!(
                        "seishin web resource preload failed: {error:?}"
                    ));
                    return;
                }

                if let Err(error) = $crate::preload_web_assets(&manifest.assets).await {
                    report_web_startup_error(&format!(
                        "seishin web asset preload failed: {error:?}"
                    ));
                    return;
                }

                if let Err(error) = $crate::run::<$game>() {
                    report_web_startup_error(&format!("seishin web startup failed: {error}"));
                }
            });

            fn report_web_startup_error(message: &str) {
                web_sys::console::error_1(&message.into());

                if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                    if document.get_element_by_id("seishin-startup-error-style").is_none() {
                        if let Ok(style) = document.create_element("style") {
                            style.set_id("seishin-startup-error-style");
                            style.set_text_content(Some(
                                ".seishin-startup-error{margin:16px;padding:12px;color:#ffb4b4;background:#240909;border:1px solid #7f1d1d;white-space:pre-wrap;}",
                            ));
                            if let Some(head) = document.head() {
                                let _ = head.append_child(&style);
                            }
                        }
                    }

                    if let Ok(element) = document.create_element("pre") {
                        element.set_text_content(Some(message));
                        element.set_class_name("seishin-startup-error");
                        if let Some(body) = document.body() {
                            let _ = body.append_child(&element);
                        }
                    }
                }
            }
        }
    };
}
