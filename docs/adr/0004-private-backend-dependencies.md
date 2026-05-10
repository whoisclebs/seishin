# ADR 0004: Backend Dependencies Remain Private

## Status

Accepted for MVP planning.

## Context

The preferred stack includes `winit`, `wgpu`, `image`, and `kira`. These libraries are implementation choices, not the engine's public identity.

## Decision

Backend dependencies stay private to their owning subsystem crates:

- `winit` behind runtime/platform integration;
- `wgpu` behind render implementation;
- `image` behind assets implementation;
- `kira` behind audio implementation;
- any future ECS backend behind engine-owned APIs.

## Consequences

- Public API uses engine-owned concepts such as `Engine`, `EntityId`, `Transform2D`, `Camera2D`, asset handles, input key codes, and audio commands.
- Examples and future bindings do not depend on backend-specific types.
- Subsystems need adapter code, but the engine can evolve internals without breaking user-facing contracts.
