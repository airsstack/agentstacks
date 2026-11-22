---
status: done
created: 2026-11-22
depends-on: [01]
built-on: f39b2a814319
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
| `PathShape` | `src/types/op/shape.rs` | §3.3 row "`path` shape", §16 R4, R25: the location-free, field-free path-shape refusal |
| `OpError` | `src/types/op/error.rs` | §3.2 case table, §3.3 table, §4.3 `read` rules, §4.4 `values` table and missing handlers, §4.5 table, §4.6 table, §10 table, §16 R1 (airsl spawn), R2 (airsl), R3, R4, R6, R12, R13, R19, R25 |
| `JsonEditError` | `src/types/json/error.rs` | §5 rules table, §5 Lua value table, §16 R3, R14 |
| `StepError` | `src/types/plan/error.rs` | §6.2 both tables, §6.3, §7 step 3, §16 R15, R21 |
| `FetchError` | `src/types/resolve/error.rs` | §9.1 row 2, §9.2 failure table, §9.3 home, §16 R1 (git spawn), R2 (git) |

Reuse instead of a twin (`modularity.md` Rule 2):

- the shared phrases `MustBeRelative`, `OutsideProject` and `Quoted` (`src/types/report/phrase.rs`)
  supply `"<text>" must be relative`, `"<text>" must stay inside the project` and the quoting;
- `InvalidName` (`src/types/name/invalid.rs:33`, `"<text>" is not a valid name`) supplies the
  binary-name refusal;
- `FieldError::Required` and `FieldError::UnknownKey` (`src/types/report/field.rs`: `required`,
  `unknown key`) are the `values` refusals whose location key already names the field
  (`claude/adapter.lua: values.root: required`, `… values.skills."core/status".dir: required` §16
  R12); no `OpError` variant repeats them. `values.root: empty` is not `FieldError::Empty`: it is
  the path shape, `OpError::ValuesPath` (§16 R4);
- one `PathShape` serves both path locations (§3.3 call paths and §16 R4 `values` paths), so the four
  shape wordings exist once; `OpPath::new` in the operations plan returns it;
- the airsl out-of-range refusal `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4`
  (spec §10, "formats §4.3 wording") already exists as `AirslRange::installed_refusal`
  (`src/types/set/airsl.rs:153`, `SetError::AirslInstalled`, `src/types/set/error.rs:150`) and is
  proven by that function's doctest and `set_error_displays_the_airsl_and_clash_messages`; no new
  variant;
- an I/O failure with no message of its own, `<path>: <io::Error Display>` (spec §7 failure table,
  §13, §16 R1, and the doctor's I/O finding of §16 R23), is the built `FormatError::Io`, proven by
  `io_error_converts_and_displays_its_own_message`; no new variant;
- an extension `.lua` file that is not UTF-8, `<tool>/<file>: not valid UTF-8` (§16 R3), is the
  refusal of the built `utf8_text` (`types/tree/text.rs` after the skeleton plan) carried by
  `FormatError::Io`; no new variant.

Type words: a Lua refusal names Lua's types, not TOML's, so `LuaType` is a separate closed set from
`TypeWord` (different words, different format; Rule 2's "real invariant" case, stated in both
docs). `ApiFunction` is a bare discriminant of the Lua functions a refusal names; it is not a twin of
the operation enum the operations plan builds (that one carries payloads and has `delete`, which no
Lua function proposes).

`OpError` and `FetchError` each carry a `std::io::Error` in one variant (the spawn failures of §16
R1), so they derive `Debug` and `thiserror::Error` only, like `FormatError`; tests compare displayed
text, as `FormatError`'s docs say. They are boxed inside `FormatError` with a hand-written `From`, as
`FormatError::Set` is today, because their largest variants (a `Location` plus quoted text; two
`ContentHash` values plus a `CommitId`) are larger than every other variant and every `Result`
carrying a `Diagnostic` would grow with them (`static-dispatch.md` table, "`Box<T>` … only when
boxing slims a large enum variant"). `JsonEditError` and `StepError` use `#[from]`.

Guideline rules applied: `mod-rs-export-only.md` (the four `mod.rs` files gain only `pub mod` and
`#[doc(inline)] pub use` lines plus doc bullets); `unit-test-mandate.md` (every error file has a
colocated `#[cfg(test)] mod tests`, one named test per spec message); `strong-types.md` (names are
`ToolName` / `PackName` / `BinaryName`, hashes `ContentHash`, revs `CommitId`, quoted values
`Quoted`, never bare strings where a type exists); `doc-comment-discipline.md` (variant docs
describe the refusal, never cite a spec section or an issue number). No doc-comment example uses a
fake (fakes are `#[cfg(test)]`, plan-set rule).

**Tech Stack:** Rust 2024, crate `agentstacks`, `thiserror 2` (workspace dependency).

**Content authority:** spec §13 (locations), and every message row named in the table above.

---

## Files that change

```
crates/agentstacks/src/types/op/word.rs         — create: LuaType and ApiFunction, each displayed as its word
crates/agentstacks/src/types/op/shape.rs        — create: PathShape, the location-free, field-free path-shape refusal
crates/agentstacks/src/types/op/error.rs        — create: OpError
crates/agentstacks/src/types/op/mod.rs          — modify: `pub mod error; pub mod shape; pub mod word;`, four `pub use` lines, doc bullets
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
Where the spec gives only the text after `<tool>/<file>:<line>: <function>: ` (§3.3 header; §16 R3
writes it as `<fn>: `), the expected string is that prefix, with the stated tool, file, line and
function, followed by the quoted text.

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

### Task 2 — Word the path-shape refusal once

**Files:**
- Create `crates/agentstacks/src/types/op/shape.rs` — `//!` block (the shape refusal of a project
  path, with neither location nor field, so the call refusals and the `values` refusals word it
  the same; the operations plan's `OpPath::new` returns it), `PathShape`, tests.
- Modify `crates/agentstacks/src/types/op/mod.rs` — `pub mod shape;`, `pub use shape::PathShape;`,
  doc bullet.

**Change:** `PathShape`, a `thiserror` enum, derives `Debug`, `Clone`, `PartialEq`, `Eq`:

| Variant | Displays | Spec |
|---|---|---|
| `Empty` | `empty` | §3.3 row "`path` shape" (`path: empty`); a path that normalises to nothing (§16 R25); `values.root: empty` (§16 R4) |
| `MustBeRelative(MustBeRelative)` | `"<p>" must be relative` | §3.3 row "`path` shape"; §4.4 `values` row 3 |
| `OutsideProject(OutsideProject)` | `"<p>" must stay inside the project` | §3.3 row "`path` shape"; §16 R4 |
| `Separator(Quoted)` | `"<p>" must use / as the separator` | §3.3 row "`path` shape"; §16 R4 |

The two phrase variants wrap the shared phrase types transparently (the phrase module's naming
rule).

**Proof:** written first, seen failing, then passing. One test per display:
- `path_shape_empty` — `PathShape::Empty` displays `empty`.
- `path_shape_must_be_relative` — for `/x`: `"/x" must be relative`.
- `path_shape_outside_project` — for `../x`: `"../x" must stay inside the project`.
- `path_shape_separator` — for `a\b` (one backslash): `"a\b" must use / as the separator`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): path shape refusal`.

### Task 3 — Word every operation and extension refusal

**Files:**
- Create `crates/agentstacks/src/types/op/error.rs` — `//!` block, `OpError`, tests.
- Modify `crates/agentstacks/src/types/op/mod.rs` — `pub mod error;`, `pub use error::OpError;`,
  doc bullet.
- Modify `crates/agentstacks/src/types/report/format.rs` — `FormatError::Op(Box<OpError>)` with a
  hand-written `From<OpError>`; module doc names the boxed variants.

**Change:** `OpError` has one variant per message below. Fields use `ApiFunction`, `LuaType`,
`Quoted`, `PathShape` (Task 2), `InvalidName`, `Location` (an earlier call, displayed
`<file>:<line>` inside the extension, e.g. `adapter.lua:9`), `ToolName`, `PackName`, `BinaryName`,
`std::io::Error`, and `String` only for a field key as printed (`path`, `covers[2]`, `foo`), an
item id, a key path, `<root>`, or text another program printed.

At a call, location `<tool>/<file>:<line>` (spec §13 row 1; the nearest Lua frame, §16 R5); text
starts `<function>: ` (§3.3 header):

| Variant | Displays (after the location and `: `) | Spec |
|---|---|---|
| `ArgumentType { function, expected, got }` | `<function>: expected <expected>, got <got>` | §3.3 row "argument is one table"; §16 R3 `require: expected string, got <type>` |
| `Required { function, field }` | `<function>: <field>: required` | §3.3 row "required field present" |
| `FieldType { function, field, expected, got }` | `<function>: <field>: expected <expected>, got <got>` | §3.3 row "field type matches exactly"; §16 R3 |
| `NotList { function, field }` | `<function>: <field>: expected a list` | §3.3 row "`covers` is a list"; §16 R3 |
| `KeyType { function, got }` | `<function>: keys must be strings, got <got>` | §16 R3 |
| `UnknownKey { function, key }` | `<function>: <key>: unknown key` | §3.3 row "no unknown field" |
| `NotUtf8 { function, field }` | `<function>: <field>: not valid UTF-8` | §3.3 row "`path` is UTF-8"; §16 R3 |
| `Empty { function, field }` | `<function>: <field>: empty` | §3.3 row "`skip.reason`" (`reason: empty`) |
| `OpPath { function, field, shape: PathShape }` | for `PathShape::Empty`: `<function>: <field>: empty`; for every other shape: `<function>: <field> <shape>`, e.g. `<function>: path "/x" must be relative` | §3.3 row "`path` shape" (all four forms); §16 R25 |
| `PathManaged { function, path: Quoted }` | `<function>: path "<p>" is managed by agentstacks` | §3.3 row "own files" |
| `ProposedAt { function, path: Quoted, earlier: Location }` | `<function>: path "<p>" is already proposed at <earlier>` | §3.3 row "not proposed twice" |
| `ProposedByTool { function, path: Quoted, tool: ToolName }` | `<function>: path "<p>" is already proposed by <tool>` | same row |
| `ProposedByCore { function, path: Quoted, pack: PackName }` | `<function>: path "<p>" is already proposed by core (root file of <pack>)` | same row |
| `ProposedByCoreRuntime { function, path: Quoted, tool: ToolName }` | `<function>: path "<p>" is already proposed by core (runtime.lua of <tool>)` | §16 R6 |
| `NotInPlan { function, field, id: Quoted }` | `<function>: <field> "<id>" is not in this plan` | §3.3 row "name an id in this run's payload" |
| `NotAFile { id: Quoted }` | `copy: file "<id>" is not a file` | §16 R13 |
| `CoversSkill { field, id: Quoted }` | `write: <field>: "<id>" is a skill; list its files` (`<field>` is `covers[<i>]`) | §16 R13, R28(m) |
| `InvalidBinaryName(InvalidName)` | `require_binary: name: "<text>" is not a valid name` | §3.3 row "`require_binary.name`" |
| `NotInReads { path: Quoted }` | `read: "<p>" is not in extension.reads` | §3.3 row "`read` path in `extension.reads`" |
| `OutsidePlanHandler { function }` | `<function>: only allowed inside the plan handler` | §3.3 row "called outside the `plan` handler" |
| `ReadOutside { path: Quoted }` | `read: "<p>" resolves outside the project` | §4.3 `read` rules |
| `SkipAfterCover { item: Quoted, earlier: Location }` | `skip: item "<id>" is already covered at <earlier>` | §3.2 case row 2 |
| `CoverAfterSkip { function, id: Quoted, earlier: Location }` | `<function>: covers "<id>", which <earlier> skipped` (for a `copy`, `<id>` is its `file`) | §3.2 case row 2, §16 R28(l) |
| `ModuleNameDotted { name: Quoted, dotted: String }` | `require: invalid module name "<name>" (use dots: <dotted>)` | §4.5 row 1; §16 R19 (only when the name contains `/`) |
| `ModuleName { name: Quoted }` | `require: invalid module name "<name>"` | §16 R19 |
| `ModuleNotFound { module: Quoted, tool: ToolName }` | `require: module "<module>" not found in extension <tool>` | §4.5 row 2 |
| `EntryModule { name: Quoted }` | `require: "<name>" is the entry file` | §4.5 row 3 |
| `ModuleCycle { module: Quoted, chain: Vec<String> }` | `require: module "<module>" requires itself (<chain joined by " → ">)` | §4.5 row 4; located at the call that closes the cycle (§16 R19) |
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
| `ValuesPath { key, shape: PathShape }` | `<key>: <shape>`, e.g. `values.root: "/x" must be relative`, `values.root: empty`; the key is part of the text, so this variant is located at `<tool>/<entry>` alone (§13 row 3), giving the same line as a key location | §4.4 `values` row 3 ("a valid path (§3.3 path rows)"); §16 R4 |
| `NotASkill` | `not a skill in this run` | §4.4 `values` row 5 |
| `NotAnAgent` | `not an agent in this run` | §16 R4 |
| `NotAVar` | `not a var in this run` | §4.4 `values` row 5 |

At `agentstacks.toml` (spec §13 rows 6-7):

| Variant | Displays | Spec |
|---|---|---|
| `PackBinaryNotFound { name: BinaryName }` | `binary "<name>" not found on PATH` | §10 row 1 (key `packs.<pack>`) |
| `CoreProposedByTool { path: Quoted, tool: ToolName }` | `path "<p>" is already proposed by <tool>` | §16 R6 (key `tools.<tool whose core write it is>`) |
| `AirslNotFound { packs: Vec<PackName> }` | `airsl not found on PATH (needed by <packs joined by ", ">)` | §10 row 3 (no key) |
| `AirslVersionText { text }` | `airsl --version printed "<text>", expected "airsl <version>"` | §10 row 3, §16 R2 (no key; `"airsl <version>"` is literal; the caller trims `<text>`; an airsl that exits non-zero counts) |
| `AirslSpawn(std::io::Error)` | `airsl: <io::Error Display>` | §16 R1 (no key) |

**Proof:** written first, seen failing, then passing. One test per message, each asserting the
`Diagnostic` display verbatim:
- `op_error_uncovered_item` — `F("claude/adapter.lua")`: `claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a reason)` (§3.2) and `claude/adapter.lua: core/agent/coder got no operation (copy it, write it, or skip it with a reason)` (§13 row 3).
- `op_error_skip_after_cover` — `L("claude/adapter.lua", 14)`, earlier `L("adapter.lua", 9)`: `claude/adapter.lua:14: skip: item "core/skill/status" is already covered at adapter.lua:9` (§3.2).
- `op_error_cover_after_skip` — `L("claude/adapter.lua", 14)`, earlier `L("adapter.lua", 9)`, function `write`: `claude/adapter.lua:14: write: covers "core/agent/coder", which adapter.lua:9 skipped` (§3.2, §16 R28(l)); function `copy`, id `core/skill/status/SKILL.md`: `claude/adapter.lua:14: copy: covers "core/skill/status/SKILL.md", which adapter.lua:9 skipped`.
- `op_error_translate_missing` — `F("claude/adapter.lua")`, root `.claude/agentstacks`: `claude/adapter.lua: lua hooks need .claude/agentstacks/translate.lua (write it)` (§3.2, "with `<root>` filled in").
- `op_error_argument_type` — `L("claude/adapter.lua", 12)`: `claude/adapter.lua:12: write: expected table, got string`; `L("claude/adapter.lua", 9)`: `claude/adapter.lua:9: read: expected string, got number` (§3.3 row 1); `L("claude/adapter.lua", 1)`: `claude/adapter.lua:1: require: expected string, got number` (§16 R3).
- `op_error_required_field` — `claude/adapter.lua:12: write: path: required`, `claude/adapter.lua:12: write: content: required`, `claude/adapter.lua:12: copy: file: required`, `claude/adapter.lua:12: skip: item: required`, `claude/adapter.lua:12: skip: reason: required`, `claude/adapter.lua:12: require_binary: name: required` (§3.3 row 2, "same shape for `content`, `file`, `item`, `reason`, `name`").
- `op_error_field_type` — `claude/adapter.lua:12: write: path: expected string, got number` (§3.3 row 3 and §13 row 1), `claude/adapter.lua:12: write: covers: expected table, got string`, `claude/adapter.lua:12: write: covers[2]: expected string, got boolean`.
- `op_error_field_not_a_list` — `claude/adapter.lua:12: write: covers: expected a list` (§3.3 row 4, §16 R3).
- `op_error_keys_must_be_strings` — `claude/adapter.lua:12: write: keys must be strings, got number` (§16 R3).
- `op_error_unknown_key` — `claude/adapter.lua:12: write: foo: unknown key`.
- `op_error_field_not_utf8` — `claude/adapter.lua:12: write: path: not valid UTF-8` (§3.3 row 6) and `claude/adapter.lua:12: skip: reason: not valid UTF-8` (§16 R3).
- `op_error_op_path_empty` — `OpPath` with field `path`, `PathShape::Empty`: `claude/adapter.lua:12: write: path: empty` (§3.3 row 7, §16 R25).
- `op_error_op_path_must_be_relative` — `OpPath`, `PathShape::MustBeRelative`: `claude/adapter.lua:12: write: path "/x" must be relative` (§3.3 row 7) and `claude/adapter.lua:12: write: path "/etc/x" must be relative` (§3.3 example).
- `op_error_op_path_outside_project` — `OpPath`, `PathShape::OutsideProject`: `claude/adapter.lua:12: write: path "../x" must stay inside the project`.
- `op_error_op_path_separator` — `OpPath`, `PathShape::Separator`: `claude/adapter.lua:12: write: path "a\b" must use / as the separator` (one backslash between `a` and `b`).
- `op_error_path_managed` — `claude/adapter.lua:12: write: path "agentstacks.lock" is managed by agentstacks` and the same with `agentstacks.toml`.
- `op_error_path_proposed_at` — earlier `L("adapter.lua", 9)`: `claude/adapter.lua:12: write: path ".claude/x" is already proposed at adapter.lua:9`.
- `op_error_path_proposed_by_tool` — tool `opencode`: `claude/adapter.lua:12: write: path ".claude/x" is already proposed by opencode`.
- `op_error_path_proposed_by_core` — pack `core`: `claude/adapter.lua:12: write: path ".claude/x" is already proposed by core (root file of core)`.
- `op_error_path_proposed_by_core_runtime` — tool `claude`, path `.claude/agentstacks/runtime.lua`: `claude/adapter.lua:12: write: path ".claude/agentstacks/runtime.lua" is already proposed by core (runtime.lua of claude)` (§16 R6).
- `op_error_core_write_proposed_by_tool` — `K("agentstacks.toml", "tools.opencode")`, tool `claude`, path `.claude/x`: `agentstacks.toml: tools.opencode: path ".claude/x" is already proposed by claude` (§16 R6).
- `op_error_id_not_in_plan` — `claude/adapter.lua:12: copy: file "core/skill/x/SKILL.md" is not in this plan`, `claude/adapter.lua:12: skip: item "core/skill/x/SKILL.md" is not in this plan`, `claude/adapter.lua:12: write: covers[1] "core/skill/x/SKILL.md" is not in this plan` (§3.3 row 10, "same shape for `item`, `covers[1]`").
- `op_error_copy_needs_a_file` — `claude/adapter.lua:12: copy: file "core/agent/coder" is not a file` (§16 R13).
- `op_error_covers_names_a_skill` — `claude/adapter.lua:12: write: covers[1]: "core/skill/status" is a skill; list its files` (§16 R13, R28(m)).
- `op_error_reason_empty` — `claude/adapter.lua:12: skip: reason: empty`.
- `op_error_binary_name_invalid` — built from `BinaryName::new("Git")`'s error: `claude/adapter.lua:12: require_binary: name: "Git" is not a valid name`.
- `op_error_read_not_in_reads` — `claude/adapter.lua:9: read: ".env" is not in extension.reads`.
- `op_error_outside_plan_handler` — `claude/adapter.lua:3: write: only allowed inside the plan handler` and `claude/adapter.lua:3: read: only allowed inside the plan handler` (§3.3 row 14, "ops and `read`").
- `op_error_read_resolves_outside` — `claude/adapter.lua:9: read: ".claude/settings.json" resolves outside the project` (§4.3).
- `op_error_no_values_handler` — `claude/adapter.lua: registers no values handler`.
- `op_error_no_plan_handler` — `claude/adapter.lua: registers no plan handler`.
- `op_error_values_type` — `K("claude/adapter.lua", "values")`: `claude/adapter.lua: values: expected table, got nil`; key `values.agents`: `claude/adapter.lua: values.agents: expected table, got list`; key `values.skills."core/status".dir`: `claude/adapter.lua: values.skills."core/status".dir: expected string, got number`.
- `values_required_is_the_field_shape` — `K("claude/adapter.lua", "values.root")` with `FieldError::Required`: `claude/adapter.lua: values.root: required` (§4.4 row 3, §13 row 2); key `values.skills."core/status".dir`: `claude/adapter.lua: values.skills."core/status".dir: required`, and the same with `.invoke` (§16 R12).
- `op_error_values_path_empty` — `F("claude/adapter.lua")`, `ValuesPath` with key `values.root`, `PathShape::Empty`: `claude/adapter.lua: values.root: empty` (§16 R4).
- `values_unknown_key_is_the_field_shape` — with `FieldError::UnknownKey`: `claude/adapter.lua: values.foo: unknown key` and `claude/adapter.lua: values.skills."core/status".icon: unknown key`.
- `op_error_values_path_must_be_relative` — `F("claude/adapter.lua")`, `ValuesPath` with key `values.root`, `PathShape::MustBeRelative`: `claude/adapter.lua: values.root: "/x" must be relative` (§4.4).
- `op_error_values_path_outside_project` — `ValuesPath`, `PathShape::OutsideProject`: `claude/adapter.lua: values.root: "../x" must stay inside the project` (§16 R4).
- `op_error_values_path_separator` — `ValuesPath`, `PathShape::Separator`: `claude/adapter.lua: values.root: "a\b" must use / as the separator` (§16 R4).
- `op_error_not_a_skill_in_run` — `claude/adapter.lua: values.skills."core/x": not a skill in this run`.
- `op_error_not_an_agent_in_run` — `claude/adapter.lua: values.agents."core/x": not an agent in this run` (§16 R4).
- `op_error_not_a_var_in_run` — `claude/adapter.lua: values.vars.home: not a var in this run`.
- `op_error_module_name_with_slash` — dotted `lib.x`: `claude/adapter.lua:1: require: invalid module name "lib/x" (use dots: lib.x)` (§4.5).
- `op_error_module_name_without_slash` — `claude/adapter.lua:1: require: invalid module name "x y"` (§16 R19).
- `op_error_module_not_found` — `claude/adapter.lua:1: require: module "lib.x" not found in extension claude`.
- `op_error_entry_module` — `claude/adapter.lua:1: require: "adapter" is the entry file`.
- `op_error_module_cycle` — `L("claude/lib/b.lua", 1)`, chain `lib.a`, `lib.b`, `lib.a`: `claude/lib/b.lua:1: require: module "lib.a" requires itself (lib.a → lib.b → lib.a)` (§4.5, §16 R19; the arrow is U+2192).
- `op_error_lua_failure_keeps_airsl_text` — `claude/adapter.lua: lua error in adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>` and `claude/adapter.lua: lua error in plan: adapter.lua:3: path must be relative` (§4.6 row 3).
- `op_error_limit_breach_keeps_airsl_text` — `` claude/adapter.lua: script `plan` exceeded its instruction limit of 10000000 `` (§4.6 row 4).
- `op_error_unknown_event_keeps_airsl_text` — `` claude/adapter.lua: lua error in adapter.lua: event `nope` is not one this host dispatches; declared events: plan, values `` (§4.6 row 5, events `plan, values`).
- `extension_file_not_utf8_is_the_shared_text_refusal` — `F("claude/lib/x.lua")` with the error `utf8_text` returns for the bytes `0x66 0xFF`: `claude/lib/x.lua: not valid UTF-8` (§16 R3).
- `op_error_pack_binary_not_found` — `K("agentstacks.toml", "packs.core")`: `agentstacks.toml: packs.core: binary "rg" not found on PATH`.
- `op_error_extension_binary_not_found` — `L("claude/adapter.lua", 7)`: `claude/adapter.lua:7: require_binary: "node" not found on PATH`.
- `op_error_airsl_not_found` — `F("agentstacks.toml")`, packs `core`, `sdlc`: `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)` (§10, §13 row 7).
- `op_error_airsl_version_unparsed` — text `airsl-dev`: `agentstacks.toml: airsl --version printed "airsl-dev", expected "airsl <version>"` (§10, §16 R2).
- `op_error_airsl_spawn` — `F("agentstacks.toml")`, an `io::Error` of kind `PermissionDenied` with message `permission denied`: `agentstacks.toml: airsl: permission denied` (§16 R1).
- `op_error_converts_and_displays_unchanged` (in `report/format.rs`) — `FormatError::from(OpError::NoPlanHandler)` displays `registers no plan handler`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): operation and extension refusals`.

### Task 4 — Word every JSON edit refusal

**Files:**
- Create `crates/agentstacks/src/types/json/error.rs` — `//!` block, `JsonEditError`, tests.
- Modify `crates/agentstacks/src/types/json/mod.rs` — `pub mod error;`,
  `pub use error::JsonEditError;`, doc bullet.
- Modify `crates/agentstacks/src/types/report/format.rs` — `JsonEdit(#[from] JsonEditError)`.

**Change:** Location `<tool>/<file>:<line>` of the caller (spec §5 header, §13 row 1). Every
variant's text starts `json.edit: `:

| Variant | Displays | Spec |
|---|---|---|
| `TextType { got: LuaType }` | `json.edit: text: expected string, got <got>` | §5 rules row "arguments" |
| `TextNotUtf8` | `json.edit: text: not valid UTF-8` | §16 R3 |
| `PathType { got: LuaType }` | `json.edit: path: expected table, got <got>` | §5 rules row "arguments" |
| `PathNotList` | `json.edit: path: expected a list` | §16 R3 |
| `Parse(ParserMessage)` | `json.edit: <serde_json message>` | §5 rules row "`text` parses as JSON" |
| `TopLevel` | `json.edit: top level must be an object` | §5 rules row "top level is an object" |
| `PathEmpty` | `json.edit: path: empty` | §5 rules row "`path` is a non-empty list" |
| `PathItemType { index, got: LuaType }` | `json.edit: path[<index>]: expected string, got <got>` (1-based, as Lua counts) | same row |
| `NotAnObject { key: Quoted }` | `json.edit: "<key>" is not an object` | §5 rules row "setting"; removing through such a parent too (§16 R14) |
| `MixedTable` | `json.edit: value: table mixes list and keyed entries` | §5 Lua value table row 4 |
| `Nan` | `json.edit: value: NaN cannot be JSON` | §5 Lua value table row 5 |
| `Infinite` | `json.edit: value: inf cannot be JSON` | §5 Lua value table row 5, "(also `inf`)" |
| `NegInfinite` | `json.edit: value: -inf cannot be JSON` | §16 R3 |
| `StringNotUtf8` | `json.edit: value: string is not valid UTF-8` | §5 Lua value table row 5 |
| `KeyNotUtf8` | `json.edit: value: key is not valid UTF-8` | §16 R3 |
| `Cyclic` | `json.edit: value: table contains itself` | §16 R3 |
| `NotJson { got: LuaType }` | `json.edit: value: <got> cannot be JSON` (function, userdata, thread) | §5 Lua value table row 6; §16 R3 |

`ParserMessage` is the built carrier of a parser's own words (`src/types/report/parser.rs`). Which
row wins when several fail (the first row of the §5 table, §16 R14) is the json plan's rule.

**Proof:** written first, seen failing, then passing. Location `L("claude/adapter.lua", 5)` in each:
- `json_edit_error_text_type` — `claude/adapter.lua:5: json.edit: text: expected string, got nil`.
- `json_edit_error_text_not_utf8` — `claude/adapter.lua:5: json.edit: text: not valid UTF-8` (§16 R3).
- `json_edit_error_path_type` — `claude/adapter.lua:5: json.edit: path: expected table, got string`.
- `json_edit_error_path_not_a_list` — `claude/adapter.lua:5: json.edit: path: expected a list` (§16 R3).
- `json_edit_error_parse_keeps_serde_text` — message `trailing comma at line 1 column 8` (`probes/json-results.md`): `claude/adapter.lua:5: json.edit: trailing comma at line 1 column 8`.
- `json_edit_error_top_level` — `claude/adapter.lua:5: json.edit: top level must be an object`.
- `json_edit_error_path_empty` — `claude/adapter.lua:5: json.edit: path: empty`.
- `json_edit_error_path_item_type` — index 2: `claude/adapter.lua:5: json.edit: path[2]: expected string, got number`.
- `json_edit_error_not_an_object` — `claude/adapter.lua:5: json.edit: "hooks" is not an object` (§5, §16 R14).
- `json_edit_error_mixed_table` — `claude/adapter.lua:5: json.edit: value: table mixes list and keyed entries`.
- `json_edit_error_nan` — `claude/adapter.lua:5: json.edit: value: NaN cannot be JSON`.
- `json_edit_error_inf` — `claude/adapter.lua:5: json.edit: value: inf cannot be JSON`.
- `json_edit_error_negative_inf` — `claude/adapter.lua:5: json.edit: value: -inf cannot be JSON` (§16 R3).
- `json_edit_error_string_not_utf8` — `claude/adapter.lua:5: json.edit: value: string is not valid UTF-8`.
- `json_edit_error_key_not_utf8` — `claude/adapter.lua:5: json.edit: value: key is not valid UTF-8` (§16 R3).
- `json_edit_error_table_contains_itself` — `claude/adapter.lua:5: json.edit: value: table contains itself` (§16 R3).
- `json_edit_error_value_not_json` — `claude/adapter.lua:5: json.edit: value: function cannot be JSON`, and the same with `userdata` and `thread` (§5, §16 R3).
- `json_edit_error_converts_and_displays_unchanged` (in `report/format.rs`) — `FormatError::from(JsonEditError::TopLevel)` displays `json.edit: top level must be an object`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): json edit refusals`.

### Task 5 — Word every file step refusal

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
| `ChangedAfterRead` | `changed after it was read; run plan again` | §16 R15 |
| `Outside` | `resolves outside the project` | §6.3 step 3 and last paragraph; §6.2 checked example 8; at apply time too (§16 R21) |
| `ChangedAfterPlan` | `changed after plan; run plan again` | §7 step 3 |

**Proof:** written first, seen failing, then passing.
- `step_error_not_regular` — `F(".claude/x")`: `.claude/x: is not a regular file`.
- `step_error_changed` — `F(".claude/skills/status/SKILL.md")`: `.claude/skills/status/SKILL.md: changed since the last apply` (§6.2 checked example 2).
- `step_error_not_ours` — `F(".claude/agents/coder.md")`: `.claude/agents/coder.md: exists and was not written by agentstacks` (§6.2 checked example 7).
- `step_error_changed_not_deleting` — `F(".claude/x")`: `.claude/x: changed since the last apply; not deleting it`.
- `step_error_changed_after_read` — `F(".claude/settings.json")`: `.claude/settings.json: changed after it was read; run plan again` (§16 R15).
- `step_error_outside` — `F(".claude/x")`: `.claude/x: resolves outside the project` (§6.3, §16 R21); and `K("agentstacks.toml", "packs.team-lint.path")`: `agentstacks.toml: packs.team-lint.path: resolves outside the project` (§6.3).
- `step_error_changed_after_plan` — `F(".claude/x")`: `.claude/x: changed after plan; run plan again` (§7, §13 row 5).
- `step_error_converts_and_displays_unchanged` (in `report/format.rs`) — `FormatError::from(StepError::Changed)` displays `changed since the last apply`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): file step refusals`.

### Task 6 — Word every source fetch refusal

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
| `GitSpawn(std::io::Error)` | `git: <io::Error Display>` | §16 R1 (the caller locates it at `agentstacks.toml` + `packs.<pack>` / `tools.<tool>`) |
| `CacheIo { path: PathBuf, error: std::io::Error }` | `<io::Error Display>` (the caller locates it with `Location::file(path)`, giving `<path>: <io::Error Display>`; for a failed rename `path` is the `<rev>.git` target) | §16 R1, R28(b) |
| `GitFailed { subcommand, stderr }` | `git <subcommand> failed: <stderr>` (the caller passes stderr already trimmed) | §9.2 failure row 2 |
| `GitUnreadable { subcommand, first_line }` | `git <subcommand> printed output agentstacks cannot read: "<first line>"` (the caller decodes lossy UTF-8 and trims, §16 R2) | §16 R2 |
| `SubdirNotFound { subdir: Quoted, at }` | `"<subdir>" not found at <tag or rev>` | §9.2 failure row 4 |
| `SubdirIsFile { subdir: Quoted }` | `"<subdir>" is a file, not a directory` | §9.2 failure row 5 |
| `HomeUnset` | `HOME is not set, so the cache (~/.airsstack/cache) has no home` (unset or empty, reported once, §16 R10) | §9.3 (no key; a unit variant built unlocated, the caller locates it at `agentstacks.toml`) |

No variant holds its own location: the port that returns a `FetchError` (the git and cache
adapters) builds it unlocated, and the flow that turns it into a `Diagnostic` adds the
`agentstacks.toml` location of the source, or the carried path for `CacheIo`.

`GitNotFound` carries the list of keys as a public field and may be built with an empty
`needed_by`: the `Git` port answers per source with an empty list, and the plan and update flows
merge the answers, filling in the key of every source that met it, into the single refusal (spec
§9.2 "once"); this enum only joins the keys it is given. A refusal shown to the user always holds
at least one key.

**Proof:** written first, seen failing, then passing.
- `fetch_error_tree_mismatch` — `K("agentstacks.toml", "packs.core")`, found and locked two distinct `ContentHash` values, rev a 40-hex `CommitId`: `agentstacks.toml: packs.core: tree sha256:<found hex> at rev <rev> differs from the lock's sha256:<locked hex>` with the three values filled in.
- `fetch_error_git_not_found` — `F("agentstacks.toml")`, keys `packs.core`, `tools.claude`: `agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)`.
- `fetch_error_git_not_found_starts_empty` — `FetchError::GitNotFound { needed_by: Vec::new() }` builds; after the keys `packs.core` and `tools.claude` are pushed into its `needed_by`, it displays at `F("agentstacks.toml")` as `agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)`.
- `fetch_error_git_spawn` — `K("agentstacks.toml", "packs.core")`, an `io::Error` of kind `PermissionDenied` with message `permission denied`: `agentstacks.toml: packs.core: git: permission denied` (§16 R1).
- `fetch_error_cache_io_is_located_at_its_path` — `CacheIo` with path `/home/u/.airsstack/cache/git/<hex>/<rev>.git` (a fixed 64-hex and 40-hex value) and an `io::Error` of kind `PermissionDenied` with message `permission denied`; `Location::file` of the carried path: `/home/u/.airsstack/cache/git/<hex>/<rev>.git: permission denied` with the two values filled in (§16 R1).
- `fetch_error_git_failed` — key `packs.core`: `agentstacks.toml: packs.core: git fetch failed: fatal: couldn't find remote ref refs/tags/packs-v9.9.9` (§9.2, G7; §13 row 6 names `git fetch failed`) and `agentstacks.toml: packs.core: git ls-remote failed: fatal: could not read Username for 'https://github.com': terminal prompts disabled` (§9.2, G1 is the `ls-remote` step).
- `fetch_error_git_unreadable` — key `packs.core`, subcommand `rev-parse`, first line `nonsense`: `agentstacks.toml: packs.core: git rev-parse printed output agentstacks cannot read: "nonsense"` (§16 R2).
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
- `FormatError` boxing: if the gate's clippy reports `result_large_err` or `large_enum_variant` for
  `JsonEditError` or `StepError` too, the same hand-written `From` with a `Box` applies; nothing
  else changes.
- Not here: the doctor findings of spec §8 and §16 R23's `outside the project: <path>` are findings,
  not refusals, so their text belongs to the steps plan's `Finding`, with the other §8 rows; R23's
  I/O form is the built `FormatError::Io` named above. The notes of §9.1, §9.4 and §16 R20 belong
  to the types that report them.

## Proof

- `cargo make dod` is green at the end of Task 6.
- The four path-shape wordings exist only in `PathShape` (`op/shape.rs`); `OpPath` and
  `ValuesPath` wrap it, and no other variant repeats them.
- Spec rows covered: §3.2 case rows 1, 2, 5 (rows 3-4 are accepted cases, no message); §3.3 every
  refusal row and the example; §3.3 type words; §4.3 `read` outside; §4.4 `values` rows 1-7,
  missing `plan` handler, type words incl. `list`; §4.5 rows 1-4; §4.6 rows 3-5 (rows 1-2 are the
  §3.3 messages and `JsonEditError`); §5 every refusal; §6.2 both tables' refusals and checked
  examples 2, 7, 8; §6.3 both outside forms; §7 step 3 and the I/O rows (built `FormatError::Io`);
  §9.1 row 2; §9.2 failure rows 1, 2, 4, 5 (row 3 is git's own refusal, row 6 reuses row 2); §9.3
  home; §10 rows 1-3 (out of range: built `SetError::AirslInstalled`); §13 every example; §16 R1,
  R2, R3, R4, R6, R12, R13, R14, R15, R19, R21, R23 (I/O form), R25, R28(b), R28(l), R28(m).

## Review findings

Reviewer (one round, batch with plan 03): plan 02 share 0🔴 2🟡 6🔵. The batch's `cargo make dod` passed with exit 0 (804 unit, 31 integration, 233 doctests).

- open 🟡 `report/phrase.rs:42`: `Quoted`, `MustBeRelative` and `OutsideProject` have `pub(in crate::types)` constructors. As a result, `FetchError::SubdirNotFound`/`SubdirIsFile`, `PathShape` and the `Quoted`-carrying `OpError` variants cannot be built from `process/` or `host/`, although `resolve/error.rs:3` says the adapters build them. This is settled where the adapters are built (plans 11–12).
- open 🟡 The `"<text>" must use / as the separator` wording now lives in four enums (`extension/error.rs`, `lock/error.rs`, `op/shape.rs`, `manifest/invalid.rs`). It is deferred to the cross-check.
- open 🔵 `json/error.rs:63`: `PathItemType.index` is 1-based but typed `usize`.
- open 🔵 `json/error.rs:259`, `op/error.rs` (`op_error_required_field`, `op_error_outside_plan_handler`): the expected text is built from the `Display` under test.
- open 🔵 `op/error.rs:29`: `path_suffix` allocates on every `Display`.
- open 🔵 `op/error.rs:531`: a redundant re-assert after the loop.
- open 🔵 `resolve/error.rs:264`: `fetch_error_converts_and_displays_unchanged` is duplicated; the copy in `report/format.rs` is the planned one.
- open 🔵 `types/mod.rs:226`: the new error enums are not re-exported at the `types` root. This is accepted, because `types/mod.rs` was not in this plan's files.

## Deviations

- 2026-11-22: One coder built the whole plan while plan 03 was built in parallel. Plan 03's files were red mid-run and green at the final gate.
- 2026-11-22: The new types are reachable at `types::{op,json,plan,resolve}::…` and not at the `types` root, because `types/mod.rs` belonged to plan 03.
