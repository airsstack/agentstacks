---
status: approved
created: 2026-11-20
spec: skipped
milestone: foundation/01
---

# Intent: Workspace skeleton

## Problem

agentstacks has a README, a CLAUDE.md and an approved roadmap, but no Cargo workspace. No chain in
`foundation/01` can produce code that passes the airsstack Definition of Done, because there is
nothing to run `cargo make dod` against, and CI has nothing to gate. Every later chain (core model,
claude extension, packs, the claudevs move, the 0.x release) would otherwise set up the workspace
shape as a side effect of its own work, mixing infrastructure decisions into feature diffs.

Spec skipped: this copies the workspace shape that claudestacks already runs (same gate, lints,
toolchain and CI pattern) and makes no new design decision.

## Affected systems

- agentstacks repository root: workspace manifest, toolchain, formatting, lint policy, cargo-make
  tasks, cargo-deny config, CI workflow.
- New crates `crates/agentstacks` (library) and `crates/agentstacks-cli` (binary).
- `CLAUDE.md` Commands section, which today says no command works yet.
- Reference only, not changed: `../claudestacks` (`Cargo.toml`, `Makefile.toml`, `deny.toml`,
  `rust-toolchain.toml`, `rustfmt.toml`, `.github/workflows/ci.yml`).

## Desired outcome

- `cargo make dod` and `cargo make dod-crate <crate>` pass on a clean checkout, locally and in CI on
  push to `main` and on pull requests.
- `cargo make fmt` and `cargo make deny` exist and behave as in the other airsstack Rust projects;
  `deny` stays outside `dod`.
- The workspace enforces the shared lint policy (`unsafe_code = "forbid"`, `unwrap_used` denied,
  pedantic + nursery clippy), edition 2024, and the toolchain pinned exactly to 1.94.1 with
  `rust-version = "1.94"`, matching claudestacks.
- `crates/agentstacks` exists as a library with the module seams the README names (model, plan,
  executor, lock, extension host), with no behaviour yet. `crates/agentstacks-cli` builds an
  `agentstacks` binary that answers `--version` and nothing more.
- `CLAUDE.md` Commands describes what now runs, with no "none work yet" caveat.

## Constraints

- `airsl`, if it is added at all, comes from crates.io, never as a path dependency. No `aisdk-*`
  crate names.
- `Makefile.toml` must agree with the `claudestacks-guideline-rust:rust-guidelines` skill. Where they
  differ, the skill wins.
- Tasks are copied only where agentstacks uses them; no claudestacks-specific tasks (plugins,
  corpus, install, book) come across.
- Nothing is published: no crates.io release, no name reservation, no `dist` config.
- Work on a branch; Conventional Commits with crate names or `repo` as scope.

## Non-goals

- `extensions/`, `packs/`, and Lua CI; they arrive with the first Lua code.
- Moving `claudevs` / `claudevs-cli` from claudestacks; that is its own chain.
- `dist-workspace.toml`, release workflows and crates.io publishing; that is the 0.x release chain.
- `book/` and the pages workflow; they arrive with the first user-facing docs.
- Any real CLI subcommand (`init`, `plan`, `apply`, `doctor`, `update`) or core type.
