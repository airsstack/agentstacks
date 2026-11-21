---
status: approved
created: 2026-11-22
depends-on: [03, 04, 05, 09]
built-on: f39b2a814319
---

# Extension host Implementation Plan

**Goal:** Run a tool extension's adapter in the airsl sandbox.

**Architecture:** `src/host/` is the adapter behind the `Adapter` port (plan 09). It is the only
place Lua runs (spec §2). It translates, and decides nothing that `types/` already decides:

- Lua values become `Operation`, `Proposal` and `ValuesResult` (plan 04) through those types' own
  constructors.
- JSON edits go through `types::json::edit` (plan 05).

The host owns only what needs Lua: reading arguments strictly as `mlua::Value`, the caller's
location, module loading, dispatch, the §5 Lua-value conversion, and wrapping airsl's errors. Each
file has one job:

```text
src/host/
├─ mod.rs        docs + `mod` / `pub use` only (export-only)
├─ adapter.rs    AirslAdapter: implements plan 09's Adapter (type Session = host::Session)
├─ session.rs    Session: one tool's engine, kept alive from the entry run through values to plan (§4.4 flow)
├─ engine.rs     builds the airsl Engine: Policy::pure + the explicit module set (the one `dyn`)
├─ module.rs     AgentstacksModule: the HostModule that installs the `agentstacks` table (§4.2, §4.3)
├─ state.rs      RunState: phase, the Proposals on loan, sticky refusals, Reads, require bookkeeping
├─ caller.rs     the nearest Lua frame as `<tool>/<file>:<line>` (§3.3, §13, R5)
├─ args.rs       strict reading of one Lua argument table: required, exact type, list, keys (§3.3, R3)
├─ ops.rs        write · copy · skip · require_binary (§4.3) into plan 04's Proposals
├─ read.rs       agentstacks.read over a pre-read snapshot of extension.reads (§4.3)
├─ require.rs    agentstacks.require over the in-memory files (§4.5, R19)
├─ events.rs     agentstacks.on forwarding + dispatch outcome: none / nil / table (§4.3, §4.4)
├─ values.rs     the values result → plan 04's ValuesResult::read (§4.4)
├─ payload.rs    ValuesInput / PlanInput (plan 04) → the §4.4 JSON payloads
├─ convert.rs    Lua value → JSON (§5 second table, R3); JSON value → LuaType word (§4.4)
├─ json.rs       agentstacks.json.edit: argument reading, conversion, call into types::json::edit (§5)
├─ failure.rs    airsl::Error → a located Diagnostic; the marker for the host's own raised refusals (§4.6)
├─ files.rs      LuaFiles: every `.lua` file of the extension in memory, module name ↔ path (§4.1, §4.5)
└─ testing.rs    #[cfg(test)] helper: open a Session from inline Lua files on a FakeFileTree
```

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6):

- `modularity.md`:
  - Rule 1: one job per file, as listed above.
  - Rule 2: the host keeps no copy of `Operation`, `OpPath`, `ItemId`, `Origin`, `Proposals`,
    `ValuesResult`, `Reads`, `ReadOutcome`, `Resolution`, `OpError`, `LuaType` or `JsonEditError`.
    It builds the canonical types.
- `static-dispatch.md`:
  - `AirslAdapter`'s methods are generic over the ports plan 09 names (`T: FileTree` on `open`,
    `W: Workspace` on `plan`).
  - The one `dyn` is in `engine.rs`: `ModuleSet::insert(Box<dyn HostModule>)`, which airsl's API
    requires (`airsl-0.1.4/src/modules/registry.rs:144`). It is marked `// dyn: airsl's ModuleSet
    stores trait objects` (spec §2).
- `mod-rs-export-only.md`: `host/mod.rs` holds its docs and `mod` / `pub use` lines only.
- `unit-test-mandate.md`: every logic file has a colocated `#[cfg(test)] mod tests`, and
  `testing.rs` is a test-only helper module.
- `strong-types.md`: the phase is an enum (`Phase::{Load, Values, Plan}`), never a `bool`. Module
  names and extension file paths are a newtype in `files.rs` and a `TreePath`, never bare `String`.
- `doc-comment-discipline.md`: doc comments describe the sandbox and the API, and never name plans,
  specs, chains or agents.
- Fakes are `#[cfg(test)]` (R24, plan-level): no doc-comment example in `host/` uses a fake. The
  `AirslAdapter` example opens the existing fixture `tests/fixtures/extensions/valid/minimal`
  (`adapter.lua` is `return {}`) through `fs::StdFileTree`. It shows that `values` is refused with
  `claude/adapter.lua: registers no values handler`.
- `mlua` is reached only as `airsl::mlua`, the re-export that pins the engine's version
  (`airsl-0.1.4/src/lib.rs:56`). No separate `mlua` dependency is added.
- Dependency rule (§2): `host` imports only `port` and `types`. Its only cross-boundary type is the
  `Workspace` / `FileTree` bounds.

**Tech Stack:** Rust 2024, `airsl = "0.1.4"` (Engine, Policy, ModuleSet, HostModule, Ext, Json,
EventName, Script, `airsl::mlua`), `serde_json` 1.0.154 with `preserve_order` (both added by plan 01).

**Content authority:** spec §4.1–§4.6, the §5 Lua-value table, the reading half of §3.3 (§13 row 1:
"`host` (reading) + `types/op` (rule)"), §14 row `host`, and §16 rows R1, R3, R5, R11, R12, R13, R28 (a, b, d, l, m) and
R19.

---

## Files that change

```
crates/agentstacks/src/host/mod.rs      — modify: module docs (four questions), mod lines, pub use AirslAdapter, Session
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

Every test below runs a real airsl 0.1.4 engine on inline Lua (spec §14 `host` row). The exceptions
are the pure conversion and reader tests in Tasks 5 and 7, which use Lua values made by evaluating
inline chunks on an `airsl::mlua::Lua` state. Expected messages are copied verbatim from the cited
spec row. Each message's enum variant is plan 02's (`OpError`, `JsonEditError`) or plan 04's; the
host never formats refusal text itself.

**One refusal per call (R5).** A host Lua function raises one refusal: the first failing check, in
the order of its spec table (§3.3 for ops and `read`, §4.5 for `require`, §5 for `json.edit`; R14
"the first row of the §5 table wins"). Its location is the nearest Lua frame (Task 3).

### Task 1 — Load the extension's Lua files into memory

**Files:**
- Create `crates/agentstacks/src/host/files.rs` — `LuaFiles`
- Create `crates/agentstacks/src/host/testing.rs` — test-only builder for an in-memory extension
- Modify `crates/agentstacks/src/host/mod.rs` — `mod files;` (+ `#[cfg(test)] mod testing;`)

**Change:** `LuaFiles::read<T: FileTree>(tree, extension) -> Result<LuaFiles, Diagnostics>` walks the
extension tree once with the built `types::tree::walk`. It reads every walked file ending in `.lua`
with `FileTree::read_file`, so Lua never touches disk (§4.1 step 3). Each file must be UTF-8
(`Script::from_source` takes a `String`). A file that is not is refused at
`Location::file("<tool>/<file>")` with the text `not valid UTF-8` (R3: `<tool>/<file>: not valid
UTF-8`). Every such file is reported, not only the first.

An I/O failure reading a file is `<path>: <io::Error Display>` at that file (R1).

`LuaFiles` exposes:

- `entry()`: the entry file's normalised path (`ExtensionManifest::entry().normalized()`) and its
  text;
- `module(name)`: maps a module name to `<name with . replaced by />.lua` (§4.5) and returns that
  file's path and text, or none;
- `entry_module()`: the module name of the entry file (`adapter.lua` → `adapter`,
  `src/main.lua` → `src.main`), used by the entry rule of §4.5.

The `testing.rs` helper builds a `FakeFileTree` holding `extension.toml` (tool `claude`, `api = 0`,
`entry = "adapter.lua"`, optional `reads`) plus inline `.lua` files. It loads the tree with
`Extension::load`. Task 12 adds `AirslAdapter::open` on it; until then it returns the `LuaFiles` and
the `Extension`.

**Proof:**
- `reads_every_lua_file_of_the_extension` — a tree with `adapter.lua`, `lib/util.lua` and
  `README.md` yields exactly the two `.lua` paths, with their text unchanged.
- `maps_a_dotted_module_name_to_its_file` — `lib.util` finds `lib/util.lua`; `lib.nope` finds none.
- `entry_module_drops_the_extension_and_uses_dots` — entry `adapter.lua` gives `adapter`; entry
  `src/main.lua` gives `src.main`.
- `non_utf8_lua_file_is_refused` — `lib/bad.lua` holding `[0xff]` gives
  `claude/lib/bad.lua: not valid UTF-8` (R3).
- `unreadable_lua_file_is_an_io_refusal` — a `FakeFileTree::unreadable("lib/x.lua")` gives
  `<path>: <io::Error Display>` at that file (R1). The assertion builds the expected text from the
  fake's own error.

**Done when:** `cargo test -p agentstacks host::files` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): read an extension's lua files into memory`.

### Task 2 — Build the sandbox

**Files:**
- Create `crates/agentstacks/src/host/engine.rs` — `build_engine(module) -> airsl::Result<Engine>`
- Create `crates/agentstacks/src/host/module.rs` — `AgentstacksModule` (`HostModule`)
- Create `crates/agentstacks/src/host/state.rs` — `RunState`, `Phase`
- Modify `crates/agentstacks/src/host/mod.rs`, `host/testing.rs` — mod lines; the helper evaluates inline Lua

**Change:** (§4.2)

- `build_engine` is `Engine::builder().policy(Policy::pure()).stdlib(set).build()`. `set` is a
  `ModuleSet` holding exactly three modules: `Json::new()`, `Ext::with_events([values, plan])`
  (two `EventName` constants), and `AgentstacksModule`. They are inserted with
  `ModuleSet::insert(Box<dyn HostModule>)` under the `// dyn: airsl's ModuleSet stores trait
  objects` comment. The set is always explicit, because `pure()` alone installs stdio, hook and time
  (airsl#11, §4.2).
- `AgentstacksModule` is installed as `airsstack.agentstacks`. Its `install`:
  - sets the global `agentstacks` to that same table;
  - sets the globals `print` and `warn` to nil (proven for `print`, Q2 `OK "nil"`);
  - fills the table with `on`, `require`, `write`, `copy`, `skip`, `require_binary`, `read`, and
    `json.edit`. Their bodies come from Tasks 4 and 6–10; this task installs the keys.
- `RunState` is shared as `Arc<Mutex<…>>`. This is needed because `HostModule: Send + Sync`, and
  the module is stored as `Box<dyn HostModule>`, so it is `'static` (registry.rs:97). It holds:
  - the tool name, the `Phase` (`Load` while the entry runs, then `Values`, then `Plan`), and the
    plan payload's id index;
  - the `Proposals` on loan for a `plan` dispatch (Task 12);
  - the sticky refusals (`Diagnostics`);
  - the `Reads` record (plan 06);
  - the read snapshot;
  - the require loading stack and loaded set.

  Lua functions capture clones of the `Arc`. Nothing captures a borrowed port or a borrowed
  `Proposals`.

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

### Task 3 — Locate the nearest Lua frame

**Files:**
- Create `crates/agentstacks/src/host/caller.rs` — `Caller { file: TreePath, line: NonZeroUsize }`, `caller(lua) -> Option<Caller>`, `Caller::location(tool)`, `Caller::origin(tool)`
- Modify `crates/agentstacks/src/host/mod.rs`

**Change:** `caller` walks `lua.inspect_stack(level, …)` upward from level 1 (§3.3,
`probes/round2-results.md` R5). It stops at the first frame that has a current line, which is the
nearest Lua frame (R5). So `pcall(agentstacks.write, t)`, whose level 1 is the C function `pcall`,
is located at the `pcall` line.

- The file is the frame's `source` without its leading `@`. `source` is used rather than
  `short_src`, which Lua truncates for long names. For the probes' names the two are equal (R5 full
  stack: `source=Some("@adapter.lua") short_src=Some("adapter.lua")`).
- `Caller::location` gives `Location::at_line("<tool>/<file>", line)` (§13 row 1).
- `Caller::origin` gives plan 04's `Origin::Extension { tool, file, line }`.

A required module runs under the chunk name `@<path>` (Task 10). So a call made inside a module
names that module (§3.3, `{"at":"lib/settings.lua:3","here":"adapter.lua:3"}` in
`probes/examples-results.txt`).

**Proof:**
- `names_the_entry_file_and_line_at_top_level` — a host function called at line 3 of `adapter.lua`
  yields `claude/adapter.lua:3`.
- `names_the_line_inside_a_dispatched_handler` — a call on line 2 of a `plan` handler yields
  `claude/adapter.lua:2` (§3.3, `probes/round3-results.md` A4).
- `names_the_required_module_not_the_entry` — a call made inside a function defined in
  `lib/settings.lua` (loaded under the chunk name `@lib/settings.lua`) yields
  `claude/lib/settings.lua:3`.
- `skips_a_c_frame_to_the_pcall_line` — `pcall(f, x)` on line 4, where `f` is the host function,
  yields `claude/adapter.lua:4` (R5).

**Done when:** `cargo test -p agentstacks host::caller` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): locate the calling lua frame`.

### Task 4 — Forward `agentstacks.on`, dispatch, and wrap Lua failures

**Files:**
- Create `crates/agentstacks/src/host/events.rs` — the `on` function; `dispatch(engine, event, payload) -> Dispatched`
- Create `crates/agentstacks/src/host/failure.rs` — `Raised` marker; `wrap(tool, entry, error) -> Option<Diagnostic>`
- Modify `crates/agentstacks/src/host/module.rs`, `host/mod.rs`

**Change:**

- **`agentstacks.on(event, fn)`.** It looks up `airsstack.ext.on` from the globals **when
  called**, never at install. The global `airsstack` does not exist while modules install
  (`install: global airsstack is nil`, `probes/examples-results.txt`). It forwards both arguments
  unchanged and returns airsl's error to Lua unchanged. A later registration replaces the earlier
  one and takes effect at the next dispatch (D5, §4.3, `probes/round3-results.md` A1).
- **`dispatch`.** It calls `Engine::dispatch` and returns `Dispatched::{NoHandler, Nil,
  Value(serde_json::Value)}` for `Ok(None)`, `Ok(Some(Null))` and `Ok(Some(other))` (§4.4, A3,
  R1b).
- **`failure::Raised`.** This is the error type the host's own Lua functions raise:
  `mlua::Error::external(Raised { diagnostic, sticky })`. Its `Display` is the diagnostic's text, so
  Lua code that catches it with `pcall` sees the located message.
- **`wrap`.** It turns an `airsl::Error` from an entry run or a dispatch into the diagnostic the
  user sees (§4.6). It walks the error's source chain looking for a `Raised`:
  - A sticky `Raised` (an op or `read` refusal, already recorded, Tasks 8 and 9) adds nothing more.
  - A non-sticky `Raised` (an uncaught `require` or `json.edit` refusal) gives its own located
    diagnostic, not the §4.6 wrapper (R19).
  - Anything else gives `Location::file("<tool>/<entry>")` with plan 02's Lua-failure variant. That
    variant carries the **first line** of the error's `Display` as text. This also covers
    `InstructionLimit` and `MemoryLimit` (§4.6 rows 3–5).

**Proof:**
- `on_forwards_to_ext_on_at_call_time` — the entry registers `values` through `agentstacks.on`, and
  dispatch returns the handler's table.
- `second_registration_replaces_the_first` — two `values` registrations: dispatch returns
  `{"which":"second"}` (§4.4, A1).
- `registration_inside_values_takes_effect_at_plan` — a `plan` handler registered from the `values`
  handler runs on the `plan` dispatch (§4.3).
- `no_handler_is_distinct_from_nil_and_table` — no handler → `NoHandler`; `return nil` → `Nil`;
  `return {a=1}` → `Value({"a":1})` (§4.4).
- `unknown_event_is_airsl_text_with_sorted_events` — `agentstacks.on("nope", f)` at line 1 gives
  `claude/adapter.lua: lua error in adapter.lua: event `nope` is not one this host dispatches;
  declared events: plan, values` (§4.6 last row).
- `syntax_error_shows_first_line` — entry `local function f(` gives `claude/adapter.lua: lua error in
  adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>` (§4.6 row 3, Q5a).
- `runtime_error_in_plan_names_the_event_and_drops_the_traceback` — a `plan` handler that calls
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

**Change:** This is the hand-written conversion of the §5 second table plus R3. mlua's serde
conversion drops data (`convert({1, a=2})` → `[1]`, `probes/round3-results.md` A4). Tables are read
with raw access only, so no metamethod runs inside the host.

- A table whose keys are exactly the integers `1..n` becomes an array, in index order.
- A table with only string keys becomes an object, with its keys sorted by bytes. A key that is not
  UTF-8 is refused (R3: `json.edit: value: key is not valid UTF-8`).
- An empty table becomes `{}` (`airsl-0.1.4/src/convert.rs:41`).
- Any other table is refused with "table mixes list and keyed entries" (§5).
- A table that is already on the current conversion path is refused (R3: `json.edit: value: table
  contains itself`). The same table reached twice by different keys is not a cycle, and is
  converted twice.
- An integer becomes a number from `i64`, and a float a number from `f64`. NaN, `inf` and `-inf`
  are refused (§5; R3 for `-inf`).
- A UTF-8 string becomes a string; any other string is refused (§5).
- A boolean becomes a boolean.
- A function, userdata or thread is refused with its word (R3: `json.edit: value: <type> cannot be
  JSON`).

`json_word` maps a dispatched JSON value back to the word the Lua author wrote (§4.4): object →
`table`, array → `list`, null → `nil`, number → `number`, string → `string`, boolean → `boolean`
(plan 02's `LuaType`). It is the word function handed to plan 04's `ValuesResult::read` (Task 11).
§13 makes host and `types/op` the joint owners.

**Proof:** Each refusal is shown as it renders with the `json.edit: ` prefix that Task 6 adds.
- `sequence_becomes_array` — `{1, "a", true}` → `[1,"a",true]`.
- `string_keys_become_sorted_object` — `{zeta=1, alpha=2}` → `{"alpha":2,"zeta":1}`, in that
  order.
- `empty_table_becomes_empty_object` — `{}` → `{}`.
- `mixed_table_is_refused` — `{1, a=2}` → `json.edit: value: table mixes list and keyed entries`.
- `sparse_integer_keys_are_refused` — `{[1]=1, [3]=3}` gives the same message ("any other
  table").
- `self_containing_table_is_refused` — `local t = {}; t.me = t` → `json.edit: value: table
  contains itself` (R3).
- `shared_subtable_is_not_a_cycle` — `local s = {1}; return {a = s, b = s}` →
  `{"a":[1],"b":[1]}`.
- `integer_and_float_become_numbers` — `3` → `3`, and `0.5` → `0.5`.
- `nan_is_refused` — `0/0` → `json.edit: value: NaN cannot be JSON`.
- `inf_is_refused` — `math.huge` → `json.edit: value: inf cannot be JSON` (§5 "(also `inf`)").
- `negative_inf_is_refused` — `-math.huge` → `json.edit: value: -inf cannot be JSON` (R3).
- `non_utf8_string_is_refused` — `"\255"` → `json.edit: value: string is not valid UTF-8`.
- `non_utf8_key_is_refused` — `{["\255"] = 1}` → `json.edit: value: key is not valid UTF-8` (R3).
- `function_is_refused` — `function() end` → `json.edit: value: function cannot be JSON`.
- `thread_and_userdata_are_refused` — a thread and a userdata made in the test state give
  `json.edit: value: thread cannot be JSON` and `json.edit: value: userdata cannot be JSON` (R3).
- `nested_values_convert_recursively` — `{ { matcher = "Read" } }` → `[{"matcher":"Read"}]`.
- `json_words_match_lua_words` — object, array, null, number, string and boolean map to `table`,
  `list`, `nil`, `number`, `string` and `boolean`.

**Done when:** `cargo test -p agentstacks host::convert` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): convert lua values to json`.

### Task 6 — Expose `agentstacks.json.edit`

**Files:**
- Create `crates/agentstacks/src/host/json.rs` — the `json.edit` Lua function
- Modify `crates/agentstacks/src/host/module.rs`, `host/mod.rs`

**Change:** `agentstacks.json.edit(text, path, value)` works in any phase (§4.3). The order of the
§5 rows belongs to plan 05's `edit` (R5, R14). The host checks only the argument row (§5 row 1):

- `text` must be a string: `json.edit: text: expected string, got <word>`;
- `path` must be a table: `json.edit: path: expected table, got <word>`.

Past that row the host refuses nothing itself. It calls
`types::json::edit(text, path: &[PathElem], value)` (plan 05) as follows:

- `text`: the Lua string as `edit` takes it.
- `path`: each entry of the path table becomes a plan 05 `PathElem`, built **without** refusing a
  bad element. `edit` then decides, in its row order, the rows for:
  - a path that is not a list (R3: `json.edit: path: expected a list`);
  - an empty path (§5 row 5: `json.edit: path: empty`);
  - an element that is not a string (§5 row 5: `json.edit: path[<i>]: expected string, got
    <word>`).
- `value`: a closure `FnOnce() -> Result<Option<serde_json::Value>, JsonEditError>` that runs the
  §5 Lua-value conversion (`lua_to_json`, Task 5), where `nil` gives `None` (remove). `edit` calls
  it at its own point in the row order, so a conversion refusal never pre-empts an earlier row.

A refusal, from the argument row or from `edit`, is raised as a **non-sticky** `Raised`. It is
located at the caller (Task 3) and carries `json.edit: ` plus the `JsonEditError` text (§5, §13 row
1). It is an ordinary Lua error (R19, §4.6 row 2):

- `pcall` catches it, and nothing is recorded;
- uncaught, it fails the run with its located message.

**Proof:**
- `checked_example_is_byte_for_byte` — the §5 checked example run through Lua returns exactly the
  §5 output block (4-space indent, trailing newline kept). This is the same text
  `probes/examples-results.txt` reports `true` for.
- `text_must_be_a_string` — `agentstacks.json.edit(nil, {"a"}, 1)` on line 1 gives
  `claude/adapter.lua:1: json.edit: text: expected string, got nil` (§5 row 1).
- `text_must_be_utf8` — `text = "\255"` gives `claude/adapter.lua:1: json.edit: text: not valid
  UTF-8` (R3, decided by `edit`).
- `path_must_be_a_table` — `path = "a"` gives `claude/adapter.lua:1: json.edit: path: expected
  table, got string` (§5 row 1).
- `path_must_be_a_list` — `path = {a = "x"}` with valid text gives `claude/adapter.lua:1: json.edit:
  path: expected a list` (R3, decided by `edit`).
- `path_must_not_be_empty` — `{}` with valid text gives `claude/adapter.lua:1: json.edit: path:
  empty` (§5 row 5, decided by `edit`).
- `path_elements_must_be_strings` — `{"a", 2}` gives `claude/adapter.lua:1: json.edit: path[2]:
  expected string, got number` (§5 row 5, decided by `edit`).
- `argument_row_wins` — `text = nil` together with `path = {}` gives only the `text` refusal (§5
  row 1, R14).
- `parse_refusal_precedes_path_rows` — `agentstacks.json.edit("{,", {}, 1)` gives the parse
  refusal: `claude/adapter.lua:1: json.edit: ` followed by the serde_json message that plan 05's
  `edit` returns for `"{,"`, not `path: empty` (§5 rows 3 and 5, R14).
- `value_is_not_converted_before_earlier_rows` — `agentstacks.json.edit("{,", {"a"}, {1, a=2})`
  gives the parse refusal, not the value refusal, because the conversion closure is never called.
- `value_refusal_is_located` — `value = {1, a=2}` gives `claude/adapter.lua:1: json.edit: value:
  table mixes list and keyed entries`.
- `edit_refusal_passes_through_located` — `text = '{"hooks": 1}'` with path `{"hooks","x"}` gives
  `claude/adapter.lua:1: json.edit: "hooks" is not an object` (§5 row 6, plan 05's message). The
  same holds with `value = nil` (R14: removing through a non-object parent refuses).
- `nil_value_removes_the_key` — removing `a` from `{"a":1,"b":2}` returns `{"b":2}` in the
  two-space layout. Removing a missing key returns the input text unchanged (§5 row 7).
- `refusal_is_catchable_with_pcall` — `pcall(agentstacks.json.edit, nil, {"a"}, 1)` returns
  `false`, the entry run succeeds, and no refusal is recorded (R19).
- `uncaught_refusal_shows_its_located_message` — an uncaught `json.edit` refusal on line 2 of a
  `plan` handler fails the run with exactly
  `claude/adapter.lua:2: json.edit: text: expected string, got nil`, with no wrapper (R19).

**Done when:** `cargo test -p agentstacks host::json` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): agentstacks.json.edit for extensions`.

### Task 7 — Read op arguments strictly

**Files:**
- Create `crates/agentstacks/src/host/args.rs` — `ArgTable` over one `mlua::Value`: `string(field)`, `bytes(field)`, `string_list(field)`, `finish()` (key checks)
- Modify `crates/agentstacks/src/host/mod.rs`

**Change:** This is the reading half of §3.3, with no coercion. airsl's raw reading turned
`path = 5` into `"5"`, and serde reading accepted unknown keys (Q3). So the reader matches
`mlua::Value` variants by hand. It returns the **first** failing check, in §3.3 table order (R5).
Each check runs over every field, in the function's field order (`path`, `content`, `covers` /
`file` / `item`, `reason` / `name`), before the next check starts:

1. **Argument shape.** The argument must be a table (`expected table, got <word>`, §3.3 row 1).
2. **Required.** `nil` counts as missing (`<field>: required`, row 2).
3. **Type.** A field's variant must match exactly (`<field>: expected string, got <word>`, row 3).
   Integer and float are both `number` (§3.3 "Type words").
4. **List.** `covers` must be a table whose keys are exactly `1..n`, otherwise `covers: expected a
   list` (row 4). Each element must be a string (`covers[<i>]: expected string, got <word>`). An
   empty `covers` is the same as none (§3.3 "Accepted").
5. **Keys.** Once the named fields are read, a non-string key gives `keys must be strings, got
   <word>` (R3), and any other key gives `<key>: unknown key` (row 5).
6. **UTF-8.** Every string field except `content` must be UTF-8: `path: not valid UTF-8` (row 6),
   and the same shape for the others (R3: `<fn>: <field>: not valid UTF-8`).
7. **Content.** `content` keeps its raw bytes (`as_bytes=[97, 255, 98, 0, 99]`, R4), and an empty
   `content` is accepted.

The words are plan 02's `LuaType`. The texts are plan 02's `OpError` variants for the §3.3 rows
and R3.

**Proof:** These are reader-level tests on Lua values; Task 8 adds the caller prefix.
- `argument_must_be_a_table` — `"x"` → `expected table, got string` (§3.3 row 1).
- `nil_field_counts_as_missing` — `{content = "x"}` read for `path` → `path: required` (row 2).
- `no_coercion_of_numbers` — `{path = 5}` → `path: expected string, got number` (row 3).
- `float_is_also_number` — `{path = 5.5}` → `path: expected string, got number`.
- `covers_must_be_a_table` — `{covers = "x"}` → `covers: expected table, got string`.
- `covers_must_be_a_list` — `{covers = {a = "x"}}` → `covers: expected a list` (row 4).
- `covers_elements_must_be_strings` — `{covers = {"a", true}}` → `covers[2]: expected string, got
  boolean`.
- `empty_covers_is_none` — `{covers = {}}` reads as no covers.
- `unknown_key_is_refused` — `{path = "a", content = "", foo = 1}` → `foo: unknown key` (row 5).
- `non_string_key_is_refused` — `{path = "a", content = "", [1] = "x"}` → `keys must be strings,
  got number` (R3).
- `path_must_be_utf8` — `{path = "a\255"}` → `path: not valid UTF-8` (row 6).
- `other_string_fields_must_be_utf8` — `{item = "a\255", reason = "r"}` read for `item` →
  `item: not valid UTF-8` (R3).
- `content_keeps_raw_bytes` — `content = "a\255b\0c"` → bytes `[97, 255, 98, 0, 99]`.
- `empty_content_is_accepted` — `content = ""` → zero bytes.
- `first_failing_check_wins` — `{foo = 1}` read for `path` and `content` returns only
  `path: required`. The required check (row 2) comes before the unknown-key check (row 5), and
  `path` comes before `content` (R5).
- `required_row_precedes_type_row` — `{path = 5}` read for `path` and `content` returns
  `content: required`, not the type refusal for `path` (R5).

**Done when:** `cargo test -p agentstacks host::args` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): strict reading of extension arguments`.

### Task 8 — Record operations during the plan dispatch

**Files:**
- Create `crates/agentstacks/src/host/ops.rs` — `write`, `copy`, `skip`, `require_binary` Lua functions
- Modify `crates/agentstacks/src/host/module.rs`, `host/state.rs`, `host/mod.rs`

**Change:** Each op function (§4.3) does the following:

1. It reads the caller (Task 3).
2. Outside `Phase::Plan` it refuses with `only allowed inside the plan handler` (§3.3 last row).
   This check runs before every other check (R28(a)).
3. It reads its fields with `ArgTable` (Task 7):
   - `write`: `path`, `content`, optional `covers`;
   - `copy`: `path`, `file`;
   - `skip`: `item`, `reason`;
   - `require_binary`: `name`.
4. It runs the rest of the §3.3 table in row order (R5), through plan 04's types, on the
   `Proposals` on loan in `RunState`. That `Proposals` already holds the core's writes and every
   earlier tool's proposals (plan 09's `Adapter::plan` contract, R6), so every check below fires at
   call time, against the same tool, another tool or the core:
   1. rows 7–8: `OpPath::proposed` checks the path shape, normalises it and refuses agentstacks'
      own files (R25: `.` is `path: empty`);
   2. row 9: `Proposals::check_path` checks that the path is not already proposed. This comes
      **before** the id rows;
   3. row 10: the item-id checks against the run's plan payload, including R13. The two refusals
      are `copy: file "core/agent/coder" is not a file` and, carrying the function prefix
      (R28(m)), `write: covers[1]: "core/skill/status" is a skill; list its files`;
   4. row 11: `skip.reason` must not be empty;
   5. row 12: `BinaryName`;
   6. last, `Proposals::propose` with `Proposal { op, origin: Caller::origin(tool) }`. It applies
      the skip/cover-conflict rules (§3.2) and records the op. It returns `Result<(), OpError>`,
      one refusal.

The first failing check (R5) becomes a `Diagnostic` at `<tool>/<file>:<line>` with the text
`<function>: <what>`.
The diagnostic is recorded in `RunState` when it happens (**sticky**, §3.3 "Refusals stick") and
raised as a sticky `Raised`. A refused op is not recorded. A plan with a sticky refusal is refused
even when the Lua code catches the error with `pcall`.

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
- `copy_of_a_non_file_is_refused` — `agentstacks.copy{path="a", file="core/agent/coder"}` gives
  `…: copy: file "core/agent/coder" is not a file` (R13).
- `covers_naming_a_skill_group_is_refused` — `agentstacks.write{path="a", content="",
  covers={"core/skill/status"}}` on line 7 gives `claude/adapter.lua:7: write: covers[1]:
  "core/skill/status" is a skill; list its files` (R13, R28(m)).
- `cover_after_skip_names_the_skip` — a skip of `core/agent/coder` at line 9, then a write covering
  it at line 14, give `claude/adapter.lua:14: write: covers "core/agent/coder", which adapter.lua:9
  skipped` (§3.2 table row 2, R28(l)).
- `duplicate_path_names_the_first_call` — two writes to `.claude/x`, at lines 9 and 14, give
  `claude/adapter.lua:14: write: path ".claude/x" is already proposed at adapter.lua:9` (§3.3 row 9).
- `duplicate_path_precedes_id_rows` — the second write to `.claude/x`, at line 14, also covers the
  unknown id `core/agent/nope`. It gives only the duplicate refusal, `claude/adapter.lua:14: write:
  path ".claude/x" is already proposed at adapter.lua:9`, because row 9 runs before row 10 (R5).
- `duplicate_against_an_earlier_tool_names_it` — a `Proposals` already holding `.claude/x` from
  tool `opencode`: a write to `.claude/x` on line 5 gives
  `claude/adapter.lua:5: write: path ".claude/x" is already proposed by opencode` (§3.3 row 9).
- `skip_after_copy_conflict_names_the_first_call` — a copy of a skill file at line 9, then a skip
  of its skill at line 14, give `claude/adapter.lua:14: skip: item "core/skill/status" is already
  covered at adapter.lua:9` (§3.2 table row 2).
- `refusal_sticks_under_pcall` — `pcall(function() agentstacks.write{path="/x", content=""} end)`
  in the `plan` handler: the dispatch succeeds, the refusal is still returned, and no `Write` is
  recorded (§3.3 "Refusals stick").
- `pcall_of_the_function_is_located_at_the_pcall_line` — `pcall(agentstacks.write, {path="/x",
  content=""})` on line 6 records `claude/adapter.lua:6: write: path "/x" must be relative` (R5).
- `uncaught_op_refusal_is_reported_once` — the same write without `pcall` fails the dispatch. The
  run returns exactly one diagnostic, the §3.3 message, with no Lua-failure wrapper (§4.6 row 1).
- `one_refusal_per_call` — `agentstacks.write{path=5}` gives only `…: write: content: required`.
  The required row (§3.3 row 2) comes before the type row (row 3), so the wrong-typed `path` is
  not reported (R5).
- `op_inside_required_module_names_the_module` — a write made by a function in `lib/settings.lua`
  that fails a path rule is located at `claude/lib/settings.lua:<line>` (§3.3, §13).

**Done when:** `cargo test -p agentstacks host::ops` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): record extension operations`.

### Task 9 — Serve `agentstacks.read` from the declared reads

**Files:**
- Create `crates/agentstacks/src/host/read.rs` — `ReadSnapshot` + the `read` Lua function
- Modify `crates/agentstacks/src/host/module.rs`, `host/state.rs`, `host/mod.rs`

**Change:** Lua functions are `'static`, so they cannot borrow the `W: Workspace` that
`Adapter::plan` receives. Before the `plan` dispatch, the Session therefore calls
`Workspace::resolve` (plan 09) once for every entry of `Extension::reads()` (plan 03), as an
`OpPath`. It keeps each answer in a `ReadSnapshot`:

- plan 06's `Resolution` (`Outside`, or `Inside { link, state }` with `Absent`, `File(DiskFile)` or
  `NotRegular`);
- or the `io::Error` the call returned.

`agentstacks.read(path)` (§4.3) checks the following in order (R5, with the phase check first,
R28(a)):

1. Outside `Phase::Plan`: `read: only allowed inside the plan handler` (§3.3 last row).
2. The argument must be one string (`read: expected string, got number`, §3.3 row 1).
3. The path is normalised (§3.3 "Normalisation"). A path not in `extension.reads` gives
   `read: ".env" is not in extension.reads`.
4. A path that resolves outside the project is refused at the caller's line:
   `read: ".claude/settings.json" resolves outside the project` (§4.3, §6.3).
5. A path whose snapshot holds an I/O failure gives `<path>: <io::Error Display>` at
   `Location::file(<path>)` (R1). It is reported only when the extension reads that path, and it
   sticks (R28(b)).
6. Otherwise it returns the file's bytes as a Lua string, non-UTF-8 included (Q7 `bytes=0,255,65`).
   It returns `nil` for `Absent` or `NotRegular` ("or `nil` when no regular file is there").
7. It records the answer in the run's `Reads` (plan 06): `ReadOutcome::Bytes(hash)` from the
   `DiskFile`'s hash, or `ReadOutcome::Absent` (§4.3, for §6.2 "Shared" and §7 step 3). Only paths
   actually read are recorded, and a second read of a path keeps the first outcome (plan 06's rule).

Refusals 1–5 are sticky, like op refusals: 1–4 because `read` is in §3.3's table, and 5 by
R28(b).

**Proof:**
- `read_returns_raw_bytes` — a reads path holding `[0, 255, 65]` returns a Lua string of length 3
  with those bytes.
- `read_returns_nil_for_an_absent_file` — returns `nil`, and records `Absent`.
- `read_returns_nil_for_a_non_regular_file` — a directory returns `nil`.
- `read_records_the_hash_of_what_it_returned` — the record holds `Bytes` with the file's hash, for
  that path only.
- `unread_reads_entries_are_not_recorded` — a declared path that is never read is absent from the
  record.
- `read_outside_reads_is_refused` — `agentstacks.read(".env")` on line 4 gives
  `claude/adapter.lua:4: read: ".env" is not in extension.reads`.
- `read_resolving_outside_is_refused` — a reads path that resolves outside, read on line 9, gives
  `claude/adapter.lua:9: read: ".claude/settings.json" resolves outside the project`.
- `read_io_failure_names_the_path` — `FakeWorkspace::fail` on the `resolve` call for the reads
  path, then a read of it, gives `.claude/settings.json: <io::Error Display>` (R1). The expected
  text is built from `FakeWorkspace::denied()`.
- `io_failure_on_an_unread_path_is_silent` — the same failing path, never read: the run succeeds.
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

**Change:** (§4.5, R19) `require` stays absent, because airsl clears it for a script built from
source. `agentstacks.require(name)` works in any phase, and checks in this order (R5):

1. **Argument.** It must be one string (R3: `require: expected string, got <type>`).
2. **Name shape.** The name must match `^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)*$`. When it does not, and
   the name contains `/`, the refusal carries the hint with `/` turned into `.`:
   `require: invalid module name "lib/x" (use dots: lib.x)`. Otherwise the refusal is
   `require: invalid module name "<name>"` (R19).
3. **File exists.** The file must be in `LuaFiles`:
   `require: module "lib.x" not found in extension claude`.
4. **Not the entry.** `require: "adapter" is the entry file`.
5. **No cycle.** A name already on the loading stack is refused at the `require` call that closes
   the cycle. The message lists the stack from its first occurrence to the repeat, joined by ` → `
   (§4.5 row 4, R19).

The checks follow §4.5's table order. A module passes them all, then loads once with
`lua.load(text).set_name("@<path>")`, the call shape `probes/examples-main.rs` uses:

- The result is cached in a named Lua registry table keyed by module name, with a loaded set in
  `RunState`. A module that returns `nil` still loads only once.
- A module that raised while loading is not cached, and is popped from the loading stack.
- An error raised inside a module keeps the module's file and line (Q4), and surfaces through
  Task 4's wrapper.

The host keeps `load` for itself and never exposes it. Refusals are raised as non-sticky `Raised`
at the caller (§4.5 "location: the caller"). They are ordinary Lua errors, and uncaught they show
their located message (R19).

**Proof:**
- `require_returns_the_module_value` — `agentstacks.require("lib.util").twice(21)` returns `42` (Q4).
- `require_loads_once_and_caches` — a module that increments a global counter, required twice:
  the counter is 1, and both results are `rawequal`.
- `module_error_carries_its_file_and_line` — `lib/util.lua` raising `boom` on line 2 gives
  `claude/adapter.lua: lua error in adapter.lua: runtime error: lib/util.lua:2: boom`
  (§4.5, §4.6 row 3).
- `failed_module_is_not_cached` — a module that raises on its first load only: with `pcall` around
  the first require, a second require returns the module's value.
- `invalid_name_with_slash_gets_the_hint` — `agentstacks.require("lib/x")` on line 1 gives
  `claude/adapter.lua:1: require: invalid module name "lib/x" (use dots: lib.x)` (§4.5 row 1).
- `invalid_name_without_slash_has_no_hint` — `agentstacks.require("x y")` on line 1 gives
  `claude/adapter.lua:1: require: invalid module name "x y"` (R19).
- `require_takes_one_string` — `agentstacks.require(5)` on line 1 gives
  `claude/adapter.lua:1: require: expected string, got number` (R3).
- `missing_module_is_refused` — `agentstacks.require("lib.x")` gives
  `claude/adapter.lua:1: require: module "lib.x" not found in extension claude` (§4.5 row 2).
- `entry_module_is_refused` — `agentstacks.require("adapter")` gives
  `claude/adapter.lua:1: require: "adapter" is the entry file` (§4.5 row 3).
- `cycle_is_reported_at_the_closing_call` — `adapter.lua` requires `lib.a`, line 1 of `lib/a.lua`
  requires `lib.b`, and line 1 of `lib/b.lua` requires `lib.a`. The run fails with exactly
  `claude/lib/b.lua:1: require: module "lib.a" requires itself (lib.a → lib.b → lib.a)`
  (§4.5 row 4, R19).
- `require_refusal_does_not_stick` — `pcall(agentstacks.require, "lib.x")` returns `false`, and the
  run succeeds with no refusal (R19).
- `load_stays_withheld` — after a require, `type(load)` is still `"nil"` (Q4).

**Done when:** `cargo test -p agentstacks host::require` passes and `cargo make dod-crate agentstacks`
is green. Commit `feat(agentstacks): agentstacks.require over in-memory modules`.

### Task 11 — Build the payloads and read the values result

**Files:**
- Create `crates/agentstacks/src/host/payload.rs` — `values_payload(&ValuesInput) -> serde_json::Value`, `plan_payload(&PlanInput) -> serde_json::Value`
- Create `crates/agentstacks/src/host/values.rs` — `values_result(entry, Dispatched, &ValuesInput) -> Result<ValuesResult, Diagnostics>`
- Modify `crates/agentstacks/src/host/mod.rs`

**Change:**

- `payload.rs` translates plan 04's domain models into the §4.4 JSON shapes, key for key:
  - `values`: `api`, `tool`, `packs[{name, version, skills, agents}]`, and `vars` in the input's
    order (R8: the vars used by this tool's packs, sorted by name, which `ValuesInput` already
    holds);
  - `plan`: `api`, `tool`, `root`, and `packs[…]`, each with `skills[{id, name, dir, meta,
    files[{id, rel, text?}]}]`, `agents`, `hooks` and `native{files, hooks}`;
  - native hooks use the hook shape (R11): `kind`, `event`, `match`, `may_block`, and `command`
    (lua) or `text` (message). `match` is the list the formats reader holds (R28(d));
  - `text` is absent for a binary file.

  This is translation only: every value comes from the input type.
- `values_result` handles the §4.4 values rule table:
  - rows 1–2 are the host's: `NoHandler` gives `registers no values handler`, and `Nil` or a
    non-object gives `values: expected table, got <json_word>`;
  - an object goes to `ValuesResult::read(object, input, entry, json_word)` (plan 04), which
    applies rows 3–7 and R4, R12 at the `values` key path (§13 row 2).

  `entry` is `<tool>/<entry file>`.

**Proof:**
- `values_payload_matches_the_spec_example` — the payload built for the §4.4 example input equals
  the §4.4 `values` payload JSON.
- `plan_payload_matches_the_spec_example` — a `PlanInput` holding the §4.4 example's packs gives
  the §4.4 `plan` payload JSON. `logo.png` has no `text`. The one difference is that the native
  hook also carries its `kind`, as R11 requires.
- `native_hook_match_is_the_readers_list` — a native hook's `match` in the payload is the JSON
  array of the list the formats reader holds for it (R11, R28(d)).
- `values_example_dispatches_to_the_spec_result` — the §4.4 `lua` example, dispatched with the §4.4
  payload, gives this `ValuesResult` (`probes/examples-results.txt`):
  - root `.claude/agentstacks`;
  - skill `core/status` → invoke `/status`, dir `.claude/skills/status`;
  - agent `core/coder` → `coder`;
  - var `tmpdir` → `$TMPDIR`.
- `missing_values_handler_is_refused` — an entry that registers only `plan` gives
  `claude/adapter.lua: registers no values handler` (§4.4 row 1).
- `values_returning_nil_is_refused` — gives `claude/adapter.lua: values: expected table, got nil`
  (§4.4 row 2).
- `values_returning_a_list_names_list` — `return {1, 2}` gives `claude/adapter.lua: values:
  expected table, got list`.
- `empty_table_arrives_as_object` — `return {}` gives `claude/adapter.lua: values.root: required`.
  This covers §4.4 row 3 and "An empty Lua table arrives as an object".
- `nested_type_word_is_list` — `agents = {"x"}` gives `claude/adapter.lua: values.agents: expected
  table, got list` (§4.4 row 4).
- `skill_entry_needs_dir` — `skills = {["core/status"] = {invoke = "/status"}}` gives
  `claude/adapter.lua: values.skills."core/status".dir: required` (R12).

**Done when:** `cargo test -p agentstacks host::payload host::values` passes and
`cargo make dod-crate agentstacks` is green. Commit
`feat(agentstacks): extension payloads and values result`.

### Task 12 — Run a tool through the `Adapter` port

**Files:**
- Create `crates/agentstacks/src/host/session.rs` — `Session`
- Create `crates/agentstacks/src/host/adapter.rs` — `AirslAdapter` implementing `Adapter`
- Modify `crates/agentstacks/src/host/mod.rs` (`pub use adapter::AirslAdapter`, `pub use session::Session`), `host/testing.rs`

**Change:** `AirslAdapter` implements plan 09's `Adapter`, with `type Session = Session`. Its three
calls are as follows.

**`open<T: FileTree>(extension, files)`** (§4.1 step 3, §4.4 step 1):

1. reads `LuaFiles` (Task 1);
2. builds the engine (Task 2) with a fresh `RunState`;
3. runs the entry file at the top level, as `Script::from_source(entry_text, entry_path)`, under
   `Phase::Load`;
4. returns entry-run failures through `wrap`.

**`values(session, input)`** (§4.4 step 2) sets `Phase::Values`, dispatches `values` with
`values_payload`, and returns `values_result`.

**`plan<W: Workspace>(session, input, workspace, proposals)`** (§4.4 step 4) returns plan 06's
`Reads` or every refusal (sticky refusals, plus a wrapped failure). It:

1. builds the `ReadSnapshot` from `workspace` (Task 9);
2. seeds the id index from `input`;
3. moves the caller's `Proposals` into `RunState` for the dispatch, leaving `Proposals::new()` in
   its place. This is needed because Lua functions are `'static` and cannot hold the `&mut`;
4. sets `Phase::Plan`, dispatches `plan` with `plan_payload`, then returns to `Phase::Load`;
5. moves the `Proposals` back into `proposals`, whether the dispatch succeeded or not;
6. maps `NoHandler` to `registers no plan handler` (§4.4) and ignores any other return value.

The same engine serves `values` and then `plan`, because the entry run registers the handlers once.
A removed tool (D8, §6.4) is the same run with an empty `packs` list. Any extension error refuses
the whole run (§4.6 "Any extension error refuses the whole plan"). The host never writes.

**Proof:**
- `full_run_values_then_plan` — an entry with both handlers. `values` returns the §4.4 example
  result. `plan` copies `core/skill/status/SKILL.md` and skips `core/agent/coder`. Both proposals
  land in the caller's `Proposals` with their origins, and the returned `Reads` is empty.
- `proposals_come_back_after_a_failed_dispatch` — a `plan` handler that proposes one write and then
  raises: the run is refused, and the caller's `Proposals` still holds its earlier contents plus
  the accepted write.
- `missing_plan_handler_is_refused` — an entry with only `values` gives
  `claude/adapter.lua: registers no plan handler`.
- `plan_return_value_is_ignored` — a `plan` handler returning `{x = 1}` yields the same proposals
  as one returning nothing.
- `removed_tool_runs_with_zero_packs` — a `PlanInput` with an empty `packs` list dispatches, and
  only the handler's shared-file write comes back.
- `entry_failure_refuses_open` — an entry with a syntax error makes `open` return the §4.6 row 3
  message, and neither handler runs.
- `fresh_state_per_session` — two sessions for two tools: a global set by the first is `nil` in
  the second.
- Doc example on `AirslAdapter`: it opens `tests/fixtures/extensions/valid/minimal` through
  `fs::StdFileTree` and shows `values` refused with `claude/adapter.lua: registers no values
  handler`. No fake is used (R24, plan-level).

**Done when:** `cargo test -p agentstacks host::` passes and `cargo make dod` is green. Commit
`feat(agentstacks): run tool extensions through the airsl adapter`.

---

## Risks

- **`on` forwarding error text.** `agentstacks.on` is a Rust function that calls the Lua function
  `airsstack.ext.on`, so airsl's `UnknownEvent` comes back wrapped in a callback error. The probe
  proved airsl's text for a direct `airsstack.ext.on` call (R1b), not through a forwarder. If the
  first line of the wrapped error differs, the forwarder returns the inner cause, so that
  `unknown_event_is_airsl_text_with_sorted_events` holds as written. The test pins the text either
  way.
- **`Raised` detection depends on the error chain.** `Raised` is found by walking the
  `airsl::Error` source chain with `downcast_ref`, the way airsl's own `exhausted_memory` walks it
  (`engine.rs:405-412`). A refusal that Lua rethrows after `pcall` keeps the same wrapped error
  value. `uncaught_op_refusal_is_reported_once` and `uncaught_refusal_shows_its_located_message`
  guard both kinds.
- **Pre-reading `reads`.** The snapshot is taken just before the `plan` dispatch, not at each `read`
  call, because Lua functions must be `'static`. Within one plan run the two are the same.
  `app/apply` re-checks the disk against the `Reads` record (§7 step 3, R15).
- **`Proposals` on loan.** The caller's `Proposals` is moved into `RunState`, and the `&mut` is left
  holding `Proposals::new()` until the move back. The move back runs on every path out of `plan`,
  and `proposals_come_back_after_a_failed_dispatch` proves it.
- **The check order spans two plan-04 calls.** `Proposals::check_path` (row 9) runs before the id
  rows (row 10), and `Proposals::propose` comes last. `propose` returns `Result<(), OpError>`, one
  refusal, so the host never has to choose among several.
- **Converting the plan return value.** `Engine::dispatch` converts the handler's return value to
  JSON even though `plan` ignores it, so a `plan` handler that returns a function fails as a Lua
  failure. airsl decides this, and the plan accepts it.

## Proof

- `cargo make dod` is green at the end of Task 12, and `cargo test -p agentstacks host::` runs every
  test above on a real airsl 0.1.4 engine.
- `rg -n "dyn " crates/agentstacks/src/host` lists only the `ModuleSet::insert` calls in
  `engine.rs`, under the `// dyn:` comment.
- `rg -n "std::fs|std::process|std::net|crate::fs|crate::process" crates/agentstacks/src/host`
  prints nothing. The host reads only through `FileTree` and `Workspace` (§2 dependency rule).
- Spec rows covered:
  - **§4.1:** steps 1–3 (Tasks 1 and 12).
  - **§4.2:** the module set, Absent, Present, the `print` / `warn` removal, the limits, the budget
    reset, the breach messages, the 8 MiB read and the caught breach (Tasks 2, 4 and 9).
  - **§4.3:** every row of the function table, and the `read` rules (Tasks 2, 4, 6, 8, 9 and 10).
  - **§4.4:**
    - the flow (Task 12);
    - the three dispatch results and the second registration (Task 4);
    - both payloads (Task 11);
    - the values example and the values rule table: rows 1–2 here, rows 3–7 through
      `ValuesResult::read` with `json_word` (Task 11);
    - the missing plan handler and the ignored return value (Task 12).
  - **§4.5:** rows 1–4, the cache, failed-not-cached, the module's file and line, and `load` kept
    withheld (Task 10).
  - **§4.6:** rows 1–5, and "refuses the whole plan" (Tasks 4, 6, 8 and 12).
  - **§5:** every row of the second table (Task 5), plus the argument rows and the checked example
    (Task 6).
  - **§3.3:** the reading rows (Task 7), the phase and `read` rows (Tasks 8 and 9), stickiness
    (Task 8), and the location in a handler and in a module (Tasks 3 and 8).
  - **§13:** rows 1–3, the location shapes (Tasks 3, 4 and 11).
  - **§16:**
    - R1 for the `.lua` file and `reads` I/O (Tasks 1 and 9);
    - R3, all of its host rows (Tasks 1, 5, 6, 7 and 10);
    - R5, one refusal per call and the nearest Lua frame (Tasks 3, 6, 7 and 8);
    - R6, call-time duplicates via the loaned `Proposals` (Task 8);
    - R8, the `vars` order (Task 11);
    - R11, the native hook shape (Task 11);
    - R12, via `ValuesResult::read` (Task 11);
    - R13, via plan 04 (Task 8);
    - R14 and R19 (Tasks 6 and 10);
    - R24, plan-level: no fake in a doc example (Task 12);
    - R25, via `OpPath::proposed` (Task 8);
    - R28(a) phase first, R28(b) reads I/O, R28(d) native `match`, R28(l) cover-after-skip, R28(m) covers prefix (Tasks 8, 9, 11).
