<a id="readme-top"></a>

<br />
<div align="center">
  <a href="https://github.com/seishin/seishin">
    <img src=".github/assets/seishin.png" alt="seishin logo" width="128" height="128">
  </a>

  <h1 align="center">seishin</h1>

  <p align="center">
    A small Rust-first 2D game engine prototype with desktop rendering, assets, audio, and a future-safe FFI boundary.
    <br />
    <a href="docs/architecture.md"><strong>Explore the architecture »</strong></a>
    <br />
    <br />
    <a href="examples/basic_2d">View Example</a>
    &middot;
    <a href="https://github.com/seishin/seishin/issues/new?template=bug_report.yml">Report Bug</a>
    &middot;
    <a href="https://github.com/seishin/seishin/issues/new?template=feature_request.yml">Request Feature</a>
  </p>
</div>

`seishin` is a native 2D game engine prototype written in Rust. The project starts with a small desktop MVP and is designed around a stable public API boundary so future language bindings can call into the engine through C ABI / FFI instead of touching Rust internals.

The current MVP opens a desktop window or browser canvas, renders a sprite, loads assets, handles keyboard input, and exposes a small future-safe FFI lifecycle boundary.

## Table of Contents

- [About](#about)
- [Current Status](#current-status)
- [Built With](#built-with)
- [Getting Started](#getting-started)
- [Usage](#usage)
- [Project Layout](#project-layout)
- [Architecture](#architecture)
- [Development](#development)
- [Testing](#testing)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)

## About

`seishin` is a focused engine-learning project with a pragmatic architecture:

```txt
Rust Engine Core
  -> Stable public Rust API
  -> C ABI / FFI
  -> Future bindings
```

Design goals:

- keep gameplay code independent from renderer internals;
- keep backend crates such as `wgpu`, `winit`, `kira`, and `image` behind engine APIs;
- expose simple handles, IDs, and C-compatible values across FFI;
- avoid large framework-scale architecture until the MVP proves a real need;
- keep the developer experience pleasant through the `seishin` facade crate and `seishin::prelude::*`.

Initial targets:

- Windows desktop
- Linux desktop
- WebAssembly/browser MVP

Future targets:

- Android
- Go bindings through C ABI / FFI
- additional tooling after the engine core stabilizes

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Current Status

The repository currently contains the MVP vertical slice:

- desktop window through `winit`;
- update/render loop;
- keyboard input;
- `wgpu`-backed clear pass and sprite rendering;
- simple camera support;
- asset loading from disk;
- simple audio playback with graceful degradation;
- browser/WebAssembly build support with no-op audio fallback;
- playable `examples/basic_2d` example;
- C ABI lifecycle smoke boundary in `seishin_ffi`.

Manual visual/audio validation is still required on a desktop session after automated checks pass.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Built With

- [Rust](https://www.rust-lang.org/)
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [`winit`](https://crates.io/crates/winit) for desktop window and events
- [`wgpu`](https://crates.io/crates/wgpu) for GPU rendering
- [`image`](https://crates.io/crates/image) for image loading
- [`kira`](https://crates.io/crates/kira) for audio playback
- [`raw-window-handle`](https://crates.io/crates/raw-window-handle) for window/render integration
- [`bytemuck`](https://crates.io/crates/bytemuck) for safe GPU buffer casts

Planned or future-facing:

- a more complete ECS may be evaluated later, but it is intentionally deferred for the first MVP slice.
- a physics backend may be evaluated later for physics.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Getting Started

### Prerequisites

Install Rust with `rustup`:

```sh
rustup toolchain install stable
rustup default stable
```

This workspace currently uses:

- Rust edition: `2021`
- MSRV declared in Cargo: `1.75`

On Linux, `winit`, `wgpu`, and `kira` may require system packages for windowing, graphics, and audio depending on your distribution. Typical dependencies include X11/Wayland, Vulkan or GPU driver support, ALSA/PulseAudio/PipeWire development packages.

### Clone

```sh
git clone <repo-url>
cd seishin
```

### Build

```sh
cargo build --workspace
```

### Run The Demo

```sh
cargo run -p seishin_basic_2d
```

### Build The Web Demo

Install the wasm target and `wasm-bindgen` CLI, then export the example:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
cargo run -p xtask -- web-build --example basic_2d
```

The export is written to `target/web/basic_2d`. Serve it locally with:

```sh
cargo run -p xtask -- web-serve --example basic_2d
```

The web MVP keeps the same game entry point (`seishin::run::<Game>()`) but runs with browser-specific runtime internals. Audio is currently a no-op fallback on wasm; assets and resources are fetched from the static export using the existing `asset://` and `res://` schemes.

Controls:

- Arrow keys or WASD: move sprite through the configured `move` input action
- Space or Enter: open/close the MVP dialogue flow in logs
- Escape: close the demo

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Usage

Game/example code should normally depend on the facade crate and import the prelude:

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

Projects are configured by `Seishin.toml`. Virtual path schemes are explicit:

- `asset://` resolves under `[assets].root` for images, audio, video, fonts, and other media.
- `res://` resolves under `[resources].root` for scenes, prefabs, configuration, scripts, markup, and data files.
- `user://` is reserved for writable user data such as saves/settings.

For larger games, prefer scene/prefab/component composition over storing every entity handle in `main.rs`:

```txt
resources/scenes/   map and screen placement
resources/prefabs/  reusable entity composition
resources/data/     characters, items, quests, dialogue, and other game data
src/components/     Rust behavior registered by type name
```

`main.rs` can register custom behavior and load the scene:

```rust
ctx.components()
    .register::<PlayerController>("PlayerController")?;
```

`seishin::run::<Game>()` discovers `Seishin.toml`, lets `Game2D::new` register components, then automatically loads `[game].main_scene`.

Scenes can also carry audio and UI data. Audio refs are loaded as assets and can be played by entity from `FrameContext`; UI refs are backend-free ECS data for layout, text, image, and interaction. The facade extracts UI entities into `RenderContext::ui_elements()` and dispatches interaction actions through `FrameContext`, while visual text/UI drawing remains layered above the current renderer.

Components can load their own game data/configuration through the resource API:

```rust
let config = ctx
    .resources()
    .toml("res://data/components/player_controller.toml")?;
let speed = config.f32("speed").unwrap_or(180.0);
```

The current dialogue MVP opens/closes via the configured `interact` action and presents text through `tracing` logs. On-screen text/UI rendering is still a future rendering feature.

See the complete example in [`examples/basic_2d`](examples/basic_2d).

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Project Layout

```txt
crates/
  seishin/          Facade crate and gameplay prelude
  seishin_core/     Engine config, lifecycle, transforms, IDs, core errors
  seishin_world/    Backend-free world, scenes, UI data, reload, procedural builders
  seishin_runtime/  Headless and desktop runtime orchestration
  seishin_render/   2D render types and wgpu renderer
  seishin_render_graph/ Backend-free frame/render ordering graph
  seishin_input/    Normalized input state
  seishin_assets/   Asset paths, roots, handles, image loading
  seishin_audio/    Audio facade and private kira backend
  seishin_physics/  Placeholder for future 2D collision/physics
  seishin_ffi/      C ABI boundary with opaque handles
examples/
  minimal/            Headless loop smoke example
  basic_2d/           Playable MVP demo
bindings/
  go/                 Future Go binding notes
docs/
  architecture.md     Architecture notes and crate boundaries
  adr/                Architecture decision records
tools/
  xtask/              Internal automation helper
```

Each crate keeps `src/lib.rs` as a small facade and places implementation in domain modules such as `engine`, `desktop`, `renderer`, `loader`, `state`, or `ffi`.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Architecture

The engine is split into focused crates. The facade crate `seishin` is the preferred entry point for game code. Lower-level subsystem crates remain available for internal composition and advanced users.

Important rules:

- Gameplay code should not depend on `wgpu`, `winit`, `kira`, or `image` directly.
- FFI must not expose Rust references, generics, traits, lifetimes, `Vec<T>`, `HashMap`, `String`, backend types, or ECS internals.
- Android and Go are future-facing design constraints, not MVP implementation requirements.

More detail:

- [`docs/architecture.md`](docs/architecture.md)
- [`docs/adr/`](docs/adr)
- [`bindings/go/README.md`](bindings/go/README.md)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Development

Recommended workflow before submitting changes:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace
```

Useful commands:

```sh
cargo run -p seishin_basic_2d
cargo run -p xtask -- check
cargo run -p xtask -- web-build --example basic_2d
cargo run -p xtask -- web-serve --example basic_2d
cargo test -p seishin_core
cargo test -p seishin_render
```

Rust repository practices used here:

- workspace-level dependency declarations;
- `Cargo.lock` tracked for reproducible workspace/example builds;
- MSRV declared with `rust-version`;
- strict Clippy gate with `-D warnings`;
- `xtask` reserved for internal automation;
- crate-level tests for pure logic;
- manual checklist for render/audio behavior that cannot be fully unit-tested.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Testing

Automated baseline:

```sh
cargo test --workspace --all-targets
cargo build --workspace
```

Full local gate:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo build --workspace
cargo build --target wasm32-unknown-unknown -p seishin_basic_2d
```

Manual demo checklist:

- Run `cargo run -p seishin_basic_2d`.
- Confirm a desktop window opens.
- Confirm the sprite appears over a clear background.
- Confirm arrow keys and WASD move the sprite through the configured `move` input action.
- Press Escape or close the window and confirm clean shutdown.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Roadmap

The roadmap lives in [`docs/roadmap.md`](docs/roadmap.md) to keep this README focused on onboarding.

Current priorities:

- harden the MVP runtime/render loop;
- improve the high-level game API;
- expand tests around assets, rendering, and FFI safety;
- prepare future bindings without exposing Rust internals.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Contributing

This project is still early. Contributions should keep the MVP philosophy intact: small, explicit, validated changes.

Suggested flow:

1. Open an issue or discussion for larger changes.
2. Create a feature branch.
3. Keep PRs focused on one subsystem or vertical slice.
4. Run the full local gate before submitting.
5. Include manual validation notes when touching windowing, rendering, input, or audio.

Please avoid:

- exposing backend internals in public APIs;
- expanding FFI before the safe Rust API is stable;
- adding broad ECS/editor/hot-reload abstractions before they are required;
- mixing unrelated refactors and features in one change.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## License

Distributed under the MIT License. See [`LICENSE`](LICENSE) for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Acknowledgments

- The README structure is inspired by [Best-README-Template](https://github.com/othneildrew/Best-README-Template).
- Rust game development projects and crates such as `wgpu`, `winit`, `kira`, and `image` provide the foundation for this prototype.

<p align="right">(<a href="#readme-top">back to top</a>)</p>
