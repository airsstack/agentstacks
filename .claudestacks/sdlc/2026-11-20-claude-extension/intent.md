---
status: draft
created: 2026-11-20
milestone: foundation/01
---

# Intent: Claude Code extension

## Problem

No tool extension exists. For Claude Code, packs must render into native `.claude/` project config
(skills, agents, commands, hooks in settings, rules) that behaves as the claudestacks marketplace
plugins do today, but without a plugin namespace or a plugin root. The rendering has to resolve
the pack format's tool-neutral cross-references and install root into what Claude Code reads. Any
item Claude Code cannot express must become a visible skip.

## Affected systems

- New `extensions/claude/` (Lua `adapter.lua`).
- Writes `.claude/` in the user's project, including merges into `.claude/settings.json`.
- Uses the `api = 0` surface from `extension-host-and-hooks`.

## Desired outcome

- The official `claude` extension, running under airsl with zero grants, turns any valid pack into
  proposed operations that produce working native `.claude/` config.
- Cross-references and install-root paths render into names and paths Claude Code resolves.
- Settings merges keep the user's existing keys.
- Each item the extension cannot express appears in the plan as a skip with a reason.

## Constraints

- Depends on `pack-and-project-formats` and `extension-host-and-hooks`.
- Uses only the public extension API, the same one a community extension would use.
- No marketplace output: the extension writes project config, not a plugin.

## Non-goals

- OpenCode (`foundation/02`).
- Converting the plugins themselves (`plugins-to-packs`).
