---
status: draft
created: 2026-11-20
milestone: foundation/01
---

# Intent: First release and migration

## Problem

A Claude Code user cannot install agentstacks without building it. There is no dist config, no
published crate and no release, and the README says nothing is usable. Users of the claudestacks
marketplace plugins have no documented path to the equivalent packs. The milestone requires a
`0.x` release with prebuilt binaries and crates on crates.io, plus a documented migration, and only
after both does claudestacks get archived, never deleted.

## Affected systems

- New `dist-workspace.toml` and release/smoke workflows; `Cargo.toml` package metadata.
- crates.io: `agentstacks`, `agentstacks-cli`, `claudevs`, `claudevs-cli`.
- `README.md`, the migration doc, and the org docs (`.github/profile/README.md` and the workspace
  `README.md` status change).
- claudestacks: README pointer and archive, after release and migration only.

## Desired outcome

- A tagged `0.x` release ships prebuilt binaries through dist, and the crates are published.
- A user installs `agentstacks` without a Rust toolchain.
- A documented migration takes a project from the claudestacks marketplace plugins to the
  equivalent packs.
- The README no longer says nothing is usable, and the org docs show agentstacks' new status.
- claudestacks is archived after the above, with its releases, `install.sh` and marketplace left
  in place.

## Constraints

- Depends on every other `foundation/01` chain.
- dist sets `aarch64-apple-darwin = "macos-15"` under `[dist.github-custom-runners]`, and the smoke
  workflow uses the same runner.
- Never delete claudestacks. The crates.io names `claudevs`/`claudevs-cli` continue from their
  claudestacks releases.

## Non-goals

- Automated migration tooling beyond what the documented path needs (milestone non-goal).
- Moving airsclaude or openrouter-rs (airsdk).
