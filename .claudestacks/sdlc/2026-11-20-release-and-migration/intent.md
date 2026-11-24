---
status: approved
created: 2026-11-20
milestone: foundation/01
---

# Intent: First release and migration

## Problem

A Claude Code user cannot install agentstacks without building it. There is no dist config, no
published crate and no release, and the README says nothing is usable. `init --tool claude` and
`init --pack <name>` resolve official names to the newest `packs-v*` tag of
`https://github.com/airsstack/agentstacks`, and no such tag exists, so official names are refused
today. The hooks the packs install run under the `airsl` binary, and nothing tells a user who
lacks it. Users of the claudestacks marketplace plugins have no documented path to the equivalent
packs. The milestone requires a `0.x` release with prebuilt binaries and crates on crates.io, plus
a documented migration, and only after both does claudestacks get archived, never deleted.

agentstacks has no book, unlike airsl, buildl and claudestacks, which each publish an mdBook to
`airsstack.github.io/<project>/`. Its documentation is seven loose files under `docs/` (1,688
lines), readable only on GitHub, and no published page tells a user how to install, use or extend
it. The claudevs documentation still lives in the claudestacks book (chapters 5–9 and appendix A),
and the moved claudevs READMEs link there, so once claudestacks is archived, claudevs' only
documentation sits in a read-only repository that no longer owns the code.

## Affected systems

- New `dist-workspace.toml` and release/smoke workflows; `Cargo.toml` package metadata.
- crates.io: `agentstacks`, `agentstacks-cli`, `claudevs`, `claudevs-cli`.
- Git tags on `airsstack/agentstacks`: the binary release tags and the `packs-v*` tag `init`
  resolves.
- The installers for the `agentstacks` and `claudevs` binaries.
- A new `book/` (mdBook) holding the documentation now under `docs/`, the claudevs chapters from
  the claudestacks book, the packs, installation and the migration; its CI check and Pages build.
- Every link to `docs/` or to the claudestacks book's claudevs chapters (`CLAUDE.md`, READMEs,
  crate READMEs, docs cross-links).
- `README.md`, the migration doc, and the org docs (`.github/profile/README.md` and the workspace
  `README.md` status change).
- claudestacks: README pointer, a pointer from its book's claudevs part to the agentstacks book, its `install.sh` (which serves claudevs from claudestacks
  releases), and the archive, after release and migration only.

## Desired outcome

- A tagged `0.x` release ships prebuilt binaries of both `agentstacks` and `claudevs` through
  dist, and the four crates are published.
- A user installs either binary without a Rust toolchain. agentstacks depends on `airsl`, so the
  installer checks for it and, when it is missing, installs a released `airsl` too, saying what it
  installs.
- A `packs-v0.1.0` tag is cut with the release, so `agentstacks init --tool claude --pack <p>`
  resolves the official extension and packs from the public repository.
- A documented migration takes a project from the claudestacks marketplace plugins to the
  equivalent packs.
- agentstacks has its own mdBook, built in CI and published to
  `https://airsstack.github.io/agentstacks/` beside the installers, holding all its user and
  contributor documentation: installing, using the CLI, the packs, migrating from the plugins,
  writing tool extensions and hooks, the architecture, and claudevs. `docs/` no longer holds a
  second copy.
- The README no longer says nothing is usable, and the org docs show agentstacks' new status.
- claudestacks is archived after the above, with its releases, `install.sh` and marketplace left
  in place.

## Constraints

- Depends on every other `foundation/01` chain (all done or dropped as of 2026-11-24).
- `airsstack/agentstacks` is public (verified 2026-11-24: anonymous `git ls-remote` succeeds), so
  release downloads and `init`'s anonymous clone of the official repository work.
- dist sets `aarch64-apple-darwin = "macos-15"` under `[dist.github-custom-runners]`, and the smoke
  workflow uses the same runner.
- Never delete claudestacks. The crates.io names `claudevs`/`claudevs-cli` continue from their
  claudestacks releases (crates.io has `claudevs` 0.1.0 and `claudevs-cli` 0.1.0, 0.1.1), so the
  moved crates publish a newer version.
- The book follows the shape of the org's other books (airsl, buildl, claudestacks): `book/` with
  `book.toml` and `src/SUMMARY.md`, the same pinned mdBook version, a CI job that builds it, and a
  Pages workflow that publishes it.
- Installing `airsl` uses airsl's own published release, never a copy built or bundled here.
  Nothing is ambient: the installer names `airsl` before it installs it, and a user who already
  has `airsl` gets it left alone.

## Non-goals

- Automated migration tooling beyond what the documented path needs (milestone non-goal).
- Pinning, upgrading or managing `airsl` versions under `$HOME/.airsstack` (the
  `airsstack tool install airsl` front door belongs to the airsstack repo).
- Moving airsclaude or openrouter-rs (airsdk).
- Rewriting the claudestacks book beyond the pointer, or removing its claudevs chapters (the
  archive keeps it read-only and served).
