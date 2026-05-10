use seishin::prelude::*;

use super::map_state::{can_actor_occupy, TileMapState};

#[derive(Debug, Default)]
pub struct DevValidation {
    ran: bool,
}

impl Component for DevValidation {
    fn update(&mut self, _entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()> {
        if self.ran {
            return Ok(());
        }
        self.ran = true;

        validate_loaded_scene(context)?;
        validate_procedural_map_pipeline()?;
        context.save_world("user://saves/dev_validation.scene.toml")?;

        Ok(())
    }
}

fn validate_loaded_scene(context: &mut FrameContext<'_>) -> GameResult<()> {
    let ui_layout = context.ui_layout(RenderSize::new(800, 600));
    let hud_title = context
        .query()
        .entity_by_name("HudTitle")
        .ok_or("DevValidation expected HudTitle UI entity")?;
    if ui_layout
        .iter()
        .find(|element| element.entity() == hud_title)
        .and_then(UiElement::computed_rect)
        .is_none()
    {
        return Err("DevValidation expected HudTitle to have computed UI layout".into());
    }

    let world = context.world();
    let map =
        TileMapState::from_world(&world).ok_or("DevValidation expected a loaded tilemap entity")?;
    let atlas_tiles = world.entities_with_tag("tile").into_iter().any(|entity| {
        world.sprite(entity).is_some_and(|sprite| {
            sprite.source_x.is_some()
                && sprite.source_y.is_some()
                && sprite.source_width.is_some()
                && sprite.source_height.is_some()
        })
    });
    if !atlas_tiles {
        return Err("DevValidation expected atlas-backed tile sprites".into());
    }

    let player = world
        .first_with_tag("player")
        .ok_or("DevValidation expected a player entity")?;
    let blocked_tile = world
        .first_with_tag("blocked")
        .ok_or("DevValidation expected at least one blocked tile")?;
    let blocked_position = world
        .transform(blocked_tile)
        .map(|transform| Vec2::new(transform.x, transform.y))
        .ok_or("DevValidation expected blocked tile transform")?;
    if can_actor_occupy(&world, Some(&map), player, blocked_position) {
        return Err("DevValidation expected blocked tiles to reject actor occupancy".into());
    }

    let npc = world
        .first_with_tag("npc")
        .ok_or("DevValidation expected an npc entity")?;
    let npc_position = world
        .transform(npc)
        .map(|transform| Vec2::new(transform.x, transform.y))
        .ok_or("DevValidation expected npc transform")?;
    if can_actor_occupy(&world, Some(&map), player, npc_position) {
        return Err("DevValidation expected npc actors to reject actor occupancy".into());
    }

    Ok(())
}

fn validate_procedural_map_pipeline() -> GameResult<()> {
    let generated = ProceduralTileMapBuilder::new(ProceduralSeed::from_text("mini-rpg-dev"), 5, 4)
        .tile_size(16.0)
        .legend_tile(
            0,
            TileDefinition {
                name: "open".to_string(),
                texture: None,
                atlas_index: Some(1),
                blocked: false,
                tint: None,
            },
        )
        .legend_tile(
            1,
            TileDefinition {
                name: "solid".to_string(),
                texture: None,
                atlas_index: Some(2),
                blocked: true,
                tint: None,
            },
        )
        .tileset(TileSetDefinition {
            atlas: "asset://sprites/open_tileset.png".to_string(),
            tile_width: 16,
            tile_height: 16,
            columns: 10,
            margin: 0,
            spacing: 0,
        })
        .spawn("GeneratedPlayer", 2, 2)
        .fill(|rng, column, row| {
            if row == 0 || column == 0 || column == 4 {
                1
            } else {
                rng.range_u32(0, 2) as u8
            }
        })
        .build();
    let entities = tile_map_to_scene_entities(&generated, 99);

    if generated.width() != 5 || generated.height() != 4 {
        return Err("DevValidation expected generated map dimensions".into());
    }
    if !entities.iter().any(|entity| {
        entity
            .name
            .as_deref()
            .is_some_and(|name| name.ends_with("Spawnpoint.GeneratedPlayer"))
    }) {
        return Err("DevValidation expected generated spawnpoint entity".into());
    }

    Ok(())
}

pub fn new() -> impl ComponentDefinition {
    component::<DevValidation>("DevValidation")
}
