# Muse Code

What the official `musecode` extension does: the files it writes, the variables it gives packs, how
hooks reach Muse Code, what it skips, and what it cannot do. It is checked against Muse Code
1.4.25-R9361.1. It runs on the same `api = 1` as every extension; see
[the extension API](ch06-01-extensions.md) and [hooks](ch06-02-hooks.md).

Use it with `agentstacks init --tool musecode --pack <p>`, or add it to a project with
`agentstacks ext add musecode`. The extension ships with the next `packs-v*` release
(`packs-v1.0.0` does not include it); until then copy `extensions/musecode` from a checkout of this
repository into the project, for example to `ext/musecode`, and pass `--tool ext/musecode`.

Start Muse Code in the project root and trust the workspace: project config loads only from a
trusted workspace. After `apply`, start a new session, because Muse Code reads its hooks, skills and
agents when a session starts.

The extension declares one file for reading, `.muse/hooks.json`, which the user and the extension
share (section 4.2).

## 1. Vars

A pack text can use these `{{var <name>}}` forms. The extension gives only the names a pack uses.

| Var | Value | Note |
|---|---|---|
| `project_dir` | `.` | right while the shell is in the project root; hooks run in the directory Muse Code was started in |
| `session_id` | `${MUSE_SESSION_ID}` | set in the `bash` tool's environment; Muse Code never substitutes it in text, so the model uses it in shell commands |
| `arguments` | `<arguments>` | Muse Code never substitutes `$ARGUMENTS` in a skill; the TUI appends the arguments after the skill text |
| `tmpdir` | `${TMPDIR:-/tmp}` | expanded by the shell |
| `home` | `$HOME` | expanded by the shell |
| `airsstack_home` | `${AIRSSTACK_HOME:-$HOME/.airsstack}` | expanded by the shell; in a hook it is always the default (section 8) |

The root for pack files is `.muse/agentstacks`. A skill `<p>/<s>` lives in `.agents/skills/<p>-<s>`
and is invoked as `/<p>-<s>`. An agent `<p>/<a>` is named `<p>-<a>`.

## 2. Files

| Item | What is written |
|---|---|
| skill `SKILL.md` | `.agents/skills/<p>-<s>/SKILL.md`, with frontmatter and the text |
| other skill file | copied next to `SKILL.md` |
| agent | `.agents/agents/<p>-<a>.md`, with frontmatter and the text |
| native file | `native/musecode/<rel>` is copied to `.agents/<rel>` |
| hooks | `.muse/hooks.json`, shared with the user (section 4.2) |
| `.muse/agentstacks/translate.lua` | written when at least one Lua hook is covered by the write of the hooks file |
| `.muse/agentstacks/runtime.lua`, root files | written by the core |

A path joins the pack and item names with `-`, so `core/status` and `sdlc/status` never clash. Two
items whose joined names are equal (pack `a-b` skill `c`, pack `a` skill `b-c`), or a native file on
a generated path, propose the same path twice, and the core refuses it.

### 2.1 Frontmatter

Every value is written as JSON, which Muse Code reads as a YAML value. The generated keys come
first, in the order below. A `native` key of the same name replaces a generated one; the other
native keys follow, sorted by name.

A skill gets:

| Key | Value | When |
|---|---|---|
| `name` | `<p>-<s>` | always |
| `description` | the skill's description | always |
| `disable-model-invocation` | `true` | `invocation: user`: the skill is hidden from the model's list, and the user still runs it as `/<p>-<s>` |
| `argument-hint` | the skill's argument hint | when set; shown as hint text after the command |

An agent gets:

| Key | Value | When |
|---|---|---|
| `name` | `<p>-<a>` | always |
| `description` | the agent's description | always |
| `tools` | the agent's tools, mapped as below | when the agent lists tools |
| `effort` | the agent's effort | when set |

The neutral tool words map to Muse Code's tool names in the order the pack lists them: `read` to
`read_file`, `edit` to `edit_file`, `write` to `write_file`, `grep` and `glob` to `search`, `shell`
to `bash`, `skill` to `read_skill`. A name already written is not repeated. An empty list is written
as `tools: []`. The `model` key is never written: a neutral model tier is not a Muse Code model id.

## 3. Hook mapping

Each hook becomes one handler in `.muse/hooks.json` or one skip. Packs are read in payload order.
Within a pack, neutral hooks come first, then native hooks.

| Neutral event | Muse Code event | Matcher |
|---|---|---|
| `session.start` | `SessionStart` | the `match` words joined with `\|`; none when `match` is absent. Muse Code matches it on the session source |
| `session.end` | `SessionEnd` | none |
| `prompt.submit` | `UserPromptSubmit` | none; Muse Code refuses a matcher on this event |
| `tool.before`, `tool.after` | `PreToolUse`, `PostToolUse` | from `match`: `read` is `Read`, `edit` is `Edit`, `write` is `Write`, joined with `\|`. No `match` means `Read\|Edit\|Write`. Muse Code matches these on its `read_file`, `edit_file` and `write_file` tools |
| `agent.stop` | `SubagentStop` | none; a hook that lists `agents` is skipped (rule H3) |

A native hook (in `native/musecode/hooks.toml`) keeps its event name as written. Its `match` is
joined with `|`, and it has no matcher when `match` is absent or empty.

A Lua hook becomes `/bin/sh -c '<quoted argv>' # agentstacks`: each argument is wrapped in double
quotes with `\`, `"` and `` ` `` escaped, the arguments are joined with spaces, and every `'` in the
result is written as `'\''`. A message hook becomes `printf '%s\n' '<json>' # agentstacks`, where
the JSON carries the text as `additionalContext`. The `# agentstacks` tail marks a handler as ours:
`plan` and `apply` replace and remove only handlers that end with it.

## 4. Delivery

### 4.1 The translator

A Lua hook runs through the hook runtime, which loads `.muse/agentstacks/translate.lua`. The
translator turns Muse Code's payload into a neutral event and turns the hook's result into the JSON
Muse Code reads from stdout. A neutral event carries `event`, `session_id` and `cwd`, plus these
fields where the event has them: `source`, `prompt`, `tool`, `path` and `last_message`. The path is
the one the model wrote, made absolute against `cwd` when it is relative. Tools other than
`read_file`, `edit_file` and `write_file` run no hook. No payload field names the running agent, so
`agent` and `agent_id` are never set.

### 4.2 The hooks file

`.muse/hooks.json` is shared with the user, so it is never locked and is rewritten whole. An empty
or blank file counts as `{}`. A file that is not JSON, not an object, or whose `hooks` is not an
object is refused with the core's message. For each event the extension touches, it keeps the user's groups, removes only the handlers
ending in `# agentstacks`, drops a group that is left empty, and appends its own groups in payload
order. An event whose array it cannot rewrite is left alone, and each hook of ours on it is skipped
(section 5).

Every group the extension writes has a string `matcher` or none and handlers with `type` and
`command` only, so it never gives Muse Code a reason to drop the file.

### 4.3 How results reach the model

| Event | Result | What happens |
|---|---|---|
| `PreToolUse` | block | the call is refused, and the model sees `tool blocked by hook: <reason>` |
| `PreToolUse` | context, alone or beside a block | a developer message after the tool output (after the blocked output when the call is refused) |
| `PostToolUse`, `UserPromptSubmit`, `SessionStart` | context | a developer message after the tool output or the prompt; at session start the context is in the first request |
| `PostToolUse` | block | the model sees the reason in place of the tool output |
| `UserPromptSubmit` | block | the run is cancelled before the model is called |
| `SubagentStop` | block | the agent runs again |
| `SessionStart` | block | the block text is joined into the context, because a session start cannot block |

Context returned by a neutral Lua `agent.stop` or `session.end` hook is dropped; only a block on
`agent.stop` has an effect. A hook that returns no context and no block adds nothing. A hook's
failure, a missing airsl included, is silent and the hook fails open: the call goes ahead.
`agentstacks doctor` checks airsl and its pin (see [hooks](ch06-02-hooks.md)).

## 5. Skips

An item the extension cannot express becomes a skip with a reason. In each table the rules are
checked in the order listed; the first that applies gives the reason.

Frontmatter rules. F2 to F5 check the values as written, after a native key has replaced a generated
one. A native `name` or `description` that is not a string is written as given and not checked.

| # | Case | Applies to | Reason |
|---|---|---|---|
| F1 | a `native` key with a character other than `A-Z`, `a-z`, `0-9`, `_` or `-` | skills, agents | `native.musecode key "<key>" cannot be written as frontmatter` |
| F2 | the skill `name` is longer than 128 characters | skills | `musecode skill names are at most 128 characters (this one has <n>)` |
| F3 | the agent `name` is not lowercase letter words joined by `-` | agents | `musecode agent names are lowercase letter words joined by - (got "<name>")` |
| F4 | the agent `description` is longer than 4096 bytes | agents | `musecode drops an agent whose description is over 4096 bytes (this one has <n>)` |
| F5 | the whole `SKILL.md` text is longer than 262144 bytes | skills | `musecode refuses a SKILL.md over 262144 bytes (this one has <n>)` |

A skill skipped by F1, F2 or F5 is skipped whole, with its other files. The pack and item names are
each at most 64 bytes, so `<p>-<s>` can reach 129 characters and meet F2.

Hook rules.

| # | Case | Applies to | Reason |
|---|---|---|---|
| H1 | the hook's text or an argument holds `${MUSE_SESSION_ID}` | both | `musecode gives hooks the session id in the event (event.session_id), not as {{var session_id}}` |
| H2 | a message hook on `session.end` | neutral hooks | `musecode shows no hook output to the model at session end` |
| H3 | an `agent.stop` hook with `agents` | neutral hooks | `musecode gives hooks no agent name at agent stop, so it cannot run this hook for only the listed agents` |
| H4 | a message hook on `agent.stop` | neutral hooks | `musecode passes no hook message to the model at agent stop` |
| H5 | a native hook on an event Muse Code does not run (list below) | native hooks | `musecode does not run hooks on "<event>"` |
| H6 | a native message hook on an event whose context does not reach the model (list below) | native hooks | `musecode passes no hook message to the model on <event>` |
| H7 | a native hook with a non-empty `match` on `UserPromptSubmit` or `Stop` | native hooks | `musecode refuses a matcher on <event>` |
| H8 | a Lua argument with a `$` left after every var form is removed | both | `musecode runs hook commands through sh, which would expand the literal $ in argument "<arg>"` |

Muse Code runs a native hook on these events: `SessionStart`, `UserPromptSubmit`, `PreToolUse`,
`PermissionRequest`, `PostToolUse`, `PostToolUseFailure`, `PostToolBatch`, `PreLLMCall`,
`PostLLMCall`, `PreCompact`, `PostCompact`, `SubagentStart`, `SubagentStop`, `Stop`, `StopFailure`,
`SessionEnd`, `Notification` and `SessionFork`. Any other name gets H5.

A native message hook renders on `SessionStart`, `SubagentStart`, `UserPromptSubmit`, `PreToolUse`,
`PostToolUse` and `PostToolUseFailure`: on those events a message reached the model in a run. On any
other event it gets H6, including `PostToolBatch`, `PreLLMCall` and `PostLLMCall`, whose delivery
was not shown.

`<event>`, `<arg>`, `<key>`, `<name>` and `<n>` stand for the hook's event, the argument, the key,
the name and the measured size. H1, H2, H6 and H8 keep the claude extension's wording with `claude`
replaced by `musecode`.

Hooks-file rule. When an event array in `.muse/hooks.json` cannot be rewritten (it holds a null, an
empty list, or a value that is not a list of hook groups), the event is left alone and each hook of
ours on it, both kinds, gets this reason, with the event name in place of `<event>`:
`.muse/hooks.json: hooks.<event> holds entries the musecode extension cannot rewrite (a null, an
empty list, or a value that is not a list of hook groups)`.

The core's own skips (missing values, see [the extension API](ch06-01-extensions.md)) apply as for
every extension.

With the five official packs, these hooks are skipped: core's `session.end` message (H2) and the
`agent.stop` handoff checks of core, sdlc and journal (H3). Core's other `agent.stop` hook, which
lists no agents, renders. The two native hooks in the packs are for `claude` only.

## 6. Not carried over from Claude

| Claude feature | Muse Code | Visible as |
|---|---|---|
| `SubagentStop` matcher by agent name | no agent name in hook payloads | skip H3 |
| `agent_type` and `agent_id` on events | none | every event reads as the main agent's |
| model tier | Muse Code model ids only | not written |
| `cd "$CLAUDE_PROJECT_DIR"` | hooks run in the launch directory | `project_dir` is `.` |
| `$ARGUMENTS` in a skill | never substituted; the TUI appends the arguments | `<arguments>` |
| `/skill args` in headless runs | `muse exec` sends `/name args` as plain text | documented |

## 7. Behaviour to know

- **Start in the project root.** From a subdirectory Muse Code does not load the project's config.
- **Trust the workspace.** Project config loads only in a trusted workspace.
- **New session after `apply`.** Muse Code reads `.muse/hooks.json` when a session starts; skills
  and agents are read by a new run.
- **Hidden skills.** A skill with `invocation: user` is written with `disable-model-invocation:
  true`. It is hidden from the model's list; the user still runs it as `/<p>-<s>`.
- **Shell.** Muse Code runs a hook command through the shell named by `SHELL`; the `/bin/sh -c`
  wrapper makes the argument quoting the same for every shell. Commands were checked under zsh, sh
  and bash. A `SHELL` that cannot start fails every hook, silently.

## 8. Limits

- With `--workspace <root>` from a subdirectory, hooks run in that subdirectory, so relative grants,
  `project_dir` and the translator's absolute paths point there. A plain start in a subdirectory
  loads no project config. Start in the project root.
- A user group in `.muse/hooks.json` that Muse Code finds invalid makes it drop the whole file, ours
  included.
- A native Lua hook on an event other than `SessionStart`, `UserPromptSubmit`, `PreToolUse`,
  `PostToolUse` and `SubagentStop` runs, but its context or block is not delivered.
- No hook knows which agent is running: `agent.stop` hooks for named agents are skipped (H3), and
  tool hooks in a subagent read as the main agent's.
- `SubagentStop` fires several times per turn for Muse Code's own `goal-reminder` agent.
- In hooks, `AIRSSTACK_HOME` is always the default `$HOME/.airsstack`: Muse Code passes hooks only
  an allowed list of variables. The `bash` tool sees the user's value.
- `{{var project_dir}}` is `.`: right while the shell is in the project root.
- A hook's failure, a missing airsl included, is silent and fails open.
- `muse exec` does not expand `/<skill> args`; only the TUI does.
- With `claude` listed, Claude-only skills show in Muse Code; they can be turned off one at a time.
  With `opencode` listed, skills only the musecode extension writes (in `.agents/skills/`) show in
  OpenCode.
- Hook commands were checked under `SHELL` zsh, sh and bash only.
- Muse Code's shell sandbox may refuse a skill's script a write outside the workspace, for example
  under `$HOME/.airsstack`. This is not probed; choose the permission profile that fits.

## 9. The check

`cargo make musecode-check` runs the extension inside the pinned real Muse Code against a local mock
model, in an isolated home. It is not part of `cargo make dod`: it needs Node and a download, so CI
runs it as its own job. Its scenarios live in `crates/agentstacks-cli/tests/musecode-check/`.
