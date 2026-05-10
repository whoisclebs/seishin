use seishin::prelude::*;

use super::map_state::TileMapState;

const DEFAULT_PLAYER_SPEED: f32 = 180.0;
const DEFAULT_STEP_SOUND_COOLDOWN_SECONDS: f32 = 0.18;
const DEFAULT_PLAYER_CONTROLLER_ACTION: &str = "move";

#[derive(Debug)]
pub struct PlayerController {
    speed: f32,
    step_sound_cooldown: f32,
}

impl PlayerController {
    pub fn new(speed: f32, step_sound_cooldown: f32) -> Self {
        Self {
            speed: if speed > 0.0 {
                speed
            } else {
                DEFAULT_PLAYER_SPEED
            },
            step_sound_cooldown: if step_sound_cooldown > 0.0 {
                step_sound_cooldown
            } else {
                DEFAULT_STEP_SOUND_COOLDOWN_SECONDS
            },
        }
    }

    fn attempt_move(
        &mut self,
        frame: &mut FrameContext<'_>,
        entity: Entity,
        movement: Vec2,
    ) -> GameResult<()> {
        let magnitude = (movement.x * movement.x + movement.y * movement.y).sqrt();
        if magnitude <= f32::EPSILON {
            return Ok(());
        }

        let normalized = if magnitude > 1.0 {
            Vec2 {
                x: movement.x / magnitude,
                y: movement.y / magnitude,
            }
        } else {
            movement
        };

        let current = frame
            .world()
            .transform(entity)
            .map(|transform| Vec2::new(transform.x, transform.y))
            .unwrap_or(Vec2::ZERO);

        let dt = frame.delta_seconds() * self.speed;
        let candidate = Vec2::new(current.x + normalized.x * dt, current.y + normalized.y * dt);
        let mut final_position = current;

        match TileMapState::from_world(&frame.world()) {
            Some(map) => {
                let candidate_x = candidate.x;
                let candidate_y = candidate.y;

                if map.is_walkable(candidate_x, current.y) {
                    final_position.x = candidate_x;
                }
                if map.is_walkable(final_position.x, candidate_y) {
                    final_position.y = candidate_y;
                }
            }
            None => {
                final_position = candidate;
            }
        }

        if (final_position.x - current.x).abs() > f32::EPSILON
            || (final_position.y - current.y).abs() > f32::EPSILON
        {
            frame
                .world()
                .set_position(entity, final_position.x, final_position.y);
            self.play_footstep(frame, entity)?;
        }

        Ok(())
    }

    fn play_footstep(&mut self, frame: &mut FrameContext<'_>, entity: Entity) -> GameResult<()> {
        if self.step_sound_cooldown > 0.0 {
            return Ok(());
        }

        let _ = frame.play_entity_audio(entity);
        self.step_sound_cooldown = DEFAULT_STEP_SOUND_COOLDOWN_SECONDS;
        Ok(())
    }
}

impl Component for PlayerController {
    fn update(&mut self, entity: Entity, context: &mut FrameContext<'_>) -> GameResult<()> {
        if self.step_sound_cooldown > 0.0 {
            self.step_sound_cooldown -= context.delta_seconds();
        }

        let movement = context.input().axis2d(DEFAULT_PLAYER_CONTROLLER_ACTION);
        self.attempt_move(context, entity, movement)
    }
}

pub fn player_controller_factory(config: &toml::Value) -> GameResult<Box<dyn Component>> {
    let speed = config
        .get("speed")
        .and_then(|value| value.as_float())
        .unwrap_or(DEFAULT_PLAYER_SPEED as f64) as f32;
    let step_sound_cooldown = config
        .get("step_sound_cooldown_seconds")
        .and_then(|value| value.as_float())
        .unwrap_or(DEFAULT_STEP_SOUND_COOLDOWN_SECONDS as f64) as f32;

    Ok(Box::new(PlayerController::new(speed, step_sound_cooldown)))
}

pub fn new() -> impl ComponentDefinition {
    component_factory("PlayerController", player_controller_factory)
}
