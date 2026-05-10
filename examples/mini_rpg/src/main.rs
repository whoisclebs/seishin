use std::collections::HashMap;

use seishin::prelude::*;

mod components;

use components::WandererController;

const TILE_SIZE: f32 = 80.0;
const DEFAULT_PLAYER_SPEED: f32 = 180.0;
const STEP_SOUND_COOLDOWN_SECONDS: f32 = 0.18;
const INTERACT_DISTANCE: f32 = TILE_SIZE * 1.4;
const MINI_RPG_MAP: &str = include_str!("../resources/data/maps/overworld.map");

#[derive(Debug, Clone)]
struct MapDefinition {
    rows: Vec<Vec<MapCell>>,
    spawns: HashMap<String, (i32, i32)>,
}

#[derive(Debug, Clone, Copy)]
enum MapCell {
    Floor,
    Water,
    Wall,
}

impl MapDefinition {
    fn width(&self) -> i32 {
        self.rows.iter().map(|row| row.len()).max().unwrap_or(0) as i32
    }

    fn height(&self) -> i32 {
        self.rows.len() as i32
    }

    fn is_walkable(&self, world_x: f32, world_y: f32) -> bool {
        let x = (world_x / TILE_SIZE).floor() as i32;
        let y = (world_y / TILE_SIZE).floor() as i32;

        if x < 0 || y < 0 || y >= self.height() || x >= self.width() {
            return false;
        }

        let row = self.rows[y as usize].as_slice();
        if x as usize >= row.len() {
            return false;
        }

        !matches!(row[x as usize], MapCell::Wall | MapCell::Water)
    }

    fn tile_position_to_world(&self, x: i32, y: i32) -> Vec2 {
        Vec2::new(x as f32 * TILE_SIZE, y as f32 * TILE_SIZE)
    }
}

impl Default for MapDefinition {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            spawns: HashMap::new(),
        }
    }
}

struct Game {
    map: MapDefinition,
    player_entity: Option<Entity>,
    initialized: bool,
    step_sound_cooldown: f32,
    player_speed: f32,
    camera_target: Vec2,
}

impl Game {
    fn parse_map(source: &str) -> MapDefinition {
        let mut map = MapDefinition::default();
        for (row_index, row_text) in source.lines().enumerate() {
            let row_text = row_text.trim_end_matches('\r');
            let mut row = Vec::with_capacity(row_text.len());
            let world_row = row_index as i32;

            for (column_index, symbol) in row_text.chars().enumerate() {
                let column = column_index as i32;

                let cell = match symbol {
                    '#' => MapCell::Wall,
                    '~' => MapCell::Water,
                    'P' => {
                        map.spawns
                            .entry("Player".to_string())
                            .or_insert((column, world_row));
                        MapCell::Floor
                    }
                    'N' => {
                        map.spawns
                            .entry("Merchant".to_string())
                            .or_insert((column, world_row));
                        MapCell::Floor
                    }
                    'G' => {
                        map.spawns
                            .entry("Goblin".to_string())
                            .or_insert((column, world_row));
                        MapCell::Floor
                    }
                    _ => MapCell::Floor,
                };

                row.push(cell);
            }

            if !row.is_empty() {
                map.rows.push(row);
            }
        }

        if !map.spawns.contains_key("Player") {
            map.spawns.insert("Player".to_string(), (1, 1));
        }

        if !map.spawns.contains_key("Merchant") {
            map.spawns
                .insert("Merchant".to_string(), (2, map.height() - 2));
        }

        if !map.spawns.contains_key("Goblin") {
            map.spawns
                .insert("Goblin".to_string(), (map.width() - 2, map.height() - 2));
        }

        map
    }

    fn ensure_tiles(context: &mut StartupContext, map: &MapDefinition) -> GameResult<()> {
        for (y, row) in map.rows.iter().enumerate() {
            for (x, cell) in row.iter().enumerate() {
                let world_pos = map.tile_position_to_world(x as i32, y as i32);
                let mut sprite = context
                    .sprite("asset://sprites/tile.png")
                    .position(world_pos.x, world_pos.y)
                    .size(TILE_SIZE, TILE_SIZE);

                sprite = match cell {
                    MapCell::Wall => sprite.rgba_tint(0.2, 0.2, 0.24, 1.0),
                    MapCell::Water => sprite.rgba_tint(0.06, 0.37, 0.64, 1.0),
                    MapCell::Floor => sprite.rgba_tint(0.23, 0.46, 0.25, 1.0),
                };

                let entity = sprite.spawn()?;

                let record = context
                    .world()
                    .entity_mut(entity)
                    .ok_or("spawned tile entity missing from world")?;
                record.name = Some(format!("Tile{y}_{x}"));
                record.tags.push("tile".to_string());

                if matches!(cell, MapCell::Wall | MapCell::Water) {
                    record.tags.push("blocked".to_string());
                    record
                        .data_refs
                        .insert("blocked".to_string(), "true".to_string());
                }
            }
        }

        Ok(())
    }

    fn sync_spawned_positions(&mut self, context: &mut FrameContext<'_>) -> GameResult<()> {
        for (name, (column, row)) in &self.map.spawns {
            let target = self.map.tile_position_to_world(*column, *row);

            if let Some(entity) = context.world().entity_by_name(name) {
                context.world().set_position(entity, target.x, target.y);

                if name == "Player" {
                    self.player_entity = Some(entity);
                }
            }
        }

        if self.player_entity.is_none() {
            self.player_entity = context.world().entity_by_name("Player");
        }

        self.initialized = true;
        Ok(())
    }

    fn player_entity(&mut self, context: &mut FrameContext<'_>) -> Option<Entity> {
        if self.player_entity.is_none() {
            self.player_entity = context.world().entity_by_name("Player");
        }

        self.player_entity
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
        let dt = frame.delta_seconds() * self.player_speed;
        let candidate_x = current.x + normalized.x * dt;
        let candidate_y = current.y + normalized.y * dt;
        let mut final_position = current;

        if self.map.is_walkable(candidate_x, current.y) {
            final_position.x = candidate_x;
        }
        if self.map.is_walkable(final_position.x, candidate_y) {
            final_position.y = candidate_y;
        }

        if (final_position.x - current.x).abs() > f32::EPSILON
            || (final_position.y - current.y).abs() > f32::EPSILON
        {
            frame
                .world()
                .set_position(entity, final_position.x, final_position.y);

            self.try_footstep(frame, entity)?;
        }

        self.camera_target = final_position;
        Ok(())
    }

    fn closest_character(&self, world: &World, from: Vec2) -> Option<Entity> {
        let max_distance = INTERACT_DISTANCE * INTERACT_DISTANCE;
        let mut nearest: Option<(Entity, f32)> = None;

        for entity in world.entities_with_tag("character") {
            if let Some(transform) = world.transform(entity) {
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
        }

        nearest.map(|(entity, _)| entity)
    }

    fn open_character_dialogue(
        &self,
        context: &mut FrameContext<'_>,
        character_entity: Entity,
    ) -> GameResult<()> {
        let character_path = {
            let world = context.world();
            world
                .data_ref(character_entity, "character")
                .map(ToString::to_string)
        };

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
        let _ = context.play_entity_audio(character_entity);

        Ok(())
    }

    fn try_footstep(&mut self, frame: &mut FrameContext<'_>, entity: Entity) -> GameResult<()> {
        if self.step_sound_cooldown > 0.0 {
            return Ok(());
        }

        let _ = frame.play_entity_audio(entity);
        self.step_sound_cooldown = STEP_SOUND_COOLDOWN_SECONDS;
        Ok(())
    }

    fn update_camera(&mut self, frame: &mut FrameContext<'_>, player_entity: Entity) {
        if let Some(transform) = frame.world().transform(player_entity) {
            self.camera_target = Vec2::new(transform.x, transform.y);
        }
    }
}

impl Game2D for Game {
    fn new(context: &mut StartupContext) -> GameResult<Self> {
        context
            .components()
            .register_factory("WandererController", |config| {
                let speed = config
                    .get("speed")
                    .and_then(|value| value.as_float())
                    .unwrap_or(WandererController::DEFAULT_SPEED as f64)
                    as f32;

                Ok(Box::new(WandererController::new(speed)))
            })?;

        let map = Self::parse_map(MINI_RPG_MAP);
        Self::ensure_tiles(context, &map)?;

        Ok(Self {
            map,
            player_entity: None,
            initialized: false,
            step_sound_cooldown: 0.0,
            player_speed: DEFAULT_PLAYER_SPEED,
            camera_target: Vec2::ZERO,
        })
    }

    fn update(&mut self, context: &mut FrameContext<'_>) -> GameResult<()> {
        if self.step_sound_cooldown > 0.0 {
            self.step_sound_cooldown -= context.delta_seconds();
        }

        if !self.initialized {
            self.sync_spawned_positions(context)?;
        }

        let Some(player_entity) = self.player_entity(context) else {
            return Ok(());
        };

        if context.dialogue().is_active() {
            if context.input().just_pressed("interact") {
                context.dialogue().advance_or_close();
            }

            self.update_camera(context, player_entity);
            return Ok(());
        }

        let movement = context.input().axis2d("move");
        if movement.x != 0.0 || movement.y != 0.0 {
            self.attempt_move(context, player_entity, movement)?;
        } else {
            self.update_camera(context, player_entity);
        }

        if context.input().just_pressed("interact") {
            let player_position = {
                context
                    .world()
                    .transform(player_entity)
                    .map(|transform| Vec2::new(transform.x, transform.y))
                    .unwrap_or(Vec2::ZERO)
            };

            let target = {
                let world = context.world();
                self.closest_character(&world, player_position)
            };

            if let Some(target) = target {
                self.open_character_dialogue(context, target)?;
            }
        }

        self.update_camera(context, player_entity);
        Ok(())
    }

    fn render(&self, context: &mut RenderContext) {
        context.camera(Camera2D {
            x: self.camera_target.x,
            y: self.camera_target.y,
            zoom: 1.0,
        });
    }
}

seishin::seishin_main!(Game);
