---
status: draft
created: 2026-11-20
milestone: foundation/01
---

# Intent: Claudestacks plugins to packs

## Problem

The milestone needs the six claudestacks plugins to live in `packs/` and render, through the
`claude` extension, into config that behaves like the plugins do today. The plugins are core
(`claudestacks`), sdlc, journal, guideline-rust, guideline-python and cmux. Today they exist only as
Claude Code marketplace plugins in `../claudestacks/plugins/`. They carry namespaced
cross-references, `${CLAUDE_PLUGIN_ROOT}` paths, Lua scripts, references, enforcement data and
`sh` hook wrappers, all tied to the plugin model.

## Affected systems

- New `packs/{core,sdlc,journal,guideline-rust,guideline-python,cmux}/`.
- Source: `../claudestacks/plugins/claudestacks{,-sdlc,-journal,-guideline-rust,-guideline-python,-cmux}`.
- Their Lua tests and the CI that runs them.

## Desired outcome

- All six plugins exist as packs in `packs/`, written in the pack format with no Claude-only
  constructs outside `native/claude/`.
- Rendered through the `claude` extension into a project, each pack behaves as its plugin does
  today. claudevs (`claudevs-move`) or an equivalent check shows this.
- The plugins' Lua tests run in agentstacks CI under airsl.
- The plugin model's couplings are gone: hook scripts speak the neutral hook protocol, each pack's
  hooks name only its own agents, the enforce hook no longer reads the installed-plugins registry,
  options reach hooks through declared arguments, and the example references to guideline skills
  become prose. Each resulting behaviour change is called out.

## Constraints

- Depends on `pack-and-project-formats`, `cli-and-claude-extension` and `core-plan-and-apply`.
- Plugin behaviour is the reference; any change in behaviour is called out, not slipped in.
- claudestacks is not edited or archived by this chain (`release-and-migration`).

## Non-goals

- New skills or agents beyond what the plugins ship.
- OpenCode rendering (`foundation/02`).
