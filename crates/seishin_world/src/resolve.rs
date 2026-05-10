use seishin_core::{EntityId, Transform2D};

use crate::{
    document::{
        CustomComponentDocument, PrefabDocument, SceneAudioDocument, SceneEntityDocument,
        SceneSpriteDocument, SceneTransformDocument, SceneUiDocument, SceneUiImageDocument,
        SceneUiInteractionDocument, SceneUiLayoutDocument, SceneUiTextDocument,
    },
    record::{
        AudioRef, CustomComponentRef, EntityRecord, SpriteRef, UiImageRef, UiInteractionRef,
        UiLayoutRef, UiRef, UiTextRef,
    },
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

#[derive(Debug, Clone, Default)]
struct EntityBlueprint {
    record: EntityRecord,
    sprite: Option<SceneSpriteDocument>,
    ui: Option<SceneUiDocument>,
}

impl EntityBlueprint {
    fn into_record(mut self) -> EntityRecord {
        self.record.sprite = finalize_sprite(self.sprite);
        self.record.ui = finalize_ui(self.ui);
        self.record
    }
}

pub fn resolve_scene_entity(
    entity: SceneEntityDocument,
    prefab: Option<PrefabDocument>,
) -> Result<ResolvedEntity, ResolveError> {
    let mut blueprint = prefab
        .map(prefab_to_blueprint)
        .transpose()?
        .unwrap_or_default();
    let id = entity.id.map(EntityId::new);
    let prefab = entity.prefab;

    if entity.name.is_some() {
        blueprint.record.name = entity.name;
    }

    if let Some(tags) = entity.tags {
        blueprint.record.tags = tags.values;
    }

    if let Some(data) = entity.data {
        blueprint.record.data_refs.extend(data);
    }

    if let Some(transform) = entity.transform {
        blueprint.record.transform = merge_transform(blueprint.record.transform, transform);
    }

    if let Some(sprite) = entity.sprite {
        blueprint.sprite = merge_sprite(blueprint.sprite.take(), sprite);
    }

    if let Some(audio) = entity.audio {
        blueprint.record.audio = merge_audio(blueprint.record.audio.take(), audio);
    }

    if let Some(ui) = entity.ui {
        blueprint.ui = merge_ui(blueprint.ui.take(), ui);
    }

    for component in entity.components {
        let component = custom_component_ref(component);
        blueprint
            .record
            .custom_components
            .retain(|existing| existing.type_name != component.type_name);
        blueprint.record.custom_components.push(component);
    }

    Ok(ResolvedEntity {
        id,
        prefab,
        record: blueprint.into_record(),
    })
}

fn prefab_to_blueprint(prefab: PrefabDocument) -> Result<EntityBlueprint, ResolveError> {
    let mut blueprint = EntityBlueprint::default();

    for (name, value) in prefab.components {
        match name.as_str() {
            "name" => {
                blueprint.record.name = value
                    .get("value")
                    .and_then(toml::Value::as_str)
                    .map(ToOwned::to_owned);
            }
            "tags" => {
                blueprint.record.tags = value
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
                blueprint.record.transform = merge_transform(blueprint.record.transform, transform);
            }
            "sprite" => {
                let sprite = value
                    .try_into()
                    .map_err(|_| ResolveError::InvalidPrefabComponent(name.clone()))?;
                blueprint.sprite = merge_sprite(blueprint.sprite.take(), sprite);
            }
            "audio" => {
                let audio = value
                    .try_into()
                    .map_err(|_| ResolveError::InvalidPrefabComponent(name.clone()))?;
                blueprint.record.audio = merge_audio(blueprint.record.audio.take(), audio);
            }
            "ui" => {
                let ui = value
                    .try_into()
                    .map_err(|_| ResolveError::InvalidPrefabComponent(name.clone()))?;
                blueprint.ui = merge_ui(blueprint.ui.take(), ui);
            }
            _ => {
                if let Some(type_name) = value.get("type").and_then(toml::Value::as_str) {
                    let config = value
                        .as_table()
                        .map(|table| table.clone().into_iter().collect())
                        .unwrap_or_default();
                    blueprint.record.custom_components.push(CustomComponentRef {
                        type_name: type_name.to_string(),
                        type_id: None,
                        config: toml::Value::Table(config),
                    });
                }
            }
        }
    }

    Ok(blueprint)
}

fn custom_component_ref(component: CustomComponentDocument) -> CustomComponentRef {
    CustomComponentRef {
        type_name: component.type_name,
        type_id: None,
        config: toml::Value::Table(component.config.into_iter().collect()),
    }
}

fn merge_sprite(
    base: Option<SceneSpriteDocument>,
    override_value: SceneSpriteDocument,
) -> Option<SceneSpriteDocument> {
    let texture = override_value
        .texture
        .or_else(|| base.as_ref().and_then(|sprite| sprite.texture.clone()));
    let width = override_value
        .width
        .or_else(|| base.as_ref().and_then(|sprite| sprite.width));
    let height = override_value
        .height
        .or_else(|| base.as_ref().and_then(|sprite| sprite.height));

    (texture.is_some() || width.is_some() || height.is_some()).then_some(SceneSpriteDocument {
        texture,
        width,
        height,
    })
}

fn finalize_sprite(sprite: Option<SceneSpriteDocument>) -> Option<SpriteRef> {
    let sprite = sprite?;
    let texture = sprite.texture?;

    Some(SpriteRef {
        texture,
        width: sprite.width,
        height: sprite.height,
    })
}

fn merge_audio(base: Option<AudioRef>, override_value: SceneAudioDocument) -> Option<AudioRef> {
    let sound = override_value
        .sound
        .or_else(|| base.as_ref().map(|audio| audio.sound.clone()))?;

    Some(AudioRef { sound })
}

fn merge_ui(
    base: Option<SceneUiDocument>,
    override_value: SceneUiDocument,
) -> Option<SceneUiDocument> {
    let layout = merge_ui_layout(
        base.as_ref().and_then(|ui| ui.layout),
        override_value.layout,
    );
    let text = merge_ui_text(
        base.as_ref().and_then(|ui| ui.text.clone()),
        override_value.text,
    );
    let image = merge_ui_image(
        base.as_ref().and_then(|ui| ui.image.clone()),
        override_value.image,
    );
    let interaction = merge_ui_interaction(
        base.as_ref().and_then(|ui| ui.interaction.clone()),
        override_value.interaction,
    );

    (layout.is_some() || text.is_some() || image.is_some() || interaction.is_some()).then_some(
        SceneUiDocument {
            layout,
            text,
            image,
            interaction,
        },
    )
}

fn merge_ui_layout(
    base: Option<SceneUiLayoutDocument>,
    override_value: Option<SceneUiLayoutDocument>,
) -> Option<SceneUiLayoutDocument> {
    let Some(override_value) = override_value else {
        return base;
    };
    let anchor = override_value
        .anchor
        .or_else(|| base.as_ref().and_then(|layout| layout.anchor));
    let offset_x = override_value
        .offset_x
        .or_else(|| base.as_ref().and_then(|layout| layout.offset_x));
    let offset_y = override_value
        .offset_y
        .or_else(|| base.as_ref().and_then(|layout| layout.offset_y));
    let width = override_value
        .width
        .or_else(|| base.as_ref().and_then(|layout| layout.width));
    let height = override_value
        .height
        .or_else(|| base.as_ref().and_then(|layout| layout.height));
    let z_index = override_value
        .z_index
        .or_else(|| base.as_ref().and_then(|layout| layout.z_index));

    (anchor.is_some()
        || offset_x.is_some()
        || offset_y.is_some()
        || width.is_some()
        || height.is_some()
        || z_index.is_some())
    .then_some(SceneUiLayoutDocument {
        anchor,
        offset_x,
        offset_y,
        width,
        height,
        z_index,
    })
}

fn merge_ui_text(
    base: Option<SceneUiTextDocument>,
    override_value: Option<SceneUiTextDocument>,
) -> Option<SceneUiTextDocument> {
    let Some(override_value) = override_value else {
        return base;
    };
    let value = override_value
        .value
        .or_else(|| base.as_ref().and_then(|text| text.value.clone()));
    let font_size = override_value
        .font_size
        .or_else(|| base.as_ref().and_then(|text| text.font_size));
    let color = override_value
        .color
        .or_else(|| base.as_ref().and_then(|text| text.color.clone()));

    (value.is_some() || font_size.is_some() || color.is_some()).then_some(SceneUiTextDocument {
        value,
        font_size,
        color,
    })
}

fn merge_ui_image(
    base: Option<SceneUiImageDocument>,
    override_value: Option<SceneUiImageDocument>,
) -> Option<SceneUiImageDocument> {
    let Some(override_value) = override_value else {
        return base;
    };
    let texture = override_value
        .texture
        .or_else(|| base.as_ref().and_then(|image| image.texture.clone()));
    let tint = override_value
        .tint
        .or_else(|| base.as_ref().and_then(|image| image.tint.clone()));

    (texture.is_some() || tint.is_some()).then_some(SceneUiImageDocument { texture, tint })
}

fn merge_ui_interaction(
    base: Option<SceneUiInteractionDocument>,
    override_value: Option<SceneUiInteractionDocument>,
) -> Option<SceneUiInteractionDocument> {
    let Some(override_value) = override_value else {
        return base;
    };
    let action = override_value.action.or_else(|| {
        base.as_ref()
            .and_then(|interaction| interaction.action.clone())
    });
    let enabled = override_value
        .enabled
        .or_else(|| base.as_ref().and_then(|interaction| interaction.enabled));

    (action.is_some() || enabled.is_some())
        .then_some(SceneUiInteractionDocument { action, enabled })
}

fn finalize_ui(ui: Option<SceneUiDocument>) -> Option<UiRef> {
    let ui = ui?;
    let has_layout = ui.layout.is_some();
    let layout = ui.layout.map(finalize_ui_layout).unwrap_or_default();
    let text = ui.text.and_then(finalize_ui_text);
    let image = ui.image.and_then(finalize_ui_image);
    let interaction = ui.interaction.and_then(finalize_ui_interaction);

    (has_layout || text.is_some() || image.is_some() || interaction.is_some()).then_some(UiRef {
        layout,
        text,
        image,
        interaction,
    })
}

fn finalize_ui_layout(layout: SceneUiLayoutDocument) -> UiLayoutRef {
    UiLayoutRef {
        anchor: layout.anchor.unwrap_or_default(),
        offset_x: layout.offset_x.unwrap_or_default(),
        offset_y: layout.offset_y.unwrap_or_default(),
        width: layout.width.unwrap_or_default(),
        height: layout.height.unwrap_or_default(),
        z_index: layout.z_index.unwrap_or_default(),
    }
}

fn finalize_ui_text(text: SceneUiTextDocument) -> Option<UiTextRef> {
    Some(UiTextRef {
        value: text.value?,
        font_size: text.font_size.unwrap_or(16.0),
        color: text.color.unwrap_or_else(|| "#ffffff".to_string()),
    })
}

fn finalize_ui_image(image: SceneUiImageDocument) -> Option<UiImageRef> {
    Some(UiImageRef {
        texture: image.texture?,
        tint: image.tint,
    })
}

fn finalize_ui_interaction(interaction: SceneUiInteractionDocument) -> Option<UiInteractionRef> {
    Some(UiInteractionRef {
        action: interaction.action?,
        enabled: interaction.enabled.unwrap_or(true),
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{PrefabDocument, SceneDocument};

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
                    (
                        "texture".to_string(),
                        toml::Value::String("asset://sprites/player.png".to_string()),
                    ),
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
        assert_eq!(
            resolved.record.transform,
            Transform2D::from_translation(5.0, 6.0)
        );
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
    fn partial_prefab_sprite_defaults_survive_scene_texture_override() {
        let prefab = PrefabDocument::from_toml_str(
            r#"
            [components.sprite]
            width = 96.0
            height = 96.0
            "#,
        )
        .expect("parse prefab");
        let mut scene = SceneDocument::from_toml_str(
            r#"
            [[entities]]

            [entities.sprite]
            texture = "asset://sprites/player.png"
            "#,
        )
        .expect("parse scene");
        let scene = scene.entities.pop().expect("scene entity");

        let resolved = resolve_scene_entity(scene, Some(prefab)).expect("resolve entity");

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
    fn toml_prefab_components_resolve_into_entity_record() {
        let prefab = PrefabDocument::from_toml_str(
            r#"
            [components.name]
            value = "Player"

            [components.tags]
            values = ["player", "spawned"]

            [components.transform]
            x = 1.0
            y = 2.0

            [components.sprite]
            texture = "asset://sprites/player.png"
            width = 96.0
            height = 96.0
            "#,
        )
        .expect("parse prefab");

        let resolved =
            resolve_scene_entity(SceneEntityDocument::default(), Some(prefab)).expect("resolve");

        assert_eq!(resolved.record.name.as_deref(), Some("Player"));
        assert_eq!(resolved.record.tags, ["player", "spawned"]);
        assert_eq!(
            resolved.record.transform,
            Transform2D::from_translation(1.0, 2.0)
        );
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
    fn prefab_name_and_tags_components_are_lenient() {
        let prefab = PrefabDocument::from_toml_str(
            r#"
            [components.name]
            text = "Ignored"

            [components.tags]
            values = ["player", 42, "spawned"]
            "#,
        )
        .expect("parse prefab");

        let resolved =
            resolve_scene_entity(SceneEntityDocument::default(), Some(prefab)).expect("resolve");

        assert_eq!(resolved.record.name, None);
        assert_eq!(resolved.record.tags, ["player", "spawned"]);
    }

    #[test]
    fn scene_custom_component_overrides_prefab_component_by_type_name() {
        let mut prefab = PrefabDocument::default();
        prefab.components.insert(
            "controller".to_string(),
            toml::Value::Table(
                [
                    (
                        "type".to_string(),
                        toml::Value::String("PlayerController".to_string()),
                    ),
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

        assert_eq!(
            component
                .config
                .get("speed")
                .and_then(toml::Value::as_float),
            Some(180.0)
        );
    }
}
