# ADR 0007: Scene, Prefab, and Component-Based Game Architecture

## Status

Accepted for incremental implementation.

## Context

The high-level sprite builder is pleasant for tiny examples, but real games should not require `main.rs` to store every player, NPC, enemy, chest, and UI entity as fields. Game projects need a scalable separation of concerns:

- scenes define placement;
- prefabs define reusable composition;
- data files define game-specific identity/content;
- assets hold static media;
- Rust components/systems hold behavior.

## Decision

`seishin` will support a scene/prefab/component architecture on top of the existing code-first API.

The code-first flow remains valid for tutorials and tiny games:

```rust
ctx.sprite("asset://sprites/player.png")
    .position(0.0, 0.0)
    .size(96.0, 96.0)
    .spawn()?;
```

For larger games, the recommended flow is:

```txt
main.rs      -> bootstrap and register components
scene files  -> map/screen placement and overrides
prefabs      -> reusable entity composition
data files   -> game-specific identity/content
assets       -> images, audio, fonts, video
components   -> Rust behavior
```

## Resource Layout

Projects should use:

```txt
resources/
  scenes/
  prefabs/
  data/
assets/
  sprites/
  tilesets/
  audio/
  fonts/
  videos/
```

Virtual schemes remain strict:

- `res://` resolves under `[resources].root`.
- `asset://` resolves under `[assets].root`.
- `user://` is reserved for writable user data.

## Minimal Implemented Slice

The first slice supports:

- scene files with multiple entities;
- scene entities referencing one prefab;
- shallow prefab + scene merge where scene values override prefab defaults;
- built-in `transform`, `sprite`, `name`, `tags`, and opaque `data` references;
- custom component type-name resolution through `ctx.components().register::<T>("Name")`;
- `Component2D` trait shape where update receives the owning `Entity`;
- automatic scene loading after `Game2D::new` so registration can happen before instantiation;
- automatic update of instantiated custom components;
- generic `ctx.resources().toml(...)` / `ctx.resources().load::<T>(...)` access so user components can load their own TOML/data;
- an MVP dialogue flow loaded from `resources/data` and presented through logs;
- world queries by name and tag;
- preservation of manual/code-first entity spawning.

Prefab inheritance chains, full ECS storage/querying for arbitrary custom component types, on-screen text/UI, hot reload, editor workflows, and scripting are deferred.

## Component Registration

Custom components are registered by string name before scene loading:

```rust
ctx.components()
    .register::<PlayerController>("PlayerController")?;
```

`seishin::run::<Game>()` calls `Game2D::new` first, then automatically loads `[game].main_scene`. Manual `ctx.load_main_scene()?` remains available and is idempotent, but it is not required for the default project flow.

Prefabs may reference custom components:

```toml
[components.controller]
type = "PlayerController"
speed = 180.0
```

The engine resolves `type = "PlayerController"` through the registry and instantiates the registered Rust component. Component-specific TOML semantics are owned by the component code, not by the engine. Components can load resources/config files from `FrameContext`:

```rust
let config = ctx
    .resources()
    .toml("res://data/components/player_controller.toml")?;
let speed = config.f32("speed").unwrap_or(180.0);
```

This keeps `main.rs` small and keeps game-specific config interpretation inside game-specific components.

## Consequences

- `main.rs` can become bootstrap-oriented instead of owning every entity handle.
- Scenes and prefabs become the preferred architecture for real games.
- The engine still supports explicit Rust code-first spawning for simple examples.
- Component-specific configuration remains explicit Rust code; the engine only provides resource loading and lifecycle wiring.
- Dialogue currently proves the resource/interact/runtime flow through logging; visual dialogue boxes require text/UI rendering work.
- TOML scene/prefab formats are provisional and should evolve carefully.
- A future ECS/scheduler can be introduced behind the public API without exposing backend internals.

## Non-Goals

- Full ECS implementation.
- Editor tooling.
- Hot reload.
- Scripting.
- CLI template generation.
- Deep prefab inheritance or complex merge policy.
