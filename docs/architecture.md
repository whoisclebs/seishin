# Architecture

## Purpose

`seishin` is a native 2D game engine prototype with a Rust core and a stable interoperability boundary for future gameplay languages and tooling.

The project is authorial, modular, and pragmatic. The MVP should prove a small 2D vertical slice before introducing broad engine abstractions.

## MVP Contract

The MVP targets Windows and Linux desktop first.

### In Scope

- desktop window and lifecycle;
- update/render loop;
- normalized keyboard input;
- simple entity/transform model;
- 2D sprite rendering;
- simple 2D camera;
- asset loading from disk;
- simple audio for MVP completion unless explicitly deferred;
- one playable example: `examples/basic_2d`;
- stable public Rust API surface;
- future-safe C ABI / FFI preparation.

### First Playable Demo Definition

`examples/basic_2d` is the first playable demo. It should open a desktop window, clear the background, load a sprite through the asset subsystem, render it, move it with keyboard input, use or validate a simple camera, log useful lifecycle information, and shut down cleanly.

### Out of Scope

- Android runtime implementation;
- Go binding implementation;
- editor tooling;
- scripting;
- hot reload;
- multiplayer;
- complex UI;
- advanced physics;
- advanced rendering features;
- parity with existing engines.

Android and Go bindings remain future design constraints, not MVP implementation requirements.

## Reference Inputs

Internal planning notes are useful for architecture risks and vocabulary, including module composition, update ordering, renderer separation, asset safety, and platform input concerns.

They are not a parity mandate. `seishin` should not import large framework-scale complexity into the MVP unless a specific MVP requirement justifies it.

## Public Boundary Model

```txt
Game Code
  -> Optional Language Binding
  -> C ABI / FFI
  -> Public Engine API
  -> Rust Engine Internals
```

Only the Rust engine internals own runtime state, rendering, assets, audio, physics, input, and lifecycle implementation details.

Bindings and examples use stable public API types, opaque handles, IDs, and C-compatible values. They must not depend on internal Rust structs.

## Crate Responsibilities

### `seishin`

Owns the ergonomic facade for applications and examples:

- reexports subsystem crates under domain modules;
- exposes `seishin::prelude::*` for common gameplay/demo types;
- keeps backend internals hidden behind subsystem public APIs.

This crate is the preferred dependency for game code. Subsystem crates remain available for advanced users and internal composition.

### `seishin_core`

Owns stable engine-domain primitives and safe public API concepts:

- `EngineConfig`;
- `Engine`;
- lifecycle/update context;
- future entity IDs and transforms;
- errors and result types.

`seishin_core` must stay backend-agnostic. It must not depend on `winit`, `wgpu`, `kira`, `image`, or platform-specific crates.

### `seishin_world`

Owns backend-free data-driven world concepts:

- entity records and deterministic entity ID allocation;
- scene and prefab document types;
- pure scene/prefab merge behavior;
- loaded scene behavior that preserves explicit entity IDs;
- scene instancing behavior that creates fresh entity IDs and records source links.
- ECS-driven UI records for layout, text, image, and interaction data;
- procedural scene document builders that emit the same format as file-backed scenes;
- explicit scene reload queues that apply full scene updates or diffs without owning file watching.

`seishin_world` may depend on `seishin_core`, `serde`, and `toml` while TOML remains the project format. It must not depend on `winit`, `wgpu`, `kira`, `image`, `web-sys`, `wasm-bindgen`, renderer internals, audio internals, or runtime crates.

### `seishin_runtime`

Owns application lifecycle orchestration:

- fixed-step/headless loops for tests and examples;
- future `winit` desktop event loop integration;
- coordination between update, input, render, assets, and audio.

Runtime composes subsystems. It should not absorb renderer, asset loader, or audio backend internals.

### `seishin_render`

Owns render-facing engine concepts and the future `wgpu` implementation:

- clear colors;
- 2D camera state;
- future sprites, textures, pipelines, and batching.

`wgpu` types must remain private implementation details. Public API should expose engine concepts such as `Camera2D`, `Sprite`, texture handles, and render configuration.

The `wgpu` backend is behind the `wgpu-backend` feature. Data-only render types can compile without `wgpu`, `raw-window-handle`, or `bytemuck`.

### `seishin_render_graph`

Owns backend-free render graph structure:

- retained node labels;
- node edges for execution ordering;
- deterministic topological execution plans;
- controlled errors for missing nodes, duplicate nodes/edges, missing edges, and cycles.

This crate does not execute GPU work. It is the ordering layer that future 2D, 3D, isometric, UI, and post-processing passes can plug into without coupling gameplay/world data directly to a renderer backend. The facade currently uses a small default graph for frame reset, world extraction, UI extraction, and user render ordering.

### `seishin_input`

Owns normalized input state independent from window backends:

- engine key codes;
- pressed/released state;
- future `just_pressed` / `just_released` transitions;
- future mouse, touch, and gamepad abstractions.

`winit` event types must not leak into public input APIs.

### `seishin_assets`

Owns stable asset identifiers and loading policy:

- asset paths;
- typed handles;
- future loaders, caches, and manifests.

The MVP asset loader should enforce an approved asset root, reject path traversal, and return controlled errors for missing or invalid files.

### `seishin_audio`

Owns the audio facade:

- load/play commands;
- future `kira` integration;
- sound/music handles without leaking backend details.

No audio backend type should appear in public engine API or FFI.

### `seishin_physics`

Reserved for simple 2D collision and future physics integration.

Physics is outside the first playable demo unless the demo explicitly needs basic collision.

### `seishin_ffi`

Owns the C ABI boundary:

- opaque `SeishinEngine` handle;
- `#[repr(C)]` config/status values;
- null-safe create/destroy/tick/frame functions;
- panic containment.

This crate is the only boundary future Go bindings should call through cgo or generated bindings.

## Dependency Direction Rules

Preferred direction:

```txt
examples/*
  -> public engine crates

seishin_ffi
  -> safe public Rust API

seishin
  -> seishin_world
  -> seishin_core

seishin_runtime
  -> seishin_core
  -> subsystem public APIs

subsystem crates
  -> seishin_core only when shared engine-domain types are needed

seishin_core
  -> no backend crates
```

Rules:

- Avoid dependency cycles.
- Subsystem crates should not depend on `seishin_runtime`.
- `seishin_ffi` must not depend directly on backend crates.
- Examples should prefer public API and must not reach into private implementation modules.
- If a facade crate is added later, it should be deliberate and documented.

## Feature Boundaries

The default facade features remain practical for the playable demo, but subsystem crates support lighter builds:

- `seishin/default`: demo-oriented facade with desktop/web/render/audio/png/logging enabled;
- `seishin/default-features = false`: facade API without runtime backends, renderer backend, PNG decoding, Kira audio, or tracing subscriber;
- `seishin_render/wgpu-backend`: enables `wgpu`, `raw-window-handle`, and `bytemuck`;
- `seishin_runtime/desktop`: enables `winit`, `pollster`, and the `wgpu` render backend;
- `seishin_runtime/web`: enables web runtime dependencies and the `wgpu` render backend;
- `seishin_assets/png`: enables PNG decoding through `image`;
- `seishin_assets/web`: enables web asset preload support;
- `seishin_audio/kira-backend`: enables native Kira audio playback;
- `seishin/logging`: enables `tracing-subscriber`.

Core/data-only crates such as `seishin_core`, `seishin_world`, and `seishin_render_graph` must stay backend-free.

## Backend Privacy Rules

- `winit` belongs behind runtime/platform integration.
- `wgpu` belongs behind render implementation.
- `image` belongs behind assets implementation.
- `kira` belongs behind audio implementation.
- any future ECS backend must stay internal and must not cross FFI.

## ECS Position

The immediate MVP should defer external ECS backends unless implementation proves a strong need.

The first vertical slice should use simple engine-owned entities, transforms, and commands. This keeps the public API small and prevents early coupling to ECS internals. A post-MVP ADR can revisit the storage model if gameplay complexity, reusable systems, query ergonomics, or editor/tooling requirements justify it.

## FFI Boundary Rules

Allowed across FFI:

- opaque handles;
- integer IDs;
- `#[repr(C)]` structs with primitive fields;
- explicit status/error codes;
- caller-owned output pointers with null checks;
- create/destroy ownership pairs.

Forbidden across FFI:

- Rust references;
- lifetimes;
- traits;
- generics;
- `Vec<T>`;
- `HashMap`;
- Rust `String`;
- closures;
- ECS internals;
- backend types such as `winit`, `wgpu`, `image`, or `kira`.

The FFI grows only after the equivalent safe Rust API behavior is implemented, tested, and considered stable enough to wrap.

## MVP Validation

Automated baseline:

```sh
cargo test --workspace --all-targets
cargo build --workspace
```

Recommended before closing substantial work:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- check
```

Manual final demo checklist:

- `examples/basic_2d` opens a native window;
- clear background is visible;
- sprite is visible;
- keyboard input moves the sprite;
- release stops movement as expected;
- missing asset behavior is controlled;
- audio plays or is explicitly skipped/deferred with rationale;
- window close and/or `Escape` exits cleanly.

## Initial Roadmap

1. Keep the workspace split aligned with engine subsystems.
2. Document the MVP contract and ADRs.
3. Strengthen pure logic tests for core/runtime/input/assets/FFI.
4. Add a simple internal entity/transform model.
5. Add a desktop runtime using `winit` that opens a window and drives update/render.
6. Add a `wgpu` renderer that clears the window surface.
7. Connect normalized keyboard input from `winit` into `seishin_input`.
8. Render a movable sprite loaded from disk.
9. Add a simple 2D camera and audio facade/playback.
10. Extend the C ABI only after the safe Rust API stabilizes.

## Open Contract Questions

- Should simple audio remain a hard MVP completion requirement, or be explicitly deferred after the first playable visual/input demo?
- Which concrete image/audio assets should be committed under `examples/basic_2d/assets/`?
- What level of Linux manual validation evidence is required before MVP closure?
