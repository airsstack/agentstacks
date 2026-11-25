# OpenCode

What the official `opencode` extension does: the files it writes, the variables it gives packs, how
hooks reach OpenCode, what it skips, and what it cannot do. It is checked against OpenCode v2.4.1
(V2 project config). It runs on the same `api = 0` as every extension; see
[the extension API](ch06-01-extensions.md) and [hooks](ch06-02-hooks.md).

Use it with `agentstacks init --tool opencode --pack <p>`.

## 1. Vars

A pack text can use these `{{var <name>}}` forms. The extension gives only the names a pack uses.

| Var | Value | Note |
|---|---|---|
| `project_dir` | `${AGENTSTACKS_PROJECT_DIR}` | set by the hook shim in the agent's shell and in hook processes |
| `session_id` | `${OPENCODE_SESSION_ID}` | set in the shell tool; OpenCode never substitutes it in text, so the model uses it in shell commands |
| `arguments` | `<arguments>` | a command fills it; OpenCode never substitutes `$ARGUMENTS` in a skill |
| `tmpdir` | `${TMPDIR:-/tmp}` | expanded by the shell |
| `home` | `$HOME` | expanded by the shell |
| `airsstack_home` | `${AIRSSTACK_HOME:-$HOME/.airsstack}` | expanded by the shell |

The root for pack files is `.opencode/agentstacks`. A skill `<p>/<s>` lives in
`.opencode/skills/<p>-<s>` and is invoked as `/<p>-<s>`. An agent `<p>/<a>` is named `<p>-<a>`.

## 2. Files

| Item | What is written |
|---|---|
| skill `SKILL.md` | `.opencode/skills/<p>-<s>/SKILL.md`, with frontmatter and the text |
| the skill's command | `.opencode/commands/<p>-<s>.md`; a skipped skill gets no command |
| other skill file | copied next to `SKILL.md` |
| agent | `.opencode/agents/<p>-<a>.md`, with frontmatter and the text |
| native file | `native/opencode/<rel>` is copied to `.opencode/<rel>` |
| hooks | covered by `.opencode/plugins/agentstacks.js`, or skipped (section 5) |
| `.opencode/plugins/agentstacks.js` | always written; with no hook it still sets `AGENTSTACKS_PROJECT_DIR` |
| `.opencode/agentstacks/translate.lua` | written when at least one Lua hook became an entry |

A path joins the pack and item names with `-`, so `core/status` and `sdlc/status` never clash. Two
items whose joined names are equal (pack `a-b` skill `c`, pack `a` skill `b-c`), or a native file on
a generated path, propose the same path twice, and the core refuses it.

## 3. Hook mapping

Each hook becomes one entry in the shim or one skip. Packs are read in payload order. Within a pack,
neutral hooks come first, then native hooks.

| Neutral event | Runs on | OpenCode tools |
|---|---|---|
| `tool.before`, `tool.after` | the same event | from `match`: read is `read`; edit is `edit` and `patch`; write is `write` and `patch`. No `match` means `read`, `edit`, `write`, `patch` |
| `prompt.submit` | `prompt.submit` | none |
| `session.start` | `session.start` | none; renders with no `match` or a `match` holding `startup`, else skipped |
| `session.end`, `agent.stop` | skipped | none |

| Native event (in `native/opencode/hooks.toml`) | Runs on | OpenCode tools |
|---|---|---|
| `tool.execute.before`, `tool.execute.after` | the same event | the native `match` as written; none or empty means every tool |
| `session.prompt` | `session.prompt` | none; a non-empty `match` is skipped |
| any other | skipped | none |

A hook is native when it comes from the pack's native hooks, whatever its event is called.

## 4. Delivery

The shim runs each Lua entry as `/bin/sh -c <command>` from the project root. The hook gets a JSON
payload on stdin and answers with JSON on stdout. Stderr is discarded. Entries run one after another
in table order, and the first block stops the run.

| OpenCode hook | Entries run | How the result is delivered |
|---|---|---|
| shell creation | none | sets `AGENTSTACKS_PROJECT_DIR` to the project root, for the main agent and subagents |
| `tool.execute.before` | `tool.before`, `tool.execute.before` for that tool | a block stops the tool, and the model sees the block text as the tool's error. Context is held until the tool finishes |
| `tool.execute.after` (completed calls only) | `tool.after`, `tool.execute.after` | held and new context is joined and added to the tool's result as one text item. A block shows the model an error with the text |
| `session.prompt` (top-level sessions only) | `session.start` on the first prompt of a session in this process, then `prompt.submit` and `session.prompt` on every prompt | context is added to the prompt after a blank line. A `session.start` block is added as context |

A hook that prints nothing, prints text that is not a JSON object, or fails to start gives no
context and no block. A hook that runs longer than 60 seconds is killed and gives no context and no
block. A subagent's first prompt runs no `session.start` and no `prompt.submit`.

### 4.1 Missing airsl

Every Lua hook runs airsl. If airsl is gone from `PATH` after install, the shell exits 127 and the
hook gives nothing. So every Lua hook is off, guards included: a `may_block` hook blocks nothing.

The shim then tells the model once per OpenCode process, at the next top-level prompt, by adding
this notice after any other context:

```text
agentstacks: airsl is not on PATH, so the agentstacks Lua hooks are not running. Install airsl, then run agentstacks doctor.
```

- A failure first seen in a tool hook is told at the next prompt.
- A later prompt in the same process does not repeat it. A new process tells it again if airsl is
  still missing.
- `opencode run` without `--standalone` uses a background service that outlives the command. With it
  the notice comes once per service start.
- The notice stays in the session history as part of that prompt.
- An airsl without an execute bit makes the shell exit 126, and the model is not told. `plan` and
  `doctor` count such a file as missing.

`agentstacks doctor` also checks the airsl pin against the binary it finds.

## 5. Skips

An item the extension cannot express becomes a skip with a reason. Rules are checked in this order;
the first that applies gives the reason.

| # | Case | Applies to | Reason |
|---|---|---|---|
| 1 | a hook on `session.end` | neutral hooks | `opencode has no hook when a session ends` |
| 2 | a hook on `agent.stop` | neutral hooks | `opencode has no hook when a subagent stops` |
| 3 | a hook on `session.start` whose non-empty `match` does not hold `startup` (no `match` renders) | neutral hooks | `opencode runs session.start hooks only for a new session (source "startup"), which this hook does not match` |
| 4 | a `may_block` Lua hook on `prompt.submit` (neutral) or `session.prompt` (native); a message hook there renders | both | `opencode cannot block a prompt` |
| 5 | a hook on `session.prompt` with a non-empty `match` | native hooks | `opencode has no matcher for session.prompt` |
| 6 | a hook on any event but `tool.execute.before`, `tool.execute.after` and `session.prompt` | native hooks | `opencode has no native hook event "<event>" (use tool.execute.before, tool.execute.after or session.prompt)` |
| 7 | a hook whose text or argv holds `${OPENCODE_SESSION_ID}` | both | `opencode gives hooks the session id in the event (event.session_id), not as {{var session_id}}` |
| 8 | a Lua argument with a `$` left after every var form is removed | both | `opencode runs hook commands through sh, which would expand the literal $ in argument "<arg>"` |
| 9 | a native key with a character other than `A-Z`, `a-z`, `0-9`, `_` or `-` | skills, agents | `native.opencode key "<key>" cannot be written as frontmatter` |

`<event>`, `<arg>` and `<key>` stand for the hook's event, the argument and the key. A native hook
whose event is `session.end`, `agent.stop`, `session.start` or `prompt.submit` gets rule 6, naming
its event.

With the five official packs, these hooks are skipped: the `compact` hook of core's `session.start`
(rule 3), core's `session.end` message, its two `agent.stop` hooks, and the `agent.stop` hooks of
sdlc and journal. The other `session.start` hooks of core, sdlc and journal are rendered.

## 6. Not carried over from Claude

| Claude feature | OpenCode | Visible as |
|---|---|---|
| `SessionEnd`, `SubagentStop` hooks | none | skips 1 and 2 |
| `SessionStart` sources `resume`, `clear`, `compact` | only `startup` | skip of hooks that match only `compact`; see section 7 |
| block on `UserPromptSubmit` | a hook that throws on a prompt only shows `UnexpectedStatus: 500` | skip 4 |
| model tier, effort | none; a subagent uses the parent's model | not written |
| `argument-hint` | none | not written |
| `/skill args` | the generated command `/<p>-<s> args` | |

## 7. Behaviour to know

- **Resumed sessions.** OpenCode does not tell a resumed session from a new one. A session resumed
  with `-c` in a new process runs `session.start` again, with the source `startup`.
- **Compaction.** There is no `session.start` after compaction.
- **Hidden skills.** A skill with `invocation: user` is written with `disable-model-invocation:
  true`. It is hidden from the model's list, not refused: if the model calls it by id, it loads.
- **Permission check.** The before hook runs before OpenCode's permission check. A `may_block` hook
  can block a call the user would have been asked about, and a call that then fails drops its held
  context. The shim never reads or changes permissions.
- **Nested projects.** OpenCode loads `.opencode/` from every parent directory, so two agentstacks
  projects, one inside the other, would load two shims. This is not supported.
- **Both tools.** With `claude` and `opencode` both listed, OpenCode also reads `.claude/skills/`.
  Skills only Claude has, such as `core-statusline`, or skills the opencode extension skipped, then
  show up in OpenCode with Claude's var forms. Set `OPENCODE_DISABLE_CLAUDE_CODE=1` to stop OpenCode
  reading `.claude/skills/`.
- **Subdirectories.** OpenCode started in a subdirectory still finds `.opencode/`. Hooks run from the
  project root; `cwd` in the payload is the subdirectory.

## 8. Limits

- A `patch` header with leading spaces is applied by OpenCode but not seen by hooks.
- A `patch` that touches one blocked file and one allowed file is blocked as a whole, because it is
  one tool call.
- Whether a real model follows a command's "Load the skill" line, or a skill's `<arguments>`
  placeholder, is model behaviour.
- A hook's stderr is discarded, including the runtime's line that a hook tried to block but is not
  declared `may_block`.
- With airsl missing, every Lua hook is off, guards included (section 4.1).
- Context added to a prompt becomes part of the stored user message. The user sees it in the history,
  and OpenCode's title request reads it.
- Files written by `shell`, `generate_image` or `generate_video` pass no tool hook as a read, edit or
  write.
- With both tools listed, Claude-only skills show in OpenCode (section 7).
