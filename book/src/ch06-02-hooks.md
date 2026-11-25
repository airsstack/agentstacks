# Hooks

How a pack hook runs, from the tool's command to the hook's result. This guide is for pack authors
and extension authors. Everything in it is `api = 0`: unstable until both official extensions run
on it.

A pack declares a hook once, in tool-neutral terms. The core builds one command line for it, the
extension makes the tool run that command, and a runtime the core ships calls the hook's Lua module.
The tool-specific parts, turning the tool's payload into a neutral event and the hook's result back
into the tool's output, belong to the extension's `translate.lua`.

Written for extension authors in [extensions.md](ch06-01-extensions.md). The core's side of planning is in
[plan-and-apply.md](ch05-00-plan-and-apply.md).

## 1. Layout

Every tool that has a lua hook gets three files under `<root>`, the directory its extension names:

```text
<root>/runtime.lua                 written by the core (below)
<root>/translate.lua               written by the extension (required)
<root>/<pack>/hooks/enforce.lua    a root file, written by the core
```

- `runtime.lua` is written by the core, the same bytes for every tool
  (`crates/agentstacks/assets/runtime.lua`).
- `translate.lua` is written by the extension, one per tool.
- `<root>/<pack>/hooks/...` are the pack's own files, written by the core as root files.

airsl's `require` root is the script's directory. A hook `run` of `{{root core}}/hooks/enforce.lua`
therefore becomes the module `core.hooks.enforce`.

## 2. Rules for a pack hook

A hook in `pack.toml` (or in `native/<tool>/hooks.toml`) is checked when the pack loads.

- `grants.policy = "pure"` is refused in `api = 0`. `confined` is the only allowed value:
  - `hooks[1].grants.policy: "pure" is not supported in api 0 (airsl has no require under pure, airsstack/airsl#12)`
  - `hooks[1].grants.policy: "foo" (expected confined)`
- Each segment of the `run` path (after the `{{root <pack>}}` form, with `.lua` removed from the
  last one) must be require-able: letters, digits, `_` and `-` only. This also applies to
  `native/<tool>/hooks.toml`.
  - `hooks[1].run: "{{root core}}/hooks/my.hook.lua" segment "my.hook" may use only letters, digits, _ and -`

The run-segment refusal quotes `run` as written and reports only the first bad segment. It applies
only to a `run` that ends in `.lua`, because other values are already refused by the `.lua` rule.

## 3. Command line

The core builds this argv for each lua hook:

```text
airsl run --fail-open --policy confined
  [--allow-read <p>]… [--allow-write <p>]… [--allow-env <N>]… [--allow-exec <b>]…
  <root>/runtime.lua <module> <may_block: 0|1> <kind: neutral|native> [args…]
```

- The core builds it with project-relative paths. A pack hook's grants map one to one onto the
  `--allow-*` flags.
- **The extension's duty:** make the tool run this command with the project directory as its
  working directory. Example: claude extension, `cd "$CLAUDE_PROJECT_DIR" && …`.
- A project-relative grant then resolves against that working directory: `--allow-read .tool/agentstacks/core`
  lets the hook read `.tool/agentstacks/core/data.txt`. Without the grant, a read is denied:
  `fs.read denied: … is outside the granted read roots: none are granted`.
- `--fail-open` is fixed. A missing module exits `0` silently. Without it, airsl prints
  ``module `core.hooks.missing` not found under …`` and exits 1.
- Elements may hold a `{{var}}` value the extension supplied (an expression such as `$TMPDIR`).
  Turning the argv into the tool's command string, quoting included, is the extension's job.

## 4. `runtime.lua`

The core writes `<root>/runtime.lua` when a pack of the tool has any lua hook, neutral or native,
whether or not the extension later skips it. This is its full text, byte for byte:

```lua
{{#include ../../crates/agentstacks/assets/runtime.lua}}
```

What it does for each run:

| Run | Output |
|---|---|
| context | `{"context":"event=tool.before session=s1 args=--manifest,x.json read=true:hello\n …"}` |
| block from a hook that isn't `may_block` | stdout empty; stderr `agentstacks: hook core.hooks.blk tried to block but is not declared may_block; reason: edits to tool.before are not allowed` |
| same hook with `may_block` 1 | `{"decision":"block","reason":"edits to tool.before are not allowed"}` |
| native | `{"context":"native payload tool_name=Edit event=nil"}` |
| context and block from a hook that isn't `may_block` | stdout `{"context":"keep this context"}`; the block line on stderr |
| translator returns `nil` (an event it does not map) | no output, exit 0 |

The table above is an example: claude extension (its `translate.lua`, with `tool_name=Edit` as the
tool's own payload field). A real tool's translator decides the actual output text.

`M.event` may return a list of events instead of one. The rules:

| Case | Result |
|---|---|
| `M.event` returns a table with `event` set | one event; the result is what the hook returned, except that a `block` from a hook that is not `may_block` is dropped with the stderr line |
| `M.event` returns a list (a table with no `event` key) | the hook runs once per element, in order; each gets `args` |
| a list, `may_block` 1, an element blocks | the loop stops; the result is `{ block = <that block> }`, plus `context = <contexts so far, joined with "\n">` when any element returned one (the blocking element's context included) |
| a list, `may_block` 0, an element blocks | the stderr line as above, the block dropped, the loop goes on |
| a list where no hook returns context or block, where every block was dropped and no context came back, or an empty list | `M.result` gets `nil` |
| native kind | one event `{ payload = <raw>, args = … }` |
| a context that is not a string | in a list it is joined after `tostring`; a single event passes it through unchanged |

The hook module is required only when a hook runs, so a translator returning `nil` never loads it.

`airsstack.stdio` (`error`, `write`) and `airsstack.hook.payload` exist under `confined`.

## 5. Neutral protocol

**Event** (built by the translator):

| Field | When |
|---|---|
| `event` | always: one of the hook events of the pack format (`session.start`, `session.end`, `prompt.submit`, `tool.before`, `tool.after`, `agent.stop`) |
| `session_id`, `cwd` | always |
| `agent` | always; `nil` = the main agent |
| `agent_id` | optional; the tool's id for the agent instance, `nil` when the tool has none |
| `args` | always (added by the runtime) |
| `source` | `session.start`: `startup`, `resume`, `clear` or `compact` |
| `prompt` | `prompt.submit` |
| `tool` (`read`, `edit` or `write`), `path` | `tool.before`, `tool.after` |
| `last_message` | `agent.stop` |

There is no raw payload on a neutral event. A native hook gets `{ payload = <raw>, args = … }`.

**Result** (returned by the hook): `nil`, `{ context = "<text>" }`, `{ block = "<reason>" }`, or both.

Native hooks reach the extension in the same hook shape as a pack hook: `kind`, `event`, `match` as
the list the pack format reader holds, `may_block`, and `command` (a lua hook) or `text` (a message
hook).

## 6. Translator contract

`<root>/translate.lua` is one file per tool, written by the extension. It returns a module with two
functions:

- `M.event(payload)` returns the neutral event, a list of neutral events, or `nil` for an event that
  is not for this runtime (the hook is then not run). A list makes the runtime run the hook once per
  event ([section 4](#4-runtimelua)).
- `M.result(result or nil, payload)` returns the string to print, or `nil`. `payload` is the raw
  payload for neutral and native hooks alike. The runtime passes it because the payload can be read
  only once.

The file is required when `runtime.lua` is written and at least one lua hook is covered by a write.
Without it the plan is refused: `claude/adapter.lua: lua hooks need <root>/translate.lua (write it)`,
with `<root>` filled in.

## 7. `may_block`

A hook declares `may_block` to say it may stop the tool. A `block` from a hook that is not declared
`may_block` is dropped with one line on stderr, and any `context` it returned is kept. The
behaviour table in [section 4](#4-runtimelua) shows both outcomes.

## 8. Filtering and message hooks

Filtering by `match` and `agents` is the extension's job, in the tool's own config. The runtime does
not filter.

Message hooks (`kind = "message"`) never reach the runtime. The extension renders them natively.

## 9. Shims for other hook languages

When a tool demands another hook language, the extension writes a thin shim. Example: OpenCode
hooks are JavaScript plugins (`.js`), so its shim is JavaScript. The shim only runs the command line
of [section 3](#3-command-line) and forwards the payload and the result. It is an ordinary `write`,
shown in the plan like any other file: the core gives it nothing new.

## 10. airsl on the machine

Hook commands run on the `airsl` binary found on `PATH`. When a locked pack declares `requires.airsl`,
the core adds a `require_binary` for `airsl` with the range the packs need, and records the installed
version in the lock's `[airsl]` table (the pin). After install, `doctor` is the check that airsl is
still there and still the pinned version. The tables and messages are in
[plan-and-apply.md](ch05-00-plan-and-apply.md#11-binaries-and-the-airsl-pin) and
[plan-and-apply.md](ch05-00-plan-and-apply.md#8-doctor).
