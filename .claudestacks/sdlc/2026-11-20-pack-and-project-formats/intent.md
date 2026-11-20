---
status: approved
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

- namespaced cross-references (`claudestacks-sdlc:status`, `claudestacks:coder`), 60 of them: 42
  in Markdown, 11 in Lua, 6 in JSON, 1 in a shell script;
- `${CLAUDE_PLUGIN_ROOT}` paths, in 42 files, with no plugin root once content lands in `.claude/`;
- content types beyond the README list: `scripts/*.lua`, `references/`, `enforcement.json`;
- hooks that locate an unpinned `airsl` binary. Only the format side of this belongs here; the
  runtime belongs to `extension-host-and-hooks`.

Two more gaps come from the plugin model itself:

- **No pack source.** Plugins arrive through the Claude Code marketplace and its cache. Without a
  marketplace or registry, nothing says where `init --pack core` (or `--tool claude`) gets its
  content from, or how that content is pinned.
- **Cross-pack coupling by ambient discovery.** The core plugin's enforce hook walks the
  installed-plugins registry under `~/.claude/plugins`, filters to the claudestacks marketplace and
  loads every guideline plugin's `enforcement.json` at runtime. The core plugin's `SubagentStop`
  matcher hard-codes agent names from the sdlc and journal plugins. Both break "nothing is
  ambient" once content is rendered per project.

## Affected systems

- New: the pack layout and pack metadata, `agentstacks.toml`, `agentstacks.lock`.
- `crates/agentstacks` `model` and `lock` seams: the types that carry these formats.
- Reference only: `../claudestacks/plugins/*` (the six plugins the format must be able to express),
  in particular `claudestacks/hooks/enforce.lua`, `hooks/lib/enforce.lua` and `hooks/hooks.json`.

## Desired outcome

- A pack format that can express everything the six plugins ship: skills, agents, Lua hooks,
  scripts, references, guideline enforcement data and `native/<tool>/` files. Commands and rules,
  which none of the six plugins ships, are reserved rather than defined.
- A tool-neutral cross-reference syntax and an install-root abstraction that an extension can
  render for its tool, covering a pack's own root and the root of a pack it requires.
- Declared dependencies between packs, so a pack can use another pack's skills, agents and hook
  scripts without discovering them at runtime.
- Pack and extension sources that say where content comes from, and a lock that pins it so the
  same lock always resolves to the same bytes.
- A compatibility declaration on packs and extensions, so a binary refuses content written for an
  `api` it does not speak.
- `agentstacks.toml` and `agentstacks.lock` formats that record tools, packs, their sources,
  pinned versions and a hash for every written file.
- The formats are parsed and validated by `crates/agentstacks`, and a malformed file is refused
  with a message naming what is wrong.

## Constraints

- Formats are `api = 0`: unstable, free to change across `0.x` (milestone non-goal: no stable format).
- Nothing is ambient: the format declares what a pack needs; nothing is inferred from the
  environment.
- Unsupported is visible: the format must let an extension report an item it cannot express.
- No `aisdk-*` names; `airsl` only from crates.io.
- Settled with the author on 2026-11-20 (design works out the shapes, not the choices):
  - Sources are `path` and `git` only; no content is embedded in the binary. Official packs and
    extensions are fetched like community ones, so they have no privileged channel.
  - Git fetches use the `git` binary as a declared requirement, not a git library.
  - Git sources can name a subdirectory, because official packs and extensions live in this repo.
  - A bare name (`--pack core`, `--tool claude`) is CLI shorthand for the official repo only. `init`
    expands it into the full source written to `agentstacks.toml`; the committed file never relies
    on a default compiled into the binary.
  - Packs share no data. Each pack's hooks act on that pack's own data; a guideline pack runs the
    core pack's enforce dispatcher on its own `enforcement.json`, and each pack names its own
    agents for the handoff check. Shared code is reached through a declared dependency.
  - An unmet dependency (a guideline pack without core) is refused, naming the missing pack.

## Non-goals

- A registry or hosted pack index (roadmap non-goal), and user-defined source aliases until real
  work needs them.
- Fetching or caching content; this chain defines the source and lock formats, and the code that
  resolves them belongs to `core-plan-and-apply` and `cli-commands`.
- OpenCode-specific needs beyond keeping `native/<tool>/` tool-neutral in shape.
- Converting any plugin (`plugins-to-packs`).
