#![cfg_attr(
    not(any(
        all(not(target_arch = "wasm32"), feature = "desktop"),
        all(target_arch = "wasm32", feature = "web")
    )),
    allow(dead_code)
)]

use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    error::Error,
    path::{Path, PathBuf},
    sync::Once,
};

use crate::platform;
use seishin_assets::{AssetHandle, AssetLoader, AssetPath, AssetRoot};
use seishin_audio::{AudioSystem, PlaybackResult, SoundAsset};
#[cfg(any(
    all(not(target_arch = "wasm32"), feature = "desktop"),
    all(target_arch = "wasm32", feature = "web")
))]
use seishin_core::EngineConfig;
use seishin_core::{Engine, EngineResult, EntityId, Game, Transform2D, UpdateContext};
use seishin_input::{InputState, KeyCode};
use seishin_render::{Camera2D, ClearColor, RenderState, Sprite, TextureData, TextureId};
#[cfg(test)]
use seishin_render_graph::NodeLabel;
use seishin_render_graph::{RenderGraph, RenderGraphError};
#[cfg(any(
    all(not(target_arch = "wasm32"), feature = "desktop"),
    all(target_arch = "wasm32", feature = "web")
))]
use seishin_runtime::{run_desktop, DesktopGame, DesktopRunConfig, FixedTimestep, WindowConfig};
use seishin_world::{
    resolve_scene_entity, CustomComponentRef, EntityRecord, PrefabDocument, ResolvedEntity,
    SceneDocument, SceneEntityDocument, UiInteractionRef, UiRef, World,
};
use serde::Deserialize;
#[cfg(feature = "logging")]
use tracing::{debug, info, warn};

pub type GameResult<T> = Result<T, Box<dyn Error>>;
pub type Entity = EntityId;

type AudioCache = HashMap<Entity, AssetHandle<SoundAsset>>;

#[derive(Debug, Clone, Deserialize)]
pub struct CharacterData {
    pub id: String,
    pub display_name: String,
    pub sprite: Option<String>,
    pub dialogue: Option<CharacterDialogueData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CharacterDialogueData {
    pub default: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DialogueData {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Resources {
    paths: ProjectPaths,
}

#[derive(Debug, Clone)]
pub struct ResourceToml {
    value: toml::Value,
}

impl ResourceToml {
    pub fn value(&self) -> &toml::Value {
        &self.value
    }

    pub fn get(&self, key: &str) -> Option<&toml::Value> {
        self.value.get(key)
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(toml::Value::as_str)
    }

    pub fn f32(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(|value| match value {
            toml::Value::Float(value) => Some(*value as f32),
            toml::Value::Integer(value) => Some(*value as f32),
            _ => None,
        })
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(toml::Value::as_bool)
    }
}

impl Resources {
    fn new(paths: ProjectPaths) -> Self {
        Self { paths }
    }

    pub fn character(&self, path: impl AsRef<str>) -> GameResult<CharacterData> {
        self.load(path)
    }

    pub fn dialogue(&self, path: impl AsRef<str>) -> GameResult<DialogueData> {
        self.load(path)
    }

    pub fn toml(&self, path: impl AsRef<str>) -> GameResult<ResourceToml> {
        Ok(ResourceToml {
            value: self.load(path)?,
        })
    }

    pub fn load<T>(&self, path: impl AsRef<str>) -> GameResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let path = path.as_ref();
        let resolved = self.paths.resolve_resource(path)?;
        let source = platform::read_to_string(&resolved).map_err(|error| {
            PathDiagnosticError::resource(
                path.to_string(),
                resolved.clone(),
                &self.paths.resource_root,
                error,
            )
        })?;

        toml::from_str(&source).map_err(|error| {
            PathDiagnosticError::resource(
                path.to_string(),
                resolved,
                &self.paths.resource_root,
                error,
            )
            .into()
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct DialogueState {
    active: Option<ActiveDialogue>,
}

impl DialogueState {
    pub fn open(&mut self, speaker: impl Into<String>, dialogue: DialogueData) {
        let active = ActiveDialogue {
            speaker: speaker.into(),
            dialogue,
        };
        #[cfg(feature = "logging")]
        info!(speaker = %active.speaker, text = %active.dialogue.text, "dialogue opened");
        self.active = Some(active);
    }

    pub fn close(&mut self) {
        #[cfg(feature = "logging")]
        if self.active.is_some() {
            info!("dialogue closed");
        }

        self.active = None;
    }

    pub fn advance_or_close(&mut self) {
        self.close();
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn active(&self) -> Option<&ActiveDialogue> {
        self.active.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct ActiveDialogue {
    pub speaker: String,
    pub dialogue: DialogueData,
}

static LOGGING_INIT: Once = Once::new();

#[derive(Debug, Clone)]
pub struct App {
    title: String,
    width: u32,
    height: u32,
    target_fps: u32,
    asset_root: PathBuf,
    resource_root: PathBuf,
    user_root: PathBuf,
    main_scene: Option<String>,
    clear_color: ClearColor,
    logging: LoggingConfig,
    input_actions: InputActions,
}

impl App {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 1280,
            height: 720,
            target_fps: 60,
            asset_root: PathBuf::from("assets"),
            resource_root: PathBuf::from("resources"),
            user_root: PathBuf::from("user"),
            main_scene: None,
            clear_color: ClearColor::BLACK,
            logging: LoggingConfig::default(),
            input_actions: InputActions::default(),
        }
    }

    pub fn from_project(path: impl AsRef<Path>) -> GameResult<Self> {
        let path = platform::project_path(path.as_ref())?;
        let project = ProjectConfig::from_path(&path)?;
        let project_dir = path.parent().unwrap_or_else(|| Path::new("."));

        Ok(Self::from_project_config(project, project_dir))
    }

    pub fn discover_project() -> GameResult<Self> {
        Self::from_project(platform::discover_project_file()?)
    }

    pub fn window_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn target_fps(mut self, target_fps: u32) -> Self {
        self.target_fps = target_fps;
        self
    }

    pub fn asset_root(mut self, asset_root: impl Into<PathBuf>) -> Self {
        self.asset_root = asset_root.into();
        self
    }

    pub fn resource_root(mut self, resource_root: impl Into<PathBuf>) -> Self {
        self.resource_root = resource_root.into();
        self
    }

    pub fn clear_color(mut self, clear_color: ClearColor) -> Self {
        self.clear_color = clear_color;
        self
    }

    pub fn with_default_logging(mut self) -> Self {
        self.logging.enabled = true;
        self
    }

    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.logging.enabled = true;
        self.logging.default_filter = level.as_filter().to_string();
        self
    }

    #[cfg(any(
        all(not(target_arch = "wasm32"), feature = "desktop"),
        all(target_arch = "wasm32", feature = "web")
    ))]
    pub fn run<G: Game2D>(self) -> GameResult<()> {
        self.logging.install();

        let paths = ProjectPaths::new(self.asset_root, self.resource_root, self.user_root);
        let _user_root = paths.user_root();
        let engine = Engine::new(EngineConfig::new(&self.title).with_target_fps(self.target_fps))?;
        if let Some(main_scene) = self.main_scene.as_deref() {
            validate_main_scene(main_scene, &paths)?;
        }

        let asset_root = AssetRoot::new(&paths.asset_root)?;
        let mut startup = StartupContext::new(
            asset_root,
            self.input_actions,
            self.clear_color,
            paths,
            self.main_scene,
        );

        if let Some(error) = startup.audio_backend_error() {
            #[cfg(feature = "logging")]
            warn!(%error, "audio unavailable, game will continue silently");
            #[cfg(not(feature = "logging"))]
            let _ = error;
        }

        let game = G::new(&mut startup)?;
        startup.load_main_scene()?;
        let runtime_parts = startup.into_runtime_parts();
        let adapter = Game2DAdapter::new(game, runtime_parts);

        run_desktop(
            engine,
            adapter,
            DesktopRunConfig::new(WindowConfig::new(self.title, self.width, self.height))
                .with_timestep(FixedTimestep::from_fps(self.target_fps)),
        )?;

        Ok(())
    }

    #[cfg(not(any(
        all(not(target_arch = "wasm32"), feature = "desktop"),
        all(target_arch = "wasm32", feature = "web")
    )))]
    pub fn run<G: Game2D>(self) -> GameResult<()> {
        let _ = self;
        Err("seishin runtime feature is disabled; enable `desktop` or `web` to run an app".into())
    }

    fn from_project_config(project: ProjectConfig, project_dir: &Path) -> Self {
        let game = project.game.unwrap_or_default();
        let window = project.window.unwrap_or_default();
        let assets = project.assets.unwrap_or_default();
        let resources = project.resources.unwrap_or_default();
        let user = project.user.unwrap_or_default();
        let logging = project.logging.unwrap_or_default();
        let input_actions = project
            .input
            .map(InputActions::from_config)
            .unwrap_or_default();

        let asset_root = project_dir.join(assets.root.unwrap_or_else(|| "assets".to_string()));
        let resource_root =
            project_dir.join(resources.root.unwrap_or_else(|| "resources".to_string()));
        let user_root = project_dir.join(user.root.unwrap_or_else(|| "user".to_string()));

        Self {
            title: game.name.unwrap_or_else(|| "seishin".to_string()),
            width: window.width.unwrap_or(1280),
            height: window.height.unwrap_or(720),
            target_fps: window.target_fps.unwrap_or(60),
            asset_root,
            resource_root,
            user_root,
            main_scene: game.main_scene,
            clear_color: window
                .clear_color
                .as_deref()
                .and_then(parse_clear_color)
                .unwrap_or(ClearColor::BLACK),
            logging: LoggingConfig {
                enabled: true,
                default_filter: logging.default_filter.unwrap_or_else(|| "info".to_string()),
            },
            input_actions,
        }
    }
}

#[cfg(any(
    all(not(target_arch = "wasm32"), feature = "desktop"),
    all(target_arch = "wasm32", feature = "web")
))]
pub fn run<G: Game2D>() -> GameResult<()> {
    App::discover_project()?.run::<G>()
}

#[cfg(not(any(
    all(not(target_arch = "wasm32"), feature = "desktop"),
    all(target_arch = "wasm32", feature = "web")
)))]
pub fn run<G: Game2D>() -> GameResult<()> {
    Err("seishin runtime feature is disabled; enable `desktop` or `web` to run an app".into())
}

#[derive(Debug, Clone)]
struct LoggingConfig {
    enabled: bool,
    default_filter: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_filter: "info".to_string(),
        }
    }
}

impl LoggingConfig {
    fn install(&self) {
        if !self.enabled {
            return;
        }

        platform::install_logging(&LOGGING_INIT, self.default_filter.clone());
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn as_filter(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

pub trait Game2D: Sized + 'static {
    fn new(context: &mut StartupContext) -> GameResult<Self>;

    fn update(&mut self, _context: &mut FrameContext<'_>) -> GameResult<()> {
        Ok(())
    }

    fn render(&self, _context: &mut RenderContext) {}

    fn shutdown(&mut self) -> GameResult<()> {
        Ok(())
    }
}

pub struct StartupContext {
    assets: Assets,
    audio: AudioSystem,
    audio_cache: AudioCache,
    world: World,
    render_cache: RenderCache,
    components: ComponentRegistry,
    component_instances: Vec<RuntimeComponent>,
    schedule: Schedule,
    paths: ProjectPaths,
    main_scene: Option<String>,
    main_scene_loaded: bool,
    input_actions: InputActions,
    clear_color: ClearColor,
}

impl StartupContext {
    fn new(
        asset_root: AssetRoot,
        input_actions: InputActions,
        clear_color: ClearColor,
        paths: ProjectPaths,
        main_scene: Option<String>,
    ) -> Self {
        Self {
            assets: Assets::new(asset_root),
            audio: default_audio_system(),
            audio_cache: AudioCache::default(),
            world: World::default(),
            render_cache: RenderCache::default(),
            components: ComponentRegistry::default(),
            component_instances: Vec::new(),
            schedule: Schedule::default(),
            paths,
            main_scene,
            main_scene_loaded: false,
            input_actions,
            clear_color,
        }
    }

    pub fn assets(&mut self) -> &mut Assets {
        &mut self.assets
    }

    pub fn world(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn components(&mut self) -> &mut ComponentRegistry {
        &mut self.components
    }

    pub fn schedule(&mut self) -> &mut Schedule {
        &mut self.schedule
    }

    pub fn load_main_scene(&mut self) -> GameResult<()> {
        if self.main_scene_loaded {
            return Ok(());
        }

        let Some(main_scene) = self.main_scene.clone() else {
            return Ok(());
        };

        load_main_scene(&main_scene, self)?;
        self.main_scene_loaded = true;

        Ok(())
    }

    pub fn spawn(&mut self, bundle: SpriteBundle) -> GameResult<Entity> {
        let renderer = SpriteRenderer::new(bundle.texture, bundle.size);
        let entity = self.world.spawn(EntityRecord {
            transform: bundle.transform,
            ..EntityRecord::default()
        });
        self.render_cache.insert(entity, renderer);
        Ok(entity)
    }

    pub fn sprite(&mut self, texture_path: impl Into<String>) -> SpriteBuilder<'_> {
        SpriteBuilder::new(self, texture_path.into())
    }

    pub fn load_texture(&mut self, path: impl AsRef<str>) -> GameResult<Texture> {
        self.assets.load_texture(path)
    }

    pub fn load_sound(&mut self, path: impl AsRef<str>) -> GameResult<AssetHandle<SoundAsset>> {
        self.assets.sound(&mut self.audio, path)
    }

    pub fn sound(&mut self, path: impl AsRef<str>) -> GameResult<AssetHandle<SoundAsset>> {
        self.load_sound(path)
    }

    pub fn audio_backend_error(&self) -> Option<&str> {
        self.audio.backend_error()
    }

    fn into_runtime_parts(self) -> RuntimeParts {
        RuntimeParts {
            audio: self.audio,
            audio_cache: self.audio_cache,
            world: self.world,
            render_cache: self.render_cache,
            input_actions: self.input_actions,
            resources: Resources::new(self.paths),
            dialogue: DialogueState::default(),
            component_instances: self.component_instances,
            schedule: self.schedule,
            clear_color: self.clear_color,
        }
    }
}

struct RuntimeParts {
    audio: AudioSystem,
    audio_cache: AudioCache,
    world: World,
    render_cache: RenderCache,
    input_actions: InputActions,
    resources: Resources,
    dialogue: DialogueState,
    component_instances: Vec<RuntimeComponent>,
    schedule: Schedule,
    clear_color: ClearColor,
}

pub trait Component {
    fn update(&mut self, entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchedulePhase {
    Startup,
    Update,
    PostUpdate,
}

type ScheduleSystem =
    Box<dyn for<'frame> FnMut(&mut FrameContext<'frame>) -> GameResult<()> + 'static>;

struct ScheduleSystemRegistration {
    name: String,
    system: ScheduleSystem,
}

#[derive(Default)]
pub struct Schedule {
    startup_complete: bool,
    startup: Vec<ScheduleSystemRegistration>,
    update: Vec<ScheduleSystemRegistration>,
    post_update: Vec<ScheduleSystemRegistration>,
}

impl Schedule {
    pub fn add_system(
        &mut self,
        phase: SchedulePhase,
        name: impl Into<String>,
        system: impl for<'frame> FnMut(&mut FrameContext<'frame>) -> GameResult<()> + 'static,
    ) -> GameResult<()> {
        let name = name.into();

        if name.trim().is_empty() {
            return Err("schedule system name must not be empty".into());
        }

        self.systems_mut(phase).push(ScheduleSystemRegistration {
            name,
            system: Box::new(system),
        });
        Ok(())
    }

    pub fn run_startup_once(&mut self, context: &mut FrameContext<'_>) -> GameResult<()> {
        if self.startup_complete {
            return Ok(());
        }

        self.run_phase(SchedulePhase::Startup, context)?;
        self.startup_complete = true;
        Ok(())
    }

    pub fn run_phase(
        &mut self,
        phase: SchedulePhase,
        context: &mut FrameContext<'_>,
    ) -> GameResult<()> {
        for registration in self.systems_mut(phase) {
            (registration.system)(context).map_err(|error| -> Box<dyn Error> {
                format!("schedule system '{}' failed: {error}", registration.name).into()
            })?;
        }

        Ok(())
    }

    pub fn system_names(&self, phase: SchedulePhase) -> Vec<&str> {
        self.systems(phase)
            .iter()
            .map(|registration| registration.name.as_str())
            .collect()
    }

    fn systems(&self, phase: SchedulePhase) -> &[ScheduleSystemRegistration] {
        match phase {
            SchedulePhase::Startup => &self.startup,
            SchedulePhase::Update => &self.update,
            SchedulePhase::PostUpdate => &self.post_update,
        }
    }

    fn systems_mut(&mut self, phase: SchedulePhase) -> &mut Vec<ScheduleSystemRegistration> {
        match phase {
            SchedulePhase::Startup => &mut self.startup,
            SchedulePhase::Update => &mut self.update,
            SchedulePhase::PostUpdate => &mut self.post_update,
        }
    }
}

pub type ComponentFactory = fn(&toml::Value) -> GameResult<Box<dyn Component>>;

#[derive(Clone, Copy)]
struct ComponentRegistration {
    type_id: Option<TypeId>,
    factory: ComponentFactory,
}

pub struct RuntimeComponent {
    entity: Entity,
    component: Box<dyn Component>,
}

#[derive(Clone, Default)]
pub struct ComponentRegistry {
    registrations: HashMap<String, ComponentRegistration>,
}

impl ComponentRegistry {
    pub fn register<T: Component + Default + 'static>(
        &mut self,
        name: impl Into<String>,
    ) -> GameResult<()> {
        let name = name.into();

        if name.trim().is_empty() {
            return Err("component registration name must not be empty".into());
        }

        self.registrations.insert(
            name,
            ComponentRegistration {
                type_id: Some(TypeId::of::<T>()),
                factory: |_| Ok(Box::<T>::default()),
            },
        );
        Ok(())
    }

    pub fn register_factory(
        &mut self,
        name: impl Into<String>,
        factory: ComponentFactory,
    ) -> GameResult<()> {
        let name = name.into();

        if name.trim().is_empty() {
            return Err("component registration name must not be empty".into());
        }

        self.registrations.insert(
            name,
            ComponentRegistration {
                type_id: None,
                factory,
            },
        );
        Ok(())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.registrations.contains_key(name)
    }

    fn type_id(&self, name: &str) -> Option<TypeId> {
        self.registrations
            .get(name)
            .and_then(|registration| registration.type_id)
    }

    fn instantiate(&self, component: &CustomComponentRef) -> GameResult<Box<dyn Component>> {
        let Some(registration) = self.registrations.get(&component.type_name) else {
            return Err(format!("unknown component type '{}'", component.type_name).into());
        };

        (registration.factory)(&component.config)
    }
}

pub struct Assets {
    loader: AssetLoader,
    next_texture_id: u64,
    texture_cache: HashMap<String, Texture>,
    sound_cache: HashMap<String, AssetHandle<SoundAsset>>,
}

impl Assets {
    pub fn new(root: AssetRoot) -> Self {
        Self {
            loader: AssetLoader::new(root),
            next_texture_id: 1,
            texture_cache: HashMap::new(),
            sound_cache: HashMap::new(),
        }
    }

    pub fn root(&self) -> &AssetRoot {
        self.loader.root()
    }

    pub fn texture(&mut self, path: impl AsRef<str>) -> GameResult<Texture> {
        self.load_texture(path)
    }

    pub fn sound(
        &mut self,
        audio: &mut AudioSystem,
        path: impl AsRef<str>,
    ) -> GameResult<AssetHandle<SoundAsset>> {
        let requested = path.as_ref().to_string();
        let virtual_path = VirtualPath::parse(&requested)?;
        ensure_asset_scheme(&virtual_path)?;
        let path = AssetPath::new(virtual_path.relative_path())?;

        if let Some(sound) = self.sound_cache.get(path.as_str()) {
            return Ok(*sound);
        }

        let resolved = self.loader.root().resolve(&path);

        let sound = audio.load_sound(self.root(), &path).map_err(|error| {
            PathDiagnosticError::asset(requested, resolved, self.loader.root().path(), error)
        })?;
        self.sound_cache.insert(path.as_str().to_string(), sound);

        Ok(sound)
    }

    pub fn load_texture(&mut self, path: impl AsRef<str>) -> GameResult<Texture> {
        let requested = path.as_ref();
        let virtual_path = VirtualPath::parse(requested)?;
        ensure_asset_scheme(&virtual_path)?;
        let path = AssetPath::new(virtual_path.relative_path())?;

        if let Some(texture) = self.texture_cache.get(path.as_str()) {
            return Ok(texture.clone());
        }

        let image = self.loader.load_image(&path).map_err(|error| {
            let resolved = self.loader.root().resolve(&path);
            PathDiagnosticError::asset(
                requested.to_string(),
                resolved,
                self.loader.root().path(),
                error,
            )
        })?;
        let texture_id = TextureId::new(self.next_texture_id);
        self.next_texture_id += 1;
        let data = TextureData::rgba8(
            texture_id,
            image.width(),
            image.height(),
            image.pixels_rgba8().to_vec(),
        )?;

        let texture = Texture { data };
        self.texture_cache
            .insert(path.as_str().to_string(), texture.clone());

        Ok(texture)
    }
}

#[derive(Debug, Clone)]
pub struct Texture {
    data: TextureData,
}

impl Texture {
    pub fn id(&self) -> TextureId {
        self.data.id()
    }

    pub fn data(&self) -> &TextureData {
        &self.data
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn splat(value: f32) -> Self {
        Self { x: value, y: value }
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpriteRenderer {
    texture: Texture,
    size: Vec2,
}

impl SpriteRenderer {
    pub fn new(texture: Texture, size: Vec2) -> Self {
        Self { texture, size }
    }
}

#[derive(Debug, Clone)]
pub struct SpriteBundle {
    pub texture: Texture,
    pub transform: Transform2D,
    pub size: Vec2,
}

impl SpriteBundle {
    pub fn new(texture: Texture) -> Self {
        Self {
            texture,
            transform: Transform2D::default(),
            size: Vec2::splat(32.0),
        }
    }
}

type RenderCache = HashMap<Entity, SpriteRenderer>;

pub trait WorldComponentExt {
    fn first_interactable(&self) -> Option<Entity>;
    fn has_custom_component(&self, entity: Entity, type_name: &str) -> bool;
    fn custom_component_config(&self, entity: Entity, type_name: &str) -> Option<&toml::Value>;
    fn has_component<T: Component + 'static>(&self, entity: Entity) -> bool;
}

impl WorldComponentExt for World {
    fn first_interactable(&self) -> Option<Entity> {
        self.first_with_tag("interactable")
    }

    fn has_custom_component(&self, entity: Entity, type_name: &str) -> bool {
        self.entity(entity).is_some_and(|record| {
            record
                .custom_components
                .iter()
                .any(|component| component.type_name == type_name)
        })
    }

    fn custom_component_config(&self, entity: Entity, type_name: &str) -> Option<&toml::Value> {
        self.entity(entity).and_then(|record| {
            record
                .custom_components
                .iter()
                .find(|component| component.type_name == type_name)
                .map(|component| &component.config)
        })
    }

    fn has_component<T: Component + 'static>(&self, entity: Entity) -> bool {
        self.has_component_type_id(entity, TypeId::of::<T>())
    }
}

fn set_custom_component_type_id_on_record(
    record: &mut EntityRecord,
    type_name: &str,
    type_id: TypeId,
) {
    if let Some(component) = record
        .custom_components
        .iter_mut()
        .find(|component| component.type_name == type_name)
    {
        component.type_id = Some(type_id);
    }
}

fn load_render_assets(
    record: &EntityRecord,
    assets: &mut Assets,
) -> GameResult<Option<SpriteRenderer>> {
    let Some(sprite) = &record.sprite else {
        return Ok(None);
    };

    Ok(Some(SpriteRenderer::new(
        assets.texture(&sprite.texture)?,
        Vec2::new(sprite.width.unwrap_or(32.0), sprite.height.unwrap_or(32.0)),
    )))
}

fn load_audio_asset(
    record: &EntityRecord,
    assets: &mut Assets,
    audio: &mut AudioSystem,
) -> GameResult<Option<AssetHandle<SoundAsset>>> {
    let Some(audio_ref) = &record.audio else {
        return Ok(None);
    };

    assets.sound(audio, &audio_ref.sound).map(Some)
}

fn render_world(world: &World, render_cache: &RenderCache, render: &mut RenderContext) {
    let mut renderables = world
        .entities()
        .filter_map(|(entity, record)| {
            render_cache
                .get(&entity)
                .map(|renderer| (entity, record, renderer))
        })
        .collect::<Vec<_>>();

    renderables.sort_by_key(|(entity, record, _)| {
        let layer = record.sprite.as_ref().map_or(0, |sprite| sprite.layer);
        let sort_order = record.sprite.as_ref().map_or(0, |sprite| sprite.sort_order);

        (layer, sort_order, *entity)
    });

    for (_entity, record, renderer) in renderables {
        render.texture(&renderer.texture);
        render.sprite(Sprite::new(
            renderer.texture.id(),
            record.transform,
            renderer.size.x,
            renderer.size.y,
        ));
    }
}

fn extract_ui_world(world: &World, render: &mut RenderContext) {
    let mut elements = world
        .entities()
        .filter_map(|(entity, record)| {
            record.ui.as_ref().map(|ui| UiElement {
                entity,
                ui: ui.clone(),
            })
        })
        .collect::<Vec<_>>();

    elements.sort_by_key(|element| (element.ui.layout.z_index, element.entity));
    render.ui_elements.extend(elements);
}

const FRAME_RENDER_NODE_RESET: &str = "reset";
const FRAME_RENDER_NODE_EXTRACT_WORLD: &str = "extract_world";
const FRAME_RENDER_NODE_EXTRACT_UI: &str = "extract_ui";
const FRAME_RENDER_NODE_USER_RENDER: &str = "user_render";

fn default_frame_render_graph() -> RenderGraph {
    let mut graph = RenderGraph::new();
    graph
        .add_node(FRAME_RENDER_NODE_RESET)
        .expect("default frame render graph reset node is unique");
    graph
        .add_node(FRAME_RENDER_NODE_EXTRACT_WORLD)
        .expect("default frame render graph extract node is unique");
    graph
        .add_node(FRAME_RENDER_NODE_EXTRACT_UI)
        .expect("default frame render graph ui node is unique");
    graph
        .add_node(FRAME_RENDER_NODE_USER_RENDER)
        .expect("default frame render graph user node is unique");
    graph
        .add_node_edge(FRAME_RENDER_NODE_RESET, FRAME_RENDER_NODE_EXTRACT_WORLD)
        .expect("default frame render graph reset edge is valid");
    graph
        .add_node_edge(
            FRAME_RENDER_NODE_EXTRACT_WORLD,
            FRAME_RENDER_NODE_EXTRACT_UI,
        )
        .expect("default frame render graph ui edge is valid");
    graph
        .add_node_edge(FRAME_RENDER_NODE_EXTRACT_UI, FRAME_RENDER_NODE_USER_RENDER)
        .expect("default frame render graph user edge is valid");
    graph
}

fn run_render_frame_graph(
    graph: &RenderGraph,
    world: &World,
    render_cache: &RenderCache,
    render: &mut RenderContext,
    mut render_game: impl FnMut(&mut RenderContext),
) -> Result<(), RenderGraphError> {
    for label in graph.execution_order()? {
        match label.as_str() {
            FRAME_RENDER_NODE_RESET => render.reset(),
            FRAME_RENDER_NODE_EXTRACT_WORLD => render_world(world, render_cache, render),
            FRAME_RENDER_NODE_EXTRACT_UI => extract_ui_world(world, render),
            FRAME_RENDER_NODE_USER_RENDER => render_game(render),
            _ => {}
        }
    }

    Ok(())
}

pub struct SpriteBuilder<'a> {
    context: &'a mut StartupContext,
    texture_path: String,
    transform: Transform2D,
    size: Vec2,
}

impl<'a> SpriteBuilder<'a> {
    fn new(context: &'a mut StartupContext, texture_path: String) -> Self {
        Self {
            context,
            texture_path,
            transform: Transform2D::default(),
            size: Vec2::splat(32.0),
        }
    }

    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.transform.x = x;
        self.transform.y = y;
        self
    }

    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Vec2::new(width, height);
        self
    }

    pub fn spawn(self) -> GameResult<Entity> {
        let texture = self.context.assets.texture(&self.texture_path)?;
        self.context.spawn(SpriteBundle {
            texture,
            transform: self.transform,
            size: self.size,
        })
    }
}

pub struct FrameContext<'a> {
    input: &'a InputState,
    input_actions: &'a InputActions,
    audio: &'a mut AudioSystem,
    audio_cache: &'a AudioCache,
    world: &'a mut World,
    resources: &'a Resources,
    dialogue: &'a mut DialogueState,
    frame: u64,
    delta_seconds: f32,
}

impl FrameContext<'_> {
    pub fn input(&self) -> GameplayInput<'_> {
        GameplayInput {
            state: self.input,
            actions: self.input_actions,
        }
    }

    pub fn world(&mut self) -> FrameWorld<'_> {
        FrameWorld { world: self.world }
    }

    pub fn resources(&self) -> &Resources {
        self.resources
    }

    pub fn dialogue(&mut self) -> &mut DialogueState {
        self.dialogue
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }

    pub fn axis(&self, negative: KeyCode, positive: KeyCode) -> f32 {
        axis(self.input, negative, positive)
    }

    pub fn play_sound(&mut self, sound: AssetHandle<SoundAsset>) -> PlaybackResult {
        self.audio.play_sound(sound)
    }

    pub fn play_entity_audio(&mut self, entity: Entity) -> Option<PlaybackResult> {
        self.audio_cache
            .get(&entity)
            .copied()
            .map(|sound| self.audio.play_sound(sound))
    }

    pub fn ui_interaction(&self, entity: Entity) -> Option<&UiInteractionRef> {
        self.world.ui(entity).and_then(|ui| ui.interaction.as_ref())
    }

    pub fn ui_action_pressed(&self, entity: Entity) -> bool {
        self.ui_interaction(entity).is_some_and(|interaction| {
            interaction.enabled && self.input().pressed(interaction.action.as_str())
        })
    }

    pub fn ui_action_just_pressed(&self, entity: Entity) -> bool {
        self.ui_interaction(entity).is_some_and(|interaction| {
            interaction.enabled && self.input().just_pressed(interaction.action.as_str())
        })
    }

    pub fn ui_action_just_released(&self, entity: Entity) -> bool {
        self.ui_interaction(entity).is_some_and(|interaction| {
            interaction.enabled && self.input().just_released(interaction.action.as_str())
        })
    }
}

pub struct FrameWorld<'a> {
    world: &'a mut World,
}

impl FrameWorld<'_> {
    pub fn entity(&mut self, entity: Entity) -> EntityMut<'_> {
        EntityMut {
            world: self.world,
            entity,
        }
    }

    pub fn translate(&mut self, entity: Entity, delta: Vec2) -> &mut Self {
        let _ = self.world.translate(entity, delta.x, delta.y);
        self
    }

    pub fn set_position(&mut self, entity: Entity, x: f32, y: f32) -> &mut Self {
        let _ = self.world.set_position(entity, x, y);
        self
    }
}

pub struct EntityMut<'a> {
    world: &'a mut World,
    entity: Entity,
}

impl EntityMut<'_> {
    pub fn translate(&mut self, delta: Vec2) -> &mut Self {
        let _ = self.world.translate(self.entity, delta.x, delta.y);
        self
    }

    pub fn set_position(&mut self, x: f32, y: f32) -> &mut Self {
        let _ = self.world.set_position(self.entity, x, y);
        self
    }
}

impl std::ops::Deref for FrameWorld<'_> {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        self.world
    }
}

impl std::ops::DerefMut for FrameWorld<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.world
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GameplayInput<'a> {
    state: &'a InputState,
    actions: &'a InputActions,
}

impl GameplayInput<'_> {
    pub fn pressed<Q: InputQuery>(&self, query: Q) -> bool {
        query.pressed(self.state, self.actions)
    }

    pub fn just_pressed<Q: InputQuery>(&self, query: Q) -> bool {
        query.just_pressed(self.state, self.actions)
    }

    pub fn just_released<Q: InputQuery>(&self, query: Q) -> bool {
        query.just_released(self.state, self.actions)
    }

    pub fn axis2d(&self, name: &str) -> Vec2 {
        self.actions.axis2d(self.state, name)
    }
}

pub trait InputQuery {
    fn pressed(self, state: &InputState, actions: &InputActions) -> bool;
    fn just_pressed(self, state: &InputState, actions: &InputActions) -> bool;
    fn just_released(self, state: &InputState, actions: &InputActions) -> bool;
}

impl InputQuery for KeyCode {
    fn pressed(self, state: &InputState, _actions: &InputActions) -> bool {
        state.pressed(self)
    }

    fn just_pressed(self, state: &InputState, _actions: &InputActions) -> bool {
        state.just_pressed(self)
    }

    fn just_released(self, state: &InputState, _actions: &InputActions) -> bool {
        state.just_released(self)
    }
}

impl InputQuery for &str {
    fn pressed(self, state: &InputState, actions: &InputActions) -> bool {
        actions.button_pressed(state, self)
    }

    fn just_pressed(self, state: &InputState, actions: &InputActions) -> bool {
        actions.button_just_pressed(state, self)
    }

    fn just_released(self, state: &InputState, actions: &InputActions) -> bool {
        actions.button_just_released(state, self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct InputActions {
    axis2d: HashMap<String, Axis2dAction>,
    buttons: HashMap<String, ButtonAction>,
}

impl InputActions {
    fn from_config(config: InputConfig) -> Self {
        let mut axis2d = HashMap::new();
        let mut buttons = HashMap::new();

        for (name, action) in config.actions {
            match action {
                InputActionConfig::Axis2d {
                    left,
                    right,
                    up,
                    down,
                } => {
                    axis2d.insert(
                        name,
                        Axis2dAction {
                            left: parse_key_list(left),
                            right: parse_key_list(right),
                            up: parse_key_list(up),
                            down: parse_key_list(down),
                        },
                    );
                }
                InputActionConfig::Button { keys } => {
                    buttons.insert(
                        name,
                        ButtonAction {
                            keys: parse_key_list(keys),
                        },
                    );
                }
            }
        }

        Self { axis2d, buttons }
    }

    fn axis2d(&self, input: &InputState, name: &str) -> Vec2 {
        let Some(action) = self.axis2d.get(name) else {
            #[cfg(feature = "logging")]
            debug!(action = name, "axis2d input action not found or not axis2d");
            return Vec2::ZERO;
        };

        Vec2::new(
            action_axis(input, &action.left, &action.right),
            action_axis(input, &action.up, &action.down),
        )
    }

    fn button_pressed(&self, input: &InputState, name: &str) -> bool {
        self.button(name)
            .is_some_and(|action| action.keys.iter().any(|key| input.pressed(*key)))
    }

    fn button_just_pressed(&self, input: &InputState, name: &str) -> bool {
        self.button(name)
            .is_some_and(|action| action.keys.iter().any(|key| input.just_pressed(*key)))
    }

    fn button_just_released(&self, input: &InputState, name: &str) -> bool {
        self.button(name)
            .is_some_and(|action| action.keys.iter().any(|key| input.just_released(*key)))
    }

    fn button(&self, name: &str) -> Option<&ButtonAction> {
        let action = self.buttons.get(name);

        if action.is_none() {
            #[cfg(feature = "logging")]
            debug!(action = name, "button input action not found or not button");
        }

        action
    }
}

#[derive(Debug, Clone, Default)]
struct Axis2dAction {
    left: Vec<KeyCode>,
    right: Vec<KeyCode>,
    up: Vec<KeyCode>,
    down: Vec<KeyCode>,
}

#[derive(Debug, Clone, Default)]
struct ButtonAction {
    keys: Vec<KeyCode>,
}

#[derive(Debug, Clone)]
pub struct UiElement {
    entity: Entity,
    ui: UiRef,
}

impl UiElement {
    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn ui(&self) -> &UiRef {
        &self.ui
    }
}

#[derive(Debug, Clone)]
pub struct RenderContext {
    clear_color: ClearColor,
    camera: Camera2D,
    texture_ids: HashSet<TextureId>,
    textures: Vec<TextureData>,
    sprites: Vec<Sprite>,
    ui_elements: Vec<UiElement>,
}

impl RenderContext {
    fn new(clear_color: ClearColor) -> Self {
        Self {
            clear_color,
            camera: Camera2D::default(),
            texture_ids: HashSet::new(),
            textures: Vec::new(),
            sprites: Vec::new(),
            ui_elements: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.texture_ids.clear();
        self.textures.clear();
        self.sprites.clear();
        self.ui_elements.clear();
    }

    pub fn clear(&mut self, color: ClearColor) {
        self.clear_color = color;
    }

    pub fn camera(&mut self, camera: Camera2D) {
        self.camera = camera;
    }

    pub fn texture(&mut self, texture: &Texture) {
        if self.texture_ids.insert(texture.id()) {
            self.textures.push(texture.data().clone());
        }
    }

    pub fn sprite(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }

    pub fn ui_element(&mut self, element: UiElement) {
        self.ui_elements.push(element);
    }

    pub fn ui_elements(&self) -> &[UiElement] {
        &self.ui_elements
    }

    fn state(&self) -> RenderState<'_> {
        RenderState {
            clear_color: self.clear_color,
            camera: self.camera,
            textures: &self.textures,
            sprites: &self.sprites,
        }
    }
}

struct Game2DAdapter<G> {
    game: G,
    input: InputState,
    input_actions: InputActions,
    audio: AudioSystem,
    audio_cache: AudioCache,
    world: World,
    render_cache: RenderCache,
    resources: Resources,
    dialogue: DialogueState,
    component_instances: Vec<RuntimeComponent>,
    schedule: Schedule,
    render: RenderContext,
    frame_graph: RenderGraph,
}

impl<G: Game2D> Game2DAdapter<G> {
    fn new(game: G, runtime_parts: RuntimeParts) -> Self {
        let mut render = RenderContext::new(runtime_parts.clear_color);
        render_world(
            &runtime_parts.world,
            &runtime_parts.render_cache,
            &mut render,
        );

        Self {
            game,
            input: InputState::default(),
            input_actions: runtime_parts.input_actions,
            audio: runtime_parts.audio,
            audio_cache: runtime_parts.audio_cache,
            world: runtime_parts.world,
            render_cache: runtime_parts.render_cache,
            resources: runtime_parts.resources,
            dialogue: runtime_parts.dialogue,
            component_instances: runtime_parts.component_instances,
            schedule: runtime_parts.schedule,
            render,
            frame_graph: default_frame_render_graph(),
        }
    }
}

#[cfg(any(
    all(not(target_arch = "wasm32"), feature = "desktop"),
    all(target_arch = "wasm32", feature = "web")
))]
impl<G: Game2D> DesktopGame for Game2DAdapter<G> {
    fn input_state(&mut self) -> &mut InputState {
        &mut self.input
    }

    fn render_state(&self) -> RenderState<'_> {
        self.render.state()
    }
}

impl<G: Game2D> Game for Game2DAdapter<G> {
    fn update(&mut self, _engine: &mut Engine, context: UpdateContext) -> EngineResult<()> {
        let mut frame = FrameContext {
            input: &self.input,
            input_actions: &self.input_actions,
            audio: &mut self.audio,
            audio_cache: &self.audio_cache,
            world: &mut self.world,
            resources: &self.resources,
            dialogue: &mut self.dialogue,
            frame: context.frame,
            delta_seconds: context.delta_seconds,
        };

        self.schedule
            .run_startup_once(&mut frame)
            .map_err(|error| seishin_core::EngineError::Runtime(error.to_string()))?;

        update_builtin_dialogue_interaction(&mut frame)
            .map_err(|error| seishin_core::EngineError::Runtime(error.to_string()))?;

        self.schedule
            .run_phase(SchedulePhase::Update, &mut frame)
            .map_err(|error| seishin_core::EngineError::Runtime(error.to_string()))?;

        for runtime_component in &mut self.component_instances {
            runtime_component
                .component
                .update(runtime_component.entity, &mut frame)
                .map_err(|error| seishin_core::EngineError::Runtime(error.to_string()))?;
        }

        self.game
            .update(&mut frame)
            .map_err(|error| seishin_core::EngineError::Runtime(error.to_string()))?;

        self.schedule
            .run_phase(SchedulePhase::PostUpdate, &mut frame)
            .map_err(|error| seishin_core::EngineError::Runtime(error.to_string()))?;

        run_render_frame_graph(
            &self.frame_graph,
            &self.world,
            &self.render_cache,
            &mut self.render,
            |render| self.game.render(render),
        )
        .map_err(|error| seishin_core::EngineError::Runtime(error.to_string()))?;

        Ok(())
    }

    fn shutdown(&mut self, _engine: &mut Engine) -> EngineResult<()> {
        self.game
            .shutdown()
            .map_err(|error| seishin_core::EngineError::Runtime(error.to_string()))
    }
}

fn update_builtin_dialogue_interaction(context: &mut FrameContext<'_>) -> GameResult<()> {
    if !context.input().just_pressed("interact") {
        return Ok(());
    }

    if context.dialogue().is_active() {
        context.dialogue().advance_or_close();
        return Ok(());
    }

    let character_ref = {
        let world = context.world();
        world
            .first_interactable()
            .and_then(|entity| world.data_ref(entity, "character"))
            .map(ToOwned::to_owned)
    };

    let Some(character_ref) = character_ref else {
        #[cfg(feature = "logging")]
        debug!("interact pressed but no interactable dialogue target was found");
        return Ok(());
    };

    let character = context.resources().character(&character_ref)?;
    let Some(dialogue_ref) = character
        .dialogue
        .as_ref()
        .map(|dialogue| &dialogue.default)
    else {
        #[cfg(feature = "logging")]
        debug!(character = %character.id, "interactable character has no default dialogue");
        return Ok(());
    };
    let dialogue = context.resources().dialogue(dialogue_ref)?;

    context.dialogue().open(character.display_name, dialogue);

    Ok(())
}

#[cfg(any(test, target_arch = "wasm32"))]
fn default_audio_system() -> AudioSystem {
    AudioSystem::without_backend("audio backend disabled for this target")
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
fn default_audio_system() -> AudioSystem {
    AudioSystem::new()
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    game: Option<GameConfig>,
    window: Option<WindowProjectConfig>,
    resources: Option<ResourcesConfig>,
    assets: Option<AssetsConfig>,
    user: Option<UserConfig>,
    logging: Option<LoggingProjectConfig>,
    input: Option<InputConfig>,
}

impl ProjectConfig {
    fn from_path(path: &Path) -> GameResult<Self> {
        let source = platform::read_to_string(path)?;
        Ok(toml::from_str(&source)?)
    }
}

#[derive(Debug, Default, Deserialize)]
struct GameConfig {
    name: Option<String>,
    main_scene: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct WindowProjectConfig {
    width: Option<u32>,
    height: Option<u32>,
    target_fps: Option<u32>,
    clear_color: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct AssetsConfig {
    root: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ResourcesConfig {
    root: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct UserConfig {
    root: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct LoggingProjectConfig {
    default_filter: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct InputConfig {
    actions: HashMap<String, InputActionConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum InputActionConfig {
    #[serde(rename = "axis2d")]
    Axis2d {
        #[serde(default)]
        left: Vec<String>,
        #[serde(default)]
        right: Vec<String>,
        #[serde(default)]
        up: Vec<String>,
        #[serde(default)]
        down: Vec<String>,
    },
    #[serde(rename = "button")]
    Button {
        #[serde(default)]
        keys: Vec<String>,
    },
}

#[derive(Debug)]
struct PathDiagnosticError {
    kind: PathDiagnosticKind,
    requested: String,
    resolved: PathBuf,
    root: PathBuf,
    source: Box<dyn Error + Send + Sync>,
}

#[derive(Debug, Clone, Copy)]
enum PathDiagnosticKind {
    Asset,
    Resource,
}

impl PathDiagnosticError {
    fn asset(
        requested: String,
        resolved: PathBuf,
        root: &Path,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind: PathDiagnosticKind::Asset,
            requested,
            resolved,
            root: root.to_path_buf(),
            source: Box::new(source),
        }
    }

    fn resource(
        requested: String,
        resolved: PathBuf,
        root: &Path,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind: PathDiagnosticKind::Resource,
            requested,
            resolved,
            root: root.to_path_buf(),
            source: Box::new(source),
        }
    }
}

impl std::fmt::Display for PathDiagnosticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (label, root_label, scheme_hint, other_scheme_hint) = match self.kind {
            PathDiagnosticKind::Asset => (
                "Asset",
                "Configured asset root",
                "Use asset:// for images, audio, video, and fonts.",
                "Use res:// for resources/configuration/scene files.",
            ),
            PathDiagnosticKind::Resource => (
                "Resource",
                "Configured resource root",
                "Use res:// for resources/configuration/scene files.",
                "Use asset:// for images, audio, video, and fonts.",
            ),
        };

        write!(
            f,
            "{label} not found or could not be loaded: {}\n\nResolved path:\n  {}\n\n{root_label}:\n  {}\n\nSuggestions:\n  - Check if the file exists.\n  - Check Seishin.toml root configuration.\n  - {scheme_hint}\n  - {other_scheme_hint}\n\nCause: {}",
            self.requested,
            self.resolved.display(),
            self.root.display(),
            self.source
        )
    }
}

impl Error for PathDiagnosticError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[derive(Debug, Clone)]
struct ProjectPaths {
    asset_root: PathBuf,
    resource_root: PathBuf,
    user_root: PathBuf,
}

impl ProjectPaths {
    fn new(asset_root: PathBuf, resource_root: PathBuf, user_root: PathBuf) -> Self {
        Self {
            asset_root,
            resource_root,
            user_root,
        }
    }

    fn resolve_resource(&self, requested: &str) -> GameResult<PathBuf> {
        let virtual_path = VirtualPath::parse(requested)?;
        ensure_resource_scheme(&virtual_path)?;
        let asset_path = AssetPath::new(virtual_path.relative_path())?;
        Ok(self.resource_root.join(asset_path.as_path()))
    }

    fn user_root(&self) -> &Path {
        &self.user_root
    }

    #[cfg(test)]
    fn resolve_asset(&self, requested: &str) -> GameResult<PathBuf> {
        let virtual_path = VirtualPath::parse(requested)?;
        ensure_asset_scheme(&virtual_path)?;
        let asset_path = AssetPath::new(virtual_path.relative_path())?;
        Ok(self.asset_root.join(asset_path.as_path()))
    }

    #[cfg(test)]
    fn resolve_user(&self, requested: &str) -> GameResult<PathBuf> {
        let virtual_path = VirtualPath::parse(requested)?;

        if virtual_path.scheme != VirtualScheme::User {
            return Err(format!(
                "user data paths must use user:// so they resolve under the user data root: {}",
                virtual_path.requested
            )
            .into());
        }

        let asset_path = AssetPath::new(virtual_path.relative_path())?;
        Ok(self.user_root.join(asset_path.as_path()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VirtualScheme {
    Asset,
    Resource,
    User,
    Relative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VirtualPath<'a> {
    scheme: VirtualScheme,
    relative_path: &'a str,
    requested: &'a str,
}

impl<'a> VirtualPath<'a> {
    fn parse(requested: &'a str) -> GameResult<Self> {
        if let Some(relative_path) = requested.strip_prefix("asset://") {
            return Ok(Self {
                scheme: VirtualScheme::Asset,
                relative_path,
                requested,
            });
        }

        if let Some(relative_path) = requested.strip_prefix("res://") {
            return Ok(Self {
                scheme: VirtualScheme::Resource,
                relative_path,
                requested,
            });
        }

        if let Some(relative_path) = requested.strip_prefix("user://") {
            return Ok(Self {
                scheme: VirtualScheme::User,
                relative_path,
                requested,
            });
        }

        if requested.contains("://") {
            return Err(format!(
                "unsupported virtual path scheme in {requested}; expected asset://, res://, or user://"
            )
            .into());
        }

        Ok(Self {
            scheme: VirtualScheme::Relative,
            relative_path: requested,
            requested,
        })
    }

    fn relative_path(&self) -> &str {
        self.relative_path
    }
}

fn ensure_asset_scheme(path: &VirtualPath<'_>) -> GameResult<()> {
    match path.scheme {
        VirtualScheme::Asset | VirtualScheme::Relative => Ok(()),
        VirtualScheme::Resource => Err(format!(
            "possible wrong scheme: you used {}. Sprites, audio, video, and fonts are assets. Try asset://{}",
            path.requested,
            path.relative_path()
        )
        .into()),
        VirtualScheme::User => Err(format!(
            "user:// paths are reserved for writable user data and cannot be loaded as assets: {}",
            path.requested
        )
        .into()),
    }
}

fn ensure_resource_scheme(path: &VirtualPath<'_>) -> GameResult<()> {
    match path.scheme {
        VirtualScheme::Resource => Ok(()),
        VirtualScheme::Asset => Err(format!(
            "possible wrong scheme: you used {}. Configuration, scenes, prefabs, and data files are resources. Try res://{}",
            path.requested,
            path.relative_path()
        )
        .into()),
        VirtualScheme::User => Err(format!(
            "user:// paths are reserved for writable user data and cannot be loaded as resources: {}",
            path.requested
        )
        .into()),
        VirtualScheme::Relative => Err(format!(
            "resource paths must use res:// so they resolve under [resources].root: {}",
            path.requested
        )
        .into()),
    }
}

fn validate_main_scene(main_scene: &str, paths: &ProjectPaths) -> GameResult<()> {
    let resolved = paths.resolve_resource(main_scene)?;
    platform::ensure_readable_file(&resolved).map_err(|error| {
        PathDiagnosticError::resource(
            main_scene.to_string(),
            resolved,
            &paths.resource_root,
            error,
        )
    })?;

    Ok(())
}

fn load_main_scene(main_scene: &str, startup: &mut StartupContext) -> GameResult<()> {
    let scene = load_scene_config(main_scene, &startup.paths)?;
    let mut prefab_cache = HashMap::new();
    let mut resolved_entities = Vec::new();

    for scene_entity in scene.entities {
        resolved_entities.push(build_scene_entity(
            scene_entity,
            startup,
            &mut prefab_cache,
        )?);
    }

    let planned_entities = plan_scene_entities(&startup.world, &resolved_entities)?;
    let mut pending_entities = Vec::new();

    for (resolved, entity) in resolved_entities.into_iter().zip(planned_entities) {
        let mut record = resolved.record;
        let renderer = load_render_assets(&record, &mut startup.assets)?;
        let audio = load_audio_asset(&record, &mut startup.assets, &mut startup.audio)?;
        let mut components = Vec::new();

        for component_ref in record.custom_components.clone() {
            let component = startup.components.instantiate(&component_ref)?;
            if let Some(type_id) = startup.components.type_id(&component_ref.type_name) {
                set_custom_component_type_id_on_record(
                    &mut record,
                    &component_ref.type_name,
                    type_id,
                );
            }
            components.push(RuntimeComponent { entity, component });
        }

        pending_entities.push(PendingSceneEntity {
            id: resolved.id,
            entity,
            record,
            renderer,
            audio,
            components,
        });
    }

    for pending in pending_entities {
        let entity = match pending.id {
            Some(entity) => {
                startup.world.insert(entity, pending.record)?;
                entity
            }
            None => startup.world.spawn(pending.record),
        };
        debug_assert_eq!(entity, pending.entity);

        if let Some(renderer) = pending.renderer {
            startup.render_cache.insert(entity, renderer);
        }
        if let Some(audio) = pending.audio {
            startup.audio_cache.insert(entity, audio);
        }

        startup.component_instances.extend(pending.components);
    }

    Ok(())
}

struct PendingSceneEntity {
    id: Option<Entity>,
    entity: Entity,
    record: EntityRecord,
    renderer: Option<SpriteRenderer>,
    audio: Option<AssetHandle<SoundAsset>>,
    components: Vec<RuntimeComponent>,
}

fn plan_scene_entities(
    world: &World,
    resolved_entities: &[ResolvedEntity],
) -> Result<Vec<Entity>, seishin_world::WorldError> {
    let mut planned_world = world.clone();
    let mut planned_entities = Vec::with_capacity(resolved_entities.len());

    for resolved in resolved_entities {
        let entity = match resolved.id {
            Some(entity) => {
                planned_world.insert(entity, resolved.record.clone())?;
                entity
            }
            None => planned_world.spawn(resolved.record.clone()),
        };
        planned_entities.push(entity);
    }

    Ok(planned_entities)
}

fn load_scene_config(path: &str, paths: &ProjectPaths) -> GameResult<SceneDocument> {
    let resolved = paths.resolve_resource(path)?;
    let source = platform::read_to_string(&resolved).map_err(|error| {
        PathDiagnosticError::resource(
            path.to_string(),
            resolved.clone(),
            &paths.resource_root,
            error,
        )
    })?;

    SceneDocument::from_toml_str(&source).map_err(|error| {
        PathDiagnosticError::resource(path.to_string(), resolved, &paths.resource_root, error)
            .into()
    })
}

fn load_prefab_config(path: &str, paths: &ProjectPaths) -> GameResult<PrefabDocument> {
    let resolved = paths.resolve_resource(path)?;
    let source = platform::read_to_string(&resolved).map_err(|error| {
        PathDiagnosticError::resource(
            path.to_string(),
            resolved.clone(),
            &paths.resource_root,
            error,
        )
    })?;

    PrefabDocument::from_toml_str(&source).map_err(|error| {
        PathDiagnosticError::resource(path.to_string(), resolved, &paths.resource_root, error)
            .into()
    })
}

fn load_prefab_config_cached(
    path: &str,
    paths: &ProjectPaths,
    cache: &mut HashMap<String, PrefabDocument>,
) -> GameResult<PrefabDocument> {
    if let Some(prefab) = cache.get(path) {
        return Ok(prefab.clone());
    }

    let prefab = load_prefab_config(path, paths)?;
    cache.insert(path.to_string(), prefab.clone());
    Ok(prefab)
}

fn build_scene_entity(
    entity: SceneEntityDocument,
    startup: &mut StartupContext,
    prefab_cache: &mut HashMap<String, PrefabDocument>,
) -> GameResult<ResolvedEntity> {
    let prefab = match entity.prefab.as_deref() {
        Some(prefab_path) => Some(load_prefab_config_cached(
            prefab_path,
            &startup.paths,
            prefab_cache,
        )?),
        None => None,
    };

    let resolved = resolve_scene_entity(entity, prefab)?;
    validate_custom_components(&resolved.record, &startup.components)?;
    validate_data_refs(&resolved.record, &startup.paths)?;

    Ok(resolved)
}

fn validate_custom_components(
    record: &EntityRecord,
    registry: &ComponentRegistry,
) -> GameResult<()> {
    for component in &record.custom_components {
        if !registry.contains(&component.type_name) {
            let name = record.name.as_deref().unwrap_or("<unnamed>");
            return Err(format!(
                "unknown component type '{}' while loading entity '{}'; register it with ctx.components().register::<T>(\"{}\") before ctx.load_main_scene()",
                component.type_name, name, component.type_name
            )
            .into());
        }
    }

    Ok(())
}

fn validate_data_refs(record: &EntityRecord, paths: &ProjectPaths) -> GameResult<()> {
    for value in record.data_refs.values() {
        let resolved = paths.resolve_resource(value)?;
        platform::ensure_readable_file(&resolved).map_err(|error| {
            PathDiagnosticError::resource(value.clone(), resolved, &paths.resource_root, error)
        })?;
    }

    Ok(())
}

fn parse_clear_color(value: &str) -> Option<ClearColor> {
    match value.to_ascii_lowercase().as_str() {
        "black" => Some(ClearColor::BLACK),
        "cornflower" | "cornflowerblue" => Some(ClearColor::CORNFLOWER),
        _ => None,
    }
}

fn parse_key_list(keys: Vec<String>) -> Vec<KeyCode> {
    keys.into_iter()
        .filter_map(|key| parse_key_code(&key))
        .collect()
}

fn parse_key_code(key: &str) -> Option<KeyCode> {
    match key {
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        "KeyW" | "W" => Some(KeyCode::KeyW),
        "KeyA" | "A" => Some(KeyCode::KeyA),
        "KeyS" | "S" => Some(KeyCode::KeyS),
        "KeyD" | "D" => Some(KeyCode::KeyD),
        "Space" => Some(KeyCode::Space),
        "Enter" => Some(KeyCode::Enter),
        "Escape" => Some(KeyCode::Escape),
        _ => None,
    }
}

fn action_axis(input: &InputState, negative: &[KeyCode], positive: &[KeyCode]) -> f32 {
    let negative_pressed = negative.iter().any(|key| input.pressed(*key));
    let positive_pressed = positive.iter().any(|key| input.pressed(*key));

    match (negative_pressed, positive_pressed) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    }
}

fn axis(input: &InputState, negative: KeyCode, positive: KeyCode) -> f32 {
    match (input.pressed(negative), input.pressed(positive)) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtual_paths_resolve_under_distinct_roots() {
        let paths = ProjectPaths::new(
            PathBuf::from("/project/assets"),
            PathBuf::from("/project/resources"),
            PathBuf::from("/user/seishin"),
        );

        assert_eq!(
            paths
                .resolve_asset("asset://sprites/player.png")
                .expect("asset path"),
            PathBuf::from("/project/assets/sprites/player.png")
        );
        assert_eq!(
            paths
                .resolve_resource("res://scenes/main.scene.toml")
                .expect("resource path"),
            PathBuf::from("/project/resources/scenes/main.scene.toml")
        );
        assert_eq!(
            paths
                .resolve_user("user://save_001.dat")
                .expect("user path"),
            PathBuf::from("/user/seishin/save_001.dat")
        );

        assert!(paths.resolve_asset("res://sprites/player.png").is_err());
        assert!(paths
            .resolve_resource("asset://scenes/main.scene.toml")
            .is_err());
    }

    #[test]
    fn input_actions_map_axis2d_from_named_keys() {
        let config = InputConfig {
            actions: HashMap::from([(
                "move".to_string(),
                InputActionConfig::Axis2d {
                    left: vec!["ArrowLeft".to_string(), "KeyA".to_string()],
                    right: vec!["ArrowRight".to_string(), "KeyD".to_string()],
                    up: vec!["ArrowUp".to_string(), "KeyW".to_string()],
                    down: vec!["ArrowDown".to_string(), "KeyS".to_string()],
                },
            )]),
        };
        let actions = InputActions::from_config(config);
        let mut input = InputState::default();

        input.press(KeyCode::KeyD);
        input.press(KeyCode::KeyW);

        assert_eq!(actions.axis2d(&input, "move"), Vec2::new(1.0, -1.0));
        assert_eq!(actions.axis2d(&input, "missing"), Vec2::ZERO);
    }

    #[test]
    fn input_actions_map_button_just_pressed_from_named_keys() {
        let config = InputConfig {
            actions: HashMap::from([(
                "interact".to_string(),
                InputActionConfig::Button {
                    keys: vec!["Space".to_string(), "Enter".to_string()],
                },
            )]),
        };
        let actions = InputActions::from_config(config);
        let mut input = InputState::default();

        input.press(KeyCode::Enter);

        assert!(actions.button_pressed(&input, "interact"));
        assert!(actions.button_just_pressed(&input, "interact"));
        assert!(!actions.button_just_released(&input, "interact"));
    }

    #[test]
    fn world_renders_spawned_sprite_entities() {
        let texture = Texture {
            data: TextureData::rgba8(TextureId::new(7), 1, 1, vec![255, 255, 255, 255])
                .expect("valid texture"),
        };
        let mut world = World::default();
        let mut render_cache = RenderCache::default();
        let entity = world.spawn(EntityRecord {
            transform: Transform2D::from_translation(1.0, 2.0),
            ..EntityRecord::default()
        });
        render_cache.insert(entity, SpriteRenderer::new(texture, Vec2::splat(16.0)));

        world.translate(entity, 3.0, 4.0).expect("translate");
        world.translate(entity, 1.0, 1.0).expect("translate");

        let mut render = RenderContext::new(ClearColor::BLACK);
        render_world(&world, &render_cache, &mut render);
        let state = render.state();

        assert_eq!(state.textures.len(), 1);
        assert_eq!(state.sprites.len(), 1);
        assert_eq!(state.sprites[0].texture_id, TextureId::new(7));
        assert_eq!(state.sprites[0].transform.x, 5.0);
        assert_eq!(state.sprites[0].transform.y, 7.0);
        assert_eq!(state.sprites[0].width, 16.0);
        assert_eq!(state.sprites[0].height, 16.0);
    }

    #[test]
    fn world_render_order_uses_sprite_layer_sort_order_and_entity_id() {
        let back_texture = Texture {
            data: TextureData::rgba8(TextureId::new(10), 1, 1, vec![255, 255, 255, 255])
                .expect("valid texture"),
        };
        let middle_texture = Texture {
            data: TextureData::rgba8(TextureId::new(20), 1, 1, vec![255, 255, 255, 255])
                .expect("valid texture"),
        };
        let front_texture = Texture {
            data: TextureData::rgba8(TextureId::new(30), 1, 1, vec![255, 255, 255, 255])
                .expect("valid texture"),
        };
        let mut world = World::default();
        let mut render_cache = RenderCache::default();
        let front = EntityId::new(30);
        let middle = EntityId::new(20);
        let back = EntityId::new(10);

        world
            .insert(
                front,
                EntityRecord {
                    sprite: Some(seishin_world::SpriteRef {
                        texture: "asset://sprites/front.png".to_string(),
                        width: Some(16.0),
                        height: Some(16.0),
                        layer: 5,
                        sort_order: 0,
                    }),
                    ..EntityRecord::default()
                },
            )
            .expect("front");
        world
            .insert(
                middle,
                EntityRecord {
                    sprite: Some(seishin_world::SpriteRef {
                        texture: "asset://sprites/middle.png".to_string(),
                        width: Some(16.0),
                        height: Some(16.0),
                        layer: 1,
                        sort_order: 7,
                    }),
                    ..EntityRecord::default()
                },
            )
            .expect("middle");
        world
            .insert(
                back,
                EntityRecord {
                    sprite: Some(seishin_world::SpriteRef {
                        texture: "asset://sprites/back.png".to_string(),
                        width: Some(16.0),
                        height: Some(16.0),
                        layer: 1,
                        sort_order: -2,
                    }),
                    ..EntityRecord::default()
                },
            )
            .expect("back");
        render_cache.insert(front, SpriteRenderer::new(front_texture, Vec2::splat(16.0)));
        render_cache.insert(
            middle,
            SpriteRenderer::new(middle_texture, Vec2::splat(16.0)),
        );
        render_cache.insert(back, SpriteRenderer::new(back_texture, Vec2::splat(16.0)));

        let mut render = RenderContext::new(ClearColor::BLACK);
        render_world(&world, &render_cache, &mut render);

        assert_eq!(
            render
                .state()
                .sprites
                .iter()
                .map(|sprite| sprite.texture_id)
                .collect::<Vec<_>>(),
            vec![TextureId::new(10), TextureId::new(20), TextureId::new(30)]
        );
    }

    #[test]
    fn world_queries_non_renderable_entities() {
        let mut world = World::default();
        let render_cache = RenderCache::default();
        let entity = world.spawn(EntityRecord {
            name: Some("Trigger".to_string()),
            tags: vec!["trigger".to_string()],
            ..EntityRecord::default()
        });

        assert_eq!(world.entity_by_name("Trigger"), Some(entity));
        assert_eq!(world.first_with_tag("trigger"), Some(entity));

        let mut render = RenderContext::new(ClearColor::BLACK);
        render_world(&world, &render_cache, &mut render);
        assert!(render.state().sprites.is_empty());
    }

    #[test]
    fn default_frame_render_graph_orders_reset_world_extract_ui_and_user_render() {
        let graph = default_frame_render_graph();

        assert_eq!(
            graph.execution_order().expect("frame graph order"),
            vec![
                NodeLabel::from("reset"),
                NodeLabel::from("extract_world"),
                NodeLabel::from("extract_ui"),
                NodeLabel::from("user_render"),
            ]
        );
    }

    #[test]
    fn frame_render_graph_resets_extracts_world_then_runs_user_render() {
        let stale_texture = Texture {
            data: TextureData::rgba8(TextureId::new(1), 1, 1, vec![255, 0, 0, 255])
                .expect("valid stale texture"),
        };
        let world_texture = Texture {
            data: TextureData::rgba8(TextureId::new(2), 1, 1, vec![0, 255, 0, 255])
                .expect("valid world texture"),
        };
        let user_texture = Texture {
            data: TextureData::rgba8(TextureId::new(3), 1, 1, vec![0, 0, 255, 255])
                .expect("valid user texture"),
        };
        let mut world = World::default();
        let mut render_cache = RenderCache::default();
        let entity = world.spawn(EntityRecord {
            transform: Transform2D::from_translation(4.0, 8.0),
            ..EntityRecord::default()
        });
        let ui_back = EntityId::new(10);
        let ui_front = EntityId::new(20);
        render_cache.insert(
            entity,
            SpriteRenderer::new(world_texture, Vec2::splat(16.0)),
        );
        world
            .insert(
                ui_front,
                EntityRecord {
                    ui: Some(UiRef {
                        layout: seishin_world::UiLayoutRef {
                            z_index: 10,
                            ..Default::default()
                        },
                        text: Some(seishin_world::UiTextRef {
                            value: "Front".to_string(),
                            font_size: 16.0,
                            color: "#ffffff".to_string(),
                        }),
                        image: None,
                        interaction: None,
                    }),
                    ..EntityRecord::default()
                },
            )
            .expect("insert front ui");
        world
            .insert(
                ui_back,
                EntityRecord {
                    ui: Some(UiRef {
                        layout: seishin_world::UiLayoutRef {
                            z_index: -1,
                            ..Default::default()
                        },
                        text: Some(seishin_world::UiTextRef {
                            value: "Back".to_string(),
                            font_size: 16.0,
                            color: "#ffffff".to_string(),
                        }),
                        image: None,
                        interaction: None,
                    }),
                    ..EntityRecord::default()
                },
            )
            .expect("insert back ui");
        let mut render = RenderContext::new(ClearColor::BLACK);
        render.texture(&stale_texture);
        render.sprite(Sprite::new(
            stale_texture.id(),
            Transform2D::default(),
            1.0,
            1.0,
        ));

        run_render_frame_graph(
            &default_frame_render_graph(),
            &world,
            &render_cache,
            &mut render,
            |render| {
                assert_eq!(
                    render.state().sprites.len(),
                    1,
                    "world extraction must run before user render"
                );
                assert_eq!(
                    render
                        .ui_elements()
                        .iter()
                        .map(UiElement::entity)
                        .collect::<Vec<_>>(),
                    vec![ui_back, ui_front],
                    "ui extraction must run before user render and respect z-index"
                );
                render.texture(&user_texture);
                render.sprite(Sprite::new(
                    user_texture.id(),
                    Transform2D::from_translation(12.0, 24.0),
                    2.0,
                    2.0,
                ));
            },
        )
        .expect("run frame graph");

        let state = render.state();
        let texture_ids = state
            .textures
            .iter()
            .map(|texture| texture.id())
            .collect::<Vec<_>>();
        let sprite_texture_ids = state
            .sprites
            .iter()
            .map(|sprite| sprite.texture_id)
            .collect::<Vec<_>>();

        assert_eq!(texture_ids, vec![TextureId::new(2), TextureId::new(3)]);
        assert_eq!(
            sprite_texture_ids,
            vec![TextureId::new(2), TextureId::new(3)]
        );
    }

    #[test]
    fn entity_by_name_returns_lowest_entity_id_for_duplicate_names() {
        let mut world = World::default();
        let first = world.spawn(EntityRecord::named("Duplicate"));
        let second = world.spawn(EntityRecord::named("Duplicate"));

        assert!(first < second);
        assert_eq!(world.entity_by_name("Duplicate"), Some(first));
    }

    #[test]
    fn scene_transform_overrides_are_field_level() {
        let prefab = PrefabDocument::from_toml_str(
            r#"
            [components.transform]
            x = 1.0
            y = 2.0
            rotation_radians = 0.5
            scale_x = 3.0
            scale_y = 4.0
            "#,
        )
        .expect("parse prefab");
        let scene = SceneEntityDocument {
            transform: Some(seishin_world::SceneTransformDocument {
                x: Some(9.0),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resolved = resolve_scene_entity(scene, Some(prefab)).expect("resolve entity");

        assert_eq!(resolved.record.transform.x, 9.0);
        assert_eq!(resolved.record.transform.y, 2.0);
        assert_eq!(resolved.record.transform.rotation_radians, 0.5);
        assert_eq!(resolved.record.transform.scale_x, 3.0);
        assert_eq!(resolved.record.transform.scale_y, 4.0);
    }

    #[test]
    fn main_scene_loads_prefabs_names_tags_and_data_refs() {
        let mut startup = basic_2d_startup();

        startup
            .components()
            .register::<TestController>("PlayerController")
            .expect("register component");
        startup.load_main_scene().expect("load scene");

        let player = startup
            .world()
            .entity_by_name("Player")
            .expect("player entity");
        let merchant = startup
            .world()
            .entity_by_name("Merchant")
            .expect("merchant entity");

        assert_eq!(startup.world().first_with_tag("player"), Some(player));
        assert!(startup.world().entities_with_tag("npc").contains(&merchant));
        assert!(startup
            .world()
            .has_custom_component(player, "PlayerController"));
        assert!(startup.world().has_component::<TestController>(player));
        assert_eq!(
            startup.world().data_ref(merchant, "character"),
            Some("res://data/characters/merchant.toml")
        );
    }

    #[test]
    fn main_scene_reuses_texture_assets_for_repeated_prefab_sprites() {
        let mut startup = basic_2d_startup();

        startup
            .components()
            .register::<TestController>("PlayerController")
            .expect("register component");
        startup.load_main_scene().expect("load scene");

        let mut render = RenderContext::new(ClearColor::BLACK);
        render_world(&startup.world, &startup.render_cache, &mut render);
        let state = render.state();
        let texture_ids = state
            .sprites
            .iter()
            .map(|sprite| sprite.texture_id)
            .collect::<HashSet<_>>();

        assert_eq!(state.sprites.len(), 3);
        assert_eq!(texture_ids.len(), 1);
        assert_eq!(state.textures.len(), 1);
    }

    #[test]
    fn main_scene_loads_audio_assets_for_entity_playback() {
        let mut startup = startup_with_scene(
            "audio.scene.toml",
            r#"
            [[entities]]
            name = "Bell"

            [entities.audio]
            sound = "asset://audio/beep.wav"

            [[entities]]
            name = "Bell Echo"

            [entities.audio]
            sound = "asset://audio/beep.wav"
            "#,
        );
        write_asset(
            &startup.paths,
            "audio/beep.wav",
            b"not decoded during tests",
        );

        startup.load_main_scene().expect("load scene");

        let entity = startup.world().entity_by_name("Bell").expect("bell entity");
        let echo = startup
            .world()
            .entity_by_name("Bell Echo")
            .expect("bell echo entity");
        assert!(startup.audio_cache.contains_key(&entity));
        assert_eq!(
            startup.audio_cache.get(&entity),
            startup.audio_cache.get(&echo)
        );

        let resources = Resources::new(startup.paths.clone());
        let mut dialogue = DialogueState::default();
        let input_actions = startup.input_actions;
        let input = InputState::default();
        let mut audio = startup.audio;
        let mut world = startup.world;
        let audio_cache = startup.audio_cache;
        let mut frame = FrameContext {
            input: &input,
            input_actions: &input_actions,
            audio: &mut audio,
            audio_cache: &audio_cache,
            world: &mut world,
            resources: &resources,
            dialogue: &mut dialogue,
            frame: 1,
            delta_seconds: 1.0,
        };

        assert_eq!(
            frame.play_entity_audio(entity),
            Some(PlaybackResult::Skipped(
                seishin_audio::AudioSkipReason::BackendUnavailable(
                    "audio backend disabled for this target".to_string()
                )
            ))
        );
        assert_eq!(frame.play_entity_audio(EntityId::new(999)), None);
    }

    #[test]
    fn scene_loaded_player_moves_from_input_action() {
        let mut startup = basic_2d_startup();

        startup
            .components()
            .register::<TestController>("PlayerController")
            .expect("register component");
        startup.load_main_scene().expect("load scene");

        let resources = Resources::new(startup.paths.clone());
        let mut dialogue = DialogueState::default();
        let mut world = startup.world;
        let input_actions = startup.input_actions;
        let mut input = InputState::default();
        let mut audio = startup.audio;
        let audio_cache = startup.audio_cache;
        let player = world.first_with_tag("player").expect("player tag");
        let before = world.transform(player).expect("player transform");

        input.press(KeyCode::KeyD);
        let mut frame = FrameContext {
            input: &input,
            input_actions: &input_actions,
            audio: &mut audio,
            audio_cache: &audio_cache,
            world: &mut world,
            resources: &resources,
            dialogue: &mut dialogue,
            frame: 1,
            delta_seconds: 1.0,
        };
        let movement = frame.input().axis2d("move");
        let displacement = movement * TestController::DEFAULT_SPEED * frame.delta_seconds();

        frame
            .world()
            .entity(player)
            .set_position(before.x, before.y);
        frame.world().entity(player).translate(displacement);

        let after = frame.world().transform(player).expect("player transform");
        assert!(after.x > before.x);
        assert_eq!(after.y, before.y);
    }

    #[test]
    fn frame_world_entity_mut_supports_repeated_mutations_from_one_handle() {
        let resources = Resources::new(ProjectPaths::new(
            PathBuf::from("/project/assets"),
            PathBuf::from("/project/resources"),
            PathBuf::from("/user/seishin"),
        ));
        let mut dialogue = DialogueState::default();
        let mut world = World::default();
        let first = world.spawn(EntityRecord::default());
        let second = world.spawn(EntityRecord::default());
        let input_actions = InputActions::default();
        let input = InputState::default();
        let mut audio = default_audio_system();
        let audio_cache = AudioCache::default();
        let mut frame = FrameContext {
            input: &input,
            input_actions: &input_actions,
            audio: &mut audio,
            audio_cache: &audio_cache,
            world: &mut world,
            resources: &resources,
            dialogue: &mut dialogue,
            frame: 1,
            delta_seconds: 1.0,
        };
        let delta = Vec2::new(3.0, 4.0);

        let mut world = frame.world();
        world.entity(first).translate(delta);
        world.entity(second).set_position(10.0, 20.0);

        assert_eq!(
            frame.world().transform(first),
            Some(Transform2D::from_translation(3.0, 4.0))
        );
        assert_eq!(
            frame.world().transform(second),
            Some(Transform2D::from_translation(10.0, 20.0))
        );
    }

    #[test]
    fn frame_world_exposes_scene_lifecycle_operations() {
        let resources = Resources::new(ProjectPaths::new(
            PathBuf::from("/project/assets"),
            PathBuf::from("/project/resources"),
            PathBuf::from("/user/seishin"),
        ));
        let mut dialogue = DialogueState::default();
        let mut world = World::default();
        let input_actions = InputActions::default();
        let input = InputState::default();
        let mut audio = default_audio_system();
        let audio_cache = AudioCache::default();
        let mut frame = FrameContext {
            input: &input,
            input_actions: &input_actions,
            audio: &mut audio,
            audio_cache: &audio_cache,
            world: &mut world,
            resources: &resources,
            dialogue: &mut dialogue,
            frame: 1,
            delta_seconds: 1.0,
        };

        let mut world = frame.world();
        let loaded = world
            .load_scene_resolved(
                "res://scenes/runtime.scene.toml",
                [ResolvedEntity {
                    id: Some(EntityId::new(7)),
                    prefab: None,
                    record: EntityRecord::named("RuntimeEntity"),
                }],
            )
            .expect("load runtime scene");

        assert_eq!(world.name(EntityId::new(7)), Some("RuntimeEntity"));
        assert_eq!(loaded.entities(), &[EntityId::new(7)]);

        world.unload_scene(&loaded).expect("unload runtime scene");

        assert_eq!(world.name(EntityId::new(7)), None);
    }

    #[test]
    fn frame_context_dispatches_ui_interaction_actions_from_world_data() {
        let resources = Resources::new(ProjectPaths::new(
            PathBuf::from("/project/assets"),
            PathBuf::from("/project/resources"),
            PathBuf::from("/user/seishin"),
        ));
        let mut dialogue = DialogueState::default();
        let mut world = World::default();
        let button = world.spawn(EntityRecord {
            ui: Some(UiRef {
                layout: seishin_world::UiLayoutRef::default(),
                text: Some(seishin_world::UiTextRef {
                    value: "Confirm".to_string(),
                    font_size: 16.0,
                    color: "#ffffff".to_string(),
                }),
                image: None,
                interaction: Some(UiInteractionRef {
                    action: "confirm".to_string(),
                    enabled: true,
                }),
            }),
            ..EntityRecord::default()
        });
        let disabled = world.spawn(EntityRecord {
            ui: Some(UiRef {
                layout: seishin_world::UiLayoutRef::default(),
                text: None,
                image: None,
                interaction: Some(UiInteractionRef {
                    action: "confirm".to_string(),
                    enabled: false,
                }),
            }),
            ..EntityRecord::default()
        });
        let input_actions = InputActions::from_config(InputConfig {
            actions: HashMap::from([(
                "confirm".to_string(),
                InputActionConfig::Button {
                    keys: vec!["Enter".to_string()],
                },
            )]),
        });
        let mut input = InputState::default();
        input.press(KeyCode::Enter);
        let mut audio = default_audio_system();
        let audio_cache = AudioCache::default();
        let frame = FrameContext {
            input: &input,
            input_actions: &input_actions,
            audio: &mut audio,
            audio_cache: &audio_cache,
            world: &mut world,
            resources: &resources,
            dialogue: &mut dialogue,
            frame: 1,
            delta_seconds: 1.0,
        };

        assert!(frame.ui_action_pressed(button));
        assert!(frame.ui_action_just_pressed(button));
        assert!(!frame.ui_action_just_released(button));
        assert!(!frame.ui_action_pressed(disabled));
        assert!(!frame.ui_action_pressed(EntityId::new(999)));
    }

    #[test]
    fn schedule_runs_startup_once_and_frame_phases_in_order() {
        let resources = Resources::new(ProjectPaths::new(
            PathBuf::from("/project/assets"),
            PathBuf::from("/project/resources"),
            PathBuf::from("/user/seishin"),
        ));
        let mut dialogue = DialogueState::default();
        let mut world = World::default();
        let input_actions = InputActions::default();
        let input = InputState::default();
        let mut audio = default_audio_system();
        let audio_cache = AudioCache::default();
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut schedule = Schedule::default();

        schedule
            .add_system(SchedulePhase::Startup, "startup", {
                let calls = calls.clone();
                move |_frame| {
                    calls.borrow_mut().push("startup");
                    Ok(())
                }
            })
            .expect("add startup system");
        schedule
            .add_system(SchedulePhase::Update, "update", {
                let calls = calls.clone();
                move |frame| {
                    assert_eq!(frame.frame(), 1);
                    calls.borrow_mut().push("update");
                    Ok(())
                }
            })
            .expect("add update system");
        schedule
            .add_system(SchedulePhase::PostUpdate, "post_update", {
                let calls = calls.clone();
                move |_frame| {
                    calls.borrow_mut().push("post_update");
                    Ok(())
                }
            })
            .expect("add post update system");

        let mut frame = FrameContext {
            input: &input,
            input_actions: &input_actions,
            audio: &mut audio,
            audio_cache: &audio_cache,
            world: &mut world,
            resources: &resources,
            dialogue: &mut dialogue,
            frame: 1,
            delta_seconds: 1.0,
        };

        schedule.run_startup_once(&mut frame).expect("startup");
        schedule
            .run_phase(SchedulePhase::Update, &mut frame)
            .expect("update");
        schedule
            .run_phase(SchedulePhase::PostUpdate, &mut frame)
            .expect("post update");
        schedule
            .run_startup_once(&mut frame)
            .expect("startup already complete");

        assert_eq!(
            calls.borrow().as_slice(),
            ["startup", "update", "post_update"]
        );
    }

    #[test]
    fn runtime_executes_registered_schedule_around_game_update() {
        let root = unique_test_project_root("runtime-schedule");
        let asset_root = root.join("assets");
        let resource_root = root.join("resources");
        std::fs::create_dir_all(&asset_root).expect("create asset root");
        std::fs::create_dir_all(&resource_root).expect("create resource root");
        let paths = ProjectPaths::new(asset_root.clone(), resource_root, root.join("user"));
        let calls = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut startup = StartupContext::new(
            AssetRoot::new(&asset_root).expect("asset root"),
            InputActions::default(),
            ClearColor::BLACK,
            paths,
            None,
        );

        startup
            .schedule()
            .add_system(SchedulePhase::Startup, "startup", {
                let calls = calls.clone();
                move |_frame| {
                    calls.borrow_mut().push("startup");
                    Ok(())
                }
            })
            .expect("add startup system");
        startup
            .schedule()
            .add_system(SchedulePhase::Update, "update", {
                let calls = calls.clone();
                move |_frame| {
                    calls.borrow_mut().push("update");
                    Ok(())
                }
            })
            .expect("add update system");
        startup
            .schedule()
            .add_system(SchedulePhase::PostUpdate, "post_update", {
                let calls = calls.clone();
                move |_frame| {
                    calls.borrow_mut().push("post_update");
                    Ok(())
                }
            })
            .expect("add post update system");

        struct ScheduledGame {
            calls: std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>,
        }

        impl Game2D for ScheduledGame {
            fn new(_context: &mut StartupContext) -> GameResult<Self> {
                unreachable!("test constructs game directly")
            }

            fn update(&mut self, _context: &mut FrameContext<'_>) -> GameResult<()> {
                self.calls.borrow_mut().push("game");
                Ok(())
            }
        }

        let mut adapter = Game2DAdapter::new(
            ScheduledGame {
                calls: calls.clone(),
            },
            startup.into_runtime_parts(),
        );
        let mut engine = Engine::new(seishin_core::EngineConfig::default()).expect("engine");

        let first = engine.tick(1.0).expect("first frame");
        adapter.update(&mut engine, first).expect("first update");
        let second = engine.tick(1.0).expect("second frame");
        adapter.update(&mut engine, second).expect("second update");

        assert_eq!(
            calls.borrow().as_slice(),
            [
                "startup",
                "update",
                "game",
                "post_update",
                "update",
                "game",
                "post_update"
            ]
        );
    }

    #[test]
    fn scene_component_config_is_passed_to_registered_factory() {
        LAST_CONFIG_SPEED_BITS.store(0, std::sync::atomic::Ordering::SeqCst);
        let mut startup = startup_with_scene(
            "configured-component.scene.toml",
            r#"
            [[entities]]
            name = "Configured"

            [[entities.components]]
            type = "ConfiguredController"
            speed = 245.0
            "#,
        );

        startup
            .components()
            .register_factory("ConfiguredController", configured_test_controller_factory)
            .expect("register factory");
        startup.load_main_scene().expect("load scene");

        assert_eq!(
            f32::from_bits(LAST_CONFIG_SPEED_BITS.load(std::sync::atomic::Ordering::SeqCst)),
            245.0
        );
    }

    #[test]
    fn failed_main_scene_duplicate_ids_do_not_pollute_startup_state() {
        let mut startup = startup_with_scene(
            "duplicate.scene.toml",
            r#"
            [[entities]]
            id = 7
            name = "First"

            [[entities]]
            id = 7
            name = "Second"
            "#,
        );

        let error = startup
            .load_main_scene()
            .expect_err("duplicate explicit ids must fail");

        assert!(error.to_string().contains("duplicate entity id 7"));
        assert_eq!(startup.world.entities().count(), 0);
        assert!(startup.render_cache.is_empty());
        assert_eq!(startup.component_instances.len(), 0);

        write_scene(
            &startup.paths,
            "duplicate.scene.toml",
            r#"
            [[entities]]
            id = 7
            name = "First"
            "#,
        );

        startup.load_main_scene().expect("retry valid scene");
        assert_eq!(
            startup.world.entity_by_name("First"),
            Some(EntityId::new(7))
        );
    }

    #[test]
    fn failed_main_scene_overflow_id_does_not_pollute_startup_state() {
        let mut startup = startup_with_scene(
            "overflow.scene.toml",
            r#"
            [[entities]]
            id = 7
            name = "First"

            [[entities]]
            id = 18446744073709551615
            name = "Overflow"
            "#,
        );

        let error = startup
            .load_main_scene()
            .expect_err("overflow explicit id must fail");

        assert!(error.to_string().contains("18446744073709551615"));
        assert_eq!(startup.world.entities().count(), 0);
        assert!(startup.render_cache.is_empty());
        assert_eq!(startup.component_instances.len(), 0);

        write_scene(
            &startup.paths,
            "overflow.scene.toml",
            r#"
            [[entities]]
            id = 7
            name = "First"
            "#,
        );

        startup.load_main_scene().expect("retry valid scene");
        assert_eq!(
            startup.world.entity_by_name("First"),
            Some(EntityId::new(7))
        );
    }

    #[test]
    fn scene_entity_planning_rejects_max_explicit_id_without_mutating_world() {
        let mut world = World::default();
        let existing = world.spawn(EntityRecord::named("Existing"));
        let resolved = vec![
            ResolvedEntity {
                id: Some(EntityId::new(7)),
                prefab: None,
                record: EntityRecord::named("First"),
            },
            ResolvedEntity {
                id: Some(EntityId::new(u64::MAX)),
                prefab: None,
                record: EntityRecord::named("Overflow"),
            },
        ];

        let error = plan_scene_entities(&world, &resolved)
            .expect_err("max entity id cannot advance allocator");

        assert_eq!(
            error,
            seishin_world::WorldError::EntityIdOverflow(EntityId::new(u64::MAX))
        );
        assert_eq!(world.entities().count(), 1);
        assert_eq!(world.entity_by_name("Existing"), Some(existing));
        assert_eq!(world.entity_by_name("First"), None);
        assert_eq!(world.entity_by_name("Overflow"), None);
    }

    #[test]
    fn dialogue_resources_load_from_character_data() {
        let startup = basic_2d_startup();
        let resources = Resources::new(startup.paths.clone());

        let character = resources
            .character("res://data/characters/merchant.toml")
            .expect("merchant character");
        let dialogue_path = character
            .dialogue
            .as_ref()
            .expect("dialogue ref")
            .default
            .as_str();
        let dialogue = resources.dialogue(dialogue_path).expect("dialogue data");

        assert_eq!(character.display_name, "Merchant");
        assert_eq!(dialogue.id, "merchant_intro");
        assert!(dialogue.text.contains("prototype village"));
    }

    #[test]
    fn generic_toml_resources_are_accessible_to_components() {
        let startup = basic_2d_startup();
        let resources = Resources::new(startup.paths.clone());
        let config = resources
            .toml("res://data/components/player_controller.toml")
            .expect("player controller config");

        assert_eq!(config.f32("speed"), Some(180.0));
    }

    #[test]
    fn unknown_scene_component_reports_clear_error() {
        let mut startup = basic_2d_startup();

        let error = startup
            .load_main_scene()
            .expect_err("unregistered PlayerController must fail");

        assert!(error
            .to_string()
            .contains("unknown component type 'PlayerController'"));
    }

    #[test]
    fn public_component_trait_name_is_backend_agnostic() {
        fn assert_component<T: Component>() {}

        #[derive(Default)]
        struct BackendAgnosticComponent;

        impl Component for BackendAgnosticComponent {
            fn update(
                &mut self,
                _entity: Entity,
                _context: &mut FrameContext<'_>,
            ) -> GameResult<()> {
                Ok(())
            }
        }

        assert_component::<BackendAgnosticComponent>();
    }

    #[derive(Default)]
    struct TestController;

    impl TestController {
        const DEFAULT_SPEED: f32 = 180.0;
    }

    impl Component for TestController {
        fn update(&mut self, entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()> {
            context.world().translate(entity, Vec2::new(1.0, 0.0));
            Ok(())
        }
    }

    static LAST_CONFIG_SPEED_BITS: std::sync::atomic::AtomicU32 =
        std::sync::atomic::AtomicU32::new(0);

    fn configured_test_controller_factory(config: &toml::Value) -> GameResult<Box<dyn Component>> {
        let speed = config
            .get("speed")
            .and_then(toml::Value::as_float)
            .unwrap_or_default() as f32;
        LAST_CONFIG_SPEED_BITS.store(speed.to_bits(), std::sync::atomic::Ordering::SeqCst);
        Ok(Box::<TestController>::default())
    }

    fn basic_2d_startup() -> StartupContext {
        let project_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/basic_2d/Seishin.toml");
        let app = App::from_project(project_path).expect("basic_2d project");
        let paths = ProjectPaths::new(
            app.asset_root.clone(),
            app.resource_root.clone(),
            app.user_root.clone(),
        );
        let asset_root = AssetRoot::new(&app.asset_root).expect("asset root");

        StartupContext::new(
            asset_root,
            app.input_actions.clone(),
            app.clear_color,
            paths,
            Some("res://scenes/main.scene.toml".to_string()),
        )
    }

    fn startup_with_scene(scene_name: &str, scene: &str) -> StartupContext {
        let root = unique_test_project_root(scene_name);
        let asset_root = root.join("assets");
        let resource_root = root.join("resources");
        std::fs::create_dir_all(&asset_root).expect("create asset root");
        std::fs::create_dir_all(resource_root.join("scenes")).expect("create scene root");

        let paths = ProjectPaths::new(asset_root.clone(), resource_root, root.join("user"));
        write_scene(&paths, scene_name, scene);

        StartupContext::new(
            AssetRoot::new(&asset_root).expect("asset root"),
            InputActions::default(),
            ClearColor::BLACK,
            paths,
            Some(format!("res://scenes/{scene_name}")),
        )
    }

    fn unique_test_project_root(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "seishin-task5-{}-{}",
                std::process::id(),
                name.replace('.', "-")
            ))
            .join(nonce.to_string())
    }

    fn write_scene(paths: &ProjectPaths, scene_name: &str, scene: &str) {
        let path = paths.resource_root.join("scenes").join(scene_name);
        std::fs::write(path, scene).expect("write scene");
    }

    fn write_asset(paths: &ProjectPaths, asset_name: &str, contents: &[u8]) {
        let path = paths.asset_root.join(asset_name);
        std::fs::create_dir_all(path.parent().expect("asset parent")).expect("create asset parent");
        std::fs::write(path, contents).expect("write asset");
    }
}
