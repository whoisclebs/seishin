use std::collections::HashMap;

use seishin::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct TileMapState {
    tile_size: f32,
    width: i32,
    height: i32,
    blocked: Vec<(i32, i32)>,
    spawns: HashMap<String, (i32, i32)>,
}

impl TileMapState {
    pub const DEFAULT_TILE_SIZE: f32 = 80.0;

    pub fn from_world(world: &FrameWorld<'_>) -> Option<Self> {
        let map_entity = world.first_with_tag("tilemap")?;
        let tile_size =
            parse_f32(world.data_ref(map_entity, "tile_size")).unwrap_or(Self::DEFAULT_TILE_SIZE);
        let width = parse_i32(world.data_ref(map_entity, "width")).unwrap_or(0);
        let height = parse_i32(world.data_ref(map_entity, "height")).unwrap_or(0);

        let mut blocked = Vec::new();
        for entity in world.entities_with_tag("tile") {
            let tags = world.tags(entity).unwrap_or_default();
            if tags.iter().any(|tag| tag == "blocked") {
                let column = parse_i32(world.data_ref(entity, "column"));
                let row = parse_i32(world.data_ref(entity, "row"));

                if let (Some(column), Some(row)) = (column, row) {
                    blocked.push((column, row));
                }
            }
        }

        let mut spawns = HashMap::new();
        for entity in world.entities_with_tag("spawnpoint") {
            let Some(name) = world.data_ref(entity, "spawn_for") else {
                continue;
            };
            let transform = world.transform(entity)?;
            let column = (transform.x / tile_size).floor() as i32;
            let row = (transform.y / tile_size).floor() as i32;

            spawns.insert(name.to_string(), (column, row));
        }

        Some(Self {
            tile_size,
            width,
            height,
            blocked,
            spawns,
        })
    }

    pub fn is_walkable(&self, world_x: f32, world_y: f32) -> bool {
        if self.width <= 0 || self.height <= 0 || self.tile_size <= 0.0 {
            return false;
        }

        let column = (world_x / self.tile_size).floor() as i32;
        let row = (world_y / self.tile_size).floor() as i32;
        if column < 0 || row < 0 || column >= self.width || row >= self.height {
            return false;
        }

        !self.blocked.contains(&(column, row))
    }

    pub fn tile_position_to_world(&self, column: i32, row: i32) -> Vec2 {
        Vec2::new(column as f32 * self.tile_size, row as f32 * self.tile_size)
    }

    pub fn interaction_range(&self, distance_factor: f32) -> f32 {
        self.tile_size * distance_factor
    }

    pub fn spawns(&self) -> impl Iterator<Item = (&str, (i32, i32))> + '_ {
        self.spawns
            .iter()
            .map(|(name, position)| (name.as_str(), *position))
    }
}

pub fn parse_f32(value: Option<&str>) -> Option<f32> {
    value.and_then(|value| value.parse::<f32>().ok())
}

pub fn parse_i32(value: Option<&str>) -> Option<i32> {
    value.and_then(|value| value.parse::<i32>().ok())
}

pub fn closest_character(
    world: &FrameWorld<'_>,
    from: Vec2,
    interaction_range: f32,
) -> Option<Entity> {
    let max_distance = interaction_range * interaction_range;
    let mut nearest: Option<(Entity, f32)> = None;

    for entity in world.entities_with_tag("character") {
        let Some(transform) = world.transform(entity) else {
            continue;
        };
        let dx = transform.x - from.x;
        let dy = transform.y - from.y;
        let distance_squared = dx * dx + dy * dy;

        if distance_squared <= max_distance {
            match nearest {
                None => nearest = Some((entity, distance_squared)),
                Some((_, closest)) if distance_squared < closest => {
                    nearest = Some((entity, distance_squared))
                }
                _ => {}
            }
        }
    }

    nearest.map(|(entity, _)| entity)
}
