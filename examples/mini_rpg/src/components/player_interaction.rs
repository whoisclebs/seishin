use seishin::prelude::*;

use super::map_state::{closest_character, TileMapState};

const DEFAULT_INTERACT_DISTANCE_TILE_FACTOR: f32 = 1.4;
const DEFAULT_INTERACT_ACTION: &str = "interact_rpg";

#[derive(Debug)]
pub struct PlayerInteraction {
    action: String,
    interact_distance_tiles: f32,
}

impl PlayerInteraction {
    pub fn new(action: String, interact_distance_tiles: f32) -> Self {
        Self {
            action,
            interact_distance_tiles,
        }
    }
}

pub fn player_interaction_factory(config: &toml::Value) -> GameResult<Box<dyn Component>> {
    let action = config
        .get("action")
        .and_then(|value| value.as_str())
        .unwrap_or(DEFAULT_INTERACT_ACTION)
        .to_string();
    let interact_distance_tiles = config
        .get("interact_distance_tiles")
        .and_then(|value| value.as_float())
        .unwrap_or(DEFAULT_INTERACT_DISTANCE_TILE_FACTOR as f64)
        as f32;

    Ok(Box::new(PlayerInteraction::new(
        action,
        interact_distance_tiles,
    )))
}

pub fn new() -> impl ComponentDefinition {
    component_factory("PlayerInteraction", player_interaction_factory)
}

impl Component for PlayerInteraction {
    fn update(&mut self, entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()> {
        if !context.input().just_pressed(self.action.as_str()) {
            return Ok(());
        }

        if context.dialogue().is_active() {
            context.dialogue().advance_or_close();
            return Ok(());
        }

        let player_position = context
            .world()
            .transform(entity)
            .map(|transform| Vec2::new(transform.x, transform.y))
            .unwrap_or(Vec2::ZERO);

        let map = TileMapState::from_world(&context.world());
        let interaction_range = map
            .map(|map| map.interaction_range(self.interact_distance_tiles))
            .unwrap_or(TileMapState::DEFAULT_TILE_SIZE * DEFAULT_INTERACT_DISTANCE_TILE_FACTOR);

        let target = {
            let world = context.world();
            closest_character(&world, player_position, interaction_range)
        };

        let Some(target) = target else {
            return Ok(());
        };

        let character_path = context
            .world()
            .data_ref(target, "character")
            .map(ToString::to_string);
        let Some(character_path) = character_path else {
            return Ok(());
        };

        let character = context.resources().character(character_path)?;
        let Some(dialogue_file) = character
            .dialogue
            .as_ref()
            .map(|dialogue| dialogue.default.as_str())
        else {
            return Ok(());
        };

        let dialogue = context.resources().dialogue(dialogue_file)?;
        context.dialogue().open(character.display_name, dialogue);
        let _ = context.play_entity_audio(target);

        Ok(())
    }
}
