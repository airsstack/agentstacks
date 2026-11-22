---
status: done
created: 2026-11-22
depends-on: [02]
built-on: f39b2a814319
---

# JSON edit Implementation Plan

**Goal:** Edit a JSON document's text at one key path.

**Architecture:** `types/json` is a pure rule module (spec §2, `docs/architecture.md` "Where logic
lives"). It takes text in and gives text or one refusal out. It never touches disk and never sees a
Lua value. Spec §16 R14 says the first failing row of the §5 table wins, so `edit` owns the row
order:
- the host (plan 12) checks row 1, the argument types, and hands over raw parts: the text, each
  path element as a key or as the bad element's type word, and a deferred value conversion;
- `edit` checks rows 2–7 in order, and only then runs the conversion (spec §5 second table, plan
  12's code).

The module has four sibling files, each with one job (modularity Rule 1):

```text
crates/agentstacks/src/types/json/
├─ mod.rs     docs + `mod` / `pub use` only (mod-rs-export-only)
├─ error.rs   JsonEditError                         (plan 02, not touched here)
├─ indent.rs  Indent: the indent detected from the input text (§5 "indent" row)
├─ print.rs   Ending + print: pretty-printing with an indent, LF, trailing newline (§5 rows 9–11)
├─ elem.rs    PathElem: one raw path element, a string key or the bad element's type word
└─ edit.rs    edit: parse, walk the key path, set or remove, then print (§5 rows 2–8, 12–13)
```

Public API: `edit<F>(text: &str, path: &[PathElem], value: F) -> Result<String, JsonEditError>`,
where `F: FnOnce() -> Result<Option<serde_json::Value>, JsonEditError>`. The arguments:
- `PathElem` is `Key(String) | NotString(LuaType)`, with `LuaType` from plan 02;
- `value` is called at most once, after every first-table row has passed;
- `Ok(None)` from `value` removes the key (Lua `nil`), and its `Err` (a §5 second-table refusal)
  is returned as is.

`Indent`, `Ending` and `print` stay `pub(in crate::types::json)`. They are building blocks of
`edit` and are not exported.

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6):
- `mod-rs-export-only.md`: `json/mod.rs` holds only the four-question `//!` block, `mod` lines,
  `pub use edit::edit;` and `pub use elem::PathElem;`.
- `modularity.md` Rule 1: one job per file (above). Rule 2: no second JSON value or error type.
  `serde_json::Value` is the value, and plan 02's `JsonEditError` is the only refusal type.
- `unit-test-mandate.md`: `indent.rs`, `print.rs`, `elem.rs` and `edit.rs` each carry a colocated
  `#[cfg(test)] mod tests`.
- `strong-types.md`: no `bool` parameter. The trailing-newline decision is the two-variant enum
  `Ending`, and the indent is the `Indent` newtype.
- `static-dispatch.md`: no trait objects. Nothing here needs a port.
- `doc-comment-discipline.md`: docs describe the behaviour, cite symbols rather than lines, and
  never name plans, specs or agents.
- Naming: one-word files, no `_`, no repeated folder word.

**Tech Stack:** Rust 2024, `serde_json = 1.0.154` with `preserve_order` and without
`arbitrary_precision` (workspace dependency, added by plan 01; spec §5 "Dependency", §15). The
indent formatter is `serde_json::ser::PrettyFormatter::with_indent` (spec §5 indent row,
`probes/round3-results.md` J1).

**Content authority:** spec §5, every row of the first table plus the checked example. The
second table (Lua value → JSON) and the argument row are plan 12's.

---

## Files that change

```
crates/agentstacks/src/types/json/mod.rs     — modify: module docs; `mod edit; mod indent; mod print;` and `pub use edit::edit;`
crates/agentstacks/src/types/json/indent.rs  — create: Indent and its detection from the input text
crates/agentstacks/src/types/json/print.rs   — create: Ending and print (pretty-printing with an indent, LF, trailing newline)
crates/agentstacks/src/types/json/elem.rs    — create: PathElem
crates/agentstacks/src/types/json/edit.rs    — create: edit (row order, parse, key-path walk, set/remove, print)
```

No other file changes. `json/error.rs` and the `mod error; pub use …` lines in `json/mod.rs`
belong to plan 02, which lands first.

## Order of work

### Task 1 — Detect the indent of the input text

**Files:**
- Create `crates/agentstacks/src/types/json/indent.rs` — `Indent`.
- Modify `crates/agentstacks/src/types/json/mod.rs` — `mod indent;` and the module docs (the
  four-question block: what, why it exists, responsibilities, non-responsibilities — the Lua value
  conversion and argument reading belong to the host).

**Change:** `Indent` is a `pub(in crate::types::json)` newtype over the indent text, so `print`
never takes a bare string. `Indent::detect(text: &str) -> Indent` applies spec §5 "indent":
- Find the first line after the first one that starts with a space or a tab. That is "the first
  indented line", as the probe's `edit` finds it (`probes/examples-main.rs`, `edit`: skip one
  line, then the first line starting with `' '` or `'\t'`).
- Take its leading run of spaces and tabs.
- If that run is all spaces, or exactly one tab, it is the indent.
- Otherwise (mixed, or more than one tab), or when no line after the first is indented, the
  indent is two spaces.

Lines are split on `\n`, and a trailing `\r` is never part of the leading run, so CRLF text
detects the same indent as LF text. `Indent::as_bytes` hands the bytes to
`PrettyFormatter::with_indent`.

**Proof:** written first, seen failing, then passing.
- `indent_takes_two_spaces_from_the_first_indented_line` — `"{\n  \"a\": 1\n}"` detects `"  "`.
- `indent_takes_four_spaces_from_the_first_indented_line` — the `probes/round3-results.md` J1
  source text (`"{\n    \"permissions\": …"`) detects `"    "`.
- `indent_takes_one_tab_from_the_first_indented_line` — `"{\n\t\"a\": 1\n}"` detects `"\t"`.
- `indent_falls_back_to_two_spaces_for_mixed_whitespace` — first indented lines led by `"\t  "`
  and by `" \t"` both detect `"  "`.
- `indent_falls_back_to_two_spaces_for_two_tabs` — a first indented line led by `"\t\t"` detects
  `"  "`.
- `indent_falls_back_to_two_spaces_without_an_indented_line` — `"{\"a\":1}"`, `""` and
  `"{\n\"a\": 1\n}"` all detect `"  "`.
- `indent_ignores_the_first_line` — `"    {\n  \"a\": 1\n}"` detects `"  "`, not four spaces.
- `indent_reads_crlf_text_like_lf_text` — the J1 source with every `\n` replaced by `\r\n` detects
  `"    "`.

**Done when:** `cargo test -p agentstacks types::json::indent` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): detect the indent of a JSON text`.

### Task 2 — Print a JSON value with an indent and a line ending

**Files:**
- Create `crates/agentstacks/src/types/json/print.rs` — `Ending`, `print`.
- Modify `crates/agentstacks/src/types/json/mod.rs` — `mod print;`, responsibilities list.

**Change:**
- `Ending` is `Newline | Absent`. `Ending::of(text: &str)` is `Newline` exactly when `text` ends
  with `\n` (so CRLF text ending in `\r\n` is `Newline`) (spec §5 "trailing newline").
- `print(value: &serde_json::Value, indent: &Indent, ending: Ending) -> String` serialises with
  `serde_json::Serializer::with_formatter` over `PrettyFormatter::with_indent(indent.as_bytes())`
  (spec §5 indent row; J1 proves 2 spaces, 4 spaces and tab byte for byte). It then appends one
  `\n` for `Ending::Newline`.
- The serializer writes only `\n`, so every output line ends in LF (spec §5 "line endings").
- Numbers come out in serde_json's normal form, because the parsed `Value` holds no original
  spelling (spec §5 "numbers", D9; `preserve_order` without `arbitrary_precision`).
- Serialising a `Value` into a `Vec<u8>` yields UTF-8. The error arm of `String::from_utf8` and of
  the serializer is unreachable for a `Value`. It is still handled without `unwrap`/`expect`
  (workspace lints deny `unwrap_used` and warn `expect_used`). It maps to the plan 02
  `JsonEditError` variant that carries a serde_json message, so no new message is invented.

**Proof:** written first, seen failing, then passing.
- `ending_of_follows_the_last_byte` — `"{}\n"` and `"{}\r\n"` are `Newline`; `"{}"` and `""` are
  `Absent`.
- `print_uses_the_given_indent` — the J1 value (the J1 source parsed) printed with `"  "`,
  `"    "` and `"\t"` equals the J1 `2sp`, `4sp` and `tab` blocks of
  `probes/round3-results.md`, byte for byte, with `Ending::Absent`.
- `print_appends_one_newline_for_newline_ending` — `{"a":[1]}` with `"\t"` and `Ending::Newline`
  is exactly `"{\n\t\"a\": [\n\t\t1\n\t]\n}\n"`.
- `print_writes_no_carriage_return` — the output of any of the above contains no `\r`.

**Done when:** `cargo test -p agentstacks types::json::print` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): print a JSON value with a detected indent`.

### Task 3 — Edit the text at a key path

**Files:**
- Create `crates/agentstacks/src/types/json/elem.rs` — `PathElem`.
- Create `crates/agentstacks/src/types/json/edit.rs` — `edit`.
- Modify `crates/agentstacks/src/types/json/mod.rs` — `pub mod elem; mod edit;`,
  `pub use elem::PathElem; pub use edit::edit;`, entry-point line in the docs.

**Change:**
- **`PathElem`** is `Key(String) | NotString(LuaType)`. The host builds one per Lua path element:
  a string element becomes its key, and any other element becomes its type word. `edit` then owns
  the element-type refusal, so it is ordered after the parse rows.
- **`edit(text, path, value)`** applies the spec §5 first table in the order of its rows, and the
  first failing row is the one refusal (§16 R14). Row 1 (`text: expected string, got <type>`,
  `path: expected table, got <type>`, and the §16 R3 argument shapes `text: not valid UTF-8` and
  `path: expected a list`) is the host's, checked before the call because `edit` takes a `&str`
  and a slice. The order inside `edit`:
  1. **Empty text** (§5 row 2): when `text` is empty or whitespace-only (`str::trim` is empty), the
     document parsed is `{}`.
  2. **Parse** (§5 row 3): otherwise `serde_json::from_str::<serde_json::Value>(text)`. A failure
     is the `JsonEditError::Parse` variant carrying the serde_json error's Display
     (`json.edit: <serde_json message>`, e.g. `trailing comma at line 1 column 8`,
     `probes/json-results.md` J2).
  3. **Top level** (§5 row 4): a parsed value that is not an object gives `json.edit: top level
     must be an object`.
  4. **Path** (§5 row 5): an empty `path` gives `json.edit: path: empty`. Otherwise the first
     `NotString(word)` at Lua position `i` (1-based) gives `json.edit: path[<i>]: expected string,
     got <word>`, e.g. `path[2]: expected string, got number`.
  5. **Keys** (§5 row 6, §16 R14): walk the keys before the last, read-only. The walk stops at the
     first missing key. An existing key whose value is not an object gives `json.edit: "<key>" is
     not an object` with that key (e.g. `"hooks"`). This holds for setting and for removing alike
     (§16 R14).
  6. **Value**: only now is `value()` called. Its `Err` is returned unchanged.
  7. **Set** (`Some(v)`, §5 row 6): every missing parent is inserted as `{}` and descended into,
     then the last key is inserted. An existing key keeps its position and gets the new value, and
     a new key goes last (§5 row 8; `preserve_order`, `probes/round3-results.md` J3).
  8. **Remove** (`None`, §5 row 7): if a parent or the last key is missing, return `text` exactly
     as it came in, with no re-print. Otherwise remove the last key so that every other key keeps
     its relative order (§5 row 8 "existing keys keep their position"; the probe's `edit` uses
     `shift_remove`, `probes/examples-main.rs`). The rest of the document stays as it is: no
     parent is pruned.
  9. **Print** (§5 rows 9–11): `print(&document, &Indent::detect(text), Ending::of(text))`. Both
     read the original `text`, whitespace-only text included (spec §16 R14: "Whitespace-only text
     takes its indent and trailing newline from the original text").

**Duplicate keys** (§5 row 13) need no code: `serde_json::Value` keeps the last one (`probes/round2-results.md`
R7: `plain Value={"a":2}`). **Numbers** (§5 row 12) need no code: Task 2 prints them in normal
form.

**Proof:** written first, seen failing, then passing. Each refusal test asserts the returned error
is the plan 02 `JsonEditError` variant for that row, and plan 02's own tests pin the Display text.
Notation in the test lines:
- a path written `["a", "b"]` is `[PathElem::Key("a"), PathElem::Key("b")]`;
- a value written `Some(v)` is a closure returning `Ok(Some(v))`;
- a value written `None` is a closure returning `Ok(None)`.
- `path_elem_keeps_a_key_or_a_type_word` (in `elem.rs`) — `Key("a")` holds `a`, and
  `NotString(number)` holds the word `number`.
- `edit_treats_empty_text_as_an_empty_object` — `edit("", ["a"], Some(1))` is exactly
  `"{\n  \"a\": 1\n}"` (§5 row 2).
- `edit_treats_whitespace_only_text_as_an_empty_object` — `edit("  ", ["a"], Some(1))` is exactly
  `"{\n  \"a\": 1\n}"` (§5 row 2).
- `edit_takes_whitespace_only_formatting_from_the_original_text` — `edit("\n", ["a"], Some(1))` is
  exactly `"{\n  \"a\": 1\n}\n"` (trailing newline kept from the original), and
  `edit("\n\t\n", ["a"], Some(1))` is exactly `"{\n\t\"a\": 1\n}\n"` (tab indent from the original)
  (§16 R14).
- `edit_first_failing_row_wins` — the Lua call `json.edit("{,", {}, 1)` reaches `edit` as
  `edit("{,", [], Some(1))` and gives the parse refusal, not `path: empty`. `edit("[1]", [],
  Some(1))` gives the top-level refusal, and `edit("{,", [NotString(number)], Some(1))` gives the
  parse refusal, not the element refusal (§16 R14).
- `edit_refuses_a_path_element_that_is_not_a_string` — `edit("{}", [Key("a"), NotString(number)],
  Some(1))` gives the refusal whose Display is `json.edit: path[2]: expected string, got number`
  (§5 row 5).
- `edit_calls_value_last` — the closure records each call. For each refusal of rows 2–6 above
  (parse, top level, empty path, element type, a parent that is not an object), the closure is
  never called. On a success it is called exactly once.
- `edit_returns_the_value_refusal_unchanged` — a closure returning the plan 02
  `JsonEditError::MixedTable` makes `edit("{}", ["a"], …)` return that same error (§5 second table,
  converted by the host).
- `edit_refuses_text_that_is_not_json` — `edit("{\"a\":1,}", ["b"], Some(1))` is the parse
  refusal whose serde_json message is `trailing comma at line 1 column 8` (§5 row 3, J2).
- `edit_refuses_a_top_level_that_is_not_an_object` — `"[1]"`, `"\"x\""`, `"null"` and `"1"`
  each give the top-level refusal (§5 row 4).
- `edit_refuses_an_empty_path` — `edit("{}", [], Some(1))` gives the `path: empty` refusal
  (§5 row 5).
- `edit_creates_missing_parents_as_objects` — `edit("{}", ["a", "b"], Some(1))` is exactly
  `"{\n  \"a\": {\n    \"b\": 1\n  }\n}"` (§5 row 6).
- `edit_refuses_a_parent_that_is_not_an_object` — `edit("{\"hooks\": []}", ["hooks",
  "PreToolUse"], Some([]))` gives the not-an-object refusal naming `hooks`, and
  `edit("{\"a\": {\"b\": 1}}", ["a", "b", "c"], Some(1))` names `b` (§5 row 6).
- `edit_replaces_the_value_at_the_last_key` — setting `["hooks"]` to `{"x":1}` in
  `{"hooks": [1]}` replaces the array: `"{\n  \"hooks\": {\n    \"x\": 1\n  }\n}"`.
- `edit_returns_the_text_unchanged_when_removing_a_missing_key` — `edit("{\"a\": 0.10}", ["b"],
  None)` returns `"{\"a\": 0.10}"` byte for byte, with no re-print of `0.10` (§5 row 7).
- `edit_returns_the_text_unchanged_when_removing_under_a_missing_parent` — `edit("{\"a\": 0.10}",
  ["x", "y"], None)` returns the input byte for byte (§5 row 7).
- `edit_refuses_removing_through_a_parent_that_is_not_an_object` — `edit("{\"hooks\": []}",
  ["hooks", "x"], None)` gives the not-an-object refusal naming `hooks`, whose Display is
  `json.edit: "hooks" is not an object` (§16 R14).
- `edit_removes_an_existing_key_and_keeps_the_others_in_order` — removing `["b"]` from
  `{"a":1,"b":2,"c":3,"d":4}` is exactly `"{\n  \"a\": 1,\n  \"c\": 3,\n  \"d\": 4\n}"`, so the
  removal does not swap `d` into `b`'s place (§5 rows 7–8).
- `edit_keeps_existing_keys_in_place_and_appends_new_ones` — on the J1 source text, setting
  `["zeta"]` to `2` keeps `zeta` second, and then setting `["new"]` to `[]` puts `new` last. The
  object keys come out as `permissions, zeta, alpha, d, hooks, new`, as J3 shows (§5 row 8).
- `edit_keeps_the_indent_of_the_input` — `edit(J1 source, ["zeta"], Some(1))` equals the J1 `4sp`
  block followed by `\n`, byte for byte. The same source with each leading four spaces replaced by
  two spaces gives the `2sp` block plus `\n`, and with one tab gives the `tab` block plus `\n`
  (§5 row 9, J1).
- `edit_falls_back_to_two_spaces` — `edit("{\"a\":1}", ["b"], Some(2))` is exactly
  `"{\n  \"a\": 1,\n  \"b\": 2\n}"` (§5 row 9).
- `edit_turns_crlf_into_lf` — the J1 source with every `\n` replaced by `\r\n` gives exactly the
  same bytes as the LF source, and the output holds no `\r` (§5 row 10).
- `edit_keeps_a_trailing_newline` — input ending in `\n` gives output ending in exactly one `\n`
  (§5 row 11).
- `edit_adds_no_trailing_newline_when_the_input_had_none` — `edit("{}", ["a"], Some(1))` ends in
  `}` (§5 row 11).
- `edit_reprints_numbers_in_normal_form` — the input `{"alpha": 12345678901234567890123, "d": 0.10}`
  with `["x"]` set to `1` gives exactly these five lines, joined by `\n` with no trailing newline:
  `{`, `  "alpha": 1.2345678901234568e+22,`, `  "d": 0.1,`, `  "x": 1`, then `}` (§5 row 12, D9;
  J1 and `probes/round3-results.md` Run 2).
- `edit_keeps_the_last_of_duplicate_keys` — `edit("{\"a\":1,\"a\":2}", ["b"], Some(true))` is
  exactly `"{\n  \"a\": 2,\n  \"b\": true\n}"` (§5 row 13, R7).
- `edit_matches_the_checked_example` — the spec §5 `old` text (as the Lua literal spells it:
  `{\n    "permissions": {"allow": ["Bash(x:*)"]},\n    "hooks": {}\n}\n`), the path
  `["hooks", "PreToolUse"]` and the value `[{"matcher":"Read"}]` (what the host's conversion makes
  of `{ { matcher = "Read" } }`) give the spec §5 "Checked example" JSON block byte for byte,
  followed by one `\n` (4-space indent, trailing newline kept, the one-line `"allow"` array made
  multi-line). The expected text is the block as `probes/examples-results.txt` printed it
  (`lua[1] output == json[2] block: true`).

**Done when:** `cargo test -p agentstacks types::json` passes and `cargo make dod` is green. Commit `feat(agentstacks): edit a JSON document at a key path`.

---

## Risks

- **Removal order (riskiest step, Task 3).** With `preserve_order`, a removal that is not
  order-preserving would move the last key into the removed key's place. That breaks "existing
  keys keep their position" with no compile error. `edit_removes_an_existing_key_and_keeps_the_others_in_order`
  uses four keys, so a swap is visible. The plan does not assert any serde_json method's semantics.
  The test decides.
- **Byte-for-byte expectations.** Every exact string in Task 2 and Task 3 is either copied from a
  probe output (`probes/round3-results.md` J1, `probes/examples-results.txt`, the spec §5 block) or
  derived from the layout those proven outputs show for `PrettyFormatter::with_indent`:
  - one key per line, with `": "` between key and value;
  - `,` at the end of every member but the last;
  - each nested object and array opens on its key's line and closes on its own line, one indent
    level out;
  - each array element on its own line (J1 `"allow"`, the §5 checked example);
  - empty `{}`/`[]` compact (J1 `"hooks": {}`, `probes/json-results.md` J4);
  - normal-form numbers (J1).

  The literals are part of the test. A mismatch is a failing test to fix in the code, never a
  deviation.
- **Not done here:** row 1 argument reading (`text: expected string, got nil`, `path: expected
  table`, and the §16 R3 shapes `text: not valid UTF-8` and `path: expected a list`), building the
  `PathElem`s, and the Lua value conversion behind the `value` closure (§5 second table, with its
  refusals). All of them are plan 12's.

## Proof

- `cargo make dod` green at the end of Task 3.
- Spec §5 rows covered (first table):
  - row 1 "arguments": plan 12 (the host reads them before the call);
  - row 2 empty/whitespace text: Task 3 (`edit_treats_empty_text_as_an_empty_object`,
    `edit_treats_whitespace_only_text_as_an_empty_object`,
    `edit_takes_whitespace_only_formatting_from_the_original_text`);
  - row 3 parse: Task 3 (`edit_refuses_text_that_is_not_json`);
  - row 4 top level: Task 3 (`edit_refuses_a_top_level_that_is_not_an_object`);
  - row 5 path non-empty and element types: Task 3 (`edit_refuses_an_empty_path`,
    `edit_refuses_a_path_element_that_is_not_a_string`);
  - row 6 set and parents: Task 3 (`edit_creates_missing_parents_as_objects`,
    `edit_refuses_a_parent_that_is_not_an_object`, `edit_replaces_the_value_at_the_last_key`);
  - row 7 remove missing: Task 3 (`edit_returns_the_text_unchanged_when_removing_a_missing_key`,
    `edit_returns_the_text_unchanged_when_removing_under_a_missing_parent`);
  - row 8 order: Task 3 (`edit_keeps_existing_keys_in_place_and_appends_new_ones`,
    `edit_removes_an_existing_key_and_keeps_the_others_in_order`);
  - row 9 indent: Task 1 (all `indent_*` tests), Task 2 (`print_uses_the_given_indent`), Task 3
    (`edit_keeps_the_indent_of_the_input`, `edit_falls_back_to_two_spaces`);
  - row 10 line endings: Task 1 (`indent_reads_crlf_text_like_lf_text`), Task 2
    (`print_writes_no_carriage_return`), Task 3 (`edit_turns_crlf_into_lf`);
  - row 11 trailing newline: Task 2 (`ending_of_follows_the_last_byte`,
    `print_appends_one_newline_for_newline_ending`), Task 3 (`edit_keeps_a_trailing_newline`,
    `edit_adds_no_trailing_newline_when_the_input_had_none`);
  - row 12 numbers: Task 2 (`print_uses_the_given_indent`, via J1), Task 3
    (`edit_reprints_numbers_in_normal_form`);
  - row 13 duplicate keys: Task 3 (`edit_keeps_the_last_of_duplicate_keys`).
- Spec §5 "Checked example": Task 3 (`edit_matches_the_checked_example`).
- Spec §5 "Dependency": no change here. Plan 01 adds `serde_json` with `preserve_order`, and
  nothing in this plan enables `arbitrary_precision`.
- Spec §5 second table: plan 12.
- Spec §16 R14: removal through a non-object parent → Task 3
  (`edit_refuses_removing_through_a_parent_that_is_not_an_object`); whitespace-only formatting →
  Task 3 (`edit_takes_whitespace_only_formatting_from_the_original_text`); first failing row wins →
  Task 3 (`edit_first_failing_row_wins`, `edit_calls_value_last`,
  `edit_returns_the_value_refusal_unchanged`).
- Spec §16 R3 (`json.edit: text: not valid UTF-8`, `json.edit: path: expected a list`, the value
  refusals) and R5 (one refusal per call, located at the nearest Lua frame): plan 12. `edit` returns
  at most one refusal by its signature.

## Review findings

Reviewer: one round, batched with plans 04, 07 and 08. Blocking: none. The batch `cargo make dod` exited 0.

- open 🔵 `print.rs:10`: the doc is stale or contradictory.
- open 🔵 `edit.rs:180`: probe/spec vocabulary in test code.
- open 🔵 The `print` tests depend on `Indent::detect`.

## Deviations

- 2026-11-22: `print` returns `Result<String, JsonEditError>`, not `String`. Its unreachable error arm goes to `JsonEditError::Parse` without `unwrap`/`expect`, which needs a `Result`.
- 2026-11-22: `edit_first_failing_row_wins` asserts `Err(JsonEditError::Parse(_))` by variant, because the plan gives no message text for `{,`.
- 2026-11-22: `Indent::as_bytes` is a `const fn`, as clippy's `missing_const_for_fn` requires.
- 2026-11-22: Module doc links are written `edit()` / `print()`, because rustdoc found each name ambiguous (module/macro).
- 2026-11-22: The red phase for `print`, `elem` and `edit` was seen only in a scratch crate, because plan 04's in-progress files kept the real crate from compiling. As a red check, `shift_remove` → `swap_remove` made `edit_removes_an_existing_key_and_keeps_the_others_in_order` fail. The real crate was green at the wave gate.
