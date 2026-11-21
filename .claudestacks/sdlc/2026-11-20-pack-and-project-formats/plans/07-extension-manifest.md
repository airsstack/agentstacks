---
status: done
created: 2026-11-20
depends-on: [01, 02, 03, 05]
built-on: 95010069150c
---

# Extension Manifest Implementation Plan

**Goal:** `Extension::load(dir)` turns an extension directory into a checked `Extension` or every
refusal it contains.

**Architecture:** Four new files under `crates/agentstacks/src/model/`, one item each, re-exported
by the export-only `model/mod.rs`.

- `extension_error.rs` holds `ExtensionError`, the module's one `thiserror` enum (spec §10). It
  joins `FormatError` (plan 01, `model/format_error.rs`) through a new `Extension` variant, and
  wraps plan 02's `InvalidName` transparently.
- `entry_path.rs` holds `EntryPath`, a newtype over the `entry` string. Its constructor refuses a
  path that leaves the extension or does not end in `.lua`.
- `extension_manifest.rs` holds `ExtensionManifest::parse(file, text)`. It parses the text into a
  raw `toml::Table` with `parse_toml_document` (plan 05). If that fails, the result is that one
  syntax diagnostic, whose line comes from the error span, and nothing else is checked. Otherwise
  it reads `[extension]` through a `TomlFields` reader (plan 05), which records plan 01's
  `FieldError` generic shapes located by key path and reports unknown keys. It never uses a
  `deny_unknown_fields` parse. `api` other than `0` is refused here and nowhere else (spec §4.5): a
  refused manifest produces no `Extension`, so the set check (plan 10) never sees it.
- `extension.rs` holds `Extension` and `Extension::load(dir)`. It walks the directory once with
  `walk(dir)` (plan 03). The walk skips dot entries first, ignores empty directories, and refuses
  symlinks, special files and non-UTF-8 paths. Plan 03 words a symlink refusal for packs
  (`WalkError::Symlink`); this plan's `reword_walk_diagnostic` (a private function in
  `extension.rs`, tested there) rewrites exactly that variant to `symlinks are not allowed in an
  extension` and passes every other diagnostic through unchanged. A walk refusal stops the load,
  as in plan 06. Otherwise the load checks that `extension.toml` is present and parses it, then
  checks that `entry` is among the walked regular files. Last, it records the directory's tree
  hash from `tree_hash(dir)` (plan 03, spec §8.1).

Every refusal is a `Diagnostic { location, error }` collected in plan 01's `Diagnostics`, so one
run reports all of them. What the adapter exports and how it runs belong to
`extension-host-and-hooks` (spec §4.5, §13). The tree hash is computed and stored here; comparing
it with the lock belongs to `core-plan-and-apply` (spec §8, §8.2, §13).

**Tech Stack:** Rust 2024; `toml` 1 and `semver` 1 (added by plan 05); `sha2` 0.11 through
`tree_hash`, and `tempfile` 3 as a dev-dependency (both added by plan 03). No new dependency.

**Content authority:** spec §4.5 (`extension.toml`), §2.1 (symlink, special-file, empty-directory,
dot-rule-first, UTF-8 and dot-entry rows, applied "as §2.1"), §4.1 (name rule), §4.2 (message
shapes, prefixed `extension.`), §8.1 (tree hash), §10 (locations, generic shapes, type words,
syntax errors, I/O failures, all errors per run), §12 (fixtures, coverage rule).

---

## Files that change

```
crates/agentstacks/src/model/mod.rs                 — modify  declare and re-export ExtensionError, EntryPath, ExtensionManifest, Extension
crates/agentstacks/src/model/format_error.rs        — modify  FormatError gains an Extension variant (#[from] ExtensionError)
crates/agentstacks/src/model/extension_error.rs     — create  ExtensionError (thiserror) + colocated Display tests
crates/agentstacks/src/model/entry_path.rs          — create  EntryPath: relative, inside the extension, ends in .lua
crates/agentstacks/src/model/extension_manifest.rs  — create  ExtensionManifest::parse(file, text)
crates/agentstacks/src/model/extension.rs           — create  Extension, Extension::load(dir)
crates/agentstacks/tests/extension_load.rs          — create  integration test over the extension fixtures
crates/agentstacks/tests/fixtures/extensions/**     — create  one valid extension; one invalid extension per refusal
```

## Order of work

### Task 1 — Add `ExtensionError` and `EntryPath`

**Files:**
- Create `crates/agentstacks/src/model/extension_error.rs`: `ExtensionError`. Each variant's
  message is copied from spec §4.5, or follows a §4.2 shape where §4.5 points there. The location
  comes from the `Diagnostic` that wraps it.
  - `NotAnExtension` displays `not an extension (no extension.toml)`.
  - `Name(#[from] InvalidName)` is transparent and displays plan 02's `"<value>" is not a valid
    name`.
  - `NotSemver { value }` displays `"<value>" is not semver`.
  - `ApiUnsupported { api }` displays `<api> not supported (this agentstacks speaks 0)`.
  - `EntryOutside { value }` displays `"<value>" must stay inside the extension`.
  - `EntryNotLua { value }` displays `"<value>" is not a .lua file`.
  - `EntryMissing { value }` displays `"<value>" does not exist`.

  The enum has no symlink variant and no I/O variant: the walk's refusals arrive as plan 03's
  `WalkError` and I/O failures as plan 01's `FormatError::Io`.
- Create `crates/agentstacks/src/model/entry_path.rs`: `EntryPath`, a private-field newtype.
  `EntryPath::new` returns an `ExtensionError` when it refuses, and checks in this order: an
  absolute path (a leading `/`) or any `..` segment gives `EntryOutside`; a path not ending in
  `.lua` gives `EntryNotLua`. It also has `as_str` and `Display`. It does no file-system access,
  because existence is checked in Task 3.
- Modify `crates/agentstacks/src/model/format_error.rs`: add the `Extension(#[from]
  ExtensionError)` variant, transparent.
- Modify `crates/agentstacks/src/model/mod.rs`: declare and re-export both items.

**Change:** The message vocabulary of spec §4.5, and its shape rule for `entry` (relative, no
`..`, ends in `.lua`). An absolute `entry` gets the same message as a `..` path, as the §4.5 row
states: "same message for an absolute `entry`".

**Proof:**
- `extension_error_displays_each_message`: each variant's `Display` equals the text listed above,
  with `value` = `adapter.lua` and `api` = `1`. For example, `ApiUnsupported { api: 1 }` displays
  `1 not supported (this agentstacks speaks 0)`, and `Name` built from plan 02's `InvalidName`
  for `Claude` displays `"Claude" is not a valid name`.
- `entry_path_accepts_relative_lua_paths`: `adapter.lua` and `lib/adapter.lua` are accepted, and
  `as_str` returns them unchanged.
- `entry_path_refuses_paths_leaving_the_extension`: `../a.lua`, `lib/../../a.lua` and `/a.lua`
  are each refused, displaying `"<that value>" must stay inside the extension`.
- `entry_path_refuses_non_lua_files`: `adapter.js` is refused as `"adapter.js" is not a .lua file`.
- `entry_path_checks_containment_before_extension`: `../a.js` and `/a.js` are each refused only
  as `"<that value>" must stay inside the extension`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add extension errors and entry paths`.

### Task 2 — Parse `extension.toml`

**Files:**
- Create `crates/agentstacks/src/model/extension_manifest.rs`: `ExtensionManifest` with private
  fields and accessors: `name` (`ToolName`, plan 02), `version` (`semver::Version`), `description`
  (a non-empty `String`) and `entry` (`EntryPath`). `api` is not stored, because `0` is the only
  accepted value.
  - `ExtensionManifest::parse(file, text) -> Result<ExtensionManifest, Diagnostics>`. Every
    location uses `file`.
  - A crate-private `ExtensionManifest::parse_with_entry(file, text)` returns that same result
    plus the `EntryPath` whenever `entry` itself is valid, even when another field failed, so
    Task 3 can check the entry's existence in the same run. `parse` delegates to it.
- Modify `crates/agentstacks/src/model/mod.rs`: declare and re-export.

**Change:** The text goes through `parse_toml_document`, then `TomlFields::new(file, "", table)`.
Every field is checked, even after an earlier field fails.
- On a syntax error, `parse_toml_document` gives the file's only diagnostic,
  `<file>:<line>: <parser message>` (§10). Duplicate keys, duplicate tables and integers beyond
  i64 arrive this way (§1.1, §10).
- The only top-level key is `extension`. `[extension]` is read with `table_or_empty`, so a missing
  table (empty text included, §10) is refused field by field. A non-table value gives
  `extension: expected table, got <type>` and its fields are not read.
- Inside `[extension]` the allowed keys are `name`, `version`, `api`, `description` and `entry`.
  `refuse_unknown_keys` reports anything else, at either level, as `<key.path>: unknown key`.
- Missing fields give `extension.<field>: required`, and fields of the wrong TOML type give
  `extension.<field>: expected <type>, got <type>`, with the §10 type words. Both come from
  `TomlFields`.
- `name` (`required_string`): a value that `ToolName::new` refuses gives
  `extension.name: "<value>" is not a valid name` (plan 02's `InvalidName` through
  `ExtensionError::Name`).
- `version` (`required_string`): a value `semver::Version::parse` refuses gives `NotSemver` at
  `extension.version`.
- `api` (`required_integer`): any integer other than `0` gives `ApiUnsupported` at
  `extension.api`.
- `description` (`required_text`): an empty or whitespace-only value gives the generic
  `extension.description: empty`.
- `entry` (`required_text`): an empty or whitespace-only value gives `extension.entry: empty`.
  Otherwise `EntryPath::new`'s refusal is reported at `extension.entry`.

This covers every file-local row of spec §4.5, with the §10 generic shapes for missing,
wrong-typed and empty fields and the §10 syntax-error line rule. Existence of `entry` is not
checked here, because it needs the directory.

**Proof:** Each case asserts the full `Diagnostic` `Display`, location plus message, with `file` =
`extension.toml`.
- `extension_manifest_parses_the_spec_example`: the spec §4.5 example parses to name `claude`,
  version `0.1.0`, description `Renders packs into Claude Code project config` and entry
  `adapter.lua`.
- `extension_manifest_reports_a_syntax_error_with_its_line`: the input is `[extension]`,
  `name = "claude"` and `a = [1,` on three lines, with no trailing newline. It yields exactly one
  diagnostic, `extension.toml:3: unclosed array, expected ']'`. The parser message is the spec §1.1
  probe output for `a = [1,`.
- `extension_manifest_refuses_an_empty_file_field_by_field`: `""` yields exactly
  `extension.toml: extension.name: required`, `… extension.version: required`,
  `… extension.api: required`, `… extension.description: required` and
  `… extension.entry: required` (§10: empty TOML is refused field by field).
- `extension_manifest_refuses_a_wrong_typed_extension_table`: `extension = "x"` yields exactly
  `extension.toml: extension: expected table, got string`.
- `extension_manifest_requires_every_field`: table-driven over the spec example with one field
  removed at a time. Each case yields exactly `extension.toml: extension.<field>: required` for
  `name`, `version`, `api`, `description` and `entry`.
- `extension_manifest_refuses_wrong_types`: `api = "0"` gives
  `extension.toml: extension.api: expected integer, got string`. `api = 0.5` gives
  `extension.toml: extension.api: expected integer, got float`. `name = 1` gives
  `extension.toml: extension.name: expected string, got integer`. `version = 1979-05-27` gives
  `extension.toml: extension.version: expected string, got datetime`. The float and datetime
  cases are spec §10's "a value of a TOML type no field accepts".
- `extension_manifest_refuses_an_invalid_name`: `name = "Claude"` gives
  `extension.toml: extension.name: "Claude" is not a valid name`.
- `extension_manifest_refuses_a_non_semver_version`: `version = "1.2"` gives
  `extension.toml: extension.version: "1.2" is not semver`.
- `extension_manifest_refuses_an_empty_description`: `""` and `"   "` each give
  `extension.toml: extension.description: empty`.
- `extension_manifest_refuses_api_other_than_zero`: `api = 1` gives
  `extension.toml: extension.api: 1 not supported (this agentstacks speaks 0)`. `api = -1` gives
  the same message with `-1`.
- `extension_manifest_refuses_bad_entries`:
  - `entry = "../a.lua"` gives `extension.toml: extension.entry: "../a.lua" must stay inside the
    extension`.
  - `entry = "/a.lua"` gives `extension.toml: extension.entry: "/a.lua" must stay inside the
    extension`.
  - `entry = "adapter.js"` gives `extension.toml: extension.entry: "adapter.js" is not a .lua
    file`.
  - `entry = ""` gives `extension.toml: extension.entry: empty`.
- `extension_manifest_refuses_unknown_keys`: `foo = 1` inside `[extension]` gives
  `extension.toml: extension.foo: unknown key` (the §4.5 row). `foo = 1` at the top level gives
  `extension.toml: foo: unknown key`.
- `extension_manifest_reports_all_errors_at_once`: one file has `name = "Claude"`,
  `version = "1.2"`, `api = 1`, `entry = "adapter.js"` and `foo = 1`. It yields exactly those five
  diagnostics, compared as a set of `Display` strings.
- `parse_with_entry_keeps_a_valid_entry_when_another_field_fails`: `name = "Claude"` with
  `entry = "adapter.lua"` returns `Err` with the name diagnostic and the entry `adapter.lua`;
  `entry = "adapter.js"` returns no entry.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse extension manifests`.

### Task 3 — Load an extension directory

**Files:**
- Create `crates/agentstacks/src/model/extension.rs`: `Extension` with private fields and
  accessors: `dir`, `manifest` (`ExtensionManifest`), `tree` (`ContentHash`, plan 03) and
  `ignored` (the walk's ignored dot entries from `Walked::ignored`, kept so `plan` can list them
  once). `Extension::load(dir) -> Result<Extension, Diagnostics>` works in this order:
  1. Walk `dir` with `walk(dir)`. If it refuses, return its diagnostics, each passed through
     `reword_walk_diagnostic` (symlink → extension wording, others unchanged), and stop. They cover symlinks (`<path>: symlinks are not allowed in an
     extension`), special files (`<path>: only regular files and directories are allowed`),
     non-UTF-8 paths (`<path>: path is not valid UTF-8`) and I/O failures, including a `dir` that
     cannot be read.
  2. If `extension.toml` is not among `Walked::files`, report `NotAnExtension` located at `dir`
     alone, and stop.
  3. Read `<dir>/extension.toml` as text. A read failure, including non-UTF-8 content, is
     `FormatError::Io` located at that path, displayed `<path>: <the std::io::Error Display>`
     (§10), and stops the load. Otherwise pass the text to `ExtensionManifest::parse_with_entry`
     with file = `<dir>/extension.toml`.
  4. If an `EntryPath` came back, compare it byte for byte with the `TreePath`s in
     `Walked::files`. A path that is not there (absent, inside a dot entry, or a directory) gives
     `EntryMissing` located at `<dir>/extension.toml`, key `extension.entry`.
  5. If no diagnostic was collected, compute `tree_hash(dir)` and return the
     `Extension`; a refusal from `tree_hash` is returned like any other. Otherwise return every
     diagnostic. A refused `api` therefore never yields an `Extension`.
- Modify `crates/agentstacks/src/model/mod.rs`: declare and re-export.

**Change:** Spec §4.5 rows "file present", "`entry` … an existing file" and "no symlinks;
non-UTF-8 paths refused; dot entries ignored | as §2.1". Through plan 03's walk, the §2.1 rows on
special files, empty directories and the dot-rule-first order apply too. The tree hash of spec
§8.1 is recorded for each loaded extension.

**Proof:** The tests run in `tempfile` directories. `<dir>` below is that directory's path as
passed to `load`, and every refusal asserts the full `Display`.
- `extension_load_accepts_a_minimal_extension`: the directory holds the spec §4.5 example and
  `adapter.lua`. The loaded `Extension` has name `claude`, entry `adapter.lua` and an empty
  `ignored` list.
- `extension_load_refuses_a_directory_without_a_manifest`: the directory holds only
  `adapter.lua`. It yields exactly `<dir>: not an extension (no extension.toml)`.
- `extension_load_refuses_a_missing_entry`: `extension.toml` names `adapter.lua`, which is
  absent. It yields exactly `<dir>/extension.toml: extension.entry: "adapter.lua" does not exist`.
- `extension_load_does_not_find_an_entry_inside_a_dot_directory`: `entry = ".hidden/adapter.lua"`
  exists on disk but is inside a dot entry. It yields exactly
  `<dir>/extension.toml: extension.entry: ".hidden/adapter.lua" does not exist`.
- `extension_load_passes_walk_refusals_through` (`#[cfg(unix)]`): a symlink `link` next to a
  valid manifest and entry yields exactly `<dir>/link: symlinks are not allowed in an extension`.
  A Unix socket `sock` (bound with `std::os::unix::net::UnixListener`) in place of the symlink
  yields exactly `<dir>/sock: only regular files and directories are allowed`.
- `extension_load_stops_at_a_walk_refusal` (`#[cfg(unix)]`): a symlinked `extension.toml` yields
  only `<dir>/extension.toml: symlinks are not allowed in an extension`, with no
  `not an extension` refusal.
- `extension_load_passes_a_non_utf8_path_refusal_through` (`#[cfg(target_os = "linux")]`): a
  file named with the bytes `0x66 0xFF` beside a valid manifest and entry yields exactly one
  diagnostic, located at `dir` joined with that name, whose message is `path is not valid UTF-8`.
- `extension_load_ignores_dot_entries_and_lists_them`: adding `.DS_Store` and `.git/config` to a
  minimal extension still loads. `ignored` holds `.DS_Store` and `.git`, and `tree` equals the
  tree of the same extension without them.
- `extension_load_ignores_empty_directories`: adding an empty directory `lib/` to a minimal
  extension still loads, and `tree` is unchanged.
- `extension_load_records_the_tree_hash`: `tree` equals `tree_hash(dir)` for
  the same directory. Changing one byte of `adapter.lua` changes `tree`.
- `extension_load_refuses_api_one_without_returning_an_extension`: `api = 1` yields `Err`
  holding exactly
  `<dir>/extension.toml: extension.api: 1 not supported (this agentstacks speaks 0)`.
- `extension_load_reports_an_unreadable_manifest_as_io`: an `extension.toml` holding bytes
  `0xFF 0xFE` yields exactly one diagnostic, located at `<dir>/extension.toml` with the
  `FormatError::Io` variant. The operating system's message text is not asserted.
- `extension_load_reports_every_refusal_together`: `name = "Claude"`, `foo = 1` in `[extension]`
  and a missing `adapter.lua` together yield exactly
  `<dir>/extension.toml: extension.name: "Claude" is not a valid name`,
  `<dir>/extension.toml: extension.foo: unknown key` and
  `<dir>/extension.toml: extension.entry: "adapter.lua" does not exist`, compared as a set of
  `Display` strings.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): load extension directories`.

### Task 4 — Add the extension fixtures and their integration test

**Files:**
- Create `crates/agentstacks/tests/fixtures/extensions/valid/minimal/`: the spec §4.5
  `extension.toml` and `adapter.lua`.
- Create one directory under `crates/agentstacks/tests/fixtures/extensions/invalid/` for each
  refusal. Each holds `adapter.lua` (except `entry-missing/`) and a manifest that differs from the
  spec example in one way:
  - `no-manifest/`: `adapter.lua` only.
  - `missing-name/`: `name` removed.
  - `bad-name/`: `name = "Claude"`.
  - `bad-version/`: `version = "1.2"`.
  - `empty-description/`: `description = ""`.
  - `api-one/`: `api = 1`.
  - `entry-outside/`: `entry = "../a.lua"`.
  - `entry-absolute/`: `entry = "/a.lua"`.
  - `entry-not-lua/`: `entry = "adapter.js"`.
  - `entry-missing/`: no `adapter.lua`.
  - `unknown-key/`: `foo = 1` in `[extension]`.
  - `syntax-error/`: the three-line Task 2 input, with no trailing newline.

  No fixture holds a symlink, a special file, a non-UTF-8 name, an empty directory or a dot
  entry, because git cannot carry them all reliably. Those cases are built at test time in Task 3.
- Create `crates/agentstacks/tests/extension_load.rs`: loads each fixture through the public
  `Extension::load`, with paths built from `CARGO_MANIFEST_DIR`.

**Change:** Spec §12's fixture requirement for extensions: one valid extension, and one invalid
input per refusal, each asserting the exact message and location.

**Proof:**
- `minimal_extension_fixture_loads`: `valid/minimal` loads with name `claude` and entry
  `adapter.lua`, and its `tree` equals `tree_hash(dir)` of the same
  directory.
- `each_invalid_extension_fixture_reports_its_refusal`: table-driven. Each fixture yields exactly
  one diagnostic whose `Display` is `<fixture dir>/extension.toml` (or `<fixture dir>` for
  `no-manifest`) followed by:
  - `no-manifest`: `: not an extension (no extension.toml)`
  - `missing-name`: `: extension.name: required`
  - `bad-name`: `: extension.name: "Claude" is not a valid name`
  - `bad-version`: `: extension.version: "1.2" is not semver`
  - `empty-description`: `: extension.description: empty`
  - `api-one`: `: extension.api: 1 not supported (this agentstacks speaks 0)`
  - `entry-outside`: `: extension.entry: "../a.lua" must stay inside the extension`
  - `entry-absolute`: `: extension.entry: "/a.lua" must stay inside the extension`
  - `entry-not-lua`: `: extension.entry: "adapter.js" is not a .lua file`
  - `entry-missing`: `: extension.entry: "adapter.lua" does not exist`
  - `unknown-key`: `: extension.foo: unknown key`
  - `syntax-error`: `:3: unclosed array, expected ']'`

**Done when:** `cargo make dod-crate agentstacks` passes, then `cargo make dod` passes, and the
Definition of Done is green. Commit `test(agentstacks): extension fixtures`.

---

## Risks

- This plan does not check that the extension's name matches its `[tools.<name>]` key (spec §7,
  `tools.claude: source contains extension "claud"`). Plan 10's `check_fetched_names` owns that,
  because the key lives in `agentstacks.toml`.
- A walk refusal stops the load, as in plan 06. Option not taken: checking presence and `entry`
  against a partial file list, which would add false `not an extension` or `does not exist`
  refusals for the entries the walk refused.
- `entry` is compared byte for byte with the walked `/`-separated paths, so `./adapter.lua` or
  `lib//adapter.lua` reports `does not exist`. No normalisation is applied.
- `load` walks the directory twice: once through `walk`, and again inside `tree_hash`, the only
  public entry point plan 03 defines. A hash-from-walked-list function was not added here, because
  it belongs to plan 03.
- `NotSemver` repeats the `"<value>" is not semver` wording that plan 05 uses for `pack.version`.
  Sharing plan 05's error variant was not done, because it would couple extension parsing to the
  pack manifest's error enum. `description` stays a validated `String` rather than a shared
  newtype for the same reason.
- The non-UTF-8 path test runs on Linux only, because macOS file systems refuse such names. CI
  runs it on `ubuntu-latest`.

## Proof

- `cargo make dod` green. No dependency changes, so `cargo make deny` is not required.
- Coverage map, one row per refusal in this plan's scope:

| Spec row | Message (after the location) | Tests |
|---|---|---|
| §4.5 file present | `not an extension (no extension.toml)` | `extension_load_refuses_a_directory_without_a_manifest`, fixture `no-manifest` |
| §4.5 `name` §4.1 | `extension.name: "Claude" is not a valid name` | `extension_manifest_refuses_an_invalid_name`, fixture `bad-name` |
| §4.5 `version` semver | `extension.version: "1.2" is not semver` | `extension_manifest_refuses_a_non_semver_version`, fixture `bad-version` |
| §4.5 `description` not empty | `extension.description: empty` | `extension_manifest_refuses_an_empty_description`, fixture `empty-description` |
| §4.5 `api` = 0, at load only | `extension.api: 1 not supported (this agentstacks speaks 0)` | `extension_manifest_refuses_api_other_than_zero`, `extension_load_refuses_api_one_without_returning_an_extension`, fixture `api-one` |
| §4.5 `entry` relative, no `..` (same message for an absolute `entry`) | `extension.entry: "../a.lua" must stay inside the extension` | `entry_path_refuses_paths_leaving_the_extension`, `extension_manifest_refuses_bad_entries`, fixtures `entry-outside`, `entry-absolute` |
| §4.5 `entry` ends in `.lua` | `extension.entry: "adapter.js" is not a .lua file` | `entry_path_refuses_non_lua_files`, `extension_manifest_refuses_bad_entries`, fixture `entry-not-lua` |
| §4.5 `entry` an existing file | `extension.entry: "adapter.lua" does not exist` | `extension_load_refuses_a_missing_entry`, `extension_load_does_not_find_an_entry_inside_a_dot_directory`, fixture `entry-missing` |
| §4.5 unknown key | `extension.foo: unknown key` | `extension_manifest_refuses_unknown_keys`, fixture `unknown-key` |
| §4.5 no symlinks (as §2.1) | `symlinks are not allowed in an extension` | `extension_load_passes_walk_refusals_through`, `extension_load_stops_at_a_walk_refusal` |
| §2.1 only regular files and directories (through the walk) | `only regular files and directories are allowed` | `extension_load_passes_walk_refusals_through` |
| §4.5 non-UTF-8 paths (as §2.1) | `path is not valid UTF-8` | `extension_load_passes_a_non_utf8_path_refusal_through` |
| §4.5 dot entries ignored | (not an error) | `extension_load_ignores_dot_entries_and_lists_them`, `extension_load_does_not_find_an_entry_inside_a_dot_directory` |
| §2.1 empty directories ignored (through the walk) | (not an error) | `extension_load_ignores_empty_directories` |
| §10 syntax error with line | `:<line>: <parser message>` | `extension_manifest_reports_a_syntax_error_with_its_line`, fixture `syntax-error` |
| §10 generic required | `extension.<field>: required` | `extension_manifest_requires_every_field`, `extension_manifest_refuses_an_empty_file_field_by_field`, fixture `missing-name` |
| §10 generic wrong type, type words | `extension.api: expected integer, got string` and others | `extension_manifest_refuses_wrong_types`, `extension_manifest_refuses_a_wrong_typed_extension_table` |
| §10 generic empty | `extension.entry: empty` | `extension_manifest_refuses_bad_entries` |
| §10 I/O failure | `<path>: <the std::io::Error Display>` | `extension_load_reports_an_unreadable_manifest_as_io` |
| §10 all errors per run | — | `extension_manifest_reports_all_errors_at_once`, `extension_load_reports_every_refusal_together`, `parse_with_entry_keeps_a_valid_entry_when_another_field_fails` |
| §8.1 tree hash for extensions | — | `extension_load_records_the_tree_hash`, `minimal_extension_fixture_loads` |

## Review findings

Built after the 2026-11-21 redesign. Crate-wide review rounds: review #1 (~70 findings) fixed in two waves; final review (0🔴 13🟡 17🔵 3❓). The 13🟡 and Q1–Q3 are fixed in the final fix wave (see the PR); the 17🔵 are listed in the PR as follow-ups.

## Deviations

- 2026-11-21 — **Redesign:** the spec was redesigned (`spec-superseded-2026-11-21.md` → `spec.md`) after plans 01–05 were built. This plan was built against the new spec, not the text above: code lives under `crates/agentstacks/src/types/<group>/` (domain types and logic, no I/O) with the read-only `types::port::FileTree` port and `fs::StdFileTree` as the only `std::fs` caller; loaders take `&impl FileTree`; file names never repeat their folder word. File paths and type names in the tasks above are superseded by the built tree.
- 2026-11-21 — Built in parallel with sibling plans by separate coders; consistency fixed afterwards by crate-wide reviews (one error collection type `Diagnostics`, `[i]` on element refusals, own error type per validating newtype, one api constant).
