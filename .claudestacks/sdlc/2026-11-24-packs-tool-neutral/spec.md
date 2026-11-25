---
status: approved
created: 2026-11-25
built-on: 289de24d599e
---

# Spec: Tool-neutral pack text and a per-tool journal

The five official packs are meant to be tool-neutral outside `native/<tool>/`, but their text
tells the model to use Claude Code's tools (`Agent`, `Task`, `TodoWrite`, `Bash`, …), and the
journal's `capture` and `note` find the session transcript only in Claude Code's store. This spec:

- rewords the neutral text so it names no tool's tools;
- moves the one tool-specific journal step (the session key, the transcript and its clean-up) into
  a small file per tool under `native/<tool>/`;
- fixes the opencode extension's agent permissions, which refuse every pack agent access outside
  the project, the journal vault and handoff reports included;
- adds a pack text gate that keeps the neutral text neutral, and tests on both tools.

The core and the extension API do not change.

All probe output cited here is saved under `probes/raw/`, and the harness that produced it under
`probes/harness/`. The two OpenCode source files cited are copied under `probes/src/`, from the
v2.4.1 source tree the sibling chain used (`../2026-11-24-opencode-extension/probes/BRIEF.md:14`).
The exact new file texts are under `probes/final/`.

## 1. Proven facts

Versions: agentstacks 0.1.0 built from this worktree at `dffc959`, airsl 0.1.5, OpenCode v2.4.1
(the pinned `opencode-check` binary), BSD grep 2.6.0 and macOS `/usr/bin/mktemp`
(`probes/raw/a0-versions.txt`, `a3-gate-candidate.txt:3`, `a10-mktemp.txt`). Every OpenCode run
is isolated: HOME, XDG and (from o7 on) TMPDIR under the probe directory, with a local mock model
(`probes/harness/run.sh`, `run3.sh`, `mock2.mjs`).

### 1.1 agentstacks and the packs

| # | Fact | Proof |
|---|---|---|
| F1 | All five packs install on both tools at once: `266 to write`, and the only skips are 7 hook skips for events a tool lacks (`core/hook/1`, `core/hook/3` twice, `core/hook/8`, `core/hook/9`, `journal/hook/1`, `sdlc/hook/1`). No skill, agent or command is skipped. | `probes/raw/a1-both-tools.txt` |
| F2 | A native file at `native/<tool>/skills/journal-capture/<name>` installs into that tool's `journal-capture` skill directory, and its tokens render (`{{root}}` → `.claude/agentstacks/journal` / `.opencode/agentstacks/journal`; `{{var tmpdir}}` → `${TMPDIR:-/tmp}` on OpenCode). | `a2-native-in-skill-dir.txt`; the exact §5.2 files: `a8-final-session-rendered.txt` |
| F3 | `{{skill_dir journal/capture}}` renders in the neutral `capture` and `note` skills of both tools, to `.claude/skills/journal-capture` and `.opencode/skills/journal-capture`. | `a5-skill-dir-in-skill.txt` |
| F4 | Pack root files are written for every tool: `.opencode/agentstacks/journal/scripts/transcript-path.lua` exists after F1's install. | `a7-root-files-claudevs-check-path.txt` |
| F5 | The claudevs tests read no pack: `grep -rln "packs/"` over their test directories finds nothing (exit 1), and the same search finds `fixtures/` in `crates/claudevs/tests/corpus/corpus.toml`. `claudevs-check` runs over `crates/claudevs/tests/fixtures` (`Makefile.toml:257`). | `a7-root-files-claudevs-check-path.txt` |
| F6 | The packs' tool words for agent `tools:` are `read write edit grep glob shell skill`. The claude extension maps them to `Read Write Edit Grep Glob Bash Skill` in frontmatter only. | `extensions/claude/lib/frontmatter.lua:9-17`; `extensions/opencode/lib/frontmatter.lua:12-20` |
| F7 | The gate 4 block (§7), run by `/bin/sh -e` over today's `packs/`, prints the 46 lines of `a3-gate-candidate.txt` (in 21 files) and `pack text gate 4: tool-specific names in neutral pack text`, and exits 1. On a tree whose only hits are under `native/` and in `CHANGELOG.md` it exits 0. A neutral line whose text holds `/native/` is still reported. Without the native filter the pattern also finds 13 lines under `native/`. | `a9-gate4-block.txt`, `a4-gate-native-filter.txt` |
| F8 | `mktemp "${TMPDIR:-/tmp}/journal-XXXXXX"` prints a new path each run and creates the file with mode `-rw-------`. | `a10-mktemp.txt` |
| F9 | On Claude, `{{var session_id}}` and `{{var project_dir}}` are substitutions in skill text (`${CLAUDE_SESSION_ID}`, `${CLAUDE_PROJECT_DIR}`). On OpenCode they render `${OPENCODE_SESSION_ID}` and `${AGENTSTACKS_PROJECT_DIR}`, which OpenCode never substitutes in text; the shell has both. | `../2026-11-20-cli-and-claude-extension/spec.md:508,510`; `book/src/ch06-03-opencode.md:16-17`; `o1-live-export-service.txt` (`env.txt`) |

### 1.2 OpenCode v2.4.1

| # | Fact | Proof |
|---|---|---|
| F10 | The main agent is offered `edit generate_image generate_video glob grep question read shell skill subagent webfetch websearch write execute`: no `Agent`, no `Task`, no `todowrite`. | `o1-live-export-service.txt` (`request 1 tools:`) |
| F11 | The spawn tool is `subagent`, properties `["agent","description","prompt","model","sessionID"]`, required `["agent","description","prompt"]`. A spawned subagent (`journal-capture`) is offered no `subagent` tool. | same file (`subagent properties`, `request 3 tools:`) |
| F12 | A subagent runs in the background. The spawn's tool result reads "The subagent is working in the background (sessionID: …). You will receive a <background-result kind="subagent"> message when it goes idle …", and the result arrives later as a `<background-result … state="completed" …>` message to the main agent. | `o7-two-turns-no-auto.txt` (requests 5 and 10) |
| F13 | The session's shell has `OPENCODE_SESSION_ID`. `opencode session export "$OPENCODE_SESSION_ID"` run there exits 0 and prints JSON with keys `["info","messages"]`, `info.id` being that id. | `o1-live-export-service.txt`, `o10-running-service.txt` |
| F14 | Run in a later turn, the export holds every finished turn and the current user message: `user assistant idle user` in turn 2. | `o7-two-turns-no-auto.txt` (`export-copy.json`), `o10-running-service.txt` (`in-session export`) |
| F15 | When no background service is running, an in-session export starts one, and the session then holds a `synthetic` "The server restarted while you were working. Continue from where you left off …" message. The same session with `echo hi` or `opencode service status` in its place gets none. | `o9-restart-cause.txt` (`[control]` 0, `[status]` 0, `[export]` 1) |
| F16 | When the background service is already running, the in-session export gets no such message, whether the session runs on the service or `--standalone`. | `o10-running-service.txt`, `o11-running-service-standalone.txt` (`server-restarted messages 0`) |
| F17 | From a session's shell, `opencode service status` exits 0 and prints `stopped` when no service runs (both `run` modes), or the service URL when one runs. It starts nothing: the session gets no restart message. | `o9-restart-cause.txt` (`[status]`), `o12-standalone-status-stopped.txt`, `o10-running-service.txt` |
| F18 | A service started by an export stays up after the session ends, until `opencode service stop`. | `o6-standalone-leaves-service.txt` |
| F19 | When OpenCode was started by absolute path and is not on `PATH`, the shell cannot find `opencode`: exit 127, `zsh:1: command not found: opencode`. No variable in its environment names the binary. | `o3-not-on-path.txt` |
| F20 | Without `--auto`, the main agent reads a file outside the project and the shell creates (`mktemp`) and removes (`rm -f`) files there, with no prompt. | `o7-two-turns-no-auto.txt` (requests 2-3, `rm.rc=0`, `fixed2.txt` gone) |
| F21 | A pack subagent (`journal-capture`, rendered `permissions: [{"action":"*","effect":"deny",…},{"action":"read",…},…]`) is refused `read` and `write` outside the project: `{"error":{"type":"permission.rejected","message":"Permission denied: external_directory"}}`. Its shell `ls` there succeeds. | `o7-two-turns-no-auto.txt` (requests 6-9, `child-ls.rc=0`) |
| F22 | With `{"action":"external_directory","effect":"allow","resource":"*"}` placed right after the deny-all rule, the same subagent reads (`… 1: outside`) and writes (`Created file successfully`) outside the project. | `o8-service-export-and-external-allow.txt` (second half) |
| F23 | A session id is `ses_` + 12 hex characters from the time + 14 random base62 characters. So the first eight characters repeat: three sessions started minutes apart all began `ses_deef`. | `probes/src/packages/schema/src/session-id.ts:9`, `identifier.ts:16-31`; `OPENCODE_SESSION_ID=` lines in `o1-*.txt`, `o3-not-on-path.txt` |
| F24 | With both tools installed, OpenCode also lists `.claude/skills/`, and for a skill id in both, the `.opencode/skills/` copy wins. | `../2026-11-24-opencode-extension/probes/skills-results.md:865` |
| F25 | No probe left a process running. | `o5-no-leftovers.txt` and the `pgrep … exit=1` lines of o6, o9, o10, o11, o12; in o8 the one match is the `tee` that captured the file |

## 2. Decisions

| # | Decision | Settled by |
|---|---|---|
| D1 | Each tool's session key, transcript and clean-up live in the journal pack, in `native/<tool>/skills/journal-capture/session.md`. The neutral `capture` and `note` skills read that file. | author, 2026-11-25 |
| D2 | Neutral text names no tool's tools. Delegation is plain prose ("Spawn the `{{agent capture}}` subagent, passing …"). An agent's own tools are described by plain verbs ("Shell commands are read-only", "You write only your report file"), never by tool names, so the text reads true on every tool (F6). A todo list is optional ("if you keep one"), since OpenCode offers none (F10). | author, 2026-11-25 |
| D3 | The opencode extension allows `external_directory` for every agent with `tools`, right after the deny-all rule (F21, F22). | author, 2026-11-25 (intent amended the same day) |
| D4 | Text that is about one tool on purpose stays (§6). | §6 |
| D5 | On OpenCode, `id8` is the last eight characters of the session id, in lower case (F23). Lower case keeps stems kebab-case (`journal/references/note-schema.md:46`), and the index lowercases stems anyway (`journal/scripts/lib/index.lua:56-57`). On Claude it stays the first eight, so existing Claude vaults keep their keys. | this spec |
| D6 | On OpenCode the transcript is exported only when the background service is already running (F15-F17), into a fresh `mktemp` file (F8). The spawned agent removes it after reading. With no service, the journal is written without a transcript, and the user is told why. | this spec |
| D7 | Pack versions: `core` 0.1.0 → 0.1.1 and `sdlc` 0.1.0 → 0.1.1 (wording only); `journal` 0.1.0 → 0.2.0 (it now captures on OpenCode). The guideline packs keep 0.1.0: only their README changes, and `README.md` is excluded from every pack (`exclude = ["README.md", …]`). The opencode extension stays 0.1.0: it is unreleased (`packs/CHANGELOG.md:8-12`). | this spec |

## 3. Architecture

The author approved the pack, gate and test layout on 2026-11-25, and added the extension fix (D3)
the same day.

```text
packs/  (neutral text: the 46 gate lines in 21 files reworded, §4, §5)
  journal/
    skills/{capture,note}/SKILL.md      read <project_dir>/{{skill_dir journal/capture}}/session.md
    agents/{capture,note}.md            new input `cleanup`, run before the receipt
    native/claude/skills/journal-capture/session.md     id8 first 8 · tpath via transcript-path.lua
    native/opencode/skills/journal-capture/session.md   id8 last 8 · service check · export · cleanup
    scripts/transcript-path.lua         unchanged (Claude-only on purpose, §6)
  {core,sdlc,journal}/pack.toml         versions (D7)
  CHANGELOG.md, journal + guideline READMEs   §9
extensions/opencode/lib/frontmatter.lua the external_directory allow rule (§6a)
Makefile.toml [tasks.lua]               pack text gate 4 (§7)
crates/agentstacks-cli/tests/packs.rs   both tools installed; gate 4 asserted (§10)
crates/agentstacks-cli/tests/opencode-check/check.mjs   scenarios S8a, S8b (§10)
crates/agentstacks/tests/fixtures/opencode/*/expected/   the new permissions line (§6a)
book/src/ch06-03-opencode.md            one sentence on agent permissions (§6a)
```

- No Rust module, type or trait is added or moved. `modularity.md:1`, `mod-rs-export-only.md:3` and
  `static-dispatch.md:1` (rust-guidelines references) are met by adding no code to `src/`.
- `unit-test-mandate.md:3`: integration tests complement unit tests. The new behaviour is pack text
  and one Lua rule, so the Lua unit test (`frontmatter_test.lua`), the integration tests in
  `packs.rs` and `opencode.rs`, and the real-OpenCode check prove it. No `src/` file gains logic.
- `doc-comment-discipline.md:3,13`: the new tests' doc comments name behaviour, never this chain,
  its plans or `.claudestacks/` paths.
- Layering: the core stays tool-agnostic. Claude's and OpenCode's store knowledge lives only in the
  journal's `native/<tool>/` files, and OpenCode's permission shape only in the opencode extension
  (`intent.md` Constraints).

The flow of `capture` and `note`, the same on every tool:

```text
1 session_id = {{var session_id}} · project_dir = {{var project_dir}}  (echo either if still ${…})
2 session id empty ──► no session.md; capture: timestamp stem · note: no id8; empty tpath, cleanup
  else read <project_dir>/{{skill_dir journal/capture}}/session.md ──► id8, tpath, cleanup
       no session.md ──► id8 = first 8, empty tpath and cleanup
3 project floor · 4 provision the vault                                  (unchanged)
5 spawn {{agent capture}} / {{agent note}} with session_id, id8, (stem), tpath, cleanup, project
      the agent reads tpath, writes, runs cleanup, returns the receipt
6 relay the receipt when it arrives (OpenCode: a later background-result, F12)
  agent cannot be spawned ──► the skill runs cleanup itself
```

## 4. Neutral wording

### 4.1 Rules

| Rule | Neutral text names | Instead of |
|---|---|---|
| W1 | "Spawn the `{{agent x}}` subagent, passing …" | "(Task / Agent tool, `subagent_type: {{agent x}}`)", "via the `Agent` tool's `subagent_type`" |
| W2 | "you cannot spawn agents" | "you have no `Agent` tool" |
| W3 | plain verbs: "search", "read", "Shell commands are …", "You write only …" | `Bash`, `Write`, `Edit`, `Grep`, `Glob`, `Read`, `Skill` |
| W4 | "one todo item per task, if you keep a todo list" | "TodoWrite item per task" |
| W5 | "session-start" | `SessionStart` |
| W6 | "the agent tool's own memory store" | "Claude's native memory tool/store", `~/.claude/projects/.../memory/` |
| W7 | "the session transcript" | "transcript JSONL" |

A spawn may still set `model:`; effort still comes only from the agent's definition. This is the
packs' policy and stays as policy text, with no claim about either tool's spawn parameters.

### 4.2 Every change

Line numbers are today's. Each "Now" quote is from `probes/raw/a6-lines-now.txt` (every changed
line, printed by `probes/harness/lines.sh`) or `probes/raw/a3-gate-candidate.txt`. "New" is the
exact replacement for the quoted part; the rest of the line is unchanged. A rewrap keeps the file's
wrap width and changes no other words.

| File:line | Now | New |
|---|---|---|
| `core/agents/reviewer.md:19` | ``Invoke the installed guidelines skill via `Skill` (e.g. `rust-guidelines`)`` | ``Load the installed guidelines skill (e.g. `rust-guidelines`)`` |
| `core/agents/reviewer.md:42` | `` `Bash` is for those and the DoD tooling only — no mutating commands.`` | `Shell commands are for those and the DoD tooling only — no mutating commands.` |
| `core/agents/reviewer.md:100` | ``- You are a leaf: you have no `Agent` tool; do not spawn agents.`` | `- You are a leaf: you cannot spawn agents; do not try.` |
| `core/agents/explorer.md:26` | ``Use `Grep` / `Glob` to find, `Read` to confirm the exact line, `Bash` for read-only inspection`` | `Search and list files to find, read to confirm the exact line, and use shell commands for read-only inspection` |
| `core/agents/explorer.md:61` | ``- Read-only: you have no `Edit`/`Write`. You change nothing.`` | `- Read-only: you edit nothing and write only your report file. You change nothing else.` |
| `core/agents/explorer.md:62` | ``- You are a leaf: you have no `Agent` tool; do not attempt to spawn agents.`` | `- You are a leaf: you cannot spawn agents; do not try.` |
| `core/agents/coder.md:18` | ``invoke the installed guidelines skill via `Skill` to load them`` | `load the installed guidelines skill` |
| `core/agents/coder.md:71` | ``- You are a leaf: you have no `Agent` tool; do not attempt to spawn other agents.`` | `- You are a leaf: you cannot spawn other agents; do not try.` |
| `core/scripts/install-airsl.sh:7` | `The SessionStart preflight` | `The session-start preflight` |
| `core/skills/process-guidelines/references/agent-orchestration.md:23` | ``none of them carry the `Agent` tool.`` | `none of them can spawn.` |
| `…/agent-orchestration.md:73-74` | ``Forbidden — agents have no `Agent` tool.`` | `Forbidden — agents cannot spawn.` |
| `…/model-routing.md:3` | ``anything spawned via the `Agent` tool.`` | `anything spawned as a subagent.` |
| `…/model-routing.md:20-21` | ``Effort is definition-only — the `Agent` tool's spawn parameters expose `model` but not `effort`, so a per-spawn override is not available.`` | ``Effort is definition-only: a spawn may set `model`, but effort comes from the agent's definition, never from the spawn.`` |
| `core/skills/orchestrate/SKILL.md:8` | ``they have no `Agent` tool and cannot chain themselves`` | `they cannot spawn and cannot chain themselves` |
| `core/skills/orchestrate/SKILL.md:18` | ``Spawn each via the `Agent` tool's `subagent_type`. Both dials are set in each agent's frontmatter — the spawn can override `model:`, but effort comes from the definition only.`` | ``Spawn each as a subagent by that name. Both dials are set in each agent's frontmatter — a spawn may set `model:`, but effort comes from the definition only.`` |
| `core/skills/orchestrate/SKILL.md:67` | ``A file you just wrote through `Edit` or `Write` is not read back`` | `A file you just wrote or edited is not read back` |
| `core/skills/orchestrate/SKILL.md:136` | ``Agents have no `Agent` tool — keep it that way.`` | `Agents cannot spawn — keep it that way.` |
| `core/skills/context-handoff/references/protocol.md:104-105` | ``{{agent coder}}` writes with its `Write` tool. The read-only agents (`{{agent explorer}}`, `{{agent reviewer}}`) carry `Write` **scoped by instruction** to the handoff directory only`` | ``{{agent coder}}` writes it like any other file. The read-only agents (`{{agent explorer}}`, `{{agent reviewer}}`) may write **scoped by instruction** to the handoff directory only`` |
| `core/skills/context-handoff/SKILL.md:25-26` | ``the environment of a `Bash` tool call.`` | `the environment of a shell tool call.` |
| `core/skills/snapshot-save/SKILL.md:11-12` | `it is **deliberately separate from Claude's native memory tool**, whose store has size limits we are working around.` | `it is **deliberately separate from the agent tool's own memory store** (Claude Code's has size limits we are working around).` |
| `core/skills/snapshot-save/SKILL.md:37` | `(NOT Claude's MEMORY.md)` | `(never MEMORY.md)` |
| `core/skills/snapshot-save/SKILL.md:155-156` | ``- Write to Claude's native memory store (`~/.claude/projects/.../memory/` or any `MEMORY.md`). This skill owns the snapshot store only.`` | ``- Write to the agent tool's own memory store, or to any `MEMORY.md`. This skill owns the snapshot store only.`` |
| `core/skills/snapshot-load/SKILL.md:13-15` | `**not** Claude's native memory tool. The native store has size limits we are working around; the snapshot store lives outside the repo at the path below.` | `**not** the agent tool's own memory store. The snapshot store lives outside the repo at the path below.` |
| `core/skills/snapshot-load/SKILL.md:21` | ``NOT Claude's `MEMORY.md` `` | ``never `MEMORY.md` `` |
| `core/skills/snapshot-load/SKILL.md:69` | `- Read Claude's native memory store. Only this snapshot store.` | `- Read the agent tool's own memory store. Only this snapshot store.` |
| `sdlc/agents/chain-reader.md:22` | ``` `Glob` enumerates, `Grep` locates, `Read` pulls the lines. `Bash` is read-only``` | `Listing files enumerates, searching locates, reading pulls the lines. Shell commands are read-only` |
| `sdlc/agents/chain-reader.md:60` | `` `Write` is for your report file alone.`` | `you write only your report file.` |
| `sdlc/agents/chain-reader.md:62` | ``- Leaf: no `Agent` tool.`` | `- Leaf: you cannot spawn agents.` |
| `sdlc/agents/artifact-reviewer.md:43` | ``(you have no `Agent` tool)`` | `(you cannot spawn agents)` |
| `sdlc/agents/artifact-reviewer.md:92` | ``- `Bash` is read-only`` | `- Shell commands are read-only` |
| `sdlc/agents/artifact-reviewer.md:95` | ``- `Write` is for your report file only.`` | `- You write only your report file.` |
| `sdlc/agents/artifact-reviewer.md:96` | ``- Leaf: no `Agent` tool.`` | `- Leaf: you cannot spawn agents.` |
| `sdlc/agents/task-briefer.md:67-68` | ``any statement about the `claude` binary, Claude Code's documented behaviour, or another system this repository does not own.`` | `any statement about an outside tool's binary or documented behaviour, or another system this repository does not own.` |
| `sdlc/agents/task-briefer.md:89` | `` `Bash` is read-only`` | `Shell commands are read-only` |
| `sdlc/skills/execute/SKILL.md:39-40` | `Put one TodoWrite item per task.` | `Put one todo item per task, if you keep a todo list.` |
| `sdlc/skills/distill/SKILL.md:42` | ``a line for the target repo's `CLAUDE.md`, or`` | `a line for the target repo's agent instructions file, or` |
| `sdlc/scripts/status.sh:8` | `airsl's status back to Claude Code.` | `airsl's status back to the agent tool.` |
| `journal/skills/recall/SKILL.md:40-41` | ``subagent (Task / Agent tool, `subagent_type: {{agent recall}}`), passing`` | `subagent, passing` |
| `journal/skills/link/SKILL.md:46-47` | ``subagent (Task / Agent tool, `subagent_type: {{agent recall}}`), passing`` | `subagent, passing` |
| `journal/skills/review/SKILL.md:69` | ``subagent (`subagent_type: {{agent curator}}`), passing`` | `subagent, passing` |
| `journal/skills/review/SKILL.md:85` | `the next SessionStart staleness check` | `the next session-start staleness check` |
| `journal/skills/{capture,note}/SKILL.md` (capture 31, 64-65; note 31, 63-64) | the session, transcript and spawn steps | §5.1 |
| `journal/agents/capture.md` (5, 22, 28), `journal/agents/note.md` (22), and their other input lines | Claude session id, transcript JSONL | §5.3 |
| `guideline-rust/README.md:14`, `guideline-python/README.md:15` | `points Claude at this skill` | `points the agent at this skill` |

These rows and §5 cover all 46 lines of `a3-gate-candidate.txt`.

## 5. Journal: per-tool session key, transcript and clean-up

### 5.1 The neutral skills

`journal/skills/capture/SKILL.md`: steps 1, 2, 5 and 6 and "If the agent is unavailable" become the
text below. Steps 3 and 4, and the worktree-guard paragraph, are unchanged.

```markdown
1. Resolve the session id and the project directory. The session id is
   `{{var session_id}}` and the project directory is `{{var project_dir}}`. If
   either still reads as a `${…}` shell variable, read its value with `echo`.

2. If the session id is empty, do not read `session.md`: set `stem` =
   `session-$(date +%Y%m%d-%H%M%S)`, pass an empty `tpath` and an empty
   `cleanup`, and note that the story cannot be keyed to the session id.
   Otherwise read `<project_dir>/{{skill_dir journal/capture}}/session.md` and
   follow it with that session id. It gives `id8`, the eight-character session
   key; `tpath`, the transcript file; and `cleanup`, a command that removes a
   temporary transcript (often empty). Set `stem` = `session-<id8>`. If
   `session.md` does not exist, this tool has no transcript lookup: take `id8`
   = the first eight characters of the session id, and pass an empty `tpath`
   and an empty `cleanup`.

5. Spawn the `{{agent capture}}` subagent, passing `session_id`, `id8`, `stem`,
   `tpath`, `cleanup`, and `project`. The agent reads the transcript, overwrites
   `sessions/<stem>.md`, links the daily note, refreshes the index, runs
   `cleanup`, and returns a one-line receipt.

6. When the agent's receipt arrives, relay it to the user. Do not write any
   note file yourself.

## If the agent is unavailable

If the `{{agent capture}}` subagent cannot be spawned (the pack's agents are
not installed), tell the user, run `cleanup` if it is set, and stop. Do nothing
destructive.
```

`journal/skills/note/SKILL.md`: steps 2, 3, 6 and 7 and "If the agent is unavailable" become the
text below. Steps 1, 4 and 5, and the worktree-guard paragraph, are unchanged.

```markdown
2. Resolve the session id and the project directory. The session id is
   `{{var session_id}}` and the project directory is `{{var project_dir}}`. If
   either still reads as a `${…}` shell variable, read its value with `echo`.

3. If the session id is empty, do not read `session.md`: pass an empty `id8`,
   an empty `tpath` and an empty `cleanup`; the note then carries no `session:`
   join value. Otherwise read
   `<project_dir>/{{skill_dir journal/capture}}/session.md` and follow it with
   that session id. It gives `id8`, the eight-character session key; `tpath`,
   the transcript file; and `cleanup`, a command that removes a temporary
   transcript (often empty). If `session.md` does not exist, this tool has no
   transcript lookup: take `id8` = the first eight characters of the session
   id, and pass an empty `tpath` and an empty `cleanup`.

6. Spawn the `{{agent note}}` subagent, passing `topic`, `session_id`, `id8`,
   `tpath`, `cleanup`, and `project`. The agent derives the kebab stem
   (rejecting `_unresolved`), creates or updates `notes/<stem>.md`, links the
   daily note, refreshes the index, runs `cleanup`, and returns a one-line
   receipt.

7. When the agent's receipt arrives, relay it to the user. Do not write any
   note file yourself.

## If the agent is unavailable

If the `{{agent note}}` subagent cannot be spawned, tell the user, run
`cleanup` if it is set, and stop. Do nothing destructive.
```

### 5.2 The native files

Their exact texts are `probes/final/claude-session.md` and `probes/final/opencode-session.md`,
copied byte for byte to:

- `packs/journal/native/claude/skills/journal-capture/session.md`
- `packs/journal/native/opencode/skills/journal-capture/session.md`

`a8-final-session-rendered.txt` shows both installed and rendered. The Claude file:

````markdown
# Session key and transcript

The journal's capture and note skills follow this file with the session id and
the project directory they resolved.

- `id8`: the first eight characters of the session id.
- `tpath`: run

  ```sh
  airsl run --policy confined \
    --allow-env CLAUDE_CONFIG_DIR --allow-env HOME --allow-env PWD \
    --allow-read . --allow-read "$HOME/.claude" \
    "<project_dir>/{{root}}/scripts/transcript-path.lua" "<session_id>"
  ```

  and take its stdout as `tpath`. An empty result or a non-zero exit means no
  transcript was found: pass an empty `tpath`.
- `cleanup`: empty.
````

It runs today's script with today's grants. Only `{{var project_dir}}` and `{{var session_id}}`
become the values step 1 resolved, and on Claude those are the same values (F9). The supporting
file is read by the model, so it holds no var that would need a substitution there.

The OpenCode file:

````markdown
# Session key and transcript

The journal's capture and note skills follow this file with the session id and
the project directory they resolved.

- `id8`: the last eight characters of the session id, in lower case. Its first
  characters come from the time and repeat across sessions.
- `tpath` and `cleanup`: run `opencode service status`.
  - Exit 127 means `opencode` is not on `PATH`. Tell the user `opencode is not
    on PATH, so the journal was written without the transcript`, and pass an
    empty `tpath` and an empty `cleanup`.
  - If it prints `stopped`, this session does not run on OpenCode's background
    service, and an export would start the service, which marks this session
    as restarted. Tell the user `this OpenCode session does not run on the
    background service, so the journal was written without the transcript`,
    and pass an empty `tpath` and an empty `cleanup`.
  - Otherwise run `mktemp "{{var tmpdir}}/journal-XXXXXX"` and take its output
    as `<export>`, then run

    ```sh
    opencode session export "<session_id>" > "<export>"
    ```

    On exit 0, `tpath` is `<export>` and `cleanup` is `rm -f "<export>"`. On a
    non-zero exit, run `rm -f "<export>"` yourself, then pass an empty `tpath`
    and an empty `cleanup`.
````

- Each run exports to its own `mktemp` file (F8), so a `capture` and a `note` whose background
  agents overlap (F12) never share a file, and each agent removes only its own.
- The export is the session as stored when it runs: every finished turn and the current user
  message (F14).
- Neither file contains `journal-capture` as text, so gate 3 still passes. Both are under
  `native/`, which gate 4 filters out (§7).

### 5.3 Agents and the note schema

`journal/agents/capture.md`:

| Line | New text |
|---|---|
| 4-5 (description) | only `session transcript JSONL` → `session transcript` |
| 19 | ``- `session_id` — the full session id.`` |
| 20 | ``- `id8` — the eight-character session key the skill took from `session.md`.`` |
| 22-23 | ``- `tpath` — absolute path to the session transcript, in the tool's own format (resolved by the skill through `session.md`), or empty if none was found.`` |
| new line after 23 | ``- `cleanup` — a shell command that removes a temporary transcript, or empty.`` |
| 28 | `read it (the full transcript)` |
| 63 (step 8) | ``8. Run `cleanup` once if it is set, whether or not you could read the transcript. Then return a one-line receipt, e.g.`` |

`journal/agents/note.md`:

| Line | New text |
|---|---|
| 21 | ``- `session_id`, `id8` — the session id and its eight-character session key.`` |
| 22 | ``- `tpath` — absolute path to the session transcript, or empty if none.`` |
| new line after 22 | ``- `cleanup` — a shell command that removes a temporary transcript, or empty.`` |
| 62 (step 8) | ``8. Run `cleanup` once if it is set, whether or not you could read the transcript. Then return a one-line receipt, e.g.`` (the rest of the step is unchanged) |

`journal/references/note-schema.md:112-113`: ``where `<id8>` is the eight-character session key
the capture and note skills take from `session.md` (the same `<id8>` used in the `session-<id8>`
story stem).``

## 6. Kept as is (about one tool on purpose)

| Text | Why it stays |
|---|---|
| `journal/scripts/transcript-path.lua` | Claude's store lookup; its header says so. Only the Claude `session.md` runs it. It is also written for OpenCode, unused (F4). |
| `core/scripts/statusline*.lua`, `core/scripts/lib/statusline.lua` and their tests | the Claude Code status line, driven by `core/native/claude/skills/core-statusline/` |
| "Claude Code's worktree-isolation guard" in `core/scripts/handoff.lua:17`, `core/scripts/lib/handoff.lua:29`, `core/skills/context-handoff/references/protocol.md:140,152` | the reason the scripts take no `--allow-exec git`; a true statement about Claude Code |
| `sdlc/scripts/lib/status.lua:769-770` | the reason for the 9,000-byte cap: Claude Code caps a hook's stdout |
| `core/skills/snapshot-save/SKILL.md:48-50` | why the index is never named `MEMORY.md` |
| `CLAUDE.md` in `core/scripts/lib/handoff_report.lua:97,326` and tests | a file name used as an example of a non-report path |
| `guideline-rust/…/doc-comment-discipline.md`, `guideline-python/…/docstring-discipline.md` | they list `Claude`, `Opus`, `SubagentStop` as words to keep out of code comments |
| every `*_test.lua` | test data and test comments |
| the claude.com link in `sdlc/README.md`; `agentstacks init --tool claude` in the `core`, `sdlc` and guideline READMEs | documentation; only the journal README gains an OpenCode line (§9), since only the journal behaves differently per tool |
| the worktree-guard paragraph in `journal/skills/{capture,note}/SKILL.md` | names no tool; it describes a guard |

## 6a. The opencode extension's agent permissions

`extensions/opencode/lib/frontmatter.lua`: `permissions(tools)` returns, when `tools` is present
(empty or not), the deny-all rule, then `{ action = "external_directory", effect = "allow",
resource = "*" }`, then one allow rule per mapped tool as today. An absent `tools` still gives
`[]`. The rendered line for `tools: [read, write, edit, shell, glob, grep]` is exactly the patched
line F22 ran (`o8-service-export-and-external-allow.txt`):

```text
permissions: [{"action":"*","effect":"deny","resource":"*"},{"action":"external_directory","effect":"allow","resource":"*"},{"action":"read","effect":"allow","resource":"*"},{"action":"edit","effect":"allow","resource":"*"},{"action":"shell","effect":"allow","resource":"*"},{"action":"glob","effect":"allow","resource":"*"},{"action":"grep","effect":"allow","resource":"*"}]
```

- `tools: []` gives `[deny-all, external_directory allow]`. The agent still has no tool, so the
  allow grants nothing it can use.
- Follows from it: `extensions/opencode/lib/frontmatter_test.lua` (the rule list in
  `agent_with_empty_tools_denies_all`, `agent_tools_map_to_permissions` and the rule-building test
  at line 65), and the `permissions:` line of
  `crates/agentstacks/tests/fixtures/opencode/{demo,full}/expected/.opencode/agents/*.md`.
- `book/src/ch06-03-opencode.md:33`, the agent row, gains: `Its \`tools\` become \`permissions\`:
  deny everything, allow paths outside the project, then allow each listed tool.`
- This changes no hook, shim or runtime file, so `runtime.lua`'s hash is untouched.

## 7. Pack text gate 4

Added to `[tasks.lua]` in `Makefile.toml` after gate 3. The block is exactly
`probes/final/gate4.sh`, which F7 ran:

```sh
# Gate 4 keeps neutral pack text free of one tool's tool names, events and store. It reads
# grep's status before filtering out native/<tool>/, as gate 3 does. The excluded files are
# about one tool on purpose, or are history (CHANGELOG.md).
rc=0
found=$(grep -rnE 'subagent_type|TodoWrite|Task / Agent|`(Agent|Task|Bash|Read|Write|Edit|MultiEdit|Grep|Glob|Skill|WebFetch|WebSearch|AskUserQuestion|NotebookEdit)`|CLAUDE_CONFIG_DIR|CLAUDE_SESSION_ID|\.claude/|transcript JSONL|SessionStart|PreToolUse|PostToolUse|SubagentStop|UserPromptSubmit' packs/ --exclude=README.md --exclude=CHANGELOG.md --exclude='*_test.lua' --exclude='statusline*.lua' --exclude=transcript-path.lua --exclude=doc-comment-discipline.md --exclude=docstring-discipline.md) || rc=$?
if [ "$rc" -gt 1 ]; then
  echo "pack text gate 4: grep failed over packs/ (exit $rc)" >&2
  exit 1
fi
left=$(printf '%s\n' "$found" | grep -vE '^packs/[^/]+/native/' || true)
if [ -n "$left" ]; then
  printf '%s\n' "$left"
  echo "pack text gate 4: tool-specific names in neutral pack text" >&2
  exit 1
fi
```

- `Makefile.toml:201` becomes `echo "pack text gates: 4 passed"`.
- `packs.rs` `the_lua_gate_tests_the_packs_and_is_part_of_dod` loops `for gate in 1..=4`
  (`crates/agentstacks-cli/tests/packs.rs:751`).
- On today's tree the block reports exactly the 46 lines and exits 1 (F7). After §4-§5 it finds none.
- It uses only `grep -rnE`, `--exclude` and a `grep -v` filter applied after grep's status is read,
  as gates 1-3 do on CI (`Makefile.toml:175,183,190,195`). The pattern holds no `'''`, so it fits
  the TOML literal string.

## 8. Edge cases

| # | Case | Result | Owner |
|---|---|---|---|
| E1 | Session id empty | `session.md` is not read. capture: timestamp stem, empty `tpath` and `cleanup`. note: empty `id8`, `tpath`, `cleanup`, no `session:` join. Same outcome as today. | §5.1 |
| E2 | No `session.md` (a tool with no native file) | `id8` = first eight characters, empty `tpath` and `cleanup`; the agent writes with partial coverage, as today. | §5.1 |
| E3 | `opencode` not on `PATH` (F19) | `service status` exits 127; the user is told `opencode is not on PATH, so the journal was written without the transcript`. | OpenCode `session.md` |
| E4 | No background service running (F15, F17) | `service status` prints `stopped`; no export; the user is told `this OpenCode session does not run on the background service, so the journal was written without the transcript`. The session is not interrupted. | OpenCode `session.md` |
| E5 | Service running, session on it or `--standalone` (F16) | export, no interruption. | OpenCode `session.md` |
| E6 | Export fails another way | the skill removes `<export>`, passes empty `tpath` and `cleanup`; partial coverage. | OpenCode `session.md` |
| E7 | The background subagent's receipt (F12) | arrives in a later message; the skill relays it then. On Claude the spawn returns the receipt directly. | §5.1 |
| E8 | A `capture` and a `note` overlap | separate `mktemp` files (F8); each agent removes its own. | §5.2 |
| E9 | The agent cannot be spawned | the skill runs `cleanup` itself. | §5.1 |
| E10 | The agent stops before its last step | the export stays in `<tmpdir>`, mode `-rw-------` (F8). Accepted. | — |
| E11 | Both tools installed | OpenCode uses its own `journal-capture` copy, and so its own `session.md` (F24). | OpenCode's rule |
| E12 | Two OpenCode sessions minutes apart | different `id8`: the last eight characters are random (F23). | D5 |
| E13 | Existing Claude vault | same `id8` rule, so the same stems and `session:` joins. | D5 |
| E14 | Files outside the project | the main agent reads, creates and removes them without a prompt (F20). Agents can after §6a (F21, F22). | §6a |
| E15 | An agent with `tools: []` | deny-all plus the external allow; still no usable tool. | §6a |
| E16 | A third tool with no todo list | W4 makes the todo list optional. | §4 |

## 9. Versions, changelog and docs

- `packs/core/pack.toml` `version = "0.1.1"`, `packs/sdlc/pack.toml` `version = "0.1.1"`,
  `packs/journal/pack.toml` `version = "0.2.0"` (D7).
- `packs/CHANGELOG.md`, under `## [Unreleased]`, after the `### Added` list:

  ```markdown
  ### Changed

  - `journal` 0.2.0: `capture` and `note` read the session transcript on OpenCode too, through
    `opencode session export`, when OpenCode runs its background service. On OpenCode the session
    key is the last eight characters of the session id, in lower case.
  - `core` 0.1.1, `sdlc` 0.1.1 and `journal`: skill and agent text no longer names one tool's own
    tools; spawning and todo lists are described in plain words.
  - The `opencode` extension lets agents read and write outside the project, as OpenCode's own
    default does, so pack agents reach the journal vault and their handoff reports.
  ```

- `packs/journal/README.md`, after the `agentstacks init` block in `## Install`:

  ```markdown
  On OpenCode, pass `--tool opencode` (or both tools). `capture` and `note` read the session
  through `opencode session export`, which needs `opencode` on `PATH` and OpenCode's background
  service running; otherwise they write without the transcript and say why.
  ```

- The book's pack chapters `ch02-01` to `ch02-05` include the pack READMEs (`{{#include}}`), so they
  change with them. `ch06-03` changes as §6a says. `cargo make book-build` prints no `[ERROR]`.

## 10. Testing

| Test | Where | What it proves |
|---|---|---|
| pack text gate 4 | `cargo make lua` (part of `dod`) | no neutral pack text names a tool's tools, events or store |
| gates 1-3 | `cargo make lua` | still pass with the new native files |
| `airsl test` over `extensions/` and `packs/` | `cargo make lua` | the updated `frontmatter_test.lua` and every other Lua test pass |
| `the_lua_gate_tests_the_packs_and_is_part_of_dod` | `packs.rs` | gates 1-4 are in the lua task |
| `five_packs_install_on_both_tools` | `packs.rs` | `init` with `ext/claude`, `ext/opencode` and the five packs exits 0, and its skip lines are exactly the 7 of F1 |
| `journal_session_lookup_is_per_tool` | `packs.rs` | `.claude/skills/journal-capture/session.md` and `.opencode/skills/journal-capture/session.md` equal the rendered texts of `a8-final-session-rendered.txt` |
| `opencode_instructions_name_no_claude_tools` | `packs.rs` | no file under `.opencode/skills/`, `.opencode/agents/` or `.opencode/commands/` matches gate 4's pattern, except the two files gate 4 also excludes (`doc-comment-discipline.md`, `docstring-discipline.md`, rendered as skill references) |
| `opencode_agents_reach_outside_the_project` | `packs.rs` | every `.opencode/agents/*.md` has the `external_directory` allow rule right after the deny-all rule |
| the `opencode` expected fixtures | `crates/agentstacks/tests/opencode.rs` | the rendered agents equal the updated expected files |
| S8a `journal export, no service` | `check.mjs` | in a `--standalone` session with no background service, the shell's `opencode service status` prints `stopped` and exits 0; after the run, the session has no message whose text starts with `The server restarted` |
| S8b `journal export, running service` | `check.mjs` | after `opencode service start` in the isolated HOME, a `--standalone` session's shell runs `opencode session export "$OPENCODE_SESSION_ID" > export.json`: exit 0, the file parses, `info.id` equals the session id; after the run, no `The server restarted` message; then `opencode service stop` |
| `claudevs-check` | `cargo make claudevs-check` | unchanged; its tests read no pack (F5) |

- S8a and S8b read the session after the run the way `o12-standalone-status-stopped.txt` does:
  `opencode service stop`, then `opencode session export --standalone <id>`.
- `check.mjs` runs OpenCode by absolute path and hands it the caller's `PATH` (`check.mjs:33,141,257`),
  on which the shell cannot find `opencode` (F19). So S8a and S8b, unlike S1-S7, prepend to
  OpenCode's `PATH` a scratch directory holding a link named `opencode` to the pinned binary.
- `packs.rs`'s project setup copies `extensions/opencode` as well as `extensions/claude`.
- Done means `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod`, `cargo make opencode-check` (S1-S8b) and
  `cargo make book-build` all pass.

## 11. Risks

| # | Risk | Owner |
|---|---|---|
| K1 | A model reads neutral prose ("Spawn the … subagent", "Shell commands are read-only") instead of a named tool. Both tools offer a spawn tool (F11; Claude's is unchanged), but whether a model picks it is not deterministic, and no test can prove it. | maintainers |
| K2 | OpenCode V2 moves fast: `session export`, its JSON, or the restart behaviour of F15 could change. S8a and S8b catch that at the next pin bump. | maintainers |
| K3 | The native paths `skills/journal-capture/` follow each extension's `<pack>-<skill>` naming. If an extension renamed skill directories, `session.md` would land elsewhere and E2 would apply. `journal_session_lookup_is_per_tool` catches it. | `packs.rs` |
| K4 | `mktemp` with a trailing `XXXXXX` template and `grep -vE` were run here on macOS only (F7, F8). CI (`ubuntu-latest`, GNU) runs gate 4 on every push, which proves the grep side. The `mktemp` side runs on the user's machine. | CI, maintainers |
| K5 | A user whose OpenCode never runs the background service always gets E4: a journal without the transcript. | this spec (by design) |

## 12. Non-goals

- The rest of the OpenCode extension (sibling chain `2026-11-24-opencode-extension`, done). Only
  §6a changes in it.
- Rewriting pack content beyond tool neutrality and the journal rework, including typos in lines
  this spec does not change.
- New skills, agents or packs; converting `claudestacks-cmux`.
- Changes to the core or the extension API; packs stay `api = 0`.
- Sanitising the OpenCode export (`--sanitize`): the Claude path reads the raw transcript, and so
  does this one.
- Starting OpenCode's background service for the user.
- Hooks that OpenCode cannot run (the 7 skips of F1).
