# ADR 0005: MVP Requires Automated and Manual Validation Evidence

## Status

Accepted for MVP planning.

## Context

Unit tests can validate engine logic, but the MVP also includes windowing, rendering, input, assets, and audio behavior that cannot be fully proven through pure tests.

## Decision

MVP closure requires both automated validation and recorded manual checks.

Automated baseline:

```sh
cargo test --workspace --all-targets
cargo build --workspace
```

Recommended checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- check
```

Manual checks must cover the playable demo: window opens, sprite appears, input moves it, asset errors are controlled, audio behavior is recorded, and shutdown is clean.

## Consequences

- Final readiness is evidence-based, not assumption-based.
- Manual render/audio evidence is acceptable for MVP if recorded clearly.
- Future CI can automate more of the smoke coverage over time.
