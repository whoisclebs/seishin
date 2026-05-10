# Changelog

All notable changes to this project will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project follows semantic versioning once releases begin.

## [Unreleased]

### Added

- Rust workspace for the `seishin` 2D engine prototype.
- Facade crate with `seishin::prelude::*`.
- Desktop MVP window/runtime path.
- Minimal `wgpu` renderer with clear pass and sprite drawing.
- Input state with pressed/transition semantics.
- Asset root/path validation and PNG loading.
- Simple audio facade with `kira` backend.
- C ABI lifecycle smoke boundary.
- Playable `examples/basic_2d` demo.
- Repository hygiene files: README, LICENSE, CONTRIBUTING, SECURITY, GitHub issue/PR templates, CI, Dependabot, `.gitignore`, `.editorconfig`.
