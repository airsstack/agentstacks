---
status: done
created: 2026-11-20
milestone: foundation/01
---

# Intent: Core plan and apply, extension host and hooks

This chain also carries the scope of `2026-11-20-extension-host-and-hooks`, merged here on
2026-11-21 because both halves share one operation model and one conflict rule; that intent is
dropped.

## Problem

The invariants say extensions only propose: the core validates every operation, `plan` shows it,
and the executor alone touches disk, never silently overwriting a user's edits. None of that
exists. `crates/agentstacks` has empty `validation`, `plan`, `executor` and `host` seams, so there
is no operation type for an extension to return, no rule deciding which operations are allowed,
nothing to show the human, and nothing that writes or detects drift.

Packs and extensions also come from `path` and `git` sources (`pack-and-project-formats`), and
nothing resolves them yet: no fetch, no cache, no lookup of the tag a shorthand name expands to,
and no way to turn a lock entry back into the exact bytes it pinned.

A tool extension is a Lua adapter (`adapter.lua`) that turns packs into one tool's artifacts. It
must run under airsl with zero grants: no file, network or process access, only proposing
operations. agentstacks has no way to load an extension, run it sandboxed, hand it pack content,
or receive its proposed operations, and there is no `api = 0` contract an extension is written
against.

Hooks have a related gap. Today the claudestacks hooks are `sh` wrappers that locate an unpinned
`airsl` binary, which conflicts with the lock's promise of determinism (`CLAUDE.md`, porting
issue 4). Packs keep Lua hooks, so the runtime that executes them in the user's project has to be
pinned and declared.

## Affected systems

- `crates/agentstacks`: replaces the empty `validation`, `plan`, `executor` and `host` seams with
  the layout in `docs/architecture.md` (types, port, app, fs, process, host).
- Consumes the formats from `pack-and-project-formats` (its spec hands over the delete operation,
  stale-lock reconciliation and the git fetch rules; its merge key is dropped, see below).
- Source resolution: the `git` binary (a declared requirement) and a local content cache.
- A new `airsl` library dependency from crates.io; the hook entry points packs ship.

## Desired outcome

- An operation model covering write file, copy file, require binary, skip with reason, and delete
  (below). There is no merge operation: a config file shared with the user is rewritten whole by
  the extension that read it, and the core stays unaware of any config format (settled
  2026-11-21).
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
- A write to a file the extension read is refused when the file changed after that read, so a
  user's concurrent edit is never lost. The lock's `[[merge]]` table is removed from the formats.
- A lock that disagrees with `agentstacks.toml` is reconciled as stale rather than refused.
- The lock's `[airsl]` table is present exactly when a locked pack needs airsl; the check
  `pack-and-project-formats` provides is called here, where the locked packs are loaded.
- The installed `airsl` binary that pack hooks run on is checked against the packs'
  `requires.airsl` when planning, its version is pinned in the lock, and `doctor` reports a
  missing binary or one that differs from the lock.
- `crates/agentstacks` runs an extension's adapter under airsl with zero grants and gets back
  proposed operations, or a refusal naming what the extension tried to use.
- A documented `api = 0` surface that both official and community extensions use; no private
  hooks.
- An extension may read only the project files its `extension.toml` declares (`reads`), and gets
  a pure JSON text-edit helper that keeps key order and indent, so it can rewrite a config file it
  shares with the user (settled 2026-11-21).
- When a tool demands another language for hooks, only a thin generated shim forwards to airsl.
- A tool-neutral hook protocol: the tool's hook payload becomes a neutral event and the hook's
  neutral result (context, block with a reason, or nothing) becomes the tool's protocol. Only a
  hook declared `may_block` can block; a block from any other hook is dropped and reported.

## Constraints

- Depends on `pack-and-project-formats`.
- Library only; CLI wiring belongs to `cli-commands`.
- Workspace lints and `cargo make dod`.
- Lua is the only scripting language; `airsl` only from crates.io.
- `airsl` is embedded as a library at its latest published version and extended only through its
  public API (settled 2026-11-21). Gaps in airsl are filed upstream (airsstack/airsl#11–#15) and
  worked around here, not fixed in airsl by this chain.
- `api` stays `0` until both Claude and OpenCode run on it (`api = 1` is `foundation/03`).
- The core is tool-agnostic: no tool's file shapes in the core; that knowledge lives only in the
  tool's extension.

## Non-goals

- A run-command operation. No pack needs one; it is left out of `api = 0` (settled
  2026-11-21) and added when real content needs it.
- Any tool-specific rendering, including the claude adapter itself (`claude-extension`).
- Extension authoring tools (`ext add`, `ext test`) and authoring docs (milestone non-goal).
