# Changelog

All notable changes to `agentstacks` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `opencode` is an official tool name.

### Changed

- The hook runtime accepts a list of events from a translator and runs the hook once per event.
  Existing projects with a lua hook show `update <root>/runtime.lua` once after upgrading.

## [0.1.0] - 2026-11-24

Initial release. Minimum supported Rust version: 1.94.

### Added

- The pack, extension, project and lock formats.
- The plan, apply, doctor and update flows.
- The extension host and the hook runtime.
