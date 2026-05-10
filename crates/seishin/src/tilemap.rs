use std::collections::HashMap;

use seishin_physics::Aabb;
use seishin_world::World;

use crate::{Entity, Vec2};

#[derive(Debug, Clone, Default)]
pub struct TileMapQuery {
    tile_size: f32,
    width: i32,
    height: i32,
    blocked: Vec<(i32, i32)>,
    spawns: HashMap<String, (i32, i32)>,
}

impl TileMapQuery {
    pub const DEFAULT_TILE_SIZE: f32 = 80.0;

    pub fn from_world(world: &World) -> Option<Self> {
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

    pub fn tile_size(&self) -> f32 {
        self.tile_size
    }

    pub fn is_area_walkable(&self, bounds: Aabb) -> bool {
        if self.width <= 0 || self.height <= 0 || self.tile_size <= 0.0 {
            return false;
        }

        let min = self.world_to_tile(bounds.min_x, bounds.min_y);
        let max = self.world_to_tile(bounds.max_x, bounds.max_y);
        let (Some((min_column, min_row)), Some((max_column, max_row))) = (min, max) else {
            return false;
        };

        for row in min_row..=max_row {
            for column in min_column..=max_column {
                if self.blocked.contains(&(column, row)) {
                    let tile = self.tile_bounds(column, row);
                    if bounds.intersects(&tile) {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn world_to_tile(&self, world_x: f32, world_y: f32) -> Option<(i32, i32)> {
        let column = ((world_x + self.tile_size * 0.5) / self.tile_size).floor() as i32;
        let row = ((world_y + self.tile_size * 0.5) / self.tile_size).floor() as i32;
        if column < 0 || row < 0 || column >= self.width || row >= self.height {
            return None;
        }

        Some((column, row))
    }

    fn tile_bounds(&self, column: i32, row: i32) -> Aabb {
        Aabb::from_center_size(
            column as f32 * self.tile_size,
            row as f32 * self.tile_size,
            self.tile_size,
            self.tile_size,
        )
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

    #[cfg(test)]
    fn is_point_walkable(&self, world_x: f32, world_y: f32) -> bool {
        if self.width <= 0 || self.height <= 0 || self.tile_size <= 0.0 {
            return false;
        }

        let Some((column, row)) = self.world_to_tile(world_x, world_y) else {
            return false;
        };

        !self.blocked.contains(&(column, row))
    }
}

pub fn entity_sprite_aabb(
    world: &World,
    entity: Entity,
    position: Vec2,
    fallback_size: f32,
) -> Aabb {
    let (width, height) = world
        .sprite(entity)
        .map(|sprite| {
            (
                sprite.width.unwrap_or(fallback_size),
                sprite.height.unwrap_or(fallback_size),
            )
        })
        .unwrap_or((fallback_size, fallback_size));

    Aabb::from_center_size(position.x, position.y, width, height)
}

pub fn intersects_entities_with_tag(
    world: &World,
    moving: Entity,
    tag: &str,
    bounds: Aabb,
    fallback_size: f32,
) -> bool {
    for entity in world.entities_with_tag(tag) {
        if entity == moving {
            continue;
        }
        let Some(transform) = world.transform(entity) else {
            continue;
        };
        let other_bounds = entity_sprite_aabb(
            world,
            entity,
            Vec2::new(transform.x, transform.y),
            fallback_size,
        );
        if bounds.intersects(&other_bounds) {
            return true;
        }
    }

    false
}

pub fn can_entity_occupy_tilemap(
    world: &World,
    map: Option<&TileMapQuery>,
    entity: Entity,
    position: Vec2,
    solid_entity_tag: Option<&str>,
) -> bool {
    let fallback_size = map
        .map(TileMapQuery::tile_size)
        .unwrap_or(TileMapQuery::DEFAULT_TILE_SIZE);
    let bounds = entity_sprite_aabb(world, entity, position, fallback_size);

    if map.is_some_and(|map| !map.is_area_walkable(bounds)) {
        return false;
    }

    !solid_entity_tag
        .map(|tag| intersects_entities_with_tag(world, entity, tag, bounds, fallback_size))
        .unwrap_or(false)
}

fn parse_f32(value: Option<&str>) -> Option<f32> {
    value.and_then(|value| value.parse::<f32>().ok())
}

fn parse_i32(value: Option<&str>) -> Option<i32> {
    value.and_then(|value| value.parse::<i32>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use seishin_core::{EntityId, Transform2D};
    use seishin_world::{EntityRecord, SpriteRef};

    fn test_map() -> TileMapQuery {
        TileMapQuery {
            tile_size: 48.0,
            width: 4,
            height: 2,
            blocked: vec![(1, 0)],
            spawns: HashMap::new(),
        }
    }

    #[test]
    fn solid_tile_blocks_actor_bounds_against_whole_tile() {
        let map = test_map();

        assert!(map.is_area_walkable(Aabb::from_center_size(0.0, 0.0, 48.0, 48.0)));
        assert!(!map.is_area_walkable(Aabb::from_center_size(48.0, 0.0, 48.0, 48.0)));
        assert!(!map.is_area_walkable(Aabb::from_center_size(72.0, 0.0, 48.0, 48.0)));
    }

    #[test]
    fn walkability_uses_tile_centers_in_world_space() {
        let map = test_map();

        assert!(map.is_point_walkable(0.0, 0.0));
        assert!(!map.is_point_walkable(48.0, 0.0));
        assert!(!map.is_point_walkable(71.9, 0.0));
        assert!(map.is_point_walkable(72.1, 0.0));
    }

    #[test]
    fn tagged_entity_bounds_block_moving_entity() {
        let mut world = World::default();
        let player = EntityId::new(1);
        let npc = EntityId::new(2);
        world
            .insert(
                player,
                EntityRecord {
                    sprite: Some(SpriteRef {
                        texture: "asset://sprites/player.png".to_string(),
                        width: Some(48.0),
                        height: Some(48.0),
                        source_x: None,
                        source_y: None,
                        source_width: None,
                        source_height: None,
                        layer: 0,
                        sort_order: 0,
                        tint: None,
                    }),
                    transform: Transform2D::from_translation(0.0, 0.0),
                    ..EntityRecord::default()
                },
            )
            .expect("player");
        world
            .insert(
                npc,
                EntityRecord {
                    tags: vec!["solid_actor".to_string()],
                    sprite: Some(SpriteRef {
                        texture: "asset://sprites/npc.png".to_string(),
                        width: Some(48.0),
                        height: Some(48.0),
                        source_x: None,
                        source_y: None,
                        source_width: None,
                        source_height: None,
                        layer: 0,
                        sort_order: 0,
                        tint: None,
                    }),
                    transform: Transform2D::from_translation(48.0, 0.0),
                    ..EntityRecord::default()
                },
            )
            .expect("npc");

        assert!(intersects_entities_with_tag(
            &world,
            player,
            "solid_actor",
            Aabb::from_center_size(47.0, 0.0, 48.0, 48.0),
            48.0
        ));
        assert!(!intersects_entities_with_tag(
            &world,
            player,
            "solid_actor",
            Aabb::from_center_size(0.0, 0.0, 48.0, 48.0),
            48.0
        ));
    }

    #[test]
    fn entity_occupancy_checks_tiles_and_tagged_entities() {
        let map = test_map();
        let mut world = World::default();
        let player = EntityId::new(1);
        let npc = EntityId::new(2);

        world
            .insert(
                player,
                EntityRecord {
                    sprite: Some(SpriteRef {
                        texture: "asset://sprites/player.png".to_string(),
                        width: Some(48.0),
                        height: Some(48.0),
                        source_x: None,
                        source_y: None,
                        source_width: None,
                        source_height: None,
                        layer: 0,
                        sort_order: 0,
                        tint: None,
                    }),
                    transform: Transform2D::from_translation(0.0, 0.0),
                    ..EntityRecord::default()
                },
            )
            .expect("player");
        world
            .insert(
                npc,
                EntityRecord {
                    tags: vec!["solid_actor".to_string()],
                    sprite: Some(SpriteRef {
                        texture: "asset://sprites/npc.png".to_string(),
                        width: Some(48.0),
                        height: Some(48.0),
                        source_x: None,
                        source_y: None,
                        source_width: None,
                        source_height: None,
                        layer: 0,
                        sort_order: 0,
                        tint: None,
                    }),
                    transform: Transform2D::from_translation(96.0, 0.0),
                    ..EntityRecord::default()
                },
            )
            .expect("npc");

        assert!(can_entity_occupy_tilemap(
            &world,
            Some(&map),
            player,
            Vec2::new(0.0, 0.0),
            Some("solid_actor")
        ));
        assert!(!can_entity_occupy_tilemap(
            &world,
            Some(&map),
            player,
            Vec2::new(48.0, 0.0),
            Some("solid_actor")
        ));
        assert!(!can_entity_occupy_tilemap(
            &world,
            Some(&map),
            player,
            Vec2::new(96.0, 0.0),
            Some("solid_actor")
        ));
    }
}
