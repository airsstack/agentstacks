---
status: done
created: 2026-11-20
depends-on: [01, 02]
built-on: 3162a314af79
---

# Frontmatter Implementation Plan

**Goal:** Every skill or agent frontmatter block parses into strict, tool-neutral metadata, each
refusal located.

**Architecture:** A Markdown file is handled in three steps, each in its own file under
`crates/agentstacks/src/model/` (one item per file, export-only `mod.rs`):

1. **Split** (`frontmatter.rs`, `Frontmatter`). `Frontmatter::split(file, text)` finds the opening
   `---` on line 1 and the first later `---` line, and returns the YAML block between them plus a
   `BodyStart` (`body_start.rs`): the byte offset and the 1-based file line where the body begins,
   right after the closing `---`. Plan 06 scans only `BodyStart::body(text)` and adds
   `BodyStart::line()` to each token line, so no text is scanned twice (spec §3.5).
2. **Parse** (`Frontmatter::mapping`). The block goes through `serde_saphyr::from_str` into
   `YamlValue` (`yaml_value.rs`), our own enum `{Null, Bool, Int, Float, Str, Seq,
   Map(Vec<(String, YamlValue)>)}` whose `Deserialize` calls `deserialize_any`; serde-saphyr exports
   no generic value type (spec §1.1, §5). A parser error becomes one syntax `Diagnostic` built by
   `yaml_syntax.rs` and stops this file only (spec §5, §10). A block that parses to anything but a
   mapping is refused; empty or comment-only YAML is `Null` (spec §1.1), so it gets
   `frontmatter: expected a mapping, got null`.
3. **Check** (`frontmatter_fields.rs`, `FrontmatterFields`). Keys and types are checked by hand over
   the mapping, never through a `deny_unknown_fields` struct, so every field error in the file is
   pushed into plan 01's `Diagnostics` (spec §5). `FrontmatterFields` holds the rules skills and
   agents share (`name`, `description`, `native`, unknown keys, string and array kinds) and the one
   token scan of frontmatter string values (spec §3.4, §3.5). `SkillMeta` (`skill_meta.rs`) and
   `AgentMeta` (`agent_meta.rs`) add their own keys on top.

Syntax diagnostics (`yaml_syntax.rs`, spec §5 row 2, §1.1): `<parser message>` is
`serde_saphyr::DefaultMessageFormatter.format_message(&err)` (trait `serde_saphyr::MessageFormatter`;
exports `SY/lib.rs:32,44`), the bare message with no location and no snippet. The error's
`Display` is never used. `<line>` is `err.location().line()` plus 1: serde-saphyr counts lines from
the YAML text, and the opening `---` is file line 1, so the sum counts from the file. No column is
ever reported (columns are not stable, spec §1.1). With a location the diagnostic is
`Location::at_line(file, line)`; when `err.location()` is `None` it is `Location::file(file)`, the
`<file>: <parser message>` form. The quoting hint ` (a value starting with {{ must be quoted)` is
appended when a line is reported and that block line has `{{` right after `:`, `-`, `[` or `,`,
with only spaces between (spec §3.4): each `{{` on the line is checked by stepping back over spaces
to the previous character. A `{{` mid-value (after other text) gets no hint, and with no reported
line there is no line to inspect and no hint.

Value kinds (spec §10 type words): `YamlValue::kind_word()` gives the `got` word of plan 01's
`FieldError::WrongType { expected, got }`: `null`, `boolean`, `integer`, `float`, `string`,
`array` (a YAML sequence) and `table` (a YAML mapping). The `expected` word is the spec's: the two
§5 rows that spell a wrong-type message use theirs verbatim (`frontmatter: expected a mapping,
got <type>`, `argument_hint: expected a string, got <type>`); every other field uses the §10
generic shape with a bare type word (`expected string`, `expected array`, `expected table`). YAML
1.1 booleans (`yes`, `on`, `y`, `True`, `no`, `off`) arrive as `Bool` (spec §1.1), so
`argument_hint: yes` is a wrong-type refusal, never a silent string.

Vocabulary enums, one per file: `Invocation` (`user`, `any`; default `any`), `ModelTier` (`fast`,
`balanced`, `deep`), `Effort` (`low`, `medium`, `high`), `AgentTool` (`read`, `write`, `edit`,
`grep`, `glob`, `shell`, `skill`). Each has an `ALL` constant in spec order, `as_str`, `Display`
and `FromStr`; the allowed list in a refusal is `ALL` joined with `", "`. `native` is a
`BTreeMap<ToolName, YamlValue>`: keys are plan 02 `ToolName`s, values stay opaque (spec §5).

Locations and messages: frontmatter field errors use `Location::at_key(file, key_path)` (spec §10);
rows whose spec text has no key path (`missing frontmatter`, `name must be a literal`,
`name "x" does not match "y"`, `unknown key "<k>"`) use `Location::file(file)`. Messages that are
not plan 01 `FieldError` shapes are variants of `FrontmatterError` (`frontmatter_error.rs`), which
joins plan 01's `FormatError` through one `#[from]` variant.

Tokens: after the mapping parses, the free-text frontmatter string values `description` and
`argument_hint` are scanned once each with plan 02's `scan(text, &origin)`, using
`TextOrigin::Field { file, key_path }` with the field's key, so every token and every scan error
is located `<file>: <key>` (spec §10: tokens in frontmatter values take the key-path form, e.g.
`<file>: description: …`). Values under `native.<tool>` are opaque and never scanned (spec §5).
`name` is not scanned: a `{{` in it is refused as non-literal (spec §3.4). The tokens are plan 02
`LocatedToken`s, each carrying `reaches_native` from the scan (spec §2.2); `SkillMeta::tokens()`
and `AgentMeta::tokens()` hand them to plan 06, which stores them on the `Pack`. Placement and
target rules (`{{var arguments}}` only in `SKILL.md`, own-pack and cross-pack targets, `native/`
from neutral content) are not checked here: plans 06 and 10 run them over all tokens (spec §3.5).

Entry points: `SkillMeta::parse(file, text, expected_name)` and
`AgentMeta::parse(file, text, expected_name)`, where `expected_name` is the skill directory name or
the agent file stem as a plan 02 `ItemName` (already validated by plan 06, spec §2.1). Each returns
`Result<SkillMeta | AgentMeta, Vec<Diagnostic>>` through `Diagnostics::into_result`, and the
returned meta exposes `body_start()`.

**Tech Stack:** Rust 2024, `serde` 1 with `derive` (spec §11), `serde-saphyr` 2 as
`{ version = "2", default-features = false, features = ["deserialize"] }` (spec §11; probed at
`2.1.1`, spec §1.1), `thiserror` 2 (added by plan 01).

**Content authority:** spec §5 (all rows), §3.4 (parse before scan, quoting hint, literal `name`),
§3.5 (frontmatter scanned once, body scan starts after the closing `---`), §2.2 (`reaches_native`
recorded by the scanner), §1.1 (serde-saphyr facts: bare message, stable lines, optional location),
§10 (locations, type words, generic shapes, a syntax error stops only that file). Coverage:

| Spec rule | Message (Display) | Task · test |
|---|---|---|
| §5 frontmatter missing, not first, unterminated | `<file>: missing frontmatter` | 2 · `split_refuses_missing_frontmatter` |
| §5 YAML syntax error, with location: line + 1, no column | `<file>:<line>: <parser message>` | 3 · `line_is_offset_to_file_line`, `no_hint_when_reported_line_has_no_token` |
| §5 / §1.1 `<parser message>` is `DefaultMessageFormatter.format_message` | `SKILL.md:2: expected string scalar …` (no `error: line … column …`, no snippet) | 3 · `quoting_hint_on_value_starting_with_token`, `no_hint_when_reported_line_has_no_token` |
| §5 YAML syntax error, no location | `<file>: <parser message>` | 3 · `no_location_gives_file_only_diagnostic_without_hint` |
| §5 / §1.1 duplicate key is a syntax error | `<file>:<line>: duplicate mapping key: …` | 3 · `duplicate_key_is_a_syntax_error` |
| §5 / §10 a syntax error stops this file only | one diagnostic, no field errors | 4 · `skill_meta_syntax_error_stops_the_file` |
| §3.4 quoting hint after `:`, `-`, `[`, `,` (spaces allowed) | `… (a value starting with {{ must be quoted)` | 3 · `hint_after_colon_dash_bracket_or_comma`, `quoting_hint_on_value_starting_with_token`, `quoting_hint_on_list_item_starting_with_token`, `quoting_hint_on_flow_sequence_item` |
| §3.4 no hint for `{{` mid-value | `<file>:<line>: <parser message>` | 3 · `no_hint_for_token_mid_value`, `duplicate_key_on_a_token_line_gets_no_hint` |
| §5 / §1.1 frontmatter is a mapping; empty is null | `<file>: frontmatter: expected a mapping, got null` | 3 · `empty_frontmatter_is_refused_as_null`, `non_mapping_frontmatter_is_refused` |
| §5 `name`, `description` required | `<file>: name: required` / `<file>: description: required` | 4 · `required_string_missing_is_required`; 4, 5 · `*_refuses_each_rule` |
| §5 / §10 not empty (whitespace-only too) | `<file>: description: empty` | 4 · `empty_or_whitespace_string_is_empty` |
| §5 / §3.4 `name` literal | `<file>: name must be a literal` | 4 · `name_must_be_a_literal` |
| §5 `name` = directory / file stem | `<file>: name "x" does not match "y"` | 4 · `name_must_match_expected`; 5 · `agent_meta_refuses_each_rule` |
| §5 unknown key | `<file>: unknown key "disable-model-invocation"` | 4 · `unknown_keys_each_reported_in_order` |
| §5 `invocation` ∈ {user, any}, default any | `<file>: invocation: "manual" (expected user, any)` | 4 · `invocation_refuses_other_values`, `invocation_defaults_to_any` |
| §5 `argument_hint` string; YAML 1.1 bool | `<file>: argument_hint: expected a string, got <type>` | 4 · `string_of_wrong_kind_is_wrong_type` |
| §5 `model` ∈ {fast, balanced, deep} | `<file>: model: "sonnet" (expected fast, balanced, deep)` | 5 · `model_tier_refuses_sonnet` |
| §5 `effort` ∈ {low, medium, high} | `<file>: effort: "max" (expected low, medium, high)` | 5 · `effort_refuses_max` |
| §5 `tools` ⊆ set, no duplicates | `<file>: tools: unknown "bash"` / `<file>: tools: "read" listed twice` | 5 · `agent_tool_refuses_unknown_values`, `agent_meta_refuses_each_rule` |
| §5 `native` keys follow §4.1 | `<file>: native: "Claude" is not a valid tool name` | 4 · `native_keys_must_be_tool_names` |
| §5 `native` contents opaque, never scanned | no token, no diagnostic | 4 · `native_values_are_never_scanned` |
| §5 every field error reported | all diagnostics of one file | 4 · `skill_meta_reports_every_field_error_at_once`; 5 · `agent_meta_reports_every_field_error_at_once` |
| §10 wrong type (generic shape) and type words | `<file>: <key.path>: expected <type>, got <type>` | 1 · `yaml_value_kind_words`; 4 · `string_of_wrong_kind_is_wrong_type`, `native_must_be_a_mapping`; 5 · `agent_meta_refuses_each_rule` |
| §3.4 / §3.5 / §10 frontmatter strings scanned once, located by key path | `<file>: description: unknown token kind "skil"` | 4 · `string_values_are_scanned_with_key_path` |
| §3.5 body scan starts after the closing `---` | `BodyStart` offset and line | 2 · `body_start_slices_body_and_reports_line`, `split_returns_yaml_and_body_start` |

Owned elsewhere: the §5 rule that a field the tool cannot express is dropped with a reason in
`plan`, and `native.<tool>` winning over the neutral field for that tool (`claude-extension`, spec
§13); reading the file and refusing non-UTF-8 content (plan 06); native `.md` files, which are not
held to §5 (plan 06, spec §2.2).

---

## Files that change

```
Cargo.toml                                          — modify  add serde and serde-saphyr to [workspace.dependencies]
crates/agentstacks/Cargo.toml                       — modify  depend on serde, serde-saphyr (workspace)
crates/agentstacks/src/model/mod.rs                 — modify  declare and re-export the new items
crates/agentstacks/src/model/format_error.rs        — modify  FormatError gains a Frontmatter variant
crates/agentstacks/src/model/yaml_value.rs          — create  YamlValue: own YAML value enum filled by deserialize_any
crates/agentstacks/src/model/frontmatter_error.rs   — create  FrontmatterError (thiserror): the §5 messages that are not FieldError shapes
crates/agentstacks/src/model/body_start.rs          — create  BodyStart: byte offset and file line where the body begins
crates/agentstacks/src/model/frontmatter.rs         — create  Frontmatter: split the --- block, parse it into a mapping
crates/agentstacks/src/model/yaml_syntax.rs         — create  syntax_diagnostic: parser message + line → located Diagnostic, quoting hint
crates/agentstacks/src/model/frontmatter_fields.rs  — create  FrontmatterFields: hand checks of keys and types, token scan of free-text values
crates/agentstacks/src/model/invocation.rs          — create  Invocation: user | any (default any)
crates/agentstacks/src/model/skill_meta.rs          — create  SkillMeta::parse
crates/agentstacks/src/model/model_tier.rs          — create  ModelTier: fast | balanced | deep
crates/agentstacks/src/model/effort.rs              — create  Effort: low | medium | high
crates/agentstacks/src/model/agent_tool.rs          — create  AgentTool: read | write | edit | grep | glob | shell | skill
crates/agentstacks/src/model/agent_meta.rs          — create  AgentMeta::parse
```

## Order of work

### Task 1 — Add `YamlValue`

**Files:**
- Modify `Cargo.toml` — add `serde = { version = "1", features = ["derive"] }` and
  `serde-saphyr = { version = "2", default-features = false, features = ["deserialize"] }` to
  `[workspace.dependencies]` (spec §11).
- Modify `crates/agentstacks/Cargo.toml` — `serde = { workspace = true }`,
  `serde-saphyr = { workspace = true }` under `[dependencies]`.
- Create `crates/agentstacks/src/model/yaml_value.rs` — `YamlValue` with variants `Null`,
  `Bool(bool)`, `Int(i64)`, `Float(f64)`, `Str(String)`, `Seq(Vec<YamlValue>)`,
  `Map(Vec<(String, YamlValue)>)` (source key order kept). Its hand-written `Deserialize` calls
  `deserialize_any` with a visitor implementing `visit_bool`, `visit_i64`, `visit_u64`,
  `visit_f64`, `visit_str`, `visit_string`, `visit_unit`, `visit_none`, `visit_some`, `visit_seq`
  and `visit_map` (map keys read as `String`, the probed shape, spec §1.1). A `visit_u64` value
  above `i64::MAX` is refused with serde's `invalid_value` error. `kind_word()` returns the spec
  §10 type word for each variant, listed in Architecture. `as_str()` returns `Some` only for `Str`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare `yaml_value`, re-export `YamlValue`.

**Change:** a generic YAML value the frontmatter checks can walk by hand (spec §5, §1.1).
Duplicate keys never reach it: serde-saphyr refuses them by default (spec §1.1), so `Map` keys are
unique.

**Proof** (colocated `#[cfg(test)] mod tests` in `yaml_value.rs`):
- `yaml_value_fills_every_kind_from_nested_mapping` — the spec §1.1 probe input
  `name: x / n: 3 / f: 1.5 / b: true / z: null / l: [a, 1] / m: {k: v}` (one key per line, `m` a
  block mapping) parses through `serde_saphyr::from_str` to `Map` of `name` `Str("x")`, `n`
  `Int(3)`, `f` `Float(1.5)`, `b` `Bool(true)`, `z` `Null`, `l` `Seq([Str("a"), Int(1)])`, `m`
  `Map([("k", Str("v"))])`, in that order.
- `yaml_value_keeps_key_order` — `z: 1`, `a: 2` comes out with `z` before `a`.
- `yaml_value_reads_yaml_1_1_booleans_and_nulls` — `k: yes`, `k: on`, `k: y`, `k: True` give
  `Bool(true)`; `k: no`, `k: off` give `Bool(false)`; `k: ~`, `k: null` give `Null` (spec §1.1).
- `yaml_value_empty_input_is_null` — `""`, a whitespace-only text and `# just a comment` each parse
  to `Null` (spec §1.1).
- `yaml_value_resolves_aliases` — `a: &x 1`, `b: *x` gives `b` = `Int(1)` (spec §1.1).
- `yaml_value_refuses_unsigned_beyond_i64` — deserialising `YamlValue` from serde's own `u64`
  value deserializer holding `i64::MAX + 1` is an error; `i64::MAX` gives `Int(i64::MAX)`.
- `yaml_value_kind_words` — each variant's `kind_word()` is `null`, `boolean`, `integer`, `float`,
  `string`, `array`, `table` respectively (spec §10: a sequence is `array`, a mapping is `table`).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green;
`cargo make deny` passes (new dependencies). Commit
`feat(agentstacks): add YamlValue for frontmatter parsing`.

### Task 2 — Split the frontmatter block from the body

**Files:**
- Create `crates/agentstacks/src/model/frontmatter_error.rs` — `FrontmatterError` (`thiserror`)
  with variants and displays: `Missing` → `missing frontmatter`; `Syntax { message, quoting_hint }`
  → the message, followed by ` (a value starting with {{ must be quoted)` when `quoting_hint` is
  set; `NameNotLiteral` → `name must be a literal`; `NameMismatch { found, expected }` →
  `name "<found>" does not match "<expected>"`; `UnknownKey { key }` → `unknown key "<key>"`;
  `NotOneOf { value, allowed }` → `"<value>" (expected <allowed>)`; `UnknownTool { value }` →
  `unknown "<value>"`; `DuplicateTool { value }` → `"<value>" listed twice`;
  `InvalidToolName { value }` → `"<value>" is not a valid tool name`. All wording is spec §3.4 and
  §5.
- Modify `crates/agentstacks/src/model/format_error.rs` — add
  `Frontmatter(#[from] FrontmatterError)`, `#[error(transparent)]`.
- Create `crates/agentstacks/src/model/body_start.rs` — `BodyStart` with private `offset` (byte
  offset into the file text) and `line` (`NonZeroUsize`, the 1-based file line of the first body
  line); `line()` and `body(text)`, which returns the slice of `text` from `offset`.
- Create `crates/agentstacks/src/model/frontmatter.rs` — `Frontmatter<'a>` holding the file path,
  the YAML block text and a `BodyStart`. `Frontmatter::split(file, text)` returns it, or a
  `Diagnostic` at `Location::file(file)` with `FrontmatterError::Missing` when line 1 is not a
  `---` line or no later `---` line closes the block. A delimiter is a line whose text, without its
  line ending, is exactly `---`; the first such line after line 1 closes the block, and later `---`
  lines belong to the body. Accessors `yaml()` and `body_start()`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export `FrontmatterError`,
  `BodyStart`, `Frontmatter`.

**Change:** spec §5 row 1, and the body boundary of spec §3.5 that plan 06 scans from.

**Proof** (colocated `#[cfg(test)] mod tests` in `frontmatter_error.rs`):
- `frontmatter_error_messages_match_spec` — table over every variant: `missing frontmatter`;
  `name must be a literal`; `name "x" does not match "y"`; `unknown key
  "disable-model-invocation"`; `NotOneOf` with `manual` and `user, any` displays `"manual"
  (expected user, any)`; `unknown "bash"`; `"read" listed twice`; `"Claude" is not a valid tool
  name`.
- `syntax_error_appends_quoting_hint_only_when_set` — `Syntax` with message
  `expected string scalar` displays `expected string scalar (a value starting with {{ must be
  quoted)` with the hint set and `expected string scalar` without it.

**Proof** (colocated `#[cfg(test)] mod tests` in `body_start.rs`):
- `body_start_slices_body_and_reports_line` — a `BodyStart` at offset 16, line 4 over
  `---\nname: x\n---\n# Title\n` returns body `# Title\n` and line 4.

**Proof** (colocated `#[cfg(test)] mod tests` in `frontmatter.rs`):
- `split_returns_yaml_and_body_start` — `---\nname: x\n---\n# Title\n` gives block `name: x\n`,
  body `# Title\n` and body line 4.
- `split_accepts_closing_line_at_end_of_file` — `---\nname: x\n---` (no final newline) gives
  block `name: x\n` and an empty body.
- `split_stops_at_first_closing_line` — a body holding its own `---` line keeps that line in the
  body.
- `split_refuses_missing_frontmatter` — table, file `SKILL.md`: text with no `---`, a `---` on
  line 2 after a blank line, and an opening `---` with no closing line each give a `Diagnostic`
  whose `Display` is `SKILL.md: missing frontmatter`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): split frontmatter from the Markdown body`.

### Task 3 — Parse the block into a YAML mapping

**Files:**
- Create `crates/agentstacks/src/model/yaml_syntax.rs` — `syntax_diagnostic(file, block,
  reported_line: Option<u64>, message: &str) -> Diagnostic`, the rule in Architecture: location
  `Location::at_line(file, reported_line + 1)` (converted to `NonZeroUsize`; a value that does not
  fit, unreachable for real text, falls back to the file-only form) or `Location::file(file)`;
  message passed through unchanged; quoting hint when the reported block line has `{{` right after
  `:`, `-`, `[` or `,`, spaces between allowed. Module visibility `pub(crate)`.
- Modify `crates/agentstacks/src/model/frontmatter.rs` — `Frontmatter::mapping()` runs
  `serde_saphyr::from_str::<YamlValue>` on the block. An error goes through `syntax_diagnostic` with
  `err.location().map(|l| l.line())` and
  `serde_saphyr::DefaultMessageFormatter.format_message(&err)` (spec §1.1, §5). A `Map` is returned
  as its entries. Any other value is refused at `Location::at_key(file, "frontmatter")` with
  `FieldError::WrongType { expected: "a mapping", got: kind_word }` (spec §5 wording).
- Modify `crates/agentstacks/src/model/mod.rs` — declare `yaml_syntax` (`pub(crate)`).

**Change:** spec §5 rows 2 and 3, and the §3.4 quoting hint. The test inputs below are the probed
inputs of spec §1.1 placed alone in the block, so each file line is the probed line plus 1.

**Proof** (colocated `#[cfg(test)] mod tests` in `yaml_syntax.rs`; pure, no parser; file
`SKILL.md`):
- `line_is_offset_to_file_line` — reported line 1 over block `a: [1, 2` with message
  `unclosed bracket '['` gives `Display` `SKILL.md:2: unclosed bracket '['`.
- `hint_after_colon_dash_bracket_or_comma` — table, each block line reported with message
  `expected string scalar`: `description: {{skill status}} runs it` (after `:`),
  `description:   {{skill status}}` (after `:` and spaces), `  - {{skill x}}` (after `-`),
  `tools: [{{skill x}}]` (after `[`) and `tools: [a,  {{skill x}}]` (after `,` and spaces) each
  give a `Display` ending in `expected string scalar (a value starting with {{ must be quoted)`.
- `no_hint_for_token_mid_value` — over block `name: x\ndescription: run {{skill status}} now`,
  reported line 2 with message `expected string scalar` gives exactly
  `SKILL.md:3: expected string scalar` (the `{{` follows `run`); reported line 1 (no `{{`) gives
  `SKILL.md:2: expected string scalar`.
- `no_location_gives_file_only_diagnostic_without_hint` — `reported_line` `None`, block
  `description: {{skill status}}` and message `unexpected input` give `Display`
  `SKILL.md: unexpected input` with no hint (spec §5 row 2, no-location form).

**Proof** (colocated `#[cfg(test)] mod tests` in `frontmatter.rs`; real serde-saphyr; file
`SKILL.md`; each asserts the full `Display`, which also proves the message is the bare
`format_message` text with no `error: line … column …` prefix, no column and no snippet):
- `quoting_hint_on_value_starting_with_token` — block `description: {{skill status}} runs it`
  gives `SKILL.md:2: expected string scalar (a value starting with {{ must be quoted)` (spec §1.1
  rows 1 and 3).
- `quoting_hint_on_list_item_starting_with_token` — block `tools:\n  - {{skill x}}` gives
  `SKILL.md:3: expected string scalar (a value starting with {{ must be quoted)` (spec §1.1 row 1).
- `quoting_hint_on_flow_sequence_item` — block `tools: [a, {{skill x}}]` gives `SKILL.md:2:
  expected string scalar (a value starting with {{ must be quoted)` (spec §1.1 row 2).
- `no_hint_when_reported_line_has_no_token` — block `a: [1, 2` gives `SKILL.md:2: unclosed bracket
  '['` with no hint (spec §1.1 row 4).
- `duplicate_key_is_a_syntax_error` — block `a: 1\na: 2` gives `SKILL.md:3: duplicate mapping key:
  a, set DuplicateKeyPolicy in Options if acceptable` (spec §1.1 row 3, the `format_message` text).
- `duplicate_key_on_a_token_line_gets_no_hint` — block `a: run {{skill x}}\na: run {{skill y}}`
  (the second line has `{{` mid-value) gives `SKILL.md:3: duplicate mapping key: a, set
  DuplicateKeyPolicy in Options if acceptable`, with no hint.
- `quoted_token_value_parses` — block `description: "{{skill status}} runs it"` gives one entry,
  `description` = `Str("{{skill status}} runs it")` (spec §1.1 row 1).
- `empty_frontmatter_is_refused_as_null` — `---\n---\n` and a block holding only `# a comment`
  each give `SKILL.md: frontmatter: expected a mapping, got null` (spec §5, §1.1).
- `non_mapping_frontmatter_is_refused` — block `- a` gives `SKILL.md: frontmatter: expected a
  mapping, got array`; block `just text` gives `SKILL.md: frontmatter: expected a mapping, got
  string` (spec §5 wording, §10 type words).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse frontmatter YAML with located errors`.

### Task 4 — Check skill frontmatter

**Files:**
- Create `crates/agentstacks/src/model/frontmatter_fields.rs` — `FrontmatterFields`
  (`pub(crate)`), built from the file path and the mapping entries, holding a `Diagnostics` and a
  `Vec<LocatedToken>`. Methods:
  - `required_string(key)` / `optional_string(key, expected)` — missing → `FieldError::Required`
    (required only); a non-`Str` value → `FieldError::WrongType { expected, got: kind_word }`,
    where `expected` is `string` (spec §10 generic) except for `argument_hint`, whose caller passes
    `a string` (spec §5 row); blank → `FieldError::check_not_empty` (plan 01); all at
    `Location::at_key(file, key)`.
  - `optional_array(key)` — a non-`Seq` → `WrongType` with `array`; a non-`Str` item at key path
    `<key>[<i>]` → `WrongType` with `string`.
  - `name(expected)` — required string; contains `{{` → `NameNotLiteral` at `Location::file`, and
    no mismatch check; else differs from `expected` → `NameMismatch` at `Location::file`. Returns
    the `ItemName`.
  - `native()` — optional; a non-`Map` → `WrongType` with `table` at key `native`; each key parsed
    as a plan 02 `ToolName`, an `InvalidName` mapped to `InvalidToolName` at key `native`; returns
    `BTreeMap<ToolName, YamlValue>`. The values are never scanned (spec §5).
  - `scan_text(key, value)` — runs plan 02's `scan(value, &TextOrigin::Field { file, key_path:
    key })` once and keeps the returned `LocatedToken`s and `Diagnostic`s, all located
    `<file>: <key>` (spec §10).
  - `finish(allowed_keys)` — pushes `UnknownKey { key }` at `Location::file` for each mapping key
    not in `allowed_keys`, in source order, and returns the diagnostics and tokens.
- Create `crates/agentstacks/src/model/invocation.rs` — `Invocation::{User, Any}`, `Default` =
  `Any`, `ALL`, `as_str`, `Display`, `FromStr` refusing any other text with
  `FrontmatterError::NotOneOf { value, allowed: "user, any" }`.
- Create `crates/agentstacks/src/model/skill_meta.rs` — `SkillMeta { name: ItemName, description,
  invocation, argument_hint: Option<String>, native, tokens: Vec<LocatedToken>, body_start }`.
  `SkillMeta::parse(file, text, expected_name)`: split (a refusal is the file's only diagnostic),
  `mapping()` (a syntax or not-a-mapping refusal is the file's only diagnostic, spec §5, §10), then
  `name`, `description` (required string, scanned), `invocation` (optional string through
  `Invocation::from_str`, refusal at key `invocation`), `argument_hint` (optional string with
  expected word `a string`, scanned), `native` (opaque), and `finish` with the keys `name`,
  `description`, `invocation`, `argument_hint`, `native`. Returns `Diagnostics::into_result`.
  Accessors for every field.
- Modify `crates/agentstacks/src/model/mod.rs` — declare `frontmatter_fields` (`pub(crate)`);
  declare and re-export `Invocation`, `SkillMeta`.

**Change:** spec §5 rows for skills, §3.4 (literal `name`, scan after parse), §3.5 (frontmatter
values scanned once, tokens kept with their location), §10 (key-path location for tokens in
frontmatter values).

**Proof** (colocated `#[cfg(test)] mod tests` in `frontmatter_fields.rs`; file `SKILL.md`; each
asserts the full `Display`):
- `required_string_missing_is_required` — a mapping without `name` gives `SKILL.md: name:
  required`; one without `description` gives `SKILL.md: description: required`.
- `string_of_wrong_kind_is_wrong_type` — `description: 3` gives `SKILL.md: description: expected
  string, got integer`; `argument_hint: yes` gives `SKILL.md: argument_hint: expected a string, got
  boolean` (spec §5, YAML 1.1 boolean); `argument_hint: [a]` gives `… expected a string, got
  array`; `argument_hint: {k: v}` gives `… expected a string, got table`; `argument_hint: ~` gives
  `… expected a string, got null`.
- `empty_or_whitespace_string_is_empty` — `description: ""` and `description: "   "` each give
  `SKILL.md: description: empty`; `name: ""` gives `SKILL.md: name: empty`.
- `unknown_keys_each_reported_in_order` — `disable-model-invocation: true` then
  `argument-hint: x` give `SKILL.md: unknown key "disable-model-invocation"` then
  `SKILL.md: unknown key "argument-hint"`.
- `name_must_be_a_literal` — `name: "{{skill x}}"` with expected `x` gives exactly one diagnostic,
  `SKILL.md: name must be a literal`.
- `name_must_match_expected` — `name: x` with expected `y` gives `SKILL.md: name "x" does not match
  "y"`.
- `native_keys_must_be_tool_names` — `native: { Claude: { model: x } }` gives `SKILL.md: native:
  "Claude" is not a valid tool name`.
- `native_must_be_a_mapping` — `native: [a]` gives `SKILL.md: native: expected table, got array`.
- `string_values_are_scanned_with_key_path` — `description: "run {{skil x}}"` gives `SKILL.md:
  description: unknown token kind "skil"`; `description: "run {{skill sdlc/status}} now"` gives no
  diagnostic and one `LocatedToken` of kind `skill` whose location displays `SKILL.md:
  description`; `argument_hint: "{{var arguments}}"` gives one `LocatedToken` of kind `var` whose
  location displays `SKILL.md: argument_hint`.
- `native_values_are_never_scanned` — `native: { claude: { args: ["{{skil x}}", "{{root}}/x"] } }`
  gives no diagnostic and no `LocatedToken`.

**Proof** (colocated `#[cfg(test)] mod tests` in `invocation.rs`):
- `invocation_parses_each_value` — `user` and `any` parse and display back unchanged.
- `invocation_refuses_other_values` — `manual` and `User` are refused; `manual` displays
  `"manual" (expected user, any)`.
- `invocation_defaults_to_any` — `Invocation::default()` is `Any`.

**Proof** (colocated `#[cfg(test)] mod tests` in `skill_meta.rs`; file
`skills/discuss/SKILL.md`, expected name `discuss`; each refusal asserts the full `Display`):
- `skill_meta_parses_spec_example` — the spec §5 skill example (`name: discuss`, a description,
  `invocation: user`, `argument_hint: "[list | <id> | done]"`, `native: { claude: { model: x } }`)
  parses with `Invocation::User`, that argument hint, and one `native` entry for `claude`.
- `skill_meta_defaults_invocation_to_any` — `name` and `description` only gives `Invocation::Any`
  and no argument hint.
- `skill_meta_refuses_each_rule` — table, one frontmatter per row: no `name` →
  `skills/discuss/SKILL.md: name: required`; no `description` → `…: description: required`;
  `description: "  "` → `…: description: empty`; `name: "{{skill discuss}}"` → `…: name must be a
  literal`; `name: talk` → `…: name "talk" does not match "discuss"`;
  `disable-model-invocation: true` → `…: unknown key "disable-model-invocation"`;
  `invocation: manual` → `…: invocation: "manual" (expected user, any)`; `argument_hint: yes` →
  `…: argument_hint: expected a string, got boolean`; `native: { Claude: { model: x } }` →
  `…: native: "Claude" is not a valid tool name`.
- `skill_meta_refuses_agent_only_keys` — `model: fast` gives `…: unknown key "model"`.
- `skill_meta_reports_every_field_error_at_once` — one frontmatter with no `name`, an empty
  `description`, `invocation: manual` and an unknown key gives all four diagnostics.
- `skill_meta_keeps_frontmatter_tokens_and_body_start` — a skill whose description holds
  `{{skill sdlc/status}}` and whose body starts on line 5 returns one `LocatedToken` and
  `body_start().line()` 5; a token in the body is not in `tokens()`.
- `skill_meta_syntax_error_stops_the_file` — a frontmatter with an unquoted
  `description: {{skill status}} runs it` and no `name` gives exactly one diagnostic, the syntax
  one.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): check skill frontmatter`.

### Task 5 — Check agent frontmatter

**Files:**
- Create `crates/agentstacks/src/model/model_tier.rs` — `ModelTier::{Fast, Balanced, Deep}`,
  `ALL`, `as_str`, `Display`, `FromStr` refusing other text with `NotOneOf { value, allowed:
  "fast, balanced, deep" }`.
- Create `crates/agentstacks/src/model/effort.rs` — `Effort::{Low, Medium, High}`, same shape,
  allowed `low, medium, high`.
- Create `crates/agentstacks/src/model/agent_tool.rs` — `AgentTool::{Read, Write, Edit, Grep,
  Glob, Shell, Skill}`, `ALL`, `as_str`, `Display`, `FromStr` refusing other text with
  `FrontmatterError::UnknownTool { value }`.
- Create `crates/agentstacks/src/model/agent_meta.rs` — `AgentMeta { name: ItemName, description,
  model: Option<ModelTier>, effort: Option<Effort>, tools: Option<Vec<AgentTool>>, native, tokens,
  body_start }`. `AgentMeta::parse(file, text, expected_name)` follows `SkillMeta::parse` for
  split, mapping, `name`, `description`, `native` and `finish`, with the keys `name`,
  `description`, `model`, `effort`, `tools`, `native`. `model` and `effort` are optional strings
  (expected word `string`) through `FromStr`, refused at their own key. `tools` is an optional
  array; each string item through `AgentTool::from_str`, a refusal at key `tools`; a tool already
  seen is refused with `DuplicateTool { value }` at key `tools`. Accessors for every field.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export `ModelTier`, `Effort`,
  `AgentTool`, `AgentMeta`.

**Change:** spec §5 rows for agents.

**Proof** (colocated `#[cfg(test)] mod tests` in `model_tier.rs`):
- `model_tier_parses_each_value` — `fast`, `balanced`, `deep` parse and display back unchanged.
- `model_tier_refuses_sonnet` — `sonnet` displays `"sonnet" (expected fast, balanced, deep)`.

**Proof** (colocated `#[cfg(test)] mod tests` in `effort.rs`):
- `effort_parses_each_value` — `low`, `medium`, `high` parse and display back unchanged.
- `effort_refuses_max` — `max` displays `"max" (expected low, medium, high)`.

**Proof** (colocated `#[cfg(test)] mod tests` in `agent_tool.rs`):
- `agent_tool_parses_each_value` — all seven names parse and display back unchanged.
- `agent_tool_refuses_unknown_values` — `bash` displays `unknown "bash"`; `Read` displays
  `unknown "Read"`.

**Proof** (colocated `#[cfg(test)] mod tests` in `agent_meta.rs`; file `agents/coder.md`, expected
name `coder`; each refusal asserts the full `Display`):
- `agent_meta_parses_spec_example` — the spec §5 agent example (`name: coder`, a description,
  `model: balanced`, `effort: high`, `tools: [read, edit, write, grep, glob, shell, skill]`,
  `native: { claude: { model: claude-opus-5-5 } }`) gives `ModelTier::Balanced`, `Effort::High`,
  the seven tools in that order, and `native` `claude` = `Map([("model",
  Str("claude-opus-5-5"))])`.
- `agent_meta_allows_missing_optional_fields` — `name` and `description` only gives no model, no
  effort and no tools.
- `agent_meta_refuses_each_rule` — table: no `name` → `agents/coder.md: name: required`; no
  `description` → `…: description: required`; `name: codr` → `…: name "codr" does not match
  "coder"`; `model: sonnet` → `…: model: "sonnet" (expected fast, balanced, deep)`; `effort: max`
  → `…: effort: "max" (expected low, medium, high)`; `tools: [bash]` → `…: tools: unknown "bash"`;
  `tools: [read, read]` → `…: tools: "read" listed twice`; `tools: read` → `…: tools: expected
  array, got string`; `tools: [1]` → `…: tools[0]: expected string, got integer`; `model: 3` →
  `…: model: expected string, got integer`.
- `agent_meta_refuses_skill_only_keys` — `invocation: user` gives `…: unknown key "invocation"`.
- `agent_meta_reports_every_field_error_at_once` — `model: sonnet`, `effort: max` and
  `tools: [bash, read, read]` in one frontmatter give all four diagnostics.
- `agent_meta_keeps_frontmatter_tokens` — a description holding `{{agent core/coder}}` returns
  one `LocatedToken` whose location displays `agents/coder.md: description`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green, then
`cargo make dod` passes for the workspace. Commit `feat(agentstacks): check agent frontmatter`.

---

## Risks

- The parser message is serde-saphyr's own `DefaultMessageFormatter` wording (spec §1.1), and
  message text depends on the target type (`YamlValue` here, the probed shape). A serde-saphyr
  release that rewords a message, or moves a reported line, fails the exact-`Display` tests of
  Task 3 on upgrade instead of shipping silently.
- serde-saphyr's `MessageFormatter::format_message` docs say that calling it directly bypasses
  the control-character neutralising its renderers do. This plan passes the message through
  unchanged, as spec §5 says; escaping before writing to a terminal belongs to whoever prints
  diagnostics (`cli-commands`).
- The quoting-hint text contains `{{`, which a `thiserror` format string must escape; a mistake
  there is caught by `syntax_error_appends_quoting_hint_only_when_set`.
- Options not taken: deserialising into a `deny_unknown_fields` struct (stops at the first error,
  spec §1.1, §5); an extra YAML value crate (serde-saphyr is the parser claudestacks already uses,
  spec §11); stripping a prefix from the error's `Display` (columns there are unstable and the
  snippet is multi-line, spec §1.1); scanning `name` or the fixed-vocabulary fields for tokens (a
  `{{` in `name` is refused as non-literal, and any `{{` in `invocation`, `model`, `effort` or
  `tools` already fails that field's vocabulary check, so scanning them would report the same
  mistake twice).

## Proof

- `cargo make dod-crate agentstacks` green after every task; `cargo make deny` green after Task 1;
  `cargo make dod` green after Task 5.
- Every §5 row, the §3.4 quoting-hint and literal-`name` rules, the §10 type words and key-path
  token location, and the §3.5 frontmatter and body boundary map to a named test in the coverage
  table above.
- 59 colocated unit tests: `yaml_value.rs` 7, `frontmatter_error.rs` 2, `body_start.rs` 1,
  `frontmatter.rs` 13, `yaml_syntax.rs` 4, `frontmatter_fields.rs` 10, `invocation.rs` 3,
  `skill_meta.rs` 7, `model_tier.rs` 2, `effort.rs` 2, `agent_tool.rs` 2, `agent_meta.rs` 6.

## Review findings

Reviewer (one round; 23 mutants, 18 killed): blocking none; code 0🔴 9🟡 11🔵 2❓, plan 2🟡 2🔵. Fixer: clippy and rustdoc `-D warnings` green, `cargo test --all-targets` → 248 passed, 0 failed (fmt red only in plan-05 files mid-edit at that time).

- 🟡 "spec order" in rustdoc (agent_tool, effort, invocation, model_tier) — fixed ("declaration order").
- 🟡 test names with "spec" — fixed (renamed, see Deviations).
- 🟡 frontmatter.rs:33 CRLF delimiter unguarded (mutant survived) — fixed: `split_accepts_crlf_delimiters`; fails with `strip_suffix('\r')` mutated, passes restored.
- 🟡 skill_meta.rs:87 `argument_hint` token scan unguarded (mutant survived) — fixed: `skill_meta_scans_argument_hint_for_tokens`, `skill_meta_refuses_malformed_token_in_argument_hint` (`argument_hint: unknown token kind "skil"`); both fail with the scan call deleted.
- 🔵 case sensitivity unguarded (mutants survived) — fixed: `effort_matching_is_case_sensitive`, `model_tier_matching_is_case_sensitive`.
- ❓ `tools: []` → `Some([])` unpinned — pinned: `agent_meta_keeps_an_empty_tools_list_distinct_from_absent`.
- ❓ YAML 1.1 names (`on`, `y`, `no`, …) read as booleans — pinned: `skill_meta_refuses_a_name_yaml_reads_as_boolean` (`name: expected string, got boolean`); follows the YAML 1.1 rule the plan states. Authors must quote such names.
- 🔵 skill_meta.rs:103 / agent_meta.rs:91 could return `Err(vec![])` — fixed (`let … else` with `debug_assert!(!refusals.is_empty())`).
- 🔵 frontmatter.rs:9 stray `//!`, frontmatter.rs:69 unreachable fallback, frontmatter_error.rs:36 doc, frontmatter_fields.rs:46 visibility, string_wording.rs exemption line — fixed.
- 🔵 frontmatter_fields.rs:76 `value()` clones per lookup — open (returning a reference needs a split-borrow restructure; not a contained change).
- 🔵 skill_meta/agent_meta and the four vocabulary enums repeat structure — open (modularity nit).
- 🟡 frontmatter.rs:101 serde-saphyr messages passed through carry Rust API advice (`cannot deserialize null into string; use Option<String>`, `multiple YAML documents detected; use from_str_multiple …`) — open: the plan requires the parser message unchanged; flagged for the author.
- 🟡 yaml_value.rs:86 `native` integer beyond i64 but within f64 (`99999999999999999999`) is stored as `Float(1e20)`; `18446744073709551615` refuses the file (as the plan requires) — open, flagged for the author.
- 🟡/🔵 plan text not amended for deviations — recorded under Deviations below; ❓ per-task commits absent — by design.

## Deviations

- 2026-11-20 — Run shape: one coder ran Tasks 1–5 in order after plans 01/02 were built, concurrent with plan 03's review/fix; review and fix concurrent with plan 05's coder.
- 2026-11-20 — `Cargo.toml` declares `serde-saphyr = "2"` (as the plan); the resolver picked 2.2.0, so `Cargo.lock` was pinned with `cargo update -p serde-saphyr --precise 2.1.1` (the probed version; tests assert its wording). Reviewer: all model tests also pass on 2.2.0.
- 2026-11-20 — `FieldError::WrongType` takes a closed `TypeWord`, so `expected a mapping` / `expected a string` are `FrontmatterError::NotAMapping{got}` / `NotAString{got}`; displayed messages identical to the plan's.
- 2026-11-20 — `optional_string` takes `StringWording::{Generic, WithArticle}` instead of an `expected` word; `FrontmatterError::not_one_of` helper added.
- 2026-11-20 — Helper modules (`frontmatter_fields`, `string_wording`, `yaml_syntax`) are private `mod` with `pub(super)` items (clippy `redundant_pub_crate`).
- 2026-11-20 — `SkillMeta::parse` / `AgentMeta::parse` take `&Path` and `&ItemName`.
- 2026-11-20 — Red-first not observed for frontmatter_fields, skill_meta, agent_meta and the vocabulary enums (tests written with the code); reviewer mutation run covered them, survivors fixed above.
- 2026-11-20 — Test renames: `agent_meta_parses_spec_example` → `agent_meta_reads_every_key`; `skill_meta_parses_spec_example` → `skill_meta_reads_every_key`; `frontmatter_error_messages_match_spec` → `frontmatter_error_displays_each_message`.

- 2026-11-20 — Final gate after fixes: `cargo make dod` → `test result: ok. 289 passed; 0 failed`, `[cargo-make] INFO - Build Done in 9.86 seconds.`
- 2026-11-21 — **Redesign after build:** the code was moved into `crates/agentstacks/src/types/<group>/` with the `types::port::FileTree` port and `fs::StdFileTree`, files renamed so no name repeats its folder word, and refined by crate-wide reviews. Paths and names above are superseded by the built tree; behaviour follows the redesigned `spec.md`.
