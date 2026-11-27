# Changelog

All notable changes to `agentstacks-cli` (the `agentstacks` binary) are recorded here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.0] - 2026-11-27

### Added

- `plan --summary` and `plan --verbose`: the plan shows counts per owner by default, the
  full one-line-per-file list with `--verbose`. The plan `apply`, `init`, `update` and
  `ext add` show is the summary.
- `init --dry-run`: prints the rendered `agentstacks.toml` and its plan (planned as a fresh
  project) and writes nothing; it conflicts with `--yes`.

### Changed

- A successful apply that wrote or deleted a file now ends with
  `note: start a new session — hooks, skills and agents load at session start`.
- `doctor` always checks the `airsl` runtime, with or without a lock pin; a missing
  `airsl` is reported with the install command.
- Cache refusals when neither `AIRSSTACK_HOME` nor `HOME` is set now name both variables,
  and a failed cache write names the `AIRSSTACK_HOME` remedy (see the library changelog).

## [1.1.0] - 2026-11-27

### Added

- `init --tool musecode` and `ext add musecode`.

## [1.0.0] - 2026-11-25

The CLI is covered by the 1.x promise: its commands, flags and exit codes are covered, and a 1.x
release may add commands and flags
([stability](https://airsstack.github.io/agentstacks/ch06-06-stability.html)).

### Added

- `ext add <SOURCE>`: adds a tool extension (an official name, a project path or a git URL with
  `--tag` or `--rev`, and `--subdir` for an extension inside the repository) to
  `agentstacks.toml`, then plans, confirms and applies; `--yes` skips the confirmation.
- `ext test <EXTENSION>`: checks an extension's Lua tests and its output against the built-in
  conformance pack in a temporary project, and prints `passed` or `failed`; `--pack <DIR>`
  (repeatable) installs a pack beside the conformance pack.
- A pack or extension with `requires.agentstacks` that excludes this version is refused with
  `install a newer agentstacks`.

### Changed

- The extension and pack API is `api = 1`. `init` writes it, and `update` moves an `api = 0`
  project to `api = 1` before it plans (see
  [Moving a Project from 0.x](https://airsstack.github.io/agentstacks/ch03-01-from-0x.html)).
  `plan`, `apply` and `ext add` refuse an `api = 0` project and say `run agentstacks update`.
- `init`, `ext add <official>` and `update` of an official `packs-v*` source pick the newest
  stable `packs-v1.*` tag, never a later major.

## [0.2.0] - 2026-11-25

### Added

- `init --tool opencode`.

## [0.1.0] - 2026-11-24

Initial release.

### Added

- The commands `init`, `plan`, `apply`, `doctor` and `update`.
- Official names `--tool claude` and `--pack core|sdlc|journal|guideline-rust|guideline-python`,
  resolved to the newest `packs-v*` tag.
- Prebuilt archives for the six release targets, and the installer
  `curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh`, which also installs airsl
  when it is missing.
