use std::{any::TypeId, collections::HashMap};

use seishin_core::{EntityId, Transform2D};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntityRecord {
    pub name: Option<String>,
    pub tags: Vec<String>,
    pub data_refs: HashMap<String, String>,
    pub custom_components: Vec<CustomComponentRef>,
    pub transform: Transform2D,
    pub sprite: Option<SpriteRef>,
    pub audio: Option<AudioRef>,
    pub ui: Option<UiRef>,
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

    pub fn with_ui(mut self, ui: UiRef) -> Self {
        self.ui = Some(ui);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpriteRef {
    pub texture: String,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub source_x: Option<u32>,
    pub source_y: Option<u32>,
    pub source_width: Option<u32>,
    pub source_height: Option<u32>,
    pub layer: i32,
    pub sort_order: i32,
    pub tint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioRef {
    pub sound: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiRef {
    pub layout: UiLayoutRef,
    pub text: Option<UiTextRef>,
    pub image: Option<UiImageRef>,
    pub interaction: Option<UiInteractionRef>,
}

impl UiRef {
    pub fn new(layout: UiLayoutRef) -> Self {
        Self {
            layout,
            text: None,
            image: None,
            interaction: None,
        }
    }

    pub fn text(value: impl Into<String>, anchor: UiAnchor) -> Self {
        Self {
            layout: UiLayoutRef {
                anchor,
                ..UiLayoutRef::default()
            },
            text: Some(UiTextRef {
                value: value.into(),
                font_size: 16.0,
                color: "#ffffff".to_string(),
            }),
            image: None,
            interaction: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiLayoutRef {
    pub anchor: UiAnchor,
    pub offset_x: f32,
    pub offset_y: f32,
    pub width: f32,
    pub height: f32,
    pub z_index: i32,
}

impl Default for UiLayoutRef {
    fn default() -> Self {
        Self {
            anchor: UiAnchor::TopLeft,
            offset_x: 0.0,
            offset_y: 0.0,
            width: 0.0,
            height: 0.0,
            z_index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UiAnchor {
    #[default]
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiTextRef {
    pub value: String,
    pub font_size: f32,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiImageRef {
    pub texture: String,
    pub tint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiInteractionRef {
    pub action: String,
    pub enabled: bool,
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
