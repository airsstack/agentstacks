---
status: draft
created: 2026-11-20
milestone: foundation/01
---

# Intent: CLI commands

## Problem

The `agentstacks` binary answers `--version` and nothing else. The milestone requires `init --tool
<t> --pack <p>`, `plan`, `apply`, `doctor` and `update` to work end to end on a real project. A user
has no way to reach the core's plan, apply and drift check, and no way to create or change
`agentstacks.toml`.

## Affected systems

- `crates/agentstacks-cli`: argument parsing, subcommands, exit codes, output.
- Calls into `crates/agentstacks` (formats, core, extension host).

## Desired outcome

- `init --tool <t> --pack <p>` creates `agentstacks.toml` and an `agentstacks.lock` in the user's
  project and installs the requested packs for the requested tool.
- `plan` shows every operation without writing. `apply` writes only what a plan shows.
- `doctor` reports drift between the project and the lock.
- `update` moves pinned pack and extension versions and shows the resulting plan.
- Every refusal names what was missing, and exit codes tell success, refusal and error apart.

## Constraints

- Depends on `core-plan-and-apply` and `extension-host-and-hooks`.
- The CLI stays thin; behaviour lives in the library.
- No plugin marketplace; setup is CLI-only.

## Non-goals

- `ext add` / `ext test` (milestone non-goal).
- The `airsstack agent …` hand-off (the airsstack repo).
