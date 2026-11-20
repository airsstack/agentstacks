---
status: approved
created: 2026-11-20
depends-on: []
built-on: 3162a314af79
---

# Diagnostics Implementation Plan

**Goal:** Every format refusal becomes a located `Diagnostic` in one sorted list.

**Architecture:** `model` becomes a directory module (`model/mod.rs`, export-only, one item per
sibling file). Five files form the error vocabulary every later plan uses:

- `model/location.rs` — `Location { file, line: Option, key_path: Option }`. Fields are private;
  three constructors build a file-only, a line or a key-path location, so a location never holds
  both a line and a key path. A fourth constructor, `Location::set_level`, builds the set-level
  location of spec §10: the file `agentstacks.toml` plus an optional key path.
- `model/type_word.rs` — `TypeWord`, the closed set of type words of spec §10: `Boolean`,
  `Integer`, `Float`, `String`, `Datetime`, `Array`, `Table`, `Null`. Each displays as its
  lowercase spec word. The TOML and YAML layers map their own value kinds onto it (a YAML sequence
  is `Array`, a mapping is `Table`, §10); no free string is ever a type word.
- `model/field_error.rs` — `FieldError::{Required, WrongType { expected, got }, Empty, UnknownKey}`,
  the generic message shapes of spec §10, used by every field that has no message of its own.
  `expected` and `got` are `TypeWord`s.
- `model/format_error.rs` — `FormatError`, the `thiserror` enum that joins every module's error
  enum through a `#[from]` variant. This plan adds `Field` (wrapping `FieldError`) and `Io`
  (wrapping `std::io::Error`). `Io` is the only I/O variant in the crate: module error enums
  (walk, pack, extension, lock, …) carry no I/O variant of their own and push an I/O failure as
  `FormatError::Io`. Each later plan adds its own `#[from]` variant to this file.
- `model/diagnostic.rs` — `Diagnostic { location, error }`. Its `Display` is the "exact message"
  of spec §10: the location's display, `": "`, then the error's display.

`model/diagnostics.rs` holds `Diagnostics`, the collection a validation run returns. It keeps
every pushed diagnostic ("all errors per run", §10) and sorts on output by the §10 rule. No other
module changes behaviour.

Display forms (spec §10). The file is always written `/`-separated, whatever the platform:

| Location | Displays as | Spec example |
|---|---|---|
| line | `<file>:<line>` | `packs/sdlc/skills/plan/SKILL.md:68` (§3.3) |
| key path | `<file>: <key.path>` | `packs/core/pack.toml: hooks[2].grant` (§4.2) |
| file only | `<file>` | `agentstacks.lock` (§8, the `[airsl] missing` row) |
| set level, with key | `agentstacks.toml: <key.path>` | `agentstacks.toml: packs.sdlc`, `agentstacks.toml: packs.core.options.archive_keep` (§10) |
| set level, no key | `agentstacks.toml` | the cycle, airsl-range and name-clash rows (§10) |

| `FieldError` | Displays as (spec §10) |
|---|---|
| `Required` | `required` |
| `WrongType { expected, got }` | `expected <expected>, got <got>` |
| `Empty` | `empty` |
| `UnknownKey` | `unknown key` |

| `TypeWord` | `Boolean` | `Integer` | `Float` | `String` | `Datetime` | `Array` | `Table` | `Null` |
|---|---|---|---|---|---|---|---|---|
| Displays as (§10) | `boolean` | `integer` | `float` | `string` | `datetime` | `array` | `table` | `null` |

An I/O failure (§10) is a `Diagnostic` at `Location::file(path)` carrying `FormatError::Io`, so
it displays `<path>: <the std::io::Error Display>`.

Sort rule (spec §10): by file path as a `/`-separated string, compared in byte order; within one
file, key-path and no-line diagnostics first in the order they were found, then lined diagnostics
by ascending line number. The sort is stable, so everything the rule does not order (two key-path
diagnostics in one file, two diagnostics on one line) keeps its insertion order. Files never
compare with `Path`'s component ordering: byte order puts `a-b.md` before `a/b.md`, `Path`
ordering does the reverse.

**Tech Stack:** Rust 2024 (workspace `rust-version = "1.94"`, `Cargo.toml`), `thiserror` 2
(`2.0.21`, spec §11).

**Content authority:** spec §10 (Errors), §11 (module layout, `thiserror` dependency), §12
(coverage rule). Coverage of §10 in this plan:

| §10 rule | Task |
|---|---|
| line form `<file>:<line>: <what>`; key-path form `<file>: <key.path>: <what>` | 2, 4 |
| set-level location: `agentstacks.toml` plus the entry's key path, or the file only | 2 |
| generic shapes: required, wrong type, empty or whitespace-only, unknown key | 3, 4 |
| type words: boolean, integer, float, string, datetime, array, table, null | 3 |
| I/O failure is `<path>: <the std::io::Error Display>` | 4 |
| "exact message" is the `Diagnostic`'s `Display` | 4 |
| sort: file as `/`-separated string in byte order; no-line first in found order (stable), then by line | 5 |
| all errors per run | 5 |
| one `thiserror` enum per module, no string errors; a run returns a list of `Diagnostic` | 3, 4, 5 |

Owned elsewhere: raw `toml::Table` parsing and the syntax-error line from the span (plan 05,
`model/toml_document.rs`); mapping a TOML value kind or a YAML value kind onto `TypeWord` (plans
05 and 04); "a syntax error stops only that file" (each parser's plan); the token and hook-field
locations of set-level refusals (plan 10, through plan 02's `LocatedToken`).

---

## Files that change

```
Cargo.toml                                   — modify  add thiserror = "2" to [workspace.dependencies]
crates/agentstacks/Cargo.toml                — modify  depend on thiserror (workspace)
crates/agentstacks/src/lib.rs                — modify  module docs: model is no longer empty
crates/agentstacks/src/model.rs              — delete  replaced by model/mod.rs
crates/agentstacks/src/model/mod.rs          — create  export-only table of contents for model
crates/agentstacks/src/model/location.rs     — create  Location: file plus an optional line or key path
crates/agentstacks/src/model/type_word.rs    — create  TypeWord: the closed set of §10 type words
crates/agentstacks/src/model/field_error.rs  — create  FieldError: the spec §10 generic shapes
crates/agentstacks/src/model/format_error.rs — create  FormatError: joins module errors; the one Io variant
crates/agentstacks/src/model/diagnostic.rs   — create  Diagnostic { location, error }; Display = exact message
crates/agentstacks/src/model/diagnostics.rs  — create  Diagnostics: collects every diagnostic, sorts per §10
```

## Order of work

### Task 1 — Turn `model` into a directory module

**Files:**
- Delete `crates/agentstacks/src/model.rs`.
- Create `crates/agentstacks/src/model/mod.rs` — the same `//!` block, rewritten to the
  four-question shape of the mod-rs-export-only rule (what, why it exists, responsibilities,
  non-responsibilities) plus the export-only exemption line. It still states that the module holds
  no items yet.

**Change:** structure only; no item is added.

**Proof:** no new test (export-only file, unit-test-mandate exemption #1); the crate still builds,
lints and documents with zero warnings.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `refactor(agentstacks): make model a directory module`.

### Task 2 — Add `Location`

**Files:**
- Create `crates/agentstacks/src/model/location.rs` — `Location` with private fields `file`
  (`PathBuf`), `line` (`Option<NonZeroUsize>`, 1-based) and `key_path` (`Option<String>`, a dotted
  path such as `hooks[2].grants.read[0]`). Constructors: `Location::file(path)`,
  `Location::at_line(path, line)`, `Location::at_key(path, key_path)` and
  `Location::set_level(key_path: Option<…>)`, whose file is `agentstacks.toml`. Accessors `file()`,
  `line()`, `key_path()`. One crate-private method, `file_text()`, gives the file as a
  `/`-separated string (the path's components joined with `/`, each as `Path::display` writes
  it); `Display` writes the file through `file_text()`, then the line or key path per the table in
  Architecture, and Task 5 sorts on the same string.
- Modify `crates/agentstacks/src/model/mod.rs` — declare `location`, re-export `Location`, and
  replace "holds no items" with the module's responsibilities.
- Modify `crates/agentstacks/src/lib.rs` — the `model` bullet names the format vocabulary and its
  diagnostics; "Every module currently holds no items" becomes "every module except `model`".

**Change:** a position for every refusal (§10): a line, a key path, the file alone, or the
set-level `agentstacks.toml` form.

**Proof** (colocated `#[cfg(test)] mod tests` in `location.rs`):
- `at_line_displays_file_colon_line` — `packs/sdlc/skills/plan/SKILL.md` at line 68 displays as
  `packs/sdlc/skills/plan/SKILL.md:68`.
- `at_key_displays_file_then_key_path` — `packs/core/pack.toml` with key path `hooks[2].grant`
  displays as `packs/core/pack.toml: hooks[2].grant`.
- `file_only_displays_the_path` — `agentstacks.lock` displays as `agentstacks.lock`.
- `file_displays_slash_separated` — a file built as `Path::new("packs").join("core").join(
  "pack.toml")` displays as `packs/core/pack.toml`, and `file_text()` returns the same string.
- `set_level_with_key_displays_project_file_then_key` — `Location::set_level` with key path
  `packs.core.options.archive_keep` displays as
  `agentstacks.toml: packs.core.options.archive_keep`.
- `set_level_without_key_displays_project_file` — `Location::set_level` with no key path displays
  as `agentstacks.toml`.
- `accessors_return_constructed_parts` — an `at_line` location reports its file and line and no key
  path; an `at_key` location reports its file and key path and no line; a `file` location reports
  neither.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add Location for format diagnostics`.

### Task 3 — Add `TypeWord` and `FieldError`, the generic message shapes

**Files:**
- Modify `Cargo.toml` — add `thiserror = "2"` to `[workspace.dependencies]` (spec §11).
- Modify `crates/agentstacks/Cargo.toml` — add `thiserror = { workspace = true }` under
  `[dependencies]`.
- Create `crates/agentstacks/src/model/type_word.rs` — `TypeWord`, a fieldless enum (`Debug`,
  `Clone`, `Copy`, `PartialEq`, `Eq`) with exactly the eight §10 variants `Boolean`, `Integer`,
  `Float`, `String`, `Datetime`, `Array`, `Table`, `Null`; `Display` writes the lowercase word of
  the Architecture table. The `//!` block states that `Null` occurs only for YAML values and that
  YAML sequences and mappings use `Array` and `Table` (§10).
- Create `crates/agentstacks/src/model/field_error.rs` — `FieldError`, a `thiserror` enum with
  `Required`, `WrongType { expected: TypeWord, got: TypeWord }`, `Empty` and `UnknownKey`,
  displaying as in the Architecture table. One associated function,
  `FieldError::check_not_empty(value: &str)`, returns `Err(FieldError::Empty)` for an empty or
  whitespace-only string (§10) and `Ok(())` otherwise; it is the one blank-string check the TOML
  and frontmatter parsers share.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export `TypeWord` and `FieldError`.

**Change:** the four §10 generic shapes and the eight §10 type words as typed values, so no parser
builds these messages or words from strings.

**Proof** (colocated `#[cfg(test)] mod tests` in `type_word.rs`):
- `each_type_word_displays_its_spec_word` — `Boolean`, `Integer`, `Float`, `String`, `Datetime`,
  `Array`, `Table`, `Null` display as `boolean`, `integer`, `float`, `string`, `datetime`,
  `array`, `table`, `null`.

**Proof** (colocated `#[cfg(test)] mod tests` in `field_error.rs`):
- `required_displays_required` — `FieldError::Required` displays as `required`.
- `wrong_type_displays_expected_then_got` — `WrongType` with `TypeWord::Integer` and
  `TypeWord::Datetime` displays as `expected integer, got datetime`.
- `wrong_type_uses_yaml_words` — `WrongType` with `TypeWord::Table` and `TypeWord::Null` displays
  as `expected table, got null`.
- `empty_displays_empty` — `FieldError::Empty` displays as `empty`.
- `unknown_key_displays_unknown_key` — `FieldError::UnknownKey` displays as `unknown key`.
- `check_not_empty_refuses_empty_string` — `""` gives `Err(FieldError::Empty)`.
- `check_not_empty_refuses_whitespace_only` — `" \t "` gives `Err(FieldError::Empty)`.
- `check_not_empty_accepts_text_with_surrounding_spaces` — `" core "` gives `Ok(())`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green;
`cargo make deny` passes (new dependency). Commit
`feat(agentstacks): add FieldError generic message shapes`.

### Task 4 — Add `FormatError` and `Diagnostic`

**Files:**
- Create `crates/agentstacks/src/model/format_error.rs` — `FormatError`, a `thiserror` enum with
  two variants: `Field(#[from] FieldError)` and `Io(#[from] std::io::Error)`, both
  `#[error(transparent)]` so each displays its inner error unchanged. The `//!` block states that
  every module error enum joins here through one `#[from]` variant, that `Io` is the crate's only
  I/O variant, and that an I/O failure is located at the path that failed (`Location::file`), so
  it reads `<path>: <the std::io::Error Display>` (§10).
- Create `crates/agentstacks/src/model/diagnostic.rs` — `Diagnostic` with private fields `location`
  (`Location`) and `error` (`FormatError`); `Diagnostic::new(location, error)` takes any error that
  converts into `FormatError`; accessors `location()` and `error()`. `Display` writes the location,
  `": "`, then the error.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export `FormatError` and
  `Diagnostic`.

**Change:** the located refusal every parser and check pushes, with the §10 "exact message" as its
`Display`. A spec message given in the form `<location>: <message>` is checked in later plans by
comparing this `Display` with the spec text.

**Proof** (colocated `#[cfg(test)] mod tests` in `format_error.rs`):
- `field_error_converts_and_displays_unchanged` — a `FormatError` built from
  `FieldError::UnknownKey` displays as `unknown key`.
- `io_error_converts_and_displays_its_own_message` — a `FormatError` built from
  `std::io::Error::new(ErrorKind::PermissionDenied, "denied")` displays exactly as that
  `std::io::Error`'s own `Display` (`denied`).

**Proof** (colocated `#[cfg(test)] mod tests` in `diagnostic.rs`; each asserts the full `Display`):
- `required_at_key_path` — `agentstacks.lock`, key `pack[1].rev`, `Required` →
  `agentstacks.lock: pack[1].rev: required` (§8).
- `required_api_in_empty_project_file` — `agentstacks.toml`, key `api`, `Required` →
  `agentstacks.toml: api: required` (§10).
- `empty_at_key_path` — `agentstacks.lock`, key `merge[0].key`, `Empty` →
  `agentstacks.lock: merge[0].key: empty` (§8).
- `unknown_key_in_pack_manifest` — `packs/core/pack.toml`, key `hooks[2].grant`, `UnknownKey` →
  `packs/core/pack.toml: hooks[2].grant: unknown key` (§4.2).
- `unknown_key_in_extension_manifest` — `extension.toml`, key `extension.foo`, `UnknownKey` →
  `extension.toml: extension.foo: unknown key` (§4.5).
- `wrong_type_at_key_path` — `agentstacks.toml`, key `api`, `WrongType` with `Integer` and
  `Datetime` → `agentstacks.toml: api: expected integer, got datetime` (§10 shape and words).
- `line_location_joins_with_colon_space` — `packs/sdlc/skills/plan/SKILL.md` at line 68, `Empty` →
  `packs/sdlc/skills/plan/SKILL.md:68: empty` (the line form of §10; composition only).
- `io_error_at_file` — `packs/core/pack.toml`, file only, the `PermissionDenied` I/O error with the
  message `denied` → `packs/core/pack.toml: denied`, i.e. `<path>: <the std::io::Error Display>`
  (§10).
- `accessors_return_parts` — `location()` displays as the location passed in; `error()` displays
  as the error passed in.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add Diagnostic and FormatError`.

### Task 5 — Add sorted `Diagnostics`

**Files:**
- Create `crates/agentstacks/src/model/diagnostics.rs` — `Diagnostics` (derives `Default`):
  `new`, `push(Diagnostic)`, an `Extend<Diagnostic>` impl, `len`, `is_empty`, `into_sorted`
  (a `Vec<Diagnostic>` in §10 order), an `IntoIterator` impl that yields the same §10 order (so
  one `Diagnostics` extends another, as a set check does with load results), and
  `into_result(value)` (`Ok(value)` when empty, else `Err` holding the sorted
  `Vec<Diagnostic>`). Sorting is a stable sort on the key (`file_text()` as bytes, has a line,
  line) described in Architecture.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export `Diagnostics`.

**Change:** "all errors per run"; sorted by file path as a `/`-separated string in byte order;
within a file, key-path and no-line diagnostics first in the order they were found, then by line
(§10).

**Proof** (colocated `#[cfg(test)] mod tests` in `diagnostics.rs`; diagnostics are told apart by
their `Display`):
- `new_collection_is_empty` — `len` is 0 and `is_empty` holds.
- `push_and_extend_keep_every_diagnostic` — one `push` and an `extend` with two more give `len` 3.
- `into_sorted_orders_by_file` — pushed `b.md` then `a.md` (file only) come out `a.md`, `b.md`.
- `into_sorted_compares_files_as_slash_separated_bytes` — pushed `a/b.md` (built as
  `Path::new("a").join("b.md")`) then `a-b.md` come out `a-b.md`, `a/b.md` (`-` is byte `0x2D`,
  `/` is `0x2F`; `Path` ordering would give the reverse).
- `into_sorted_puts_unlined_before_lined_in_one_file` — pushed `a.md:2`, `a.md` key `x`, `a.md`
  file only come out key `x`, file only, then `a.md:2`.
- `into_sorted_orders_lines_numerically` — pushed `a.md:10`, `a.md:9`, `a.md:2` come out 2, 9, 10.
- `into_sorted_keeps_found_order_for_ties` — two diagnostics at `a.md:4` keep their push order;
  key paths `pack.version` then `pack.api` in one file keep their push order; a file-only
  diagnostic pushed before a key-path one in the same file stays first.
- `into_sorted_mixed_files_and_lines` — pushed `b.md:3`, `a.md:9`, `a.md` key `x`, `a.md:2` come
  out `a.md` key `x`, `a.md:2`, `a.md:9`, `b.md:3`.
- `extend_with_another_collection_keeps_all_and_sorts` — a collection holding `b.md:1` extended
  with a second collection holding `a.md` key `x` and `a.md:3` gives `len` 3 and sorts to `a.md`
  key `x`, `a.md:3`, `b.md:1`.
- `into_result_is_ok_only_when_empty` — an empty collection gives `Ok` with the value; a collection
  holding `b.md:1` then `a.md:1` gives `Err` with both, `a.md:1` first.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green, then
`cargo make dod` passes for the workspace. Commit `feat(agentstacks): add sorted format
diagnostics`.

---

## Risks

- `FormatError` grows one `#[from]` variant per later plan, each in
  `crates/agentstacks/src/model/format_error.rs`. A plan that forgets its variant cannot call
  `Diagnostic::new` with its error type, so the omission fails to compile rather than slipping
  through.
- `FormatError` holds a `std::io::Error`, so tests compare `Display` output, never error values.
- A later plan could add an I/O variant to its own module enum and so create a second I/O concept;
  the `//!` block of `format_error.rs` names `Io` as the only one, and reviewers hold later plans
  to it.
- `TypeWord` is closed: a parser that meets a value kind outside the eight §10 words cannot express
  it, which is the point. Messages the spec words differently (§5 `expected a string, got
  <type>`, `frontmatter: expected a mapping, got null`; §6.2 `expected true or false`) are not
  `FieldError::WrongType` and belong to their own module's error enum.
- The spec does not say how a path that is not valid UTF-8 is displayed in a location (it occurs
  in the §2.1 `path is not valid UTF-8` refusal). `file_text()` uses `Path::display`'s lossy form
  per component; open for the author.
- Doc comments must follow the doc-comment-discipline rule: no chain, plan or task identifiers in
  source.
- Option not taken: a `KeyPath` newtype (key paths are only built by the parsers and displayed,
  never parsed back, so a `String` field behind a private constructor suffices for now).

## Proof

- `cargo make dod-crate agentstacks` green after every task; `cargo make deny` green after Task 3;
  `cargo make dod` green after Task 5.
- 37 colocated unit tests: `location.rs` 7, `type_word.rs` 1, `field_error.rs` 8,
  `format_error.rs` 2, `diagnostic.rs` 9, `diagnostics.rs` 10.
