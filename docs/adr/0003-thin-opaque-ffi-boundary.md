# ADR 0003: Thin Opaque FFI Boundary

## Status

Accepted for MVP planning.

## Context

Future bindings, especially Go, should access the engine through a C ABI / FFI layer. Exposing Rust internals would create safety and compatibility problems.

## Decision

`seishin_ffi` remains a thin, opaque, command-oriented wrapper over the safe public Rust API.

Allowed FFI surface:

- opaque handles;
- `#[repr(C)]` primitive structs/enums;
- explicit ownership functions;
- status/error codes;
- panic containment;
- null pointer validation.

Forbidden FFI surface:

- Rust references;
- traits;
- generics;
- lifetimes;
- `Vec<T>`;
- `HashMap`;
- Rust `String`;
- closures;
- ECS internals;
- backend types such as `winit`, `wgpu`, `image`, or `kira`.

## Consequences

- FFI grows only after safe Rust behavior is implemented and tested.
- Go bindings remain future work and should wrap the C ABI rather than Rust internals.
- ABI stability is protected at the cost of slower FFI expansion.
