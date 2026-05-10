# Go Binding Plan

Go gameplay support is planned for the future and is not part of the MVP implementation.

Future Go support should be layered above `seishin_ffi`:

```txt
Go Gameplay
  -> cgo or generated binding layer
  -> C ABI / FFI
  -> safe public Rust engine API
  -> Rust internals
```

## MVP Status

- No Go binding is implemented for the MVP.
- This directory is planning documentation only.
- Go work depends on a stabilized `seishin_ffi` contract.
- The Rust MVP should prove the safe public API before the C ABI grows wider.

## Boundary Rules

- Go code calls only the C ABI surface.
- Rust internals remain opaque.
- Engine-owned objects are represented as handles on the Go side.
- Ownership is explicit: every create function must have a matching destroy/release function.
- Cross-language callbacks must be added deliberately after the lifecycle API is stable.
- Go bindings must not depend on Rust structs, `Vec<T>`, `HashMap`, `String`, traits, generics, lifetimes, ECS internals, or backend types.

## First Future Binding Target

The first future Go wrapper should only cover stable lifecycle functions:

- wrap `seishin_engine_create`;
- wrap `seishin_engine_tick`;
- wrap `seishin_engine_frame`;
- wrap `seishin_engine_destroy`.

Additional functions should be added only after the equivalent safe Rust API behavior is implemented, tested, and intentionally exposed through `seishin_ffi`.
