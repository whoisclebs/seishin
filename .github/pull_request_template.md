## Summary

- 

## Type of Change

- [ ] Bug fix
- [ ] Feature
- [ ] Refactor
- [ ] Documentation
- [ ] Tests/tooling

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-targets`
- [ ] `cargo build --workspace`
- [ ] Manual demo validation, if touching window/render/input/audio

## Architecture Checklist

- [ ] Public API does not expose backend internals (`wgpu`, `winit`, `kira`, `image`, etc.).
- [ ] FFI changes use opaque handles / C-compatible values only.
- [ ] Scope remains MVP-sized or is explicitly documented as post-MVP.
- [ ] Docs or ADRs were updated if behavior/architecture changed.

## Notes

- 
