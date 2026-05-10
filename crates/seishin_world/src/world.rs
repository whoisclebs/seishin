use std::{
    any::TypeId,
    collections::{hash_map::Entry, HashMap},
};

use seishin_core::{EntityId, Transform2D};

use crate::{
    record::{
        AudioRef, ColliderRef, CustomComponentRef, EntityRecord, InstanceSource, SpriteRef, UiRef,
    },
    resolve::ResolvedEntity,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldError {
    DuplicateEntityId(EntityId),
    EntityIdOverflow(EntityId),
    MissingEntity(EntityId),
}

impl std::fmt::Display for WorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateEntityId(entity) => {
                write!(f, "duplicate entity id {}", entity.raw())
            }
            Self::EntityIdOverflow(entity) => {
                write!(f, "entity id {} cannot advance allocator", entity.raw())
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SceneInstance {
    source_to_instance: HashMap<EntityId, EntityId>,
}

impl SceneInstance {
    pub fn entity_for_source(&self, source: EntityId) -> Option<EntityId> {
        self.source_to_instance.get(&source).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedScene {
    source: String,
    entities: Vec<EntityId>,
}

impl LoadedScene {
    pub fn new(source: impl Into<String>, mut entities: Vec<EntityId>) -> Self {
        entities.sort();
        Self {
            source: source.into(),
            entities,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn entities(&self) -> &[EntityId] {
        &self.entities
    }
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
        self.try_spawn(record)
            .expect("entity id allocator exhausted")
    }

    pub fn try_spawn(&mut self, record: EntityRecord) -> Result<EntityId, WorldError> {
        let entity = self.try_allocate_entity()?;
        self.entities.insert(entity, record);
        Ok(entity)
    }

    pub fn load_resolved(
        &mut self,
        entities: impl IntoIterator<Item = ResolvedEntity>,
    ) -> Result<Vec<EntityId>, WorldError> {
        let entities = entities.into_iter().collect::<Vec<_>>();
        let mut staged = self.clone();
        let mut loaded = Vec::new();

        for resolved in entities {
            let entity = match resolved.id {
                Some(entity) => {
                    staged.insert(entity, resolved.record)?;
                    entity
                }
                None => staged.try_spawn(resolved.record)?,
            };
            loaded.push(entity);
        }

        *self = staged;
        Ok(loaded)
    }

    pub fn load_scene_resolved(
        &mut self,
        source: impl Into<String>,
        entities: impl IntoIterator<Item = ResolvedEntity>,
    ) -> Result<LoadedScene, WorldError> {
        let loaded = self.load_resolved(entities)?;
        Ok(LoadedScene::new(source, loaded))
    }

    pub fn replace_scene_resolved(
        &mut self,
        scene: &LoadedScene,
        entities: impl IntoIterator<Item = ResolvedEntity>,
    ) -> Result<LoadedScene, WorldError> {
        let mut staged = self.clone();
        staged.remove_scene_entities(scene)?;
        let loaded = staged.load_resolved(entities)?;

        *self = staged;
        Ok(LoadedScene::new(scene.source.clone(), loaded))
    }

    pub fn unload_scene(&mut self, scene: &LoadedScene) -> Result<Vec<EntityId>, WorldError> {
        let mut staged = self.clone();
        staged.remove_scene_entities(scene)?;
        let removed = scene.entities.clone();

        *self = staged;
        Ok(removed)
    }

    pub fn instantiate_resolved(
        &mut self,
        scene: impl Into<String>,
        entities: impl IntoIterator<Item = ResolvedEntity>,
    ) -> Result<SceneInstance, WorldError> {
        let scene = scene.into();
        let mut instance = SceneInstance::default();
        let entities = entities.into_iter().collect::<Vec<_>>();
        let source_entities = Self::instance_source_entities(&entities)?;
        let mut staged = self.clone();

        for (resolved, source_entity) in entities.into_iter().zip(source_entities.iter().copied()) {
            let mut record = resolved.record;
            record.instance_source = Some(InstanceSource {
                scene: scene.clone(),
                source_entity,
            });
            let entity = staged.try_spawn_skipping_sources(record, &source_entities)?;
            instance.source_to_instance.insert(source_entity, entity);
        }

        *self = staged;
        Ok(instance)
    }

    pub fn insert(&mut self, entity: EntityId, record: EntityRecord) -> Result<(), WorldError> {
        if entity.raw() == u64::MAX {
            return Err(WorldError::EntityIdOverflow(entity));
        }

        match self.entities.entry(entity) {
            Entry::Occupied(_) => Err(WorldError::DuplicateEntityId(entity)),
            Entry::Vacant(entry) => {
                if entity.raw() >= self.next_entity {
                    self.next_entity = Self::next_allocatable_after(entity.raw());
                }
                entry.insert(record);
                Ok(())
            }
        }
    }

    pub fn entity(&self, entity: EntityId) -> Option<&EntityRecord> {
        self.entities.get(&entity)
    }

    pub fn entity_mut(&mut self, entity: EntityId) -> Option<&mut EntityRecord> {
        self.entities.get_mut(&entity)
    }

    pub fn entities(&self) -> impl Iterator<Item = (EntityId, &EntityRecord)> + '_ {
        let mut entities = self
            .entities
            .iter()
            .map(|(id, record)| (*id, record))
            .collect::<Vec<_>>();
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

    pub fn entities_named(&self, name: &str) -> Vec<EntityId> {
        self.entities_matching(|record| record.name.as_deref() == Some(name))
    }

    pub fn entities_with_tag(&self, tag: &str) -> Vec<EntityId> {
        let mut entities = self
            .entities
            .iter()
            .filter_map(|(entity, record)| {
                record
                    .tags
                    .iter()
                    .any(|value| value == tag)
                    .then_some(*entity)
            })
            .collect::<Vec<_>>();
        entities.sort();
        entities
    }

    pub fn first_with_tag(&self, tag: &str) -> Option<EntityId> {
        self.entities_with_tag(tag).into_iter().next()
    }

    pub fn tags(&self, entity: EntityId) -> Option<&[String]> {
        self.entities
            .get(&entity)
            .map(|record| record.tags.as_slice())
    }

    pub fn transform(&self, entity: EntityId) -> Option<Transform2D> {
        self.entities.get(&entity).map(|record| record.transform)
    }

    pub fn name(&self, entity: EntityId) -> Option<&str> {
        self.entities
            .get(&entity)
            .and_then(|record| record.name.as_deref())
    }

    pub fn data_ref(&self, entity: EntityId, key: &str) -> Option<&str> {
        self.entities
            .get(&entity)
            .and_then(|record| record.data_refs.get(key))
            .map(String::as_str)
    }

    pub fn entities_with_data_ref(&self, key: &str, value: &str) -> Vec<EntityId> {
        self.entities_matching(|record| {
            record
                .data_refs
                .get(key)
                .is_some_and(|current| current == value)
        })
    }

    pub fn sprite(&self, entity: EntityId) -> Option<&SpriteRef> {
        self.entities
            .get(&entity)
            .and_then(|record| record.sprite.as_ref())
    }

    pub fn collider(&self, entity: EntityId) -> Option<&ColliderRef> {
        self.entities
            .get(&entity)
            .and_then(|record| record.collider.as_ref())
    }

    pub fn entities_with_sprite_texture(&self, texture: &str) -> Vec<EntityId> {
        self.entities_matching(|record| {
            record
                .sprite
                .as_ref()
                .is_some_and(|sprite| sprite.texture == texture)
        })
    }

    pub fn audio(&self, entity: EntityId) -> Option<&AudioRef> {
        self.entities
            .get(&entity)
            .and_then(|record| record.audio.as_ref())
    }

    pub fn ui(&self, entity: EntityId) -> Option<&UiRef> {
        self.entities
            .get(&entity)
            .and_then(|record| record.ui.as_ref())
    }

    pub fn custom_components(&self, entity: EntityId) -> Option<&[CustomComponentRef]> {
        self.entities
            .get(&entity)
            .map(|record| record.custom_components.as_slice())
    }

    pub fn entities_with_sprite(&self) -> Vec<EntityId> {
        self.entities_matching(|record| record.sprite.is_some())
    }

    pub fn entities_with_audio(&self) -> Vec<EntityId> {
        self.entities_matching(|record| record.audio.is_some())
    }

    pub fn entities_with_ui(&self) -> Vec<EntityId> {
        self.entities_matching(|record| record.ui.is_some())
    }

    pub fn entities_with_component_type_name(&self, type_name: &str) -> Vec<EntityId> {
        self.entities_matching(|record| {
            record
                .custom_components
                .iter()
                .any(|component| component.type_name == type_name)
        })
    }

    pub fn entities_with_component_type_id(&self, type_id: TypeId) -> Vec<EntityId> {
        self.entities_matching(|record| {
            record
                .custom_components
                .iter()
                .any(|component| component.type_id == Some(type_id))
        })
    }

    pub fn has_component_type_id(&self, entity: EntityId, type_id: TypeId) -> bool {
        self.entities.get(&entity).is_some_and(|record| {
            record
                .custom_components
                .iter()
                .any(|component| component.type_id == Some(type_id))
        })
    }

    fn entities_matching(&self, matches: impl Fn(&EntityRecord) -> bool) -> Vec<EntityId> {
        let mut entities = self
            .entities
            .iter()
            .filter_map(|(entity, record)| matches(record).then_some(*entity))
            .collect::<Vec<_>>();
        entities.sort();
        entities
    }

    pub fn translate(
        &mut self,
        entity: EntityId,
        delta_x: f32,
        delta_y: f32,
    ) -> Result<(), WorldError> {
        let record = self.require_entity_mut(entity)?;

        record.transform = record.transform.translated(delta_x, delta_y);
        Ok(())
    }

    pub fn set_position(&mut self, entity: EntityId, x: f32, y: f32) -> Result<(), WorldError> {
        let record = self.require_entity_mut(entity)?;

        record.transform.x = x;
        record.transform.y = y;
        Ok(())
    }

    pub fn set_transform(
        &mut self,
        entity: EntityId,
        transform: Transform2D,
    ) -> Result<(), WorldError> {
        self.require_entity_mut(entity)?.transform = transform;
        Ok(())
    }

    pub fn set_sprite(&mut self, entity: EntityId, sprite: SpriteRef) -> Result<(), WorldError> {
        self.require_entity_mut(entity)?.sprite = Some(sprite);
        Ok(())
    }

    pub fn set_collider(
        &mut self,
        entity: EntityId,
        collider: ColliderRef,
    ) -> Result<(), WorldError> {
        self.require_entity_mut(entity)?.collider = Some(collider);
        Ok(())
    }

    pub fn set_audio(&mut self, entity: EntityId, audio: AudioRef) -> Result<(), WorldError> {
        self.require_entity_mut(entity)?.audio = Some(audio);
        Ok(())
    }

    pub fn set_ui(&mut self, entity: EntityId, ui: UiRef) -> Result<(), WorldError> {
        self.require_entity_mut(entity)?.ui = Some(ui);
        Ok(())
    }

    pub fn set_data_ref(
        &mut self,
        entity: EntityId,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), WorldError> {
        self.require_entity_mut(entity)?
            .data_refs
            .insert(key.into(), value.into());
        Ok(())
    }

    pub fn remove_data_ref(
        &mut self,
        entity: EntityId,
        key: &str,
    ) -> Result<Option<String>, WorldError> {
        Ok(self.require_entity_mut(entity)?.data_refs.remove(key))
    }

    fn require_entity_mut(&mut self, entity: EntityId) -> Result<&mut EntityRecord, WorldError> {
        self.entities
            .get_mut(&entity)
            .ok_or(WorldError::MissingEntity(entity))
    }

    fn remove_scene_entities(&mut self, scene: &LoadedScene) -> Result<(), WorldError> {
        for entity in &scene.entities {
            if !self.entities.contains_key(entity) {
                return Err(WorldError::MissingEntity(*entity));
            }
        }

        for entity in &scene.entities {
            self.entities.remove(entity);
        }

        Ok(())
    }

    fn instance_source_entities(entities: &[ResolvedEntity]) -> Result<Vec<EntityId>, WorldError> {
        let mut used_sources = Vec::new();

        for source_entity in entities.iter().filter_map(|resolved| resolved.id) {
            if used_sources.contains(&source_entity) {
                return Err(WorldError::DuplicateEntityId(source_entity));
            }
            used_sources.push(source_entity);
        }

        let mut source_entities = Vec::with_capacity(entities.len());
        let mut next_synthetic = 1;

        for resolved in entities {
            let source_entity = match resolved.id {
                Some(source_entity) => source_entity,
                None => {
                    let source_entity =
                        Self::next_unused_synthetic_source(&used_sources, &mut next_synthetic)?;
                    used_sources.push(source_entity);
                    source_entity
                }
            };
            source_entities.push(source_entity);
        }

        Ok(source_entities)
    }

    fn next_unused_synthetic_source(
        used_sources: &[EntityId],
        next_synthetic: &mut u64,
    ) -> Result<EntityId, WorldError> {
        while *next_synthetic < u64::MAX {
            let source_entity = EntityId::new(*next_synthetic);
            *next_synthetic += 1;
            if !used_sources.contains(&source_entity) {
                return Ok(source_entity);
            }
        }

        Err(WorldError::EntityIdOverflow(EntityId::new(u64::MAX)))
    }

    fn try_allocate_entity(&mut self) -> Result<EntityId, WorldError> {
        let start = Self::normalize_allocatable(self.next_entity);
        let mut candidate = start;

        loop {
            let entity = EntityId::new(candidate);
            if !self.entities.contains_key(&entity) {
                self.next_entity = Self::next_allocatable_after(candidate);
                return Ok(entity);
            }

            candidate = Self::next_allocatable_after(candidate);
            if candidate == start {
                return Err(WorldError::EntityIdOverflow(EntityId::new(u64::MAX)));
            }
        }
    }

    fn try_spawn_skipping_sources(
        &mut self,
        record: EntityRecord,
        source_entities: &[EntityId],
    ) -> Result<EntityId, WorldError> {
        let entity = self.try_allocate_entity_skipping_sources(source_entities)?;
        self.entities.insert(entity, record);
        Ok(entity)
    }

    fn try_allocate_entity_skipping_sources(
        &mut self,
        source_entities: &[EntityId],
    ) -> Result<EntityId, WorldError> {
        loop {
            let entity = self.try_allocate_entity()?;
            if !source_entities.contains(&entity) {
                return Ok(entity);
            }
        }
    }

    fn normalize_allocatable(raw: u64) -> u64 {
        if raw == 0 || raw == u64::MAX {
            1
        } else {
            raw
        }
    }

    fn next_allocatable_after(raw: u64) -> u64 {
        if raw >= u64::MAX - 1 {
            1
        } else {
            raw + 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{InstanceSource, SpriteRef};

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

        world
            .insert(entity, EntityRecord::default())
            .expect("first insert");
        let error = world
            .insert(entity, EntityRecord::default())
            .expect_err("duplicate id must fail");

        assert_eq!(error, WorldError::DuplicateEntityId(entity));
    }

    #[test]
    fn max_explicit_id_is_rejected_to_preserve_allocator() {
        let mut world = World::default();
        let entity = EntityId::new(u64::MAX);

        let error = world
            .insert(entity, EntityRecord::default())
            .expect_err("max entity id cannot advance allocator");
        let next = world.spawn(EntityRecord::named("First"));

        assert_eq!(error, WorldError::EntityIdOverflow(entity));
        assert_eq!(world.entity(entity), None);
        assert_eq!(next.raw(), 1);
        assert_eq!(world.name(next), Some("First"));
    }

    #[test]
    fn allocator_wraps_before_reserved_max_entity_id() {
        let mut world = World::default();
        let high = EntityId::new(u64::MAX - 1);

        world
            .insert(high, EntityRecord::named("High"))
            .expect("high insert");
        let wrapped = world.spawn(EntityRecord::named("Wrapped"));

        assert_eq!(world.name(high), Some("High"));
        assert_eq!(wrapped, EntityId::new(1));
        assert_eq!(world.name(wrapped), Some("Wrapped"));
    }

    #[test]
    fn entities_iterate_in_entity_id_order() {
        let mut world = World::default();
        let high = EntityId::new(9);
        let low = EntityId::new(2);
        let middle = EntityId::new(5);

        world
            .insert(high, EntityRecord::default())
            .expect("high insert");
        world
            .insert(low, EntityRecord::default())
            .expect("low insert");
        world
            .insert(middle, EntityRecord::default())
            .expect("middle insert");

        let entities = world
            .entities()
            .map(|(entity, _)| entity)
            .collect::<Vec<_>>();

        assert_eq!(entities, vec![low, middle, high]);
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
    fn world_queries_and_mutates_common_record_fields() {
        let mut world = World::default();
        let high = EntityId::new(9);
        let low = EntityId::new(2);

        world
            .insert(
                high,
                EntityRecord::named("Duplicate").with_sprite(SpriteRef {
                    texture: "asset://sprites/high.png".to_string(),
                    width: None,
                    height: None,
                    source_x: None,
                    source_y: None,
                    source_width: None,
                    source_height: None,
                    layer: 0,
                    sort_order: 0,
                    tint: None,
                }),
            )
            .expect("insert high");
        world
            .insert(low, EntityRecord::named("Duplicate"))
            .expect("insert low");

        world
            .set_data_ref(low, "role", "merchant")
            .expect("set data ref");
        world
            .set_transform(high, Transform2D::from_translation(12.0, 34.0))
            .expect("set transform");
        world
            .set_audio(
                low,
                AudioRef {
                    sound: "asset://audio/beep.wav".to_string(),
                },
            )
            .expect("set audio");
        world
            .set_ui(low, UiRef::text("Talk", crate::record::UiAnchor::Bottom))
            .expect("set ui");
        world
            .set_sprite(
                low,
                SpriteRef {
                    texture: "asset://sprites/low.png".to_string(),
                    width: Some(16.0),
                    height: Some(16.0),
                    source_x: None,
                    source_y: None,
                    source_width: None,
                    source_height: None,
                    layer: 1,
                    sort_order: 2,
                    tint: None,
                },
            )
            .expect("set sprite");

        assert_eq!(world.entities_named("Duplicate"), vec![low, high]);
        assert_eq!(world.entities_with_data_ref("role", "merchant"), vec![low]);
        assert_eq!(
            world.entities_with_sprite_texture("asset://sprites/high.png"),
            vec![high]
        );
        assert_eq!(
            world.entities_with_sprite_texture("asset://sprites/low.png"),
            vec![low]
        );
        assert_eq!(
            world.transform(high),
            Some(Transform2D::from_translation(12.0, 34.0))
        );
        assert_eq!(
            world.audio(low).map(|audio| audio.sound.as_str()),
            Some("asset://audio/beep.wav")
        );
        assert_eq!(
            world
                .ui(low)
                .and_then(|ui| ui.text.as_ref())
                .map(|text| text.value.as_str()),
            Some("Talk")
        );

        assert_eq!(
            world.set_data_ref(EntityId::new(404), "missing", "value"),
            Err(WorldError::MissingEntity(EntityId::new(404)))
        );
    }

    #[test]
    fn transform_mutation_keeps_render_data_separate() {
        let mut world = World::default();
        let entity = world.spawn(EntityRecord::default().with_sprite(SpriteRef {
            texture: "asset://sprites/player.png".to_string(),
            width: Some(96.0),
            height: Some(96.0),
            source_x: None,
            source_y: None,
            source_width: None,
            source_height: None,
            layer: 0,
            sort_order: 0,
            tint: None,
        }));

        world.translate(entity, 3.0, 4.0).expect("translate");
        world
            .set_position(entity, 10.0, 20.0)
            .expect("set position");

        assert_eq!(
            world.transform(entity),
            Some(Transform2D::from_translation(10.0, 20.0))
        );
        assert_eq!(
            world.sprite(entity).map(|sprite| sprite.texture.as_str()),
            Some("asset://sprites/player.png")
        );
    }

    #[test]
    fn component_type_id_query_uses_stored_custom_component_metadata() {
        use std::any::TypeId;

        struct PlayerController;
        struct OtherController;

        let player_type = TypeId::of::<PlayerController>();
        let other_type = TypeId::of::<OtherController>();
        let mut world = World::default();
        let entity = world.spawn(EntityRecord {
            custom_components: vec![crate::record::CustomComponentRef {
                type_name: "PlayerController".to_string(),
                type_id: Some(player_type),
                config: toml::Value::Table(Default::default()),
            }],
            ..EntityRecord::default()
        });

        assert!(world.has_component_type_id(entity, player_type));
        assert!(!world.has_component_type_id(entity, other_type));
        assert!(!world.has_component_type_id(EntityId::new(99), player_type));
    }

    #[test]
    fn transform_mutation_returns_missing_entity_error() {
        let mut world = World::default();
        let missing = EntityId::new(99);

        assert_eq!(
            world.translate(missing, 3.0, 4.0),
            Err(WorldError::MissingEntity(missing))
        );
        assert_eq!(
            world.set_position(missing, 10.0, 20.0),
            Err(WorldError::MissingEntity(missing))
        );
    }

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
    fn loaded_scene_handles_replace_and_unload_only_tracked_entities() {
        let mut world = World::default();
        let external = world.spawn(EntityRecord::named("External"));

        let loaded = world
            .load_scene_resolved(
                "res://scenes/room.scene.toml",
                [
                    crate::resolve::ResolvedEntity {
                        id: Some(EntityId::new(10)),
                        prefab: None,
                        record: EntityRecord::named("OldExplicit"),
                    },
                    crate::resolve::ResolvedEntity {
                        id: None,
                        prefab: None,
                        record: EntityRecord::named("OldGenerated"),
                    },
                ],
            )
            .expect("load scene");

        assert_eq!(loaded.source(), "res://scenes/room.scene.toml");
        assert_eq!(loaded.entities(), &[EntityId::new(10), EntityId::new(11)]);
        assert_eq!(world.entity_by_name("External"), Some(external));

        let replaced = world
            .replace_scene_resolved(
                &loaded,
                [crate::resolve::ResolvedEntity {
                    id: Some(EntityId::new(20)),
                    prefab: None,
                    record: EntityRecord::named("NewExplicit"),
                }],
            )
            .expect("replace scene");

        assert_eq!(replaced.source(), loaded.source());
        assert_eq!(replaced.entities(), &[EntityId::new(20)]);
        assert_eq!(world.entity_by_name("OldExplicit"), None);
        assert_eq!(world.entity_by_name("OldGenerated"), None);
        assert_eq!(world.entity_by_name("NewExplicit"), Some(EntityId::new(20)));
        assert_eq!(world.entity_by_name("External"), Some(external));

        let removed = world.unload_scene(&replaced).expect("unload scene");

        assert_eq!(removed, vec![EntityId::new(20)]);
        assert_eq!(world.entity_by_name("NewExplicit"), None);
        assert_eq!(world.entity_by_name("External"), Some(external));
    }

    #[test]
    fn load_resolved_duplicate_explicit_ids_return_error() {
        let mut world = World::default();
        let existing = EntityId::new(99);
        let duplicate = EntityId::new(7);

        world
            .insert(existing, EntityRecord::named("Existing"))
            .expect("insert existing");
        let error = world
            .load_resolved([
                crate::resolve::ResolvedEntity {
                    id: Some(duplicate),
                    prefab: None,
                    record: EntityRecord::named("First"),
                },
                crate::resolve::ResolvedEntity {
                    id: Some(duplicate),
                    prefab: None,
                    record: EntityRecord::named("Second"),
                },
            ])
            .expect_err("duplicate explicit id must fail");

        assert_eq!(error, WorldError::DuplicateEntityId(duplicate));
        assert_eq!(world.entities().count(), 1);
        assert_eq!(world.entity_by_name("Existing"), Some(existing));
        assert_eq!(world.entity_by_name("First"), None);
        assert_eq!(world.entity_by_name("Second"), None);
    }

    #[test]
    fn load_resolved_overflow_error_does_not_commit_earlier_records() {
        let mut world = World::default();
        let too_high = EntityId::new(u64::MAX);

        let error = world
            .load_resolved([
                crate::resolve::ResolvedEntity {
                    id: None,
                    prefab: None,
                    record: EntityRecord::named("GeneratedBeforeFailure"),
                },
                crate::resolve::ResolvedEntity {
                    id: Some(too_high),
                    prefab: None,
                    record: EntityRecord::named("TooHigh"),
                },
            ])
            .expect_err("overflow explicit id must fail");

        assert_eq!(error, WorldError::EntityIdOverflow(too_high));
        assert_eq!(world.entities().count(), 0);
        assert_eq!(world.entity_by_name("GeneratedBeforeFailure"), None);
        let next = world.spawn(EntityRecord::named("AfterFailure"));
        assert_eq!(next, EntityId::new(1));
    }

    #[test]
    fn load_resolved_max_explicit_id_returns_overflow_error() {
        let mut world = World::default();
        let entity = EntityId::new(u64::MAX);

        let error = world
            .load_resolved([crate::resolve::ResolvedEntity {
                id: Some(entity),
                prefab: None,
                record: EntityRecord::named("TooHigh"),
            }])
            .expect_err("max entity id cannot advance allocator");

        assert_eq!(error, WorldError::EntityIdOverflow(entity));
        assert_eq!(world.entities().count(), 0);
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
        let spawned = instance
            .entity_for_source(EntityId::new(1))
            .expect("source map");

        assert_ne!(spawned, EntityId::new(1));
        assert_eq!(spawned.raw(), 2);
        assert_eq!(
            world
                .entity(spawned)
                .and_then(|record| record.instance_source.as_ref()),
            Some(&InstanceSource {
                scene: "res://scenes/room.scene.toml".to_string(),
                source_entity: EntityId::new(1),
            })
        );
    }

    #[test]
    fn instantiate_resolved_entities_never_reuses_source_id_in_empty_world() {
        let mut world = World::default();

        let instance = world
            .instantiate_resolved(
                "res://scenes/room.scene.toml",
                [crate::resolve::ResolvedEntity {
                    id: Some(EntityId::new(1)),
                    prefab: None,
                    record: EntityRecord::named("RoomPlayer"),
                }],
            )
            .expect("instantiate");
        let spawned = instance
            .entity_for_source(EntityId::new(1))
            .expect("source map");

        assert_ne!(spawned, EntityId::new(1));
        assert_eq!(spawned.raw(), 2);
        assert_eq!(world.entity_by_name("RoomPlayer"), Some(spawned));
    }

    #[test]
    fn instantiate_resolved_idless_entities_have_deterministic_source_links() {
        let mut world = World::default();

        let instance = world
            .instantiate_resolved(
                "res://scenes/room.scene.toml",
                [
                    crate::resolve::ResolvedEntity {
                        id: None,
                        prefab: None,
                        record: EntityRecord::named("GeneratedOne"),
                    },
                    crate::resolve::ResolvedEntity {
                        id: None,
                        prefab: None,
                        record: EntityRecord::named("GeneratedTwo"),
                    },
                ],
            )
            .expect("instantiate");
        let first = world.entity_by_name("GeneratedOne").expect("first entity");
        let second = world.entity_by_name("GeneratedTwo").expect("second entity");
        let first_source = EntityId::new(1);
        let second_source = EntityId::new(2);

        assert_eq!(first.raw(), 3);
        assert_eq!(second.raw(), 4);
        assert_eq!(
            world
                .entity(first)
                .and_then(|record| record.instance_source.as_ref()),
            Some(&InstanceSource {
                scene: "res://scenes/room.scene.toml".to_string(),
                source_entity: first_source,
            })
        );
        assert_eq!(
            world
                .entity(second)
                .and_then(|record| record.instance_source.as_ref()),
            Some(&InstanceSource {
                scene: "res://scenes/room.scene.toml".to_string(),
                source_entity: second_source,
            })
        );
        assert_eq!(instance.entity_for_source(first_source), Some(first));
        assert_eq!(instance.entity_for_source(second_source), Some(second));
    }

    #[test]
    fn instantiate_resolved_duplicate_source_ids_error_without_mutating_world() {
        let mut world = World::default();
        let existing = EntityId::new(9);
        let duplicate = EntityId::new(1);

        world
            .insert(existing, EntityRecord::named("Existing"))
            .expect("insert existing");
        let error = world
            .instantiate_resolved(
                "res://scenes/room.scene.toml",
                [
                    crate::resolve::ResolvedEntity {
                        id: Some(duplicate),
                        prefab: None,
                        record: EntityRecord::named("First"),
                    },
                    crate::resolve::ResolvedEntity {
                        id: Some(duplicate),
                        prefab: None,
                        record: EntityRecord::named("Second"),
                    },
                ],
            )
            .expect_err("duplicate source id must fail");

        assert_eq!(error, WorldError::DuplicateEntityId(duplicate));
        assert_eq!(world.entities().count(), 1);
        assert_eq!(world.entity_by_name("Existing"), Some(existing));
        assert_eq!(world.entity_by_name("First"), None);
        assert_eq!(world.entity_by_name("Second"), None);
    }
}
