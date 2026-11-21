---
status: draft
created: 2026-11-22
depends-on: [03, 04, 05, 09]
built-on: ea104af7451e
---

# Extension host Implementation Plan

**Goal:** Run a tool extension's adapter in the airsl sandbox.

**Architecture:** `src/host/` is the adapter behind the `Adapter` port (plan 09). It is the only
place Lua runs (spec §2). It translates, and decides nothing that `types/` already decides. Lua
values become `Operation` / `Proposal` / `ValuesResult` (plan 04) through those types' own
constructors, and JSON edits go through `types::json::edit` (plan 05). The host owns only what
needs Lua: reading arguments strictly as `mlua::Value`, the caller's location, module loading,
dispatch, the §5 Lua-value conversion, and wrapping airsl's errors. Each file has one job:

```text
src/host/
├─ mod.rs        docs + `mod` / `pub use` only (export-only)
├─ adapter.rs    AirslAdapter: implements the Adapter port by driving a Session per tool
├─ session.rs    Session: one tool's engine, kept alive from the entry run through values to plan (§4.4 flow)
├─ engine.rs     builds the airsl Engine: Policy::pure + the explicit module set (the one `dyn`)
├─ module.rs     AgentstacksModule: the HostModule that installs the `agentstacks` table (§4.2, §4.3)
├─ state.rs      RunState: phase, recorded proposals, sticky refusals, reads record, require bookkeeping
├─ caller.rs     the calling Lua frame as `<tool>/<file>:<line>` (§3.3, §13)
├─ args.rs       strict reading of one Lua argument table: required, exact type, list, unknown key (§3.3)
├─ ops.rs        write · copy · skip · require_binary (§4.3) into plan 04's Proposals
├─ read.rs       agentstacks.read over a pre-read snapshot of extension.reads (§4.3)
├─ require.rs    agentstacks.require over the in-memory files (§4.5)
├─ events.rs     agentstacks.on forwarding + dispatch outcome: none / nil / table (§4.3, §4.4)
├─ values.rs     the values result → plan 04's ValuesResult (§4.4)
├─ payload.rs    ValuesInput / PlanInput (plan 04) → the §4.4 JSON payloads
├─ convert.rs    Lua value → JSON (§5 second table); JSON value → LuaType word (§4.4)
├─ json.rs       agentstacks.json.edit: argument reading, conversion, call into types::json::edit (§5)
├─ failure.rs    airsl::Error → a located Diagnostic; the marker for the host's own raised refusals (§4.6)
├─ files.rs      LuaFiles: every `.lua` file of the extension in memory, module name ↔ path (§4.1, §4.5)
└─ testing.rs    #[cfg(test)] helper: build a Session from inline Lua files on a FakeFileTree
```

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6):

- `modularity.md` Rule 1: one job per file (above). Rule 2: no host copy of `Operation`, `OpPath`,
  `ItemId`, `Origin`, `Proposals`, `ValuesResult`, `OpError`, `LuaType` or `JsonEditError`. The host
  builds the canonical types.
- `static-dispatch.md`: `AirslAdapter` and `Session` are generic over the ports they are handed
  (`FileTree`, `Workspace`). The one `dyn` is in `engine.rs`: `ModuleSet::insert(Box<dyn
  HostModule>)`, which airsl's API requires (`airsl-0.1.4/src/modules/registry.rs:144`). It is marked
  `// dyn: airsl's ModuleSet stores trait objects` (spec §2).
- `mod-rs-export-only.md`: `host/mod.rs` holds its docs and `mod` / `pub use` lines only.
- `unit-test-mandate.md`: every logic file has a colocated `#[cfg(test)] mod tests`. `testing.rs` is
  a test-only helper module.
- `strong-types.md`: phase is an enum (`Phase::{Load, Values, Plan}`), never a `bool`. Module names
  and extension file paths are `TreePath` / a `ModuleName`-style newtype in `files.rs`, never bare
  `String`.
- `doc-comment-discipline.md`: doc comments describe the sandbox and the API only. They never name
  plans, specs, chains or agents.
- `mlua` is reached only as `airsl::mlua` (the re-export that pins the engine's version,
  `airsl-0.1.4/src/lib.rs:56`). No separate `mlua` dependency is added.

**Tech Stack:** Rust 2024, `airsl = "0.1.4"` (Engine, Policy, ModuleSet, HostModule, Ext, Json,
EventName, Script, `airsl::mlua`), `serde_json` 1.0.154 with `preserve_order` (both added by plan 01).

**Content authority:** spec §4.1–§4.6, the §5 Lua-value table, the reading half of §3.3 (§13 row 1:
"`host` (reading) + `types/op` (rule)"), and §14 row `host`.

---

## Files that change

```
crates/agentstacks/src/host/mod.rs      — modify: module docs (four questions), mod lines, pub use AirslAdapter
crates/agentstacks/src/host/adapter.rs  — create: AirslAdapter implements Adapter
crates/agentstacks/src/host/session.rs  — create: Session (entry run, values dispatch, plan dispatch)
crates/agentstacks/src/host/engine.rs   — create: engine builder with the explicit module set
crates/agentstacks/src/host/module.rs   — create: AgentstacksModule (HostModule)
crates/agentstacks/src/host/state.rs    — create: RunState + Phase
crates/agentstacks/src/host/caller.rs   — create: caller location
crates/agentstacks/src/host/args.rs     — create: strict argument-table reader
crates/agentstacks/src/host/ops.rs      — create: the four op functions
crates/agentstacks/src/host/read.rs     — create: agentstacks.read + ReadSnapshot
crates/agentstacks/src/host/require.rs  — create: agentstacks.require
crates/agentstacks/src/host/events.rs   — create: agentstacks.on + dispatch outcome
crates/agentstacks/src/host/values.rs   — create: values result translation
crates/agentstacks/src/host/payload.rs  — create: JSON payload builders
crates/agentstacks/src/host/convert.rs  — create: Lua→JSON conversion, JSON→LuaType word
crates/agentstacks/src/host/json.rs     — create: agentstacks.json.edit
crates/agentstacks/src/host/failure.rs  — create: error wrapping + Raised marker
crates/agentstacks/src/host/files.rs    — create: LuaFiles
crates/agentstacks/src/host/testing.rs  — create: #[cfg(test)] session helper
```

No other file is touched. `lib.rs`, `Cargo.toml` and the `host/mod.rs` stub belong to plan 01.

## Order of work

Every test below runs a real airsl 0.1.4 engine on inline Lua (spec §14 `host` row), except the
pure conversion tests in Tasks 1 and 5, which use Lua values made by evaluating inline chunks on an
`airsl::mlua::Lua` state. Expected messages are copied verbatim from the cited spec row. Each
message's enum variant is plan 02's (`OpError`, `JsonEditError`) or plan 04's. The host never
formats refusal text itself.

### Task 1 — Load the extension's Lua files into memory

**Files:**
- Create `crates/agentstacks/src/host/files.rs` — `LuaFiles`
- Create `crates/agentstacks/src/host/testing.rs` — test-only builder for an in-memory extension
- Modify `crates/agentstacks/src/host/mod.rs` — `mod files;` (+ `#[cfg(test)] mod testing;`)

**Change:** `LuaFiles::read<T: FileTree>(tree, extension)` walks the extension tree once with the
built `types::tree::walk`, and reads every walked file ending in `.lua` with `FileTree::read_file`.
Lua never touches disk (§4.1 step 3). It exposes:

- `entry()`: the entry file's normalised path (`ExtensionManifest::entry().normalized()`) and text;
- `module(name)`: maps a module name to `<name with . replaced by />.lua` (§4.5) and returns that
  file's path and bytes, or none;
- `entry_module()`: the module name of the entry file (`adapter.lua` → `adapter`,
  `src/main.lua` → `src.main`), used by the entry rule of §4.5.

The `testing.rs` helper builds a `FakeFileTree` holding `extension.toml` (tool `claude`, `api = 0`,
`entry = "adapter.lua"`, optional `reads`) plus inline `.lua` files. It loads it with
`Extension::load` and builds a Session from it (Task 12 completes the Session half; until then it
returns the `LuaFiles` and the `Extension`).

**Proof:**
- `reads_every_lua_file_of_the_extension` — a tree with `adapter.lua`, `lib/util.lua` and
  `README.md` yields exactly the two `.lua` paths, with their bytes unchanged.
- `maps_a_dotted_module_name_to_its_file` — `lib.util` finds `lib/util.lua`; `lib.nope` finds none.
- `entry_module_drops_the_extension_and_uses_dots` — entry `adapter.lua` gives `adapter`; entry
  `src/main.lua` gives `src.main`.

**Done when:** `cargo test -p agentstacks host::files` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): read an extension's lua files into memory`.

### Task 2 — Build the sandbox

**Files:**
- Create `crates/agentstacks/src/host/engine.rs` — `build_engine(module) -> airsl::Result<Engine>`
- Create `crates/agentstacks/src/host/module.rs` — `AgentstacksModule` (`HostModule`)
- Create `crates/agentstacks/src/host/state.rs` — `RunState`, `Phase`
- Modify `crates/agentstacks/src/host/mod.rs`, `host/testing.rs` — mod lines; helper evaluates inline Lua

**Change:** (§4.2)

- `build_engine` is `Engine::builder().policy(Policy::pure()).stdlib(set).build()`. `set` is a
  `ModuleSet` holding exactly `Json::new()`, `Ext::with_events([values, plan])` (two `EventName`
  constants) and `AgentstacksModule`, inserted with `ModuleSet::insert(Box<dyn HostModule>)` under
  the `// dyn: airsl's ModuleSet stores trait objects` comment. The set is always explicit, because
  `pure()` alone installs stdio, hook and time (airsl#11, §4.2).
- `AgentstacksModule` is installed as `airsstack.agentstacks`. Its `install`:
  - sets the global `agentstacks` to that same table;
  - sets the globals `print` and `warn` to nil (proven for `print`, Q2 `OK "nil"`);
  - fills the table: `on`, `require`, `write`, `copy`, `skip`, `require_binary`, `read`, and
    `json.edit`. Their bodies come from Tasks 4–10, and this task installs the keys.
- `RunState` is shared as `Arc<Mutex<…>>`, because `HostModule: Send + Sync` and the module is
  stored as `Box<dyn HostModule>` (so `'static`; registry.rs:97). It holds:
  - the `Phase` (`Load` while the entry runs, `Values`, `Plan`);
  - the run's `Proposals` (plan 04);
  - the sticky refusals (`Diagnostics`);
  - the reads record;
  - the require loading stack and loaded set;
  - the tool name and the plan payload's id index.

  Lua functions capture clones of the `Arc`, and nothing captures a borrowed port.

**Proof:**
- `sandbox_globals_are_exactly_the_allowed_set` — the sorted names of `_G` are Q1's `ALL_GLOBALS`
  (`probes/airsl-results.md`) without `print` and `warn`, plus `agentstacks`.
- `print_and_warn_are_removed` — `type(print)` and `type(warn)` are both `"nil"`.
- `withheld_globals_are_absent` — `require`, `load`, `dofile`, `os`, `io` and `coroutine` are all
  `nil` (§4.2 "Absent").
- `airsstack_holds_only_agentstacks_ext_and_json` — the sorted keys of `airsstack` are
  `agentstacks,ext,json`.
- `agentstacks_global_is_the_module_table` — `rawequal(agentstacks, airsstack.agentstacks)` is true.
- `agentstacks_table_has_exactly_the_api_zero_functions` — the sorted keys are
  `copy,json,on,read,require,require_binary,skip,write`, and `json` has exactly `edit`.
- `string_table_math_utf8_are_present` — each is a table (§4.2 "Present").

**Done when:** `cargo test -p agentstacks host::` passes and `cargo make dod-crate agentstacks` is
green. Commit `feat(agentstacks): airsl sandbox for tool extensions`.

### Task 3 — Locate the calling Lua frame

**Files:**
- Create `crates/agentstacks/src/host/caller.rs` — `Caller { file: TreePath, line: NonZeroUsize }`, `caller(lua, tool) -> Option<Caller>`, `Caller::location(tool) -> Location`
- Modify `crates/agentstacks/src/host/mod.rs`

**Change:** `caller` reads `lua.inspect_stack(1, …)` (§3.3; `probes/round2-results.md` R5). It takes
the frame's `source` without its leading `@` as the extension file, and `current_line()` as the line.
`source` is used rather than `short_src`, which Lua truncates for long names; for the names in the
probes the two are equal (R5 full stack: `source=Some("@adapter.lua") short_src=Some("adapter.lua")`).

`Caller::location` gives `Location::at_line("<tool>/<file>", line)` (§13 row 1), and
`Caller::origin(tool)` gives plan 04's `Origin::Extension { tool, file, line }`. A chunk run by the
host for a required module is named `@<path>` (Task 10), so a call made inside a module names that
module (§3.3, `{"at":"lib/settings.lua:3","here":"adapter.lua:3"}` in `probes/examples-results.txt`).

**Proof:**
- `names_the_entry_file_and_line_at_top_level` — a host function called at line 3 of `adapter.lua`
  yields `claude/adapter.lua:3`.
- `names_the_line_inside_a_dispatched_handler` — a call on line 2 of a `plan` handler yields
  `claude/adapter.lua:2` (§3.3, `probes/round3-results.md` A4).
- `names_the_required_module_not_the_entry` — a call made inside a function defined in
  `lib/settings.lua` (loaded under chunk name `@lib/settings.lua`) yields
  `claude/lib/settings.lua:3`.

**Done when:** `cargo test -p agentstacks host::caller` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): locate the calling lua frame`.

### Task 4 — Forward `agentstacks.on`, dispatch, and wrap Lua failures

**Files:**
- Create `crates/agentstacks/src/host/events.rs` — the `on` function; `dispatch(engine, event, payload) -> Dispatched`
- Create `crates/agentstacks/src/host/failure.rs` — `Raised` marker; `wrap(tool, entry, error) -> Option<Diagnostic>`
- Modify `crates/agentstacks/src/host/module.rs`, `host/mod.rs`

**Change:**

- `agentstacks.on(event, fn)` looks up `airsstack.ext.on` from the globals **when called**, never at
  install, because the global `airsstack` does not exist while modules install (`install: global
  airsstack is nil`, `probes/examples-results.txt`). It forwards both arguments unchanged, and
  returns airsl's error to Lua unchanged. A later registration replaces the earlier one and takes
  effect at the next dispatch (D5, §4.3; `probes/round3-results.md` A1).
- `dispatch` calls `Engine::dispatch` and returns `Dispatched::{NoHandler, Nil, Value(serde_json::Value)}`
  for `Ok(None)`, `Ok(Some(Null))` and `Ok(Some(other))` (§4.4, A3, R1b).
- `failure::Raised` is the error type the host's own Lua functions raise
  (`mlua::Error::external(Raised { diagnostic, sticky })`). Its `Display` is the diagnostic's text,
  so Lua code that catches it with `pcall` sees the located message.
- `wrap` turns an `airsl::Error` from an entry run or a dispatch into the diagnostic the user sees
  (§4.6). It walks the error's source chain for a `Raised`:
  - a sticky `Raised` (an op or `read` refusal, already recorded, Task 8): nothing more is added;
  - a non-sticky `Raised` (an uncaught `json.edit` or `require` refusal): its own located diagnostic;
  - anything else: `Location::file("<tool>/<entry>")` with plan 02's Lua-failure variant, carrying
    the **first line** of the error's `Display` as text. This also covers `InstructionLimit` and
    `MemoryLimit` (§4.6 rows 3–5).

**Proof:**
- `on_forwards_to_ext_on_at_call_time` — the entry registers `values` through `agentstacks.on`, and
  dispatch returns the handler's table.
- `second_registration_replaces_the_first` — two `values` registrations: dispatch returns
  `{"which":"second"}` (§4.4, A1).
- `registration_inside_values_takes_effect_at_plan` — a `plan` handler registered from the `values`
  handler runs on the `plan` dispatch (§4.3).
- `no_handler_is_distinct_from_nil_and_table` — no handler → `NoHandler`, `return nil` → `Nil`,
  `return {a=1}` → `Value({"a":1})` (§4.4).
- `unknown_event_is_airsl_text_with_sorted_events` — `agentstacks.on("nope", f)` at line 1 gives
  `claude/adapter.lua: lua error in adapter.lua: event `nope` is not one this host dispatches;
  declared events: plan, values` (§4.6 last row).
- `syntax_error_shows_first_line` — entry `local function f(` gives `claude/adapter.lua: lua error in
  adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>` (§4.6 row 3, Q5a).
- `runtime_error_in_plan_names_the_event_and_drops_the_traceback` — a `plan` handler calling
  `error("path must be relative")` on line 3 gives `claude/adapter.lua: lua error in plan:
  adapter.lua:3: path must be relative`, with no `stack traceback` text (§4.6 row 3, A5).
- `instruction_limit_in_plan` — a `plan` handler running `while true do end` gives
  `claude/adapter.lua: script `plan` exceeded its instruction limit of 10000000` (§4.6 row 4).
- `memory_limit_in_entry` — entry `local s = string.rep("x", 20*1024*1024)` gives
  `claude/adapter.lua: script `adapter.lua` exceeded its memory limit of 16777216 bytes` (§4.2, Q9a).
- `breach_caught_by_pcall_is_the_extensions_business` — a `plan` handler that wraps
  `while true do end` in `pcall` returns normally, and the dispatch succeeds (§4.2, P).
- `budget_resets_on_each_dispatch` — the `values` handler exhausts the budget inside `pcall` and
  returns. The `plan` handler then runs a one-million-iteration loop (R2 eval7) and returns
  normally (§4.2, `engine.rs:342`).

**Done when:** `cargo test -p agentstacks host::events host::failure` passes and
`cargo make dod-crate agentstacks` is green. Commit
`feat(agentstacks): event registration, dispatch and lua failure messages`.

### Task 5 — Convert Lua values to JSON

**Files:**
- Create `crates/agentstacks/src/host/convert.rs` — `lua_to_json(&mlua::Value) -> Result<serde_json::Value, JsonEditError>`; `json_word(&serde_json::Value) -> LuaType`
- Modify `crates/agentstacks/src/host/mod.rs`

**Change:** The hand-written conversion of the §5 second table. mlua's serde conversion drops data
(`convert({1, a=2})` → `[1]`, `probes/round3-results.md` A4). Tables are read with raw access only,
so no metamethod runs inside the host.

- A table whose keys are exactly the integers `1..n` becomes an array, in index order.
- A table with only string keys becomes an object, with keys sorted by bytes.
- An empty table becomes `{}` (`airsl-0.1.4/src/convert.rs:41`).
- Any other table is refused with plan 02's `JsonEditError` variant for "table mixes list and keyed
  entries".
- An integer becomes a number from `i64`. A float becomes a number from `f64`, and NaN or inf is
  refused.
- A UTF-8 string becomes a string; any other string is refused. A boolean becomes a boolean.
- A function, userdata or thread is refused with its Lua type word.

`json_word` maps a dispatched JSON value back to the word the Lua author wrote (§4.4): object →
`table`, array → `list`, null → `nil`, number → `number`, string → `string`, boolean → `boolean`
(plan 02's `LuaType`).

**Proof:** (each refusal message is the §5 second table's, shown here as it renders with the
`json.edit: ` prefix Task 6 adds)
- `sequence_becomes_array` — `{1, "a", true}` → `[1,"a",true]`.
- `string_keys_become_sorted_object` — `{zeta=1, alpha=2}` → `{"alpha":2,"zeta":1}`, in that
  order.
- `empty_table_becomes_empty_object` — `{}` → `{}`.
- `mixed_table_is_refused` — `{1, a=2}` → `json.edit: value: table mixes list and keyed entries`.
- `sparse_integer_keys_are_refused` — `{[1]=1, [3]=3}` → the same message ("any other table").
- `integer_and_float_become_numbers` — `3` → `3`, `0.5` → `0.5`.
- `nan_is_refused` — `0/0` → `json.edit: value: NaN cannot be JSON`.
- `inf_is_refused` — `math.huge` → `json.edit: value: inf cannot be JSON` (§5 "(also `inf`)").
- `non_utf8_string_is_refused` — `"\255"` → `json.edit: value: string is not valid UTF-8`.
- `function_is_refused` — `function() end` → `json.edit: value: function cannot be JSON`.
- `thread_and_userdata_are_refused` — a thread and a userdata made in the test state give the same
  row's shape with their own word (`json.edit: value: thread cannot be JSON`,
  `json.edit: value: userdata cannot be JSON`).
- `nested_values_convert_recursively` — `{ { matcher = "Read" } }` → `[{"matcher":"Read"}]`.
- `json_words_match_lua_words` — object/array/null/number/string/boolean map to
  `table`/`list`/`nil`/`number`/`string`/`boolean`.

**Done when:** `cargo test -p agentstacks host::convert` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): convert lua values to json`.

### Task 6 — Expose `agentstacks.json.edit`

**Files:**
- Create `crates/agentstacks/src/host/json.rs` — the `json.edit` Lua function
- Modify `crates/agentstacks/src/host/module.rs`, `host/mod.rs`

**Change:** `agentstacks.json.edit(text, path, value)` works in any phase (§4.3). The arguments
are read strictly as `mlua::Value`:

- `text` must be a string;
- `path` must be a table, a non-empty list whose every element is a string;
- `value` goes through `lua_to_json`, and `nil` means remove.

It then calls `types::json::edit(text, path, value)` (plan 05). A refusal, from reading or from
`edit`, is raised as a **non-sticky** `Raised`, located at the caller (Task 3) with `json.edit: ` and
the `JsonEditError` text (§5, §13 row 1). It is a normal Lua error: `pcall` catches it, and an
uncaught one fails the dispatch (§4.6 row 2).

**Proof:**
- `checked_example_is_byte_for_byte` — the §5 checked example run through Lua returns exactly the
  §5 output block (4-space indent, trailing newline kept). This is the same text
  `probes/examples-results.txt` reports `true` for.
- `text_must_be_a_string` — `agentstacks.json.edit(nil, {"a"}, 1)` on line 1 gives
  `claude/adapter.lua:1: json.edit: text: expected string, got nil` (§5 row 1).
- `path_must_be_a_table` — `path = "a"` gives `claude/adapter.lua:1: json.edit: path: expected
  table, got string` (§5 row 1).
- `path_must_not_be_empty` — `{}` gives `claude/adapter.lua:1: json.edit: path: empty` (§5 row 5).
- `path_elements_must_be_strings` — `{"a", 2}` gives `claude/adapter.lua:1: json.edit: path[2]:
  expected string, got number` (§5 row 5).
- `value_refusal_is_located` — `value = {1, a=2}` gives `claude/adapter.lua:1: json.edit: value:
  table mixes list and keyed entries`.
- `edit_refusal_passes_through_located` — `text = '{"hooks": 1}'`, path `{"hooks","x"}` gives
  `claude/adapter.lua:1: json.edit: "hooks" is not an object` (§5 row 6, the message from plan 05).
- `nil_value_removes_the_key` — removing `a` from `{"a":1,"b":2}` returns `{"b":2}` in two-space
  layout. A missing key returns the input text unchanged (§5 row 7).
- `refusal_is_catchable_with_pcall` — `pcall(agentstacks.json.edit, nil, {"a"}, 1)` returns
  `false`, the entry run succeeds, and no refusal is recorded.
- `uncaught_refusal_fails_with_its_located_message` — an uncaught `json.edit` refusal on line 2 of a
  `plan` handler fails the run with exactly
  `claude/adapter.lua:2: json.edit: text: expected string, got nil`.

**Done when:** `cargo test -p agentstacks host::json` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): agentstacks.json.edit for extensions`.

### Task 7 — Read op arguments strictly

**Files:**
- Create `crates/agentstacks/src/host/args.rs` — `ArgTable` over one `mlua::Value`: `string(field)`, `bytes(field)`, `string_list(field)`, `finish()` (unknown keys)
- Modify `crates/agentstacks/src/host/mod.rs`

**Change:** The reading half of §3.3, with no coercion. airsl's raw reading turned `path = 5` into
`"5"`, and serde reading accepted unknown keys (Q3), so the reader matches `mlua::Value` variants by
hand:

- The argument must be a table (`expected table, got <word>`).
- `nil` counts as missing (`<field>: required`).
- A field's variant must match exactly (`<field>: expected string, got <word>`), and integer and
  float are both `number` (§3.3 "Type words").
- `covers` must be a table whose keys are exactly `1..n`, otherwise `covers: expected a list`.
  Each element must be a string (`covers[<i>]: expected string, got <word>`). An empty `covers` is
  the same as none (§3.3 "Accepted").
- After all named fields are read, any other key gives `<key>: unknown key`.
- `path` must be UTF-8 (`path: not valid UTF-8`). `content` keeps its raw bytes
  (`as_bytes=[97, 255, 98, 0, 99]`, R4); an empty `content` is accepted.

Every problem in one call is collected, never first-only (§6.1 "collected, never first-only"). The
words are plan 02's `LuaType`. The texts are plan 02's `OpError` variants for the §3.3 rows.

**Proof:** (reader-level tests on Lua values; the caller prefix is added in Task 8)
- `argument_must_be_a_table` — `"x"` → `expected table, got string` (§3.3 row 1).
- `nil_field_counts_as_missing` — `{content = "x"}` read for `path` → `path: required` (§3.3 row 2).
- `no_coercion_of_numbers` — `{path = 5}` → `path: expected string, got number` (§3.3 row 3).
- `float_is_also_number` — `{path = 5.5}` → `path: expected string, got number`.
- `covers_must_be_a_table` — `{covers = "x"}` → `covers: expected table, got string`.
- `covers_must_be_a_list` — `{covers = {a = "x"}}` → `covers: expected a list` (§3.3 row 4).
- `covers_elements_must_be_strings` — `{covers = {"a", true}}` → `covers[2]: expected string, got
  boolean`.
- `empty_covers_is_none` — `{covers = {}}` reads as no covers.
- `unknown_key_is_refused` — `{path = "a", content = "", foo = 1}` → `foo: unknown key` (§3.3 row 5).
- `path_must_be_utf8` — `{path = "a\255"}` → `path: not valid UTF-8` (§3.3 row 6).
- `content_keeps_raw_bytes` — `content = "a\255b\0c"` → bytes `[97, 255, 98, 0, 99]`.
- `empty_content_is_accepted` — `content = ""` → zero bytes.
- `every_problem_in_one_call_is_collected` — `{foo = 1}` read for `path` and `content` yields
  `content: required`, `path: required` and `foo: unknown key`.

**Done when:** `cargo test -p agentstacks host::args` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): strict reading of extension arguments`.

### Task 8 — Record operations during the plan dispatch

**Files:**
- Create `crates/agentstacks/src/host/ops.rs` — `write`, `copy`, `skip`, `require_binary` Lua functions
- Modify `crates/agentstacks/src/host/module.rs`, `host/state.rs`, `host/mod.rs`

**Change:** Each op function (§4.3) does the following:

1. Reads the caller (Task 3).
2. Refuses outside `Phase::Plan` with `only allowed inside the plan handler` (§3.3 last row).
3. Reads its fields with `ArgTable` (Task 7):
   - `write`: `path`, `content`, optional `covers`;
   - `copy`: `path`, `file`;
   - `skip`: `item`, `reason`;
   - `require_binary`: `name`.
4. Hands the values to plan 04's constructors: `OpPath` (shape, normalisation, own files), the
   item-id check against the run's plan payload, `skip.reason` non-empty, and `BinaryName` (§3.3
   rows 7–12). It then adds the resulting `Proposal { op, origin: Origin::Extension{…} }` to the
   run's `Proposals`, which applies the duplicate-path and skip/cover-conflict rules (§3.2, §3.3).

Every refusal is a `Diagnostic` at `<tool>/<file>:<line>` with text `<function>: <what>`. It is
recorded in `RunState` when it happens (**sticky**, §3.3 "Refusals stick") and raised as a sticky
`Raised`. When one call holds several refusals, all of them are recorded, and the raised error
carries the first in `Diagnostics` sort order (G10). A refused op is not recorded. A plan with a sticky refusal is refused even when the Lua
code catches the error with `pcall`.

**Proof:**
- `write_inside_plan_is_recorded_with_its_origin` — `agentstacks.write{path=".claude/x.md",
  content="a"}` on line 2 of a `plan` handler records one `Write` whose origin is
  `Extension{claude, adapter.lua, 2}`.
- `copy_skip_and_require_binary_are_recorded` — one of each, with payload ids, records three
  proposals in call order.
- `ops_at_top_level_are_refused` — `agentstacks.write{…}` on line 1 of the entry gives
  `claude/adapter.lua:1: write: only allowed inside the plan handler`.
- `ops_in_values_are_refused` — `agentstacks.skip{…}` on line 3 inside the `values` handler gives
  `claude/adapter.lua:3: skip: only allowed inside the plan handler`.
- `argument_refusal_carries_location_and_function` — `agentstacks.write{path=5, content="x"}` on
  line 12 gives `claude/adapter.lua:12: write: path: expected string, got number` (§13 row 1).
- `path_rule_refusal_carries_location` — `agentstacks.write{path="/etc/x", content="x"}` on line 12
  gives `claude/adapter.lua:12: write: path "/etc/x" must be relative` (§3.3 example).
- `id_refusal_carries_location` — `agentstacks.copy{path="a", file="core/skill/x/SKILL.md"}` gives
  `…: copy: file "core/skill/x/SKILL.md" is not in this plan` (§3.3 row 10).
- `duplicate_path_names_the_first_call` — two writes to `.claude/x` at lines 9 and 14 give
  `claude/adapter.lua:14: write: path ".claude/x" is already proposed at adapter.lua:9` (§3.3 row 9).
- `skip_after_copy_conflict_names_the_first_call` — copy of a skill file at line 9, then skip of its
  skill at line 14, give `claude/adapter.lua:14: skip: item "core/skill/status" is already covered
  at adapter.lua:9` (§3.2 table row 2).
- `refusal_sticks_under_pcall` — `pcall(function() agentstacks.write{path="/x", content=""} end)`
  in the `plan` handler: the dispatch succeeds, the refusal is still returned, and no `Write` is
  recorded (§3.3 "Refusals stick").
- `uncaught_op_refusal_is_reported_once` — the same write without `pcall` fails the dispatch, and
  the run returns exactly one diagnostic, the §3.3 message, with no Lua-failure wrapper (§4.6 row 1).
- `op_inside_required_module_names_the_module` — a write made by a function in `lib/settings.lua`
  that fails a path rule is located at `claude/lib/settings.lua:<line>` (§3.3, §13).

**Done when:** `cargo test -p agentstacks host::ops` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): record extension operations`.

### Task 9 — Serve `agentstacks.read` from the declared reads

**Files:**
- Create `crates/agentstacks/src/host/read.rs` — `ReadSnapshot` + the `read` Lua function
- Modify `crates/agentstacks/src/host/module.rs`, `host/state.rs`, `host/mod.rs`

**Change:** Lua functions are `'static`, so they cannot borrow the `Workspace`. Before the `plan`
dispatch, the Session therefore asks the `Workspace` port (plan 09) for every entry of
`extension.reads` (exposed by plan 03). For each it gets the §6.3 resolution (inside / outside) and,
when inside, the disk state, and keeps the answers in a `ReadSnapshot`. `agentstacks.read(path)`
then (§4.3):

1. refuses outside `Phase::Plan` with `read: only allowed inside the plan handler`;
2. requires one string argument (`read: expected string, got number`, §3.3 row 1);
3. normalises the path (§3.3 "Normalisation") and refuses one not in `extension.reads`:
   `read: ".env" is not in extension.reads`;
4. refuses an outside path at the caller's line:
   `read: ".claude/settings.json" resolves outside the project` (§4.3, §6.3);
5. returns the bytes as a Lua string, non-UTF-8 included (Q7 `bytes=0,255,65`), or `nil` when no
   regular file is there;
6. records what it returned in the reads record: the `ContentHash` of the bytes, or absent (§4.3,
   for §6.2 "Shared" and §7 step 3). Only paths actually read are recorded. A path read twice is
   recorded once.

The refusals are sticky, like op refusals (they are §3.3 rows).

**Proof:**
- `read_returns_raw_bytes` — a reads path holding `[0, 255, 65]` returns a Lua string of length 3
  with those bytes.
- `read_returns_nil_for_an_absent_file` — returns `nil`, and records absent.
- `read_returns_nil_for_a_non_regular_file` — a directory returns `nil`.
- `read_records_the_hash_of_what_it_returned` — the record holds the `ContentHash` of the bytes,
  for that path only.
- `unread_reads_entries_are_not_recorded` — a declared but unread path is absent from the record.
- `read_outside_reads_is_refused` — `agentstacks.read(".env")` on line 4 gives
  `claude/adapter.lua:4: read: ".env" is not in extension.reads`.
- `read_resolving_outside_is_refused` — a reads path that resolves outside, read on line 9, gives
  `claude/adapter.lua:9: read: ".claude/settings.json" resolves outside the project`.
- `read_takes_one_string` — `agentstacks.read(5)` gives `…: read: expected string, got number`.
- `read_outside_plan_is_refused` — at top level: `…: read: only allowed inside the plan handler`.
- `read_path_is_normalised` — `./.claude//settings.json` matches the reads entry
  `.claude/settings.json`.
- `eight_mib_read_fits_the_memory_limit` — an 8 MiB file read in the `plan` handler returns
  `#s == 8388608` (§4.2, Q9d).

**Done when:** `cargo test -p agentstacks host::read` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): agentstacks.read over declared reads`.

### Task 10 — Load modules with `agentstacks.require`

**Files:**
- Create `crates/agentstacks/src/host/require.rs` — the `require` Lua function
- Modify `crates/agentstacks/src/host/module.rs`, `host/state.rs`, `host/mod.rs`

**Change:** (§4.5) `require` stays absent: airsl clears it for a script built from source.
`agentstacks.require(name)` works in any phase:

1. It checks `name` against `^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)*$`.
2. It refuses the entry module (`LuaFiles::entry_module`).
3. It refuses a name already on the loading stack (a cycle), listing the stack from the first
   occurrence to the repeat, joined by ` → `.
4. It looks the file up in `LuaFiles`.
5. It loads the file once with `lua.load(bytes).set_name("@<path>")`, the call shape
   `probes/examples-main.rs` uses.

It caches the result in a named Lua registry table, keyed by module name, plus a loaded set in
`RunState`. A module that returns `nil` is still loaded only once. A module that raised while loading
is not cached, and is popped from the loading stack. An error raised inside a module keeps the
module's file and line (Q4), and surfaces through Task 4's wrapper. The host keeps `load` for itself
and never exposes it.

Refusals are raised as non-sticky `Raised` at the caller (§4.5 table: "location: the caller").

**Proof:**
- `require_returns_the_module_value` — `agentstacks.require("lib.util").twice(21)` returns `42` (Q4).
- `require_loads_once_and_caches` — a module that increments a global counter, required twice:
  the counter is 1 and both results are `rawequal`.
- `module_error_carries_its_file_and_line` — `lib/util.lua` raising `boom` on line 2 gives
  `claude/adapter.lua: lua error in adapter.lua: runtime error: lib/util.lua:2: boom`
  (§4.5, §4.6 row 3).
- `failed_module_is_not_cached` — a module that raises on its first load only: `pcall` around the
  first require, then a second require returns the module's value.
- `invalid_name_is_refused` — `agentstacks.require("lib/x")` on line 1 gives
  `claude/adapter.lua:1: require: invalid module name "lib/x" (use dots: lib.x)` (§4.5 row 1).
- `missing_module_is_refused` — `agentstacks.require("lib.x")` gives
  `claude/adapter.lua:1: require: module "lib.x" not found in extension claude` (§4.5 row 2).
- `entry_module_is_refused` — `agentstacks.require("adapter")` gives
  `claude/adapter.lua:1: require: "adapter" is the entry file` (§4.5 row 3).
- `load_stays_withheld` — after a require, `type(load)` is still `"nil"` (Q4).
- The cycle row (§4.5 row 4) gets its test once spec gap G6 (below) is settled.

**Done when:** `cargo test -p agentstacks host::require` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): agentstacks.require over in-memory modules`.

### Task 11 — Build the payloads and read the values result

**Files:**
- Create `crates/agentstacks/src/host/payload.rs` — `values_payload(&ValuesInput) -> serde_json::Value`, `plan_payload(&PlanInput) -> serde_json::Value`
- Create `crates/agentstacks/src/host/values.rs` — `values_result(tool, entry, Dispatched, &ValuesInput) -> Result<ValuesResult, Diagnostics>`
- Modify `crates/agentstacks/src/host/mod.rs`

**Change:**

- `payload.rs` translates plan 04's domain models into the §4.4 JSON shapes, key for key:
  - `values`: `api`, `tool`, `packs[{name, version, skills, agents}]`, `vars`;
  - `plan`: `api`, `tool`, `root`, `packs[…]` with `skills[{id, name, dir, meta, files[{id, rel,
    text?}]}]`, `agents`, `hooks`, `native{files, hooks}`;
  - `text` is absent for a binary file.

  Translation only: every value comes from the input type.
- `values_result` (§4.4):
  - `NoHandler` gives `registers no values handler`;
  - `Nil` or a non-object gives `values: expected table, got <json_word>`;
  - an object goes to plan 04's `ValuesResult` validation (root, skills, agents, vars, unknown
    keys, item and var names of this run, path normalisation of `root` and `dir`).

  All of these are at `Location::file("<tool>/<entry>")` or with the `values` key path (§13 row 2).

**Proof:**
- `values_payload_matches_the_spec_example` — the payload built for the §4.4 example input equals
  the §4.4 `values` payload JSON.
- `plan_payload_matches_the_spec_example` — a `PlanInput` holding the §4.4 example's packs equals
  the §4.4 `plan` payload JSON, including `logo.png` without `text`.
- `values_example_dispatches_to_the_spec_result` — the §4.4 `lua` example dispatched with the §4.4
  payload gives a `ValuesResult` with root `.claude/agentstacks`, skill `core/status` → invoke
  `/status`, dir `.claude/skills/status`, agent `core/coder` → `coder`, var `tmpdir` → `$TMPDIR`
  (`probes/examples-results.txt`).
- `missing_values_handler_is_refused` — an entry that registers only `plan` gives
  `claude/adapter.lua: registers no values handler` (§4.4 row 1).
- `values_returning_nil_is_refused` — gives `claude/adapter.lua: values: expected table, got nil`
  (§4.4 row 2).
- `values_returning_a_list_names_list` — `return {1, 2}` gives `claude/adapter.lua: values:
  expected table, got list`.
- `empty_table_arrives_as_object` — `return {}` gives `claude/adapter.lua: values.root: required`
  (§4.4 rows 3 and "An empty Lua table arrives as an object").
- `nested_type_word_is_list` — `agents = {"x"}` gives `claude/adapter.lua: values.agents: expected
  table, got list` (§4.4 row 4).

**Done when:** `cargo test -p agentstacks host::payload host::values` passes and
`cargo make dod-crate agentstacks` is green. Commit
`feat(agentstacks): extension payloads and values result`.

### Task 12 — Run a tool through the `Adapter` port

**Files:**
- Create `crates/agentstacks/src/host/session.rs` — `Session`
- Create `crates/agentstacks/src/host/adapter.rs` — `AirslAdapter` implementing `Adapter`
- Modify `crates/agentstacks/src/host/mod.rs` (`pub use adapter::AirslAdapter`), `host/testing.rs`

**Change:** `Session::open(extension, files)`:

1. Builds the engine (Task 2) for one tool, with a fresh `RunState`.
2. Runs the entry file at the top level as `Script::from_source(entry_text, entry_path)` under
   `Phase::Load`. Entry-run failures go through `wrap` (§4.4 flow step 1).

`Session::values(&ValuesInput)` sets `Phase::Values`, dispatches `values`, and returns
`values_result` (§4.4 step 2). `Session::plan(&PlanInput, workspace)`:

1. builds the `ReadSnapshot` (Task 9);
2. seeds the run's id index from the payload;
3. sets `Phase::Plan`, dispatches `plan`, then returns to `Phase::Load`;
4. returns `NoHandler` as `registers no plan handler` (§4.4) and ignores any other return value;
5. returns the recorded `Proposals` and the reads record, or every refusal (sticky ones plus a
   wrapped failure).

`AirslAdapter` implements plan 09's `Adapter` by mapping its calls onto `Session`. The same engine
serves `values` and then `plan`, because the handlers are registered once by the entry run. A
removed tool (D8, §6.4) is the same run with an empty `packs` list. Any extension error refuses the
whole run (§4.6 "Any extension error refuses the whole plan"). The host never writes.

**Proof:**
- `full_run_values_then_plan` — an entry with both handlers. `values` returns the §4.4 example
  result; `plan` copies `core/skill/status/SKILL.md` and skips `core/agent/coder`. The adapter
  returns both proposals with origins.
- `missing_plan_handler_is_refused` — an entry with only `values` gives
  `claude/adapter.lua: registers no plan handler`.
- `plan_return_value_is_ignored` — a `plan` handler returning `{x = 1}` yields the same proposals
  as one returning nothing.
- `removed_tool_runs_with_zero_packs` — a `PlanInput` with an empty `packs` list dispatches, and
  only the handler's shared-file write comes back.
- `entry_failure_refuses_the_run` — an entry with a syntax error gives the §4.6 row 3 message, and
  neither handler runs.
- `fresh_state_per_tool` — two sessions for two tools: a global set by the first is `nil` in the
  second, and their proposals do not mix.

**Done when:** `cargo test -p agentstacks host::` passes and `cargo make dod` is green. Commit
`feat(agentstacks): run tool extensions through the airsl adapter`.

---

## Risks

- **`on` forwarding error text.** `agentstacks.on` is a Rust function calling the Lua function
  `airsstack.ext.on`, so airsl's `UnknownEvent` comes back wrapped in a callback error. The probe
  proved airsl's text for a direct `airsstack.ext.on` call (R1b), not through a forwarder. If the
  first line of the wrapped error differs, the forwarder returns the inner cause, so that
  `unknown_event_is_airsl_text_with_sorted_events` holds as written. The test pins the text either
  way.
- **Raised detection depends on the error chain.** `Raised` is found by walking the `airsl::Error`
  source chain with `downcast_ref`, the way airsl's own `exhausted_memory` walks it
  (`engine.rs:405-412`). A refusal rethrown by Lua after `pcall` keeps the same wrapped error
  value. `uncaught_op_refusal_is_reported_once` guards against a double report.
- **Pre-reading `reads`.** The snapshot is taken just before the `plan` dispatch, rather than at
  each `read` call, because Lua functions must be `'static`. Within one plan run the two are the
  same. `app/apply` re-checks disk state against the reads record (§7 step 3).
- **Plan return value conversion.** `Engine::dispatch` converts the handler's return value to JSON
  even though `plan` ignores it, so a `plan` handler returning a function fails as a Lua failure.
  airsl decides this, and the plan accepts it.
- **Cross-plan interface.** This plan needs three things from plan 09's `Adapter` signature:
  - one engine kept alive across `values` and `plan` for a tool;
  - the extension's `FileTree`, so the `.lua` files can be read into memory, because `Extension`
    keeps no file contents;
  - a `Workspace` for `read`.

  It also needs the earlier tools' and the core's claimed paths, so the cross-tool duplicate rule
  (`… already proposed by opencode` / `by core (…)`) fires at call time, as §3.3 requires. The
  orchestrator reconciles this with plan 09 before execute.
- **Spec gaps** (to settle in the spec before execute; nothing above invents an answer to them):
  - G1: the boundary between host and plan 04 for the values result (who walks the nested JSON;
    plan 04 needs the `json_word` mapping);
  - G2: no refusal for a non-UTF-8 `.lua` file (`Script::from_source` needs a `String`);
  - G3: the caller location when the immediate caller is a C frame, as in
    `pcall(agentstacks.write, t)`;
  - G4: the display of an uncaught `json.edit` / `require` refusal (this plan uses the located
    message), and whether `require` refusals are sticky;
  - G5: the `invalid module name` hint for names whose fault is not `/`;
  - G6: the cycle example's location (`lib/a.lua:1`) versus "location: the caller" (`lib/b.lua`);
  - G7: missing messages for non-string `require` / `json.edit` arguments beyond §5 row 1, a
    non-list `json.edit` path table, non-UTF-8 `json.edit` text, a non-string key in an op table,
    `-inf`, and a non-UTF-8 key inside a `json.edit` value;
  - G8: a self-referencing table passed to `json.edit` (no refusal, and the conversion recurses);
  - G9: an I/O failure while reading a `reads` path (no message);
  - G10: the order of checks within one op call.

## Proof

- `cargo make dod` is green at the end of Task 12. `cargo test -p agentstacks host::` runs every
  test above on a real airsl 0.1.4 engine.
- `rg -n "dyn " crates/agentstacks/src/host` lists only the `ModuleSet::insert` calls in
  `engine.rs`, under the `// dyn:` comment.
- `rg -n "std::fs|std::process|std::net" crates/agentstacks/src/host` prints nothing. The host reads
  only through `FileTree` and `Workspace`.
- Spec rows covered:
  - §4.1: steps 1–3 (Task 1, Task 12).
  - §4.2: the set, Absent, Present, removal, limits, budget reset, breach messages, 8 MiB, caught
    breach (Tasks 2, 4, 9).
  - §4.3: every row of the function table, and the `read` rules (Tasks 2, 4, 6, 8, 9, 10).
  - §4.4:
    - the flow (Task 12);
    - the three dispatch results (Task 4);
    - the second registration (Task 4);
    - both payloads (Task 11);
    - the values example and the values rule table: rows 1–2 here, and rows 3–7 through plan
      04's `ValuesResult` with the host's words (Task 11);
    - the missing plan handler and the ignored return value (Task 12).
  - §4.5: rows 1–3 (Task 10), the cache, failed-not-cached, the module file and line, and `load`
    withheld (Task 10). Row 4 waits on G6.
  - §4.6: rows 1–5 and "refuses the whole plan" (Tasks 4, 6, 8, 12).
  - §5 second table: every row (Task 5), plus the argument rows and the checked example (Task 6).
  - §3.3:
    - the reading rows: argument, required, type, list, unknown key, UTF-8, raw content, empty
      content, empty covers (Task 7);
    - phase and the `read` rows (Tasks 8, 9);
    - stickiness (Task 8);
    - location in a handler and in a module (Tasks 3, 8).
  - §13 rows 1–3: location shapes (Tasks 3, 4, 11).
