---
status: approved
created: 2026-11-20
---

# Milestone: Claude Code extension on api = 0

## Goal

`agentstacks init --tool claude --pack <p>`, `plan`, `apply`, `doctor` and `update` work end to end
on a real project. A pack format exists that expresses everything the six claudestacks plugins ship
(core, sdlc, journal, guideline-rust, guideline-python, cmux), including cross-references between
packs, an install root in place of `${CLAUDE_PLUGIN_ROOT}`, scripts, references and
`native/<tool>/` files. Those six plugins live in `packs/` and render, through the official `claude`
extension running under airsl with zero grants, into native `.claude/` project config that behaves
like the plugins do today. claudevs and claudevs-cli have moved here under the same crate names.

A `0.x` release is out: prebuilt binaries through dist, and the crates published to crates.io, so a
Claude Code user can install `agentstacks` without building it. A documented migration takes a
project from the claudestacks marketplace plugins to the equivalent packs, and the README no longer
says nothing is usable. Only after that release and migration are out is claudestacks archived,
never deleted.

## Non-goals

- OpenCode, or any tool other than Claude Code.
- A stable extension API or pack format; both may change freely across `0.x` releases.
- Third-party extension tooling (`ext add`, `ext test`) and authoring docs.
- Automated migration tooling beyond what the documented path needs.
