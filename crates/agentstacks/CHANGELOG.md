# Changelog

All notable changes to `agentstacks` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.3.0] - 2026-11-28

### Added

- Independent per-artifact release streams: packs resolve to `packs-<name>-vX.Y.Z` tags
  and extensions to `ext-<name>-vX.Y.Z`, replacing the single `packs-v*` bundle tag
  (frozen, still resolvable).
- Stream-aware newest-release discovery: `init`, `update` and `ext add` find the newest
  tag in each artifact's own stream.

### Changed

- `update` warns on legacy `packs-v*` pins and never rewrites them automatically.
- `doctor` reports per-stream staleness as advisories only: exit 0, fully offline-capable.

## [1.2.0] - 2026-11-27

### Added

- `app::plan::plan_rendered`, which plans a given project text as a fresh project (no
  lock, no target files) while resolving its path sources against a real workspace, and
  `app::init::render`, the resolve-and-render half of `init` that writes nothing.

### Changed

- The git cache root is `AIRSSTACK_HOME` when it is set and non-empty, otherwise
  `$HOME/.airsstack`; `cache_home` takes both values and returns the root, and `url_dir`
  composes from the root. `GitCli::new` takes the `AIRSSTACK_HOME` value beside `HOME`.
- `types::plan::airsl_finding` takes an optional pin and reports a missing `airsl` with
  the install command, with or without a pin (`Finding::AirslMissing` without one);
  `doctor` asks for the `airsl` version on every lock path.
- A failed cache operation's refusal now ends with
  `(set AIRSSTACK_HOME to a writable directory to move the cache)`.

## [1.1.0] - 2026-11-27

### Added

- `musecode` is an official tool name.

## [1.0.0] - 2026-11-25

The extension and pack API is frozen at `api = 1`. The Rust API of this crate is the engine of the
`agentstacks` binary and is not covered by the 1.x promise; the pack format, the extension API, the
hook runtime, `agentstacks.toml`, `agentstacks.lock` and the CLI are
([stability](https://airsstack.github.io/agentstacks/ch06-06-stability.html)).

### Added

- `requires.agentstacks`, an optional version range in `pack.toml` and in a new `[requires]` table
  of `extension.toml`. A pack or extension whose range excludes this version is refused first,
  with that one refusal.
- The `agentstacks ext add` flow: it adds a tool extension to `agentstacks.toml`, then plans and
  applies.
- The `agentstacks ext test` flow: it runs an extension's Lua tests and then plans, applies and
  re-plans the built-in conformance pack in a scratch project.

### Changed

- `api = 1` is the supported API. Packs and extensions at `api = 0` are refused and named as
  coming from agentstacks 0.x. A lock at `api = 0` is still read, and the next apply that writes
  anything writes `api = 1`.
- `update` moves an `api = 0` project to `api = 1`, and `plan`, `apply` and `ext add` refuse such
  a project and name `update`.
- The official shorthand and `update` of an official-repository `packs-v*` source pick the newest
  stable tag whose major version equals the API, so a 1.x binary never moves to a `packs-v2.*`.
- The files written under `api = 1` say `api 1` in their headers (the hook runtime
  `runtime.lua`), so the first apply after `update` shows `update <root>/runtime.lua` once.

## [0.2.0] - 2026-11-25

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
