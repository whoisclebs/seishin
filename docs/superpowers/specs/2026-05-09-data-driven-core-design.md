# Data-Driven Core Design

## Status

Accepted for planning.

## Context

`seishin` has moved past the first 2D MVP. The current workspace already includes:

- a facade crate named `seishin`;
- backend-facing crates for runtime, render, assets, audio, input, physics, and FFI;
- desktop and web runtime support;
- TOML-backed `Seishin.toml`, scenes, prefabs, resource files, and input actions;
- a small engine-owned `World` in `crates/seishin/src/app.rs`.

The next direction is data-driven development with a smaller dependency surface. The immediate goal is not to add every large-engine feature. The immediate goal is to extract a lightweight, backend-free data model that can later support render graph, UI, hot reload, procedural generation, save games, and cross-platform backends without forcing `wgpu`, `winit`, `kira`, `image`, or web crates into core gameplay code.

## Reference Boundary

Architecture notes are references, not parity targets.

Use these project-owned ideas:

- Data first: entity/component data should drive rendering, audio, UI, and gameplay behavior.
- Modular by default: users should be able to use only the pieces they need.
- Scenes as world data: loading a scene should preserve entity IDs, while instancing should allocate fresh IDs.
- Render graph shape: rendering should evolve toward nodes and edges rather than a monolithic renderer method.
- ECS-driven UI direction: UI should eventually be world data, not a separate immediate-mode island.
- Productive compile times: the lightest path should avoid backend dependencies.

Do not add these choices now:

- no external ECS dependency in the first data-driven core slice;
- no archetype storage, scheduler, reflection system, asset server, or plugin framework yet;
- no render graph implementation until the world/scene extraction is stable;
- no editor or full hot reload pipeline in this slice.

## Goals

- Move the persistent entity/world model out of `crates/seishin/src/app.rs`.
- Keep the new world/data crate backend-free.
- Preserve the existing scene and prefab behavior in `examples/basic_2d`.
- Add a clear distinction between scene loading and scene instancing.
- Support deterministic entity IDs for loaded scenes.
- Keep instanced scenes linked to their source while giving them new entity IDs.
- Make dependency weight visible and optional where practical.
- Keep public gameplay APIs stable enough for the current example to keep working.

## Non-Goals

- Building a full ECS.
- Adding an external ECS backend.
- Replacing TOML immediately.
- Replacing `wgpu`, `winit`, `kira`, or `image` immediately.
- Implementing render graph.
- Implementing UI.
- Implementing file watching hot reload.
- Implementing Android or iOS runtime support.
- Adding procedural generation beyond designing where it will plug in.

## First Slice

Create a small data-driven crate, preferably `crates/seishin_world`, for world, scene, prefab, and instance data.

Responsibilities:

- `EntityId`: stable integer entity handle with raw round-trip support.
- `World`: owns entity records and deterministic ID allocation.
- `EntityRecord`: stores built-in data only.
- Built-in data:
  - `Name`;
  - `Tags`;
  - `Transform2D`;
  - `SpriteRef`;
  - `AudioRef`;
  - `DataRefs`;
  - `CustomComponentRef`;
  - `InstanceSource`.
- `SceneDocument`: serializable/deserializable representation of scene files.
- `PrefabDocument`: serializable/deserializable representation of prefab files.
- `SceneLoader`: pure merge/load logic that does not touch GPU, audio, window, or file watching APIs.
- `SceneInstance`: result mapping source IDs to new instance IDs.

The facade crate can continue to expose `World`, `Entity`, and scene-driven APIs through `seishin::prelude::*`, but storage and pure scene logic should live in the new backend-free crate.

## Entity IDs

The first implementation should keep IDs simple and explicit:

```text
EntityId = u64
World.next_entity = next available u64
```

Loaded scene behavior:

- if a scene file provides IDs, the world inserts those exact IDs;
- if a scene file omits IDs, the loader allocates deterministic new IDs;
- `next_entity` advances past the highest inserted ID;
- duplicate IDs are a hard load error.

Instancing behavior:

- instancing never reuses source entity IDs;
- the instance receives fresh IDs from the target world;
- each new entity records `InstanceSource { scene, source_entity }`;
- the return value includes a source-to-instance mapping.

This is intentionally simpler than a generational entity model. Generations can be added later if despawn/reuse bugs become real.

## Scene Format

Keep TOML in the first slice because it is already present and working. Do not expand the dependency surface to support more formats yet.

Allow optional entity IDs:

```toml
[[entities]]
id = 1
name = "Player"
prefab = "res://prefabs/player.prefab.toml"

[entities.transform]
x = 0.0
y = 0.0

[entities.tags]
values = ["player", "controllable"]
```

The in-memory `SceneDocument` should not depend on `toml::Value` except where opaque custom component config is unavoidable. If practical, isolate raw document values behind a small type so a future custom parser or alternate format does not leak through the entire engine.

## Dependency Policy

The first slice must not add new third-party dependencies.

Target dependency direction:

```text
seishin
  -> seishin_world
  -> seishin_core

seishin_runtime
  -> seishin_core
  -> seishin_input
  -> seishin_render

seishin_render
  -> seishin_core

seishin_assets
  -> no engine backend crates

seishin_audio
  -> seishin_assets
```

The new world crate may use `serde` and `toml` only if that is the lowest-risk way to preserve current behavior. It must not depend on `wgpu`, `winit`, `kira`, `image`, `web-sys`, `wasm-bindgen`, `tracing-subscriber`, or runtime crates.

After extraction, add or document feature boundaries for heavy pieces:

- `desktop`: `winit`, raw window handles, desktop runtime;
- `web`: `web-sys`, `wasm-bindgen`, web runtime;
- `render-wgpu`: `wgpu`, `bytemuck`;
- `audio-kira`: `kira`;
- `png`: `image` PNG decoding;
- `logging`: `tracing-subscriber`.

The default feature set can remain practical for the demo, but there must be a documented light path for core/data-only builds.

## Runtime Integration

`crates/seishin/src/app.rs` should become thinner:

- keep `App`, `Game2D`, `StartupContext`, `FrameContext`, and high-level builders there;
- delegate world storage to `seishin_world`;
- delegate scene/prefab document merge logic to `seishin_world`;
- keep asset decoding and audio loading in their existing subsystem crates;
- translate `SpriteRef` from the world crate into renderable `SpriteRenderer` data at the facade/runtime boundary.

This keeps the data model portable while letting the current example keep using `seishin::run::<Game>()`.

## Future Render Graph Direction

The future render graph should stay small and data-driven:

- retained graph;
- node labels;
- node edges for order;
- optional slots/resources later;
- graph execution after world extraction and render preparation.

The first implementation does not build the graph. It should only make the world/render boundary clean enough that a later graph can consume render data without reading gameplay internals directly.

## Future UI Direction

UI should be represented as world data:

- entities with UI layout, text, image, and interaction components;
- layout and input systems that read/write world data;
- rendering through the same future render graph path.

The first slice should not add UI types unless needed to avoid painting the architecture into a corner.

## Future Hot Reload Direction

Hot reload should be a resource reload and diff/apply problem, not a direct file watcher embedded in core:

- core world crate exposes apply operations;
- asset/resource layer detects changed scene files;
- runtime queues reload requests;
- world applies changes while preserving loaded entity IDs where possible.

No file watcher dependency should be added in the first slice.

## Future Procedural Generation Direction

Procedural generation should create the same `SceneDocument` or spawn-command data used by file-backed scenes.

This keeps generated content, loaded content, and instanced content on one path:

```text
procedural generator -> SceneDocument -> load or instantiate -> World
scene file           -> SceneDocument -> load or instantiate -> World
```

No RNG or noise dependency should be added in this slice.

## Testing Strategy

Add focused pure tests around `seishin_world`:

- loaded scene preserves explicit IDs;
- missing IDs are allocated deterministically;
- duplicate explicit IDs fail;
- instancing creates fresh IDs;
- instancing records source links;
- prefab merge preserves current override behavior;
- tag/name queries stay deterministic;
- current `examples/basic_2d` scene still produces the same visible entity set.

Run at minimum:

```sh
cargo test -p seishin_world
cargo test -p seishin
cargo test --workspace --all-targets
```

Use full `xtask` validation before merging implementation work.

## Implementation Decisions

- Keep `EntityId` in `seishin_core`; `seishin_world` depends on `seishin_core`.
- Keep `Transform2D` in `seishin_core` as a shared math primitive.
- Keep opaque custom component config as `toml::Value` in this slice, but isolate it to scene/prefab document types and custom component refs.
- Implement the world extraction before broad feature-flag work.
- In the same implementation plan, add a dependency audit and document the intended feature map.
- If the extraction exposes a low-risk direct win, make `tracing-subscriber` optional behind `logging`; otherwise leave dependency reduction to the next plan.
