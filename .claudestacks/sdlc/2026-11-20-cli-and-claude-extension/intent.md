---
status: approved
created: 2026-11-20
milestone: foundation/01
---

# Intent: CLI commands and Claude Code extension

Merged on 2026-11-22 from two intents, `cli-commands` and `claude-extension`, so both halves are
designed, planned and built together as one chain.

## Problem

The `agentstacks` binary answers `--version` and nothing else. The milestone requires `init --tool
<t> --pack <p>`, `plan`, `apply`, `doctor` and `update` to work end to end on a real project. A user
has no way to reach the core's plan, apply and drift check, and no way to create or change
`agentstacks.toml`.

No tool extension exists either. For Claude Code, packs must render into native `.claude/` project
config (skills, agents, commands, hooks in settings, rules) that behaves as the claudestacks
marketplace plugins do today, but without a plugin namespace or a plugin root. The rendering has to
resolve the pack format's tool-neutral cross-references and install root into what Claude Code
reads. Any item Claude Code cannot express must become a visible skip.

The core's writes inside the project are not safe against a planted symlink. `agentstacks.toml`,
`agentstacks.lock` and every staged file are written through a temp path beside the target. A repo
that ships a symlink at one of those temp paths makes `init`, `update` or `apply` overwrite the file
the link points to, outside the project, and leaves the target as a link. Found in review on
2026-11-22 (reviewer probe: an outside file ended up holding `tools = []`).

## Affected systems

- `crates/agentstacks-cli`: argument parsing, subcommands, exit codes, output.
- Calls into `crates/agentstacks` (formats, plan, apply, doctor, update, extension host).
- New `extensions/claude/` (Lua `adapter.lua` and `translate.lua`).
- `.claude/` in the user's project, including the shared `.claude/settings.json`.
- The disk adapter in `crates/agentstacks` (`fs/`): every write the core makes inside the project.

## Desired outcome

- `init --tool <t> --pack <p>` creates `agentstacks.toml` and an `agentstacks.lock` in the user's
  project and installs the requested packs for the requested tool.
- `plan` shows every operation without writing. `apply` writes only what a plan shows.
- `doctor` reports drift between the project and the lock.
- `update` moves pinned pack and extension versions and shows the resulting plan.
- Every refusal names what was missing, and exit codes tell success, refusal and error apart.
- The official `claude` extension, running under airsl with zero grants, turns any valid pack into
  proposed operations that produce working native `.claude/` config.
- Cross-references and install-root paths render into names and paths Claude Code resolves.
- `.claude/settings.json` is read by the extension (declared `reads`), edited with
  `agentstacks.json.edit` and rewritten whole, keeping the user's existing keys (core D2).
- Each item the extension cannot express appears in the plan as a skip with a reason.
- No write the core makes inside the project follows a symlink planted at its temp path. A repo
  that ships one cannot make `init`, `update` or `apply` change a file outside the project.

## Constraints

- Depends on `pack-and-project-formats` and `core-plan-and-apply` (which carries the extension
  host and hook runtime).
- The CLI stays thin; behaviour lives in the library.
- The extension uses only the public `api = 0` surface, the same one a community extension uses.
- No plugin marketplace: setup is CLI-only, and the extension writes project config, not a plugin.

## Non-goals

- `ext add` / `ext test` (milestone non-goal).
- The `airsstack agent …` hand-off (the airsstack repo).
- OpenCode (`foundation/02`).
- Converting the plugins themselves (`plugins-to-packs`).
