# ADR 0006: Layered Gameplay API for Better Developer Experience

## Status

Accepted.

## Context

The current `seishin` facade proves the MVP runtime, asset loading, rendering, input, and audio path, but the developer experience still feels closer to a low-level graphics library than to a game engine.

The first playable example currently asks game code to manage details that should not be required for an introductory engine workflow:

- `asset_root` configuration in code;
- direct `load_texture` and `load_sound` calls;
- audio fallback handling in game code;
- separate `Texture` and `Sprite` fields;
- direct mutation of sprite position;
- a manual `render` method;
- per-frame `clear`, `camera`, `texture`, and `sprite` calls;
- `tracing_subscriber` setup boilerplate;
- loose string paths without a project-aware resource scheme;
- hardcoded `KeyCode` usage for gameplay input.

The current API shape makes users write rendering orchestration code instead of gameplay code:

```rust
fn update(&mut self, ctx: &mut FrameContext<'_>) -> GameResult<()> {
    self.player.transform.x += movement_x;
}

fn render(&self, ctx: &mut RenderContext) {
    ctx.clear(ClearColor::CORNFLOWER);
    ctx.camera(self.camera);
    ctx.texture(&self.player_texture);
    ctx.sprite(self.player);
}
```

This low-level API is still useful, but it should be treated as an advanced layer rather than the default first experience.

## Decision

`seishin` will expose layered gameplay APIs. Lower layers remain available, but examples and documentation should prefer the highest layer that fits the task.

### Layer 1: Low-Level Runtime And Rendering API

This layer keeps the current explicit rendering model for advanced users, experiments, tests, and engine development:

- `RenderContext`;
- `Sprite`;
- `Texture`;
- `Camera2D`;
- `ClearColor`;
- manual render submission.

This layer remains public, but it is not the preferred introductory API.

### Layer 2: Game Object API

This layer introduces engine-owned gameplay concepts while keeping backend internals private:

- `Entity` handles;
- `Transform` or `Transform2D` components;
- `SpriteRenderer` components;
- `AudioSource` components;
- `InputAction` bindings;
- `World` accessors and commands;
- `SpriteBundle`, `CameraBundle`, and `AudioBundle`;
- builder APIs for common objects.

The recommended beginner-facing sprite API should be a builder:

```rust
let player = ctx
    .sprite("asset://sprites/player.png")
    .position(0.0, 0.0)
    .size(96.0, 96.0)
    .spawn()?;
```

Structured bundle APIs should also exist for reusable or generated content:

```rust
let player = ctx.spawn(SpriteBundle {
    texture: ctx.assets().texture("asset://sprites/player.png")?,
    transform: Transform2D::from_xy(0.0, 0.0),
    size: Vec2::splat(96.0),
    ..default()
})?;
```

The first examples should prefer builders because they reduce ceremony and make intent clear. Bundles remain useful for tests, tools, scene loading, and more explicit gameplay code.

### Layer 3: Scene, Prefab, And Project API

This layer introduces project-level conventions and file-backed configuration:

- `Seishin.toml` project files;
- `asset://` paths for media assets;
- `res://` paths for resources such as scenes, prefabs, configuration, and data files;
- reserved `user://` paths for writable user data;
- scene files;
- prefab definitions;
- resources;
- future script/component registration;
- future CLI templates and `seishin run` workflows.

The scene and prefab formats are not decided by this ADR. They are future-facing direction, not near-term implementation requirements.

## Project File

Game code should not need to hardcode window, asset root, or default logging configuration in the first example.

Instead of:

```rust
App::new("seishin basic 2d")
    .window_size(960, 540)
    .asset_root(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
    .run::<Basic2D>()
```

The preferred project layout should support:

```txt
basic_2d/
  Seishin.toml
  resources/
    scenes/
      main.scene.toml
  assets/
  src/
  Cargo.toml
```

Example `Seishin.toml`:

```toml
[game]
name = "seishin basic 2d"
main_scene = "res://scenes/main.scene.toml"

[window]
width = 960
height = 540
clear_color = "cornflower"

[assets]
root = "assets"

[resources]
root = "resources"

[logging]
default_filter = "info"

[input.actions.move]
type = "axis2d"
left = ["ArrowLeft", "KeyA"]
right = ["ArrowRight", "KeyD"]
up = ["ArrowUp", "KeyW"]
down = ["ArrowDown", "KeyS"]

[input.actions.interact]
type = "button"
keys = ["Space", "Enter"]
```

The application entry point should be able to load explicit project configuration:

```rust
fn main() -> GameResult<()> {
    App::from_project("Seishin.toml")?.run::<Basic2D>()
}
```

The engine should also provide a convention-based entry point that discovers the project file from the current package or working directory:

```rust
fn main() -> GameResult<()> {
    seishin::run::<Basic2D>()
}
```

## Logging

The first example should not manually initialize `tracing_subscriber`.

The engine should provide default logging through one of these paths:

```rust
App::new()
    .with_default_logging()
    .run::<Basic2D>()
```

or:

```rust
App::new()
    .log_level(LogLevel::Debug)
    .run::<Basic2D>()
```

or, preferably for the simplest project:

```rust
seishin::run::<Basic2D>()
```

Advanced users can still install their own tracing subscriber before starting the engine.

## Assets

Asset access should move from direct loader methods to a project-aware asset facade:

```rust
let player_texture = ctx.assets().texture("asset://sprites/player.png")?;
let beep = ctx.assets().sound("asset://audio/beep.wav")?;
```

The `asset://` scheme resolves relative to the project asset root declared in `Seishin.toml`. The `res://` scheme resolves relative to the project resource root. These schemes must not be mixed: images, audio, video, and fonts are assets; scenes, prefabs, configuration, scripts, markup, and data files are resources.

Asset errors must be actionable. A missing asset should report the requested path, resolved path, asset root, and suggested fixes:

```txt
Asset not found: asset://sprites/player.png

Looked in:
  /basic_2d/assets/sprites/player.png

Current asset root:
  /basic_2d/assets

Suggestions:
  - Check if the file exists.
  - Use an asset:// path relative to [assets].root.
  - Check Seishin.toml [assets].root.
```

## Input

`KeyCode` remains available for low-level input handling, but beginner-facing gameplay code should use named input actions:

```rust
let movement = ctx.input().axis2d("move");
```

Input actions should be configurable through project configuration or another asset-backed input map. The first implementation may support keyboard-only actions. Mouse, touch, and gamepad mappings can be added later without changing the gameplay API.

## Automatic Rendering

The engine should render entities that have renderable components, such as `Transform2D` plus `SpriteRenderer`, without requiring game code to implement a manual `render` method.

Target beginner-facing example:

```rust
use seishin::prelude::*;

mod components;

use components::PlayerController;

struct Game;

impl Game2D for Game {
    fn new(ctx: &mut StartupContext) -> GameResult<Self> {
        ctx.components()
            .register::<PlayerController>("PlayerController")?;

        Ok(Self)
    }
}

fn main() -> GameResult<()> {
    seishin::run::<Game>()
}
```

The code-first sprite builder remains available for tiny demos and tests, but the preferred real-game flow is component registration plus automatic scene loading from `Seishin.toml`.

The current manual render path remains valid for advanced usage:

```rust
fn render(&self, ctx: &mut RenderContext) {
    ctx.clear(ClearColor::CORNFLOWER);
    ctx.camera(self.camera);
    ctx.texture(&self.player_texture);
    ctx.sprite(self.player);
}
```

## CLI Direction

A CLI should eventually support project creation and running:

```sh
seishin new basic_2d --template 2d
cd basic_2d
seishin run
```

The CLI should generate at least:

- `Seishin.toml`;
- `Cargo.toml`;
- starter `src/main.rs`;
- `resources/`;
- `resources/scenes/main.scene.toml`;
- `assets/`;
- default input actions;
- a minimal sprite example.

This ADR does not require the CLI to be implemented before the layered API. It records CLI support as an important developer-experience direction.

## Implementation Order

The intended implementation order is:

1. `Seishin.toml` parsing and project discovery.
2. Default logging owned by `App` or `seishin::run`.
3. `res://` resource paths and improved asset diagnostics.
4. `ctx.assets()` facade.
5. `Entity` handles and a minimal world API.
6. `SpriteBundle` and sprite builder spawning.
7. Automatic rendering for sprite entities.
8. Basic input actions and `axis2d`.
9. Update examples to use the high-level API and keep `main.rs` bootstrap-oriented for scene-driven projects.
10. Add CLI templates after the project model stabilizes.

## Consequences

- The engine gains a larger public API surface.
- The introductory example becomes more engine-like and less graphics-library-like.
- The low-level API remains available as an escape hatch and for advanced rendering control.
- The runtime needs a render queue or world traversal for renderable entities.
- The asset system needs project-aware path resolution and stronger error messages.
- The input system needs named action mapping while preserving key-level access.
- If an ECS implementation is adopted later, it must remain internal and must not leak through FFI.
- Scene, prefab, editor, hot reload, and scripting designs remain deferred.

## Non-Goals

- Removing the current low-level rendering API.
- Committing to a specific internal ECS implementation.
- Defining the final scene or prefab file format.
- Building an editor as part of this decision.
- Adding hot reload or scripting as part of this decision.
- Implementing the CLI before the project and gameplay API stabilize.
