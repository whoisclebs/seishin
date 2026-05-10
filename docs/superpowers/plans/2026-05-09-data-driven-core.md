# Data-Driven Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract a lightweight backend-free `seishin_world` crate for world, scene, prefab, loading, and instancing data while preserving the current `seishin::run::<Game>()` behavior.

**Architecture:** Keep `EntityId` and `Transform2D` in `seishin_core`, move persistent world data and pure scene/prefab merge logic into `seishin_world`, and keep asset decoding, audio loading, renderer texture data, and platform file access at the `seishin` facade boundary. The first pass keeps TOML and `serde` because they already exist, but does not add any third-party dependency.

**Tech Stack:** Rust 2021, existing workspace crates, `serde`, `toml`, `std::collections`, current `cargo test`/`xtask` workflow.

---

## File Structure

- Create: `crates/seishin_world/Cargo.toml`
  - Backend-free crate metadata and dependencies on `seishin_core`, `serde`, and `toml`.
- Create: `crates/seishin_world/src/lib.rs`
  - Module facade and public exports.
- Create: `crates/seishin_world/src/record.rs`
  - `EntityRecord`, `SpriteRef`, `AudioRef`, `CustomComponentRef`, `InstanceSource`.
- Create: `crates/seishin_world/src/world.rs`
  - `World`, deterministic IDs, spawn/load/instantiate/query/mutation APIs.
- Create: `crates/seishin_world/src/document.rs`
  - TOML-facing `SceneDocument`, `PrefabDocument`, and document component structs.
- Create: `crates/seishin_world/src/resolve.rs`
  - Pure scene/prefab merge into `ResolvedEntity` records.
- Modify: `Cargo.toml`
  - Add `crates/seishin_world` to workspace members and `seishin_world` to workspace dependencies.
- Modify: `crates/seishin/Cargo.toml`
  - Add `seishin_world.workspace = true`.
- Modify: `crates/seishin/src/app.rs`
  - Remove local persistent world/document structs after the new crate exists.
  - Keep `SpriteRenderer`, `Texture`, `Assets`, `ComponentRegistry`, `RenderContext`, `StartupContext`, and runtime bridge code in the facade.
- Modify: `crates/seishin/src/lib.rs`
  - Reexport `seishin_world` and keep `seishin::prelude::*` compatible.
- Modify: `docs/architecture.md`
  - Document `seishin_world` and the dependency boundary.
- Modify: `docs/roadmap.md`
  - Add the data-driven core extraction to Near Term or mark it complete after implementation.

---

### Task 1: Workspace Wiring For `seishin_world`

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/seishin_world/Cargo.toml`
- Create: `crates/seishin_world/src/lib.rs`
- Test: `crates/seishin_world/src/lib.rs`

- [ ] **Step 1: Write the first crate smoke test**

Create `crates/seishin_world/src/lib.rs` with:

```rust
pub use seishin_core::{EntityId, Transform2D};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_crate_reexports_core_entity_primitives() {
        let entity = EntityId::new(7);
        let transform = Transform2D::from_translation(1.0, 2.0);

        assert_eq!(entity.raw(), 7);
        assert_eq!(transform.x, 1.0);
        assert_eq!(transform.y, 2.0);
    }
}
```

- [ ] **Step 2: Wire the crate into the workspace**

Add this member in the root `Cargo.toml` `members` list directly after `crates/seishin_core`:

```toml
    "crates/seishin_world",
```

Add this dependency in the root `Cargo.toml` `[workspace.dependencies]` block directly after `seishin_core`:

```toml
seishin_world = { path = "crates/seishin_world" }
```

Create `crates/seishin_world/Cargo.toml`:

```toml
[package]
name = "seishin_world"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
seishin_core.workspace = true
serde.workspace = true
toml.workspace = true
```

- [ ] **Step 3: Run the crate smoke test**

Run:

```sh
cargo test -p seishin_world
```

Expected: PASS with one test from `seishin_world`.

- [ ] **Step 4: Commit**

```sh
git add Cargo.toml crates/seishin_world
git commit -m "feat: add seishin world crate"
```

---

### Task 2: World Records And Deterministic IDs

**Files:**
- Modify: `crates/seishin_world/src/lib.rs`
- Create: `crates/seishin_world/src/record.rs`
- Create: `crates/seishin_world/src/world.rs`
- Test: `crates/seishin_world/src/world.rs`

- [ ] **Step 1: Write tests for deterministic IDs, queries, and mutation**

Create `crates/seishin_world/src/world.rs` with tests first:

```rust
use std::collections::HashMap;

use seishin_core::{EntityId, Transform2D};

use crate::record::EntityRecord;

#[derive(Debug, Clone)]
pub struct World {
    next_entity: u64,
    entities: HashMap<EntityId, EntityRecord>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            next_entity: 1,
            entities: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::SpriteRef;

    #[test]
    fn spawn_allocates_monotonic_entity_ids() {
        let mut world = World::default();

        let first = world.spawn(EntityRecord::default());
        let second = world.spawn(EntityRecord::default());

        assert_eq!(first.raw(), 1);
        assert_eq!(second.raw(), 2);
        assert!(first < second);
    }

    #[test]
    fn insert_with_explicit_id_preserves_scene_id_and_advances_allocator() {
        let mut world = World::default();
        let explicit = EntityId::new(42);

        world
            .insert(explicit, EntityRecord::named("Player"))
            .expect("insert explicit entity");
        let next = world.spawn(EntityRecord::default());

        assert_eq!(world.entity_by_name("Player"), Some(explicit));
        assert_eq!(next.raw(), 43);
    }

    #[test]
    fn duplicate_explicit_id_is_rejected() {
        let mut world = World::default();
        let entity = EntityId::new(3);

        world.insert(entity, EntityRecord::default()).expect("first insert");
        let error = world
            .insert(entity, EntityRecord::default())
            .expect_err("duplicate id must fail");

        assert_eq!(error, WorldError::DuplicateEntityId(entity));
    }

    #[test]
    fn name_and_tag_queries_are_deterministic() {
        let mut world = World::default();
        let high = EntityId::new(9);
        let low = EntityId::new(2);

        world
            .insert(high, EntityRecord::named("Duplicate").with_tag("npc"))
            .expect("high insert");
        world
            .insert(low, EntityRecord::named("Duplicate").with_tag("npc"))
            .expect("low insert");

        assert_eq!(world.entity_by_name("Duplicate"), Some(low));
        assert_eq!(world.entities_with_tag("npc"), vec![low, high]);
    }

    #[test]
    fn transform_mutation_keeps_render_data_separate() {
        let mut world = World::default();
        let entity = world.spawn(
            EntityRecord::default().with_sprite(SpriteRef {
                texture: "asset://sprites/player.png".to_string(),
                width: Some(96.0),
                height: Some(96.0),
            }),
        );

        world.translate(entity, 3.0, 4.0);
        world.set_position(entity, 10.0, 20.0);

        assert_eq!(
            world.transform(entity),
            Some(Transform2D::from_translation(10.0, 20.0))
        );
        assert_eq!(
            world.sprite(entity).map(|sprite| sprite.texture.as_str()),
            Some("asset://sprites/player.png")
        );
    }
}
```

- [ ] **Step 2: Add record types**

Create `crates/seishin_world/src/record.rs`:

```rust
use std::{any::TypeId, collections::HashMap};

use seishin_core::{EntityId, Transform2D};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntityRecord {
    pub name: Option<String>,
    pub tags: Vec<String>,
    pub data_refs: HashMap<String, String>,
    pub custom_components: Vec<CustomComponentRef>,
    pub transform: Transform2D,
    pub sprite: Option<SpriteRef>,
    pub audio: Option<AudioRef>,
    pub instance_source: Option<InstanceSource>,
}

impl EntityRecord {
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::default()
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_sprite(mut self, sprite: SpriteRef) -> Self {
        self.sprite = Some(sprite);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpriteRef {
    pub texture: String,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioRef {
    pub sound: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustomComponentRef {
    pub type_name: String,
    pub type_id: Option<TypeId>,
    pub config: toml::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceSource {
    pub scene: String,
    pub source_entity: EntityId,
}
```

- [ ] **Step 3: Implement `World`**

Replace the non-test portion of `crates/seishin_world/src/world.rs` with:

```rust
use std::collections::HashMap;

use seishin_core::{EntityId, Transform2D};

use crate::record::{EntityRecord, SpriteRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    DuplicateEntityId(EntityId),
    MissingEntity(EntityId),
}

impl std::fmt::Display for WorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateEntityId(entity) => {
                write!(f, "duplicate entity id {}", entity.raw())
            }
            Self::MissingEntity(entity) => write!(f, "missing entity {}", entity.raw()),
        }
    }
}

impl std::error::Error for WorldError {}

#[derive(Debug, Clone)]
pub struct World {
    next_entity: u64,
    entities: HashMap<EntityId, EntityRecord>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            next_entity: 1,
            entities: HashMap::new(),
        }
    }
}

impl World {
    pub fn spawn(&mut self, record: EntityRecord) -> EntityId {
        let entity = self.allocate_entity();
        self.entities.insert(entity, record);
        entity
    }

    pub fn insert(&mut self, entity: EntityId, record: EntityRecord) -> Result<(), WorldError> {
        if self.entities.contains_key(&entity) {
            return Err(WorldError::DuplicateEntityId(entity));
        }

        self.next_entity = self.next_entity.max(entity.raw().saturating_add(1));
        self.entities.insert(entity, record);
        Ok(())
    }

    pub fn entity(&self, entity: EntityId) -> Option<&EntityRecord> {
        self.entities.get(&entity)
    }

    pub fn entity_mut(&mut self, entity: EntityId) -> Option<&mut EntityRecord> {
        self.entities.get_mut(&entity)
    }

    pub fn entities(&self) -> impl Iterator<Item = (EntityId, &EntityRecord)> + '_ {
        let mut entities = self.entities.iter().map(|(id, record)| (*id, record)).collect::<Vec<_>>();
        entities.sort_by_key(|(id, _)| *id);
        entities.into_iter()
    }

    pub fn entity_by_name(&self, name: &str) -> Option<EntityId> {
        self.entities
            .iter()
            .filter_map(|(entity, record)| {
                record
                    .name
                    .as_deref()
                    .is_some_and(|value| value == name)
                    .then_some(*entity)
            })
            .min()
    }

    pub fn entities_with_tag(&self, tag: &str) -> Vec<EntityId> {
        let mut entities = self
            .entities
            .iter()
            .filter_map(|(entity, record)| {
                record.tags.iter().any(|value| value == tag).then_some(*entity)
            })
            .collect::<Vec<_>>();
        entities.sort();
        entities
    }

    pub fn first_with_tag(&self, tag: &str) -> Option<EntityId> {
        self.entities_with_tag(tag).into_iter().next()
    }

    pub fn tags(&self, entity: EntityId) -> Option<&[String]> {
        self.entities.get(&entity).map(|record| record.tags.as_slice())
    }

    pub fn transform(&self, entity: EntityId) -> Option<Transform2D> {
        self.entities.get(&entity).map(|record| record.transform)
    }

    pub fn name(&self, entity: EntityId) -> Option<&str> {
        self.entities.get(&entity).and_then(|record| record.name.as_deref())
    }

    pub fn data_ref(&self, entity: EntityId, key: &str) -> Option<&str> {
        self.entities
            .get(&entity)
            .and_then(|record| record.data_refs.get(key))
            .map(String::as_str)
    }

    pub fn sprite(&self, entity: EntityId) -> Option<&SpriteRef> {
        self.entities.get(&entity).and_then(|record| record.sprite.as_ref())
    }

    pub fn translate(&mut self, entity: EntityId, delta_x: f32, delta_y: f32) {
        if let Some(record) = self.entities.get_mut(&entity) {
            record.transform = record.transform.translated(delta_x, delta_y);
        }
    }

    pub fn set_position(&mut self, entity: EntityId, x: f32, y: f32) {
        if let Some(record) = self.entities.get_mut(&entity) {
            record.transform.x = x;
            record.transform.y = y;
        }
    }

    fn allocate_entity(&mut self) -> EntityId {
        let entity = EntityId::new(self.next_entity);
        self.next_entity += 1;
        entity
    }
}
```

Keep the tests from Step 1 below the implementation.

- [ ] **Step 4: Export modules**

Replace `crates/seishin_world/src/lib.rs` with:

```rust
pub mod record;
pub mod world;

pub use record::{AudioRef, CustomComponentRef, EntityRecord, InstanceSource, SpriteRef};
pub use seishin_core::{EntityId, Transform2D};
pub use world::{World, WorldError};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_crate_reexports_core_entity_primitives() {
        let entity = EntityId::new(7);
        let transform = Transform2D::from_translation(1.0, 2.0);

        assert_eq!(entity.raw(), 7);
        assert_eq!(transform.x, 1.0);
        assert_eq!(transform.y, 2.0);
    }
}
```

- [ ] **Step 5: Run world tests**

Run:

```sh
cargo test -p seishin_world
```

Expected: PASS with tests covering ID allocation, explicit IDs, duplicate rejection, deterministic queries, and transform mutation.

- [ ] **Step 6: Commit**

```sh
git add crates/seishin_world
git commit -m "feat: add lightweight world storage"
```

---

### Task 3: Scene Documents And Pure Prefab Merge

**Files:**
- Modify: `crates/seishin_world/src/lib.rs`
- Create: `crates/seishin_world/src/document.rs`
- Create: `crates/seishin_world/src/resolve.rs`
- Test: `crates/seishin_world/src/resolve.rs`

- [ ] **Step 1: Write document and merge tests**

Create `crates/seishin_world/src/resolve.rs` with tests first:

```rust
use std::collections::HashMap;

use seishin_core::Transform2D;

use crate::{
    document::{PrefabDocument, SceneDocument, SceneEntityDocument, SceneSpriteDocument},
    record::{CustomComponentRef, EntityRecord, SpriteRef},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedEntity {
    pub id: Option<seishin_core::EntityId>,
    pub prefab: Option<String>,
    pub record: EntityRecord,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_document_parses_optional_entity_ids() {
        let scene = SceneDocument::from_toml_str(
            r#"
            [[entities]]
            id = 7
            name = "Player"

            [entities.transform]
            x = 10.0
            y = 20.0
            "#,
        )
        .expect("parse scene");

        assert_eq!(scene.entities.len(), 1);
        assert_eq!(scene.entities[0].id, Some(7));
        assert_eq!(scene.entities[0].name.as_deref(), Some("Player"));
    }

    #[test]
    fn prefab_sprite_and_scene_transform_are_merged() {
        let mut prefab = PrefabDocument::default();
        prefab.components.insert(
            "sprite".to_string(),
            toml::Value::Table(
                [
                    ("texture".to_string(), toml::Value::String("asset://sprites/player.png".to_string())),
                    ("width".to_string(), toml::Value::Float(96.0)),
                    ("height".to_string(), toml::Value::Float(96.0)),
                ]
                .into_iter()
                .collect(),
            ),
        );

        let scene = SceneEntityDocument {
            name: Some("Player".to_string()),
            transform: Some(crate::document::SceneTransformDocument {
                x: Some(5.0),
                y: Some(6.0),
                ..Default::default()
            }),
            prefab: Some("res://prefabs/player.prefab.toml".to_string()),
            ..Default::default()
        };

        let resolved = resolve_scene_entity(scene, Some(prefab)).expect("resolve entity");

        assert_eq!(resolved.record.name.as_deref(), Some("Player"));
        assert_eq!(resolved.record.transform, Transform2D::from_translation(5.0, 6.0));
        assert_eq!(
            resolved.record.sprite,
            Some(SpriteRef {
                texture: "asset://sprites/player.png".to_string(),
                width: Some(96.0),
                height: Some(96.0),
            })
        );
    }

    #[test]
    fn scene_custom_component_overrides_prefab_component_by_type_name() {
        let mut prefab = PrefabDocument::default();
        prefab.components.insert(
            "controller".to_string(),
            toml::Value::Table(
                [
                    ("type".to_string(), toml::Value::String("PlayerController".to_string())),
                    ("speed".to_string(), toml::Value::Float(100.0)),
                ]
                .into_iter()
                .collect(),
            ),
        );

        let scene = SceneEntityDocument {
            components: vec![crate::document::CustomComponentDocument {
                type_name: "PlayerController".to_string(),
                config: [("speed".to_string(), toml::Value::Float(180.0))]
                    .into_iter()
                    .collect(),
            }],
            ..Default::default()
        };

        let resolved = resolve_scene_entity(scene, Some(prefab)).expect("resolve entity");
        let component = resolved
            .record
            .custom_components
            .iter()
            .find(|component| component.type_name == "PlayerController")
            .expect("controller component");

        assert_eq!(component.config.get("speed").and_then(toml::Value::as_float), Some(180.0));
    }
}
```

- [ ] **Step 2: Implement document types**

Create `crates/seishin_world/src/document.rs`:

```rust
use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SceneDocument {
    #[serde(default)]
    pub entities: Vec<SceneEntityDocument>,
}

impl SceneDocument {
    pub fn from_toml_str(source: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(source)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SceneEntityDocument {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub prefab: Option<String>,
    pub transform: Option<SceneTransformDocument>,
    pub tags: Option<TagsDocument>,
    pub data: Option<HashMap<String, String>>,
    pub sprite: Option<SceneSpriteDocument>,
    pub audio: Option<SceneAudioDocument>,
    #[serde(default)]
    pub components: Vec<CustomComponentDocument>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PrefabDocument {
    #[serde(default)]
    pub components: HashMap<String, toml::Value>,
}

impl PrefabDocument {
    pub fn from_toml_str(source: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(source)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TagsDocument {
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct SceneTransformDocument {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub rotation_radians: Option<f32>,
    pub scale_x: Option<f32>,
    pub scale_y: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SceneSpriteDocument {
    pub texture: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SceneAudioDocument {
    pub sound: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomComponentDocument {
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(flatten)]
    pub config: HashMap<String, toml::Value>,
}
```

- [ ] **Step 3: Implement pure resolve logic**

Replace the non-test portion of `crates/seishin_world/src/resolve.rs` with:

```rust
use seishin_core::{EntityId, Transform2D};

use crate::{
    document::{
        CustomComponentDocument, PrefabDocument, SceneAudioDocument, SceneEntityDocument,
        SceneSpriteDocument, SceneTransformDocument,
    },
    record::{AudioRef, CustomComponentRef, EntityRecord, SpriteRef},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedEntity {
    pub id: Option<EntityId>,
    pub prefab: Option<String>,
    pub record: EntityRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    InvalidPrefabComponent(String),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPrefabComponent(name) => {
                write!(f, "invalid prefab component '{name}'")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

pub fn resolve_scene_entity(
    entity: SceneEntityDocument,
    prefab: Option<PrefabDocument>,
) -> Result<ResolvedEntity, ResolveError> {
    let mut record = prefab.map(prefab_to_record).transpose()?.unwrap_or_default();

    if entity.name.is_some() {
        record.name = entity.name;
    }

    if let Some(tags) = entity.tags {
        record.tags = tags.values;
    }

    if let Some(data) = entity.data {
        record.data_refs.extend(data);
    }

    if let Some(transform) = entity.transform {
        record.transform = merge_transform(record.transform, transform);
    }

    if let Some(sprite) = entity.sprite {
        record.sprite = merge_sprite(record.sprite.take(), sprite);
    }

    if let Some(audio) = entity.audio {
        record.audio = merge_audio(record.audio.take(), audio);
    }

    for component in entity.components {
        let component = custom_component_ref(component);
        record
            .custom_components
            .retain(|existing| existing.type_name != component.type_name);
        record.custom_components.push(component);
    }

    Ok(ResolvedEntity {
        id: entity.id.map(EntityId::new),
        prefab: entity.prefab,
        record,
    })
}

fn prefab_to_record(prefab: PrefabDocument) -> Result<EntityRecord, ResolveError> {
    let mut record = EntityRecord::default();

    for (name, value) in prefab.components {
        match name.as_str() {
            "name" => {
                record.name = value
                    .get("value")
                    .and_then(toml::Value::as_str)
                    .map(ToOwned::to_owned);
            }
            "tags" => {
                record.tags = value
                    .get("values")
                    .and_then(toml::Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(toml::Value::as_str)
                            .map(ToOwned::to_owned)
                            .collect()
                    })
                    .unwrap_or_default();
            }
            "transform" => {
                let transform = value
                    .try_into()
                    .map_err(|_| ResolveError::InvalidPrefabComponent(name.clone()))?;
                record.transform = merge_transform(record.transform, transform);
            }
            "sprite" => {
                let sprite = value
                    .try_into()
                    .map_err(|_| ResolveError::InvalidPrefabComponent(name.clone()))?;
                record.sprite = merge_sprite(record.sprite.take(), sprite);
            }
            "audio" => {
                let audio = value
                    .try_into()
                    .map_err(|_| ResolveError::InvalidPrefabComponent(name.clone()))?;
                record.audio = merge_audio(record.audio.take(), audio);
            }
            _ => {
                if let Some(type_name) = value.get("type").and_then(toml::Value::as_str) {
                    let config = value
                        .as_table()
                        .map(|table| table.clone().into_iter().collect())
                        .unwrap_or_default();
                    record.custom_components.push(CustomComponentRef {
                        type_name: type_name.to_string(),
                        type_id: None,
                        config: toml::Value::Table(config),
                    });
                }
            }
        }
    }

    Ok(record)
}

fn custom_component_ref(component: CustomComponentDocument) -> CustomComponentRef {
    CustomComponentRef {
        type_name: component.type_name,
        type_id: None,
        config: toml::Value::Table(component.config.into_iter().collect()),
    }
}

fn merge_sprite(base: Option<SpriteRef>, override_value: SceneSpriteDocument) -> Option<SpriteRef> {
    let texture = override_value
        .texture
        .or_else(|| base.as_ref().map(|sprite| sprite.texture.clone()))?;

    Some(SpriteRef {
        texture,
        width: override_value.width.or_else(|| base.as_ref().and_then(|sprite| sprite.width)),
        height: override_value
            .height
            .or_else(|| base.as_ref().and_then(|sprite| sprite.height)),
    })
}

fn merge_audio(base: Option<AudioRef>, override_value: SceneAudioDocument) -> Option<AudioRef> {
    let sound = override_value
        .sound
        .or_else(|| base.as_ref().map(|audio| audio.sound.clone()))?;

    Some(AudioRef { sound })
}

fn merge_transform(mut base: Transform2D, override_value: SceneTransformDocument) -> Transform2D {
    if let Some(x) = override_value.x {
        base.x = x;
    }
    if let Some(y) = override_value.y {
        base.y = y;
    }
    if let Some(rotation_radians) = override_value.rotation_radians {
        base.rotation_radians = rotation_radians;
    }
    if let Some(scale_x) = override_value.scale_x {
        base.scale_x = scale_x;
    }
    if let Some(scale_y) = override_value.scale_y {
        base.scale_y = scale_y;
    }
    base
}
```

Keep the tests from Step 1 below the implementation.

- [ ] **Step 4: Export document and resolve APIs**

Update `crates/seishin_world/src/lib.rs`:

```rust
pub mod document;
pub mod record;
pub mod resolve;
pub mod world;

pub use document::{
    CustomComponentDocument, PrefabDocument, SceneAudioDocument, SceneDocument,
    SceneEntityDocument, SceneSpriteDocument, SceneTransformDocument, TagsDocument,
};
pub use record::{AudioRef, CustomComponentRef, EntityRecord, InstanceSource, SpriteRef};
pub use resolve::{resolve_scene_entity, ResolveError, ResolvedEntity};
pub use seishin_core::{EntityId, Transform2D};
pub use world::{World, WorldError};
```

- [ ] **Step 5: Run scene merge tests**

Run:

```sh
cargo test -p seishin_world
```

Expected: PASS with scene parsing, prefab merge, and custom component override tests.

- [ ] **Step 6: Commit**

```sh
git add crates/seishin_world
git commit -m "feat: add scene document resolution"
```

---

### Task 4: Scene Loading And Instancing Into World

**Files:**
- Modify: `crates/seishin_world/src/world.rs`
- Modify: `crates/seishin_world/src/lib.rs`
- Test: `crates/seishin_world/src/world.rs`

- [ ] **Step 1: Add tests for loading and instancing**

Append these tests to the `tests` module in `crates/seishin_world/src/world.rs`:

```rust
use crate::record::InstanceSource;

#[test]
fn load_resolved_entities_preserves_explicit_ids() {
    let mut world = World::default();
    let loaded = world
        .load_resolved([
            crate::resolve::ResolvedEntity {
                id: Some(EntityId::new(5)),
                prefab: None,
                record: EntityRecord::named("SavedPlayer"),
            },
            crate::resolve::ResolvedEntity {
                id: None,
                prefab: None,
                record: EntityRecord::named("GeneratedNpc"),
            },
        ])
        .expect("load entities");

    assert_eq!(loaded, vec![EntityId::new(5), EntityId::new(6)]);
    assert_eq!(world.entity_by_name("SavedPlayer"), Some(EntityId::new(5)));
    assert_eq!(world.entity_by_name("GeneratedNpc"), Some(EntityId::new(6)));
}

#[test]
fn instantiate_resolved_entities_allocates_fresh_ids_and_source_links() {
    let mut world = World::default();
    world
        .insert(EntityId::new(1), EntityRecord::named("Existing"))
        .expect("insert existing");

    let instance = world.instantiate_resolved(
        "res://scenes/room.scene.toml",
        [crate::resolve::ResolvedEntity {
            id: Some(EntityId::new(1)),
            prefab: None,
            record: EntityRecord::named("RoomPlayer"),
        }],
    );

    let instance = instance.expect("instantiate");
    let spawned = instance.entity_for_source(EntityId::new(1)).expect("source map");

    assert_ne!(spawned, EntityId::new(1));
    assert_eq!(spawned.raw(), 2);
    assert_eq!(
        world.entity(spawned).and_then(|record| record.instance_source.as_ref()),
        Some(&InstanceSource {
            scene: "res://scenes/room.scene.toml".to_string(),
            source_entity: EntityId::new(1),
        })
    );
}
```

- [ ] **Step 2: Implement load and instantiate APIs**

Add this type above `impl World` in `crates/seishin_world/src/world.rs`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneInstance {
    source_to_instance: HashMap<EntityId, EntityId>,
}

impl SceneInstance {
    pub fn entity_for_source(&self, source: EntityId) -> Option<EntityId> {
        self.source_to_instance.get(&source).copied()
    }
}
```

Add these imports:

```rust
use crate::{
    record::{EntityRecord, InstanceSource, SpriteRef},
    resolve::ResolvedEntity,
};
```

Replace the existing `use crate::record::{EntityRecord, SpriteRef};` import with the combined import above.

Add these methods inside `impl World`:

```rust
pub fn load_resolved(
    &mut self,
    entities: impl IntoIterator<Item = ResolvedEntity>,
) -> Result<Vec<EntityId>, WorldError> {
    let mut loaded = Vec::new();

    for resolved in entities {
        let entity = match resolved.id {
            Some(entity) => {
                self.insert(entity, resolved.record)?;
                entity
            }
            None => self.spawn(resolved.record),
        };
        loaded.push(entity);
    }

    Ok(loaded)
}

pub fn instantiate_resolved(
    &mut self,
    scene: impl Into<String>,
    entities: impl IntoIterator<Item = ResolvedEntity>,
) -> Result<SceneInstance, WorldError> {
    let scene = scene.into();
    let mut instance = SceneInstance::default();

    for resolved in entities {
        let source_entity = resolved.id.unwrap_or_else(|| EntityId::new(0));
        let mut record = resolved.record;
        record.instance_source = Some(InstanceSource {
            scene: scene.clone(),
            source_entity,
        });
        let entity = self.spawn(record);
        instance.source_to_instance.insert(source_entity, entity);
    }

    Ok(instance)
}
```

- [ ] **Step 3: Export `SceneInstance`**

Update the `world` reexport in `crates/seishin_world/src/lib.rs`:

```rust
pub use world::{SceneInstance, World, WorldError};
```

- [ ] **Step 4: Run world tests**

Run:

```sh
cargo test -p seishin_world
```

Expected: PASS with explicit load and instancing tests.

- [ ] **Step 5: Commit**

```sh
git add crates/seishin_world
git commit -m "feat: support scene load and instancing"
```

---

### Task 5: Integrate `seishin_world` Into Facade

**Files:**
- Modify: `crates/seishin/Cargo.toml`
- Modify: `crates/seishin/src/lib.rs`
- Modify: `crates/seishin/src/app.rs`
- Test: `crates/seishin/src/app.rs`

- [ ] **Step 1: Add facade dependency and reexports**

Add this dependency to `crates/seishin/Cargo.toml` after `seishin_core.workspace = true`:

```toml
seishin_world.workspace = true
```

In `crates/seishin/src/lib.rs`, add:

```rust
pub mod world {
    pub use seishin_world::*;
}
```

Update the `core`/`world` prelude area by adding:

```rust
    pub use seishin_world::{AudioRef, InstanceSource, SpriteRef};
```

Keep `World` and `Entity` available from the top-level `pub use app::{ ... }` list during this task; remove them from `app` only after `app.rs` compiles against `seishin_world::World`.

- [ ] **Step 2: Change `app.rs` imports to use world data**

In `crates/seishin/src/app.rs`, add:

```rust
use seishin_world::{
    resolve_scene_entity, CustomComponentRef, EntityRecord, PrefabDocument, SceneDocument,
    SceneEntityDocument, SpriteRef, World,
};
```

Remove `EntityId` from the existing `seishin_core` import only if it becomes unused. Keep:

```rust
pub type Entity = EntityId;
```

- [ ] **Step 3: Replace local scene document parsing types**

Delete these local types from `crates/seishin/src/app.rs` after references are moved:

```rust
struct SceneConfig
struct SceneEntityConfig
struct PrefabConfig
struct TagsConfig
struct SceneTransformConfig
struct SceneSpriteConfig
struct CustomComponentConfig
struct EntityBlueprint
```

Also delete the local `merge_sprite` and `merge_transform` functions after `resolve_scene_entity` owns merge behavior.

Replace `load_scene_config` with:

```rust
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
```

Replace `load_prefab_config` with:

```rust
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
```

Update `load_prefab_config_cached` to return `PrefabDocument` and use `HashMap<String, PrefabDocument>`.

- [ ] **Step 4: Replace `build_scene_entity` with resolver-backed load**

Replace `build_scene_entity` with:

```rust
fn build_scene_entity(
    entity: SceneEntityDocument,
    startup: &mut StartupContext,
    prefab_cache: &mut HashMap<String, PrefabDocument>,
) -> GameResult<EntityRecord> {
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

    Ok(resolved.record)
}
```

Add these helper functions near `build_scene_entity`:

```rust
fn validate_custom_components(record: &EntityRecord, registry: &ComponentRegistry) -> GameResult<()> {
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
```

- [ ] **Step 5: Keep facade render integration in `app.rs`**

Do not move `SpriteRenderer` into `seishin_world`.

Add this helper near `load_main_scene`:

```rust
fn load_render_assets(record: &EntityRecord, assets: &mut Assets) -> GameResult<Option<SpriteRenderer>> {
    let Some(sprite) = &record.sprite else {
        return Ok(None);
    };

    Ok(Some(SpriteRenderer::new(
        assets.texture(&sprite.texture)?,
        Vec2::new(sprite.width.unwrap_or(32.0), sprite.height.unwrap_or(32.0)),
    )))
}
```

If `seishin_world::World` stores `SpriteRef`, adjust `World::render_into` replacement in `app.rs` to iterate through `world.entities()` and translate `SpriteRef` plus loaded `Texture` into the existing `RenderContext`. The simplest safe bridge is to keep a facade-local render cache:

```rust
type RenderCache = HashMap<Entity, SpriteRenderer>;
```

Add `render_cache: RenderCache` to `RuntimeParts` and `Game2DAdapter`. In `load_main_scene`, after the record is built and before custom component instantiation, call `load_render_assets(&record, &mut startup.assets)?` and store the returned `SpriteRenderer` in a startup-local cache keyed by the inserted entity.

- [ ] **Step 6: Update `load_main_scene` to preserve explicit scene IDs**

Change the `StartupContext` fields by adding:

```rust
render_cache: RenderCache,
```

Initialize it in `StartupContext::new`:

```rust
render_cache: HashMap::new(),
```

Add it to `RuntimeParts`:

```rust
render_cache: RenderCache,
```

In `load_main_scene`, replace the old spawn block with:

```rust
for entity in scene.entities {
    let explicit_id = entity.id.map(EntityId::new);
    let mut record = build_scene_entity(entity, startup, &mut prefab_cache)?;
    let custom_components = record.custom_components.clone();
    let renderer = load_render_assets(&record, &mut startup.assets)?;
    let entity = match explicit_id {
        Some(entity) => {
            startup.world.insert(entity, record)?;
            entity
        }
        None => startup.world.spawn(record),
    };

    if let Some(renderer) = renderer {
        startup.render_cache.insert(entity, renderer);
    }

    for component_ref in custom_components {
        let instance = startup.components.instantiate(&component_ref)?;
        if let Some(type_id) = startup.components.type_id(&component_ref.type_name) {
            if let Some(record) = startup.world.entity_mut(entity) {
                if let Some(component) = record
                    .custom_components
                    .iter_mut()
                    .find(|component| component.type_name == component_ref.type_name)
                {
                    component.type_id = Some(type_id);
                }
            }
        }
        startup.component_instances.push(RuntimeComponent {
            entity,
            component: instance,
        });
    }
}
```

- [ ] **Step 7: Replace `World::render_into` usage**

Remove the old `World::render_into` method from `app.rs`.

Add this facade-local function:

```rust
fn render_world(world: &World, render_cache: &RenderCache, render: &mut RenderContext) {
    for (entity, record) in world.entities() {
        let Some(renderer) = render_cache.get(&entity) else {
            continue;
        };

        render.texture(&renderer.texture);
        render.sprite(Sprite::new(
            renderer.texture.id(),
            record.transform,
            renderer.size.x,
            renderer.size.y,
        ));
    }
}
```

Replace:

```rust
runtime_parts.world.render_into(&mut render);
```

with:

```rust
render_world(&runtime_parts.world, &runtime_parts.render_cache, &mut render);
```

Replace the update-frame render call:

```rust
self.world.render_into(&mut self.render);
```

with:

```rust
render_world(&self.world, &self.render_cache, &mut self.render);
```

- [ ] **Step 8: Run facade tests and fix compile errors from extraction**

Run:

```sh
cargo test -p seishin
```

Expected: PASS. If compile errors mention now-private world fields, add narrow accessor methods to `seishin_world` instead of making fields public.

- [ ] **Step 9: Commit**

```sh
git add Cargo.toml crates/seishin crates/seishin_world
git commit -m "refactor: move world data into seishin_world"
```

---

### Task 6: Keep Public API Compatibility And Add Facade Regression Tests

**Files:**
- Modify: `crates/seishin/src/app.rs`
- Modify: `crates/seishin/src/lib.rs`
- Test: `crates/seishin/src/app.rs`

- [ ] **Step 1: Add regression tests for current scene behavior**

Keep or restore these tests in `crates/seishin/src/app.rs`:

```rust
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
    let player = world.first_with_tag("player").expect("player tag");
    let before = world.transform(player).expect("player transform");

    input.press(KeyCode::KeyD);
    let mut frame = FrameContext {
        input: &input,
        input_actions: &input_actions,
        audio: &mut audio,
        world: &mut world,
        resources: &resources,
        dialogue: &mut dialogue,
        frame: 1,
        delta_seconds: 1.0,
    };
    let movement = frame.input().axis2d("move");
    let displacement = movement * TestController::DEFAULT_SPEED * frame.delta_seconds();

    frame.world().entity(player).translate(displacement);

    let after = frame.world().transform(player).expect("player transform");
    assert!(after.x > before.x);
    assert_eq!(after.y, before.y);
}
```

- [ ] **Step 2: Add compatibility methods if missing**

If `seishin_world::World` does not yet expose these methods, add them to `crates/seishin_world/src/world.rs`:

```rust
pub fn first_interactable(&self) -> Option<EntityId> {
    self.first_with_tag("interactable")
}

pub fn has_custom_component(&self, entity: EntityId, type_name: &str) -> bool {
    self.entities.get(&entity).is_some_and(|record| {
        record
            .custom_components
            .iter()
            .any(|component| component.type_name == type_name)
    })
}

pub fn custom_component_config(&self, entity: EntityId, type_name: &str) -> Option<&toml::Value> {
    self.entities.get(&entity).and_then(|record| {
        record
            .custom_components
            .iter()
            .find(|component| component.type_name == type_name)
            .map(|component| &component.config)
    })
}
```

Add this type-id helper to `crates/seishin_world/src/world.rs`:

```rust
pub fn has_component_type_id(&self, entity: EntityId, type_id: std::any::TypeId) -> bool {
    self.entities.get(&entity).is_some_and(|record| {
        record
            .custom_components
            .iter()
            .any(|component| component.type_id == Some(type_id))
    })
}
```

Add this extension trait to `crates/seishin/src/app.rs` so the existing `startup.world().has_component::<T>(entity)` call remains valid when the trait is in prelude/test scope:

```rust
pub trait WorldComponentExt {
    fn has_component<T: Component2D + 'static>(&self, entity: Entity) -> bool;
}

impl WorldComponentExt for World {
    fn has_component<T: Component2D + 'static>(&self, entity: Entity) -> bool {
        self.has_component_type_id(entity, TypeId::of::<T>())
    }
}
```

Export `WorldComponentExt` from `crates/seishin/src/lib.rs` and the prelude if the trait path is used.

- [ ] **Step 3: Run facade and world tests**

Run:

```sh
cargo test -p seishin_world
cargo test -p seishin
```

Expected: both packages pass.

- [ ] **Step 4: Commit**

```sh
git add crates/seishin crates/seishin_world
git commit -m "test: preserve scene facade behavior"
```

---

### Task 7: Dependency Audit And Light Build Documentation

**Files:**
- Modify: `docs/architecture.md`
- Modify: `docs/roadmap.md`
- Test: command output only

- [ ] **Step 1: Capture current package dependency shape**

Run:

```sh
cargo tree -p seishin_world --depth 2
cargo tree -p seishin --depth 1
```

Expected:

- `seishin_world` depends only on `seishin_core`, `serde`, and `toml` plus their transitive dependencies.
- `seishin` still pulls render/runtime/audio/assets dependencies through the facade.

- [ ] **Step 2: Update architecture docs**

In `docs/architecture.md`, add this crate section after `seishin_core`:

```markdown
### `seishin_world`

Owns backend-free data-driven world concepts:

- entity records and deterministic entity ID allocation;
- scene and prefab document types;
- pure scene/prefab merge behavior;
- loaded scene behavior that preserves explicit entity IDs;
- scene instancing behavior that creates fresh entity IDs and records source links.

`seishin_world` may depend on `seishin_core`, `serde`, and `toml` while TOML remains the project format. It must not depend on `winit`, `wgpu`, `kira`, `image`, `web-sys`, `wasm-bindgen`, renderer internals, audio internals, or runtime crates.
```

In the dependency direction diagram, include:

```text
seishin
  -> seishin_world
  -> seishin_core
```

- [ ] **Step 3: Update roadmap**

In `docs/roadmap.md`, add this checked item under `MVP Completed` only after the implementation is merged:

```markdown
- [x] Backend-free data-driven world crate for scene records, ID-preserving loads, and instancing.
```

If implementation is not yet merged, add this unchecked item under `Near Term`:

```markdown
- [ ] Extract backend-free data-driven world storage and scene/prefab merge logic.
```

- [ ] **Step 4: Commit docs**

```sh
git add docs/architecture.md docs/roadmap.md
git commit -m "docs: document data-driven world boundary"
```

---

### Task 8: Full Verification

**Files:**
- No planned edits
- Test: workspace validation

- [ ] **Step 1: Format**

Run:

```sh
cargo fmt --all -- --check
```

Expected: PASS. If it fails with formatting diffs, run `cargo fmt --all`, inspect `git diff`, then rerun the check.

- [ ] **Step 2: Clippy**

Run:

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: PASS with no warnings.

- [ ] **Step 3: Tests**

Run:

```sh
cargo test --workspace --all-targets
```

Expected: PASS across all crates and examples.

- [ ] **Step 4: Build**

Run:

```sh
cargo build --workspace
```

Expected: PASS.

- [ ] **Step 5: Web compile smoke**

Run:

```sh
cargo build --target wasm32-unknown-unknown -p seishin_basic_2d
```

Expected: PASS if the wasm target is installed. If the target is missing, run `rustup target add wasm32-unknown-unknown` only after user approval because it may need network access.

- [ ] **Step 6: Xtask gate**

Run:

```sh
cargo run -p xtask -- check
```

Expected: PASS. If `xtask` does not include `seishin_world`, update `tools/xtask/src/main.rs` in a separate small commit so the new crate participates in the standard gate.

- [ ] **Step 7: Final commit if verification caused fixes**

If verification required formatting or `xtask` edits:

```sh
git add .
git commit -m "chore: verify data-driven world extraction"
```

If verification produced no edits, do not create an empty commit.

---

## Self-Review Checklist

- Spec coverage:
  - `seishin_world` crate extraction: Tasks 1-4.
  - Backend-free dependency boundary: Tasks 1, 7.
  - Scene load preserves IDs: Task 4.
  - Scene instancing creates fresh IDs and source links: Task 4.
  - Facade compatibility: Tasks 5-6.
  - Dependency audit and light path documentation: Task 7.
  - Verification: Task 8.
- No new third-party dependency is introduced.
- `EntityId` and `Transform2D` stay in `seishin_core`.
- Opaque custom component config remains `toml::Value` and is isolated to world document/record types.
- Render graph, UI, hot reload, Android, iOS, and procedural generation implementation remain outside this plan.
