use std::collections::BTreeMap;

use seishin_core::{EntityId, Transform2D};

use crate::{
    document::{
        CustomComponentDocument, SceneAudioDocument, SceneDocument, SceneEntityDocument,
        SceneSpriteDocument, SceneTransformDocument, SceneUiDocument,
    },
    procedural::{ProceduralSceneBuilder, ProceduralSeed},
    record::{AudioRef, SpriteRef, UiRef},
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneDocumentBuilder {
    entities: Vec<SceneEntityDocument>,
}

impl SceneDocumentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn procedural(seed: ProceduralSeed) -> ProceduralSceneBuilder {
        ProceduralSceneBuilder::new(seed)
    }

    pub fn entity(mut self, entity: SceneEntityBuilder) -> Self {
        self.entities.push(entity.build());
        self
    }

    pub fn push_entity(mut self, entity: SceneEntityDocument) -> Self {
        self.entities.push(entity);
        self
    }

    pub fn build(self) -> SceneDocument {
        SceneDocument {
            maps: Vec::new(),
            entities: self.entities,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneEntityBuilder {
    entity: SceneEntityDocument,
}

impl SceneEntityBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, entity: EntityId) -> Self {
        self.entity.id = Some(entity.raw());
        self
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.entity.name = Some(name.into());
        self
    }

    pub fn prefab(mut self, prefab: impl Into<String>) -> Self {
        self.entity.prefab = Some(prefab.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        let mut tags = self.entity.tags.take().unwrap_or_default();
        tags.values.push(tag.into());
        self.entity.tags = Some(tags);
        self
    }

    pub fn data_ref(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut data = self.entity.data.take().unwrap_or_default();
        data.insert(key.into(), value.into());
        self.entity.data = Some(data);
        self
    }

    pub fn transform(mut self, transform: Transform2D) -> Self {
        self.entity.transform = Some(SceneTransformDocument {
            x: Some(transform.x),
            y: Some(transform.y),
            rotation_radians: Some(transform.rotation_radians),
            scale_x: Some(transform.scale_x),
            scale_y: Some(transform.scale_y),
        });
        self
    }

    pub fn sprite(mut self, sprite: SpriteRef) -> Self {
        self.entity.sprite = Some(SceneSpriteDocument {
            texture: Some(sprite.texture),
            width: sprite.width,
            height: sprite.height,
            layer: (sprite.layer != 0).then_some(sprite.layer),
            sort_order: (sprite.sort_order != 0).then_some(sprite.sort_order),
            tint: sprite.tint,
        });
        self
    }

    pub fn audio(mut self, audio: AudioRef) -> Self {
        self.entity.audio = Some(SceneAudioDocument {
            sound: Some(audio.sound),
        });
        self
    }

    pub fn ui(mut self, ui: UiRef) -> Self {
        self.entity.ui = Some(SceneUiDocument::from(ui));
        self
    }

    pub fn component(mut self, component: CustomComponentDocument) -> Self {
        self.entity.components.push(component);
        self
    }

    pub fn build(mut self) -> SceneEntityDocument {
        if self
            .entity
            .tags
            .as_ref()
            .is_some_and(|tags| tags.values.is_empty())
        {
            self.entity.tags = None;
        }
        if self.entity.data.as_ref().is_some_and(BTreeMap::is_empty) {
            self.entity.data = None;
        }
        self.entity
    }
}
