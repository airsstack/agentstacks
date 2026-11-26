---
status: done
created: 2026-11-25
---

# Intent: The official Muse Code extension

## Problem

agentstacks sets up Claude Code and OpenCode only. Meta's Muse Code is a third agentic coding tool
the same packs should reach, through an official `musecode` extension on the public `api = 0`
surface. Nothing exists for it: `extensions/` holds `claude/` and `opencode/`, and `OFFICIAL_TOOLS`
(`crates/agentstacks/src/types/shorthand.rs:48`) lists only those two, so
`agentstacks init --tool musecode` has nothing to resolve.

Research on 2026-11-25 (the user docs at `dev.meta.ai/docs/muse-code`, the developer guides at
`meta-models.github.io/muse-code-sdk`, and runs of the real binary, Muse Code 1.4.25
`1.4.25-R9361.1`, with its `echo` provider and a mock Responses-API model) reports:

- **Muse Code is a closed-source CLI** (`muse`), installed by a script that downloads a native
  binary from Meta. The download needed no login. Only the SDK is open (MIT).
- **Hooks use Claude Code's format** in `.muse/hooks.json` (command handlers, `matcher`, JSON on
  stdin, `hookSpecificOutput` on stdout). Every neutral event has a Muse Code event:
  `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `SubagentStop`, `SessionEnd`.
  A `PreToolUse` deny blocked a write; `additionalContext` reached the model on session start,
  prompt and after a tool.
- **Tool names differ from Claude's.** The payload names `read_file`, `edit_file`, `write_file`;
  the matchers `Read`, `Edit`, `Write` select them.
- **The hook environment is cut to an allowlist** (`HOME`, `PATH`, `TMPDIR`, `USER`, …, plus
  `MUSE_SESSION_ID`). `AIRSSTACK_HOME` and other variables are dropped, and there is no
  project-directory variable. Hooks run from the workspace root.
- **The workspace root is the launch directory, not the git root.** Started in a subdirectory,
  Muse Code did not load the project's `.muse/hooks.json`.
- **Skills and agents are native.** `.agents/skills/<id>/SKILL.md` (`disable-model-invocation`
  read) and `.agents/agents/<name>.md` (`name`, `description`, `model`, `effort`, `tools`). An agent
  with an unknown frontmatter key is dropped from the catalog with no warning.
- **Muse Code also reads Claude's files.** It scans `.claude/skills/` (a skill with the same id under
  `.agents/skills/` wins) and reads `CLAUDE.md`, including the user's `~/.claude/CLAUDE.md`.
- **Workspace trust gates project config.** Project skills, hooks and agents load only in a trusted
  workspace.
- **The agent model tier has no meaning in Muse Code**, which names models by id.
- **Built-in agents fire hooks too.** `SubagentStop` fired for Muse Code's own `goal-reminder` agent
  on every turn.

## Affected systems

- New `extensions/musecode/` (`extension.toml`, `adapter.lua`, `lib/` rules and their `airsl test`
  files, the `translate.lua` text).
- `crates/agentstacks/src/types/shorthand.rs` (`OFFICIAL_TOOLS`) and the CLI help and docs that
  list official tools.
- `extensions/claude/` and `extensions/opencode/`: any API change Muse Code forces lands there too.
- Book: a new Muse Code chapter beside `ch06-03-opencode.md`, `SUMMARY.md`,
  `ch06-00-authoring.md`, `ch06-01-extensions.md`, `ch06-02-hooks.md` where they name tools,
  `ch04-00-cli.md`, `ch01-*` getting started, the README and the CLAUDE.md status.
- `cargo make lua` coverage of the new extension; a new `cargo make musecode-check` task and CI job;
  the `packs-v*` release that carries the extension.
- Org docs: `.github/profile/README.md` and the workspace `README.md`, if the project status line
  changes.

## Desired outcome

- `agentstacks init --tool musecode --pack <p>`, `plan`, `apply`, `doctor` and `update` work end to
  end on a real project, writing native Muse Code project config.
- A manifest that lists `musecode` beside `claude` and/or `opencode` sets up every tool from one
  `apply`, with no path clash and no skill seen twice by Muse Code.
- Pack skills, agents and `native/musecode/` files reach Muse Code. Lua hooks fire in a real Muse
  Code session, add context, and block when declared `may_block` and the event allows it.
- Every item Muse Code cannot express appears in `plan` as a skip with a reason a user can act on.
- Each behaviour claim about Muse Code is proven by running Muse Code 1.4.25 itself.
- `cargo make musecode-check` runs the extension inside the pinned real Muse Code (version and
  binary hash pinned, a mock model, no account, no secrets), and a CI job runs it, as
  `opencode-check` does for OpenCode. It is not part of `dod`.

## Constraints

- Muse Code 1.4.25 (`1.4.25-R9361.1`) is the pinned version.
- Only the public extension API, with no private hooks for official extensions. `api` stays `0`.
- Lua is the only scripting language; hooks run under airsl.
- The extension only proposes: zero grants, no file, process or network access. A config file shared
  with the user, if one is needed, is read through `reads` and rewritten whole.
- The core stays tool-agnostic: no Muse Code file shapes in `types/`, `app/` or `host/`.
- Probes isolate `HOME`, `XDG_CONFIG_HOME` and `TMPDIR`, and never use or store a Meta account.
- Prove every claim by running the real Muse Code binary at the pinned version, plus agentstacks and
  airsl.
- Outside the `foundation` roadmap: `foundation/03` lists more official tools as a non-goal, so this
  chain has no milestone.

## Non-goals

- Freezing the API (`foundation/03`).
- Pack content changes.
- Muse Code user-level config (`~/.config/muse/`), MCP servers, permission profiles, `AGENTS.md`
  rules, and Muse Code plugins (`.muse-plugin/`).
- A test against Meta's real model; the mock model and the `echo` provider prove the contract.
- Windows.
- Tools other than Claude Code, OpenCode and Muse Code.
