---
status: draft
created: 2026-11-20
milestone: foundation/01
---

# Intent: Core plan and apply

## Problem

The invariants say extensions only propose: the core validates every operation, `plan` shows it,
and the executor alone touches disk, never silently overwriting a user's edits. None of that
exists. `crates/agentstacks` has empty `model`, `validation`, `plan`, `executor` and `lock` seams,
so there is no operation type for an extension to return, no rule deciding which operations are
allowed, nothing to show the human, and nothing that writes or detects drift.

## Affected systems

- `crates/agentstacks`: the `model`, `validation`, `plan`, `executor` and `lock` seams.
- Consumes the formats from `pack-and-project-formats`.

## Desired outcome

- An operation model covering what the README names: write file, merge config key, run declared
  command, require binary, skip with reason.
- Validation that refuses an operation outside what the pack and extension declared, naming what
  was missing.
- A plan that lists every operation in a form a human can review before anything is written.
- An executor that is the only code that writes. It writes the lock with a hash of every file, and
  refuses to overwrite a file whose content no longer matches the lock.
- A drift check against the lock, which `doctor` will call.
- A third-party command never runs unannounced: every run-command operation appears in the plan.

## Constraints

- Depends on `pack-and-project-formats`.
- Library only; CLI wiring belongs to `cli-commands`.
- Workspace lints and `cargo make dod`.

## Non-goals

- Running Lua or loading extensions (`extension-host-and-hooks`).
- Any tool-specific rendering (`claude-extension`).
