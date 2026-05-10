# Roadmap

This roadmap tracks the intended direction for `seishin`. It is not a release promise. Priorities may change as the engine architecture is validated through examples and real usage.

## MVP Completed

- [x] Rust workspace with modular engine crates.
- [x] Facade crate with `seishin::prelude::*`.
- [x] Desktop window and event loop.
- [x] Basic input state with pressed and transition semantics.
- [x] Asset root/path handling and image loading.
- [x] Minimal `wgpu` renderer.
- [x] Sprite rendering and camera math.
- [x] Simple audio playback with graceful degradation.
- [x] Playable `examples/basic_2d` MVP example.
- [x] C ABI lifecycle smoke boundary.
- [x] Backend-free data-driven world crate for scene records, ID-preserving loads, and instancing.
- [x] Backend-free render graph crate with retained nodes, edges, deterministic execution ordering, and cycle detection.
- [x] Render graph wired into the facade frame path for reset, world extraction, and user render ordering.
- [x] Scene export and deterministic scene diff/apply APIs for save-game and hot-reload preparation.
- [x] Explicit scene reload queue for applying whole-scene updates or diffs without a watcher.
- [x] Backend-free ECS/UI data records for layout, text, image, and interaction.
- [x] Facade UI extraction and interaction action dispatch from ECS world data.
- [x] Procedural scene document builder that emits the same data format as file-backed scenes.
- [x] Scene audio references loaded as assets and playable on demand from entity context.
- [x] Lightweight feature boundaries for renderer/runtime/assets/audio/logging backends.
- [x] Repository hygiene: README, license, contribution docs, issue/PR templates, CI, Dependabot, changelog, and Rust formatting config.

## Near Term

- [ ] Improve render batching and multi-sprite correctness.
- [ ] Add stronger asset symlink/path regression tests.
- [ ] Improve frame pacing and redraw policy.
- [ ] Reduce logging noise in the demo and replace `println!` with a proper tracing setup.
- [ ] Add a higher-level scene/entity API on top of the current `Game2D` facade.
- [ ] Add asset/resource file watching and route changed scene files into the reload queue.
- [ ] Add visual UI text/image rendering on top of extracted UI records.
- [ ] Add more ergonomic sprite helpers, such as `Sprite::from_texture`.
- [ ] Add smoke tests for the facade crate API.
- [ ] Document manual desktop validation expectations for Windows and Linux.

## Mid Term

- [ ] Evaluate whether a more complete ECS backend is needed after the MVP API is exercised by more examples.
- [ ] Add a simple 2D collision layer or integrate a physics backend behind `seishin_physics`.
- [ ] Add more asset formats and clearer asset error diagnostics.
- [ ] Add renderer resilience tests for resize/minimize/surface loss paths where practical.
- [ ] Expand the C ABI only after equivalent safe Rust APIs stabilize.
- [ ] Add Go binding proof of concept over the C ABI.

## Long Term

- [ ] Explore Android runtime support.
- [ ] Add editor/tooling experiments only after the runtime and asset model are stable.
- [ ] Investigate hot reload after the asset pipeline has stronger invariants.
- [ ] Add more complete documentation and tutorials.

## Explicit Non-Goals For Now

- Cloning existing engines.
- Exposing renderer/window/audio backend internals in public gameplay APIs.
- Exposing Rust collections, references, traits, generics, or lifetimes across FFI.
- Adding a large ECS/plugin/editor architecture before the need is proven.
