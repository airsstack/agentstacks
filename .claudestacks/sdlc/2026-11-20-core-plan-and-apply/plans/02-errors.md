---
status: draft
created: 2026-11-22
depends-on: [01]
built-on: ea104af7451e
---

# Errors Implementation Plan

**Goal:** Give every new refusal one exact, located message.

**Architecture:** Each new rule module gets one `thiserror` enum whose `Display` is the exact spec
text that follows the location, the way every built module error does (`LockError`,
`ExtensionError`). The location is never part of the enum: a refusal is a built `Diagnostic`, a
`Location` (`src/types/report/location.rs`) paired with a `FormatError`, displayed
`<location>: <error>` (`src/types/report/diagnostic.rs`; spec §13). `FormatError`
(`src/types/report/format.rs`) gains one variant per new enum, so a `Diagnostic` can carry any of
them. Every test builds the whole `Diagnostic` and compares its `Display` with the spec line
verbatim, so the location shapes of spec §13 are proven with the messages.

| Enum | File | Spec rows |
|---|---|---|
| `LuaType`, `ApiFunction` | `src/types/op/word.rs` | §3.3 type words, §4.4 `list`; the function names every §3.3 / §4.3 / §4.5 / §10 call refusal starts with |
| `OpError` | `src/types/op/error.rs` | §3.2 case table, §3.3 table, §4.3 `read` rules, §4.4 `values` table and missing handlers, §4.5 table, §4.6 table, §10 table |
| `JsonEditError` | `src/types/json/error.rs` | §5 rules table, §5 Lua value table |
| `StepError` | `src/types/plan/error.rs` | §6.2 both tables, §6.3, §7 step 3 |
| `FetchError` | `src/types/resolve/error.rs` | §9.1 row 2, §9.2 failure table, §9.3 home |

Reuse instead of a twin (`modularity.md` Rule 2):

- the shared phrases `MustBeRelative`, `OutsideProject` and `Quoted` (`src/types/report/phrase.rs`)
  supply `"<text>" must be relative`, `"<text>" must stay inside the project` and the quoting;
- `InvalidName` (`src/types/name/invalid.rs:33`, `"<text>" is not a valid name`) supplies the
  binary-name refusal;
- `FieldError::Required` and `FieldError::UnknownKey` (`src/types/report/field.rs`, `required`,
  `unknown key`) are the `values` refusals whose location key already names the field
  (`claude/adapter.lua: values.root: required`); no `OpError` variant repeats them;
- the airsl out-of-range refusal `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4`
  (spec §10, "formats §4.3 wording") already exists as `AirslRange::installed_refusal`
  (`src/types/set/airsl.rs:153`, `SetError::AirslInstalled`, `src/types/set/error.rs:149`) and is
  proven by that function's doctest and `set_error_displays_the_airsl_and_clash_messages`; no new
  variant;
- an I/O failure, `<path>: <io::Error Display>` and `agentstacks.lock: <io::Error Display>` (spec
  §7 failure table, §13), is the built `FormatError::Io`, proven by
  `io_error_converts_and_displays_its_own_message`; no new variant.

Type words: a Lua refusal names Lua's types, not TOML's, so `LuaType` is a separate closed set from
`TypeWord` (different words, different format; Rule 2's "real invariant" case, stated in both
docs). `ApiFunction` is a bare discriminant of the Lua functions a refusal names; it is not a twin of
the operation enum the operations plan builds (that one carries payloads and has `delete`, which no
Lua function proposes).

`OpError` and `FetchError` are boxed inside `FormatError` with a hand-written `From`, as
`FormatError::Set` is today (`src/types/report/format.rs`), because their largest variants (a
`Location` plus quoted text; two `ContentHash` values plus a `CommitId`) are larger than every
other variant and every `Result` carrying a `Diagnostic` would grow with them
(`static-dispatch.md` table, "`Box<T>` … only when boxing slims a large enum variant").
`JsonEditError` and `StepError` use `#[from]`.

Guideline rules applied: `mod-rs-export-only.md` (the four `mod.rs` files gain only `pub mod` and
`#[doc(inline)] pub use` lines plus doc bullets); `unit-test-mandate.md` (every error file has a
colocated `#[cfg(test)] mod tests`, one named test per spec message); `strong-types.md` (names are
`ToolName` / `PackName` / `BinaryName`, hashes `ContentHash`, revs `CommitId`, quoted values
`Quoted`, never bare strings where a type exists); `doc-comment-discipline.md` (variant docs
describe the refusal, never cite a spec section or an issue number).

**Tech Stack:** Rust 2024, crate `agentstacks`, `thiserror 2` (workspace dependency).

**Content authority:** spec §13 (locations), and every message row named in the table above.

---

## Files that change

```
crates/agentstacks/src/types/op/word.rs         — create: LuaType and ApiFunction, each displayed as its word
crates/agentstacks/src/types/op/error.rs        — create: OpError
crates/agentstacks/src/types/op/mod.rs          — modify: `pub mod error; pub mod word;`, three `pub use` lines, doc bullets
crates/agentstacks/src/types/json/error.rs      — create: JsonEditError
crates/agentstacks/src/types/json/mod.rs        — modify: `pub mod error;`, `pub use error::JsonEditError;`, doc bullet
crates/agentstacks/src/types/plan/error.rs      — create: StepError
crates/agentstacks/src/types/plan/mod.rs        — modify: `pub mod error;`, `pub use error::StepError;`, doc bullet
crates/agentstacks/src/types/resolve/error.rs   — create: FetchError
crates/agentstacks/src/types/resolve/mod.rs     — modify: `pub mod error;`, `pub use error::FetchError;`, doc bullet
crates/agentstacks/src/types/report/format.rs   — modify: variants Op, JsonEdit, Step, Fetch and their From conversions; docs
```

In every test below, `L(file, n)` is `Location::at_line(file, n)`, `K(file, key)` is
`Location::at_key(file, key)` and `F(file)` is `Location::file(file)`. Each test asserts the
`Display` of `Diagnostic::new(location, error)`, and the expected string is the spec line quoted.
Where the spec gives only the text after `<tool>/<file>:<line>: <function>: ` (§3.3 header), the
expected string is that prefix, with the stated tool, file, line and function, followed by the
quoted text.

## Order of work

### Task 1 — Name Lua's type words and the API functions

**Files:**
- Create `crates/agentstacks/src/types/op/word.rs` — `//!` block (the closed words an op refusal
  prints; why separate from `TypeWord`), `LuaType`, `ApiFunction`, tests.
- Modify `crates/agentstacks/src/types/op/mod.rs` — `pub mod word;`, `pub use word::LuaType;`,
  `pub use word::ApiFunction;`, two doc bullets.

**Change:**
- `LuaType` — the nine words a Lua refusal may name: `nil`, `boolean`, `number`, `string`, `table`,
  `function`, `userdata`, `thread` (spec §3.3 "Type words"; an integer and a float are both
  `number`), and `list`, the word for a JSON array coming back from `values` (spec §4.4 "Type
  words map JSON back": object → `table`, array → `list`, null → `nil`). `Display` writes the word.
- `ApiFunction` — `write`, `copy`, `skip`, `require_binary`, `read`, `require` (spec §4.3 table,
  §4.5); `Display` writes the name as the Lua author calls it.

**Proof:** written first, seen failing, then passing.
- `lua_type_displays_each_word` — each of the nine variants displays exactly its word above.
- `api_function_displays_each_name` — each of the six variants displays exactly its name above.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): lua type words and api function names`.

### Task 2 — Word every operation and extension refusal

**Files:**
- Create `crates/agentstacks/src/types/op/error.rs` — `//!` block, `OpError`, tests.
- Modify `crates/agentstacks/src/types/op/mod.rs` — `pub mod error;`, `pub use error::OpError;`,
  doc bullet.
- Modify `crates/agentstacks/src/types/report/format.rs` — `FormatError::Op(Box<OpError>)` with a
  hand-written `From<OpError>`; module doc names the boxed variants.

**Change:** `OpError` has one variant per message below. Fields use `ApiFunction`, `LuaType`,
`Quoted`, `MustBeRelative`, `OutsideProject`, `InvalidName`, `Location` (an earlier call, displayed
`<file>:<line>` inside the extension, e.g. `adapter.lua:9`), `ToolName`, `PackName`, `BinaryName`,
and `String` only for a field key as printed (`path`, `covers[2]`, `foo`), an item id, a key path,
`<root>`, or a line of airsl's own text.

At a call, location `<tool>/<file>:<line>` (spec §13 row 1); text starts `<function>: ` (§3.3):

| Variant | Displays (after the location and `: `) | Spec |
|---|---|---|
| `ArgumentType { function, expected, got }` | `<function>: expected <expected>, got <got>` | §3.3 row "argument is one table" |
| `Required { function, field }` | `<function>: <field>: required` | §3.3 row "required field present" |
| `FieldType { function, field, expected, got }` | `<function>: <field>: expected <expected>, got <got>` | §3.3 row "field type matches exactly" |
| `CoversNotList { function }` | `<function>: covers: expected a list` | §3.3 row "`covers` is a list" |
| `UnknownKey { function, key }` | `<function>: <key>: unknown key` | §3.3 row "no unknown field" |
| `PathNotUtf8 { function }` | `<function>: path: not valid UTF-8` | §3.3 row "`path` is UTF-8" |
| `Empty { function, field }` | `<function>: <field>: empty` | §3.3 rows "`path` shape" (`path: empty`) and "`skip.reason`" (`reason: empty`) |
| `PathRelative { function, path: MustBeRelative }` | `<function>: path "<p>" must be relative` | §3.3 row "`path` shape" |
| `PathOutside { function, path: OutsideProject }` | `<function>: path "<p>" must stay inside the project` | §3.3 row "`path` shape" |
| `PathSeparator { function, path: Quoted }` | `<function>: path "<p>" must use / as the separator` | §3.3 row "`path` shape" |
| `PathManaged { function, path: Quoted }` | `<function>: path "<p>" is managed by agentstacks` | §3.3 row "own files" |
| `ProposedAt { function, path: Quoted, earlier: Location }` | `<function>: path "<p>" is already proposed at <earlier>` | §3.3 row "not proposed twice" |
| `ProposedByTool { function, path: Quoted, tool: ToolName }` | `<function>: path "<p>" is already proposed by <tool>` | same row |
| `ProposedByCore { function, path: Quoted, pack: PackName }` | `<function>: path "<p>" is already proposed by core (root file of <pack>)` | same row |
| `NotInPlan { function, field, id: Quoted }` | `<function>: <field> "<id>" is not in this plan` | §3.3 row "name an id in this run's payload" |
| `InvalidBinaryName(InvalidName)` | `require_binary: name: "<text>" is not a valid name` | §3.3 row "`require_binary.name`" |
| `NotInReads { path: Quoted }` | `read: "<p>" is not in extension.reads` | §3.3 row "`read` path in `extension.reads`" |
| `OutsidePlanHandler { function }` | `<function>: only allowed inside the plan handler` | §3.3 row "called outside the `plan` handler" |
| `ReadOutside { path: Quoted }` | `read: "<p>" resolves outside the project` | §4.3 `read` rules |
| `SkipAfterCover { item: Quoted, earlier: Location }` | `skip: item "<id>" is already covered at <earlier>` | §3.2 case row 2 |
| `CoverAfterSkip { function, id: Quoted, earlier: Location }` | `<function>: covers "<id>", which <earlier> skipped` | §3.2 case row 2 (see Risks) |
| `ModuleName { name: Quoted, dotted: String }` | `require: invalid module name "<name>" (use dots: <dotted>)` | §4.5 row 1 |
| `ModuleNotFound { module: Quoted, tool: ToolName }` | `require: module "<module>" not found in extension <tool>` | §4.5 row 2 |
| `EntryModule { name: Quoted }` | `require: "<name>" is the entry file` | §4.5 row 3 |
| `ModuleCycle { module: Quoted, chain: Vec<String> }` | `require: module "<module>" requires itself (<chain joined by " → ">)` | §4.5 row 4 |
| `BinaryNotFound { name: BinaryName }` | `require_binary: "<name>" not found on PATH` | §10 row 2 |

At the entry with no line, location `<tool>/<entry>` (spec §13 row 3):

| Variant | Displays | Spec |
|---|---|---|
| `Uncovered { id }` | `<id> got no operation (copy it, write it, or skip it with a reason)` | §3.2 case row 1 |
| `TranslateMissing { root }` | `lua hooks need <root>/translate.lua (write it)` | §3.2 case row 5 |
| `NoValuesHandler` | `registers no values handler` | §4.4 `values` table row 1 |
| `NoPlanHandler` | `registers no plan handler` | §4.4 after the `plan` payload |
| `Lua { first_line }` | `<first_line>`, unchanged | §4.6 rows 3-5 |

At the entry with a `values` key path, location `<tool>/<entry>` + key (spec §13 row 2):

| Variant | Displays | Spec |
|---|---|---|
| `ValuesType { expected, got }` | `expected <expected>, got <got>` | §4.4 `values` rows 2, 4, 6 |
| `MustBeRelative(MustBeRelative)` | `"<p>" must be relative` | §4.4 `values` row 3 |
| `NotASkill` | `not a skill in this run` | §4.4 `values` row 5 |
| `NotAVar` | `not a var in this run` | §4.4 `values` row 5 |

At `agentstacks.toml` (spec §13 rows 6-7):

| Variant | Displays | Spec |
|---|---|---|
| `PackBinaryNotFound { name: BinaryName }` | `binary "<name>" not found on PATH` | §10 row 1 (key `packs.<pack>`) |
| `AirslNotFound { packs: Vec<PackName> }` | `airsl not found on PATH (needed by <packs joined by ", ">)` | §10 row 3 (no key) |
| `AirslVersionText { text }` | `airsl --version printed "<text>", expected "airsl <version>"` | §10 row 3 (no key; `"airsl <version>"` is literal text) |

**Proof:** written first, seen failing, then passing. One test per message, each asserting the
`Diagnostic` display verbatim:
- `op_error_uncovered_item` — `F("claude/adapter.lua")`: `claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a reason)` (§3.2) and `claude/adapter.lua: core/agent/coder got no operation (copy it, write it, or skip it with a reason)` (§13 row 3).
- `op_error_skip_after_cover` — `L("claude/adapter.lua", 14)`, earlier `L("adapter.lua", 9)`: `claude/adapter.lua:14: skip: item "core/skill/status" is already covered at adapter.lua:9` (§3.2).
- `op_error_cover_after_skip` — `L("claude/adapter.lua", 14)`, earlier `L("adapter.lua", 9)`, function `write`: `claude/adapter.lua:14: write: covers "core/agent/coder", which adapter.lua:9 skipped` (§3.2, the `…` read as that row's `<location>: <function>: ` prefix).
- `op_error_translate_missing` — `F("claude/adapter.lua")`, root `.claude/agentstacks`: `claude/adapter.lua: lua hooks need .claude/agentstacks/translate.lua (write it)` (§3.2, "with `<root>` filled in").
- `op_error_argument_type` — `L("claude/adapter.lua", 12)`: `claude/adapter.lua:12: write: expected table, got string`; `L("claude/adapter.lua", 9)`: `claude/adapter.lua:9: read: expected string, got number` (§3.3 row 1).
- `op_error_required_field` — `claude/adapter.lua:12: write: path: required`, `claude/adapter.lua:12: write: content: required`, `claude/adapter.lua:12: copy: file: required`, `claude/adapter.lua:12: skip: item: required`, `claude/adapter.lua:12: skip: reason: required`, `claude/adapter.lua:12: require_binary: name: required` (§3.3 row 2, "same shape for `content`, `file`, `item`, `reason`, `name`").
- `op_error_field_type` — `claude/adapter.lua:12: write: path: expected string, got number` (§3.3 row 3 and §13 row 1), `claude/adapter.lua:12: write: covers: expected table, got string`, `claude/adapter.lua:12: write: covers[2]: expected string, got boolean`.
- `op_error_covers_not_a_list` — `claude/adapter.lua:12: write: covers: expected a list`.
- `op_error_unknown_key` — `claude/adapter.lua:12: write: foo: unknown key`.
- `op_error_path_not_utf8` — `claude/adapter.lua:12: write: path: not valid UTF-8`.
- `op_error_path_empty` — `claude/adapter.lua:12: write: path: empty`.
- `op_error_path_must_be_relative` — `claude/adapter.lua:12: write: path "/x" must be relative` (§3.3 row 7) and `claude/adapter.lua:12: write: path "/etc/x" must be relative` (§3.3 example).
- `op_error_path_outside_project` — `claude/adapter.lua:12: write: path "../x" must stay inside the project`.
- `op_error_path_separator` — `claude/adapter.lua:12: write: path "a\b" must use / as the separator` (one backslash between `a` and `b`).
- `op_error_path_managed` — `claude/adapter.lua:12: write: path "agentstacks.lock" is managed by agentstacks` and the same with `agentstacks.toml`.
- `op_error_path_proposed_at` — earlier `L("adapter.lua", 9)`: `claude/adapter.lua:12: write: path ".claude/x" is already proposed at adapter.lua:9`.
- `op_error_path_proposed_by_tool` — tool `opencode`: `claude/adapter.lua:12: write: path ".claude/x" is already proposed by opencode`.
- `op_error_path_proposed_by_core` — pack `core`: `claude/adapter.lua:12: write: path ".claude/x" is already proposed by core (root file of core)`.
- `op_error_id_not_in_plan` — `claude/adapter.lua:12: copy: file "core/skill/x/SKILL.md" is not in this plan`, `claude/adapter.lua:12: skip: item "core/skill/x/SKILL.md" is not in this plan`, `claude/adapter.lua:12: write: covers[1] "core/skill/x/SKILL.md" is not in this plan` (§3.3 row 10, "same shape for `item`, `covers[1]`").
- `op_error_reason_empty` — `claude/adapter.lua:12: skip: reason: empty`.
- `op_error_binary_name_invalid` — built from `BinaryName::new("Git")`'s error: `claude/adapter.lua:12: require_binary: name: "Git" is not a valid name`.
- `op_error_read_not_in_reads` — `claude/adapter.lua:9: read: ".env" is not in extension.reads`.
- `op_error_outside_plan_handler` — `claude/adapter.lua:3: write: only allowed inside the plan handler` and `claude/adapter.lua:3: read: only allowed inside the plan handler` (§3.3 row 14, "ops and `read`").
- `op_error_read_resolves_outside` — `claude/adapter.lua:9: read: ".claude/settings.json" resolves outside the project` (§4.3).
- `op_error_no_values_handler` — `claude/adapter.lua: registers no values handler`.
- `op_error_no_plan_handler` — `claude/adapter.lua: registers no plan handler`.
- `op_error_values_type` — `K("claude/adapter.lua", "values")`: `claude/adapter.lua: values: expected table, got nil`; key `values.agents`: `claude/adapter.lua: values.agents: expected table, got list`; key `values.skills."core/status".dir`: `claude/adapter.lua: values.skills."core/status".dir: expected string, got number`.
- `values_required_is_the_field_shape` — `K("claude/adapter.lua", "values.root")` with `FieldError::Required`: `claude/adapter.lua: values.root: required` (§4.4 row 3, §13 row 2).
- `values_unknown_key_is_the_field_shape` — with `FieldError::UnknownKey`: `claude/adapter.lua: values.foo: unknown key` and `claude/adapter.lua: values.skills."core/status".icon: unknown key`.
- `op_error_values_root_must_be_relative` — `claude/adapter.lua: values.root: "/x" must be relative`.
- `op_error_not_a_skill_in_run` — `claude/adapter.lua: values.skills."core/x": not a skill in this run`.
- `op_error_not_a_var_in_run` — `claude/adapter.lua: values.vars.home: not a var in this run`.
- `op_error_module_name` — dotted `lib.x`: `claude/adapter.lua:1: require: invalid module name "lib/x" (use dots: lib.x)`.
- `op_error_module_not_found` — `claude/adapter.lua:1: require: module "lib.x" not found in extension claude`.
- `op_error_entry_module` — `claude/adapter.lua:1: require: "adapter" is the entry file`.
- `op_error_module_cycle` — `L("claude/lib/a.lua", 1)`, chain `lib.a`, `lib.b`, `lib.a`: `claude/lib/a.lua:1: require: module "lib.a" requires itself (lib.a → lib.b → lib.a)` (the arrow is U+2192).
- `op_error_lua_failure_keeps_airsl_text` — `claude/adapter.lua: lua error in adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>` and `claude/adapter.lua: lua error in plan: adapter.lua:3: path must be relative` (§4.6 row 3).
- `op_error_limit_breach_keeps_airsl_text` — `` claude/adapter.lua: script `plan` exceeded its instruction limit of 10000000 `` (§4.6 row 4).
- `op_error_unknown_event_keeps_airsl_text` — `` claude/adapter.lua: lua error in adapter.lua: event `nope` is not one this host dispatches; declared events: plan, values `` (§4.6 row 5, events `plan, values`).
- `op_error_pack_binary_not_found` — `K("agentstacks.toml", "packs.core")`: `agentstacks.toml: packs.core: binary "rg" not found on PATH`.
- `op_error_extension_binary_not_found` — `L("claude/adapter.lua", 7)`: `claude/adapter.lua:7: require_binary: "node" not found on PATH`.
- `op_error_airsl_not_found` — `F("agentstacks.toml")`, packs `core`, `sdlc`: `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)` (§10, §13 row 7).
- `op_error_airsl_version_unparsed` — text `airsl-dev`: `agentstacks.toml: airsl --version printed "airsl-dev", expected "airsl <version>"`.
- `op_error_converts_and_displays_unchanged` (in `report/format.rs`) — `FormatError::from(OpError::NoPlanHandler)` displays `registers no plan handler`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): operation and extension refusals`.

### Task 3 — Word every JSON edit refusal

**Files:**
- Create `crates/agentstacks/src/types/json/error.rs` — `//!` block, `JsonEditError`, tests.
- Modify `crates/agentstacks/src/types/json/mod.rs` — `pub mod error;`,
  `pub use error::JsonEditError;`, doc bullet.
- Modify `crates/agentstacks/src/types/report/format.rs` — `JsonEdit(#[from] JsonEditError)`.

**Change:** Location `<tool>/<file>:<line>` of the caller (spec §5 header, §13 row 1). Every
variant's text starts `json.edit: `:

| Variant | Displays | Spec §5 |
|---|---|---|
| `TextType { got: LuaType }` | `json.edit: text: expected string, got <got>` | rules row "arguments" |
| `PathType { got: LuaType }` | `json.edit: path: expected table, got <got>` | rules row "arguments" |
| `Parse(ParserMessage)` | `json.edit: <serde_json message>` | rules row "`text` parses as JSON" |
| `TopLevel` | `json.edit: top level must be an object` | rules row "top level is an object" |
| `PathEmpty` | `json.edit: path: empty` | rules row "`path` is a non-empty list" |
| `PathItemType { index, got: LuaType }` | `json.edit: path[<index>]: expected string, got <got>` (1-based, as Lua counts) | same row |
| `NotAnObject { key: Quoted }` | `json.edit: "<key>" is not an object` | rules row "setting" |
| `MixedTable` | `json.edit: value: table mixes list and keyed entries` | Lua value table row 4 |
| `Nan` | `json.edit: value: NaN cannot be JSON` | Lua value table row 5 |
| `Infinite` | `json.edit: value: inf cannot be JSON` | Lua value table row 5, "(also `inf`)" |
| `StringNotUtf8` | `json.edit: value: string is not valid UTF-8` | Lua value table row 5 |
| `NotJson { got: LuaType }` | `json.edit: value: <got> cannot be JSON` | Lua value table row 6 |

`ParserMessage` is the built carrier of a parser's own words (`src/types/report/parser.rs`).

**Proof:** written first, seen failing, then passing. Location `L("claude/adapter.lua", 5)` in each:
- `json_edit_error_text_type` — `claude/adapter.lua:5: json.edit: text: expected string, got nil`.
- `json_edit_error_path_type` — `claude/adapter.lua:5: json.edit: path: expected table, got string`.
- `json_edit_error_parse_keeps_serde_text` — message `trailing comma at line 1 column 8` (`probes/json-results.md`): `claude/adapter.lua:5: json.edit: trailing comma at line 1 column 8`.
- `json_edit_error_top_level` — `claude/adapter.lua:5: json.edit: top level must be an object`.
- `json_edit_error_path_empty` — `claude/adapter.lua:5: json.edit: path: empty`.
- `json_edit_error_path_item_type` — index 2: `claude/adapter.lua:5: json.edit: path[2]: expected string, got number`.
- `json_edit_error_not_an_object` — `claude/adapter.lua:5: json.edit: "hooks" is not an object`.
- `json_edit_error_mixed_table` — `claude/adapter.lua:5: json.edit: value: table mixes list and keyed entries`.
- `json_edit_error_nan` — `claude/adapter.lua:5: json.edit: value: NaN cannot be JSON`.
- `json_edit_error_inf` — `claude/adapter.lua:5: json.edit: value: inf cannot be JSON`.
- `json_edit_error_string_not_utf8` — `claude/adapter.lua:5: json.edit: value: string is not valid UTF-8`.
- `json_edit_error_function_value` — `claude/adapter.lua:5: json.edit: value: function cannot be JSON`.
- `json_edit_error_converts_and_displays_unchanged` (in `report/format.rs`) — `FormatError::from(JsonEditError::TopLevel)` displays `json.edit: top level must be an object`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): json edit refusals`.

### Task 4 — Word every file step refusal

**Files:**
- Create `crates/agentstacks/src/types/plan/error.rs` — `//!` block, `StepError`, tests.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `pub mod error;`, `pub use error::StepError;`,
  doc bullet.
- Modify `crates/agentstacks/src/types/report/format.rs` — `Step(#[from] StepError)`.

**Change:** Location: the project-relative `<path>` (spec §13 rows 4-5), or
`agentstacks.toml` + `packs.<pack>.path` for a `path` source (spec §6.3 last paragraph).

| Variant | Displays | Spec |
|---|---|---|
| `NotRegular` | `is not a regular file` | §6.2 proposed table row 1; locked table row 5 |
| `Changed` | `changed since the last apply` | §6.2 proposed table row 7; checked example 2 |
| `NotOurs` | `exists and was not written by agentstacks` | §6.2 proposed table row 10; checked example 7 |
| `ChangedNotDeleting` | `changed since the last apply; not deleting it` | §6.2 locked table row 4 |
| `Outside` | `resolves outside the project` | §6.3 step 3 and last paragraph; §6.2 checked example 8 |
| `ChangedAfterPlan` | `changed after plan; run plan again` | §7 step 3 |

**Proof:** written first, seen failing, then passing.
- `step_error_not_regular` — `F(".claude/x")`: `.claude/x: is not a regular file`.
- `step_error_changed` — `F(".claude/skills/status/SKILL.md")`: `.claude/skills/status/SKILL.md: changed since the last apply` (§6.2 checked example 2).
- `step_error_not_ours` — `F(".claude/agents/coder.md")`: `.claude/agents/coder.md: exists and was not written by agentstacks` (§6.2 checked example 7).
- `step_error_changed_not_deleting` — `F(".claude/x")`: `.claude/x: changed since the last apply; not deleting it`.
- `step_error_outside` — `F(".claude/x")`: `.claude/x: resolves outside the project`; and `K("agentstacks.toml", "packs.team-lint.path")`: `agentstacks.toml: packs.team-lint.path: resolves outside the project` (§6.3).
- `step_error_changed_after_plan` — `F(".claude/x")`: `.claude/x: changed after plan; run plan again` (§7, §13 row 5).
- `step_error_converts_and_displays_unchanged` (in `report/format.rs`) — `FormatError::from(StepError::Changed)` displays `changed since the last apply`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): file step refusals`.

### Task 5 — Word every source fetch refusal

**Files:**
- Create `crates/agentstacks/src/types/resolve/error.rs` — `//!` block, `FetchError`, tests.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `pub mod error;`,
  `pub use error::FetchError;`, doc bullet.
- Modify `crates/agentstacks/src/types/report/format.rs` — `FormatError::Fetch(Box<FetchError>)`
  with a hand-written `From<FetchError>`.

**Change:** Location `agentstacks.toml`, with the entry key (`packs.core`, `tools.claude`, or
`packs.core.subdir`) or none (spec §13 rows 6-7).

| Variant | Displays | Spec |
|---|---|---|
| `TreeMismatch { found: ContentHash, rev: CommitId, locked: ContentHash }` | `tree <found> at rev <rev> differs from the lock's <locked>` (a `ContentHash` displays `sha256:<hex>`, `src/types/tree/hash/content.rs:114`) | §9.1 row 2 |
| `GitNotFound { needed_by: Vec<String> }` | `git not found on PATH (needed by <keys joined by ", ">)` | §9.2 failure row 1 (no key) |
| `GitFailed { subcommand, stderr }` | `git <subcommand> failed: <stderr>` (the caller passes stderr already trimmed) | §9.2 failure row 2 |
| `SubdirNotFound { subdir: Quoted, at }` | `"<subdir>" not found at <tag or rev>` | §9.2 failure row 4 |
| `SubdirIsFile { subdir: Quoted }` | `"<subdir>" is a file, not a directory` | §9.2 failure row 5 |
| `HomeUnset` | `HOME is not set, so the cache (~/.airsstack/cache) has no home` | §9.3 (no key) |

**Proof:** written first, seen failing, then passing.
- `fetch_error_tree_mismatch` — `K("agentstacks.toml", "packs.core")`, found and locked two distinct `ContentHash` values, rev a 40-hex `CommitId`: `agentstacks.toml: packs.core: tree sha256:<found hex> at rev <rev> differs from the lock's sha256:<locked hex>` with the three values filled in.
- `fetch_error_git_not_found` — `F("agentstacks.toml")`, keys `packs.core`, `tools.claude`: `agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)`.
- `fetch_error_git_failed` — key `packs.core`: `agentstacks.toml: packs.core: git fetch failed: fatal: couldn't find remote ref refs/tags/packs-v9.9.9` (§9.2, G7; §13 row 6 names `git fetch failed`) and `agentstacks.toml: packs.core: git ls-remote failed: fatal: could not read Username for 'https://github.com': terminal prompts disabled` (§9.2, G1 is the `ls-remote` step).
- `fetch_error_subdir_not_found` — key `packs.core.subdir`, at `packs-v0.1.0`: `agentstacks.toml: packs.core.subdir: "packs/nope" not found at packs-v0.1.0`.
- `fetch_error_subdir_is_a_file` — `agentstacks.toml: packs.core.subdir: "packs/core/pack.toml" is a file, not a directory`.
- `fetch_error_home_unset` — `F("agentstacks.toml")`: `agentstacks.toml: HOME is not set, so the cache (~/.airsstack/cache) has no home`.
- `fetch_error_converts_and_displays_unchanged` (in `report/format.rs`) — `FormatError::from(FetchError::HomeUnset)` displays `HOME is not set, so the cache (~/.airsstack/cache) has no home`.

**Done when:** `cargo make dod-crate agentstacks` passes, then `cargo make dod`, and the
Definition of Done is green. Commit `feat(agentstacks): source fetch refusals`.

---

## Risks

- **Which plan raises what.** This plan only words refusals; it decides none. Each variant's doc
  names the condition, and the rule plans (operations, json, steps, resolve, host, flows) construct
  them. A rule plan that meets a refusal with no variant here reports it rather than inventing
  text.
- `CoverAfterSkip`: spec §3.2 writes this message with a leading `…`. The plan reads it as the
  same `<location>: <function>: ` prefix as the `skip:` message beside it; the function is the one
  that covers (`copy` or `write`), and for `copy` the id is its `file` (§3.1: a copy "covers
  `file`").
- `FormatError` boxing: if the gate's clippy reports `result_large_err` or `large_enum_variant` for
  `JsonEditError` or `StepError` too, the same hand-written `From` with a `Box` applies; nothing
  else changes.
- Not planned, because the spec gives no verbatim text (sent as open questions with this plan set):
  the other §3.3 path forms under a `values` key (`values.root` / `dir` empty, outside the
  project, `\` separator); a `values.agents` key that names no agent of the run; a duplicate with
  core's `<root>/runtime.lua` write (only `(root file of <pack>)` is given); `-inf` in
  `json.edit`. `NotJson` carries the `LuaType`, so `userdata` and `thread` print their own word in
  the shape of the `function` example.
- Not here: the doctor findings of spec §8 and the notes of §9.1 and §9.4 are not refusals; their
  text belongs to the types that report them (steps and resolve plans).

## Proof

- `cargo make dod` is green at the end of Task 5.
- Spec rows covered: §3.2 case rows 1, 2, 5 (rows 3-4 are accepted cases, no message); §3.3 every
  refusal row and the example; §3.3 type words; §4.3 `read` outside; §4.4 `values` rows 1-7 as
  given verbatim, missing `plan` handler, type words incl. `list`; §4.5 rows 1-4; §4.6 rows 3-5
  (rows 1-2 are covered by the §3.3 messages and by `JsonEditError`); §5 every refusal; §6.2 both
  tables' refusals and checked examples 2, 7, 8; §6.3 both outside forms; §7 step 3 and the I/O rows
  (built `FormatError::Io`); §9.1 row 2; §9.2 failure rows 1, 2, 4, 5 (row 3 is git's own refusal,
  row 6 reuses row 2); §9.3 home; §10 rows 1-3 (out of range: built `SetError::AirslInstalled`);
  §13 every example.
