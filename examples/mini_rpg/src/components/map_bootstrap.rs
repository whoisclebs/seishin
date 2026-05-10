use seishin::prelude::*;

use super::map_state::TileMapState;

#[derive(Debug)]
pub struct MapBootstrap {
    initialized: bool,
}

impl MapBootstrap {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Component for MapBootstrap {
    fn update(&mut self, _entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()> {
        if self.initialized {
            return Ok(());
        }

        let Some(map) = TileMapState::from_world(&context.world()) else {
            return Ok(());
        };

        for (name, (column, row)) in map.spawns() {
            if let Some(entity) = context.world().entity_by_name(name) {
                let target = map.tile_position_to_world(column, row);
                context.world().set_position(entity, target.x, target.y);
            }
        }

        self.initialized = true;
        Ok(())
    }
}

pub fn map_bootstrap_factory(_: &toml::Value) -> GameResult<Box<dyn Component>> {
    Ok(Box::new(MapBootstrap::new()))
}

pub fn new() -> impl ComponentDefinition {
    component_factory("MapBootstrap", map_bootstrap_factory)
}
