---
status: approved
created: 2026-11-25
built-on: 81b57dbbc627
---

# Spec: The official OpenCode extension

This spec adds the official `opencode` tool extension, which renders the same packs into native
OpenCode V2 project config through the public `api = 0` surface the `claude` extension uses. It
also covers the three core changes OpenCode forces: `opencode` joins the official tools, the shared
hook runtime accepts a list of events from a translator, and a CI check runs the extension inside
the real OpenCode binary against a local mock model.

Proof convention:

- `probes/<file>` is a file in this chain directory holding raw probe output. A section is cited as
  `probes/<file> P<n>` and a line as `probes/<file>:<line>`.
- Every OpenCode run used the pinned binary `opencode v2.4.1` (`~/.opencode/bin/opencode --version`)
  through the mock-model harness in `probes/harness/` (`probes/BRIEF.md`), isolated from the user's
  OpenCode config.
- `probes/docs/<page>.mdx:<line>` is the OpenCode V2 documentation at tag `v2.4.1`, and
  `probes/src/<path>:<line>` is OpenCode source at the same tag (`anomalyco/opencode@6f05140`).
- `src/…` is `crates/agentstacks/src/…` at commit `8a96f78`.
- `claude §N` cites the approved spec `2026-11-20-cli-and-claude-extension/spec.md`; `core §N` cites
  `2026-11-20-core-plan-and-apply/spec.md`.

## 1. Settled decisions

| # | Decision | Why / proof |
|---|---|---|
| D1 | Target **OpenCode V2 only**, pinned to v2.4.1 for checks | author, 2026-11-24 (intent). V2 does not load V1 function plugins (`probes/plugin-tool-results.md` P1) |
| D2 | session.start is emulated on the first prompt a top-level session sends in this OpenCode process (source `startup`); agent.stop and session.end are skipped. Only documented, non-experimental APIs | author, 2026-11-25. No session-start, session-end or subagent-stop hook exists (`probes/plugin-session-results.md` P3-P5) |
| D3 | Every skill also gets a command `.opencode/commands/<pack>-<skill>.md` that loads it and passes the arguments | author, 2026-11-25. OpenCode has no `/skill args` and never substitutes `$ARGUMENTS` in a skill (`probes/skills-results.md` P6) |
| D4 | GPT models' `patch` tool is split per file in `translate.lua`; `runtime.lua` accepts a list of events (an `api = 0` change, backward compatible) | author, 2026-11-25. `gpt-` models get `patch` instead of edit/write (`probes/plugin-tool-results.md` P6) |
| D5 | `opencode-check`: a cargo-make task outside `dod` plus a CI job, running scripted sessions in the pinned real binary against a mock model | author, 2026-11-25 |
| D6 | The Linux build of OpenCode is proven by the first CI run of `opencode-check`, not before approval | author, 2026-11-25 (§10 risk K1) |
| D7 | One generated hook file per project, `.opencode/plugins/agentstacks.js`: plain JavaScript with no imports beyond Node built-ins, holding a data table of hook entries and a fixed forwarder | a `.js` file with `export default { id, setup }` loads with no `node_modules` (`probes/plugin-tool-results.md` P1a), also in a `"type": "commonjs"` project (`probes/fix-results.md` F3). Background, source only and not load-bearing: `.mjs` is not discovered (`probes/src/packages/core/src/plugin/source-directory.ts:23`) |
| D8 | Hook commands run as `/bin/sh -c "<quoted argv>"`, asynchronously, from the project root the shim computes from its own path | `node:child_process` spawn works in plugins (`probes/plugin-tool-results.md` P2); `new URL("../../", import.meta.url)` is the project root from any launch directory (`probes/followup-results.md` P1); sh expands var forms (`probes/shim-results.md:435`, `$TMPDIR` expanded) |
| D9 | The shim fails open: any fault of its own is caught; it throws only to deliver a block | a throw from a before hook blocks the call, even an accidental one (`probes/plugin-tool-results.md` P7) |
| D10 | Context for `tool.before` is held by call id and appended to that call's result in `execute.after`; `tool.after` context is appended to the result; prompt and session-start context is appended to the prompt text | appending `result.content` reaches the model (`probes/followup-results.md` P6); editing `prompt.text` reaches the model (`probes/plugin-session-results.md` P2); instruction entries pile up as `<system-update>` messages (`probes/followup-results.md` P4), so they are not used |
| D11 | `{{var project_dir}}` is `${AGENTSTACKS_PROJECT_DIR}`, which the shim sets in every shell the agent starts and in every hook process | no OpenCode variable holds the project dir and the shell starts in the launch dir (`probes/config-results.md` P2); `shell.create.before` can add env (`probes/followup-results.md` P2) |
| D12 | Agents: `mode: "subagent"`, tools as a V2 `permissions` list, model tier and effort left out (the subagent uses the parent's model) | `probes/agents-results.md` P1, P3, P4 |
| D13 | No shared config file: no `reads`, the user's `opencode.json` is never read or written | skills, agents, commands and plugins load with no `opencode.json` (`probes/config-results.md` P4) |
| D14 | Each skill's `SKILL.md` and its command are written; `invocation = "user"` writes `disable-model-invocation: true` (hidden from the model's list, still loadable by id) | `probes/skills-results.md` P2 |
| D15 | prompt.submit and session.start run only for top-level sessions; tool hooks run in every session, with the subagent named on the event | a subagent's first prompt is the parent's instruction, sent with the prefix `You are a subagent spawned by another session.`, not a user prompt (`probes/plugin-session-results.md` P7); `ctx.session.get` gives `parentID` (`probes/followup-results.md` P7, `probes/docs/build-plugins/server.mdx:777`) |
| D16 | The mock model and the check scenarios are Node scripts under `crates/agentstacks-cli/tests/opencode-check/`. They are test tooling, never installed into a project | the harness is the one proven here (`probes/harness/`); Node runs on the CI runner (§8.4) |
| D17 | When a hook process exits 127 (sh could not find airsl), the shim adds one notice to the next top-level prompt of this OpenCode process: `agentstacks: airsl is not on PATH, so the agentstacks Lua hooks are not running. Install airsl, then run agentstacks doctor.` It stays fail-open | the shim discards stderr (§4.5 **Process**), so without it a missing airsl is silent at run time. Every Lua hook command starts with `airsl` (ch06-02 §3); `sh` exits 127 for a missing command and airsl exits 0 under `--fail-open` even for a missing script (`probes/review-results.md` R1). Proven in OpenCode: one notice after two failed hooks, session goes on (R1, ms-01); no change with airsl present (ms-02); not repeated for a later prompt or a new session of the same process, and a failure first seen in a tool hook told at the next prompt (R4) |

## 2. Architecture

Approved by the author on 2026-11-25.

```text
extensions/opencode/                          (Lua under airsl, zero grants, no reads)
  extension.toml    manifest
  adapter.lua       the only agentstacks.* caller: values + plan → operations
  lib/values.lua      root, skill/command/agent names, var forms           ┐
  lib/frontmatter.lua SKILL.md, agent (with permissions) and command text  │ pure: plain data in,
  lib/hooks.lua       pack hook → plugin entry, or skip reason             │ text or tables out;
  lib/plugin.lua      agentstacks.js text: fixed shim + entry table        │ each has a <name>_test.lua
  lib/translate.lua   translate.lua text: payload ↔ neutral, patch split   ┘ (airsl test)
                         │ proposes
core (Rust)              ▼
  src/types/shorthand.rs   OFFICIAL_TOOLS gains "opencode"
  assets/runtime.lua       translate.event may return a list (§3.2)
  src/types/protocol/runtime.rs   its pinned hash, length and doc follow the asset (§3.2)
  tests/opencode.rs        the shipped extension on the real host (mirrors tests/claude.rs)
  tests/runtime.rs         list cases added
crates/agentstacks-cli/tests/opencode-check/   mock model, runner, scenarios, check pack (§8.3)
Makefile.toml  opencode-check task;  .github/workflows/ci.yml  opencode-check job
book: ch06-00, ch06-01, ch06-02, ch06-03 (new), SUMMARY.md, ch04, ch01;  README.md, CLAUDE.md
```

**Why each module exists.**

| Module | Its one job | Why it is separate |
|---|---|---|
| `adapter.lua` | turn the `values` and `plan` payloads into operations | the only file allowed to call `agentstacks.*` (ch06-00 "What you build") |
| `lib/values.lua` | where OpenCode puts things and what each `{{var}}` stands for | answers the `values` event alone |
| `lib/frontmatter.lua` | the three markdown file texts OpenCode reads (skill, agent, command) | one concept: OpenCode markdown files with frontmatter; all three share one reader in OpenCode (`probes/src/packages/core/src/config/plugin/command.ts:140`, `agent.ts:151`) |
| `lib/hooks.lua` | decide, per pack hook, its plugin entry or its skip reason | the hook rules; no text output |
| `lib/plugin.lua` | the text of `agentstacks.js` from a list of entries | one generated file |
| `lib/translate.lua` | the text of `<root>/translate.lua` | one generated file, with its own contract (ch06-02 §6) |

**Dependency direction.** `adapter → lib/*`. A `lib/` module never calls `agentstacks.*` and never
calls `require`; `adapter.lua` hands it data, as in `extensions/claude/` (ch06-00 step 4). The core
stays tool-agnostic: the Rust change is one constant in `types/`, a runtime asset that knows no
tool, and that asset's pinned hash and doc in `types/protocol/runtime.rs`. Each `lib/` file is named after one concept with one word (`frontmatter`, `plugin`, …).

**How logic is tested.** Every `lib/` rule is a unit test on plain values (`airsl test
extensions`, part of `dod` through `cargo make lua`). The adapter is covered end to end on the real
host by `tests/opencode.rs` (plan fixtures) and by `opencode-check` inside real OpenCode (§8).

**Rust guideline rules applied** (`claudestacks-guideline-rust:rust-guidelines`):

- `modularity.md` Rule 1: no Rust module gains a second responsibility; `shorthand.rs` keeps "the one
  place the official source is named" (`src/types/shorthand.rs:10-11`).
- `mod-rs-export-only.md`: no `mod.rs` or `lib.rs` changes; no new Rust module.
- `static-dispatch.md`: no trait objects; the change adds none.
- `unit-test-mandate.md`: `shorthand.rs` gains a colocated test for `opencode`; the runtime asset is
  tested by `tests/runtime.rs` (an asset, not a `src/*.rs` file).
- `doc-comment-discipline.md`: the `OFFICIAL_TOOLS` doc stays "The tool names the official repository
  holds under `extensions/`" with no count, so it cannot rot.
- `strong-types.md`, `workspace.md`, `strict-quality.md`: no new public parameter, crate or lint
  change; the gate is `cargo make dod`.
- Module layout rules (global): no `<concept>_<qualifier>` files; the Lua tests follow the
  `<module>_test.lua` convention `airsl test` requires (`extensions/claude/lib/`).

## 3. Core changes

### 3.1 `OFFICIAL_TOOLS`

`src/types/shorthand.rs:48` becomes `pub const OFFICIAL_TOOLS: [&str; 2] = ["claude", "opencode"];`.
`init --tool opencode` then expands to the official repository with subdir `extensions/opencode`,
as `claude` does (`src/types/shorthand.rs:205-210`). Today it refuses:
`--tool "opencode": not an official tool; pass a path, or add a git source to agentstacks.toml`
(`probes/agentstacks-results.md` P2). New test: `official_tool_opencode_expands_to_extensions_subdir`.

The official extension reaches users with the next `packs-v*` tag, which holds `extensions/` and
`packs/` together (`src/types/shorthand.rs:35`, `TAG_PREFIX = "packs-v"`).

### 3.2 `runtime.lua` accepts a list

`crates/agentstacks/assets/runtime.lua` becomes, byte for byte (proven on airsl 0.1.5,
`probes/runtime-results.md`):

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
local hook

-- Runs the hook on one event; a block from a hook that is not may_block is dropped with a line.
local function run(event)
  hook = hook or require(name)
  event.args = args
  local result = hook(event)
  if type(result) == "table" and result.block ~= nil and not may_block then
    airsstack.stdio.error("agentstacks: hook " .. name .. " tried to block but is not declared may_block; reason: "
      .. tostring(result.block) .. "\n")
    result.block = nil
  end
  return result
end

local result
if kind == "native" then
  result = run({ payload = payload })
else
  local event = translate.event(payload)
  if event == nil then
    return
  end
  if event.event ~= nil then
    result = run(event)
  else
    -- A list of events: the hook runs once per event, in order, until one blocks.
    local contexts = {}
    for _, each in ipairs(event) do
      local one = run(each)
      if type(one) == "table" then
        if one.context ~= nil then
          contexts[#contexts + 1] = tostring(one.context)
        end
        if one.block ~= nil then
          result = { block = one.block }
          break
        end
      end
    end
    if #contexts > 0 then
      result = result or {}
      result.context = table.concat(contexts, "\n")
    end
  end
end
local out = translate.result(result, payload)
if out ~= nil then
  airsstack.stdio.write(out)
end
```

**Rules** (all from `probes/runtime-results.md`):

| Case | Result |
|---|---|
| `M.event` returns a table with `event` set | one event; the result is exactly what the hook returned, as before |
| `M.event` returns a list (a table with no `event` key) | the hook runs once per element in order; each gets `args` |
| a list, `may_block` 1, an element blocks | the loop stops; the result is `{ block = <that block>, context = <contexts so far, joined with "\n"> }` (the blocking element's context included) |
| a list, `may_block` 0, an element blocks | the stderr line as today, the block dropped, the loop goes on |
| a list where no hook returns context or block, or an empty list | `M.result` gets `nil` |
| native kind | unchanged: one event `{ payload = <raw>, args = … }` |
| a context that is not a string | in a list it is joined after `tostring`; a single event passes it through unchanged |

The module is still required only when a hook runs, so a translator returning `nil` never loads it.
`tests/runtime.rs` keeps its cases and gains: `runtime_list_joins_contexts`,
`runtime_list_stops_at_the_first_block`, `runtime_list_drops_a_block_when_not_may_block`,
`runtime_empty_list_gives_nil`. The claude translator never returns a list, so the claude extension
behaves unchanged on the new runtime.

**Pinned hash and doc** (`src/types/protocol/runtime.rs`, `probes/runtime-hash-results.md`):

- `runtime_lua_hash_is_pinned` changes from
  `sha256:05ba59c1af4002b5a212f9eacd1cb74253f69cc79c9bf3a95d4dda312f89a282` and `914` to
  `sha256:1d9e23f6d26d49ae14e28d0425d5d31fd7be5dae7e6c8fa33a3153ba8acde459` and `1715`.
- The `RUNTIME_LUA` doc paragraph becomes: "The runtime loads the extension's `translate` module and
  reads the hook module's name, the `may_block` flag and the hook's kind from the command line's
  first three arguments; the rest are the hook's own. For a neutral hook it builds a neutral event,
  or a list of neutral events, from the payload the tool sends and stays silent when the translator
  has none; for a native hook the event holds the raw payload. It runs the hook module once per
  event; for a list it joins the contexts with a newline and stops at the first block. A `block` from
  a hook not declared `may_block` is dropped with one line on standard error, and the translator's
  `result` is printed; it receives the result and the raw payload, for neutral and native hooks
  alike."

**Existing projects.** `runtime.lua` is a core write, so every project with a lua hook, a Claude one
included, shows `update <root>/runtime.lua` on its first `plan` after upgrading agentstacks, and
`apply` rewrites it. The changelog entry says so (§8.5).

### 3.3 Docs that change with the API

- `book/src/ch06-02-hooks.md` §4: the new runtime text and the rules table; §6: "`M.event(payload)`
  returns the neutral event, a list of neutral events, or `nil`"; §9: OpenCode hooks are JavaScript
  plugins (`.js`), not TypeScript, and the shim is the extension's ordinary `write`.
- `book/src/ch06-01-extensions.md` §14 last bullet: "OpenCode plugins are JavaScript or TypeScript;
  the opencode extension writes a JavaScript shim".
- `book/src/ch06-00-authoring.md` step 5 point 4: a translator may return a list. Rule in
  `CLAUDE.md`: these three change together.

## 4. The opencode extension

### 4.1 `extension.toml` and `values`

```toml
[extension]
name        = "opencode"
version     = "0.1.0"
api         = 0
description = "Renders packs into OpenCode V2 project config"
entry       = "adapter.lua"
```

No `reads` (D13). This manifest loads and plans (`probes/manifest-results.md`).

**`values` result** (`lib/values`):

- `root = ".opencode/agentstacks"`.
- `skills["<p>/<s>"] = { invoke = "/<p>-<s>", dir = ".opencode/skills/<p>-<s>" }`. A skill's id is its
  directory name (`probes/skills-results.md` P1); `/<p>-<s>` is its command (D3, §4.2).
- `agents["<p>/<a>"] = "<p>-<a>"`. An agent's id is its file name without `.md`
  (`probes/agents-results.md` P1).
- `vars` holds only the names the payload lists (core §4.4):

| Var | Value | Proof |
|---|---|---|
| `project_dir` | `${AGENTSTACKS_PROJECT_DIR}` | set by the shim in the agent's shell (`probes/shim-results.md:439`) and in hook processes (§4.5) |
| `session_id` | `${OPENCODE_SESSION_ID}` | OpenCode sets it in the shell tool to the session id (`probes/config-results.md` P2); never substituted in text (P3), so the model uses it in shell commands |
| `arguments` | `<arguments>` | the command fills it (§4.2); OpenCode never substitutes `$ARGUMENTS` in a skill (`probes/config-results.md` P3) |
| `tmpdir` | `${TMPDIR:-/tmp}` | expanded by sh in hook commands (`probes/shim-results.md:435`) and by the agent's shell (`$SHELL`, `/bin/zsh` on macOS, `probes/config-results.md` P2) |
| `home` | `$HOME` | same |
| `airsstack_home` | `${AIRSSTACK_HOME:-$HOME/.airsstack}` | claude §9.1, same form |

The packs use five of these names, all but `home` (`probes/agentstacks-results.md` P5); `home` is in
the table for packs that use it, as in claude §9.1.

### 4.2 Frontmatter (`lib/frontmatter`)

Shape, values, key order and native keys follow claude §9.2 exactly: `---\n`, one `key: <value>\n`
per key with `airsstack.json.encode(value)`, `---\n`, then the body. Generated keys come in the
order below; `meta.native` (only this tool's entry) replaces a generated key of the same name, and
other native keys follow sorted by name. A native key that is not `^[A-Za-z0-9_-]+$` skips the item
(§4.6). A value that is an empty table, generated or native, is written as `[]`: `encode` writes an
empty table as `{}` (`probes/review-results.md` R2), and Lua cannot tell an empty list from an empty
map, so the list form is chosen (`permissions: []` is the V2 form OpenCode reads,
`probes/followup-results.md` P3). An empty table nested inside a value is written as `encode` gives
it. OpenCode decodes JSON-quoted scalars, escapes and flow lists in this frontmatter
(`probes/skills-results.md` P3, `probes/agents-results.md` P2).

| Skill (`SKILL.md`) | Key | Proof |
|---|---|---|
| name | `name` = `<p>-<s>` (a label; the id is the directory) | `probes/skills-results.md` P1 |
| `description` | `description` | same |
| `invocation: user` | `disable-model-invocation: true`; `any` writes nothing | P2 |
| `argument_hint` | not written: OpenCode skills and commands have no hint field | skill fields `probes/docs/skills.mdx:138-141`; command fields `probes/docs/commands.mdx:73-78` |

| Agent (`.opencode/agents/<p>-<a>.md`) | Key | Proof |
|---|---|---|
| name | not written: the id comes from the file name; a frontmatter `name` is ignored | `probes/agents-results.md` P1 |
| `description` | `description` | P1 |
| (always) | `mode: "subagent"` | callable by the `subagent` tool, hidden from default selection (P1) |
| `tools` | `permissions`: the deny-all rule, then one allow rule per mapped tool in written order, duplicates dropped, encoded as one list with sorted keys: `[{"action":"*","effect":"deny","resource":"*"},{"action":"read","effect":"allow","resource":"*"}]` for `["read"]`. Map: read→`read`, write→`edit`, edit→`edit`, grep→`grep`, glob→`glob`, shell→`shell`, skill→`skill` | P3 (a deny-all plus read and grep left exactly `grep, read, skill`); the same list in `encode`'s sorted, unspaced bytes gave the same (`probes/review-results.md` R2); write and edit share the `edit` action |
| no `tools` | `permissions: []` (the empty-table rule above) | keeps the file V2 and keeps the default rules (`probes/followup-results.md` P3) |
| `tools = []` (empty, kept apart from absent, `src/types/frontmatter/agent/meta.rs:396`) | `permissions` holds only the deny-all rule | the same rule with no allow; the agent gets no tool it can use |
| `model`, `effort` | not written; the subagent uses the parent's model | OpenCode has no tier; effort is a variant of one pinned model (`probes/agents-results.md` P4) |

`permissions` is always written, because a file without it is read as V1 and a native key in V1 form
could then drop the whole agent (`probes/agents-results.md` P2).

**Command** (`.opencode/commands/<p>-<s>.md`): frontmatter `description` = the skill's description,
no native keys; then this body, with `<p>-<s>` filled in. For `core/discuss`:

```text
---
description: "<the skill's description, JSON-encoded>"
---
Load the skill `core-discuss` with the skill tool and follow it.
Wherever it says `<arguments>`, use: $ARGUMENTS
```

The file ends with `\n` after the last line. A command's `$ARGUMENTS` becomes the raw argument string, and its body reaches the model as a user
prompt (`probes/config-results.md` P1b). A `skill` call by id returns the body, `Base directory for
this skill: <abs dir>` and the sibling file paths, also for a `disable-model-invocation` skill
(`probes/skills-results.md` P1, P2, P7).

### 4.3 Files

| Item | Operation |
|---|---|
| skill `SKILL.md` | `write{ path = <dir>.."/SKILL.md", content = frontmatter .. text, covers = { id } }` |
| the skill's command | `write{ path = ".opencode/commands/<p>-<s>.md", content = <§4.2>, covers = { <SKILL.md id> } }`. A skipped skill (§4.6) gets no command: a write covering a file of a skipped group is refused (ch06-01 §8) |
| other skill file | `copy{ path = <dir>.."/"..rel, file = id }` |
| agent | `write{ path = ".opencode/agents/<p>-<a>.md", content = frontmatter .. text, covers = { id } }` |
| native file | `copy{ path = ".opencode/"..rel, file = id }` |
| hook, native hook | covered by the `agentstacks.js` write, or skipped (§4.6) |
| `.opencode/plugins/agentstacks.js` | always written, `covers` = every hook id that became an entry, sorted; with no entry it still sets `AGENTSTACKS_PROJECT_DIR` (§4.5) |
| `<root>/translate.lua` | written when at least one lua hook (neutral or native) became an entry, with the text of §4.7 |

Two writes covering the same item are accepted (ch06-01 §8). Two packs with the same skill or agent
name propose one path twice and the core refuses it (claude §9.3); the extension adds no check.

### 4.4 Hooks (`lib/hooks`)

Each payload hook (packs in payload order; within a pack, `hooks` by index, then native hooks by
index) becomes one entry or one skip. An entry is a table with these keys:

| Key | Value |
|---|---|
| `on` | the event the shim dispatches on (below) |
| `tools` | the OpenCode tool names it runs for (tool events only); absent = no filter |
| `match` | the neutral `match` list, for the translator's patch split (neutral tool events only); absent = none |
| `command` | lua hooks: each argv element in double quotes with `\`, `"` and `` ` `` escaped by `\`, joined by single spaces (claude §9.4 quoting, without `cd` and without a marker) |
| `text` | message hooks: the text |

| Neutral event | `on` | `tools` |
|---|---|---|
| `tool.before` / `tool.after` | same name | from `match`: read→`read`; edit→`edit`, `patch`; write→`write`, `patch`; duplicates dropped, first-seen order. No `match` → `read`, `edit`, `write`, `patch` |
| `prompt.submit` | `prompt.submit` | — |
| `session.start` | `session.start` | — (`match` decides render or skip, §4.6) |
| `session.end`, `agent.stop` | skipped (§4.6) | — |

| Native event (as written in `native/opencode/hooks.toml`) | `on` | `tools` |
|---|---|---|
| `tool.execute.before`, `tool.execute.after` | same name | the native `match` as written (OpenCode tool names); none or empty → absent (every tool) |
| `session.prompt` | `session.prompt` | — (a non-empty native `match` here is skipped, §4.6) |
| any other | skipped (§4.6) | — |

A hook is native when it comes from the pack's `native.hooks` list, never by its event name: the
adapter passes that to `lib/hooks`, since a native hook's event is free text and can equal a neutral
name (`src/types/hooks/native/hook.rs:1-19`). The core refuses an empty neutral `match`
(`src/types/hooks/neutral.rs:184`) but passes an empty native one; an empty native `match` counts as
none, as the claude extension renders it `"matcher": ""` (`probes/review-results.md` R3), which
Claude Code reads as every tool (`2026-11-20-cli-and-claude-extension/probes/docs/hooks.md:281`).
So an entry's `tools` and `match`, when present, are never empty lists (`encode` would write `{}`, R2).

The packs' `agents` key is allowed only on `agent.stop` (`src/types/hooks/neutral.rs:212`), which is
skipped, so no entry needs an agent filter.

### 4.5 `agentstacks.js` (`lib/plugin`)

The text is the header and table below, then one empty line, then the fixed body, byte for byte.
The table's last line is `];`; the file ends with `};` and a newline. Each entry is
`airsstack.json.encode(entry)` (an object with its keys sorted, as the claude fixture
`crates/agentstacks/tests/fixtures/claude/full/expected/.claude/settings.json` shows for
`extensions/claude/lib/hooks.lua:124`), one per line, joined by `,\n  `; with no entries the table
line is `const HOOKS = [];`. This shim, with a hand-built table, ran in OpenCode v2.4.1 and gave
every behaviour of §4.5 and §4.7 (`probes/shim-results.md`; with stderr ignored and in a CommonJS
project, `probes/fix-results.md` F3; the airsl notice, this exact fixed body,
`probes/review-results.md` R1).

```js
// agentstacks hook shim for OpenCode, api 0. Written by `agentstacks apply`; edits are drift.
// It runs the hook commands in HOOKS and forwards each event and result. All hook logic is Lua.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const HOOKS = [
  <entry>,
  <entry>
];
```

Fixed body:

```js
const ROOT = fileURLToPath(new URL("../../", import.meta.url)).replace(/\/+$/, "");
const TIMEOUT_MS = 60000;
const AIRSL_MISSING =
  "agentstacks: airsl is not on PATH, so the agentstacks Lua hooks are not running. Install airsl, then run agentstacks doctor.";

// Set when sh could not find a hook command's program (exit 127): every Lua hook runs airsl.
let airslMissing = false;

// Runs one hook command under sh from the project root, with `payload` as JSON on stdin.
// Resolves to its stdout, or null when it cannot run or runs past the timeout.
function run(command, payload) {
  return new Promise((resolve) => {
    let out = "";
    let settled = false;
    const finish = (value) => {
      if (!settled) {
        settled = true;
        resolve(value);
      }
    };
    let child;
    try {
      child = spawn("/bin/sh", ["-c", command], {
        cwd: ROOT,
        env: { ...process.env, AGENTSTACKS_PROJECT_DIR: ROOT },
        stdio: ["pipe", "pipe", "ignore"],
      });
    } catch {
      finish(null);
      return;
    }
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      finish(null);
    }, TIMEOUT_MS);
    child.on("error", () => {
      clearTimeout(timer);
      finish(null);
    });
    child.stdout.on("data", (chunk) => {
      out += chunk;
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      if (code === 127) airslMissing = true;
      finish(out);
    });
    child.stdin.on("error", () => {});
    child.stdin.end(JSON.stringify(payload));
  });
}

function parse(text) {
  try {
    const value = JSON.parse(text);
    return value !== null && typeof value === "object" ? value : null;
  } catch {
    return null;
  }
}

// Runs, in table order, every entry whose `on` is in `events` and whose `tools` holds `tool`.
// Returns the joined contexts and the first block.
async function dispatch(events, tool, payload) {
  const contexts = [];
  for (const entry of HOOKS) {
    if (!events.includes(entry.on)) continue;
    if (entry.tools && !entry.tools.includes(tool)) continue;
    if (typeof entry.text === "string") {
      contexts.push(entry.text);
      continue;
    }
    const result = parse((await run(entry.command, { ...payload, hook: entry.on, match: entry.match ?? null })) ?? "");
    if (result === null) continue;
    if (typeof result.context === "string") contexts.push(result.context);
    if (typeof result.block === "string") return { context: contexts.join("\n"), block: result.block };
  }
  return { context: contexts.join("\n"), block: null };
}

function joined(block, context) {
  return context ? block + "\n" + context : block;
}

export default {
  id: "agentstacks",
  setup: async (ctx) => {
    const parents = new Map();
    const started = new Set();
    let told = false;
    const held = new Map();

    async function parentOf(sessionID) {
      if (!parents.has(sessionID)) {
        let parent = null;
        try {
          const info = await ctx.session.get({ sessionID });
          parent = typeof info?.parentID === "string" ? info.parentID : null;
        } catch {}
        parents.set(sessionID, parent);
      }
      return parents.get(sessionID);
    }

    async function base(sessionID) {
      return { session_id: sessionID, parent_id: await parentOf(sessionID), cwd: ctx.location.directory };
    }

    await ctx.shell.hook("create.before", (e) => {
      try {
        e.env.AGENTSTACKS_PROJECT_DIR = ROOT;
      } catch {}
    });

    await ctx.tool.hook("execute.before", async (e) => {
      let block = null;
      try {
        const payload = { ...(await base(e.sessionID)), agent: e.agent, tool: e.tool, input: e.input };
        const r = await dispatch(["tool.before", "tool.execute.before"], e.tool, payload);
        if (r.block !== null) block = joined(r.block, r.context);
        else if (r.context) held.set(e.id, r.context);
      } catch {}
      if (block !== null) throw new Error(block);
    });

    await ctx.tool.hook("execute.after", async (e) => {
      let block = null;
      try {
        const before = held.get(e.id);
        held.delete(e.id);
        if (e.status !== "completed") return;
        const payload = { ...(await base(e.sessionID)), agent: e.agent, tool: e.tool, input: e.input };
        const r = await dispatch(["tool.after", "tool.execute.after"], e.tool, payload);
        const context = [before, r.context].filter(Boolean).join("\n");
        if (r.block !== null) block = joined(r.block, context);
        else if (context) {
          e.result = { ...e.result, content: [...(e.result.content ?? []), { type: "text", text: context }] };
        }
      } catch {}
      if (block !== null) throw new Error(block);
    });

    await ctx.session.hook("prompt", async (e) => {
      try {
        const payload = await base(e.sessionID);
        if (payload.parent_id !== null) return;
        const parts = [];
        if (!started.has(e.sessionID)) {
          started.add(e.sessionID);
          const s = await dispatch(["session.start"], null, { ...payload, source: "startup" });
          if (s.context) parts.push(s.context);
          if (s.block !== null) parts.push(s.block);
        }
        const r = await dispatch(["prompt.submit", "session.prompt"], null, { ...payload, prompt: e.prompt.text });
        if (r.context) parts.push(r.context);
        if (airslMissing && !told) {
          told = true;
          parts.push(AIRSL_MISSING);
        }
        if (parts.length > 0) e.prompt.text = e.prompt.text + "\n\n" + parts.join("\n");
      } catch {}
    });
  },
};
```

**What each OpenCode hook does** (API, all documented: `ctx.session.hook("prompt")`
`probes/docs/build-plugins/server.mdx:1397`, `ctx.shell.hook("create.before")` `:1785`,
`ctx.tool.hook("execute.before"/"execute.after")` `:1812`, `:1820`, `ctx.session.get` `:777`; event fields
`probes/plugin-tool-results.md` P3, P5, `probes/plugin-session-results.md` P2, P7):

| OpenCode hook | Entries run | Delivery | Proof |
|---|---|---|---|
| `shell.create.before` | none | sets `AGENTSTACKS_PROJECT_DIR` = project root in the agent's shell, main and subagent | `probes/followup-results.md` P2; `probes/shim-results.md:439` |
| `tool.execute.before` | `tool.before`, `tool.execute.before` whose `tools` holds the tool | block → `throw new Error(<block>\n<context>)`: the tool does not run and the model sees `{"error":{"type":"unknown","message":"<text>"},"content":[]}`. Context → held under the call id | `probes/plugin-tool-results.md` P4; `probes/shim-results.md:433`, `:445` |
| `tool.execute.after` (status `completed` only) | `tool.after`, `tool.execute.after` | held and new contexts joined with `\n`, appended as one `{type:"text"}` item to `result.content`; block → throw (the tool already ran, the model sees the error with the text) | `probes/followup-results.md` P5, P6; `probes/shim-results.md:435`, `:437` |
| `session.prompt` (top-level sessions only) | first prompt of a session id in this process: `session.start` (source `startup`); every prompt: `prompt.submit`, `session.prompt` | contexts (and a session.start block, as context), then the airsl notice once (D17), appended to `prompt.text` after a blank line, joined with `\n` | `probes/plugin-session-results.md` P2; `probes/shim-results.md:431`; `probes/review-results.md` R1 |

**Payload on stdin** (the "raw payload" the translator and native hooks see):

| Field | When | Value |
|---|---|---|
| `hook` | always | the entry's `on` |
| `session_id`, `cwd` | always | `sessionID`; `ctx.location.directory`, the launch directory, where a relative tool path resolves (`probes/resolve-results.md`) |
| `parent_id` | always | the parent session id, or `null` for a top-level session |
| `match` | always | the entry's `match`, or `null` |
| `agent`, `tool`, `input` | tool hooks | `e.agent` (the agent id), `e.tool`, `e.input` as the model sent it (`path` for read/edit/write, `patchText` for patch) |
| `source` | session.start | `"startup"` |
| `prompt` | prompt hooks | `e.prompt.text` before any context is added |

**Process.** `/bin/sh -c <command>`, cwd = the project root, env = OpenCode's env plus
`AGENTSTACKS_PROJECT_DIR`, stderr discarded (so nothing can reach OpenCode's terminal), stdout read
as UTF-8 text and parsed as JSON. Entries run one after another in table order; the first block
stops the dispatch.

**Held context.** `held` maps a tool call id to its before-hook context. An entry is removed when
`execute.after` fires for that call, whatever its status. A call that fails after the before hook
ran (a permission denial, an unavailable tool) does not show the held context
(`probes/fix-results.md` F1).

**Fault rules (D9).** Each of these leaves the event as it was and the session going: a spawn
failure, a process running past 60 s (killed), stdout that is not a JSON object, a non-string
`context` or `block`, `ctx.session.get` failing (the session is treated as top-level), and any
exception inside the shim. A plugin that fails to load at all is shown only in OpenCode's log file
(`probes/plugin-tool-results.md` P1d); §8.3 S1 catches that before release.

**Missing airsl (D17).** A hook process that exits 127 sets `airslMissing` for this OpenCode process.
The next top-level prompt hook appends `AIRSL_MISSING` after any other context and sets `told`; a
later prompt, in the same session or a new one, served by the same process does not repeat it, and a
new process tells it again if airsl is still missing. A failure first seen in a tool hook is told at
the next prompt (`probes/review-results.md` R1, R4). One process serves many sessions: `opencode run`
without `--standalone` uses a background `opencode serve --service` that outlives the CLI
(`probes/config-results.md:81-87`), so there the notice comes once per service start; it stays in the
session history as part of that user message (§9). Nothing else changes: the hook gave no output, so
no context and no block, and so every `may_block` guard is off while airsl is missing (R4, ms-03).
Only exit 127 is told: an airsl on `PATH` without an execute bit makes sh exit 126 and gives no
notice (R5); `plan` and `doctor` count such a file as missing (`book/src/ch05-00-plan-and-apply.md:369`).

### 4.6 Skips

| # | Case | Applies to | Item | Reason (exact) |
|---|---|---|---|---|
| 1 | a hook on `session.end` (message or lua) | neutral hooks | the hook | `opencode has no hook when a session ends` |
| 2 | a hook on `agent.stop` (message or lua) | neutral hooks | the hook | `opencode has no hook when a subagent stops` |
| 3 | a hook on `session.start` whose `match` does not hold `startup` | neutral hooks | the hook | `opencode runs session.start hooks only for a new session (source "startup"), which this hook does not match` |
| 4 | a `may_block` lua hook on `prompt.submit` (neutral), or on `session.prompt` (native); a message hook on either renders | both, by those event names | the hook | `opencode cannot block a prompt` |
| 5 | a hook on `session.prompt` with a non-empty `match` | native hooks | the hook | `opencode has no matcher for session.prompt` |
| 6 | a hook on any event but `tool.execute.before`, `tool.execute.after` and `session.prompt` | native hooks | the hook | `opencode has no native hook event "<event>" (use tool.execute.before, tool.execute.after or session.prompt)` |
| 7 | a hook whose text or argv holds `${OPENCODE_SESSION_ID}` | both | the hook | `opencode gives hooks the session id in the event (event.session_id), not as {{var session_id}}` |
| 8 | a lua argv element with a `$` left after removing every var form present in `values.vars`, longest first (claude §9.4) | both | the hook | `opencode runs hook commands through sh, which would expand the literal $ in argument "<arg>"` |
| 9 | a native key that is not a plain word | skills, agents | the skill (group) or agent | `native.opencode key "<key>" cannot be written as frontmatter` |

A rule applies only to the hooks its "Applies to" names (native = from `native.hooks`, §4.4). So a
native hook is judged by rules 4 (on `session.prompt` only) to 8, and one whose event text is
`session.end`, `agent.stop`, `session.start` or `prompt.submit` gets rule 6, naming its event, even
when it is a `may_block` lua hook; a neutral hook never reaches rules 5-6.
Rules are checked in table order; the first that applies gives the reason. Every other item has an
OpenCode form (§4.2-§4.4), so nothing else is skipped. Items the core already skipped for a missing
value never reach the extension (ch06-01 §6).

With the five official packs, these hooks are skipped: core's session.start `compact` hook (rule 3),
its session.end message, its two agent.stop hooks, and the agent.stop hooks of sdlc and journal. The
core's session.start message (`match = ["startup", "resume", "clear"]`), its style hook (no
`match`), and the session.start hooks of sdlc (no `match`) and journal (`["startup", "resume",
"clear"]`) are rendered (`packs/*/pack.toml`).

### 4.7 `translate.lua` (`lib/translate`)

The text, byte for byte (ran in `probes/shim-results.md`):

```lua
-- agentstacks translator for opencode, api 0.
local M = {}

local TOOLS = { read = "read", edit = "edit", write = "write" }

local function str(v)
  if type(v) == "string" then
    return v
  end
  return nil
end

local function trim(s)
  return (s:gsub("^%s+", ""):gsub("%s+$", ""))
end

-- True when `tool` is in the hook's neutral `match` list, or the hook has none.
local function allowed(match, tool)
  if type(match) ~= "table" then
    return true
  end
  for _, word in ipairs(match) do
    if word == tool then
      return true
    end
  end
  return false
end

-- The files a patch writes, in patch order: an Add File is a write, an Update File an edit (at its
-- Move to path when it has one). A Delete File leaves nothing to check.
local function patch_files(text)
  local files = {}
  for line in (text .. "\n"):gmatch("([^\n]*)\n") do
    line = line:gsub("\r$", "")
    local add = line:match("^%*%*%* Add File: (.+)$")
    local update = line:match("^%*%*%* Update File: (.+)$")
    local move = line:match("^%*%*%* Move to: (.+)$")
    if add then
      files[#files + 1] = { tool = "write", path = trim(add) }
    elseif update then
      files[#files + 1] = { tool = "edit", path = trim(update) }
    elseif move and #files > 0 and files[#files].tool == "edit" then
      files[#files].path = trim(move)
    end
  end
  return files
end

function M.event(p)
  local e = { event = p.hook, session_id = str(p.session_id), cwd = str(p.cwd) }
  if p.hook == "session.start" then
    e.source = "startup"
    return e
  elseif p.hook == "prompt.submit" then
    e.prompt = str(p.prompt)
    return e
  elseif p.hook ~= "tool.before" and p.hook ~= "tool.after" then
    return nil
  end
  if str(p.parent_id) ~= nil then
    e.agent = str(p.agent)
    e.agent_id = e.session_id
  end
  local input = type(p.input) == "table" and p.input or {}
  if p.tool == "patch" then
    local list = {}
    for _, file in ipairs(patch_files(str(input.patchText) or "")) do
      if allowed(p.match, file.tool) then
        local one = {}
        for k, v in pairs(e) do
          one[k] = v
        end
        one.tool, one.path = file.tool, file.path
        list[#list + 1] = one
      end
    end
    if #list == 0 then
      return nil
    end
    return list
  end
  local tool = TOOLS[p.tool]
  if tool == nil or not allowed(p.match, tool) then
    return nil
  end
  e.tool, e.path = tool, str(input.path)
  return e
end

function M.result(r, p)
  if type(r) ~= "table" then
    return nil
  end
  local ctx, blk = r.context, r.block
  if p.hook == "session.start" and blk ~= nil then
    ctx = ctx ~= nil and (tostring(ctx) .. "\n" .. tostring(blk)) or blk
    blk = nil
  end
  if ctx == nil and blk == nil then
    return nil
  end
  local out = {}
  if ctx ~= nil then
    out.context = tostring(ctx)
  end
  if blk ~= nil then
    out.block = tostring(blk)
  end
  return airsstack.json.encode(out) .. "\n"
end

return M
```

**`M.event(p)`** (neutral event, ch06-02 §5):

| `p.hook` | Event | Fields |
|---|---|---|
| `session.start` | `session.start` | `source = "startup"` |
| `prompt.submit` | `prompt.submit` | `prompt` |
| `tool.before` / `tool.after`, tool `read`/`edit`/`write` in `match` (or no `match`) | same | `tool`, `path` = `input.path` as sent (relative to `cwd` or absolute) |
| same, tool `patch` | a **list**, one event per file in patch order, keeping only tools in `match`: `*** Add File: p` → write `p`; `*** Update File: p` → edit `p`, or edit `q` when `*** Move to: q` follows; `*** Delete File:` → none. Empty → `nil` | patch grammar `probes/src/packages/util/src/patch.ts:71-105`; Add/Update split `probes/shim-results.md:445-447`; Move blocked as an edit of `q`, Move allowed with context on `q`, delete-only patch runs no hook (`probes/fix-results.md` F2) |
| any other tool or event | `nil` | — |

Always `session_id` and `cwd`. On tool events in a subagent session (`parent_id` set): `agent` = the
agent id and `agent_id` = the child session id; otherwise both `nil` (the main agent).

A header line counts only when it starts at column 0 with `*** `. OpenCode also applies a header
with leading spaces (`  *** Add File: d.txt` wrote `d.txt`, `probes/fix-results.md` F2; it trims,
`patch.ts:62`); such a file is not seen by hooks (§9).

**`M.result(r, p)`**: `nil` when `r` is not a table or holds neither field; otherwise
`{"context":…,"block":…}` + `\n` with the fields present. On `session.start`, a block is joined
into the context (`ctx\nblk`), as claude §9.7 does for `SessionStart`. A `prompt.submit` block never
arrives: a `may_block` prompt hook is skipped (§4.6) and the runtime drops any other block.

## 5. Errors: locations and owners

The extension adds no refusal of its own: every unsupported item is a skip (§4.6), and the
operations go through the core's existing checks (ch06-01 §9). Owners:

| Check | Owner |
|---|---|
| `--tool opencode` accepted | `types/shorthand` (§3.1) |
| list results in the runtime | `assets/runtime.lua` (§3.2) |
| skip decisions | `lib/hooks`, `lib/frontmatter` (native key) |
| path clashes, coverage, `translate.lua` presence | core (ch06-01 §8-§9) |
| hook faults at run time | the shim, fail-open (§4.5) |

## 6. Edge cases

| Case | Result | Owner |
|---|---|---|
| a project lists `claude` and `opencode` | both apply; each tool has its own root, runtime and root files; no path clash (`probes/agentstacks-results.md` P1). OpenCode also reads `.claude/skills/`; a skill in both lists once and the `.opencode` copy wins (`probes/skills-results.md` P5) | core, OpenCode |
| no pack has a hook | `agentstacks.js` with `const HOOKS = [];`, `covers` empty, no `translate.lua` | `lib/plugin`, adapter |
| every lua hook skipped | the core still writes `runtime.lua` (ch06-01 §7); no `translate.lua` | core, adapter |
| OpenCode started in a subdirectory | `.opencode/` is still found (`probes/config-results.md` P5); hooks run from the project root; `cwd` in the payload is the subdirectory, where relative paths resolve (`probes/resolve-results.md`) | shim |
| a GPT model's `patch` touching one blocked file and one allowed file | the whole patch is blocked (one tool call) | runtime, shim |
| a `patch` that only deletes files | no event, the hook does not run (`probes/fix-results.md` F2) | `translate.lua` |
| a `patch` with `*** Move to: q` | one edit event for `q`; a guard on `q` blocks the whole patch (`probes/fix-results.md` F2) | `translate.lua` |
| a project whose `package.json` says `"type": "commonjs"` | the shim loads and works (`probes/fix-results.md` F3) | — |
| both tools listed: skills only Claude has (`native/claude` ones such as `core-statusline`, or skills the opencode extension skipped) | OpenCode lists them from `.claude/skills/` with Claude's var forms (`probes/skills-results.md` P5). A limit (§9); the book names `OPENCODE_DISABLE_CLAUDE_CODE=1`, which stops OpenCode reading `.claude/skills/` (P5) | documented |
| a session resumed with `-c` in a new process | its first prompt there runs `session.start` again, as `startup` (OpenCode does not tell resume from new, `probes/plugin-session-results.md` P3) | shim (documented) |
| compaction | no `session.start`; hooks matching only `compact` are skipped at plan time | `lib/hooks` |
| a subagent's first prompt | no `session.start` and no `prompt.submit` (D15) | shim |
| the model calls a `disable-model-invocation` skill by id | it loads (`probes/skills-results.md` P2); documented in the book as "hidden, not refused" | documented |
| a skill whose body uses no `<arguments>` | its command still carries the `Wherever it says` line; harmless | `lib/frontmatter` |
| a message hook with a newline, `"` or `\` | JSON-encoded in the table, so the shim gets the exact text | `lib/plugin` |
| a hook process prints nothing, or non-JSON | no context, no block | shim |
| a hook process hangs | killed after 60 s, no context, no block | shim |
| airsl missing from `PATH` | `require_binary` refuses at plan when a locked pack needs airsl (ch06-02 §10); if it disappears later, sh exits 127, each Lua hook gives nothing (so no `may_block` guard blocks), and the model is told once per OpenCode process at the next top-level prompt (D17, `probes/review-results.md` R4); `doctor` reports it. An airsl without an execute bit gives exit 126 and no notice (R5) | core, shim (documented) |
| a native frontmatter value that is an empty table (`permissions = []`) | written `[]` (§4.2) | `lib/frontmatter` |
| a native hook with `match = []` | the same as no `match`: every tool, and on `session.prompt` it renders (`probes/review-results.md` R3) | `lib/hooks` |
| the user edits `agentstacks.js` | drift: `doctor` reports it; `apply` restores it (core) | core |
| the user removes the `opencode` tool | the core deletes every locked file of the tool, the shim included | core |
| a nested agentstacks project inside another | OpenCode loads `.opencode/` from every parent (`probes/config-results.md` P5), so two shims with the same id would load. Not supported; the book says so | documented |
| OpenCode's permission check (no `--auto`, or a deny rule) | `execute.before` runs before it: a `may_block` hook can block a call the user would have been asked about, and a call that then fails drops its held context (`probes/fix-results.md` F1). The shim never reads or changes permissions | shim (documented) |

## 7. Not carried over from Claude

| Claude feature | OpenCode | Visible as |
|---|---|---|
| `SessionEnd`, `SubagentStop` hooks | none | skips (§4.6) |
| `SessionStart` sources `resume`, `clear`, `compact` | only the emulated `startup` | skip of `compact`-only hooks; book note |
| block on `UserPromptSubmit` | a prompt-hook throw only shows `UnexpectedStatus: 500` (`probes/plugin-session-results.md` P2) | skip `opencode cannot block a prompt` |
| model tier, effort | none | book table (§8.5) |
| `argument-hint` | none | book table |
| `/skill args` | the generated command | — |

## 8. Testing

### 8.1 Unit (`airsl test extensions`, in `dod`)

One `<module>_test.lua` per `lib/` module, plain values in, asserted out:

- `values_test`: root; skill invoke and dir; agent names; each var; a var not in the payload absent.
- `frontmatter_test`: skill with and without `invocation = user`; `argument_hint` not written; agent
  with no tools (the literal `permissions: []`), with `tools = []` (deny-all only, byte exact), with
  `["read","write","edit","grep"]` (deny-all, then `read`, `edit`, `grep`, in `encode`'s sorted
  bytes); native key replaces a generated one; a native `permissions = []` on an agent and an empty
  native list on a skill both written `[]`; bad native key → `nil` plus the §4.6 reason;
  command text for `core-discuss`, byte exact.
- `hooks_test`: each row of §4.4 and every skip row of §4.6 with its exact reason, including: a
  native `session.prompt` message hook and a native `session.prompt` lua hook without `may_block`
  (both render, no `tools`), and one with `match = []` (renders); a native `tool.execute.before`
  hook with no `match` and one with `match = []` (no `tools`); a neutral `session.start` hook with no
  `match` (renders); native hooks on `session.end`, `agent.stop`, `session.start` and a `may_block`
  lua one on `prompt.submit` (each rule 6, naming its event). Quoting of an
  argv element holding a `"`, a `\` and a backtick (claude §9.4 example); the `$` rule with `^a$`
  (skip) and `${TMPDIR:-/tmp}/x` (render).
- `plugin_test`: header and body exact; empty table → `const HOOKS = [];`; two entries joined with
  `,\n  `; a text with `"` and a newline encoded.
- `translate_test`: the text defines `M.event` and `M.result` and ends with `return M` (as the claude
  test does).

### 8.2 Real host (`cargo test`, in `dod`)

- `crates/agentstacks/tests/opencode.rs`, mirroring `tests/claude.rs`: plan fixtures under
  `tests/fixtures/opencode/<case>/expected` (every written file and skip line for the demo pack,
  copied from `crates/agentstacks-cli/tests/fixtures/project/packs/demo/` as `tests/claude.rs`
  copies its fixtures, and for a two-tool project), and `translate.lua` run on `airsl` against recorded payloads under
  `tests/fixtures/opencode/payloads` (read, write, patch with Add/Update/Move/Delete, a subagent
  event, session.start with a block, prompt.submit), asserting stdout byte for byte.
- `tests/runtime.rs`: the four list cases of §3.2.
- `shorthand.rs`: `official_tool_opencode_expands_to_extensions_subdir`.

### 8.3 `opencode-check` (real OpenCode, not in `dod`)

Files under `crates/agentstacks-cli/tests/opencode-check/`:

| File | Role |
|---|---|
| `mock.mjs` | the mock model, the `probes/harness/mock.mjs` logic: an OpenAI-compatible chat server on `127.0.0.1` that plays scripted tool calls and logs every request |
| `check.mjs` | builds a scratch project by copying `extensions/opencode` to `<scratch>/ext/opencode` and `pack/` to `<scratch>/packs/check`, and passes those relative paths (`init` refuses absolute ones, `probes/agentstacks-results.md` P1). Runs each scenario in OpenCode isolated exactly as `probes/harness/run.sh`: HOME and XDG dirs inside the scratch dir, `--standalone --auto`, `OPENCODE_DISABLE_MODELS_FETCH=1`, `OPENCODE_DISABLE_AUTOUPDATE=1`, and a global `opencode.json` registering provider `mock` (package `@opencode/ai/providers/openai-compatible`, `baseURL` `http://127.0.0.1:<port>/v1`, model `mock/m`). Asserts, prints one `ok`/`FAIL` line per scenario, exits 1 on any failure |
| `pack/` | the check pack, below |

The check pack (`pack.toml` name `check`, `requires.airsl = ">=0.1.5"`):

| Item | Definition |
|---|---|
| skill `hello` | `invocation = "any"`, body holding the marker `HELLO-BODY`, plus `references/note.md` |
| skill `ask` | `invocation = "user"`, body `ASK-BODY args=<{{var arguments}}>` |
| agent `helper` | `tools = ["read", "grep"]`, body holding `HELPER-BODY` |
| hook 0 | `kind = "message"`, `session.start`, `match = ["startup"]`, text `CHECK-WELCOME` |
| hook 1 | `kind = "lua"`, `session.start`, no `match`, returns `{ context = "CHECK-START " .. event.source }` |
| hook 2 | `kind = "lua"`, `prompt.submit`, returns `{ context = "CHECK-PROMPT" }` |
| hook 3 | `kind = "lua"`, `tool.before`, `match = ["write"]`, `may_block = true`: `blocked.txt` → `{ block = "CHECK-BLOCK " .. event.path }`, else `{ context = "CHECK-BEFORE " .. event.path }` |
| hook 4 | `kind = "lua"`, `tool.after`, `match = ["read"]`, returns `{ context = "CHECK-AFTER " .. event.path }` |
| hook 5 | `kind = "message"`, `session.end`, text `CHECK-END` (skipped) |
| hook 6 | `kind = "lua"`, `agent.stop`, returns nothing (skipped) |

Scenarios (each asserts on the requests the mock received, the tool results the model saw, and the
files left):

| # | Scenario | Asserts |
|---|---|---|
| S1 | `agentstacks init --tool ext/opencode --pack packs/check --yes`, then `plan`, `doctor`, `update` | exit 0; `0 to write`; `healthy`; skip lines for hooks 5 and 6 with the §4.6 reasons; `update` prints `nothing to update` (path sources only, claude §11); the plugin loaded (the S4 assertions would fail otherwise) |
| S2 | skill: the model calls `skill` with the check skill id | result holds the body marker, `Base directory for this skill:` and the `references/` path; the system prompt lists the skill and not the user-only one |
| S3 | agent: the model calls `subagent` with the check agent | a later request carries the agent body marker; that request offers only `grep, read, skill` |
| S4 | hooks: prompt, write `blocked.txt`, write `ok.txt`, read, shell `echo "$AGENTSTACKS_PROJECT_DIR"` | the first user message holds the session.start and prompt contexts; `blocked.txt` absent and its error carries the block reason; `ok.txt` result carries the before context; read result carries the after context; shell prints the project root |
| S5 | patch: project model `mock/gpt-5`, a patch adding `blocked.txt` and updating a file, then a patch adding `ok2.txt` | the first patch blocked with `CHECK-BLOCK blocked.txt` and nothing written; the second applied with `CHECK-BEFORE ok2.txt` in its result (hook 3 matches `write`; the shape of `probes/shim-results.md:443-449`) |
| S6 | command: `opencode api --standalone session.command` with the user-only skill's command and text `a b`, then `opencode run --session <id>` | the model receives the expanded body with `Load the skill` and `a b` (`probes/config-results.md` P1b) |
| S7 | missing airsl, in one `opencode serve --port <p>` process started with `PATH` minus every directory holding an `airsl` file and with `OPENCODE_PASSWORD` set, driven by `opencode run --server http://127.0.0.1:<p> --auto` clients (the shape of `probes/review-results.md` R4): prompt 1 `hello`, in which the model writes `blocked.txt`; prompt 2 `again` in the same session (`--session <id>`) | prompt 1's user message holds `CHECK-WELCOME` and ends with the D17 notice, exactly once, with `CHECK-START` and `CHECK-PROMPT` absent; `blocked.txt` is written (the guard is off); prompt 2's user message holds no notice; both clients exit 0 |

### 8.4 Task and CI

- **Task** `[tasks.opencode-check]` in `Makefile.toml`, category `OpenCode`, not a dependency of
  `dod` (like `claudevs-check`). Script:
  1. Pick the build from `uname -s`/`uname -m`: `Linux x86_64` → `opencode-linux-x64.tar.gz`,
     SHA256 `9ddd18e3faf64eaeccf6a0fcbc96a5c7a14520ae5fd5b61e5be51d894f478b47`; `Darwin arm64` →
     `opencode-darwin-arm64.zip`, SHA256
     `95a101eda95108478f1db43cff8fcbd4811f6c0e79af1e167301843c6a48d731`
     (`probes/opencode-v2.4.1-SHA256SUMS`). Anything else: exit 1 with
     `opencode-check: no pinned OpenCode build for <os> <arch>`.
  2. When `target/opencode-check/2.4.1/opencode` is missing: `curl -sSfL -o
     target/opencode-check/2.4.1/<asset>
     https://github.com/anomalyco/opencode/releases/download/v2.4.1/<asset>` (both URLs answer
     `302`, so `-L` is needed, `probes/release-results.md`), verify the SHA256 (`sha256sum`, else
     `shasum -a 256`), then unpack into the same directory: `tar -xzf` for the `.tar.gz`, `unzip -o
     -q` for the `.zip`. Each archive holds one entry, `opencode`, at its root, mode `rwxr-xr-x`
     (`probes/release-results.md`, archive contents), so the binary is
     `target/opencode-check/2.4.1/opencode`. A mismatch: exit 1 with
     `opencode-check: <asset>: checksum mismatch`.
  3. `cargo build -p agentstacks-cli`.
  4. `node crates/agentstacks-cli/tests/opencode-check/check.mjs target/opencode-check/2.4.1/opencode
     target/debug/agentstacks`.
- **CI job** `opencode-check` in `.github/workflows/ci.yml`: `ubuntu-latest`; the same checkout,
  toolchain, cache, cargo-make and airsl steps as `dod` (airsl 0.1.5 with its pinned hash);
  `actions/setup-node@v4` with `node-version: 22`; then `cargo make opencode-check`. It runs on the
  same triggers as the other jobs.
- Bumping OpenCode means changing the version and both hashes together, as the airsl pin does.

### 8.5 Docs

- New book page `book/src/ch06-03-opencode.md` (in `SUMMARY.md` after ch06-02): what the extension
  writes (§4.3), the hook mapping (§4.4-§4.5), the not-carried-over table (§7), the edge notes of §6
  marked "documented".
- `ch04-00-cli.md` and `ch01-*`: `--tool opencode` alongside `--tool claude`.
- `README.md` and `CLAUDE.md` Status: the opencode extension is built; the layout lists
  `extensions/opencode/`; Commands lists `cargo make opencode-check`. The stale lines change too:
  `CLAUDE.md:66` and `README.md:149` ("OpenCode plugins are TypeScript") become "OpenCode hooks are
  JavaScript plugins", and `README.md:163` ("opencode/ planned") becomes "opencode/ built". Also
  `README.md:7` ("OpenCode is not here yet"), `README.md:106` (`| OpenCode | Planned |`),
  `book/src/introduction.md:18` ("Claude Code is the one supported tool") and `CLAUDE.md:113`
  ("claude is built") name OpenCode as built.
- `ch05-00-plan-and-apply.md:372-373` ("A missing airsl after install is caught in two ways: the
  tool's own error for a missing command, and `doctor`") adds that under OpenCode the error is the
  shim's one-time notice to the model (D17).
- Changelog entries: `crates/agentstacks/CHANGELOG.md` (the runtime change, saying existing projects
  see `update <root>/runtime.lua` once, and `opencode` as an official tool),
  `crates/agentstacks-cli/CHANGELOG.md` (`--tool opencode`) and `packs/CHANGELOG.md` (the opencode
  extension).
- `extensions/opencode/README.md` is not added: the book page is the reference.

## 9. Limits (documented, not fixed)

- A `patch` header with leading spaces is applied by OpenCode but not seen by hooks (§4.7).
- Context and blocks from hooks reach the model; whether a real model then follows a command's
  "Load the skill" line, or a skill's `<arguments>` placeholder, is model behaviour that the mock
  cannot prove.
- A resumed session gets `session.start` as `startup` again in a new OpenCode process (§6).
- A hook's stderr is discarded, including the runtime's "tried to block but is not declared
  may_block" line.
- With airsl missing, every Lua hook is off, guards included; the model is told once per OpenCode
  process (once per background-service start), and an airsl without an execute bit is not told at
  all (§4.5 **Missing airsl**). `doctor` checks the airsl pin against the binary it finds (`src/app/doctor.rs:60-61`).
- Context added to a prompt becomes part of the stored user message: the user sees it in the history,
  and OpenCode's title request reads it (`probes/plugin-session-results.md` P2).
- Files written by `shell`, `generate_image` or `generate_video` (optional `path`, which overwrites,
  `probes/plugin-tool-results.md` P6) pass no tool hook as a read, edit or write.
- With both tools listed, Claude-only skills show in OpenCode (§6).

## 10. Risks

| # | Risk | Owner |
|---|---|---|
| K1 | The Linux build has not run here (D6), nor Linux `/bin/sh` exiting 127 for a missing airsl (D17, proven on macOS). The first CI run of `opencode-check` proves both (S7); if it fails, the plan that adds the CI job reports it | CI plan, execute |
| K2 | OpenCode V2 moves fast: v2.3.2 on 2026-11-18, v2.4.1 on 2026-11-23 (`probes/cadence-results.md`). The pin holds the check steady; a bump re-runs S1-S7 | maintainers |
| K3 | Every OpenCode API the shim calls is documented (§4.5 sources); none is `experimental` (D2). A V2 release could still change them; S1-S7 catch that at the next pin bump | maintainers |

## 11. Handed to other chains

- **`2026-11-24-packs-tool-neutral`**: Claude wording in pack text, and the journal's transcript
  lookup for OpenCode. Sessions live in `<XDG_DATA_HOME>/opencode/opencode.db`, and
  `opencode session export --standalone <id>` prints `{info, messages}` (`probes/config-results.md`
  P6), for that chain to design on.
- **Org docs** (`.github/profile/README.md`, workspace `README.md`): updated when `foundation/02` is
  rolled up, outside this repository.

## 12. Non-goals

- OpenCode V1, and any OpenCode version other than the pinned v2.4.1 in checks.
- agent.stop and session.end on OpenCode, and the experimental `experimental.idle` hook (D2).
- OpenCode global config, MCP servers, formatters, TUI plugins, `opencode.json`.
- Pack content changes (sibling chain).
- Freezing the API (`foundation/03`).
- Nested agentstacks projects (§6).
- Windows: the shim runs `/bin/sh`.
