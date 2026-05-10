use seishin::prelude::*;

const SOLID_ACTOR_TAG: &str = "solid_actor";

pub type TileMapState = TileMapQuery;

pub fn can_actor_occupy(
    world: &World,
    map: Option<&TileMapState>,
    entity: Entity,
    position: Vec2,
) -> bool {
    can_entity_occupy_tilemap(world, map, entity, position, Some(SOLID_ACTOR_TAG))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solid_actor_tag_blocks_actor_occupancy() {
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
                    tags: vec![SOLID_ACTOR_TAG.to_string()],
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

        assert!(!can_actor_occupy(
            &world,
            None,
            player,
            Vec2::new(47.0, 0.0)
        ));
        assert!(can_actor_occupy(&world, None, player, Vec2::new(0.0, 0.0)));
    }
}
