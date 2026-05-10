use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::record::{UiAnchor, UiImageRef, UiInteractionRef, UiRef, UiTextRef};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct SceneDocument {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub maps: Vec<SceneMapDocument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<SceneEntityDocument>,
}

impl SceneDocument {
    pub fn from_toml_str(source: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(source)
    }

    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct SceneEntityDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefab: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<SceneInstanceDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<SceneTransformDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<TagsDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sprite: Option<SceneSpriteDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<SceneAudioDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<SceneUiDocument>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<CustomComponentDocument>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct SceneMapDocument {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_size: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct PrefabDocument {
    #[serde(default)]
    pub components: HashMap<String, toml::Value>,
}

impl PrefabDocument {
    pub fn from_toml_str(source: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(source)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SceneInstanceDocument {
    pub scene: String,
    pub source_entity: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct TagsDocument {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
pub struct SceneTransformDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation_radians: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_x: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_y: Option<f32>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct SceneSpriteDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_x: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_y: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct SceneAudioDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct SceneUiDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<SceneUiLayoutDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<SceneUiTextDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<SceneUiImageDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction: Option<SceneUiInteractionDocument>,
}

impl From<&UiRef> for SceneUiDocument {
    fn from(ui: &UiRef) -> Self {
        Self {
            layout: Some(SceneUiLayoutDocument {
                anchor: Some(ui.layout.anchor),
                offset_x: Some(ui.layout.offset_x),
                offset_y: Some(ui.layout.offset_y),
                width: Some(ui.layout.width),
                height: Some(ui.layout.height),
                z_index: Some(ui.layout.z_index),
            }),
            text: ui.text.as_ref().map(SceneUiTextDocument::from),
            image: ui.image.as_ref().map(SceneUiImageDocument::from),
            interaction: ui
                .interaction
                .as_ref()
                .map(SceneUiInteractionDocument::from),
        }
    }
}

impl From<UiRef> for SceneUiDocument {
    fn from(ui: UiRef) -> Self {
        Self::from(&ui)
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
pub struct SceneUiLayoutDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<UiAnchor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_x: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_y: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct SceneUiTextDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl From<&UiTextRef> for SceneUiTextDocument {
    fn from(text: &UiTextRef) -> Self {
        Self {
            value: Some(text.value.clone()),
            font_size: Some(text.font_size),
            color: Some(text.color.clone()),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct SceneUiImageDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tint: Option<String>,
}

impl From<&UiImageRef> for SceneUiImageDocument {
    fn from(image: &UiImageRef) -> Self {
        Self {
            texture: Some(image.texture.clone()),
            tint: image.tint.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct SceneUiInteractionDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

impl From<&UiInteractionRef> for SceneUiInteractionDocument {
    fn from(interaction: &UiInteractionRef) -> Self {
        Self {
            action: Some(interaction.action.clone()),
            enabled: Some(interaction.enabled),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CustomComponentDocument {
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(flatten, skip_serializing_if = "BTreeMap::is_empty")]
    pub config: BTreeMap<String, toml::Value>,
}
