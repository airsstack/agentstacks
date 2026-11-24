---
status: approved
created: 2026-11-24
milestone: foundation/02
---

# Intent: The official OpenCode extension

## Problem

agentstacks sets up only Claude Code. `foundation/02` requires the same packs to render into
OpenCode through an official `opencode` extension that uses only the public `api = 0` surface the
`claude` extension uses. No such extension exists: `extensions/` holds only `claude/`, and
`OFFICIAL_TOOLS` (`crates/agentstacks/src/types/shorthand.rs:48`) lists only `claude`, so
`agentstacks init --tool opencode` has nothing to resolve.

OpenCode has changed shape since the roadmap was written. Research on 2026-11-24 (docs at
opencode.ai and the `anomalyco/opencode` source, not yet run here) reports:

- **V2 is out.** v2.4.1 shipped 2026-11-23, and v1.19.6 (2026-11-17) is still maintained. V2 uses
  plural config keys (`agents`, `commands`, `plugins`, `permissions` as a rule list) and a new
  plugin API (`Plugin.define({ id, setup })`) that does not load V1 function-style plugins.
- **Skills and agents are native.** `.opencode/skills/<id>/SKILL.md` (`disable-model-invocation`
  is read) and `.opencode/agents/<name>.md` (name from the path, `mode: subagent`).
- **Hooks exist only as plugins** (TypeScript or JavaScript, auto-loaded from `.opencode/plugins/`).
  Our hook runtime runs Lua under airsl, so a shim must forward each event to the hook command and
  bring the result back. Whether a plugin may spawn a subprocess is not documented.
- **Several neutral events have no OpenCode hook.** `tool.before` and `tool.after` map to the tool
  `execute.before` and `execute.after` hooks, and `prompt.submit` maps to the session `prompt` hook
  (which cannot block). `session.start`, `session.end` and `agent.stop` have no counterpart. Six of
  core's ten hooks, plus the session-start and agent-stop hooks of sdlc and journal, sit on those
  events.
- **The agent model tier has no meaning in OpenCode.** OpenCode names a model as
  `provider/model[#variant]`, and it has no tier, so the neutral `fast`, `balanced` and `deep`
  have no direct mapping. The neutral `effort` has no known mapping either.
- **OpenCode also reads `.claude/skills/`.** With both tools listed, it may see every skill twice.
- **User-invoked skills.** Whether a skill can be run as `/name` the way Claude runs a
  user-invoked skill is unclear. The neutral format reserves `commands/` (`layout.rs:67`), so a
  pack cannot ship a command instead.
- **Vars.** `{{var project_dir}}` (74 uses) and `{{var session_id}}` (7 uses) need an OpenCode
  value or a visible skip.

## Affected systems

- New `extensions/opencode/` (`extension.toml`, `adapter.lua`, `lib/` rules and their `airsl test`
  files, the generated TypeScript shim text, the `translate.lua` text).
- `crates/agentstacks/src/types/shorthand.rs` (`OFFICIAL_TOOLS`) and the CLI help and docs that
  list official tools.
- `extensions/claude/`: any API change OpenCode forces lands there too.
- Book: `ch06-00-authoring.md`, `ch06-01-extensions.md`, `ch06-02-hooks.md` (the shim section, the
  OpenCode examples), `ch04-00-cli.md`, `ch01-*` getting started, the README and the CLAUDE.md
  status.
- `cargo make lua` coverage of the new extension, CI, and the `packs-v*` release that carries the
  extension.
- Org docs: `.github/profile/README.md` and the workspace `README.md`, if the project status line
  changes.

## Desired outcome

- `agentstacks init --tool opencode --pack <p>`, `plan`, `apply`, `doctor` and `update` work end to
  end on a real project, writing native OpenCode V2 project config.
- A manifest that lists both `claude` and `opencode` sets up both from one `apply`, with no path
  clash and no duplicate items seen by either tool.
- Pack skills, agents and `native/opencode/` files reach OpenCode. Lua hooks on events OpenCode can
  express fire in a real OpenCode session through the shim, add context, and block when declared
  `may_block` and the event allows it.
- Every item OpenCode cannot express appears in `plan` as a skip with a reason a user can act on.
- Each behaviour claim about OpenCode is proven by running OpenCode v2.4.1 itself.
- The `claude` extension runs on whatever API shape the `opencode` extension ends up needing.

## Constraints

- **OpenCode V2 only** (decided 2026-11-24). V1 config and plugins are not targeted.
- Only the public extension API, with no private hooks for official extensions. `api` stays `0`.
- Lua is the only scripting language: the TypeScript file is a generated shim that only runs the
  hook argv from the project directory and forwards the payload and result. It holds no logic.
- The extension only proposes: zero grants, no file, process or network access. A shared OpenCode
  config file, if one is needed, is read through `reads` and rewritten whole.
- The core stays tool-agnostic: no OpenCode file shapes in `types/`, `app/` or `host/`.
- Prove every claim by running the real OpenCode binary at a pinned version, plus agentstacks and airsl.

## Non-goals

- OpenCode V1.
- Freezing the API (`foundation/03`).
- Pack content changes, including the journal transcript lookup and the Claude wording in pack
  text. Both belong to the sibling chain `2026-11-24-packs-tool-neutral`.
- Tools other than Claude Code and OpenCode.
- OpenCode global config (`~/.config/opencode/`), MCP servers, formatters and TUI plugins.
