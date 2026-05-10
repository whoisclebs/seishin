use seishin::prelude::*;

mod components;

fn main() -> GameResult<()> {
    App::discover_project()?
        .add_component(components::map_bootstrap::new())
        .add_component(components::dev_validation::new())
        .add_component(components::player_controller::new())
        .add_component(components::player_interaction::new())
        .add_component(components::player_camera::new())
        .add_component(components::wanderer::new())
        .run()
}

#[cfg(test)]
mod tests {
    use seishin::prelude::*;

    const OVERWORLD_MAP: &str = include_str!("../resources/data/maps/overworld.map");

    #[test]
    fn overworld_legend_points_to_expected_tileset_cells() {
        let map = parse_tile_map(OVERWORLD_MAP).expect("parse overworld map");
        let entities = tile_map_to_scene_entities(&map, 0);

        assert_tile(&map, &entities, 0, "grass", 0, (0, 0));
        assert_tile(&map, &entities, 1, "stone_wall", 32, (32, 48));
        assert_tile(&map, &entities, 2, "water", 11, (16, 16));
        assert_tile(&map, &entities, 3, "dirt_path", 31, (16, 48));
    }

    fn assert_tile(
        map: &ParsedTileMap,
        entities: &[SceneEntityDocument],
        code: u8,
        kind: &str,
        atlas_index: u32,
        source: (u32, u32),
    ) {
        let definition = map.definition(code).expect("legend tile");
        assert_eq!(definition.name, kind);
        assert_eq!(definition.atlas_index, Some(atlas_index));

        let sprite = entities
            .iter()
            .find(|entity| {
                entity
                    .data
                    .as_ref()
                    .and_then(|data| data.get("kind"))
                    .is_some_and(|value| value == kind)
            })
            .and_then(|entity| entity.sprite.as_ref())
            .expect("tile sprite");

        assert_eq!(sprite.source_x, Some(source.0));
        assert_eq!(sprite.source_y, Some(source.1));
        assert_eq!(sprite.source_width, Some(16));
        assert_eq!(sprite.source_height, Some(16));
    }
}
