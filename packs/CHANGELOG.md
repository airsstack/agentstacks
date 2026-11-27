# Changelog

All notable changes to the official packs and tool extensions are recorded here. They are released
together under `packs-v*` tags, which `agentstacks init` and `agentstacks update` resolve. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and each pack and extension
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [packs-v1.1.0] - 2026-11-27

### Added

- The `musecode` tool extension at 1.0.0, on extension API `api = 1`, for Muse Code 1.4.25.

## [packs-v1.0.0] - 2026-11-25

### Changed

- Every pack and tool extension is on `api = 1`, the frozen API of agentstacks 1.0, and at version
  1.0.0: `core`, `sdlc`, `journal`, `guideline-rust`, `guideline-python`, and the `claude` and
  `opencode` extensions. They need agentstacks 1.0 or newer; agentstacks 0.x refuses them, so run
  `agentstacks update` with 1.0 to move a project.
- The headers of the written files `runtime.lua` and the Claude and OpenCode translators and
  plugin say `api 1`. The first apply after `update` shows each as an `update` step of a locked
  file, not as a user edit.
- From this release the tag's major version is the API: a 1.x agentstacks resolves `packs-v1.*`
  only, so `packs-v2.*` waits for `api = 2`.

## [packs-v0.2.0] - 2026-11-25

### Added

- The `opencode` tool extension at 0.1.0, on extension API `api = 0`, for OpenCode V2.

### Changed

- `journal` 0.2.0: `capture` and `note` read the session transcript on OpenCode too, through
  `opencode session export`, when OpenCode runs its background service. On OpenCode the session
  key is the last eight characters of the session id, in lower case.
- `core` 0.1.1, `sdlc` 0.1.1 and `journal`: skill and agent text no longer names one tool's own
  tools; spawning and todo lists are described in plain words.
- The `opencode` extension lets agents read and write outside the project, as OpenCode's own
  default does, so pack agents reach the journal vault and their handoff reports.

## [packs-v0.1.0] - 2026-11-24

First release.

### Added

- Packs `core`, `sdlc`, `journal`, `guideline-rust` and `guideline-python`, each at 0.1.0.
- The `claude` tool extension at 0.1.0, on extension API `api = 0`.

Coming from the claudestacks plugins? See
[Moving from the claudestacks Plugins](https://airsstack.github.io/agentstacks/ch03-00-migration.html).
