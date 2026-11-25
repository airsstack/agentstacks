---
status: approved
created: 2026-11-25
built-on: 63f62977694c
---

# Neutral Pack Wording Implementation Plan

**Goal:** Pack text outside the journal's capture and note names no single tool's own tools.

**Architecture:** Text edits only, in `packs/core`, `packs/sdlc`, `packs/journal` (recall, link,
review) and the two guideline READMEs. Each change replaces the quoted "Now" part of one line (or
line range) with the "New" text; the rest of the line is unchanged. A rewrap keeps the file's wrap
width and changes no other words. Line numbers are those of commit `cb0c028`. The journal's capture
and note skills and agents are plan 03; the gate that keeps this true is plan 04. Text listed in
spec §6 stays as it is.

**Tech Stack:** Markdown and shell-comment text in the packs; airsl 0.1.5 for `cargo make lua`.

**Content authority:** spec §4.1 (W1-W7), §4.2 (every row but the journal capture/note rows), §6,
E16.

The neutrality check each task uses (the gate 4 pattern of spec §7):

```sh
GATE4='subagent_type|TodoWrite|Task / Agent|`(Agent|Task|Bash|Read|Write|Edit|MultiEdit|Grep|Glob|Skill|WebFetch|WebSearch|AskUserQuestion|NotebookEdit)`|CLAUDE_CONFIG_DIR|CLAUDE_SESSION_ID|\.claude/|transcript JSONL|SessionStart|PreToolUse|PostToolUse|SubagentStop|UserPromptSubmit'
```

---

## Files that change

```
packs/core/agents/{reviewer,explorer,coder}.md                         — modify: tool names → plain verbs
packs/core/scripts/install-airsl.sh                                    — modify: line 7 comment
packs/core/skills/process-guidelines/references/{agent-orchestration,model-routing}.md — modify
packs/core/skills/orchestrate/SKILL.md                                 — modify: lines 8, 18, 67, 136
packs/core/skills/context-handoff/{SKILL.md,references/protocol.md}    — modify
packs/core/skills/snapshot-{save,load}/SKILL.md                        — modify: memory-store wording
packs/sdlc/agents/{chain-reader,artifact-reviewer,task-briefer}.md     — modify
packs/sdlc/skills/{execute,distill}/SKILL.md                           — modify
packs/sdlc/scripts/status.sh                                           — modify: line 8 comment
packs/journal/skills/{recall,link,review}/SKILL.md                     — modify
packs/guideline-rust/README.md, packs/guideline-python/README.md       — modify: one line each
```

## Order of work

### Task 1 — Reword the core agents

**Files:** Modify `packs/core/agents/reviewer.md`, `packs/core/agents/explorer.md`,
`packs/core/agents/coder.md`.

**Change:**

| File:line | Now | New |
|---|---|---|
| `reviewer.md:19` | ``Invoke the installed guidelines skill via `Skill` (e.g. `rust-guidelines`)`` | ``Load the installed guidelines skill (e.g. `rust-guidelines`)`` |
| `reviewer.md:42` | `` `Bash` is for those and the DoD tooling only — no mutating commands.`` | `Shell commands are for those and the DoD tooling only — no mutating commands.` |
| `reviewer.md:100` | ``- You are a leaf: you have no `Agent` tool; do not spawn agents.`` | `- You are a leaf: you cannot spawn agents; do not try.` |
| `explorer.md:26` | ``Use `Grep` / `Glob` to find, `Read` to confirm the exact line, `Bash` for read-only inspection`` | `Search and list files to find, read to confirm the exact line, and use shell commands for read-only inspection` |
| `explorer.md:61` | ``- Read-only: you have no `Edit`/`Write`. You change nothing.`` | `- Read-only: you edit nothing and write only your report file. You change nothing else.` |
| `explorer.md:62` | ``- You are a leaf: you have no `Agent` tool; do not attempt to spawn agents.`` | `- You are a leaf: you cannot spawn agents; do not try.` |
| `coder.md:18` | ``invoke the installed guidelines skill via `Skill` to load them`` | `load the installed guidelines skill` |
| `coder.md:71` | ``- You are a leaf: you have no `Agent` tool; do not attempt to spawn other agents.`` | `- You are a leaf: you cannot spawn other agents; do not try.` |

The agents' `tools:` frontmatter does not change.

**Proof:** before the edit, `grep -rnE "$GATE4" packs/core/agents/` lists the table's lines; after
it, the same command exits 1, and `grep -nF` finds each New text in its file.

**Done when:** `cargo make lua` passes and `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green.
Commit `docs(repo): describe core agent tools in plain words`.

### Task 2 — Reword the core skills and script

**Files:** Modify `packs/core/scripts/install-airsl.sh`,
`packs/core/skills/process-guidelines/references/agent-orchestration.md`,
`packs/core/skills/process-guidelines/references/model-routing.md`,
`packs/core/skills/orchestrate/SKILL.md`, `packs/core/skills/context-handoff/SKILL.md`,
`packs/core/skills/context-handoff/references/protocol.md`,
`packs/core/skills/snapshot-save/SKILL.md`, `packs/core/skills/snapshot-load/SKILL.md`.

**Change:**

| File:line | Now | New |
|---|---|---|
| `install-airsl.sh:7` | `The SessionStart preflight` | `The session-start preflight` |
| `agent-orchestration.md:23` | ``none of them carry the `Agent` tool.`` | `none of them can spawn.` |
| `agent-orchestration.md:73-74` | ``Forbidden — agents have no `Agent` tool.`` | `Forbidden — agents cannot spawn.` |
| `model-routing.md:3` | ``anything spawned via the `Agent` tool.`` | `anything spawned as a subagent.` |
| `model-routing.md:20-21` | ``Effort is definition-only — the `Agent` tool's spawn parameters expose `model` but not `effort`, so a per-spawn override is not available.`` | ``Effort is definition-only: a spawn may set `model`, but effort comes from the agent's definition, never from the spawn.`` |
| `orchestrate/SKILL.md:8` | ``they have no `Agent` tool and cannot chain themselves`` | `they cannot spawn and cannot chain themselves` |
| `orchestrate/SKILL.md:18` | ``Spawn each via the `Agent` tool's `subagent_type`. Both dials are set in each agent's frontmatter — the spawn can override `model:`, but effort comes from the definition only.`` | ``Spawn each as a subagent by that name. Both dials are set in each agent's frontmatter — a spawn may set `model:`, but effort comes from the definition only.`` |
| `orchestrate/SKILL.md:67` | ``A file you just wrote through `Edit` or `Write` is not read back`` | `A file you just wrote or edited is not read back` |
| `orchestrate/SKILL.md:136` | ``Agents have no `Agent` tool — keep it that way.`` | `Agents cannot spawn — keep it that way.` |
| `protocol.md:104-105` | ``{{agent coder}}` writes with its `Write` tool. The read-only agents (`{{agent explorer}}`, `{{agent reviewer}}`) carry `Write` **scoped by instruction** to the handoff directory only`` | ``{{agent coder}}` writes it like any other file. The read-only agents (`{{agent explorer}}`, `{{agent reviewer}}`) may write **scoped by instruction** to the handoff directory only`` |
| `context-handoff/SKILL.md:25-26` | ``the environment of a `Bash` tool call.`` | `the environment of a shell tool call.` |
| `snapshot-save/SKILL.md:11-12` | `it is **deliberately separate from Claude's native memory tool**, whose store has size limits we are working around.` | `it is **deliberately separate from the agent tool's own memory store** (Claude Code's has size limits we are working around).` |
| `snapshot-save/SKILL.md:37` | `(NOT Claude's MEMORY.md)` | `(never MEMORY.md)` |
| `snapshot-save/SKILL.md:155-156` | ``- Write to Claude's native memory store (`~/.claude/projects/.../memory/` or any `MEMORY.md`). This skill owns the snapshot store only.`` | ``- Write to the agent tool's own memory store, or to any `MEMORY.md`. This skill owns the snapshot store only.`` |
| `snapshot-load/SKILL.md:13-15` | `**not** Claude's native memory tool. The native store has size limits we are working around; the snapshot store lives outside the repo at the path below.` | `**not** the agent tool's own memory store. The snapshot store lives outside the repo at the path below.` |
| `snapshot-load/SKILL.md:21` | ``NOT Claude's `MEMORY.md` `` | ``never `MEMORY.md` `` |
| `snapshot-load/SKILL.md:69` | `- Read Claude's native memory store. Only this snapshot store.` | `- Read the agent tool's own memory store. Only this snapshot store.` |

Kept (spec §6): "Claude Code's worktree-isolation guard" in `protocol.md:140,152`, and
`snapshot-save/SKILL.md:48-50`.

**Proof:** before the edit, `grep -nE "$GATE4"` over the eight files lists their gate lines; after
it, the same command exits 1; `grep -nF` finds each New text in its file; `grep -n "Claude's"`
over the two snapshot skills prints no line outside `snapshot-save/SKILL.md:48-50` and the New text
of row `snapshot-save/SKILL.md:11-12`.

**Done when:** `cargo make lua` passes and `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green.
Commit `docs(repo): describe core skills without one tool's tool names`.

### Task 3 — Reword the sdlc pack

**Files:** Modify `packs/sdlc/agents/chain-reader.md`, `packs/sdlc/agents/artifact-reviewer.md`,
`packs/sdlc/agents/task-briefer.md`, `packs/sdlc/skills/execute/SKILL.md`,
`packs/sdlc/skills/distill/SKILL.md`, `packs/sdlc/scripts/status.sh`.

**Change:**

| File:line | Now | New |
|---|---|---|
| `chain-reader.md:22` | ``` `Glob` enumerates, `Grep` locates, `Read` pulls the lines. `Bash` is read-only``` | `Listing files enumerates, searching locates, reading pulls the lines. Shell commands are read-only` |
| `chain-reader.md:60` | `` `Write` is for your report file alone.`` | `you write only your report file.` |
| `chain-reader.md:62` | ``- Leaf: no `Agent` tool.`` | `- Leaf: you cannot spawn agents.` |
| `artifact-reviewer.md:43` | ``(you have no `Agent` tool)`` | `(you cannot spawn agents)` |
| `artifact-reviewer.md:92` | ``- `Bash` is read-only`` | `- Shell commands are read-only` |
| `artifact-reviewer.md:95` | ``- `Write` is for your report file only.`` | `- You write only your report file.` |
| `artifact-reviewer.md:96` | ``- Leaf: no `Agent` tool.`` | `- Leaf: you cannot spawn agents.` |
| `task-briefer.md:67-68` | ``any statement about the `claude` binary, Claude Code's documented behaviour, or another system this repository does not own.`` | `any statement about an outside tool's binary or documented behaviour, or another system this repository does not own.` |
| `task-briefer.md:89` | `` `Bash` is read-only`` | `Shell commands are read-only` |
| `execute/SKILL.md:39-40` | `Put one TodoWrite item per task.` | `Put one todo item per task, if you keep a todo list.` |
| `distill/SKILL.md:42` | ``a line for the target repo's `CLAUDE.md`, or`` | `a line for the target repo's agent instructions file, or` |
| `status.sh:8` | `airsl's status back to Claude Code.` | `airsl's status back to the agent tool.` |

Kept (spec §6): `sdlc/scripts/lib/status.lua:769-770`.

**Proof:** before the edit, `grep -rnE "$GATE4" packs/sdlc --exclude=README.md --exclude='*_test.lua'`
lists the table's gate lines; after it, the command exits 1; `grep -nF` finds each New text in its
file.

**Done when:** `cargo make lua` passes and `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green.
Commit `docs(repo): describe sdlc agents and skills in plain words`.

### Task 4 — Reword the journal's recall, link and review skills

**Files:** Modify `packs/journal/skills/recall/SKILL.md`, `packs/journal/skills/link/SKILL.md`,
`packs/journal/skills/review/SKILL.md`.

**Change:**

| File:line | Now | New |
|---|---|---|
| `recall/SKILL.md:40-41` | ``subagent (Task / Agent tool, `subagent_type: {{agent recall}}`), passing`` | `subagent, passing` |
| `link/SKILL.md:46-47` | ``subagent (Task / Agent tool, `subagent_type: {{agent recall}}`), passing`` | `subagent, passing` |
| `review/SKILL.md:69` | ``subagent (`subagent_type: {{agent curator}}`), passing`` | `subagent, passing` |
| `review/SKILL.md:85` | `the next SessionStart staleness check` | `the next session-start staleness check` |

Each spawn sentence keeps its `Spawn the `{{agent …}}`` opening (W1).

**Proof:** before the edit, `grep -nE "$GATE4"` over the three files lists the four lines; after it,
the command exits 1; `grep -nF 'subagent, passing'` finds the three spawn lines.

**Done when:** `cargo make lua` passes (gates 2 and 3 included) and
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`docs(repo): describe journal spawns in plain words`.

### Task 5 — Reword the guideline READMEs

**Files:** Modify `packs/guideline-rust/README.md` (line 14), `packs/guideline-python/README.md`
(line 15).

**Change:** `points Claude at this skill` → `points the agent at this skill`, in both.

**Proof:** `grep -n 'points Claude' packs/guideline-*/README.md` lists two lines before and exits
1 after; `grep -n 'points the agent at this skill'` lists both.

**Done when:** `cargo make book-build` prints no `[ERROR]` (the book includes these READMEs) and
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`docs(repo): name the agent, not Claude, in the guideline READMEs`.

## Risks

- A line number may have moved before execute; the Now text is the anchor, the number is a guide.
- A rewrap can shift a backticked word onto another line; the Proof greps run per file, so a
  leftover is still caught.
- Riskiest step: Task 2's `protocol.md` row spans two lines and keeps two `{{agent …}}` tokens;
  gate 2 in `cargo make lua` catches a broken token.
- Not done here: the journal's capture and note (plan 03) and the gate itself (plan 04).

## Proof

- `grep -rnE "$GATE4" packs/core packs/sdlc packs/journal/skills/{recall,link,review} --exclude=README.md --exclude='*_test.lua' --exclude='statusline*.lua'`
  filtered of `^packs/[^/]+/native/` prints nothing.
- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` green; `cargo make book-build` with no `[ERROR]`.
