use seishin::prelude::*;

mod components;

fn main() -> GameResult<()> {
    App::discover_project()?
        .add_component(components::map_bootstrap::new())
        .add_component(components::player_controller::new())
        .add_component(components::player_interaction::new())
        .add_component(components::player_camera::new())
        .add_component(components::wanderer::new())
        .run()
}
