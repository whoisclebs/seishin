use std::collections::BTreeMap;

use seishin_core::EntityId;

use crate::{
    document::{
        CustomComponentDocument, SceneAudioDocument, SceneDocument, SceneEntityDocument,
        SceneInstanceDocument, SceneSpriteDocument, SceneTransformDocument, SceneUiDocument,
        TagsDocument,
    },
    record::{CustomComponentRef, EntityRecord},
    resolve::ResolvedEntity,
    world::World,
};

#[derive(Debug, Clone, PartialEq)]
pub struct SceneDocumentExport {
    document: SceneDocument,
    omissions: Vec<SceneExportOmission>,
}

impl SceneDocumentExport {
    pub fn document(&self) -> &SceneDocument {
        &self.document
    }

    pub fn into_document(self) -> SceneDocument {
        self.document
    }

    pub fn omissions(&self) -> &[SceneExportOmission] {
        &self.omissions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneExportOmission {
    /// Scene component documents can represent table-shaped TOML config only.
    CustomComponentConfigNotRepresented {
        entity: Option<EntityId>,
        type_name: String,
    },
}

impl World {
    pub fn to_scene_document(&self) -> SceneDocument {
        self.to_scene_document_export().into_document()
    }

    pub fn to_scene_document_export(&self) -> SceneDocumentExport {
        scene_document_from_records(self.entities())
    }
}

pub fn scene_document_from_records<'a>(
    records: impl IntoIterator<Item = (EntityId, &'a EntityRecord)>,
) -> SceneDocumentExport {
    let mut records = records.into_iter().collect::<Vec<_>>();
    records.sort_by_key(|(entity, _)| *entity);

    let mut omissions = Vec::new();
    let entities = records
        .into_iter()
        .map(|(entity, record)| {
            scene_entity_document_from_record(Some(entity), record, None, &mut omissions)
        })
        .collect();

    SceneDocumentExport {
        document: SceneDocument { entities },
        omissions,
    }
}

pub fn scene_document_from_resolved_entities(
    entities: impl IntoIterator<Item = ResolvedEntity>,
) -> SceneDocument {
    scene_document_export_from_resolved_entities(entities).into_document()
}

pub fn scene_document_export_from_resolved_entities(
    entities: impl IntoIterator<Item = ResolvedEntity>,
) -> SceneDocumentExport {
    let mut entities = entities.into_iter().collect::<Vec<_>>();
    entities.sort_by_key(|resolved| resolved.id);

    let mut omissions = Vec::new();
    let entities = entities
        .into_iter()
        .map(|resolved| {
            scene_entity_document_from_record(
                resolved.id,
                &resolved.record,
                resolved.prefab,
                &mut omissions,
            )
        })
        .collect();

    SceneDocumentExport {
        document: SceneDocument { entities },
        omissions,
    }
}

fn scene_entity_document_from_record(
    entity: Option<EntityId>,
    record: &EntityRecord,
    prefab: Option<String>,
    omissions: &mut Vec<SceneExportOmission>,
) -> SceneEntityDocument {
    SceneEntityDocument {
        id: entity.map(EntityId::raw),
        name: record.name.clone(),
        prefab,
        instance: record
            .instance_source
            .as_ref()
            .map(|source| SceneInstanceDocument {
                scene: source.scene.clone(),
                source_entity: source.source_entity.raw(),
            }),
        transform: Some(SceneTransformDocument {
            x: Some(record.transform.x),
            y: Some(record.transform.y),
            rotation_radians: Some(record.transform.rotation_radians),
            scale_x: Some(record.transform.scale_x),
            scale_y: Some(record.transform.scale_y),
        }),
        tags: (!record.tags.is_empty()).then_some(TagsDocument {
            values: record.tags.clone(),
        }),
        data: (!record.data_refs.is_empty()).then_some(
            record
                .data_refs
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ),
        sprite: record.sprite.as_ref().map(|sprite| SceneSpriteDocument {
            texture: Some(sprite.texture.clone()),
            width: sprite.width,
            height: sprite.height,
            layer: (sprite.layer != 0).then_some(sprite.layer),
            sort_order: (sprite.sort_order != 0).then_some(sprite.sort_order),
        }),
        audio: record.audio.as_ref().map(|audio| SceneAudioDocument {
            sound: Some(audio.sound.clone()),
        }),
        ui: record.ui.as_ref().map(SceneUiDocument::from),
        components: record
            .custom_components
            .iter()
            .map(|component| custom_component_document(entity, component, omissions))
            .collect(),
    }
}

fn custom_component_document(
    entity: Option<EntityId>,
    component: &CustomComponentRef,
    omissions: &mut Vec<SceneExportOmission>,
) -> CustomComponentDocument {
    let config = match component.config.as_table() {
        Some(table) => table
            .iter()
            .filter(|(key, _)| key.as_str() != "type")
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<BTreeMap<_, _>>(),
        None => {
            omissions.push(SceneExportOmission::CustomComponentConfigNotRepresented {
                entity,
                type_name: component.type_name.clone(),
            });
            BTreeMap::new()
        }
    };

    CustomComponentDocument {
        type_name: component.type_name.clone(),
        config,
    }
}
