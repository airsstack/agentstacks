---
status: draft
created: 2026-11-20
milestone: foundation/01
---

# Intent: Pack and project formats

## Problem

Every later chain in `foundation/01` reads or writes formats that do not exist yet: the pack (what
the agent gets), `agentstacks.toml` (what the human asked for) and `agentstacks.lock` (what was
pinned and written). Without them, the core, the extension host and the claude extension would
each invent their own shape.

The six claudestacks plugins also do not map 1:1 onto the content model the README describes.
`CLAUDE.md` records four gaps to settle before anything is converted:

- namespaced cross-references (`claudestacks-sdlc:status`, `claudestacks:coder`), about 60 of them;
- `${CLAUDE_PLUGIN_ROOT}` paths, about 42 files, with no plugin root once content lands in `.claude/`;
- content types beyond the README list: `scripts/*.lua`, `references/`, `enforcement.json`;
- hooks that locate an unpinned `airsl` binary. Only the format side of this belongs here; the
  runtime belongs to `extension-host-and-hooks`.

## Affected systems

- New: the pack layout and pack metadata, `agentstacks.toml`, `agentstacks.lock`.
- `crates/agentstacks` `model` and `lock` seams: the types that carry these formats.
- Reference only: `../claudestacks/plugins/*` (the six plugins the format must be able to express).

## Desired outcome

- A pack format that can express everything the six plugins ship: skills, agents, commands, Lua
  hooks, rules, scripts, references, guideline enforcement data and `native/<tool>/` files.
- A tool-neutral cross-reference syntax and an install-root abstraction that an extension can
  render for its tool.
- `agentstacks.toml` and `agentstacks.lock` formats that record tools, packs, pinned versions and
  a hash for every written file.
- The formats are parsed and validated by `crates/agentstacks`, and a malformed file is refused
  with a message naming what is wrong.

## Constraints

- Formats are `api = 0`: unstable, free to change across `0.x` (milestone non-goal: no stable format).
- Nothing is ambient: the format declares what a pack needs; nothing is inferred from the
  environment.
- Unsupported is visible: the format must let an extension report an item it cannot express.
- No `aisdk-*` names; `airsl` only from crates.io.

## Non-goals

- Pack versioning across a registry or hosted source (no registry: roadmap non-goal).
- OpenCode-specific needs beyond keeping `native/<tool>/` tool-neutral in shape.
- Converting any plugin (`plugins-to-packs`).
