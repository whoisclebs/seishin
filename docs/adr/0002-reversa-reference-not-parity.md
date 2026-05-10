# ADR 0002: Planning Notes Are Reference Material, Not Parity Scope

## Status

Accepted for MVP planning.

## Context

The repository may contain internal planning notes and generated specifications. Those documents include useful architectural patterns, but also describe systems far beyond the `seishin` MVP.

## Decision

Use planning notes as a reference corpus for patterns, risks, and vocabulary. Do not treat them as a requirement to reproduce another engine's behavior or public API surface.

## Consequences

- Large framework-scale plugin graphs, exact scheduler parity, advanced render nodes, UI stacks, remote protocols, hot reload, and editor-like tooling are out of MVP scope.
- Conflicting planning signals are resolved in favor of the explicit `seishin` MVP contract.
- Architecture lessons can be adopted selectively when they serve the small vertical slice.
