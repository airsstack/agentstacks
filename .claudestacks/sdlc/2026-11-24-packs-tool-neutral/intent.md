---
status: approved
created: 2026-11-24
milestone: foundation/02
---

# Intent: Tool-neutral pack text and a per-tool journal

## Problem

The five packs are meant to be tool-neutral outside `native/<tool>/`, but parts of them only make
sense in Claude Code. Rendered into OpenCode, those parts give the model wrong instructions or
fail.

- **Claude wording in skill text.** Several skills tell the model how to delegate in Claude's own
  terms:
  - journal `note`, `capture`, `recall`, `link` and `review` say "Task / Agent tool,
    `subagent_type: …`";
  - core `orchestrate` says "the `Agent` tool's `subagent_type`";
  - sdlc `execute` says "TodoWrite".

  OpenCode names its tools differently, so an OpenCode agent following these steps reaches for
  tools it does not have.
- **The journal reads Claude transcripts.** `capture` and `note` find the session transcript
  through `packs/journal/scripts/transcript-path.lua`. That script reads Claude's store layout
  (`$CLAUDE_CONFIG_DIR` or `~/.claude`, then `projects/<slug>/<session_id>.jsonl`), and the
  `capture` and `note` agents read Claude's JSONL line format. OpenCode keeps sessions in its own
  store and format, so on OpenCode the journal cannot capture a session at all.
- **Other Claude names in pack text.** `CLAUDE.md` and `Claude Code` appear in core scripts
  (`lib/handoff.lua`, `lib/handoff_report.lua`, `handoff.lua`), in the context-handoff protocol
  reference, in sdlc `distill` and `lib/status.lua`, and in the sdlc `task-briefer` agent. Some
  are correct as they are, for example a file name the project really has, and some are not.

## Affected systems

- `packs/journal/`: `scripts/transcript-path.lua`, the `capture`, `note`, `recall`, `link` and
  `review` skills, the `capture` and `note` agents, `references/note-schema.md`, and their tests.
- `packs/core/`: `skills/orchestrate/SKILL.md`, `skills/context-handoff/references/protocol.md`,
  `scripts/handoff.lua`, `scripts/lib/handoff.lua`, `scripts/lib/handoff_report.lua`, and their
  tests.
- `packs/sdlc/`: `skills/execute/SKILL.md`, `skills/distill/SKILL.md`, `scripts/lib/status.lua`,
  `agents/task-briefer.md`.
- The pack format, if neutral text needs a new token to name a tool-specific thing.
- `extensions/claude/`, which must keep rendering the reworked packs into the same behaviour.
- The claudevs corpus and the pack chapters of the book (`ch02-*`).
- `packs/CHANGELOG.md` and the next `packs-v*` release.

## Desired outcome

- Pack text outside `native/<tool>/` names no tool-specific tool, store or file, unless the text
  is about that tool on purpose.
- The journal's `capture` and `note` work in both Claude Code and OpenCode, each reading that
  tool's own session store and format.
- On Claude Code, every pack behaves exactly as it does today. The existing Lua tests and the
  claudevs corpus stay green.
- Each OpenCode store or format claim is proven by running OpenCode v2.4.1 itself.

## Constraints

- **Rework the journal per tool, do not skip it** (decided 2026-11-24).
- Lua under airsl only. A hook or script gets only the grants it declares.
- The neutral pack format stays tool-neutral. Tool-specific content lives in `native/<tool>/` or
  comes from the extension's `values`, never as `if tool == …` branches in neutral text.
- Packs stay at `api = 0`.

## Non-goals

- The OpenCode extension itself. That is the sibling chain `2026-11-24-opencode-extension`.
- Rewriting pack content for OpenCode beyond what tool neutrality and the journal rework need
  (`foundation/02` non-goal).
- New skills, agents or packs.
- Converting `claudestacks-cmux`.
