---
status: approved
created: 2026-11-20
depends-on: [01, 02, 04]
built-on: 3162a314af79
---

# Pack Manifest Implementation Plan

**Goal:** Pack manifest files (`pack.toml`, `native/<tool>/hooks.toml`) parse into validated types
that refuse every file-local rule with its location.

**Architecture:** Every TOML file in the crate goes through two shared files this plan owns
(plans 07, 08 and 09 reuse them):

- `toml_document.rs` — `parse_toml_document(file, text)` parses the text into a raw `toml::Table`
  or returns one syntax `Diagnostic` at `<file>:<line>`, where the line is the count of `\n`
  before the error span's start plus 1 and the message is the parser's message (spec §1.1, §10).
  Duplicate keys, duplicate tables and integers beyond i64 arrive here as syntax errors (§1.1).
  Empty, whitespace-only and comment-only text is an empty table (§1.1), so it is refused field by
  field. There is no `deny_unknown_fields` struct parse anywhere, because one stops at the first
  error (§1.1).
- `toml_fields.rs` — `TomlFields`, a reader over one raw table at a key-path prefix (empty at the
  top level, `pack`, `hooks[2]`, `hooks[2].grants`). It reads typed fields and records each
  failure as a `Diagnostic` (plan 01) at `<file>: <key.path>` with plan 01's `FieldError` generic
  shapes (§10): `required`, `expected <type>, got <type>`, `empty`, `unknown key`. The type words
  are the spec §10 list: `boolean`, `integer`, `float`, `string`, `datetime`, `array`, `table`
  (`null` is YAML only; TOML has no null). `TomlFields::type_word(value)` names a raw value's word,
  so a datetime or float in an integer field gets `expected integer, got datetime` (§10), and plan
  10 names the TOML type of an option value with the same function. A list item's key path carries
  its index (`requires.packs[1]`, like §10's `hooks[2].grants.read[0]`); a child table's carries its
  key (`hooks[2].grants`).

`PackManifest::parse(file, text)` reads `[pack]`, `[requires]`, `render`, `exclude`, `[options]`
and `[[hooks]]` and returns the manifest or every `Diagnostic` (plan 01's `Diagnostics`); every
field is checked even after another fails ("all errors per run", §10). Value types parse on
construction, one item per file: `AirslReq` (a `semver::VersionReq` whose operators are all among
the 8 of §1.1), `BinaryName`, `PackPath` (relative, no `..` segment), `ExcludeGlob` (relative,
compiled with `globset`), `EnvName`, `Policy`, `OptionType`, `OptionDecl`, `HookEvent`,
`MatchValue`, `HookRun`, `Grants`. Hooks are `Hook { event, matches, agents, action }` with
`HookAction::{Message(MessageHook), Lua(LuaHook)}`, so a field that does not apply to a kind
cannot be represented. `native/<tool>/hooks.toml` is parsed by `NativeHooks::parse` into
`NativeHook { event, matches, action }`, where `event` and `matches` are opaque strings (§2.2) and
`HookAction` (with its grants, policy, `may_block`, `exec` and `run` rules) is shared unchanged.

Value refusals on a list item are located at the list's key path, as the spec's examples are
(`requires.packs: "core" listed twice`, `hooks[1].grants.read: "data" must be …`). Tokens in hook
fields (`text`, `args`, `run`, grant paths, §3.4) are scanned here, once (§3.5), with plan 02's
`scan(text, &TextOrigin::Field { file, key_path })`; list items use indexed key paths
(`hooks[1].grants.read[0]`, §10). Every `LocatedToken` found is kept on `PackManifest` (or
`NativeHooks`) for plan 06's pack-local rules; every token error is a `Diagnostic` of the parse.

`ManifestError` is this module's `thiserror` enum. Its messages are copied from the spec; it
wraps plan 02's `InvalidName` and `InvalidOptionName` transparently. `FormatError` (plan 01) gains
`TomlSyntax` (the parser's message, displayed as-is) and `Manifest(ManifestError)`.

Owned elsewhere: `render` existence and not-excluded, `exclude` protecting `pack.toml`,
`SKILL.md`, agent files and own-pack `run` targets, `exclude` ∩ `render`, `requires.airsl` being
required when a non-excluded `.lua` exists (§4.2 "checked at pack load": `requires.airsl: required
(the pack contains .lua files)`), own-pack `run` existence, `agents` existence, the
`native/<tool>` directory name, and the pack-local token rules all need the pack's files (plan
06). This plan never refuses a missing `requires.airsl`. Other-pack `run`, option resolution
(§4.4) and `airsl` intersection need the whole set (plan 10).

**Tech Stack:** Rust 2024; `toml` 1 (`1.2.1+spec-1.1.0`, probed at `1.2.0`), `semver` 1
(`1.0.28`), `globset` 0.4 (`0.4.21`), all per spec §11; `thiserror` 2 (added by plan 01).

**Content authority:** spec §1.1 (toml, semver and globset facts), §2.2 (native hooks), §3.4 (token-bearing
manifest fields; `render` code rule), §3.5 (scanned once, tokens kept located), §4 and §4.1–§4.3
(`pack.toml`, names, field rules, `airsl` operators), §6.1–§6.3 (hook vocabulary, field rules,
fail-open is not a field), §10 (locations, generic shapes, raw-table parse, syntax line, all
errors), §11 (dependencies).

---

## Files that change

```
Cargo.toml                                       — modify  add toml, semver, globset to [workspace.dependencies]
crates/agentstacks/Cargo.toml                    — modify  depend on toml, semver, globset (workspace)
crates/agentstacks/src/model/mod.rs              — modify  declare and re-export the new items
crates/agentstacks/src/model/format_error.rs     — modify  FormatError gains TomlSyntax and Manifest variants
crates/agentstacks/src/model/toml_document.rs    — create  parse_toml_document: text → raw toml::Table or one syntax Diagnostic
crates/agentstacks/src/model/toml_fields.rs      — create  TomlFields: typed reads by key path, generic shapes, unknown keys
crates/agentstacks/src/model/manifest_error.rs   — create  ManifestError (thiserror), every manifest message
crates/agentstacks/src/model/airsl_req.rs        — create  AirslReq: VersionReq refusing operators outside the 8 of §1.1
crates/agentstacks/src/model/binary_name.rs      — create  BinaryName: a requires.binaries / grants.exec entry
crates/agentstacks/src/model/pack_path.rs        — create  PackPath: relative, "/"-separated, no ".." segment
crates/agentstacks/src/model/exclude_glob.rs     — create  ExcludeGlob: pattern text plus compiled glob
crates/agentstacks/src/model/env_name.rs         — create  EnvName: ^[A-Z_][A-Z0-9_]*$
crates/agentstacks/src/model/policy.rs           — create  Policy: confined | pure, default confined
crates/agentstacks/src/model/requires.rs         — create  Requires { packs, airsl, binaries }
crates/agentstacks/src/model/pack_manifest.rs    — create  PackManifest::parse
crates/agentstacks/src/model/option_type.rs      — create  OptionType: boolean | integer | string
crates/agentstacks/src/model/option_decl.rs      — create  OptionDecl { option_type, default, description }
crates/agentstacks/src/model/hook_run.rs         — create  HookRun: the three run forms, written text kept
crates/agentstacks/src/model/grants.rs           — create  Grants { policy, read, write, env, exec }
crates/agentstacks/src/model/hook_event.rs       — create  HookEvent: the six §6.1 events and their selectors
crates/agentstacks/src/model/match_value.rs      — create  MatchValue: startup resume clear compact read edit write
crates/agentstacks/src/model/message_hook.rs     — create  MessageHook { text }
crates/agentstacks/src/model/lua_hook.rs         — create  LuaHook { run, args, may_block, grants }
crates/agentstacks/src/model/hook_action.rs      — create  HookAction::{Message, Lua}, chosen by kind
crates/agentstacks/src/model/hook.rs             — create  Hook { event, matches, agents, action }
crates/agentstacks/src/model/native_hook.rs      — create  NativeHook { event, matches, action }
crates/agentstacks/src/model/native_hooks.rs     — create  NativeHooks::parse for native/<tool>/hooks.toml
```

## Order of work

### Task 1 — Parse TOML into a raw table and read fields by key path

**Files:**
- Modify `Cargo.toml` — add `toml = "1"` to `[workspace.dependencies]`.
- Modify `crates/agentstacks/Cargo.toml` — depend on `toml` (workspace).
- Modify `crates/agentstacks/src/model/format_error.rs` — `FormatError` gains `TomlSyntax`
  holding the parser's message, displayed as-is.
- Create `crates/agentstacks/src/model/toml_document.rs` — `parse_toml_document(file, text) ->
  Result<toml::Table, Diagnostic>`; a private `line_of(text, offset)` counts `\n` before `offset`
  and adds 1.
- Create `crates/agentstacks/src/model/toml_fields.rs` — `TomlFields`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** `parse_toml_document` as described in Architecture. `TomlFields::new(file, prefix,
table)` reads one table; each read records its own failure and returns `None`, so reading
continues:
- `key_path(key)` and `item_path(key, index)` build `prefix.key` and `prefix.key[index]` (just
  `key` at the top level).
- `required_string` / `optional_string`: absent → `Required` (required only); not a string →
  `WrongType`. `required_text` / `optional_text` add `Empty` for an empty or whitespace-only
  string.
- `required_integer` / `optional_integer`, `optional_bool`: the same shapes for their types.
- `optional_string_list(key)`: not an array → `WrongType` at the key; a non-string item →
  `WrongType` at `key[index]`; returns the string items with their indexes.
- `table_or_empty(key)`: a child `TomlFields` over the table at `key`, or over an empty table when
  `key` is absent (so a missing `[pack]` reports its fields one by one); not a table → `WrongType`.
- `optional_table(key)`: the child, or `None` when absent; not a table → `WrongType`.
- `table_array(key)`: one child per item at prefix `key[index]`; not an array → `WrongType` at the
  key; a non-table item → `WrongType` at `key[index]`.
- `entries()`: the table's keys and raw values in key order, for tables whose keys are names
  (`[options.*]`).
- `raw(key)`: the raw value, for fields with their own wrong-type message.
- `type_word(value)` (associated, public): the §10 type word of a raw `toml::Value` (`boolean`,
  `integer`, `float`, `string`, `datetime`, `array`, `table`); every `WrongType` built here uses it.
- `refuse(key, error)` and `refuse_at_file(error)`: push a `Diagnostic` carrying any error
  convertible into `FormatError`, at `prefix.key` or at the file alone.
- `refuse_unknown_keys(allowed)`: `UnknownKey` at `prefix.key` for every key not in `allowed`.
- `absorb(child)` moves a child's diagnostics into this reader; `into_diagnostics()` returns them.

**Proof:**
- `parse_document_reads_a_table` (toml_document.rs) — `[pack]` then `name = "core"` gives a table
  whose `pack.name` is the string `core`, and no diagnostic.
- `parse_document_reads_empty_text_as_an_empty_table` (toml_document.rs) — `""`, `"  \n\t"` and
  `"# only a comment\n"` each give an empty table and no diagnostic (§1.1 probe Q6).
- `parse_document_locates_a_syntax_error_by_line` (toml_document.rs) — `a = [1,` gives exactly the
  diagnostic `packs/core/pack.toml:1: unclosed array, expected ']'` (§1.1 message); the same text
  after two `\n` gives `packs/core/pack.toml:3: unclosed array, expected ']'`.
- `parse_document_treats_duplicates_and_overflow_as_syntax_errors` (toml_document.rs) —
  `a = 1\na = 2` and `[x]\n[x]` each give one diagnostic with a line whose message contains
  `duplicate key`; `a = 9223372036854775808` gives one whose message contains `u64 value was too
  large` (§1.1).
- `line_of_counts_newlines_before_the_offset` (toml_document.rs) — offset 0 is line 1; the offset
  right after the first `\n` is line 2; the end of `"a\nb\nc"` is line 3.
- `toml_fields_reads_typed_values` (toml_fields.rs) — a string, an integer, a boolean, a string
  list and a child table read back with their values and no diagnostic.
- `type_word_names_each_toml_type` (toml_fields.rs) — `true`, `1`, `1.5`, `"a"`, `1979-05-27`,
  `[1]` and `{a = 1}` give `boolean`, `integer`, `float`, `string`, `datetime`, `array`, `table`
  (§10 list).
- `toml_fields_reports_missing_and_wrong_types` (toml_fields.rs) — prefix `pack` over an empty
  table: `required_string("name")` gives `packs/core/pack.toml: pack.name: required`; `api` holding
  `"0"`, `0.5` and `1979-05-27` gives `packs/core/pack.toml: pack.api: expected integer, got
  string`, `… got float` and `… got datetime` (§10).
- `toml_fields_reports_empty_and_whitespace_text` (toml_fields.rs) — `""` and `"   "` read with
  `required_text("description")` give `packs/core/pack.toml: pack.description: empty`.
- `toml_fields_locates_list_items_by_index` (toml_fields.rs) — prefix `requires`, `packs = ["a",
  1]` gives `packs/core/pack.toml: requires.packs[1]: expected string, got integer`; `packs = "a"`
  gives `packs/core/pack.toml: requires.packs: expected array, got string`.
- `toml_fields_builds_nested_key_paths` (toml_fields.rs) — the third `[[hooks]]` item's child
  `grants` table reports an unknown key `net` as `packs/core/pack.toml: hooks[2].grants.net:
  unknown key`.
- `toml_fields_reports_every_unknown_key` (toml_fields.rs) — prefix `pack`, keys `foo` and `bar`
  outside the allowed set give both `pack.bar: unknown key` and `pack.foo: unknown key`, not only
  the first (§1.1 contrast with `deny_unknown_fields`).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green;
`cargo make deny` passes (`toml` added). Commit `feat(agentstacks): parse toml into raw tables
read by key path`.

### Task 2 — Add the manifest value types

**Files:**
- Modify `Cargo.toml` — add `semver = "1"` and `globset = "0.4"` to `[workspace.dependencies]`.
- Modify `crates/agentstacks/Cargo.toml` — depend on `semver` and `globset` (workspace).
- Create `crates/agentstacks/src/model/manifest_error.rs` — `ManifestError` with the variants
  below, plus transparent `Name(InvalidName)` and `OptionName(InvalidOptionName)` (plan 02).
- Modify `crates/agentstacks/src/model/format_error.rs` — `FormatError` gains
  `Manifest(#[from] ManifestError)`, transparent.
- Create `crates/agentstacks/src/model/airsl_req.rs`, `crates/agentstacks/src/model/binary_name.rs`,
  `crates/agentstacks/src/model/pack_path.rs`, `crates/agentstacks/src/model/exclude_glob.rs`,
  `crates/agentstacks/src/model/env_name.rs`, `crates/agentstacks/src/model/policy.rs`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** one newtype per file, each built only through a fallible constructor:
- `AirslReq::new(text)`: `semver::VersionReq::parse` fails → `NotVersionRange`
  (`"<text>" is not a version range`, §4.2); then every comparator's `Op` passes a private
  `is_supported(op)`: a match naming the 8 variants of §1.1 (Exact, Greater, GreaterEq, Less,
  LessEq, Tilde, Caret, Wildcard) as supported, with the fallback arm that `#[non_exhaustive]`
  requires (§1.1) refusing `UnsupportedOperator` (`"<text>" uses an operator this agentstacks does
  not support`, §4.3). `*` has no comparators and is accepted (§1.1). `as_version_req()` exposes
  the inner value for plan 10's `AirslRange`; `Display` gives the range as written, which plan 10
  quotes in its §4.3 messages (`core needs >=0.2`).
- `BinaryName::new(text)`: a §4.1 name ("names", §4.2), built through `ItemName` so the refusal is
  plan 02's `"<text>" is not a valid name`.
- `PackPath::new(text)`: a leading `/` → `MustBeRelative` (`"<text>" must be relative`); any `..`
  segment → `MustStayInsidePack` (`"<text>" must stay inside the pack`) (§4.2 `render` row).
- `ExcludeGlob::new(text)`: a leading `/` → `MustBeRelative`; `globset::Glob::new` fails →
  `InvalidGlob` (`"<text>" is not a valid glob`) (§4.2 `exclude` row). The leading-`/` check is
  agentstacks' own, because globset `0.4.21` refuses `[` but accepts `/tmp/**` (§1.1 globset row).
  Keeps the pattern text for plan 06's messages and the compiled matcher for plan 06's matching.
- `EnvName::new(text)`: not `^[A-Z_][A-Z0-9_]*$`, checked by hand → `NotEnvName`
  (`"<text>" is not an environment variable name`, §6.2).
- `Policy::parse(text)`: `confined` or `pure`; anything else → `PolicyNotAllowed`
  (`"<text>" is not allowed for pack hooks`, §6.2). `Policy::default()` is `Confined`.

**Proof:** each refusal is wrapped in a `Diagnostic` at `packs/core/pack.toml` and the key path its
manifest field uses, and its `Display` is asserted.
- `manifest_error_messages_match_the_spec` (manifest_error.rs) — table-driven over every variant
  present, each `Display` equal to the spec wording quoted in this plan; Tasks 3–6 add a row per
  variant they add.
- `airsl_req_accepts_cited_ranges` (airsl_req.rs) — `>=0.1.4`, `>= 0.1.4`, `0.1` and `*` are
  accepted (§1.1, §4 example); `*` holds no comparators; each displays as written.
- `airsl_req_refuses_a_non_range` (airsl_req.rs) — `nope` gives `packs/core/pack.toml:
  requires.airsl: "nope" is not a version range`; `""` gives `packs/core/pack.toml:
  requires.airsl: "" is not a version range` (§1.1: `""` is an error).
- `operator_check_accepts_the_eight_operators` (airsl_req.rs) — `is_supported` is true for each of
  the 8 `Op` variants named in §1.1.
- `unsupported_operator_message_names_the_range` (airsl_req.rs) — `UnsupportedOperator` for the
  §4.3 range `>=0.1` at `requires.airsl` displays `packs/core/pack.toml: requires.airsl: ">=0.1"
  uses an operator this agentstacks does not support`. The message is built directly: semver
  `1.0.28` has exactly the 8 operators (§1.1), so no input string reaches the fallback arm.
- `binary_name_accepts_the_real_binaries` (binary_name.rs) — `git`, `cmux`, `airsl`, `tar` (§1).
- `binary_name_refuses_an_invalid_name` (binary_name.rs) — `Git` at `requires.binaries` gives
  `packs/core/pack.toml: requires.binaries: "Git" is not a valid name`.
- `pack_path_accepts_relative_paths` (pack_path.rs) — `enforcement.json`, `hooks/x.lua`.
- `pack_path_refuses_absolute_and_escaping_paths` (pack_path.rs) — at `render`: `/x` gives
  `packs/core/pack.toml: render: "/x" must be relative`; `../x` gives `packs/core/pack.toml:
  render: "../x" must stay inside the pack`; `a/../../x` gives the same message with that path.
- `exclude_glob_compiles_the_spec_example` (exclude_glob.rs) — `**/*_test.lua` is accepted and
  keeps its text; `**/*.md` is accepted and matches `skills/discuss/SKILL.md` (§1.1 globset row).
- `exclude_glob_refuses_invalid_and_absolute_globs` (exclude_glob.rs) — at `exclude`: `[` gives
  `packs/core/pack.toml: exclude: "[" is not a valid glob`; `/tmp/**`, which globset `0.4.21`
  itself accepts (§1.1), gives `packs/core/pack.toml: exclude: "/tmp/**" must be relative`.
- `env_name_accepts_the_real_grants` (env_name.rs) — `HOME`, `TMPDIR`, `AIRSSTACK_HOME`,
  `AIRSSTACK_ENFORCE_REGISTRY`, `CLAUDE_PLUGIN_OPTION_STYLE_REINJECT` (§1) and `_X`.
- `env_name_refuses_other_shapes` (env_name.rs) — at `hooks[1].grants.env`: `home` gives
  `packs/core/pack.toml: hooks[1].grants.env: "home" is not an environment variable name`; `1X`
  and `""` give the same message with their value.
- `policy_parses_confined_and_pure` (policy.rs) — both parse; the default is `Confined`.
- `policy_refuses_other_values` (policy.rs) — at `hooks[1].grants.policy`: `trusted` gives
  `packs/core/pack.toml: hooks[1].grants.policy: "trusted" is not allowed for pack hooks`; `open`
  gives the same message with `open`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green;
`cargo make deny` passes (`semver` and `globset` added). Commit `feat(agentstacks): add pack
manifest value types`.

### Task 3 — Parse `[pack]`, `[requires]`, `render` and `exclude`

**Files:**
- Modify `crates/agentstacks/src/model/manifest_error.rs` — add `NotSemver` (`"<v>" is not
  semver`), `ApiUnsupported` (`<pack>: api <n> not supported (this agentstacks speaks 0)`),
  `ApiUnsupportedUnnamed` (`<n> not supported (this agentstacks speaks 0)`), `ListedTwice` (`"<v>" listed twice`), `RequiresItself` (`a pack cannot require itself`),
  `RenderIsCode` (`"<p>" is code; code is installed verbatim`).
- Create `crates/agentstacks/src/model/requires.rs` — `Requires { packs: Vec<PackName>, airsl:
  Option<AirslReq>, binaries: Vec<BinaryName> }` and `Requires::read(fields, own_name)`.
- Create `crates/agentstacks/src/model/pack_manifest.rs` — `PackManifest` (name, version,
  description, requires, render, exclude; later tasks add options, hooks, tokens) and
  `PackManifest::parse(file, text) -> Result<PackManifest, Diagnostics>`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:**
- The text goes through `parse_toml_document`; a syntax error is the file's only diagnostic.
- Top-level keys are `pack`, `requires`, `render`, `exclude`, `options`, `hooks`; any other is
  `<key>: unknown key` (§4.2).
- `[pack]` is read with `table_or_empty`, keys `name`, `version`, `api`, `description`:
  `name` required, a `PackName` (plan 02 refusal at `pack.name`); `version` required, a
  `semver::Version` (`Version::parse("1.2")` fails, §1.1) else `NotSemver` at `pack.version`;
  `api` a required integer; any value other than 0 is refused per the §4.2 `pack.api` row: with a
  valid `pack.name`, `ApiUnsupported` at the file alone, `<pack>` being that name (`sdlc: api 1 …`);
  when `pack.name` is missing, not a string or not a valid name, `ApiUnsupportedUnnamed` at
  `pack.api` (`pack.api: 1 not supported …`), next to the `pack.name` refusal; `description`
  required text.
- `[requires]` (optional; keys `packs`, `airsl`, `binaries`): `packs` items are `PackName`s
  (refusal at `requires.packs`), a repeat → `ListedTwice` at `requires.packs`, the pack's own
  valid name → `RequiresItself` at `requires.packs`; `airsl` a string through `AirslReq`, refused
  at `requires.airsl`; `binaries` items are `BinaryName`s, a repeat → `ListedTwice` at
  `requires.binaries`. Absent → no packs, no `airsl`, no binaries.
- `render` items are `PackPath`s, refused at `render`; one ending in `.lua` or `.sh` →
  `RenderIsCode` at `render` (§3.4).
- `exclude` items are `ExcludeGlob`s, refused at `exclude`.

**Proof:**
- `requires_parses_the_spec_example` (requires.rs) — `packs = ["core"]`, `airsl = ">=0.1.4"`,
  `binaries = ["git"]` read back; an absent `[requires]` gives no packs, no `airsl`, no binaries.
- `requires_refuses_each_rule` (requires.rs) — table-driven, file `packs/sdlc/pack.toml`, own name
  `sdlc`: `packs = ["core", "core"]` → `packs/sdlc/pack.toml: requires.packs: "core" listed twice`;
  `packs = ["sdlc"]` → `packs/sdlc/pack.toml: requires.packs: a pack cannot require itself`;
  `packs = ["Core"]` → `packs/sdlc/pack.toml: requires.packs: "Core" is not a valid name`;
  `airsl = "nope"` → `packs/sdlc/pack.toml: requires.airsl: "nope" is not a version range`;
  `binaries = ["git", "git"]` → `packs/sdlc/pack.toml: requires.binaries: "git" listed twice`;
  `foo = 1` → `packs/sdlc/pack.toml: requires.foo: unknown key`.
- `pack_manifest_parses_the_metadata_example` (pack_manifest.rs) — the spec §4 example without
  `[options]` and `[[hooks]]` gives name `sdlc`, version `0.4.2`, the description, requires as
  above, `render` `enforcement.json`, `exclude` `**/*_test.lua`, and no diagnostic.
- `pack_manifest_refuses_each_pack_field` (pack_manifest.rs) — table-driven, each file otherwise
  valid: `name = "Core"` → `packs/core/pack.toml: pack.name: "Core" is not a valid name`;
  `version = "1.2"` → `packs/core/pack.toml: pack.version: "1.2" is not semver`; `api = 1` in
  `packs/sdlc/pack.toml` with `name = "sdlc"` → `packs/sdlc/pack.toml: sdlc: api 1 not supported
  (this agentstacks speaks 0)`; `description = ""` and `description = "  "` →
  `packs/core/pack.toml: pack.description: empty`; `api = 1979-05-27` → `packs/core/pack.toml:
  pack.api: expected integer, got datetime`; `api = 0.5` → `… got float` (§10).
- `pack_manifest_refuses_api_without_a_valid_name` (pack_manifest.rs) — table-driven, `api = 1`
  in `packs/sdlc/pack.toml`: no `name` gives exactly `packs/sdlc/pack.toml: pack.name: required`
  and `packs/sdlc/pack.toml: pack.api: 1 not supported (this agentstacks speaks 0)`; `name =
  "Core"` gives exactly `packs/sdlc/pack.toml: pack.name: "Core" is not a valid name` and the same
  `pack.api` diagnostic; `name = 1` gives `packs/sdlc/pack.toml: pack.name: expected string, got
  integer` and the same `pack.api` diagnostic (§4.2 `pack.api` row).
- `pack_manifest_refuses_an_empty_file_field_by_field` (pack_manifest.rs) — `""` gives exactly
  `packs/core/pack.toml: pack.name: required`, `… pack.version: required`, `… pack.api: required`
  and `… pack.description: required` (§10).
- `pack_manifest_refuses_render_entries` (pack_manifest.rs) — `render = ["/x", "../x", "x.lua",
  "y.sh"]` gives `packs/core/pack.toml: render: "/x" must be relative`, `packs/core/pack.toml:
  render: "../x" must stay inside the pack`, `packs/core/pack.toml: render: "x.lua" is code; code
  is installed verbatim` and the same for `y.sh`.
- `pack_manifest_refuses_exclude_entries` (pack_manifest.rs) — `exclude = ["[", "/tmp/**"]`
  gives `packs/core/pack.toml: exclude: "[" is not a valid glob` and `packs/core/pack.toml:
  exclude: "/tmp/**" must be relative`.
- `pack_manifest_refuses_unknown_keys_at_each_level` (pack_manifest.rs) — top-level `foo = 1` and
  `pack.foo = 1` give `packs/core/pack.toml: foo: unknown key` and `packs/core/pack.toml:
  pack.foo: unknown key`.
- `pack_manifest_reports_all_errors_at_once` (pack_manifest.rs) — `name = "Core"`, `version =
  "1.2"` and `render = ["/x"]` in one file give exactly those three diagnostics.
- `pack_manifest_stops_at_a_syntax_error` (pack_manifest.rs) — `[pack]`, `name = "Core"`,
  `a = [1,` on three lines gives only `packs/core/pack.toml:3: unclosed array, expected ']'`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse pack.toml metadata`.

### Task 4 — Parse option declarations

**Files:**
- Modify `crates/agentstacks/src/model/manifest_error.rs` — add `OptionTypeUnknown`
  (`"<t>" (expected boolean, integer, string)`) and `DefaultType` (`expects <type>, got <type>`).
- Create `crates/agentstacks/src/model/option_type.rs` — `OptionType::{Boolean, Integer, String}`
  with `parse(text)` and its type word.
- Create `crates/agentstacks/src/model/option_decl.rs` — `OptionDecl { option_type, default,
  description }` and `OptionDecl::read(fields)`.
- Modify `crates/agentstacks/src/model/pack_manifest.rs` — `options: Vec<(OptionName,
  OptionDecl)>` read from `[options]`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** `[options]` (optional, a table) holds one table per option. Each key must be an
`OptionName` (plan 02); a bad key is refused at `options.<key>` with plan 02's
`InvalidOptionName` (`options.archive-keep: "archive-keep" is not a valid option name`, §4.2
`options.<n>` row); the entry's fields are still read.
Per entry (keys `type`, `default`, `description`): `type` required text (missing →
`options.<n>.type: required`, §4.2 "required"), an `OptionType`, else
`OptionTypeUnknown` at `options.<n>.type`; `default` optional, its TOML type must be the declared
type (`integer` is i64, §4.2), else `DefaultType` at `options.<n>.default` with the declared word
and the TOML type word from `TomlFields::type_word`; `description` required text. The default is kept as its TOML value, already
checked against `type`; plan 10 turns it into `OptionValue`.

**Proof:**
- `option_type_parses_the_three_types` (option_type.rs) — `boolean`, `integer`, `string`.
- `option_type_refuses_others` (option_type.rs) — `number` at `options.keep.type` gives
  `packs/core/pack.toml: options.keep.type: "number" (expected boolean, integer, string)`.
- `option_decl_parses_each_type` (option_decl.rs) — the spec §4 `archive_keep` (integer, default
  20) parses; a boolean with default `true` and a string with no default parse.
- `option_decl_refuses_each_rule` (option_decl.rs) — prefix `options.archive_keep`: integer type
  with `default = "20"` → `packs/core/pack.toml: options.archive_keep.default: expects integer, got
  string`; with `default = 1.5` → `… expects integer, got float`; `description = ""` →
  `packs/core/pack.toml: options.archive_keep.description: empty`; no `description` → `…
  options.archive_keep.description: required`; no `type` → `… options.archive_keep.type:
  required`; `foo = 1` → `… options.archive_keep.foo: unknown key`.
- `pack_manifest_reads_the_spec_options` (pack_manifest.rs) — the full spec §4 example up to
  `[[hooks]]` gives one option `archive_keep`.
- `pack_manifest_refuses_bad_option_names_and_shapes` (pack_manifest.rs) — `[options.archive-keep]`
  gives `packs/core/pack.toml: options.archive-keep: "archive-keep" is not a valid option name`;
  `[options.x]` with only `description = "d"` gives `packs/core/pack.toml: options.x.type:
  required`; `options = 1`
  gives `packs/core/pack.toml: options: expected table, got integer`; `options.keep = 1` gives
  `packs/core/pack.toml: options.keep: expected table, got integer`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse pack option declarations`.

### Task 5 — Parse hook `run` and `grants`

**Files:**
- Modify `crates/agentstacks/src/model/manifest_error.rs` — add `NotLuaFile` (`"<run>" is not a
  .lua file`), `RunForm` (`must be pack-relative, {{root}}/… or {{root <pack>}}/…`), `GrantPath`
  (`"<path>" must be absolute or start with a token`), `NotInBinaries` (`"<name>" is not in
  requires.binaries`).
- Create `crates/agentstacks/src/model/hook_run.rs` — `HookRun { written, pack:
  Option<PackName>, path: PackPath }` and `HookRun::read(fields, tokens)`.
- Create `crates/agentstacks/src/model/grants.rs` — `Grants { policy, read, write, env, exec }`
  and `Grants::read(fields, binaries, tokens)`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:**
- `HookRun::read` reads `run` as required text and scans it (origin `Field`, key path
  `hooks[i].run`). A token error is reported and the form check is skipped, so no follow-on error
  appears. Not ending in `.lua` → `NotLuaFile`. Form (§6.2): a text with no `{{` that is a
  `PackPath` is pack-relative (`pack` is `None`); a text whose first token starts at byte 0, is a
  `root` token, and is followed by `/` plus a `PackPath` with no `{{` is `{{root}}/…` (`pack`
  `None`) or `{{root <pack>}}/…` (`pack` `Some`). Anything else (absolute, `..`, `{{skill_dir …}}`,
  `{{var …}}`, a token later in the text) → `RunForm`. All refusals at `hooks[i].run`. `written`
  keeps the text for plan 06's and plan 10's existence messages; plan 06 treats `pack` equal to its
  own name as own-pack.
- `Grants::read` (keys `policy`, `read`, `write`, `env`, `exec`; absent table → `Policy::default()`
  and empty lists): `policy` through `Policy::parse`; each `read` and `write` item is scanned
  (key path `….read[j]`) and must start with `/` or have its first token at byte 0 with kind
  `root`, `skill_dir` or `var`, else `GrantPath` at `….read` / `….write`; each `env` item is an
  `EnvName`, refused at `….env`; each `exec` item must be `airsl` or a name in `binaries`, else
  `NotInBinaries` at `….exec`, kept as a `BinaryName`.

**Proof:**
- `hook_run_accepts_the_three_forms` (hook_run.rs) — `hooks/x.lua` and `{{root}}/hooks/x.lua` and
  `{{ root }}/hooks/x.lua` give `pack` `None`, path `hooks/x.lua`; `{{root core}}/hooks/enforce.lua`
  gives `pack` `core`, path `hooks/enforce.lua`, a `root` token located at `hooks[1].run`, and
  `written` equal to the input.
- `hook_run_refuses_a_missing_run` (hook_run.rs) — no `run` gives `packs/core/pack.toml:
  hooks[1].run: required`.
- `hook_run_refuses_a_non_lua_file` (hook_run.rs) — `x.sh` gives `packs/core/pack.toml:
  hooks[1].run: "x.sh" is not a .lua file`.
- `hook_run_refuses_other_forms` (hook_run.rs) — `/abs/x.lua`, `../x.lua`,
  `{{skill_dir discuss}}/x.lua`, `{{var tmpdir}}/x.lua`, `hooks/{{root}}/x.lua` and `{{root}}x.lua`
  each give `packs/core/pack.toml: hooks[1].run: must be pack-relative, {{root}}/… or {{root
  <pack>}}/…`.
- `hook_run_reports_a_token_error_without_a_form_error` (hook_run.rs) — `{{root Core}}/x.lua` gives
  only `packs/core/pack.toml: hooks[1].run: "Core" is not a valid name`.
- `grants_parse_the_spec_example` (grants.rs) — §6: policy `confined`, read `/`, write
  `{{var tmpdir}}` with its `var` token located at `hooks[1].grants.write[0]`, env `HOME` and
  `TMPDIR`, exec empty.
- `grants_default_to_confined_and_nothing` (grants.rs) — an absent `grants` table gives policy
  `Confined` and empty lists.
- `grants_refuse_each_rule` (grants.rs) — table-driven, prefix `hooks[1].grants`, binaries empty:
  `policy = "trusted"` → `packs/core/pack.toml: hooks[1].grants.policy: "trusted" is not allowed
  for pack hooks`; `read = ["data"]` → `packs/core/pack.toml: hooks[1].grants.read: "data" must be
  absolute or start with a token`; `write = ["data"]` → the same at `hooks[1].grants.write`;
  `read = ["{{option keep}}/x"]` → the `GrantPath` message with that value; `env = ["home"]` →
  `packs/core/pack.toml: hooks[1].grants.env: "home" is not an environment variable name`;
  `exec = ["git"]` → `packs/core/pack.toml: hooks[1].grants.exec: "git" is not in
  requires.binaries`; `net = []` → `packs/core/pack.toml: hooks[1].grants.net: unknown key`.
- `grants_accept_airsl_without_declaring_it` (grants.rs) — `exec = ["airsl"]` with empty binaries
  gives no diagnostic.
- `grants_accept_declared_binaries` (grants.rs) — binaries `git`, `exec = ["git"]` gives no
  diagnostic.
- `grants_report_token_errors_by_indexed_key` (grants.rs) — `read = ["/", "{{skil x}}/a"]` gives
  `packs/core/pack.toml: hooks[1].grants.read[1]: unknown token kind "skil"`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse hook run and grants`.

### Task 6 — Parse neutral `[[hooks]]`

**Files:**
- Modify `crates/agentstacks/src/model/manifest_error.rs` — add `KindUnknown` (`"<k>" (expected
  message, lua)`), `EventUnknown` (`"<e>" unknown`), `MatchNotAllowed` (`not allowed on <event>`),
  `MatchNotValid` (`"<v>" not valid for <event>`), `MatchEmpty` (`empty list never fires`),
  `AgentsNotAllowed` (`only allowed on agent.stop`), `NotOnMessageHook` (`not allowed on a message
  hook`), `NotOnLuaHook` (`not allowed on a lua hook`), `MessageCannotBlock` (`message hooks cannot block`), `ExpectedBool` (`expected true or
  false`).
- Create `crates/agentstacks/src/model/hook_event.rs` — `HookEvent::{SessionStart, SessionEnd,
  PromptSubmit, ToolBefore, ToolAfter, AgentStop}` with `parse`, `Display` as the §6.1 name,
  `match_values()` (the event's set, or none when `match` is not allowed) and `allows_agents()`.
- Create `crates/agentstacks/src/model/match_value.rs` — `MatchValue::{Startup, Resume, Clear,
  Compact, Read, Edit, Write}` with `parse` and `Display`.
- Create `crates/agentstacks/src/model/message_hook.rs` — `MessageHook { text }` and its `read`.
- Create `crates/agentstacks/src/model/lua_hook.rs` — `LuaHook { run: HookRun, args, may_block,
  grants }` and its `read`.
- Create `crates/agentstacks/src/model/hook_action.rs` — `HookAction::{Message, Lua}` and
  `HookAction::read(fields, binaries, tokens)`.
- Create `crates/agentstacks/src/model/hook.rs` — `Hook { event, matches: Option<Vec<MatchValue>>,
  agents: Option<Vec<ItemName>>, action }` and `Hook::read`.
- Modify `crates/agentstacks/src/model/pack_manifest.rs` — `hooks` read with `table_array`, each
  item through `Hook::read` with the manifest's binaries; `tokens: Vec<LocatedToken>` collects
  every hook token.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** per §6.1–§6.3, all at `hooks[i].<field>`:
- `HookAction::read`: `kind` required string; `message` → `MessageHook::read`, `lua` →
  `LuaHook::read`, else `KindUnknown`. With `kind` missing or unknown, the kind-specific keys
  (`text`, `run`, `args`, `grants`, `may_block`) are neither checked nor reported as unknown.
- `MessageHook::read`: `text` required text (`required` / `empty`), scanned at `hooks[i].text`;
  `run`, `args`, `grants` present → `NotOnMessageHook` at that key; `may_block` present (either
  value) → `MessageCannotBlock`.
- `LuaHook::read`: `text` present → `NotOnLuaHook` at `hooks[i].text` (§6.2), not `unknown key`;
  `run` through `HookRun::read`; `args` an optional string list, each item scanned
  at `hooks[i].args[j]`; `may_block` optional, a boolean else `ExpectedBool` (default false);
  `grants` through `Grants::read`. Fail-open is not a field (§6.3), so `fail_open` is an unknown
  key.
- `Hook::read`: `event` required string, a `HookEvent` else `EventUnknown`. With a valid event:
  `match` present where `match_values()` is none → `MatchNotAllowed`; an empty list →
  `MatchEmpty`; an item outside the set → `MatchNotValid`; a repeat → `ListedTwice`; absent → all.
  `agents` present when `!allows_agents()` → `AgentsNotAllowed`; items are `ItemName`s, refused at
  `hooks[i].agents`; absent → any agent; existence is plan 06's. With `event` missing or unknown,
  `match` and `agents` are not checked. Allowed keys: `kind`, `event`, `match`, `agents` plus the
  action's keys (all five when the kind is unknown).

**Proof:**
- `hook_event_parses_the_six_events` (hook_event.rs) — each §6.1 name parses and displays back.
- `hook_event_refuses_unknown_events` (hook_event.rs) — `tool.pre` at `hooks[0].event` gives
  `packs/core/pack.toml: hooks[0].event: "tool.pre" unknown`.
- `hook_event_lists_selectors_per_event` (hook_event.rs) — `session.start` → startup, resume,
  clear, compact; `tool.before` and `tool.after` → read, edit, write; the other three → none;
  `allows_agents()` only for `agent.stop`.
- `match_value_parses_the_seven_values` (match_value.rs) — each parses and displays back; `bash`
  does not parse.
- `message_hook_parses_the_spec_example` (message_hook.rs) — §6 `session.end` hook: the text is
  kept and its `skill` token is located at `hooks[0].text`.
- `message_hook_refuses_each_rule` (message_hook.rs) — prefix `hooks[0]`: no `text` →
  `packs/core/pack.toml: hooks[0].text: required`; `text = ""` → `… hooks[0].text: empty`;
  `run`, `args`, `grants` each → `packs/core/pack.toml: hooks[0].run: not allowed on a message
  hook` (and the same at `args`, `grants`); `may_block = true` and `may_block = false` each →
  `packs/core/pack.toml: hooks[0].may_block: message hooks cannot block`.
- `lua_hook_parses_the_spec_example` (lua_hook.rs) — §6 lua hook: run `{{root core}}/hooks/enforce.lua`,
  args `--manifest` and `{{root}}/enforcement.json` with the `root` token located at
  `hooks[1].args[1]`, `may_block` false, the example grants.
- `lua_hook_defaults_may_block_to_false` (lua_hook.rs) — no `may_block` gives false.
- `lua_hook_refuses_each_rule` (lua_hook.rs) — prefix `hooks[1]`: `may_block = "yes"` →
  `packs/core/pack.toml: hooks[1].may_block: expected true or false`; `args = [1]` →
  `… hooks[1].args[0]: expected string, got integer`; `text = "x"` → `packs/core/pack.toml:
  hooks[1].text: not allowed on a lua hook`; `fail_open = false` → `… hooks[1].fail_open: unknown
  key`.
- `hook_action_dispatches_on_kind` (hook_action.rs) — `message` and `lua` give the matching
  variant.
- `hook_action_refuses_a_missing_or_unknown_kind` (hook_action.rs) — `kind = "sh"` with `run` and
  `text` gives only `packs/core/pack.toml: hooks[0].kind: "sh" (expected message, lua)`; no `kind`
  gives only `packs/core/pack.toml: hooks[0].kind: required`.
- `hook_accepts_every_allowed_selector` (hook.rs) — `session.start` with all four values,
  `tool.after` with `read`, `edit`, `write`, `agent.stop` with `agents = ["coder"]`, and any event
  with `match` omitted (all) parse.
- `hook_refuses_each_selector_rule` (hook.rs) — table-driven: `match` on `session.end` →
  `packs/core/pack.toml: hooks[1].match: not allowed on session.end` (same shape on
  `prompt.submit` and `agent.stop`); `match = ["bash"]` on `tool.before` → `… hooks[1].match:
  "bash" not valid for tool.before`; `match = []` → `… hooks[1].match: empty list never fires`;
  `match = ["read", "read"]` → `… hooks[1].match: "read" listed twice`; `agents` on `tool.before`
  → `packs/core/pack.toml: hooks[0].agents: only allowed on agent.stop`; `agents = ["Coder"]` on
  `agent.stop` → `… hooks[0].agents: "Coder" is not a valid name`; no `event` → `…
  hooks[0].event: required`.
- `hook_skips_selectors_when_the_event_is_unknown` (hook.rs) — `event = "tool.pre"` with
  `match = ["bash"]` gives only the `EventUnknown` diagnostic.
- `pack_manifest_parses_the_spec_hooks` (pack_manifest.rs) — the §4 example with the two §6 hooks
  appended parses; `tokens` holds every hook token with its key path.
- `pack_manifest_refuses_an_unknown_hook_key` (pack_manifest.rs) — three hooks, the third a lua
  hook with `grant` in place of `grants`, gives `packs/core/pack.toml: hooks[2].grant: unknown key`
  (§4.2 verbatim).
- `pack_manifest_checks_exec_against_its_own_binaries` (pack_manifest.rs) — `binaries = ["git"]`
  with a hook `exec = ["git"]` parses; with no `binaries` it gives `packs/core/pack.toml:
  hooks[0].grants.exec: "git" is not in requires.binaries`.
- `pack_manifest_refuses_a_non_array_hooks_key` (pack_manifest.rs) — `hooks = 1` gives
  `packs/core/pack.toml: hooks: expected array, got integer`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse neutral pack hooks`.

### Task 7 — Parse `native/<tool>/hooks.toml`

**Files:**
- Create `crates/agentstacks/src/model/native_hook.rs` — `NativeHook { event: String, matches:
  Option<Vec<String>>, action: HookAction }` and `NativeHook::read(fields, binaries, tokens)`.
- Create `crates/agentstacks/src/model/native_hooks.rs` — `NativeHooks { hooks, tokens }` and
  `NativeHooks::parse(file, text, requires) -> Result<NativeHooks, Diagnostics>`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** the text goes through `parse_toml_document`; a syntax error is the file's only
diagnostic. The only top-level key is `hooks` (optional; absent or empty text means no hooks).
Each `[[hooks]]` item: `event` required text, kept opaque; `match` an optional list of strings,
kept opaque; the action through `HookAction::read` with `requires.binaries`, so `kind`, `text`,
`run`, `args`, `may_block`, `grants`, policy, `exec ⊆ requires.binaries ∪ {airsl}` and forced
fail-open apply unchanged (§2.2). Allowed keys: `kind`, `event`, `match` plus the action's keys.
Tokens are scanned and kept located as in pack.toml.

**Proof:** file `packs/core/native/claude/hooks.toml` (written `N` below).
- `native_hook_accepts_tool_specific_event_and_match` (native_hook.rs) — a lua hook with
  `event = "PreToolUse"`, `match = ["SubagentHandback"]`, `may_block = true` and
  `run = "hooks/handoff-check.lua"` parses with those strings kept.
- `native_hook_keeps_the_neutral_rules` (native_hook.rs) — table-driven: `policy = "trusted"` →
  `N: hooks[0].grants.policy: "trusted" is not allowed for pack hooks`; `exec = ["git"]` with no
  binaries → `N: hooks[0].grants.exec: "git" is not in requires.binaries`; `run = "x.sh"` →
  `N: hooks[0].run: "x.sh" is not a .lua file`; a message hook with `may_block = true` →
  `N: hooks[0].may_block: message hooks cannot block`; a lua hook with `text = "x"` →
  `N: hooks[0].text: not allowed on a lua hook`; `kind = "sh"` → `N: hooks[0].kind: "sh"
  (expected message, lua)`.
- `native_hook_refuses_event_and_match_shapes` (native_hook.rs) — no `event` → `N:
  hooks[0].event: required`; `event = ""` → `N: hooks[0].event: empty`; `match = [1]` → `N:
  hooks[0].match[0]: expected string, got integer`; `agents = ["coder"]` → `N: hooks[0].agents:
  unknown key` (§2.2: `agents` is not accepted, generic `unknown key`).
- `native_hooks_parse_a_file_of_hooks` (native_hooks.rs) — two hooks parse; a `{{root}}` token in
  one hook's `args` is kept at `hooks[1].args[0]`.
- `native_hooks_read_an_empty_file_as_no_hooks` (native_hooks.rs) — `""` and a comment-only text
  give zero hooks and no diagnostic.
- `native_hooks_refuse_unknown_top_level_keys` (native_hooks.rs) — `tools = 1` gives
  `N: tools: unknown key`.
- `native_hooks_report_a_syntax_error_by_line` (native_hooks.rs) — `a = [1,` gives only
  `N:1: unclosed array, expected ']'`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse native hook declarations`.

---

## Risks

- `toml_fields.rs` and `toml_document.rs` are shared with plans 07, 08 and 09. Task 1 fixes their
  method set (named in its Change); a later rename ripples into three plans.
- The `UnsupportedOperator` arm cannot be reached through input with semver `1.0.28`, whose `Op`
  has exactly the 8 supported variants (§1.1). Its message is tested directly; the arm exists so a
  later semver release cannot widen what is accepted silently.
- Option not taken: a `deny_unknown_fields` struct parse, which stops at the first error (§1.1).

## Proof

- `cargo make dod` green and `cargo make deny` green after Task 7.
- Coverage map (every file-local refusal in scope → test):
  - §1.1/§10 syntax line, empty text → `parse_document_locates_a_syntax_error_by_line`,
    `parse_document_reads_empty_text_as_an_empty_table`,
    `pack_manifest_refuses_an_empty_file_field_by_field`, `native_hooks_report_a_syntax_error_by_line`.
  - §10 generic shapes and type words → `toml_fields_reports_missing_and_wrong_types`,
    `toml_fields_reports_empty_and_whitespace_text`, `toml_fields_reports_every_unknown_key`,
    `type_word_names_each_toml_type`.
  - §4.2 unknown key, `pack.*` → `pack_manifest_refuses_an_unknown_hook_key`,
    `pack_manifest_refuses_each_pack_field`, `pack_manifest_refuses_api_without_a_valid_name`.
  - §4.2 `requires.*` (except the `.lua` row, plan 06), §4.3 operator →
    `requires_refuses_each_rule`, `airsl_req_refuses_a_non_range`,
    `unsupported_operator_message_names_the_range`.
  - §4.2 `render` (relative, `..`), §3.4 code → `pack_manifest_refuses_render_entries`.
  - §4.2 `exclude` (glob, relative; §1.1 globset row) → `exclude_glob_refuses_invalid_and_absolute_globs`,
    `pack_manifest_refuses_exclude_entries`.
  - §4.2 `options.*` → `option_type_refuses_others`, `option_decl_refuses_each_rule`,
    `pack_manifest_refuses_bad_option_names_and_shapes`.
  - §6.2 kind, event, match, agents → `hook_action_refuses_a_missing_or_unknown_kind`,
    `hook_event_refuses_unknown_events`, `hook_refuses_each_selector_rule`.
  - §6.2 message hook → `message_hook_refuses_each_rule`; lua `text` → `lua_hook_refuses_each_rule`.
  - §6.2 lua `run` (required, `.lua`, form) → `hook_run_refuses_a_missing_run`,
    `hook_run_refuses_a_non_lua_file`, `hook_run_refuses_other_forms`.
  - §6.2 grants, `may_block` → `grants_refuse_each_rule`, `lua_hook_refuses_each_rule`.
  - §2.2 native hooks → `native_hook_keeps_the_neutral_rules`,
    `native_hook_refuses_event_and_match_shapes`.
  - §3.4/§3.5 token errors located by key → `grants_report_token_errors_by_indexed_key`,
    `hook_run_reports_a_token_error_without_a_form_error`.
- 80 colocated unit tests across 23 logic-bearing files; `mod.rs` is export-only and has none.
