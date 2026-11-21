---
status: done
created: 2026-11-20
depends-on: [01, 02, 03, 05, 08]
built-on: 95010069150c
---

# Lockfile Implementation Plan

**Goal:** `agentstacks.lock` round-trips through one strict `Lockfile` type with a deterministic
writer.

**Architecture:** Ten new files under `crates/agentstacks/src/lock/`, one item each, re-exported
by the export-only `lock/mod.rs` (created by plan 03).

- `lock_error.rs` holds `LockError`, the module's one `thiserror` enum (spec §10). It joins
  `FormatError` (plan 01, `model/format_error.rs`) through a new `Lock` variant. Every message is
  copied from spec §8; the location is never part of it.
- `owner.rs` holds `Owner`, the value of a `by` field: the name of a locked pack or tool.
- `merge_key.rs` holds `MergeKey`, the non-empty `[[merge]].key` locator. Its meaning belongs to
  `core-plan-and-apply` (spec §8, §13).
- `merge_file.rs` holds `MergeFile`, the non-empty `[[merge]].file` string (spec §8: "`[[merge]].file`
  a non-empty string"; no path rule is stated for it).
- `locked_entry.rs` holds `LockedEntry<N>`, one `[[tool]]` (`N` = `ToolName`) or `[[pack]]`
  (`N` = `PackName`) entry: name, version, `Source` (plan 08), the optional recorded `GitTag`
  (plan 08), and the tree hash (`ContentHash`, plan 03).
- `file_entry.rs` holds `FileEntry` (`path`, `hash`, `by`); `merge_entry.rs` holds `MergeEntry`
  (`file`, `key`, `hash`, `by`).
- `lockfile.rs` holds `Lockfile` with `Lockfile::parse(file, text)` and `Lockfile::new(…)`. Both
  enforce the whole-file rules (api, `[airsl]`, unique names, paths and merges, `by`) and store
  every entry list sorted (spec §8 writer rule), so the writer never sorts.
- `write_lock.rs` holds `write_lock(&Lockfile)`, which formats the text itself. A serialised
  `toml::Table` cannot be used: a raw table is sorted by key (spec §1.1, `preserve_order` off), so
  fields would come out alphabetically, not in the §8 order.
- `check_airsl_presence.rs` holds `check_airsl_presence`, the "`[airsl]` present iff a locked pack
  declares `requires.airsl`" rule. It needs the loaded pack manifests, so it is tested here and
  called by `core-plan-and-apply` (spec §8, §13), never by plan 10.

Parsing never uses a `deny_unknown_fields` struct: the text goes through plan 05's
`parse_toml_document` (`toml_document.rs`) to a raw `toml::Table`, and every field is read by hand
through plan 05's `TomlFields` (`toml_fields.rs`), so every error in the file is reported (spec
§10; §1.1: a `deny_unknown_fields` parse stops at the first error). A syntax error, including a
duplicate key or table (§1.1), is the file's only diagnostic. Every refusal is a `Diagnostic`
(plan 01) located per spec §10: `agentstacks.lock: <key.path>: <what>` for a field
(`Location::at_key`, `pack[1].rev`), `agentstacks.lock: <what>` for a whole-file rule
(`Location::file`), `agentstacks.lock:<line>: <parser message>` for a syntax error; all are
collected in plan 01's `Diagnostics`. Wrong-type messages use the §10 type words through
`TomlFields` (`string`, `integer`, `float`, `boolean`, `datetime`, `array`, `table`).

Sources are read with plan 08's `SourceFields::read(entry, file, key_path, PinRule::RevRequired)`:
git entries need `rev`, `tag` is optional, and every other §7 source rule applies unchanged with
plan 08's `SourceError` messages. `Source::from_fields` then pins the rev (spec §8: "`rev` wins
over a tag"), and the entry keeps the recorded tag from `SourceFields::tag()` for the writer.
Whether a lock `tag` agrees with `agentstacks.toml` is never checked here: a missing or extra tag
makes the lock stale, not malformed, and staleness belongs to `core-plan-and-apply` (§8, §13).

Errors placed by this plan but owned elsewhere: `FieldError` (plan 01: `required`,
`expected <type>, got <type>`, `empty`, `unknown key`), plan 02's `InvalidName`
(`"<value>" is not a valid name`), plan 03's `ContentHashError`
(`bad hash (expected sha256:<64 lowercase hex>)`, wrapped by `LockError`), and plan 08's
`SourceError`.

**Tech Stack:** Rust 2024; `toml` 1 and `semver` 1 (added by plan 05); `sha2` 0.11 through
`ContentHash` (added by plan 03); `thiserror` 2 (added by plan 01). No new dependency.

**Content authority:** spec §8 (the example, every row of the table including the entry
`name`/`version` messages and the non-empty `[[merge]].file`, the writer rule, and the prose: a
missing lock is not an error, a disagreeing lock is stale, not malformed), §7 (the source rules §8
applies to `[[tool]]`/`[[pack]]`), §4.2 (the shapes §8 reuses for an entry's `name` and
`version`), §1.1 (toml facts: raw `Table` sorted by key, syntax
errors with a span, duplicate keys and tables are syntax errors, `Datetime` values in a raw table,
empty TOML is an empty table, `Version::parse("1.2")` is an error), §10 (locations, generic shapes
and type words, all errors per run, syntax line from the span, empty TOML refused field by field),
§11 (`lock/lockfile.rs`), §13 (`check_airsl_presence` is called by `core-plan-and-apply`).

---

## Files that change

```
crates/agentstacks/src/lock/mod.rs                   — modify  declare and re-export the ten new items; docs name the lockfile
crates/agentstacks/src/model/format_error.rs         — modify  FormatError gains Lock(#[from] LockError)
crates/agentstacks/src/lock/lock_error.rs            — create  LockError (thiserror): the §8 messages
crates/agentstacks/src/lock/owner.rs                 — create  Owner: the name a by field holds
crates/agentstacks/src/lock/merge_key.rs             — create  MergeKey: non-empty [[merge]].key
crates/agentstacks/src/lock/merge_file.rs            — create  MergeFile: non-empty [[merge]].file
crates/agentstacks/src/lock/locked_entry.rs          — create  LockedEntry<N> and LockedEntry::from_table for [[tool]]/[[pack]]
crates/agentstacks/src/lock/file_entry.rs            — create  FileEntry and FileEntry::from_table for [[file]]
crates/agentstacks/src/lock/merge_entry.rs           — create  MergeEntry and MergeEntry::from_table for [[merge]]
crates/agentstacks/src/lock/lockfile.rs              — create  Lockfile, Lockfile::parse, Lockfile::new
crates/agentstacks/src/lock/write_lock.rs            — create  write_lock: the §8 text, sorted, fields in §8 order
crates/agentstacks/src/lock/check_airsl_presence.rs  — create  check_airsl_presence: [airsl] present iff a locked pack needs airsl
```

## Order of work

### Task 1 — Add `LockError`, `Owner`, `MergeKey` and `MergeFile`

**Files:**
- Create `crates/agentstacks/src/lock/lock_error.rs` — `LockError`. Variants and their `Display`
  text, copied from spec §8:
  - `ApiUnsupported { api }` → `api <api> not supported (this agentstacks speaks 0)`;
  - `NotAVersion { value }` → `"<value>" is not a version` (`[airsl] version`);
  - `NotSemver { value }` → `"<value>" is not semver` (an entry's `version`, §8
    `pack[1].version: "1.2" is not semver`);
  - `Hash(ContentHashError)` → transparent, so it displays plan 03's
    `bad hash (expected sha256:<64 lowercase hex>)`;
  - `NotRelative { value }` → `"<value>" must be relative`;
  - `OutsideProject { value }` → `"<value>" must stay inside the project`;
  - `UnknownOwner { name }` → `"<name>" is not a locked pack or tool`;
  - `DuplicateTool { name }` → `tool "<name>" listed twice`;
  - `DuplicatePack { name }` → `pack "<name>" listed twice`;
  - `DuplicateFile { path }` → `file "<path>" listed twice`;
  - `DuplicateMerge { file, key }` → `merge "<file>" "<key>" listed twice`;
  - `AirslMissing` → `[airsl] missing, but locked packs require airsl`;
  - `AirslUnneeded` → `[airsl] present, but no locked pack requires airsl`.
- Create `crates/agentstacks/src/lock/owner.rs` — `Owner`, a private-field newtype over a §4.1
  name. `Owner::parse(&str)` accepts what plan 02's `PackName::new` accepts (packs and tools share
  the §4.1 rule) and returns plan 02's `InvalidName` otherwise; `From<&PackName>` and
  `From<&ToolName>` convert infallibly; `as_str`, `Display`, `Ord`.
- Create `crates/agentstacks/src/lock/merge_key.rs` — `MergeKey`, a private-field newtype.
  `MergeKey::new` refuses an empty or whitespace-only string through plan 01's
  `FieldError::check_not_empty` (`FieldError::Empty`, displays `empty`); `as_str`, `Display`,
  `Ord`.
- Create `crates/agentstacks/src/lock/merge_file.rs` — `MergeFile`, the same shape as `MergeKey`
  for the `[[merge]].file` string: `MergeFile::new` refuses an empty or whitespace-only string
  with `FieldError::Empty`; no other rule (spec §8 states none); `as_str`, `Display`, `Ord`.
- Modify `crates/agentstacks/src/model/format_error.rs` — add the `Lock(#[from] LockError)`
  variant.
- Modify `crates/agentstacks/src/lock/mod.rs` — declare and re-export the four items; the
  responsibilities list names the lockfile.

**Change:** the message vocabulary of spec §8 and the three small value types the entries hold.

**Proof:**
- `lock_error_displays_each_spec_message` (lock_error.rs) — each variant's `Display` equals its
  text above with the spec's sample values: `ApiUnsupported { api: 1 }` →
  `api 1 not supported (this agentstacks speaks 0)`; `NotAVersion` `0.1` → `"0.1" is not a
  version`; `NotSemver` `1.2` → `"1.2" is not semver`; `Hash` → `bad hash (expected
  sha256:<64 lowercase hex>)`; `NotRelative` `/x` → `"/x" must be relative`; `OutsideProject`
  `../x` → `"../x" must stay inside the project`; `UnknownOwner` `kore` → `"kore" is not a locked
  pack or tool`; `DuplicateTool` `claude` → `tool "claude" listed twice`; `DuplicatePack` `core` →
  `pack "core" listed twice`; `DuplicateFile` `.claude/x` → `file ".claude/x" listed twice`;
  `DuplicateMerge` `.claude/settings.json`, `<key>` → `merge ".claude/settings.json" "<key>"
  listed twice`; `AirslMissing` and `AirslUnneeded` → the two §8 texts.
- `owner_parses_names_and_converts_from_pack_and_tool` (owner.rs) — `core` parses and displays
  `core`; `Kore` is refused with `"Kore" is not a valid name`; `Owner::from(&PackName core)` equals
  the parsed `core`; `Owner::from(&ToolName claude)` displays `claude`.
- `merge_key_accepts_a_locator` (merge_key.rs) — `<locator>` constructs and `as_str` returns it
  unchanged.
- `merge_key_refuses_empty_and_whitespace` (merge_key.rs) — `""` and `" \t"` are each refused,
  displaying `empty`.
- `merge_file_accepts_the_spec_file` (merge_file.rs) — `.claude/settings.json` constructs and
  `as_str` returns it unchanged.
- `merge_file_refuses_empty_and_whitespace` (merge_file.rs) — `""` and `" \t"` are each refused,
  displaying `empty`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add lock errors and lock value types`.

### Task 2 — Parse `[[tool]]` and `[[pack]]` entries

**Files:**
- Create `crates/agentstacks/src/lock/locked_entry.rs` — `LockedEntry<N>` with private fields
  `name: N`, `version: semver::Version`, `source: Source`, `tag: Option<GitTag>` and
  `tree: ContentHash`, and accessors for each. Two infallible constructors keep the lock's
  invariants by type: `LockedEntry::git(name, version, url: GitUrl, tag: Option<GitTag>, subdir:
  Option<RelPath>, rev: CommitId, tree)` builds `Source::Git` pinned by `Pin::Rev` (all plan 08
  types), and `LockedEntry::path(name, version, path: RelPath, tree)` builds `Source::Path` with
  no tag. `LockedEntry::from_table(file, key_path, table, name_rule, diagnostics)` reads one raw
  entry table through `TomlFields::new(file, key_path, table)`, where `key_path` is `tool[i]` or
  `pack[i]` and `name_rule` is `ToolName::new` or `PackName::new` (plan 02); it pushes every
  refusal and returns the entry only when there was none.
- Modify `crates/agentstacks/src/lock/mod.rs` — declare and re-export `LockedEntry`.

**Change:** the `[[tool]]`/`[[pack]]` row of spec §8, all in one pass over the entry:

- allowed keys `name`, `version`, `git`, `path`, `tag`, `rev`, `subdir`, `tree`; any other is
  `<key_path>.<key>: unknown key` (`pack[1].branch`, `pack[0].options`);
- `name`, `version`, `tree` required (`<key_path>.<field>: required`) and strings (generic wrong
  type);
- `name` through `name_rule`, its `InvalidName` refused as `<key_path>.name: "<value>" is not a
  valid name` (§8);
- `version` through `semver::Version::parse`, refused as `<key_path>.version: "<value>" is not
  semver` (§8);
- `tree` through `ContentHash::parse` (plan 03), refused as `<key_path>.tree: bad hash (expected
  sha256:<64 lowercase hex>)`;
- the source keys through `SourceFields::read(table, file, key_path, PinRule::RevRequired)`
  (plan 08), which reports `needs exactly one of git, path`, `tag, rev and subdir apply only to
  git sources` (both at `<key_path>`), `<key_path>.rev: required`, and the field shapes (`rev` not
  a commit id, `git` leading `-`, `subdir` escape, `path` absolute or climbing, empty strings);
- a read git source becomes `Source::from_fields` (pinned by rev) plus the tag from
  `SourceFields::tag()`; a path source carries no tag.

The tag is kept as written; no rule compares it with `agentstacks.toml` (spec §8: stale, not
malformed).

**Proof** (each refusal asserts the full `Diagnostic` `Display`; file `agentstacks.lock`):
- `locked_entry_parses_the_spec_git_entry` — the §8 `core` entry, with `<40 hex>` and `<64 hex>`
  replaced by fixed lowercase hex, parses under `pack[0]` with no diagnostics: name `core`,
  version `0.3.0`, `Source::Git` with url `https://github.com/airsstack/agentstacks`, subdir
  `packs/core` and `Pin::Rev` of that hex, tag `packs-v0.1.0`, tree equal to that hash.
- `locked_entry_parses_the_spec_path_entry` — the §8 `team-lint` entry parses to
  `Source::Path` `packs/team-lint`, no tag.
- `locked_entry_keeps_the_tag_optional` — a git entry with `rev` and no `tag` parses with no tag;
  a git entry with both parses and keeps both; under `tool[0]` the §8 `claude` entry parses as a
  `LockedEntry<ToolName>`.
- `locked_entry_refuses_each_rule` — table-driven, each entry otherwise valid, exactly one
  diagnostic each:
  - no `rev` on a git entry → `agentstacks.lock: pack[1].rev: required`;
  - no `tree` → `agentstacks.lock: pack[1].tree: required`;
  - no `name` → `agentstacks.lock: pack[1].name: required`;
  - no `version` → `agentstacks.lock: pack[1].version: required`;
  - both `git` and `path`, and neither → `agentstacks.lock: pack[1]: needs exactly one of git,
    path`;
  - a path entry with `tag`, with `rev`, with `subdir` → `agentstacks.lock: pack[2]: tag, rev and
    subdir apply only to git sources`;
  - `rev = "main"` → `agentstacks.lock: pack[1].rev: not a commit id`;
  - `git = "-x"` → `agentstacks.lock: pack[1].git: URL must not start with "-"`;
  - `subdir = "../x"` → `agentstacks.lock: pack[1].subdir: must stay inside the repository`;
  - `path = "/abs"` → `agentstacks.lock: pack[2].path: absolute paths are not portable`;
  - `path = "../x"` → `agentstacks.lock: pack[2].path: must stay inside the project`;
  - `tag = " "` → `agentstacks.lock: pack[1].tag: empty`;
  - `tree` with uppercase hex, and `tree` without the `sha256:` prefix →
    `agentstacks.lock: pack[1].tree: bad hash (expected sha256:<64 lowercase hex>)`;
  - `name = "Core"` → `agentstacks.lock: pack[1].name: "Core" is not a valid name`;
  - `version = "1.2"` → `agentstacks.lock: pack[1].version: "1.2" is not semver`;
  - `name = 3` → `agentstacks.lock: pack[1].name: expected string, got integer`;
  - `branch = "main"` → `agentstacks.lock: pack[1].branch: unknown key`;
  - under `tool[0]`, no `rev` → `agentstacks.lock: tool[0].rev: required`.
- `locked_entry_reports_every_field_error` — one entry with no `name`, `tree = "sha256:XYZ"` and
  `branch = "main"` gives exactly those three diagnostics, compared as a set of `Display` strings,
  and no entry.
- `locked_entry_constructors_pin_the_rev` — `LockedEntry::git` yields `Source::Git` with
  `Pin::Rev` and the given tag; `LockedEntry::path` yields `Source::Path` and no tag.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse locked tool and pack entries`.

### Task 3 — Parse `[[file]]` and `[[merge]]` entries

**Files:**
- Create `crates/agentstacks/src/lock/file_entry.rs` — `FileEntry` with private fields
  `path: RelPath` (plan 08), `hash: ContentHash`, `by: Owner`, accessors, an infallible
  `FileEntry::new(path, hash, by)`, and `FileEntry::from_table(file, key_path, table,
  diagnostics)` for `file[i]`.
- Create `crates/agentstacks/src/lock/merge_entry.rs` — `MergeEntry` with private fields
  `file: MergeFile`, `key: MergeKey`, `hash: ContentHash`, `by: Owner`, accessors, an infallible
  `MergeEntry::new(file, key, hash, by)`, and `MergeEntry::from_table(file, key_path, table,
  diagnostics)` for `merge[i]`.
- Modify `crates/agentstacks/src/lock/mod.rs` — declare and re-export both.

**Change:** the `[[file]]`, `[[merge]]`, hash and `by`-shape rows of spec §8, each table read
through `TomlFields::new(file, key_path, table)`:

- `[[file]]` allows `path`, `hash`, `by`; `[[merge]]` allows `file`, `key`, `hash`, `by`; any
  other key is `<key_path>.<key>: unknown key`. Every listed field is required and a string
  (generic shapes).
- `path`: a value starting with `/` is `NotRelative` (`file[0].path: "/x" must be relative`);
  otherwise it is built with `RelPath::in_repository` (plan 08), whose invariant (no leading `/`,
  no `..` segment) is exactly §8's, and its `SourceError::OutsideRepository` refusal is reported
  as `OutsideProject` (`file[0].path: "../x" must stay inside the project`); an empty or
  whitespace-only value is `empty`. Paths are `/`-separated strings, stored and written as given.
- `hash` through `ContentHash::parse`, refused as `<key_path>.hash: bad hash (expected
  sha256:<64 lowercase hex>)`.
- `file` (merge) through `MergeFile::new`, refused as `<key_path>.file: empty` (§8: a non-empty
  string).
- `key` through `MergeKey::new`, refused as `<key_path>.key: empty`.
- `by` through `Owner::parse`; a value that is not a name is refused as `UnknownOwner` at
  `<key_path>.by` (`file[0].by` / `merge[i].by`, §8; a non-name can never be a locked pack or
  tool). Whether a valid name is actually locked is checked by the lockfile (Task 4), which holds
  the entry list.

**Proof** (each refusal asserts the full `Diagnostic` `Display`; file `agentstacks.lock`):
- `file_entry_parses_the_spec_entry` — the §8 `[[file]]` entry, `<64 hex>` replaced, parses
  under `file[0]` to path `.claude/skills/discuss/SKILL.md`, that hash and owner `core`.
- `file_entry_refuses_each_rule` — table-driven, exactly one diagnostic each: no `path` →
  `agentstacks.lock: file[0].path: required` (same for `hash`, `by`); `path = "/x"` →
  `agentstacks.lock: file[0].path: "/x" must be relative`; `path = "../x"` and `path = "a/../b"`
  → `agentstacks.lock: file[0].path: "<that value>" must stay inside the project`;
  `path = ""` → `agentstacks.lock: file[0].path: empty`; `hash = "sha256:XYZ"` →
  `agentstacks.lock: file[0].hash: bad hash (expected sha256:<64 lowercase hex>)`;
  `by = "Kore"` → `agentstacks.lock: file[0].by: "Kore" is not a locked pack or tool`;
  `path = 7` → `agentstacks.lock: file[0].path: expected string, got integer`;
  `mode = "0644"` → `agentstacks.lock: file[0].mode: unknown key`.
- `merge_entry_parses_the_spec_entry` — the §8 `[[merge]]` entry parses under `merge[0]` to file
  `.claude/settings.json`, key `<locator>`, that hash and owner `core`.
- `merge_entry_refuses_each_rule` — table-driven, exactly one diagnostic each: no `file`, `key`,
  `hash`, `by` → `agentstacks.lock: merge[0].<field>: required`; `key = ""` and `key = "  "` →
  `agentstacks.lock: merge[0].key: empty`; `file = ""` and `file = "  "` →
  `agentstacks.lock: merge[0].file: empty`; `file = 3` → `agentstacks.lock: merge[0].file:
  expected string, got integer`; a bad `hash` → `agentstacks.lock: merge[0].hash: bad
  hash (expected sha256:<64 lowercase hex>)`; `by = "Core"` → `agentstacks.lock: merge[0].by:
  "Core" is not a locked pack or tool`; `path = "x"` → `agentstacks.lock: merge[0].path: unknown
  key`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse locked file and merge entries`.

### Task 4 — Parse the whole lockfile

**Files:**
- Create `crates/agentstacks/src/lock/lockfile.rs` — `Lockfile` with private fields
  `airsl: Option<semver::Version>`, `tools: Vec<LockedEntry<ToolName>>`,
  `packs: Vec<LockedEntry<PackName>>`, `files: Vec<FileEntry>`, `merges: Vec<MergeEntry>`, and
  accessors. `api` is not stored, because `0` is the only accepted value.
  `Lockfile::parse(file, text)` returns the `Lockfile` or every `Diagnostic`.
  `Lockfile::new(airsl, tools, packs, files, merges)` builds one from typed entries (for
  `core-plan-and-apply` to write) and applies the same whole-file rules, returning
  `Diagnostics` on refusal.
- Modify `crates/agentstacks/src/lock/mod.rs` — declare and re-export `Lockfile`.

**Change:** the whole-file rows of spec §8:

- the text goes through `parse_toml_document`; a syntax error is the only diagnostic, located
  `<file>:<line>` (§10, line from the span, §1.1). Empty text is an empty table (§1.1) and is
  refused field by field;
- top-level keys `api`, `airsl`, `tool`, `pack`, `file`, `merge`; any other is
  `<key>: unknown key`;
- `api` required, an integer (generic shapes at `api`); any other value than 0 is
  `ApiUnsupported`, located at the file alone, matching the §8 text;
- `airsl`, when present, is a table whose only key is `version` (`airsl.<key>: unknown key`),
  required and a string; `semver::Version::parse` refuses a non-exact version (§1.1:
  `Version::parse("1.2")` is an error), reported as `airsl.version: "<value>" is not a version`;
- `tool`, `pack`, `file`, `merge` are each optional arrays of tables (generic wrong type at the
  array key, or at `<array>[i]` for an element); each element goes to Task 2 or Task 3 with key
  path `<array>[i]`;
- uniqueness: tool names, pack names, file paths, and merge (`file`, `key`) pairs. Each repeated
  value gives one diagnostic at the file alone (`DuplicateTool`, `DuplicatePack`,
  `DuplicateFile`, `DuplicateMerge`), however often it repeats. Tools and packs are separate
  namespaces, as are files and merges. The check uses every entry whose identifying field is
  well-formed, even when another field of that entry failed, so a broken entry still counts;
- `by`: the owner set is the names of every `[[tool]]` and `[[pack]]` entry whose `name` is a
  valid name (again even when another field failed, so one broken entry produces no follow-on
  `by` errors). A `by` not in the set is `UnknownOwner` at `file[i].by` / `merge[i].by`;
- the stored lists are sorted by byte order: tools and packs by name, files by path, merges by
  (`file`, `key`). Key paths in diagnostics keep the text's indices.

A missing lock is not an error: `parse` takes text, and a caller with no file has no lock to
parse (spec §8).

**Proof** (each refusal asserts the full `Diagnostic` `Display`; file `agentstacks.lock`):
- `lockfile_parses_the_spec_example` — the §8 example verbatim, except that each `<40 hex>` and
  `<64 hex>` placeholder is replaced by fixed lowercase hex of that length (the placeholders are
  not values a lock may hold; `<locator>` is a valid key and stays), parses with no diagnostics:
  `[airsl]` `0.1.4`, tool `claude`, packs `core` and `team-lint`, file
  `.claude/skills/discuss/SKILL.md` by `core`, merge `.claude/settings.json` `<locator>` by
  `core`.
- `lockfile_refuses_empty_text_field_by_field` — `""` gives exactly `agentstacks.lock: api:
  required`.
- `lockfile_reports_a_syntax_error_alone` — `api = 0` followed by the line `x = [1,` gives
  exactly one diagnostic, `agentstacks.lock:2: unclosed array, expected ']'` (the parser message
  is the spec §1.1 probe text).
- `lockfile_refuses_top_level_rules` — table-driven, each text otherwise valid, exactly one
  diagnostic each:
  - `api = 1` → `agentstacks.lock: api 1 not supported (this agentstacks speaks 0)`;
  - `api = "0"` → `agentstacks.lock: api: expected integer, got string`;
  - `api = 0.0` → `agentstacks.lock: api: expected integer, got float`;
  - `[airsl] version = "0.1"` → `agentstacks.lock: airsl.version: "0.1" is not a version`;
  - `[airsl]` with no `version` → `agentstacks.lock: airsl.version: required`;
  - `[airsl] version = 1979-05-27` → `agentstacks.lock: airsl.version: expected string, got
    datetime` (§1.1: a raw table holds `Datetime`; §10 wrong-type rule);
  - `airsl = "0.1.4"` → `agentstacks.lock: airsl: expected table, got string`;
  - `[airsl] channel = "x"` → `agentstacks.lock: airsl.channel: unknown key`;
  - `pack = "core"` → `agentstacks.lock: pack: expected array, got string`;
  - `pack = [1]` → `agentstacks.lock: pack[0]: expected table, got integer`;
  - `foo = 1` → `agentstacks.lock: foo: unknown key`.
- `lockfile_refuses_duplicates` — table-driven, exactly one diagnostic each: two `[[tool]]`
  `claude` → `agentstacks.lock: tool "claude" listed twice`; two `[[pack]]` `core` →
  `agentstacks.lock: pack "core" listed twice`; three `[[pack]]` `core` → that one diagnostic
  only; two `[[file]]` `.claude/x` → `agentstacks.lock: file ".claude/x" listed twice`; two
  `[[merge]]` with file `.claude/settings.json` and key `hooks` → `agentstacks.lock: merge
  ".claude/settings.json" "hooks" listed twice`; two `[[pack]]` `core` where the second has a bad
  `tree` → the duplicate diagnostic plus the `tree` diagnostic.
- `lockfile_accepts_distinct_namespaces` — a tool and a pack both named `claude`; two merges on
  one file with different keys; a `[[file]]` path equal to a `[[merge]]` file — each parses with
  no diagnostics.
- `lockfile_checks_by_against_tools_and_packs` — a `by` naming a locked tool parses; a `by`
  naming a pack whose entry has a bad `tree` gives only the `tree` diagnostic; `by = "kore"` in
  `merge[0]` gives `agentstacks.lock: merge[0].by: "kore" is not a locked pack or tool`.
- `lockfile_reports_every_error_in_one_run` — one text with `api = 1`, a `[[pack]]` without
  `rev`, and a `[[file]]` with `path = "../x"` gives exactly those three diagnostics, compared as
  a set of `Display` strings.
- `lockfile_sorts_parsed_entries` — packs written `team-lint`, `core`, files written `b`, `a`,
  merges written (`f`, `z`), (`f`, `a`) come back from the accessors as `core`, `team-lint`; `a`,
  `b`; (`f`, `a`), (`f`, `z`).
- `lockfile_new_applies_the_whole_file_rules` — `Lockfile::new` with entries in reverse order
  returns them sorted; with two packs `core` it refuses `agentstacks.lock: pack "core" listed
  twice`; with a file owned by `Owner` `kore` and no such entry it refuses
  `agentstacks.lock: file[0].by: "kore" is not a locked pack or tool`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): read agentstacks.lock`.

### Task 5 — Write the lockfile text

**Files:**
- Create `crates/agentstacks/src/lock/write_lock.rs` — `write_lock(&Lockfile) -> String`.
- Modify `crates/agentstacks/src/lock/mod.rs` — declare and re-export `write_lock`.

**Change:** the §8 writer rule, with the text formatted here and never through a serialised
`toml::Table` (§1.1: a raw table is sorted by key):

- line 1 `api = 0`, line 2 `# Generated by agentstacks. Do not edit.`, then one blank line;
- `[airsl]` with `version` when present, omitted otherwise;
- then every `[[tool]]`, `[[pack]]`, `[[file]]`, `[[merge]]` in the `Lockfile`'s stored (sorted)
  order, one blank line between tables, the text ending with exactly one newline;
- fields in the §8 order: tool and pack `name`, `version`, then `git`, `tag`, `subdir`, `rev` for
  a git source or `path` for a path source, then `tree`; file `path`, `hash`, `by`; merge `file`,
  `key`, `hash`, `by`. An absent `tag` or `subdir` is left out;
- within one table each key is padded with spaces to the longest key written in that table, then
  ` = ` (the §8 example: `name    = ` beside `version = `, `by   = ` beside `hash = `);
- every value is a TOML basic string (double-quoted), with `"`, `\` and control characters
  escaped.

**Proof:**
- `write_lock_reproduces_the_spec_example` — parsing the Task 4 example text (the §8 example
  verbatim, only the hex placeholders filled) and writing it gives that text byte for byte, plus
  one final newline.
- `write_lock_round_trips_through_parse` — writing, parsing the result and writing again gives
  identical text, and the parsed accessors return the written values; writing the same
  `Lockfile` twice gives identical text.
- `write_lock_starts_with_api_then_the_comment` — the first two lines are `api = 0` and
  `# Generated by agentstacks. Do not edit.`, with and without `[airsl]`; without `[airsl]` the
  text has no `[airsl]` line.
- `write_lock_orders_fields_and_omits_absent_ones` — a git entry without `tag` and `subdir` is
  written as `name`, `version`, `git`, `rev`, `tree`, padded to `version`; a path entry as
  `name`, `version`, `path`, `tree`.
- `write_lock_sorts_entries` — a `Lockfile::new` built with tools, packs, files and merges in
  reverse order is written with tools and packs by name, files by path and merges by (`file`,
  `key`).
- `write_lock_escapes_strings` — a merge key holding `"`, `\`, a tab and a newline is written as
  a valid basic string: parsing the text gives back the same key.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): write agentstacks.lock`.

### Task 6 — Check `[airsl]` presence against the locked packs

**Files:**
- Create `crates/agentstacks/src/lock/check_airsl_presence.rs` — `check_airsl_presence(file,
  lock, locked_packs) -> Result<(), Diagnostic>`, where `locked_packs` iterates the
  `PackManifest`s (plan 05) of the locked packs.
- Modify `crates/agentstacks/src/lock/mod.rs` — declare and re-export `check_airsl_presence`.

**Change:** the `[airsl]` presence row of spec §8. "Needed" means at least one manifest's
`Requires` holds an `airsl` (`Option<AirslReq>`, plan 05). Needed with no `[airsl]` is `AirslMissing`; `[airsl]` with
no need is `AirslUnneeded`; both are located at the file alone. Nothing in this chain calls it;
`core-plan-and-apply` does, with the locked packs (spec §13).

**Proof** (manifests built through plan 05's `PackManifest::parse` from a minimal `pack.toml`,
with and without `[requires] airsl = ">=0.1.4"`):
- `airsl_presence_refuses_a_missing_section` — a lock without `[airsl]` and one manifest with
  `requires.airsl` gives `agentstacks.lock: [airsl] missing, but locked packs require airsl`.
- `airsl_presence_refuses_an_unneeded_section` — a lock with `[airsl]` and manifests none of
  which declares `requires.airsl` (including no manifests at all) gives
  `agentstacks.lock: [airsl] present, but no locked pack requires airsl`.
- `airsl_presence_accepts_consistent_locks` — `[airsl]` with a needing manifest, and no
  `[airsl]` with no needing manifest, both return `Ok`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done
(`cargo make dod`) is green. Commit `feat(agentstacks): check airsl presence in the lock`.

---

## Risks

- Task 5 is the riskiest step: a writer bug produces a lock the parser refuses or reads back
  differently. The byte-identical, round-trip and escaping tests pin the format; the padding rule
  is read off the §8 example, so a change to that example changes the expected bytes.
- Lock sources depend on plan 08's `SourceFields::read` with `PinRule::RevRequired`. A change made
  there for `agentstacks.toml` reaches the lock; `locked_entry_refuses_each_rule` and plan 08's
  `rev_required_keeps_tag_optional` fail first.
- `[[file]].path` reuses plan 08's `RelPath::in_repository`, whose invariant equals §8's; only the
  message is reworded. Option not taken: a lock-only path newtype, which would duplicate that
  invariant (modularity rule 2).
- `[[merge]].file` gets its own `MergeFile` newtype (non-empty only, §8) rather than reusing
  `MergeKey`, so a file and a key cannot be swapped at a call site. Option not taken: applying the
  `[[file]].path` rule to it, which §8 does not state.

## Proof

- `cargo make dod-crate agentstacks` green, then `cargo make dod` green. No dependency changes, so
  `cargo make deny` is not required.
- Coverage map, one row per §8 rule in this plan's scope:

| Spec §8 row | Message (after `agentstacks.lock`) | Tests |
|---|---|---|
| `api` required, `0` | `: api 1 not supported (this agentstacks speaks 0)`, `: api: required` | `lockfile_refuses_top_level_rules`, `lockfile_refuses_empty_text_field_by_field` |
| `[airsl]` `version` exact semver | `: airsl.version: "0.1" is not a version` | `lockfile_refuses_top_level_rules` |
| `[airsl]` present iff needed | `: [airsl] missing, but locked packs require airsl` / `: [airsl] present, but no locked pack requires airsl` | `airsl_presence_*` |
| `[[tool]]`/`[[pack]]` required fields, git/path, `rev` required, `tag` optional, git-only fields, other §7 rules | `: pack[1].rev: required`, `: pack[1].tree: required`, `: pack[1]: needs exactly one of git, path`, `: pack[2]: tag, rev and subdir apply only to git sources`, … | `locked_entry_*`, `lockfile_reports_every_error_in_one_run` |
| `[[file]]` required, relative, no `..`, unique | `: file[0].path: "/x" must be relative`, `: file[0].path: "../x" must stay inside the project`, `: file ".claude/x" listed twice` | `file_entry_refuses_each_rule`, `lockfile_refuses_duplicates` |
| `[[merge]]` required, `key` non-empty, unique | `: merge[0].key: empty`, `: merge ".claude/settings.json" "hooks" listed twice` | `merge_key_refuses_empty_and_whitespace`, `merge_entry_refuses_each_rule`, `lockfile_refuses_duplicates` |
| entry `name` §4.1, `version` semver; `[[merge]].file` a non-empty string | `: pack[1].name: "Core" is not a valid name`, `: pack[1].version: "1.2" is not semver`, `: merge[0].file: empty` | `locked_entry_refuses_each_rule`, `merge_file_refuses_empty_and_whitespace`, `merge_entry_refuses_each_rule` |
| hashes `^sha256:[0-9a-f]{64}$` | `: file[0].hash: bad hash (expected sha256:<64 lowercase hex>)` (also `tree`, `merge[0].hash`) | `file_entry_refuses_each_rule`, `merge_entry_refuses_each_rule`, `locked_entry_refuses_each_rule` |
| `by` names a locked pack or tool | `: file[0].by: "kore" is not a locked pack or tool`, same for `merge[i].by` | `file_entry_refuses_each_rule`, `merge_entry_refuses_each_rule`, `lockfile_checks_by_against_tools_and_packs`, `lockfile_new_applies_the_whole_file_rules` |
| tool and pack names unique | `: pack "core" listed twice`, `: tool "claude" listed twice` | `lockfile_refuses_duplicates` |
| syntax error | `:2: unclosed array, expected ']'` | `lockfile_reports_a_syntax_error_alone` |
| unknown key | `: pack[1].branch: unknown key` | `locked_entry_refuses_each_rule`, `lockfile_refuses_top_level_rules` |
| written sorted, fields in §8 order, comment after `api = 0` | (writer rule) | `write_lock_*`, `lockfile_sorts_parsed_entries` |
| missing lock not an error; disagreeing lock stale, not malformed | (no refusal) | `parse` takes text only; `locked_entry_keeps_the_tag_optional` |

## Review findings

Built after the 2026-11-21 redesign. Crate-wide review rounds: review #1 (~70 findings) fixed in two waves; final review (0🔴 13🟡 17🔵 3❓). The 13🟡 and Q1–Q3 are fixed in the final fix wave (see the PR); the 17🔵 are listed in the PR as follow-ups.

## Deviations

- 2026-11-21 — **Redesign:** the spec was redesigned (`spec-superseded-2026-11-21.md` → `spec.md`) after plans 01–05 were built. This plan was built against the new spec, not the text above: code lives under `crates/agentstacks/src/types/<group>/` (domain types and logic, no I/O) with the read-only `types::port::FileTree` port and `fs::StdFileTree` as the only `std::fs` caller; loaders take `&impl FileTree`; file names never repeat their folder word. File paths and type names in the tasks above are superseded by the built tree.
- 2026-11-21 — Built in parallel with sibling plans by separate coders; consistency fixed afterwards by crate-wide reviews (one error collection type `Diagnostics`, `[i]` on element refusals, own error type per validating newtype, one api constant).
