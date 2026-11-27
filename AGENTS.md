# AGENTS.md

Guidance for any coding agent working in this repository. `CLAUDE.md` is the
canonical, fuller version of this guidance; read it before making changes. If
this file and `CLAUDE.md` or the book disagree, `CLAUDE.md` and the book win.

agentstacks is a Rust CLI that sets up agentic coding tools (Claude Code,
Muse Code, OpenCode) from one tool-neutral copy of skills, agents, commands,
hooks and rules, distributed as versioned packs through per-tool Lua extensions.

## Commands

Install the gate tools once: `cargo install --locked cargo-make cargo-deny`
(plus the `airsl` binary on PATH for the Lua step).

- `cargo make dod` — the Definition of Done: fmt-check, clippy, rustdoc, tests,
  doctests and the Lua gates, all with warnings as errors. This is the gate; a
  shorter run counts as iteration, not a green result.
- `cargo make dod-crate <crate>` — the five cargo steps of the gate scoped to
  one crate (no Lua step).
- `cargo make fmt` — rewrite formatting (`clippy-fix` also edits sources).
- `cargo make deny` — cargo-deny policy check; deliberately not part of `dod`.
- `cargo make claudevs-check` — the claudevs fixture corpus;
  `cargo make opencode-check` and `cargo make musecode-check` — the real
  OpenCode and Muse Code against a mock model. All three are deliberately
  not part of `dod`.
- `cargo make book-build` — build the mdBook once into `book/book/`.
- `cargo test -p <crate> <path::to::test> [-- --exact]` — run a single test.

## Code Map

- `crates/agentstacks/` — the library: `types` · `port` · `app` · `fs` ·
  `process` · `host` (see Architecture below).
- `crates/agentstacks-cli/` — the `agentstacks` binary: `init`, `plan`,
  `apply`, `doctor`, `update`, `ext add`, `ext test`.
- `crates/claudevs/`, `crates/claudevs-cli/` — the deterministic Claude Code
  setup test engine and its `claudevs` binary.
- `extensions/claude/`, `extensions/musecode/`, `extensions/opencode/` — the
  official tool extensions: a Lua `adapter.lua` under airsl each (OpenCode adds
  a thin JavaScript shim that only forwards events to airsl).
- `packs/` — the five official packs: `core`, `sdlc`, `journal`,
  `guideline-rust`, `guideline-python`. Tool-neutral content only; no code
  beyond Lua hooks.
- `book/` — the mdBook documentation. `book/src/ch07-00-architecture.md` is the
  layering contract for `crates/agentstacks`;
  `book/src/ch06-00-authoring.md` is the step-by-step guide for writing a tool
  extension.
- `RELEASING.md` — the maintainer's release runbook. `Makefile.toml` and
  `deny.toml` define the cargo-make gate and the cargo-deny policy.

## Architecture

Ports and adapters (hexagonal), for `crates/agentstacks`:

- `types/` — domain data and pure decisions; depends on nothing.
- `port/` — traits plus a fake for each port; depends on `types`.
- `app/` — use-case flows generic over ports; depends on `port` and `types`.
- `fs/`, `process/`, `host/` — adapters that only translate (disk, processes,
  the airsl/Lua host); depends on `port` and `types`.

A rule goes in `types/`, a sequence of steps in `app/`, never the reverse.
Code inside `types`, `port` and `app` never imports an adapter and never calls
`std::fs`, `std::process` or `std::net`; `tests/isolation.rs` enforces this.
The core is tool-agnostic: a tool's file shapes (for example Claude Code's
`settings.json`) live only in that tool's extension, never in `types/`, `app/`
or `host/`. Every rule and flow is unit-tested with plain values or fakes;
adapters get one contract test each against the real thing.

## Invariants (do not break)

- Extensions only propose operations (write file, copy file, require binary,
  skip-with-reason). An extension has no file, network or process access; the
  core validates every operation, `plan` shows it, and the executor in the core
  is the only writer. A third-party command never runs unannounced.
- Nothing is ambient: an extension gets only what it declared in
  `extension.toml`, and a refusal names what was missing.
- Unsupported is visible: an item a tool cannot express becomes a skip
  operation with a reason, never a silent drop.
- Lua is the only scripting language, always run under airsl. When a tool
  demands another language, generate a thin shim that only forwards events to
  airsl.
- The official extensions use the same public API as community extensions,
  frozen as `api = 1`; 1.x changes are additive only.
- `airsl` comes from crates.io, never as a path dependency. Never use the
  `aisdk-*` crate names; that family belongs to another project.
- `apply` never silently overwrites user edits: `agentstacks.lock` pins pack
  and extension versions and hashes every written file, and `doctor` checks
  drift against it.

## How work is done

- Work goes through the claudestacks-sdlc chain (intent → design/spec → plan →
  execute), with files under `.claudestacks/sdlc/`. Read progress from that
  board, not from `CLAUDE.md`.
- Conventional Commits, `type(scope): summary`; the scope is the crate name, or
  `repo` for cross-cutting changes.
- Work on a branch or worktree, never on `main`.
- A book chapter describes what the code does; when code and chapter disagree,
  fix one of them in the same change.
