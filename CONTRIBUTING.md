# Contributing

Thanks for your interest in `seishin`.

This project is early and intentionally MVP-focused. Prefer small, validated changes over broad abstractions.

## Local Setup

```sh
cargo build --workspace
cargo test --workspace --all-targets
```

## Required Checks

Before opening a pull request, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace
```

If you touch windowing, rendering, input, assets, or audio, also run:

```sh
cargo run -p seishin_basic_2d
```

Record what you manually verified.

## Architecture Rules

- Keep backend dependencies private to subsystem crates.
- Do not expose `wgpu`, `winit`, `kira`, `image`, or future ECS internals in gameplay APIs.
- FFI must use opaque handles, `#[repr(C)]` values, explicit ownership, and error/status codes.
- Do not add large framework-scale architecture before there is an MVP need.
- Update README, examples, or tests when behavior changes.

## Pull Requests

- Keep PRs focused.
- Include validation output.
- Add tests for pure logic changes.
- Include manual notes for visual/audio changes.
