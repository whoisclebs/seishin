use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display};

use crate::document::{
    SceneEntityDocument, SceneSpriteDocument, SceneTransformDocument, TagsDocument,
};
use seishin_core::Transform2D;
use serde::Deserialize;

const DEFAULT_TILE_SIZE: f32 = 80.0;

const TILEMAP_METADATA_TAG: &str = "tilemap";
const TILE_TAG: &str = "tile";
const BLOCKED_TAG: &str = "blocked";
const SPAWN_TAG: &str = "spawnpoint";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileCell {
    pub code: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileDefinition {
    pub name: String,
    pub texture: Option<String>,
    pub blocked: bool,
    pub tint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTileMap {
    pub tile_size: f32,
    pub legend: BTreeMap<u8, TileDefinition>,
    pub rows: Vec<Vec<TileCell>>,
    pub spawns: BTreeMap<String, (i32, i32)>,
}

#[derive(Debug, Clone)]
pub struct TileMapError {
    message: String,
}

impl TileMapError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for TileMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for TileMapError {}

impl Default for ParsedTileMap {
    fn default() -> Self {
        Self {
            tile_size: DEFAULT_TILE_SIZE,
            legend: BTreeMap::new(),
            rows: Vec::new(),
            spawns: BTreeMap::new(),
        }
    }
}

impl ParsedTileMap {
    pub fn width(&self) -> i32 {
        self.rows.iter().map(|row| row.len()).max().unwrap_or(0) as i32
    }

    pub fn height(&self) -> i32 {
        self.rows.len() as i32
    }

    pub fn tile_size(&self) -> f32 {
        self.tile_size
    }

    pub fn spawns(&self) -> impl Iterator<Item = (&str, (i32, i32))> {
        self.spawns
            .iter()
            .map(|(name, position)| (name.as_str(), *position))
    }

    pub fn definition(&self, code: u8) -> Option<&TileDefinition> {
        self.legend.get(&code)
    }
}

pub fn parse_tile_map(source: &str) -> Result<ParsedTileMap, TileMapError> {
    if source
        .lines()
        .any(|line| line.trim_start().starts_with("[legend"))
    {
        return parse_toml_tile_map(source);
    }

    parse_legacy_tile_map(source)
}

#[derive(Debug, Deserialize)]
struct TomlTileMap {
    tile_size: Option<f32>,
    #[serde(default)]
    legend: BTreeMap<String, TomlTileDefinition>,
    tiles: TomlTileRows,
    #[serde(default)]
    objects: BTreeMap<String, [i32; 2]>,
}

#[derive(Debug, Deserialize)]
struct TomlTileRows {
    rows: Vec<Vec<u8>>,
}

#[derive(Debug, Deserialize)]
struct TomlTileDefinition {
    name: Option<String>,
    texture: Option<String>,
    #[serde(default)]
    blocked: bool,
    tint: Option<String>,
}

fn parse_toml_tile_map(source: &str) -> Result<ParsedTileMap, TileMapError> {
    let document = toml::from_str::<TomlTileMap>(source)
        .map_err(|error| TileMapError::new(format!("invalid tile map TOML: {error}")))?;
    let tile_size = document.tile_size.unwrap_or(DEFAULT_TILE_SIZE);

    if tile_size <= 0.0 {
        return Err(TileMapError::new("tile_size must be greater than zero"));
    }
    if document.tiles.rows.is_empty() {
        return Err(TileMapError::new("map has no tile rows"));
    }

    if document.legend.is_empty() {
        return Err(TileMapError::new(
            "TOML tile maps must define at least one [legend.<code>] entry",
        ));
    }

    let mut legend = BTreeMap::new();

    for (code, definition) in document.legend {
        let code = code
            .parse::<u8>()
            .map_err(|_| TileMapError::new(format!("invalid legend tile code '{code}'")))?;
        if definition
            .texture
            .as_deref()
            .is_some_and(|texture| texture.trim().is_empty())
        {
            return Err(TileMapError::new(format!(
                "legend tile code {code} has an empty texture"
            )));
        }

        legend.insert(
            code,
            TileDefinition {
                name: definition.name.unwrap_or_else(|| code.to_string()),
                texture: definition.texture,
                blocked: definition.blocked,
                tint: definition.tint,
            },
        );
    }

    let rows = document
        .tiles
        .rows
        .into_iter()
        .enumerate()
        .map(|(row_index, row)| {
            if row.is_empty() {
                return Err(TileMapError::new(format!(
                    "tile row {row_index} must not be empty"
                )));
            }

            row.into_iter()
                .map(|code| {
                    if !legend.contains_key(&code) {
                        return Err(TileMapError::new(format!(
                            "tile code {code} in row {row_index} has no legend definition"
                        )));
                    }
                    Ok(TileCell { code })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let spawns = document
        .objects
        .into_iter()
        .map(|(name, [x, y])| (name, (x, y)))
        .collect();

    Ok(ParsedTileMap {
        tile_size,
        legend,
        rows,
        spawns,
    })
}

fn parse_legacy_tile_map(source: &str) -> Result<ParsedTileMap, TileMapError> {
    let mut map = ParsedTileMap::default();
    let mut in_tiles = false;
    let mut in_objects = false;

    for (line_number, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim().trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.trim();

        if line == "[tiles]" {
            in_tiles = true;
            in_objects = false;
            continue;
        }

        if line == "[objects]" {
            in_tiles = false;
            in_objects = true;
            continue;
        }

        if !in_tiles && !in_objects {
            let Some((key, value)) = line.split_once('=') else {
                return Err(TileMapError::new(format!(
                    "invalid map setting at line {}",
                    line_number + 1
                )));
            };
            if key.trim() == "tile_size" {
                map.tile_size = value.trim().parse::<f32>().map_err(|_| {
                    TileMapError::new(format!(
                        "invalid tile_size value at line {}",
                        line_number + 1
                    ))
                })?;
            }
            continue;
        }

        if in_tiles {
            let mut row = Vec::with_capacity(16);
            for token in line.split(',') {
                let value = token.trim().parse::<u8>().map_err(|_| {
                    TileMapError::new(format!(
                        "invalid tile code '{}' at line {}",
                        token.trim(),
                        line_number + 1
                    ))
                })?;

                let code = value;
                map.legend
                    .entry(code)
                    .or_insert_with(|| generic_tile_definition(code));
                row.push(TileCell { code });
            }
            if !row.is_empty() {
                map.rows.push(row);
            }
            continue;
        }

        if in_objects {
            let Some((name, values)) = line.split_once('=') else {
                return Err(TileMapError::new(format!(
                    "invalid object line at {}",
                    line_number + 1
                )));
            };
            let Some((x_value, y_value)) = values.split_once(',') else {
                return Err(TileMapError::new(format!(
                    "invalid object value at line {}",
                    line_number + 1
                )));
            };
            let x = x_value.trim().parse::<i32>().map_err(|_| {
                TileMapError::new(format!(
                    "invalid object x coordinate at line {}",
                    line_number + 1
                ))
            })?;
            let y = y_value.trim().parse::<i32>().map_err(|_| {
                TileMapError::new(format!(
                    "invalid object y coordinate at line {}",
                    line_number + 1
                ))
            })?;

            map.spawns.insert(name.trim().to_string(), (x, y));
            continue;
        }
    }

    if map.rows.is_empty() {
        return Err(TileMapError::new(
            "map has no [tiles] section or row definitions",
        ));
    }
    if map.tile_size <= 0.0 {
        return Err(TileMapError::new("tile_size must be greater than zero"));
    }

    Ok(map)
}

fn generic_tile_definition(code: u8) -> TileDefinition {
    TileDefinition {
        name: format!("tile_{code}"),
        texture: None,
        blocked: code != 0,
        tint: None,
    }
}

pub fn tile_map_to_scene_entities(
    map: &ParsedTileMap,
    map_index: usize,
) -> Vec<SceneEntityDocument> {
    let mut entities = Vec::new();
    let tile_size = map.tile_size();
    let map_key = format!("map_{map_index}");

    entities.push(SceneEntityDocument {
        name: Some(format!("TileMap.{map_key}")),
        tags: Some(TagsDocument {
            values: vec![TILEMAP_METADATA_TAG.to_string()],
        }),
        data: Some({
            let mut data = BTreeMap::new();
            data.insert("tile_size".to_string(), map.tile_size.to_string());
            data.insert("width".to_string(), map.width().to_string());
            data.insert("height".to_string(), map.height().to_string());
            data.insert("map_index".to_string(), map_index.to_string());
            data.insert("source".to_string(), format!("map://{map_key}"));
            data
        }),
        ..SceneEntityDocument::default()
    });

    for (row, row_tiles) in map.rows.iter().enumerate() {
        for (column, tile) in row_tiles.iter().enumerate() {
            let cell_position =
                Transform2D::from_translation(column as f32 * tile_size, row as f32 * tile_size);

            let definition = map
                .definition(tile.code)
                .expect("tile parser validates all tile codes against the legend");
            let mut tags = vec![TILE_TAG.to_string()];
            if definition.blocked {
                tags.push(BLOCKED_TAG.to_string());
            }

            let mut data = BTreeMap::new();
            data.insert("kind".to_string(), definition.name.clone());
            data.insert("code".to_string(), tile.code.to_string());
            data.insert("column".to_string(), column.to_string());
            data.insert("row".to_string(), row.to_string());

            entities.push(SceneEntityDocument {
                name: Some(format!("TileMap.{map_key}.Tile_{row}_{column}")),
                transform: Some(SceneTransformDocument {
                    x: Some(cell_position.x),
                    y: Some(cell_position.y),
                    ..SceneTransformDocument::default()
                }),
                sprite: definition
                    .texture
                    .as_ref()
                    .map(|texture| SceneSpriteDocument {
                        texture: Some(texture.clone()),
                        width: Some(tile_size),
                        height: Some(tile_size),
                        tint: definition.tint.clone(),
                        ..SceneSpriteDocument::default()
                    }),
                tags: Some(TagsDocument { values: tags }),
                data: Some(data),
                ..SceneEntityDocument::default()
            });
        }
    }

    for (name, (x, y)) in map.spawns() {
        entities.push(SceneEntityDocument {
            name: Some(format!("TileMap.{map_key}.Spawnpoint.{name}")),
            tags: Some(TagsDocument {
                values: vec![SPAWN_TAG.to_string()],
            }),
            transform: Some(SceneTransformDocument {
                x: Some(x as f32 * tile_size),
                y: Some(y as f32 * tile_size),
                ..SceneTransformDocument::default()
            }),
            data: Some({
                let mut data = BTreeMap::new();
                data.insert("spawn_for".to_string(), name.to_string());
                data.insert("map".to_string(), map_key.clone());
                data
            }),
            ..SceneEntityDocument::default()
        });
    }

    entities
}

pub const fn blocked_tag() -> &'static str {
    BLOCKED_TAG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tile_map_reads_tile_size_tiles_and_objects() {
        let source = r"
            tile_size = 40.0

            [tiles]
            0,1,2
            1,0,1

            [objects]
            Player=1,0
            Merchant=2,1
            ";

        let map = parse_tile_map(source).expect("parse map");

        assert_eq!(map.tile_size(), 40.0);
        assert_eq!(map.width(), 3);
        assert_eq!(map.height(), 2);
        assert_eq!(
            map.spawns,
            BTreeMap::from([("Player".into(), (1, 0)), ("Merchant".into(), (2, 1))])
        );
        assert_eq!(
            map.rows[0],
            vec![
                TileCell { code: 0 },
                TileCell { code: 1 },
                TileCell { code: 2 }
            ]
        );
        assert_eq!(
            map.definition(0)
                .and_then(|definition| definition.texture.as_deref()),
            None
        );
        assert_eq!(
            map.definition(1).map(|definition| definition.blocked),
            Some(true)
        );
    }

    #[test]
    fn parse_tile_map_reads_toml_legend_rows_and_objects() {
        let source = r#"
            tile_size = 48.0

            [legend.0]
            name = "open"
            texture = "asset://tiles/open.png"
            blocked = false

            [legend.1]
            name = "solid"
            texture = "asset://tiles/solid.png"
            blocked = true

            [tiles]
            rows = [
              [0, 1],
              [1, 0],
            ]

            [objects]
            Player = [1, 0]
            "#;

        let map = parse_tile_map(source).expect("parse map");

        assert_eq!(map.tile_size(), 48.0);
        assert_eq!(
            map.rows[0],
            vec![TileCell { code: 0 }, TileCell { code: 1 }]
        );
        assert_eq!(map.spawns, BTreeMap::from([("Player".into(), (1, 0))]));
        assert_eq!(
            map.definition(1)
                .and_then(|definition| definition.texture.as_deref()),
            Some("asset://tiles/solid.png")
        );
        assert_eq!(
            map.definition(1).map(|definition| definition.blocked),
            Some(true)
        );
    }

    #[test]
    fn tile_map_to_scene_entities_emits_tiles_and_spawnpoints() {
        let map = ParsedTileMap {
            tile_size: 64.0,
            legend: BTreeMap::from([
                (
                    0,
                    TileDefinition {
                        name: "open".to_string(),
                        texture: Some("asset://game/open.png".to_string()),
                        blocked: false,
                        tint: None,
                    },
                ),
                (
                    1,
                    TileDefinition {
                        name: "blocked".to_string(),
                        texture: Some("asset://game/blocked.png".to_string()),
                        blocked: true,
                        tint: None,
                    },
                ),
            ]),
            rows: vec![vec![TileCell { code: 0 }, TileCell { code: 1 }]],
            spawns: BTreeMap::from([("Player".into(), (3, 0))]),
        };

        let entities = tile_map_to_scene_entities(&map, 2);

        assert!(entities.iter().any(|entity| entity
            .tags
            .as_ref()
            .is_some_and(|tags| tags.values.contains(&TILE_TAG.to_string()))));
        assert_eq!(entities.len(), 1 + 2 + 1);
        assert_eq!(entities[1].name.as_deref(), Some("TileMap.map_2.Tile_0_0"));
        assert_eq!(entities[0].name.as_deref(), Some("TileMap.map_2"));
        assert_eq!(
            entities[3].name.as_deref(),
            Some("TileMap.map_2.Spawnpoint.Player")
        );
        assert_eq!(
            entities[1]
                .sprite
                .as_ref()
                .and_then(|sprite| sprite.texture.as_deref()),
            Some("asset://game/open.png")
        );
        assert_eq!(
            entities[2]
                .sprite
                .as_ref()
                .and_then(|sprite| sprite.texture.as_deref()),
            Some("asset://game/blocked.png")
        );
    }
}
