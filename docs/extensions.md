# Writing a tool extension

A tool extension is the Lua that turns packs into one tool's files. This guide is the contract an
extension author codes against. Everything in it is `api = 0`: unstable until both official
extensions run on it, and free to change until then.

Every message below is shown with the claude extension as the example (`claude/adapter.lua`), the
way the core prints it for that tool. Another tool prints its own name and file in its place.

How the pieces fit together is in [architecture.md](architecture.md). The core's side of planning
and applying is in [plan-and-apply.md](plan-and-apply.md). The hook runtime an extension feeds is in
[hooks.md](hooks.md).

## 1. What an extension is and is not

An extension only proposes. It has no file, network or process access. It registers two handlers,
the core calls them, and what they return is a list of operations the core validates. `plan` shows
every operation, and `apply` is the only writer.

- There is no merge operation and no run-command operation. A config file shared with the user is
  read (declared in `extension.toml` `reads`), edited with `agentstacks.json.edit` and proposed
  whole. See [section 11](#11-agentstacksjsonedit).
- The operations are `write`, `copy`, `skip` and `require_binary`. The core adds `delete` and its
  own writes ([section 7](#7-operations)).
- Everything here is `api = 0`.

## 2. `extension.toml`

The manifest is checked by `Extension::load` (`crates/agentstacks/src/types/extension/loaded.rs`).
Its `reads` key is optional: a list of project paths the extension may read with `agentstacks.read`.

```toml
# example: claude extension
[extension]
name = "claude"
version = "0.1.0"
api = 0
description = "Renders packs into Claude Code project config"
entry = "adapter.lua"
reads = [".claude/settings.json"]
```

Each entry follows the lock's file path rule: relative, inside the project, `/` as the separator.
Entries are normalised (`./a` is `a`, `a//b` is `a/b`) before the duplicate check. A repeated entry
is quoted as written. Refusals:

| Case | Refusal |
|---|---|
| not an array | `extension.toml: extension.reads: expected array, got string` |
| an entry that is not a string | `extension.toml: extension.reads[0]: expected string, got integer` |
| an absolute path | `extension.toml: extension.reads[0]: "/x" must be relative` |
| a path that leaves the project | `extension.toml: extension.reads[0]: "../x" must stay inside the project` |
| a repeated entry | `extension.toml: extension.reads[1]: ".claude/settings.json" listed twice` |
| a `\` in an entry | `extension.toml: extension.reads[0]: "a\b" must use / as the separator` |
| an empty entry | `extension.toml: extension.reads[0]: empty` |

Naming `agentstacks.toml` or `agentstacks.lock` in `reads` is allowed, because reading them is
harmless. Writing to them is not allowed ([section 9](#9-validation)).

## 3. Load

1. The core resolves the extension's source (`agentstacks.toml`, a git tag or a project path) and
   loads it with `Extension::load`, which checks `extension.toml`.
2. `api = 0` is required by the manifest.
3. Every `.lua` file of the extension is read into memory. Lua never touches the disk.

A `.lua` file that is not UTF-8 is refused: `<tool>/<file>: not valid UTF-8`.

## 4. Sandbox

The extension runs under the airsl engine with the `pure` policy and an explicit stdlib set:
`Json`, `Ext` with the events `values` and `plan`, and `agentstacks`. The host lives in
`crates/agentstacks/src/host/`, and it is the only place Lua runs.

| | What |
|---|---|
| Removed | `print` and `warn`, the two output channels |
| Absent | `require` (use `agentstacks.require`, [section 10](#10-agentstacksrequire)), `load`, `dofile`, `os`, `io`, `coroutine`, and every airsl module not in the set |
| Present | Lua's remaining base functions (`pcall`, `error`, `pairs`, `ipairs`, `type`, `tostring`, `setmetatable`, `rawget`, …), plus `string`, `table`, `math` and `utf8`; `airsstack.json` and `airsstack.ext`; `airsstack.agentstacks`, also set as the global `agentstacks` |

Limits:

- 10,000,000 instructions per evaluation or dispatch;
- 16 MiB of memory, and data the host hands to Lua counts toward it.

The budget resets on each dispatch. The breach messages are:

- ``script `spin` exceeded its instruction limit of 10000000``
- ``script `adapter.lua` exceeded its memory limit of 16777216 bytes``

A breach caught by the extension's own `pcall` is that extension's business: the dispatch then
succeeds.

## 5. The `agentstacks` table

| Function | Phase | Returns |
|---|---|---|
| `agentstacks.on(event, fn)` | any | registers a handler for `values` or `plan`. A registration replaces the earlier one and takes effect at the next dispatch |
| `agentstacks.require(name)` | any | the module's return value ([section 10](#10-agentstacksrequire)) |
| `agentstacks.write{path, content, covers}` | plan | nothing; records an operation |
| `agentstacks.copy{path, file}` | plan | nothing; records an operation |
| `agentstacks.skip{item, reason}` | plan | nothing; records an operation |
| `agentstacks.require_binary{name}` | plan | nothing; records an operation |
| `agentstacks.read(path)` | plan | the file's bytes as a Lua string, or `nil` when no regular file is there |
| `agentstacks.json.edit(text, path, value)` | any | new text ([section 11](#11-agentstacksjsonedit)) |

`agentstacks.on` looks the engine's registry up when it is called: while modules install, the global
`airsstack` does not exist yet.

`read` rules:

- The path must be listed in `extension.reads`: `".env" is not in extension.reads`.
- It must resolve inside the project, else
  `claude/adapter.lua:9: read: ".claude/settings.json" resolves outside the project`.
- The bytes are returned as they are, non-UTF-8 included. A Lua string holds raw bytes.
- An I/O failure reading a `reads` path is `<path>: <io::Error Display>`. It is reported only when
  the extension reads that path, and it sticks like the other `read` refusals.
- The core records what each read returned (the hash, or "absent"). The record decides whether a
  file is shared ([plan-and-apply.md](plan-and-apply.md#3-steps)).

## 6. Events

The flow, once per tool:

1. The entry file runs at the top level. This registers handlers.
2. The core dispatches `values`.
3. The core renders the packs.
4. The core dispatches `plan`.

`dispatch` has three outcomes: no handler, a handler that returns `nil`, and a handler that returns
a table. A second `agentstacks.on` for the same event replaces the first: the last one wins.

### `values`

The payload the extension receives. Example: claude extension.

```json
{ "api": 0, "tool": "claude",
  "packs": [ { "name": "core", "version": "0.3.0", "skills": ["status"], "agents": ["coder"] } ],
  "vars": ["session_id", "tmpdir"] }
```

`vars` holds the `{{var}}` names this tool's packs use, sorted by name.

The result is a table that says where this tool puts things and what each token stands for:

```lua
-- example: claude extension
agentstacks.on("values", function(q)
  local out = { root = ".claude/agentstacks", skills = {}, agents = {}, vars = { tmpdir = "${TMPDIR:-/tmp}" } }
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills) do
      out.skills[p.name .. "/" .. s] = {
        invoke = "/" .. p.name .. "-" .. s,
        dir = ".claude/skills/" .. p.name .. "-" .. s,
      }
    end
    for _, a in ipairs(p.agents) do out.agents[p.name .. "/" .. a] = p.name .. "-" .. a end
  end
  return out
end)
```

The core fills every token from it:

| Token | Value used |
|---|---|
| `{{root}}`, `{{root <pack>}}` | `<root>/<pack>` |
| `{{skill p/s}}` | `skills["p/s"].invoke` |
| `{{skill_dir p/s}}` | `skills["p/s"].dir` |
| `{{agent p/a}}` | `agents["p/a"]` |
| `{{var v}}` | `vars[v]` |
| `{{option o}}` | resolved by the core from the pack's options |

**Missing values.** When a token has no value, the item that holds it is skipped by the core and left
out of the `plan` payload. The skip reason is, for example, `claude gave no value for {{skill core/status}}`.
With several missing values the reason names the first missing token in text order, and a missing
skill entry comes before any token.

- Skill: the item is the whole skill (group), because a half-installed skill is broken. A skill with
  no `skills[...]` entry at all is skipped too: `claude gave no install location for skill core/status`.
- Agent, hook (any field: `text`, `args`, `run`, grant paths), native file, native hook: the item
  itself.
- Root file: a core skip of that file.

The result arrives as JSON, so Lua's types come back as these words: object is `table`, array is
`list`, null is `nil`, plus `number`, `string` and `boolean`. An empty Lua table arrives as an
object.

**Refusals for the `values` result** (location `<tool>/<entry>`, the key path starts at `values`):

| Rule | Refusal |
|---|---|
| a `values` handler exists | `claude/adapter.lua: registers no values handler` |
| result is a table | `claude/adapter.lua: values: expected table, got nil` |
| `root` required, a string, a valid path | `claude/adapter.lua: values.root: required` / `claude/adapter.lua: values.root: "/x" must be relative` |
| `skills`, `agents`, `vars` are tables with string keys | `claude/adapter.lua: values.agents: expected table, got list` |
| their keys name items and vars of this payload | `claude/adapter.lua: values.skills."core/x": not a skill in this run` / `claude/adapter.lua: values.vars.home: not a var in this run` |
| `invoke`, `dir`, agent value, var value are strings; `dir` a valid path | `claude/adapter.lua: values.skills."core/status".dir: expected string, got number` |
| unknown key, at any level | `claude/adapter.lua: values.foo: unknown key` / `claude/adapter.lua: values.skills."core/status".icon: unknown key` |

`values.root` and each `dir` follow the path rows of [section 9](#9-validation) under their key. `.`
normalises to empty. The managed-file rule does not apply. More refusals:

- `claude/adapter.lua: values.root: empty`
- `… values.root: "../x" must stay inside the project`
- `… values.root: "a\b" must use / as the separator`
- `claude/adapter.lua: values.agents."core/x": not an agent in this run`

A `values.skills` entry needs both `invoke` and `dir`: `claude/adapter.lua: values.skills."core/status".dir: required`
(the same shape for `invoke`). Absent `values.skills`, `agents` or `vars` are accepted. Their items
then fall under missing values.

### `plan`

Every text in the payload is already rendered, and each id is a string to copy back. Example: claude
extension.

```json
{ "api": 0, "tool": "claude", "root": ".claude/agentstacks",
  "packs": [ { "name": "core", "version": "0.3.0",
    "skills": [ { "id": "core/skill/status", "name": "status", "dir": ".claude/skills/core-status",
                  "meta": { "description": "…", "invocation": "user" },
                  "files": [ { "id": "core/skill/status/SKILL.md", "rel": "SKILL.md", "text": "<body after frontmatter>" },
                             { "id": "core/skill/status/logo.png", "rel": "logo.png" } ] } ],
    "agents": [ { "id": "core/agent/coder", "name": "coder", "meta": { "description": "…", "model": "balanced" }, "text": "<body>" } ],
    "hooks": [ { "id": "core/hook/0", "kind": "lua", "event": "tool.before", "match": ["read", "edit", "write"],
                 "may_block": false, "command": ["airsl", "run", "…"] },
               { "id": "core/hook/1", "kind": "message", "event": "session.end", "text": "Reminder: run /snapshot-save …" } ],
    "native": { "files": [ { "id": "core/native/claude/x.md", "rel": "x.md", "text": "…" } ],
                "hooks": [ { "id": "core/native-hook/0", "event": "SubagentStop", "match": "…", "may_block": false, "command": ["…"] } ] } } ] }
```

- `text` is present for a UTF-8 file and absent for a binary one, which is installed with `copy`.
  For `SKILL.md` and agents, `text` is the body only, and `meta` holds the frontmatter fields, with
  `native` limited to this tool's entry.
- `match` and `agents` are given as lists, as the pack hook format holds them. A hook that has none
  has no `match` or `agents` key.
- `command` is the argv of the hook command line ([hooks.md](hooks.md#3-command-line)).
- A native hook always carries `kind` (`lua` or `message`), `event` (named as the tool names it),
  `match` as the list the pack format reader holds (left out when the hook has none), and `may_block`
  with `command` (a lua hook) or `text` (a message hook). It has no `agents`.
- A native hook's `match` is passed through as written, so one that matches an agent must name it
  by its installed name, `<pack>-<agent>` (for example `core-coder`), not the name in the pack.

In the example above, the native hook entry is shown shortened. As built, it reads:

```json
{ "id": "core/native-hook/0", "kind": "lua", "event": "SubagentStop", "match": ["…"],
  "may_block": false, "command": ["…"] }
```

The return value of `plan` is ignored. With no handler: `claude/adapter.lua: registers no plan handler`.

**Order.** Listed tools run in name order, then removed tools in name order. Each listed tool's core
writes are recorded just before its `plan` dispatch. That order decides which tool's call gets
`… already proposed by <tool>`.

## 7. Operations

| Kind | Proposed by | Fields | Effect |
|---|---|---|---|
| `write` | extension | `path`, `content` (bytes), `covers` (list of item ids, optional) | put exactly these bytes at `path` |
| `copy` | extension | `path`, `file` (a file id from the plan payload) | put that pack file's final bytes at `path`. For a rendered file that is the whole rendered file; for `SKILL.md` or an agent, the neutral frontmatter is included. Otherwise the raw bytes. It covers `file` |
| `skip` | extension | `item` (an item id), `reason` (non-empty) | nothing is installed; `plan` shows the reason |
| `require_binary` | extension | `name` | plan refuses when `name` is not on `PATH` |
| `write` (core) | core | none | each pack root file goes to `<root>/<pack>/<rel>`; `runtime.lua` goes to `<root>/runtime.lua` when a pack of this tool has a `lua` hook or a native `lua` hook (whether or not the extension later skips it) |
| `require_binary` (core) | core | none | each pack's `requires.binaries`; `airsl` with the intersected range when any locked pack declares `requires.airsl` |
| `delete` | core | `path` | a locked file no longer proposed |

`runtime.lua` is written when a pack of the tool has any lua hook, neutral or native, even if the
extension later skips it.

`by`, the lock's owner of a file (a pack or tool name), is derived and never passed:

- a copy, or a write whose `covers` all belong to one pack, is owned by that pack;
- a core write of a root file is owned by its pack;
- any other write (no `covers`, `covers` spanning several packs, `runtime.lua`) is owned by the tool.

A shared file ([plan-and-apply.md](plan-and-apply.md#3-steps)) is never locked, so it has no `by`.

## 8. Item ids and coverage

Ids are strings the `plan` payload hands out. The extension copies them and never builds one.

| Item | Id | Covered by |
|---|---|---|
| skill file | `core/skill/status/SKILL.md`, `core/skill/status/references/x.md` | a `copy` of it, a `write` listing it in `covers`, or a `skip` of it or of its skill |
| skill (group) | `core/skill/status` | used only by `skip`; covers all of its files |
| agent | `core/agent/coder` | `write` with `covers`, or `skip` |
| hook (`pack.toml`) | `core/hook/0` (index in `[[hooks]]`) | `write` with `covers`, or `skip` |
| native file | `core/native/claude/x.md` (only this tool's `native/<tool>/`) | `copy`, `write` with `covers`, or `skip` |
| native hook | `core/native-hook/0` (index in `native/<tool>/hooks.toml`) | `write` with `covers`, or `skip` |

Every item the extension receives must be covered by an operation (write, copy or skip), otherwise
the plan is refused. Root files are not items, because the core writes them. Items the core already
skipped ([section 6](#missing-values)) are left out of the payload and need no cover.

| Case | Result |
|---|---|
| an item covered by no op | refused, one per item, in id byte order (`core/hook/10` before `core/hook/2`): `claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a reason)` |
| an item both skipped and copied or written; a skill group skipped while one of its files is copied or written | refused at the later call: `claude/adapter.lua:14: skip: item "core/skill/status" is already covered at adapter.lua:9`. After a skip, a `copy` or `write` reads `<tool>/<file>:<line>: <function>: covers "<id>", which <file>:<line> skipped`, where a `copy`'s `<id>` is its `file` |
| the same item skipped twice | accepted; the plan shows the first reason |
| the same file copied to two paths; two writes covering the same item; a repeated id inside one `covers` | accepted |
| lua hooks covered by a write, but no `write` to `<root>/translate.lua` | refused: `claude/adapter.lua: lua hooks need <root>/translate.lua (write it)`, with `<root>` filled in |

"Already covered at" names the first covering op.

The `translate.lua` rule applies when `runtime.lua` is written and at least one lua hook is covered
by a write. The contract for that file is in [hooks.md](hooks.md#6-translator-contract).

Two more refusals keep ids to their kind:

- `copy.file` must name a file id: `copy: file "core/agent/coder" is not a file`
- `covers` may not name a skill group: `write: covers[1]: "core/skill/status" is a skill; list its files`

## 9. Validation

An operation is checked the moment the Lua function is called. The refusal names the caller's file
and line (a refusal raised with no Lua frame on the stack is located at `<tool>` alone, see below),
and has this shape (`<function>` is the `agentstacks` function called):

```text
claude/adapter.lua:12: write: path: expected string, got number
```

The phase check (`only allowed inside the plan handler`) runs before every other check. After it,
a call raises one refusal: the first failing check, in the table order below. The location is the
nearest Lua frame on the stack, so `pcall(agentstacks.write, t)` is located at the `pcall` line.
A call made from a required module names that module.

A call with no Lua frame on the stack, for example `agentstacks.on('plan', agentstacks.write)`
where the host calls `write` itself, is refused with `<tool>: <function> must be called from the
extension's Lua code`, for example `claude: write must be called from the extension's Lua code`.
Any refusal with no Lua frame is located at `<tool>` alone, with no file or line: the phase check
for such a call reads `claude: write: only allowed inside the plan handler`.

| Rule | Refusal (after `<tool>/<file>:<line>: <function>: `) |
|---|---|
| called outside the `plan` handler (ops and `read`) | `only allowed inside the plan handler` |
| argument is one table (`read` takes one string) | `expected table, got string` / `expected string, got number` |
| required field present (`nil` counts as missing) | `path: required` (same shape for `content`, `file`, `item`, `reason`, `name`) |
| field type matches exactly; no coercion | `path: expected string, got number` / `covers: expected table, got string` / `covers[2]: expected string, got boolean` |
| `covers` is a list | `covers: expected a list` |
| no unknown field | `foo: unknown key` |
| `path` is UTF-8 | `path: not valid UTF-8` |
| `path` shape: the lock's file path rule, plus `/` as separator | `path: empty` / `path "/x" must be relative` / `path "../x" must stay inside the project` / `path "a\b" must use / as the separator` |
| `path` is not one of agentstacks' own files | `path "agentstacks.lock" is managed by agentstacks` (same for `agentstacks.toml`) |
| `path` not proposed twice in one plan, by any op of any tool or by the core | `path ".claude/x" is already proposed at adapter.lua:9` / `… already proposed by opencode` / `… already proposed by core (root file of core)` |
| `file`, `item`, `covers[i]` name an id in this run's payload | `file "core/skill/x/SKILL.md" is not in this plan` (same shape for `item`, `covers[1]`) |
| `skip.reason` not empty or whitespace-only | `reason: empty` |
| `require_binary.name` is a valid binary name | `name: "Git" is not a valid name` |
| `read` path in `extension.reads` | `".env" is not in extension.reads` |

The generic argument shapes, for arguments that have no row of their own:

- `<fn>: <field>: expected <type>, got <type>`
- `<fn>: <field>: expected a list`
- `<fn>: <field>: not valid UTF-8`
- `<fn>: keys must be strings, got <type>`
- `require: expected string, got <type>`

A path that normalises to nothing (`.`, `./`) is `path: empty`.

When the earlier proposer is the core's own `runtime.lua`: `path "<p>" is already proposed by core (runtime.lua of claude)`.

**Normalisation.** Every op path, every `reads` entry and every `values` path (`root`, `dir`) is
normalised on receipt by removing `.` and empty segments. The duplicate check, the lock lookup and
the lock's `[[file]].path` all use the normalised form.

**Accepted without refusal:**

- an empty `content` (an empty file);
- an empty `covers` (same as none);
- the same `require_binary` name more than once (checked once per tool).

**Type words** are Lua's: `nil`, `boolean`, `number`, `string`, `table`, `function`, `userdata`,
`thread`. An integer and a float are both `number`.

**Strictness.** Arguments are matched by their Lua type, with no coercion: `path = 5` is refused,
not turned into `"5"`. Unknown keys are refused. `content` keeps its raw bytes.

**Refusals stick.** The host records each op refusal when it happens. The plan is refused even when
the Lua code catches the error with `pcall`, and the refused op is not recorded.

## 10. `agentstacks.require`

`require` stays absent. Modules load through `agentstacks.require(name)`.

- `name` must match `^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)*$`.
- It maps to `<name with . replaced by />.lua` among the extension's in-memory files.
- It is loaded once, with chunk name `@<path>`, and the result is cached.
- Errors carry the module's file and line, for example `lib/util.lua:2: boom`.
- `load` stays with the host and is never exposed.

| Rule | Refusal (location: the caller) |
|---|---|
| name shape | `claude/adapter.lua:1: require: invalid module name "lib/x" (use dots: lib.x)` |
| file exists | `claude/adapter.lua:1: require: module "lib.x" not found in extension claude` |
| not the entry file | `claude/adapter.lua:1: require: "adapter" is the entry file` |
| no cycle | `claude/lib/b.lua:1: require: module "lib.a" requires itself (lib.a → lib.b → lib.a)` |

A cycle is reported at the `require` call that closes it, here `claude/lib/b.lua:1`. The
`(use dots: …)` hint appears only when the name contains `/`, which becomes `.`. Otherwise the
message is `claude/adapter.lua:1: require: invalid module name "x y"`.

`require` refusals are ordinary Lua errors and do not stick. Uncaught, they show their located
message, not the wrapper of [section 12](#12-errors-and-locations). A module that raised while
loading is not cached: requiring it again runs it again.

## 11. `agentstacks.json.edit`

`edit(text, path, value)` returns new text. `path` is a list of object keys, `value` is a Lua value,
and `nil` removes the key. The core knows no config format: a shared file is read (declared in
`reads`), edited with this pure helper and rewritten whole.

| Rule | Behaviour / refusal (location: `<tool>/<file>:<line>: json.edit: …`) |
|---|---|
| arguments | `json.edit: text: expected string, got nil` / `json.edit: path: expected table, got string` |
| `text` empty or whitespace-only | treated as `{}` |
| `text` parses as JSON | else `json.edit: <serde_json message>` (for example `trailing comma at line 1 column 8`) |
| top level is an object | else `json.edit: top level must be an object` |
| `path` is a non-empty list of strings | `json.edit: path: empty` / `json.edit: path[2]: expected string, got number` |
| setting: every key before the last names an object, and a missing one is created as `{}` | `json.edit: "hooks" is not an object` |
| removing (`nil`): a missing key or parent | no change; the text is returned as it came in |
| existing keys keep their position; a new key goes last | none |
| indent | the leading whitespace of the first indented line after the first line, if it is all spaces or one tab; otherwise two spaces |
| line endings | `\n`; CRLF input comes back as LF |
| trailing newline | kept when the input ended with `\n`, absent otherwise |
| numbers | re-printed in serde_json's normal form: `12345678901234567890123` becomes `1.2345678901234568e+22`, `0.10` becomes `0.1` |
| duplicate keys | the last one wins |

More behaviour:

- Removing through a parent that exists but is not an object refuses like setting:
  `json.edit: "hooks" is not an object`.
- Whitespace-only text takes its indent and trailing newline from the original text.
- With several failing rows, the first row of the table wins.
- The generic shapes: `json.edit: text: not valid UTF-8`, `json.edit: path: expected a list`,
  `json.edit: value: <type> cannot be JSON`, `json.edit: value: -inf cannot be JSON`,
  `json.edit: value: table contains itself`, `json.edit: value: key is not valid UTF-8`.
- A value nested more than 128 levels deep: `json.edit: value: nested more than 128 levels deep`.
- `json.edit` refusals are ordinary Lua errors: an extension may catch them with `pcall`, they do not
  stick, and an uncaught one fails the dispatch.

Converting the Lua value is hand-written, because a generic conversion drops data:

| Lua value | JSON | Refusal |
|---|---|---|
| table with keys exactly `1..n` | array | none |
| table with only string keys | object, keys sorted (Lua has no key order, so the output stays deterministic) | none |
| empty table | `{}`. No `[]` can be produced: to empty a list, remove its key | none |
| any other table | none | `json.edit: value: table mixes list and keyed entries` |
| integer, float, string (UTF-8), boolean | number, number, string, boolean | `json.edit: value: NaN cannot be JSON` (also `inf`) / `json.edit: value: string is not valid UTF-8` |
| function, userdata, thread | none | `json.edit: value: function cannot be JSON` |

A checked example. Example: claude extension, editing `.claude/settings.json`.

```lua
local old = '{\n    "permissions": {"allow": ["Bash(x:*)"]},\n    "hooks": {}\n}\n'
agentstacks.json.edit(old, {"hooks", "PreToolUse"}, { { matcher = "Read" } })
```

```json
{
    "permissions": {
        "allow": [
            "Bash(x:*)"
        ]
    },
    "hooks": {
        "PreToolUse": [
            {
                "matcher": "Read"
            }
        ]
    }
}
```

The indent is 4 spaces and the trailing newline is kept. The inner layout is pretty-printed: the
one-line `"allow"` array becomes multi-line.

Numbers in a shared JSON file are re-printed in serde_json's normal form, and the plan's diff shows
any change. Exact number spelling and the inner layout are not preserved.

## 12. Errors and locations

| Case | What the user sees |
|---|---|
| our op refused | the message of [section 9](#9-validation) (sticky) |
| `json.edit` refused | a normal Lua error: an extension may catch it with `pcall`, and an uncaught one fails the dispatch |
| any other Lua failure | `<tool>/<entry>: ` + the first line of airsl's error Display, e.g. `claude/adapter.lua: lua error in adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>`, `claude/adapter.lua: lua error in plan: runtime error: adapter.lua:3: path must be relative` (a dispatch names the event) (example: claude extension) |
| limit breach | ``claude/adapter.lua: script `plan` exceeded its instruction limit of 10000000`` |
| `agentstacks.on` with another event | airsl's own text, e.g. ``claude/adapter.lua: lua error in adapter.lua: event `nope` is not one this host dispatches; declared events: plan, values``. The declared events are kept sorted |

Any extension error refuses the whole plan. Nothing is written.

Where a refusal is located (the first three rows of the core's locations table):

| Location file | Used for | Example |
|---|---|---|
| `<tool>/<extension file>` with line | op arguments, `read`, `require`, `json.edit` | `claude/adapter.lua:12: write: path: expected string, got number` |
| `<tool>/<entry>` with key | the `values` result | `claude/adapter.lua: values.root: required` |
| `<tool>/<entry>`, no line | coverage, missing handler, Lua failure | `claude/adapter.lua: core/agent/coder got no operation (copy it, write it, or skip it with a reason)` |

`<extension file>` is the file's path inside the extension, for example `claude/lib/settings.lua:3`.
The full table is in [plan-and-apply.md](plan-and-apply.md#12-where-a-refusal-is-located).

## 13. `require_binary`

| Requirement | Check | Refusal |
|---|---|---|
| an extension's `require_binary` | `PATH` lookup | `claude/adapter.lua:7: require_binary: "node" not found on PATH` |

A binary required by both a pack and an extension is reported once for each. The same name more than
once in one extension is checked once.

## 14. The extension's duties outside Lua

The core builds the hook command line and the runtime. The rest is the extension's, because only the
tool knows its own config:

- **Run the hook argv from the project directory.** Make the tool run the hook's argv with the
  project directory as its working directory, and turn the argv into the tool's command string with
  its own quoting. Example: claude extension, `cd "$CLAUDE_PROJECT_DIR" && …`. See
  [hooks.md](hooks.md#3-command-line).
- **Write `<root>/translate.lua`** to the translator contract in [hooks.md](hooks.md#6-translator-contract).
- **Filter by `match` and `agents`** in the tool's own config. The runtime does not filter.
- **Render message hooks natively.** A message hook never reaches the runtime.
- **Write a shim with `write`** when the tool demands another hook language. Example: OpenCode
  plugins are TypeScript, so the extension writes a thin TypeScript shim that only runs the argv and
  forwards the payload and result ([hooks.md](hooks.md#9-shims-for-other-hook-languages)).
