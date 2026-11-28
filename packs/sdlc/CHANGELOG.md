# Changelog

All notable changes to the `sdlc` pack are recorded here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the pack
follows [Semantic Versioning](https://semver.org/spec/v2.0.0/): the version
moves if and only if the pack's shipped content changed. Each release is
tagged `packs-sdlc-v<version>`.

## [1.1.0] - 2026-11-28

First per-stream release.

### Changed

- The chain root, status board and setup flow moved from the legacy
  `.claudestacks/sdlc/` tree to `.agentstacks/sdlc/`, with a copy-once
  migration that heals existing checkouts and leaves in-flight sessions
  working.

[Unreleased]: https://github.com/airsstack/agentstacks/compare/packs-sdlc-v1.1.0...HEAD
[1.1.0]: https://github.com/airsstack/agentstacks/tree/packs-sdlc-v1.1.0
