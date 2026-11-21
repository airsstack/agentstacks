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

Packs and extensions also come from `path` and `git` sources (`pack-and-project-formats`), and
nothing resolves them yet: no fetch, no cache, no lookup of the tag a shorthand name expands to,
and no way to turn a lock entry back into the exact bytes it pinned.

## Affected systems

- `crates/agentstacks`: the `model`, `validation`, `plan`, `executor` and `lock` seams.
- Consumes the formats from `pack-and-project-formats` (its spec hands over the delete operation,
  the merge key, stale-lock reconciliation and the git fetch rules).
- Source resolution: the `git` binary (a declared requirement) and a local content cache.

## Desired outcome

- An operation model covering what the README names: write file, merge config key, run declared
  command, require binary, skip with reason.
- Validation that refuses an operation outside what the pack and extension declared, naming what
  was missing.
- A plan that lists every operation in a form a human can review before anything is written.
- An executor that is the only code that writes. It writes the lock with a hash of every file, and
  refuses to overwrite a file whose content no longer matches the lock.
- A drift check against the lock, which `doctor` will call.
- Source resolution for packs and extensions: fetch a `git` source at its tag, read a `path`
  source, and check the content against the lock's pinned revision and hash. Content already
  cached for a locked revision resolves without network access. A missing `git` binary is refused
  by name.
- The latest official release tag is found for shorthand expansion and for `update`.
- Git fetches pass the URL after `--` and use `core.autocrlf=false`, so the bytes hashed are the
  bytes committed on every platform.
- A pack removed from `agentstacks.toml` has its files removed through a visible operation; the
  operation list gains a delete.
- A config-key merge has a defined identity (the `key` the lock records for it), and a lock that
  disagrees with `agentstacks.toml` is reconciled as stale rather than refused.
- The lock's `[airsl]` table is present exactly when a locked pack needs airsl; the check
  `pack-and-project-formats` provides is called here, where the locked packs are loaded.
- A third-party command never runs unannounced: every run-command operation appears in the plan.

## Constraints

- Depends on `pack-and-project-formats`.
- Library only; CLI wiring belongs to `cli-commands`.
- Workspace lints and `cargo make dod`.

## Non-goals

- Running Lua or loading extensions (`extension-host-and-hooks`).
- Any tool-specific rendering (`claude-extension`).
