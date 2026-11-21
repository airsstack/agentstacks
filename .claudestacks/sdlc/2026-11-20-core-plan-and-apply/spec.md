---
status: approved
created: 2026-11-21
built-on: 97136a84a006
---

# Spec: Core plan and apply, extension host and hooks

This spec turns the built formats (`2026-11-20-pack-and-project-formats`) into a working core.
It covers:

- resolving pack and extension sources;
- running a tool extension's Lua under airsl with zero grants, and validating what it proposes;
- building a plan a human reads, then applying it as the only writer and recording the lock;
- checking drift for `doctor`, and finding the latest tag for `update`;
- the tool-neutral hook runtime that pack hooks run on.

It is library code only. The CLI wires it later (`cli-commands`).

Proof convention: `probes/<file>` is a file in this chain directory holding raw probe output,
pasted byte for byte. `src/…` is `crates/agentstacks/src/…` as built at commit `34265ef`. A
`formats §N` cite is the approved formats spec.

Every `json` and `lua` block in this spec was extracted from this file and run against airsl 0.1.4 and serde_json 1.0.154 (`probes/examples-main.rs`, output `probes/examples-results.txt`):

- the two payloads parse;
- the `values` example dispatches to `{"agents":{"core/coder":"coder"},"root":".claude/agentstacks",…}`;
- the `json.edit` example's output equals the block printed after it (`true`);
- `runtime.lua` equals the file that ran on the airsl binary (`true`).

## 1. Settled decisions

All settled with the author on 2026-11-21.

| # | Decision |
|---|---|
| D1 | Operations: `write`, `copy`, `skip`, `require_binary` (proposed by an extension), plus `delete` and core-made writes (§3). No merge and no run-command operation. |
| D2 | A config file shared with the user (example: claude extension, `.claude/settings.json`) is read by the extension (declared `reads`), edited with the pure `agentstacks.json.edit` helper, and rewritten whole. The core knows no config format. A shared file is not recorded in the lock (§6.2). |
| D3 | Refusals only where an invariant needs one: the file changed or is not ours; the write lands outside the project; the operation is not declared or is malformed; a binary is missing or out of range. Tool errors (git) are passed through. |
| D4 | airsl is embedded as a library (`airsl = "0.1.4"`, kept at the newest `0.1.x` in `Cargo.lock`). It is extended only through its public `HostModule` API. Gaps are filed upstream (airsstack/airsl#11–#15). |
| D5 | The adapter registers handlers, and the core dispatches two events, `values` then `plan`. A handler registered twice: the last one wins. |
| D6 | Every pack item the extension receives must be covered by an operation (write, copy or skip), otherwise the plan is refused. |
| D7 | The core writes each pack's root files to `<root>/<pack>/…` and the shared `<root>/runtime.lua`. The extension names `<root>`. |
| D8 | A tool removed from `agentstacks.toml` runs its locked extension once with zero packs, so it cleans its own shared-file entries. Its locked files are then deleted. |
| D9 | Numbers in a shared JSON file are re-printed in serde_json's normal form. The plan's diff shows any change. |
| D10 | Cache: `~/.airsstack/cache/`. Concurrency is handled by fetching into a temporary directory and renaming it into place. There is no lock file. |
| D11 | Git never prompts (`GIT_TERMINAL_PROMPT=0`). Git's own error is shown. |
| D12 | A `block` from a hook not declared `may_block` is dropped, with one line on stderr. Any `context` it returned is kept. |
| D13 | Layout: ports and adapters (`docs/architecture.md`). Rules go in `types/`, flows in `app/`, traits in `port/`, translation in `fs/`, `process/` and `host/`. |
| D14 | Pack hooks with `grants.policy = "pure"` are refused at pack load in api 0. Each segment of a hook `run` path must be require-able. |

## 2. Architecture

Approved by the author on 2026-11-21, and written up in `docs/architecture.md`, which `CLAUDE.md` and `README.md` link to.

```text
crates/agentstacks/src/
├─ types/      RULES: domain data + pure decisions; existing formats + op · plan · resolve · json · protocol
├─ port/       traits + a fake for each: FileTree (moved from types/port) · Workspace · Git · Binary · Adapter
├─ app/        FLOWS: use cases generic over ports: plan · apply · doctor · update
├─ fs/         adapter: FileTree (built) + Workspace on the real disk; the only std::fs caller
├─ process/    adapter: Git on the git CLI · Binary (PATH lookup, `airsl --version`); the only std::process caller
└─ host/       adapter: Adapter on the airsl sandbox; the only place Lua runs
removed: the empty seams executor.rs, host.rs, plan.rs, validation.rs (src/lib.rs:26-31)
```

**Dependency rule:**

```text
types → port::{FileTree, EntryKind} only (X2)      port → types          app → port, types
fs, process, host → port, types;  process → fs::cache only (X3)    agentstacks-cli → app + adapters
```

`tests/isolation.rs` today guards `src/types` against `std::fs`, `std::process`, `std::net` and
`File::open` (`tests/isolation.rs:17`). It is extended to cover `src/port` and `src/app`, and to
refuse any `use crate::fs`, `crate::process` or `crate::host` inside those three.

| Module | Job (one each, modularity Rule 1) |
|---|---|
| `types/op` | `Operation` and its validation (§3), defined once (Rule 2); `host` builds it, never a copy |
| `types/resolve` | lock vs `agentstacks.toml` reconciliation, tag choice (§9) |
| `types/plan` | the step for each path, from op, lock and disk state (§6) |
| `types/json` | `edit(text, path, value)` (§5) |
| `types/protocol` | the hook command line, the text of `runtime.lua` (§11) |
| `port/*` | `Workspace` (project disk state, staging, swap), `Git`, `Binary`, `Adapter`. One `Fake*` each under `#[cfg(test)]`, as `types/port/fake.rs` is today (`src/types/port/mod.rs:19-20`) |
| `app/plan`, `app/apply`, `app/doctor`, `app/update` | the order of steps; each calls ports and hands the results to `types` rules |
| `fs`, `process`, `host` | translation only |

**Rust guideline rules applied** (claudestacks-guideline-rust 0.1.6; its architecture references are unchanged from 0.1.5):

| Rule | Applied |
|---|---|
| `modularity.md` Rule 1 (one responsibility, no "and" in a module's job) | the table above; `app/` holds no rule |
| `modularity.md` Rule 2 (one canonical type per concept) | `Operation`, `Plan`, `Step` exist only in `types/` |
| `static-dispatch.md` | ports are generic parameters (`fn plan<W: Workspace, G: Git, B: Binary, A: Adapter>`). One `dyn`: `ModuleSet::insert(Box<dyn HostModule>)`, which airsl's API requires (`registry.rs:144`, `probes/airsl-results.md` Q8), marked `// dyn: airsl's ModuleSet stores trait objects` |
| `mod-rs-export-only.md` | every `mod.rs` holds only its docs and `mod` / `pub use` |
| naming (project rule) | one-word folders and files, no `_`, no repeated folder word |

## 3. Operations (`types/op`)

### 3.1 Kinds

| Kind | Proposed by | Fields | Effect |
|---|---|---|---|
| `write` | extension | `path`, `content` (bytes), `covers` (list of item ids, optional) | put exactly these bytes at `path` |
| `copy` | extension | `path`, `file` (a file id from the plan payload) | put that pack file's final bytes at `path`. For a rendered file that is the whole rendered file; for `SKILL.md` or an agent, the neutral frontmatter is included. Otherwise the raw bytes. It covers `file` |
| `skip` | extension | `item` (an item id), `reason` (non-empty) | nothing is installed; `plan` shows the reason |
| `require_binary` | extension | `name` | plan refuses when `name` is not on `PATH` |
| `write` (core) | core | — | each pack root file → `<root>/<pack>/<rel>`; `runtime.lua` → `<root>/runtime.lua` when a pack of this tool has a `lua` hook or a native hook that is not skipped |
| `require_binary` (core) | core | — | each pack's `requires.binaries`; `airsl` with the intersected range when any locked pack declares `requires.airsl` (§10) |
| `delete` | core | `path` | a locked file no longer proposed (§6.2) |

`by` (the lock's owner, `src/types/lock/owner.rs`: a pack or tool name) is derived, never passed:

- a copy, or a write whose `covers` all belong to one pack, is owned by that pack;
- a core write of a root file is owned by its pack;
- any other write (no `covers`, `covers` spanning several packs, `runtime.lua`) is owned by the tool.

A shared file (§6.2) is never locked, so it has no `by`.

### 3.2 Item ids and coverage (D6)

Ids are strings the plan payload hands out (§4.4). The extension copies them and never builds one.

| Item | Id | Covered by |
|---|---|---|
| skill file | `core/skill/status/SKILL.md`, `core/skill/status/references/x.md` | a `copy` of it, a `write` listing it in `covers`, or a `skip` of it or of its skill |
| skill (group) | `core/skill/status` | used only by `skip`; covers all of its files |
| agent | `core/agent/coder` | `write` with `covers`, or `skip` |
| hook (`pack.toml`) | `core/hook/0` (index in `[[hooks]]`) | `write` with `covers`, or `skip` |
| native file | `core/native/claude/x.md` (only this tool's `native/<tool>/`) | `copy`, `write` with `covers`, or `skip` |
| native hook | `core/native-hook/0` (index in `native/<tool>/hooks.toml`) | `write` with `covers`, or `skip` |

Root files are not items, because the core writes them (D7). Items the core already skipped (§4.4) are left out of the payload and need no cover.

| Case | Result |
|---|---|
| an item covered by no op | refused, one per item, in id order: `claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a reason)` |
| an item both skipped and copied or written; a skill group skipped while one of its files is copied or written | refused at the later call: `claude/adapter.lua:14: skip: item "core/skill/status" is already covered at adapter.lua:9` (the same shape for `copy` and `write` after a skip: `… covers "core/agent/coder", which adapter.lua:9 skipped`) |
| the same item skipped twice | accepted; the plan shows the first reason |
| the same file copied to two paths; two writes covering the same item; a repeated id inside one `covers` | accepted |
| lua hooks covered by a write, but no `write` to `<root>/translate.lua` | refused: `claude/adapter.lua: lua hooks need <root>/translate.lua (write it)`, with `<root>` filled in |

### 3.3 Validation

An extension op is checked the moment the Lua function is called (§4.3). The refusal names the caller's file and line:

- inside a handler, `lua.inspect_stack(1, …)` gives `Some((Some("adapter.lua"), Some(7)))` (`probes/round2-results.md` R5);
- in a dispatched handler, `convert at adapter.lua:2` (`probes/round3-results.md` A4);
- a call made from a required module names that module: `{"at":"lib/settings.lua:3","here":"adapter.lua:3"}` (`probes/examples-results.txt`).

Location shape: §13. Example: `claude/adapter.lua:12: write: path "/etc/x" must be relative`.

| Rule | Refusal (after `<tool>/<file>:<line>: <function>: `) |
|---|---|
| argument is one table (`read` takes one string) | `expected table, got string` / `expected string, got number` |
| required field present (`nil` counts as missing) | `path: required` (same shape for `content`, `file`, `item`, `reason`, `name`) |
| field type matches exactly; no coercion | `path: expected string, got number` / `covers: expected table, got string` / `covers[2]: expected string, got boolean` |
| `covers` is a list | `covers: expected a list` |
| no unknown field | `foo: unknown key` |
| `path` is UTF-8 | `path: not valid UTF-8` |
| `path` shape: the lock's file path rule (formats §8 `[[file]].path`), plus `/` as separator | `path: empty` / `path "/x" must be relative` / `path "../x" must stay inside the project` / `path "a\b" must use / as the separator` |
| `path` is not one of agentstacks' own files | `path "agentstacks.lock" is managed by agentstacks` (same for `agentstacks.toml`) |
| `path` not proposed twice in one plan, by any op of any tool or by the core | `path ".claude/x" is already proposed at adapter.lua:9` / `… already proposed by opencode` / `… already proposed by core (root file of core)` |
| `file`, `item`, `covers[i]` name an id in this run's payload | `file "core/skill/x/SKILL.md" is not in this plan` (same shape for `item`, `covers[1]`) |
| `skip.reason` not empty or whitespace-only | `reason: empty` |
| `require_binary.name` follows the binary name rule (`src/types/name/binary.rs:53`) | `name: "Git" is not a valid name` |
| `read` path in `extension.reads` | `".env" is not in extension.reads` |
| called outside the `plan` handler (ops and `read`) | `only allowed inside the plan handler` |

**Normalisation:** every op path, every `reads` entry and every `values` path (`root`, `dir`) is normalised on receipt by removing `.` and empty segments (formats §8: `./a` = `a`, `a//b` = `a/b`). The duplicate check, the lock lookup and `[[file]].path` all use the normalised form.

**Accepted without refusal:**

- an empty `content` (an empty file);
- an empty `covers` (same as none);
- the same `require_binary` name more than once (checked once).

**Type words** are Lua's: `nil`, `boolean`, `number`, `string`, `table`, `function`, `userdata`, `thread`. An integer and a float are both `number`.

**Strictness is hand-written.** The probes show why:

- airsl's raw reading turned `path = 5` into `"5"`: `raw path=5: OK ()`, `RawRecord { kind: "write", path: "5", … }` (`probes/airsl-results.md` Q3);
- serde reading accepts unknown keys (Q3 `serde unknown key: OK ()`).

So arguments are read as `mlua::Value` and matched by variant. `content` keeps raw bytes: `as_bytes=[97, 255, 98, 0, 99]` (`probes/round2-results.md` R4).

**Refusals stick:** the host records each op refusal when it happens. The plan is refused even when the Lua code catches the error with `pcall`, and the refused op is not recorded.

## 4. Extension API, `api = 0` (`host`)

### 4.1 Load

1. Resolve the source (§9) and load it with `Extension::load` (`src/types/extension/loaded.rs:100`), which checks `extension.toml` (formats §4.5, plus `reads`, §12).
2. `api = 0` is already required by formats §4.5.
3. Every `.lua` file of the extension is read into memory. Lua never touches disk.

### 4.2 Sandbox

```text
Engine::builder().policy(Policy::pure()).stdlib(set).build()
set = Json · Ext::with_events([values, plan]) · Agentstacks (ours)
```

**Removed by our module:** `print` and `warn`, the two output channels. Removing one is proven: Q2 `OK "nil"` (`probes/airsl-results.md`).

**Absent:** `require`, `load`, `dofile`, `os`, `io`, `coroutine`, and every airsl module not in the set. Q1 shows `require=nil load=nil dofile=nil os=nil io=nil … coroutine=nil` and `airsstack keys … agentstacks,json`.

**Present:**

- Lua's remaining base functions (`pcall`, `error`, `pairs`, `ipairs`, `type`, `tostring`, `setmetatable`, `rawget`, …), plus `string`, `table`, `math` and `utf8`, as Q1 `ALL_GLOBALS` lists them;
- `airsstack.json`, `airsstack.ext`;
- `airsstack.agentstacks`, also set as the global `agentstacks` (§4.3).

`Ext` and `dispatch` work under `pure()` (`probes/round2-results.md` R1b). `pure()` alone, without a custom set, still installs stdio, hook and time (airsl#11), so the set is always explicit.

**Limits** (`Policy::pure()`):

- 10,000,000 instructions per evaluation or dispatch;
- 16 MiB of memory, and host-supplied data counts toward it.

The budget resets on each dispatch (`airsl-0.1.4/src/engine.rs:342`: "a dispatch resets the instruction budget").

- Breach messages: `script \`spin\` exceeded its instruction limit of 10000000` (`probes/round2-results.md` R2b), `script \`adapter.lua\` exceeded its memory limit of 16777216 bytes` (`probes/airsl-results.md` Q9a).
- An 8 MiB string returned from a host function fits (Q9d).
- A breach caught by the extension's own `pcall` is that extension's business. The dispatch then succeeds: `{"caught":false,"err":"instruction budget of 10000000 exhausted"}` (`probes/examples-results.txt`, P).

### 4.3 The `agentstacks` table

| Function | Phase | Returns |
|---|---|---|
| `agentstacks.on(event, fn)` | any | forwards to `airsstack.ext.on`, looked up when called. While modules install, the global `airsstack` does not exist yet (`install: global airsstack is nil`, `probes/examples-results.txt`). Events: `values`, `plan`. A registration replaces the earlier one and takes effect at the next dispatch |
| `agentstacks.require(name)` | any | the module's return value (§4.5) |
| `agentstacks.write{path, content, covers}` | plan | nothing; records an op (§3) |
| `agentstacks.copy{path, file}` | plan | nothing; records an op |
| `agentstacks.skip{item, reason}` | plan | nothing; records an op |
| `agentstacks.require_binary{name}` | plan | nothing; records an op |
| `agentstacks.read(path)` | plan | the file's bytes as a Lua string, or `nil` when no regular file is there |
| `agentstacks.json.edit(text, path, value)` | any | new text (§5) |

`read` rules:

- The path must be listed in `extension.reads`.
- It must resolve inside the project (§6.3), else `claude/adapter.lua:9: read: ".claude/settings.json" resolves outside the project`.
- The bytes are returned as-is, non-UTF-8 included. A Lua string holds raw bytes (`probes/airsl-results.md` Q7: `bytes=0,255,65`).
- The core records what each read returned (the hash, or "absent") for §6.2 and §7.

### 4.4 Events

The flow:

1. Run the entry file at the top level. This registers handlers.
2. `dispatch("values", …)`.
3. Render the packs.
4. `dispatch("plan", …)`.

`dispatch` returns three different results (proven):

- no handler → `Ok(None)`;
- handler returns `nil` → `Ok(Some(Null))`;
- handler returns a table → `Ok(Some(Object …))`

(`probes/round3-results.md` A3, `probes/round2-results.md` R1b). A second registration replaces the first: `A1 dispatch values: OK Some(Object {"which": String("second")})` (`probes/round3-results.md` A1).

**`values` payload:**

```json
{ "api": 0, "tool": "claude",
  "packs": [ { "name": "core", "version": "0.3.0", "skills": ["status"], "agents": ["coder"] } ],
  "vars": ["session_id", "tmpdir"] }
```

**`values` result:**

```lua
-- example: claude extension
agentstacks.on("values", function(q)
  local out = { root = ".claude/agentstacks", skills = {}, agents = {}, vars = { tmpdir = "$TMPDIR" } }
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills) do
      out.skills[p.name .. "/" .. s] = { invoke = "/" .. s, dir = ".claude/skills/" .. s }
    end
    for _, a in ipairs(p.agents) do out.agents[p.name .. "/" .. a] = a end
  end
  return out
end)
```

| Token (formats §3.1) | Value used |
|---|---|
| `{{root}}`, `{{root <pack>}}` | `<root>/<pack>` |
| `{{skill p/s}}` | `skills["p/s"].invoke` |
| `{{skill_dir p/s}}` | `skills["p/s"].dir` |
| `{{agent p/a}}` | `agents["p/a"]` |
| `{{var v}}` | `vars[v]` |
| `{{option o}}` | resolved by the core (formats §4.4) |

Every token is filled through `TokenValues::insert` and `render` (`src/types/tokens/values.rs:80`, `src/types/tokens/render.rs:51`).

**Missing values** (formats §3.5): when a token has no value, the item that holds it is skipped by the core and left out of the `plan` payload. The skip reason is `claude gave no value for {{skill core/status}}`.

- **Skill:** the item is the whole skill (group), because a half-installed skill is broken. A skill with no `skills[...]` entry at all is skipped too: `claude gave no install location for skill core/status`.
- **Agent, hook (any field: `text`, `args`, `run`, grant paths), native file, native hook:** the item itself.
- **Root file:** a core skip of that file.

The `values` result arrives as JSON (`dispatch` converts it, `airsl-0.1.4/src/engine.rs:381`). Type words map JSON back to what the Lua author wrote: object → `table`, array → `list`, null → `nil`, number, string, boolean. An empty Lua table arrives as an object (`A2 … "empty": Object {}`, `probes/round3-results.md` Run 2).

| `values` rule | Refusal (location `<tool>/<entry>`, key path from `values`) |
|---|---|
| a `values` handler exists | `claude/adapter.lua: registers no values handler` |
| result is a table | `claude/adapter.lua: values: expected table, got nil` |
| `root` required, a string, a valid path (§3.3 path rows) | `claude/adapter.lua: values.root: required` / `claude/adapter.lua: values.root: "/x" must be relative` |
| `skills`, `agents`, `vars` are tables with string keys | `claude/adapter.lua: values.agents: expected table, got list` |
| their keys name items and vars of this payload | `claude/adapter.lua: values.skills."core/x": not a skill in this run` / `claude/adapter.lua: values.vars.home: not a var in this run` |
| `invoke`, `dir`, agent value, var value are strings; `dir` a valid path | `claude/adapter.lua: values.skills."core/status".dir: expected string, got number` |
| unknown key, at any level | `claude/adapter.lua: values.foo: unknown key` / `claude/adapter.lua: values.skills."core/status".icon: unknown key` |

**`plan` payload.** Every text is already rendered, and each id is a string to copy back.

```json
{ "api": 0, "tool": "claude", "root": ".claude/agentstacks",
  "packs": [ { "name": "core", "version": "0.3.0",
    "skills": [ { "id": "core/skill/status", "name": "status", "dir": ".claude/skills/status",
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

- `text` is present for a UTF-8 file and absent for a binary one, which is installed with `copy`. For `SKILL.md` and agents, `text` is the body only, and `meta` holds the frontmatter fields (formats §5, with `native` limited to this tool's entry).
- `match` and `agents` are given as in formats §6.1.
- `command` is the argv from §11.2.

The return value of `plan` is ignored. If no handler exists: `claude/adapter.lua: registers no plan handler`.

Tools run in name order. That order decides which tool's call gets `… already proposed by <tool>`.

### 4.5 `agentstacks.require`

`require` stays absent. airsl clears the global before every evaluation of a script built from source (`engine.rs:244` calls `set_require`; `require_loader.rs:69-75` decides "absent" for a script with no root; `require_loader.rs:134-141` sets it to `nil`). So modules load through `agentstacks.require(name)`, a module function, the same form Q4 proved (`Q4 require twice(21): OK 42`, `probes/airsl-results.md`).

- `name` must match `^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)*$`, the character set airsl's binary enforces (`probes/hookreq-results.md`: `invalid require target … must contain only ASCII letters, digits, underscores, hyphens and dots`).
- It maps to `<name with . replaced by />.lua` among the extension's in-memory files.
- It is loaded once, with chunk name `@<path>`, and the result is cached.
- Errors carry the module's file and line: Q4 `runtime error: lib/util.lua:2: boom`.
- The host keeps `load` for itself and never exposes it (Q4: `load still withheld from script: OK "nil"`).

| Rule | Refusal (location: the caller) |
|---|---|
| name shape | `claude/adapter.lua:1: require: invalid module name "lib/x" (use dots: lib.x)` |
| file exists | `claude/adapter.lua:1: require: module "lib.x" not found in extension claude` |
| not the entry file | `claude/adapter.lua:1: require: "adapter" is the entry file` |
| no cycle (reported at the `require` call that closes it, R19) | `claude/lib/b.lua:1: require: module "lib.a" requires itself (lib.a → lib.b → lib.a)` |

A module that raised while loading is not cached. Requiring it again runs it again.

### 4.6 Errors

| Case | What the user sees |
|---|---|
| our op refused | the §3.3 message (sticky, §3.3) |
| `json.edit` refused | a normal Lua error: an extension may catch it with `pcall`, and an uncaught one fails the dispatch |
| any other Lua failure | `<tool>/<entry>: ` + the first line of airsl's error Display, e.g. `claude/adapter.lua: lua error in adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>` (`probes/airsl-results.md` Q5a), `claude/adapter.lua: lua error in plan: adapter.lua:3: path must be relative` (dispatch names the event, `probes/round3-results.md` A5) |
| limit breach | `claude/adapter.lua: script \`plan\` exceeded its instruction limit of 10000000` |
| `agentstacks.on` with another event | airsl's own text, e.g. `claude/adapter.lua: lua error in adapter.lua: event \`nope\` is not one this host dispatches; declared events: spin, values`, as in `probes/round2-results.md` R1b. Here the events are `plan, values`, kept sorted (`BTreeSet`, `airsl-0.1.4/src/modules/ext.rs:34`) |

Any extension error refuses the whole plan. Nothing is written.

## 5. JSON helper (`types/json`, exposed as `agentstacks.json.edit`)

`edit(text, path, value) → text`. `path` is a list of object keys; `value` is a Lua value, and `nil` removes the key.

| Rule | Behaviour / refusal (location: `<tool>/<file>:<line>: json.edit: …`) |
|---|---|
| arguments | `json.edit: text: expected string, got nil` / `json.edit: path: expected table, got string` |
| `text` empty or whitespace-only | treated as `{}` |
| `text` parses as JSON | else `json.edit: <serde_json message>` (e.g. `trailing comma at line 1 column 8`, `probes/json-results.md`) |
| top level is an object | else `json.edit: top level must be an object` |
| `path` is a non-empty list of strings | `json.edit: path: empty` / `json.edit: path[2]: expected string, got number` |
| setting: every key before the last names an object, and a missing one is created as `{}` | `json.edit: "hooks" is not an object` |
| removing (`nil`): a missing key or parent | no change; the text is returned as it came in |
| existing keys keep their position; a new key goes last | `J3 after insert existing+new: {"permissions":{"allow":["Bash(x:*)"]},"zeta":2,"alpha":1.2345678901234568e+22,"d":0.1,"hooks":{},"new":[]}` (`probes/round3-results.md` Run 2, serde_json `preserve_order`) |
| indent | the leading whitespace of the first indented line after the first line, if it is all spaces or one tab; otherwise two spaces. Output via `PrettyFormatter::with_indent` (2 spaces, 4 spaces and tab are all proven byte for byte, `probes/round3-results.md` J1) |
| line endings | `\n`; CRLF input comes back as LF |
| trailing newline | kept when the input ended with `\n`, absent otherwise |
| numbers | re-printed in serde_json's normal form (D9): `12345678901234567890123` → `1.2345678901234568e+22`, `0.10` → `0.1` (J1, Run 2) |
| duplicate keys | the last one wins, serde_json's behaviour (`probes/round2-results.md` R7: `plain Value={"a":2}`) |

Converting the Lua value is hand-written, because mlua's serde conversion drops data: `convert({1, a=2})` → `[1]` (`probes/round3-results.md` A4).

| Lua value | JSON | Refusal |
|---|---|---|
| table with keys exactly `1..n` | array | — |
| table with only string keys | object, keys sorted (Lua has no key order, so output stays deterministic) | — |
| empty table | `{}` (airsl's own rule, `airsl-0.1.4/src/convert.rs:41`: "an empty table encodes as `{}`"). No `[]` can be produced: to empty a list, remove its key | — |
| any other table | — | `json.edit: value: table mixes list and keyed entries` |
| integer, float, string (UTF-8), boolean | number, number, string, boolean | `json.edit: value: NaN cannot be JSON` (also `inf`) / `json.edit: value: string is not valid UTF-8` |
| function, userdata, thread | — | `json.edit: value: function cannot be JSON` |

**Checked example:**

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

The indent is 4 spaces and the trailing newline is kept. The inner layout is pretty-printed: the one-line `"allow"` array becomes multi-line.

**Dependency:** `serde_json = { version = "1.0.154", features = ["preserve_order"] }`, with no `arbitrary_precision`. With `arbitrary_precision` on, airsl's dispatch hands Lua `{"$serde_json::private::Number": String("1")}` in place of `1` (`probes/round3-results.md` Run 1, A2). Cargo feature unification would turn it on for airsl too.

## 6. Plan (`types/plan`, `app/plan`)

### 6.1 Flow (`app/plan`)

```text
1  read agentstacks.toml + agentstacks.lock                         Workspace
2  reconcile lock vs toml → what to fetch (§9.1)                    types/resolve
3  fetch / read sources, tree hashes                                Git, Workspace
4  load + check the set (formats §9 check)                          types (built)
5  per tool, in name order: values → render → plan (§4)             Adapter
   per removed tool: values → plan with zero packs (D8, §6.4)
6  validate ops, coverage, duplicates (§3)                          types/op
7  binary checks: packs' requires.binaries, extension require_binary, airsl (§10)   Binary
8  read each target's disk state                                    Workspace
9  decide each path's step (§6.2)                                   types/plan
10 return Plan, or every refusal                                    —
```

`Plan` holds:

- steps, each with path, action, owner and op origin, plus the new bytes; for a shared file, the old bytes too, so the CLI can show a diff;
- skips with their reasons;
- binary checks;
- notes (formats' dot-entry and unused-`native/<tool>` listings);
- the disk state each step was decided on;
- the new lock contents.

Refusals are collected, never first-only, and sorted (formats §10 `Diagnostics`).

### 6.2 Steps

**Disk states** (from `Workspace`, §6.3): absent · regular file (bytes, hash, mode) · not a regular file.

**Shared** means the path is in this tool's `extension.reads` *and* the extension read it in this run. A shared file is never written to the lock (D2), so `doctor` never sees it, and a pack or tool removal handles it by rerunning the extension.

| Proposed path | In the lock | Disk | Step |
|---|---|---|---|
| any | any | not a regular file | refused `<path>: is not a regular file` |
| shared | — | absent, and `read` returned absent | `create` |
| shared | — | equals what `read` returned | `update`, or `unchanged` when the new bytes equal the disk; old bytes kept for the diff. A lock entry for it, if any, is dropped |
| not shared | yes | absent | `restore` |
| not shared | yes | hash = lock hash | `update`, or `unchanged` when equal |
| not shared | yes | hash ≠ lock hash, bytes = new bytes | `unchanged` (the user made the same edit; the lock takes the new hash) |
| not shared | yes | hash ≠ lock hash, bytes ≠ new bytes | refused `<path>: changed since the last apply` |
| not shared | no | absent | `create` |
| not shared | no | bytes = new bytes | `unchanged` (adopted into the lock) |
| not shared | no | bytes ≠ new bytes | refused `<path>: exists and was not written by agentstacks` |

| Locked path no longer proposed | Disk | Step |
|---|---|---|
| any | absent | `forget` (dropped from the lock) |
| any | the path itself is now a symlink | `forget`; the user replaced it, so it is left alone |
| any | hash = lock hash | `delete` |
| any | hash ≠ lock hash | refused `<path>: changed since the last apply; not deleting it` |
| any | not a regular file | refused `<path>: is not a regular file` |

**Checked examples:**

| Situation | Step |
|---|---|
| first apply; `.claude/skills/status/SKILL.md` absent | `create` |
| user edited that file after apply; pack unchanged | refused `.claude/skills/status/SKILL.md: changed since the last apply` |
| user deleted it | `restore` |
| pack `team-lint` removed from toml; its file untouched | `delete` |
| `.claude/settings.json` holds the user's permissions; the claude extension read it and proposes it with hooks added | `update` (shared), diff shown, not locked |
| `.claude/settings.json` absent; the extension read `nil` and proposes it | `create` (shared), new-file mode |
| `.claude/agents/coder.md` exists, never written by agentstacks, different content | refused `.claude/agents/coder.md: exists and was not written by agentstacks` |
| a proposed path is a symlink to `/etc/hosts` | refused `<path>: resolves outside the project` |

### 6.3 Inside the project (one rule)

Every proposed path, deleted path and `read` path is resolved by `Workspace`:

1. If the path is a symlink, follow it. A dangling symlink is "not a regular file".
2. Otherwise canonicalise the deepest existing ancestor. `canonicalize` of a missing path gives `NotFound` (`probes/round3-results.md` S5).
3. The result must start with the canonical project root. Otherwise: refused `<path>: resolves outside the project` (for `read`, at the caller's line, §4.3).

The same `canonicalize` plus `starts_with` check catches an escape (`probes/round2-results.md`, crate std).

A symlink that stays inside the project is written through: the write goes to its canonical target, and the link survives (`probes/round3-results.md` S4: `rename onto canonical target: link2 is_symlink=true real content="B"`). The lock records the normalised proposed path, never the canonical one.

A `path` source directory goes through the same rule. Formats decides "inside" lexically and hands symlinks to this chain (formats spec, `path` rules: "symlinks are resolved by `core-plan-and-apply`"). Outside: refused `agentstacks.toml: packs.team-lint.path: resolves outside the project`.

### 6.4 Removed tool (D8)

- The removed tool's locked extension is resolved from the lock (§9.1) and run with an empty `packs` list.
- Only its writes to paths it read in that run (shared files) are kept. Every other proposal is dropped, and no core write is made for its root.
- All of its locked files then get delete steps under the §6.2 rules.
- Nothing from that tool is locked.

## 7. Apply (`app/apply`, `fs`)

`app/apply` takes the `Plan` that `app/plan` returned, and the same `Plan` the CLI showed. It never re-plans.

```text
1  refuse a Plan that holds refusals
2  stage: for each create/update/restore write <dir>/.<name>.agentstacks-tmp
      (parents created; a leftover temp of the same name is overwritten)
3  check: every target's disk state still equals the state the Plan recorded
      (for a shared file: what read returned); any differs →
      remove the temps, refused `<path>: changed after plan; run plan again`
4  swap: rename each temp onto its (canonical) target; perform each delete
5  lock: write agentstacks.lock last (write_lock, src/types/lock/write.rs:58), via temp + rename
```

**Modes:**

- A new or restored file gets what `std::fs::write` creates (`644` here, `probes/round3-results.md` S2).
- An updated file keeps its old mode. A rename installs the temp's mode (`S3 mode after rename over 755 file: 644`), so `fs` copies the old permissions onto the temp before the rename.

**Failures:**

| Step | Result |
|---|---|
| 2 or 3 | the temps written so far are removed, nothing is swapped, the lock is untouched; I/O failure: `<path>: <io::Error Display>`. Directories already created are left |
| 4 | stop; remove the remaining temps; write the lock with only what landed (renamed files carry their new hash, the rest keep their old entries); report `<path>: <io::Error Display>` |
| 5 | `agentstacks.lock: <io::Error Display>`. The files have landed, so the next plan finds each one equal to its new bytes (the "bytes = new bytes" rows of §6.2), and the lock heals |

Directories a delete leaves empty are left in place.

**Lock content:**

- `[[tool]]` and `[[pack]]` with rev and tree;
- `[[file]]` for every non-shared path in a create, update, restore or unchanged step, with its `by`;
- `[airsl] version` exactly when a locked pack declares `requires.airsl`, so `check_airsl_presence` (`src/types/lock/airsl.rs:57`, keyed on `requires.airsl`) passes.

## 8. Doctor (`app/doctor`)

Offline. Reads the lock and the disk, and returns findings; none means healthy.

| Check | Finding |
|---|---|
| a lock exists | `agentstacks.lock: not found (run apply)` (then no other check) |
| lock parses | the lock's own refusals (formats §8) |
| each `[[file]]` exists | `missing: <path>` |
| each `[[file]]` is a regular file (a symlink to one inside the project counts, §6.3) | `not a regular file: <path>` |
| each `[[file]]` hash matches | `changed: <path>` |
| `[airsl]` present: the binary is on `PATH` | `airsl: not found on PATH (lock pins 0.1.4)` |
| its `--version` parses | `airsl: --version printed "<text>", expected "airsl <version>"` |
| installed version = lock version | `airsl: 0.1.5 installed, lock pins 0.1.4` |

Shared files are not in the lock (§6.2), so doctor needs no extension.

## 9. Sources (`types/resolve`, `app/plan`, `process/git`)

### 9.1 Lock reconciliation

| `agentstacks.toml` | `agentstacks.lock` | Result |
|---|---|---|
| entry | none | new: resolve (git: tag → rev; path: read) |
| entry | entry with the same source fields (`git`, `tag`, `rev`, `subdir` / `path`) | use the lock's `rev`; the git tree must equal the lock's `tree`, else refused `agentstacks.toml: packs.core: tree sha256:<a> at rev <rev> differs from the lock's sha256:<b>`; a changed path tree is a normal change, shown in the plan |
| entry | entry with different source fields | stale: resolve as new; plan note `agentstacks.toml: packs.core: source changed, resolved again` |
| none | pack entry | removed: its files get `delete` steps (§6.2) |
| none | tool entry | removed: §6.4 |

A moved remote tag does not move a locked entry. The lock's `rev` wins, and no network is used (formats §13 "rev over a moved tag"). `update` (§9.4) is the only way to move it.

### 9.2 Git (`process/git`)

Every command runs with `GIT_TERMINAL_PROMPT=0` and `-c core.autocrlf=false`, and passes the URL after `--`. `subdir` is normalised (§3.3) before use.

| Step | Command | Proof |
|---|---|---|
| list tags | `git ls-remote --tags --refs -- <url> 'refs/tags/<prefix>*'` → lines `<40hex>\t<ref>`; empty output with exit 0 when nothing matches | `probes/git-results.md` G1 |
| fetch a tag | `git init --bare -q <tmp>`; `git -C <tmp> fetch --no-tags --depth 1 -- <url> refs/tags/<tag>:refs/tags/<tag>`; rev = `rev-parse --verify --end-of-options refs/tags/<tag>^{commit}` (annotated tags need `^{commit}`) | G2 |
| fetch a rev | `git -C <tmp> fetch --no-tags --depth 1 -- <url> <rev>` | G3 (file:// and GitHub) |
| list files | `git -C <repo> ls-tree -r -z --full-tree <rev> -- <subdir>`; records `<mode> SP <type> SP <oid> TAB <path> NUL`, byte-ordered | G4, G9 |
| read blobs | `git -C <repo> cat-file --batch`; bytes unconverted even with `core.autocrlf=true` | G5 |

The source's tree is read into memory once and served through the `FileTree` port:

- mode `120000` → `EntryKind::Symlink`, so formats' "symlinks are not allowed" refusal applies;
- mode `160000` (gitlink) → `EntryKind::Other`.

| Failure | Refusal |
|---|---|
| `git` not on `PATH` (io `NotFound`, which does not name the program: G8) | once: `agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)` |
| git exits non-zero | `agentstacks.toml: packs.core: git <subcommand> failed: <git's stderr, trimmed>`, e.g. `… fatal: could not read Username for 'https://github.com': terminal prompts disabled` (`probes/round3-results.md` G1), `… fatal: couldn't find remote ref refs/tags/packs-v9.9.9` (G7) |
| URL starting with `-` | never reaches a flag: git refuses it itself, `fatal: strange pathname '-oops' blocked` (G6) |
| `subdir` lists nothing at that rev (empty `ls-tree` output, G4 `packs/nope`) | `agentstacks.toml: packs.core.subdir: "packs/nope" not found at <tag or rev>` |
| `subdir` names a file (one blob record whose path equals `subdir`, G4 `packs/core/pack.toml`) | `agentstacks.toml: packs.core.subdir: "packs/core/pack.toml" is a file, not a directory` |
| a removed tool (§6.4) whose locked source is neither cached nor fetchable | the git refusal above; the plan is refused, and nothing is removed until the source is reachable |

### 9.3 Cache

- **Layout:** `~/.airsstack/cache/git/<hex sha256 of url>/<rev>.git` (bare). A locked rev whose `<rev>.git` exists resolves without network.
- **Filling it:**
  1. Fetch into `<url dir>/tmp-<pid>-<n>` (`<n>` counts up within the process). A tag fetch learns its rev here.
  2. Rename it to `<rev>.git`. A rename onto an existing non-empty directory fails with `DirectoryNotEmpty` (`probes/round3-results.md` S1); another run cached it first, so the temp is removed and the existing one is used.
- **Home directory:** taken from `$HOME`. When it is not set: `agentstacks.toml: HOME is not set, so the cache (~/.airsstack/cache) has no home`.
- Temps left by a killed process are never cleaned up (§18).

### 9.4 Latest tag (`app/update`, shorthand expansion)

`latest_tag(url, prefix) → Option<tag>`:

1. List `refs/tags/<prefix>*` (G1).
2. Take each tag's suffix after `prefix`. Keep those that parse as semver and are not pre-releases.
3. Return the highest by semver; `ls-remote` sorts lexically (G1). If none remains, return `None`.

**Splitting a current tag into prefix and version:** the version starts at the leftmost ASCII digit whose remainder parses as semver.

- `packs-v0.1.0` → prefix `packs-v`, version `0.1.0`
- `x-1.2.3-4.5.6` → prefix `x-`, version `1.2.3-4.5.6`

| Case | Result |
|---|---|
| current tag has no such split | note `agentstacks.toml: packs.core: tag "main-latest" has no version to update from` |
| `latest_tag` returns `None`, or nothing newer than the current tag | note `agentstacks.toml: packs.core: no newer tag than packs-v0.1.0` |
| source pinned by `rev` | note `agentstacks.toml: packs.core: pinned to rev <rev>, not updated` |
| `path` source | nothing |

`app/update` returns one entry per git source: its key, the current tag, and the new tag or the note. Writing a new tag into `agentstacks.toml` belongs to `cli-commands`, and so do shorthand expansion (prefix `packs-v`) and its refusal when `None` (formats §13).

## 10. Binaries and the airsl pin

| Requirement | Check | Refusal |
|---|---|---|
| a pack's `requires.binaries` | `PATH` lookup through `Binary` | `agentstacks.toml: packs.core: binary "rg" not found on PATH` |
| an extension's `require_binary` | `PATH` lookup | `claude/adapter.lua:7: require_binary: "node" not found on PATH` |
| `airsl`, when any locked pack declares `requires.airsl` | `airsl --version` → `airsl 0.1.4` (`probes/hookreq-results.md`), parsed as `airsl <semver>` | not found: `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)` / unparseable: `agentstacks.toml: airsl --version printed "<text>", expected "airsl <version>"` / out of range (formats §4.3 wording): `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4` |

A binary required by both a pack and an extension is reported once for each.

The installed airsl version is written to `[airsl] version`. A newer version in range moves the pin, and the plan shows `airsl 0.1.4 → 0.1.5`.

Formats §13 asked for "a runtime warning when airsl disappears after install". It is met in two ways: the tool's own error for a missing command, and `doctor` (§8).

## 11. Hooks (`types/protocol`)

### 11.1 Layout

```text
<root>/runtime.lua                 written by the core (§11.3)
<root>/translate.lua               written by the extension (required, §3.2)
<root>/<pack>/hooks/enforce.lua    a root file, written by the core (D7)
```

airsl's `require` root is the script's directory (`probes/hookreq-results.md`: `require("core.hooks.enforce")` from `root/runtime.lua` works). A hook `run` of `{{root core}}/hooks/enforce.lua` becomes the module `core.hooks.enforce`.

### 11.2 Command line

```text
airsl run --fail-open --policy confined
  [--allow-read <p>]… [--allow-write <p>]… [--allow-env <N>]… [--allow-exec <b>]…
  <root>/runtime.lua <module> <may_block: 0|1> <kind: neutral|native> [args…]
```

- The core builds this argv with project-relative paths. Grants map one to one (formats §6.3).
- **api 0 contract, the extension's job:** make the tool run this command with the project directory as its working directory. Example: claude extension, `cd "$CLAUDE_PROJECT_DIR" && …`. How a tool sets the working directory is not this chain's claim; it is handed to each tool extension (§17).
- A project-relative grant then resolves against that working directory: `--allow-read .tool/agentstacks/core` let the hook read `.tool/agentstacks/core/data.txt` (`probes/runtime-results.txt`, first run). Without the grant: `fs.read denied: … is outside the granted read roots: none are granted` (second run).
- `--fail-open` is fixed (formats §6.3). A missing module exits `0` silently (sixth run). Without it: `module \`core.hooks.missing\` not found under …`, exit 1 (seventh run).
- Elements may hold a `{{var}}` value the extension supplied (an expression such as `$TMPDIR`). Turning the argv into the tool's command string, quoting included, is the extension's job (§17).

### 11.3 `runtime.lua` (shipped by the core, api 0)

```lua
-- agentstacks hook runtime, api 0.
-- usage: airsl run ... runtime.lua <hook-module> <may_block: 0|1> <kind: neutral|native> [args...]
local translate = require("translate")
local name, may_block, kind = arg[1], arg[2] == "1", arg[3]
local args = {}
for i = 4, #arg do
  args[#args + 1] = arg[i]
end
local payload = airsstack.hook.payload()
local event
if kind == "native" then
  event = { payload = payload, args = args }
else
  event = translate.event(payload)
  if event == nil then
    return
  end
  event.args = args
end
local result = require(name)(event)
if type(result) == "table" and result.block ~= nil and not may_block then
  airsstack.stdio.error("agentstacks: hook " .. name .. " tried to block but is not declared may_block; reason: "
    .. tostring(result.block) .. "\n")
  result.block = nil
end
local out = translate.result(result)
if out ~= nil then
  airsstack.stdio.write(out)
end
```

This exact text ran on airsl 0.1.4 (`probes/runtime-runtime.lua`, `probes/runtime-results.txt`):

| Run | Output |
|---|---|
| context | `{"context":"event=tool.before session=s1 args=--manifest,x.json read=true:hello\n …"}` |
| block from a hook that isn't `may_block` | stdout empty; stderr `agentstacks: hook core.hooks.blk tried to block but is not declared may_block; reason: edits to tool.before are not allowed` |
| same hook with `may_block` 1 | `{"decision":"block","reason":"edits to tool.before are not allowed"}` |
| native | `{"context":"native payload tool_name=Edit event=nil"}` |
| context and block from a hook that isn't `may_block` | stdout `{"context":"keep this context"}`; the block line on stderr |
| translator returns `nil` (an event it does not map) | no output, exit 0 |

`airsstack.stdio` (`error`, `write`) and `airsstack.hook.payload` exist under `confined` (`probes/runtime-inspect-results.txt`).

### 11.4 Neutral protocol

**Event** (built by the translator):

| Field | When |
|---|---|
| `event` | always: one of formats §6.1 |
| `session_id`, `cwd` | always |
| `agent` | always; `nil` = the main agent |
| `args` | always (added by the runtime) |
| `source` | `session.start`: `startup`, `resume`, `clear` or `compact` |
| `prompt` | `prompt.submit` |
| `tool` (`read`, `edit` or `write`), `path` | `tool.before`, `tool.after` |
| `last_message` | `agent.stop` |

There is no raw payload on a neutral event. A native hook gets `{ payload = <raw>, args = … }`.

**Result** (returned by the hook): `nil` · `{ context = "<text>" }` · `{ block = "<reason>" }` · both.

**Translator contract** (`<root>/translate.lua`, one per tool, written by the extension):

- `M.event(payload)` returns the neutral event, or `nil` for an event that is not for this runtime (the hook is then not run).
- `M.result(result or nil)` returns the string to print, or `nil`.

The probe's example translator is `probes/runtime-translate.lua`.

**Filtering by `match` and `agents`** (formats §6.1) is the extension's job, in the tool's own config. The runtime does not filter.

**Message hooks** (`kind = "message"`) never reach the runtime; the extension renders them natively.

### 11.5 Shims for other hook languages

When a tool demands another language (example: OpenCode plugins are TypeScript), the extension writes a thin shim with `write`. The shim only runs the §11.2 argv and forwards the payload and result. The core gives it nothing new: the shim is an ordinary file, shown in the plan like any other. Writing it belongs to that tool's extension (§17).

## 12. Changes to the built formats

| Change | Where | Refusal |
|---|---|---|
| `extension.reads`: optional list of project paths (formats §8 path rule); duplicates refused after normalising | `src/types/extension/manifest.rs` (today an unknown key: `extension.toml: extension.foo: unknown key` is the shape tested at `manifest.rs:414`) | `extension.toml: extension.reads: expected array, got string` / `extension.toml: extension.reads[0]: expected string, got integer` / `extension.toml: extension.reads[0]: "/x" must be relative` / `extension.toml: extension.reads[0]: "../x" must stay inside the project` / `extension.toml: extension.reads[1]: ".claude/settings.json" listed twice` |
| lock `[[merge]]` removed; the key is then unknown | `src/types/lock/merge.rs`, `src/types/lock/file.rs:62` (`TOP_LEVEL_KEYS`), `:175` | `agentstacks.lock: merge: unknown key` (the shape at `file.rs:649`) |
| `grants.policy = "pure"` refused; the allowed list becomes `confined` only | `src/types/hooks/policy.rs:54` | `hooks[1].grants.policy: "pure" is not supported in api 0 (airsl has no require under pure, airsstack/airsl#12)` / `hooks[1].grants.policy: "foo" (expected confined)` |
| each segment of the path after the `run` form, with `.lua` removed from the last one, matches `^[A-Za-z0-9_-]+$` (require-able, §11.1); also in `native/<tool>/hooks.toml` | `src/types/hooks/run.rs` | `hooks[1].run: "hooks/my.hook.lua" segment "my.hook" may use only letters, digits, _ and -` |
| `FileTree` and `EntryKind` move from `types/port/` to `port/` | `src/types/port/*` and their users: `grep -rlE "FileTree\|EntryKind" src tests` in `crates/agentstacks` lists 22 files outside `src/types/port` | — |

## 13. Errors: locations and owners

Every refusal is a `Diagnostic` with a `Location`: a file, plus a line or key path (`src/types/report/location.rs:51-56`). It is displayed as `<file>:<line>: <what>` or `<file>: <key>: <what>`, and sorted by the built `Diagnostics` (formats §10).

| Location file | Used for | Example | Raised in |
|---|---|---|---|
| `<tool>/<extension file>` with line | op arguments, `read`, `require`, `json.edit` | `claude/adapter.lua:12: write: path: expected string, got number` | `host` (reading) + `types/op` (rule) |
| `<tool>/<entry>` with key | the `values` result | `claude/adapter.lua: values.root: required` | `host` + `types/op` |
| `<tool>/<entry>`, no line | coverage, missing handler, Lua failure | `claude/adapter.lua: core/agent/coder got no operation (copy it, write it, or skip it with a reason)` | `types/op`, `host` |
| `<path>` (project-relative) | file steps | `<path>: changed since the last apply` | `types/plan`, with the resolution from `Workspace` |
| `<path>` | apply race, I/O | `<path>: changed after plan; run plan again` | `app/apply`, `fs` |
| `agentstacks.toml` with key | sources, a pack's binaries | `agentstacks.toml: packs.core: git fetch failed: <stderr>` | `process`, `types/resolve` |
| `agentstacks.toml`, no key | git, airsl and cache-home checks | `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)` | `types/op`, from a `Binary` answer |
| `agentstacks.lock` | lock write | `agentstacks.lock: <io::Error Display>` | `fs` |
| formats files | as formats §10 | — | `types` (built) |

`<extension file>` is the file's path inside the extension, e.g. `claude/lib/settings.lua:3`. A plan with any refusal applies nothing.

## 14. Testing

| Layer | How | Real OS, git or airsl |
|---|---|---|
| `types/op`, `types/plan`, `types/resolve`, `types/json`, `types/protocol` | colocated unit tests on plain values; every row of every table in §3, §5, §6, §9, §12 is a test | no |
| `app/plan`, `app/apply`, `app/doctor`, `app/update` | unit tests with `FakeWorkspace`, `FakeGit`, `FakeBinary`, `FakeAdapter`. Cover: every checked example in §6.2, the apply race, each failure row in §7 (a fake I/O error at the second rename leaves a lock holding only the first), removed pack, removed tool (§6.4), stale lock, moved tag, every doctor row | no |
| `fs::StdWorkspace` | contract tests on a `tempfile` dir: mode kept, symlink written through, symlink delete forgotten, escape refused, dangling link, cache rename race | disk |
| `process::GitCli` | contract tests on a local bare repo (`file://`), never the network | git |
| `process::Binary` | `airsl --version` parse on the real binary when present, skipped with a printed reason when absent | airsl |
| `host` | real airsl engine on inline Lua: each §4 rule and message, require (cycle, missing, entry), sticky refusal under `pcall`, limits, the `values`/`plan` examples | airsl library |
| `runtime.lua` | the §11.3 text run with `airsl run` against fixtures like `probes/runtime.sh`, when the binary is present | airsl binary |

No logic is tested only end to end. `cargo make dod` is the gate.

## 15. Dependencies, policy and docs

| Change | Proof / reason |
|---|---|
| `airsl = "0.1.4"` (workspace dependency) | MSRV `1.94` and `Apache-2.0` (`airsl-0.1.4/Cargo.toml:14,33`) match the workspace |
| `serde_json = { version = "1.0.154", features = ["preserve_order"] }` | §5 |
| `deny.toml`: `[bans] skip-tree = [{ crate = "airsl@0.1.4", reason = "airsl 0.1.4 pins sha2 0.10 and toml 0.8; agentstacks uses sha2 0.11 and toml 1" }]` (X1) | without it `cargo deny check` gives `advisories ok, bans FAILED, licenses ok, sources ok` (duplicates `block-buffer`, `cpufeatures`, `crypto-common`, `digest`, `serde_spanned`, `sha2`, `toml`, `toml_datetime`, `winnow`); with it `advisories ok, bans ok, licenses ok, sources ok` (`probes/deny-results.md`, run on a copy of this workspace with both deps and `Zlib` added) |
| `deny.toml`: add `Zlib` to `[licenses] allow` | `preserve_order` pulls `foldhash 0.2.0` (`license = "Zlib"`), which `cargo deny check licenses` rejects today (`probes/round3-results.md` D1, D2) |
| `tempfile` stays a dev-dependency | staging uses `std::fs` only (§7) |
| `docs/extensions.md`, `docs/hooks.md`, `docs/plan-and-apply.md` written; `docs/README.md` table updated | `docs/README.md` lists them as "after the spec" |

## 16. Edge rules (settled with the author on 2026-11-22)

These rules close the gaps found while planning. They override any earlier section where the two meet.

### 16.1 Decisions

| # | Decision |
|---|---|
| X1 | `deny.toml` exempts airsl's subtree from the duplicate-version ban (`skip-tree`, §15, `probes/deny-results.md`). An airsl issue asks for `sha2` 0.11 and `toml` 1, so the exemption can go later. |
| X2 | `types` may import only `port::{FileTree, EntryKind}`, the read-only tree port the built formats loaders take. `tests/isolation.rs` allows exactly these two names from `crate::port` inside `src/types`. |
| X3 | `process` may use `fs::cache`, the cache-directory helper, and nothing else from `fs`. It is the only edge from one adapter to another. |

### 16.2 Rules

| # | Rule | Messages |
|---|---|---|
| R1 | Any I/O failure with no message of its own: the cache, a `reads` path, the lock, `agentstacks.toml`, or spawning git or airsl | `<path>: <io::Error Display>` at the path involved. For a spawn: `agentstacks.toml: packs.core: git: <io::Error Display>` / `agentstacks.toml: airsl: <io::Error Display>` |
| R2 | Git or airsl output that does not parse: a `cat-file` frame, a `<oid> missing` answer, `rev-parse` output that is not a commit id, an `ls-tree` record or an `ls-remote` line. Output is decoded as lossy UTF-8 and trimmed of surrounding whitespace. airsl exiting non-zero counts as unparseable | `agentstacks.toml: packs.core: git <subcommand> printed output agentstacks cannot read: "<first line>"` / `agentstacks.toml: airsl --version printed "<text>", expected "airsl <version>"` (`<text>` trimmed) |
| R3 | Lua argument shapes that have no row of their own | `<fn>: <field>: expected <type>, got <type>` / `<fn>: <field>: expected a list` / `<fn>: <field>: not valid UTF-8` / `<fn>: keys must be strings, got <type>` / `require: expected string, got <type>` / `json.edit: text: not valid UTF-8` / `json.edit: path: expected a list` / `json.edit: value: <type> cannot be JSON` (function, userdata, thread) / `json.edit: value: -inf cannot be JSON` / `json.edit: value: table contains itself` / `json.edit: value: key is not valid UTF-8`. An extension `.lua` file that is not UTF-8: `<tool>/<file>: not valid UTF-8` |
| R4 | `values.root` and each `dir` follow the §3.3 path rows under their key. `.` normalises to empty. The managed-file rule does not apply. A key in `values.agents` that names no agent in the run | `claude/adapter.lua: values.root: empty` / `… values.root: "../x" must stay inside the project` / `… values.root: "a\b" must use / as the separator` / `claude/adapter.lua: values.agents."core/x": not an agent in this run` |
| R5 | One Lua call raises one refusal: the first failing check, in table order. The location is the nearest Lua frame on the stack, so `pcall(agentstacks.write, t)` is located at the `pcall` line | — |
| R6 | Order: listed tools in name order, then removed tools in name order. Each listed tool's core writes are recorded just before its `plan` dispatch | earlier proposer is the core's `runtime.lua`: `path "<p>" is already proposed by core (runtime.lua of claude)` / a core write that meets an earlier tool's path: `agentstacks.toml: tools.opencode: path "<p>" is already proposed by claude` |
| R7 | Any refusal in §6.1 steps 1–7 skips steps 8–9. No step is decided, so no false delete appears | — |
| R8 | Tie-breaks: equal semver precedence (build metadata only) → the greater tag in byte order; `values` payload `vars` are the vars used by this tool's packs, sorted by name; doctor findings follow the lock's path order, then airsl; update entries follow key order (`packs.*`, then `tools.*`); with several missing values, the skip reason names the first missing token in text order, and a missing skill entry comes before any token | — |
| R9 | "Same source fields" (§9.1) compares normalised values (`./packs/core` = `packs/core`). Messages quote a value as written | — |
| R10 | `HOME` that is unset or empty is "not set", reported once. "On PATH": a regular file with an execute bit; empty `PATH` entries are skipped; an unset `PATH` finds nothing. Git modes: `100644` and `100755` → file, `120000` → symlink, any other → other | — |
| R11 | `runtime.lua` is written when a pack of the tool has any lua hook, neutral or native, whether or not the extension later skips it. The `translate.lua` rule (§3.2) applies when `runtime.lua` is written and at least one lua hook is covered by a write. Native hooks in the payload use the hook shape: `kind`, `event`, `match` as written (string or list), `may_block`, and `command` (lua) or `text` (message) | — |
| R12 | A `values.skills` entry needs both `invoke` and `dir` | `claude/adapter.lua: values.skills."core/status".dir: required` (same shape for `invoke`) |
| R13 | `copy.file` must name a file id; `covers` may not name a skill group | `copy: file "core/agent/coder" is not a file` / `covers[1]: "core/skill/status" is a skill; list its files` |
| R14 | `json.edit`: removing through a parent that exists but is not an object refuses, the same as setting. Whitespace-only text takes its indent and trailing newline from the original text. Several failing rows: the first row of the §5 table wins | `json.edit: "hooks" is not an object` |
| R15 | §6.2 additions: a shared file whose disk differs from what `read` returned at step 8 is refused. A locked path no longer proposed that is now a symlink (dangling or pointing outside included) is `forget`, checked before any other row | `<path>: changed after it was read; run plan again` |
| R16 | A lock `[[file]].path` containing `\` is refused when the lock is read (formats change, §12) | `agentstacks.lock: file[0].path: "a\b" must use / as the separator` |
| R17 | Lock after a step-4 failure (§7): `[[tool]]`, `[[pack]]` and `[airsl]` take the new values. `[[file]]` holds new entries for files that landed and old entries for those that did not. `forget` and adopted `unchanged` entries apply, because they never touch disk | — |
| R18 | A removed tool's git tree is checked against its lock `tree` like any locked entry. Its dropped (non-shared) proposals never reach the duplicate check | — |
| R19 | `require`: the `(use dots: …)` hint appears only when the name contains `/`, which becomes `.`; otherwise `require: invalid module name "<name>"`. A cycle is reported at the `require` call that closes it (§4.5). `require` and `json.edit` refusals are ordinary Lua errors and do not stick; uncaught, they show their located message, not the §4.6 wrapper | `claude/adapter.lua:1: require: invalid module name "x y"` |
| R20 | Plan notes for formats §2.1/§2.2 listings | `<pack>/<path>: ignored (dot entry)` / `native/cluade: not for claude` (formats §2.2 wording) |
| R21 | Apply temps: beside the canonical target. The lock temp is `.agentstacks.lock.agentstacks-tmp`, and the lock keeps its mode. A path that resolves outside the project at apply time is refused with the §6.3 text. A leftover cache temp of the same name is removed and reused | `<path>: resolves outside the project` |
| R22 | A git source's `FileTree::root()` label, which prefixes its load messages, is the `subdir` as written, or `.` without one | — |
| R23 | Doctor findings for a `[[file]]` that resolves outside the project, and for an I/O failure | `outside the project: <path>` / `<path>: <io::Error Display>` |

## 17. Handed to other chains

| Chain | Receives |
|---|---|
| `cli-commands` | rendering `Plan` (the diff of shared files included) and doctor findings; running `app/apply` on the `Plan` it showed; writing a new tag into `agentstacks.toml`; shorthand expansion and its refusal; exit codes |
| `claude-extension` | `adapter.lua` and `translate.lua`; the Claude event and tool mapping; `match`/`agents` filtering in its config; message hooks; the settings.json layout; running the hook argv from the project directory, with its own quoting |
| OpenCode extension (`foundation/02`) | the TypeScript shim (§11.5) |
| `release-and-migration` | github.com/airsstack/agentstacks is private today: `git ls-remote` asks for a username (`probes/git-results.md` § G3 GitHub), so official packs cannot be fetched until it is public |

## 18. Non-goals

- A merge or run-command operation (D1).
- Any tool-specific code in the core.
- Writing `agentstacks.toml` (CLI).
- Network access in `doctor`.
- A lock file for the cache (D10), and cleaning up cache temps left by a killed process.
- Exact preservation of number spelling or of the inner layout of a shared JSON file (D9, §5).
- Pure-policy pack hooks (D14) until airsl#12 lands.
- Removing directories a delete leaves empty.
- Case-insensitive file systems: paths are compared by bytes, so `A.md` and `a.md` are two paths.
- A stable API: everything here is `api = 0`.
