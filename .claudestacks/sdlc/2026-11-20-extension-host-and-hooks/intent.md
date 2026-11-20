---
status: draft
created: 2026-11-20
milestone: foundation/01
---

# Intent: Extension host and hook runtime

## Problem

A tool extension is a Lua adapter (`adapter.lua`) that turns packs into one tool's artifacts. It
must run under airsl with zero grants: no file, network or process access, only proposing
operations. agentstacks has no way to load an extension, run it sandboxed, hand it pack content, or
receive its proposed operations. There is also no `api = 0` contract an extension is written
against, and official and community extensions are required to share that same public API.

Hooks have a related gap. Today the claudestacks hooks are `sh` wrappers that locate an unpinned
`airsl` binary, which conflicts with the lock's promise of determinism (`CLAUDE.md`, porting
issue 4). Packs keep Lua hooks, so the runtime that executes them in the user's project has to be
pinned and declared.

## Affected systems

- `crates/agentstacks` `extension_host` seam; a new `airsl` dependency from crates.io (claudestacks
  uses `airsl = "0.1.3"`).
- The hook entry points that packs ship, and the shim a tool invokes to reach airsl.
- The operation model from `core-plan-and-apply`.

## Desired outcome

- `crates/agentstacks` runs an extension's adapter under airsl with zero grants and gets back
  proposed operations, or a refusal naming what the extension tried to use.
- A documented `api = 0` surface that both official and community extensions use; no private hooks.
- Pack hooks run on an airsl binary whose version the lock pins and whose presence is a declared
  requirement, never an ambient one.
- When a tool demands another language for hooks, only a thin generated shim forwards to airsl.

## Constraints

- Depends on `pack-and-project-formats` and `core-plan-and-apply`.
- Lua is the only scripting language; `airsl` only from crates.io.
- `api` stays `0` until both Claude and OpenCode run on it (`api = 1` is `foundation/03`).

## Non-goals

- Extension authoring tools (`ext add`, `ext test`) and authoring docs (milestone non-goal).
- The claude adapter itself (`claude-extension`).
