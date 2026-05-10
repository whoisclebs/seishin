# ADR 0001: Desktop-First MVP Scope

## Status

Accepted for MVP planning.

## Context

`seishin` is intended to become a modular 2D engine with future Android and language binding support. The first delivery must remain small enough to validate the core architecture through one playable example.

## Decision

The MVP targets Windows and Linux desktop first. The primary playable example is `examples/basic_2d`.

Android is a future target and should influence architecture decisions, but Android runtime/build support is not an MVP acceptance gate.

## Consequences

- Runtime work starts with desktop windowing and event loop behavior.
- Platform abstractions should avoid blocking Android later.
- MVP validation focuses on desktop build/tests and manual demo checks.

## Explicit Non-Goals

- Android runtime implementation.
- iOS, wasm, or mobile-specific behavior.
- Multiple playable examples.
