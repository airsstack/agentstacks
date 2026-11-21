---
status: done
created: 2026-11-20
depends-on: [01, 02, 05]
built-on: 95010069150c
---

# Project File Implementation Plan

**Goal:** `agentstacks.toml` parses into a `ProjectFile` that enforces every rule decidable from
the file alone.

**Architecture:** The text is parsed to a raw `toml::Table` by plan 05's
`parse_toml_document(file, text)` (`toml_document.rs`) and validated by hand through plan 05's
`TomlFields` (`toml_fields.rs`); there is no `deny_unknown_fields` struct parse, so every error in
the file is reported (spec §10, §1.1). Wrong-type messages use the spec §10 type words that
`TomlFields` supplies (`boolean`, `integer`, `float`, `string`, `datetime`, `array`, `table`;
`null` is YAML only and never arises here). Source values are parse-on-construction newtypes, one
per file: `GitUrl` (no leading `-`), `GitTag` (non-empty), `CommitId` (40 or 64 lowercase hex) and
`RelPath` with two named constructors, `RelPath::in_repository` (subdir: no `..` segment, no
leading `/`) and `RelPath::in_project` (path: not absolute, and its `..` segments never climb above
the project directory). Spec §7 decides "resolving inside" for `path` lexically on the declared
string; symlinks are resolved by `core-plan-and-apply`, so `in_project` never touches the file
system.
`Source::{Git { url, pin, subdir }, Path(RelPath)}` with `Pin::{Tag(GitTag), Rev(CommitId)}` is
shared with the lock (plan 09). The tag/rev rule differs per file, so it is a parameter of the one
shared reader, not of `Source`: `SourceFields::read(entry, file, key_path, rule)` applies every
§7 source rule to one raw entry table in a single pass, and the `PinRule` argument selects the
pin check: `PinRule::ExactlyOneOfTagRev` (`agentstacks.toml`) or `PinRule::RevRequired`
(`agentstacks.lock`, where `tag` is optional). `Source::from_fields` then converts a read
`SourceFields` infallibly, preferring `rev` over `tag` (spec §8: "`rev` wins over a tag"); the lock
keeps its recorded tag through `SourceFields::tag()`. `ProjectFile { tools, packs }` maps
`ToolName` to `Source` and `PackName` to `PackEntry { source, options }`, where `options` is the
raw `toml::Table` (typing it against the pack's declarations is plan 10). `SourceError` and
`ProjectError` join `FormatError`. The §7 rows that need fetched content (key = fetched name,
`requires.packs` listed, cycles, option typing) are plan 10's.

**Tech Stack:** Rust 2024, `toml` 1 (added by plan 05), `thiserror` 2 (added by plan 01). No new
dependencies.

**Content authority:** spec §7 (file-local rows: `api`; nothing to install; table keys §4.1;
exactly one of `git`/`path`; exactly one of `tag`/`rev`; `rev` shape and no `branch`; `-` URL;
`subdir`; `tag`/`rev`/`subdir` only on git, with an entry holding both `git` and `path` getting
only the `git, path` refusal plus malformed field values; `path` "resolving inside" decided
lexically; `path`; `options` placement; empty strings; unknown keys), §8 row
`[[tool]]`/`[[pack]]` (the `rev`-required pin rule this plan's reader provides to plan 09), §10
(locations, generic shapes, type words, all errors, raw-table parse, syntax line from span, empty
TOML refused field by field, datetime gets the wrong-type message), §11 (`source.rs`,
`project.rs`), §1.1 (toml parser facts).

---

## Files that change

```
crates/agentstacks/src/model/mod.rs           — modify  declare and re-export the new items
crates/agentstacks/src/model/git_url.rs       — create  GitUrl: non-empty, no leading "-"
crates/agentstacks/src/model/git_tag.rs       — create  GitTag: non-empty, not whitespace-only
crates/agentstacks/src/model/commit_id.rs     — create  CommitId: 40 or 64 lowercase hex
crates/agentstacks/src/model/rel_path.rs      — create  RelPath with in_repository and in_project constructors
crates/agentstacks/src/model/pin.rs           — create  Pin::{Tag(GitTag), Rev(CommitId)}
crates/agentstacks/src/model/pin_rule.rs      — create  PinRule::{ExactlyOneOfTagRev, RevRequired}
crates/agentstacks/src/model/source_error.rs  — create  SourceError (thiserror)
crates/agentstacks/src/model/source_fields.rs — create  SourceFields::read: every source rule over one raw entry
crates/agentstacks/src/model/source.rs        — create  Source::{Git, Path} and Source::from_fields
crates/agentstacks/src/model/project_error.rs — create  ProjectError (thiserror)
crates/agentstacks/src/model/pack_entry.rs    — create  PackEntry { source, options (raw table) }
crates/agentstacks/src/model/project.rs       — create  ProjectFile and ProjectFile::parse
crates/agentstacks/src/model/format_error.rs  — modify  FormatError gains Source and Project variants
```

## Order of work

### Task 1 — Add source value types

**Files:**
- Create `crates/agentstacks/src/model/git_url.rs` — `GitUrl`.
- Create `crates/agentstacks/src/model/git_tag.rs` — `GitTag`.
- Create `crates/agentstacks/src/model/commit_id.rs` — `CommitId`.
- Create `crates/agentstacks/src/model/rel_path.rs` — `RelPath`.
- Create `crates/agentstacks/src/model/pin.rs` — `Pin`.
- Create `crates/agentstacks/src/model/source_error.rs` — `SourceError`.
- Modify `crates/agentstacks/src/model/format_error.rs` — add the `Source(SourceError)` variant.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export the six items.

**Change:** Each newtype is a private tuple struct with a fallible constructor (documented
`# Errors`), an `as_str` accessor, `Display` and `FromStr` delegating to the constructor.
`GitUrl::new` refuses an empty or whitespace-only string and a string starting with `-`.
`GitTag::new` refuses an empty or whitespace-only string. `CommitId::new` refuses an empty or
whitespace-only string, and anything that is not exactly 40 or 64 characters of `[0-9a-f]`.
`RelPath::in_repository` refuses an empty or whitespace-only string, a leading `/` and any `..`
segment. `RelPath::in_project` refuses an empty or whitespace-only string and a leading `/`
(absolute), then walks the `/`-separated segments of the declared string keeping a depth count
(`..` decrements, `.` and empty segments are skipped, others increment) and refuses when the depth
goes below zero. This is the lexical decision of spec §7; it never reads the file system.
`Pin` holds data only (no `impl` with logic), so it carries no test. `SourceError` variants and
their `Display` text, copied from spec §7: `LeadingDash` → `URL must not start with "-"`;
`NotACommitId` → `not a commit id`; `OutsideRepository` → `must stay inside the repository`;
`AbsolutePath` → `absolute paths are not portable`; `OutsideProject` → `must stay inside the
project`; `GitOrPath` → `needs exactly one of git, path`; `TagOrRev` → `needs exactly one of tag,
rev`; `GitOnlyFields` → `tag, rev and subdir apply only to git sources`; and `Field(FieldError)`
(transparent, plan 01) so an empty string reads `empty` (spec §10 generic shape) without a second
"empty" concept. The location is never part of the message; the caller's `Location` supplies it.

**Proof:**
- `git_url_accepts_the_spec_url` (git_url.rs) — `https://github.com/airsstack/agentstacks`
  constructs and `as_str` returns it unchanged.
- `git_url_refuses_a_leading_dash_or_empty` (git_url.rs) — `-x` and `--upload-pack=x` are
  refused with `Display` `URL must not start with "-"`; `""` and `" \t"` are refused with
  `empty`.
- `git_tag_accepts_a_tag` (git_tag.rs) — `packs-v0.1.0` constructs and round-trips through
  `as_str`.
- `git_tag_refuses_empty_or_whitespace` (git_tag.rs) — `""` and `"  "` are refused with `empty`.
- `commit_id_accepts_40_and_64_lowercase_hex` (commit_id.rs) — a 40-character and a 64-character
  lowercase hex string construct.
- `commit_id_refuses_other_forms` (commit_id.rs) — 39 hex, 41 hex, 40 uppercase hex, 40 characters
  containing `g`, and `main` are each refused with `not a commit id`; `""` with `empty`.
- `in_repository_accepts_relative_paths` (rel_path.rs) — `packs/core`, `extensions/claude` and
  `a..b/c` (`..` only counts as a whole segment) construct.
- `in_repository_refuses_escape_and_leading_slash` (rel_path.rs) — `../x`, `a/../b` and `/x` are
  each refused with `must stay inside the repository`; `""` with `empty`.
- `in_project_accepts_paths_that_stay_inside` (rel_path.rs) — `packs/team-lint` and `a/../b`
  construct.
- `in_project_refuses_absolute_and_climbing_paths` (rel_path.rs) — `/abs` is refused with
  `absolute paths are not portable`; `../x` and `a/../../x` with `must stay inside the project`;
  `""` with `empty`.
- `source_error_messages_match_the_spec` (source_error.rs) — every variant's `Display` equals the
  §7 text listed under Change, and `Field(FieldError::Empty)` displays `empty`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add source value types`.

### Task 2 — Read sources with a per-file pin rule

**Files:**
- Create `crates/agentstacks/src/model/pin_rule.rs` — `PinRule`.
- Create `crates/agentstacks/src/model/source_fields.rs` — `SourceFields` and `SourceFields::read`.
- Create `crates/agentstacks/src/model/source.rs` — `Source` and `Source::from_fields`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export the three items.

**Change:** `PinRule::{ExactlyOneOfTagRev, RevRequired}` holds data only. `SourceFields` is
`Git { url: GitUrl, tag: Option<GitTag>, rev: Option<CommitId>, subdir: Option<RelPath> }` or
`Path(RelPath)`, with a `tag()` accessor. `SourceFields::read` takes one raw entry table, the
file, the entry's key path (`packs.core`, `pack[1]`) and a `PinRule`, reads only the keys `git`,
`path`, `tag`, `rev`, `subdir` through `toml_fields.rs` (the caller checks unknown keys against
its own key set), and returns the `SourceFields` or every `Diagnostic` found, all in one pass:

- each present key must be a string (generic wrong type, located at `<entry>.<key>`) and is
  constructed through its Task 1 newtype (`git` → `GitUrl`, `tag` → `GitTag`, `rev` →
  `CommitId`, `subdir` → `RelPath::in_repository`, `path` → `RelPath::in_project`), each refusal
  located at `<entry>.<key>`;
- both `git` and `path` → one `GitOrPath` diagnostic at `<entry>` and, per spec §7, no
  `GitOnlyFields` or pin diagnostic; malformed field values in that entry are still reported;
- neither `git` nor `path` → one `GitOrPath` diagnostic at `<entry>`; the entry is neither a path
  entry nor a git entry, so the two checks below do not apply to it;
- a `path` entry with any of `tag`, `rev`, `subdir` → one `GitOnlyFields` diagnostic at `<entry>`;
- a `git` entry under `ExactlyOneOfTagRev` with neither or both of `tag` and `rev` → one
  `TagOrRev` diagnostic at `<entry>`; under `RevRequired` without `rev` → `FieldError::Required`
  at `<entry>.rev`, with `tag` optional.

Pin checks look at key presence, so they are reported even when a value in the same entry is
malformed. `Source::from_fields` is infallible: `Path` maps to `Source::Path`; `Git` maps to
`Source::Git` with `Pin::Rev` when `rev` is present, else `Pin::Tag`, and the subdir carried over.

**Proof:**
- `source_fields_reads_git_and_path_forms` (source_fields.rs) — under `ExactlyOneOfTagRev`, the §7
  `core` entry (git, tag, subdir), a git entry with only `rev`, and the `team-lint` entry (path)
  each read to the matching `SourceFields` with no diagnostics.
- `source_fields_refuses_git_and_path_together_or_neither` (source_fields.rs) — an entry with both
  `git` and `path`, and an entry with neither, each give exactly one diagnostic whose `Display` is
  `agentstacks.toml: packs.core: needs exactly one of git, path`.
- `source_fields_both_git_and_path_gets_only_that_refusal` (source_fields.rs) — under
  `ExactlyOneOfTagRev`, an entry with `git`, `path` and `subdir` (no `tag`, no `rev`) gives exactly
  `agentstacks.toml: packs.core: needs exactly one of git, path`; the same entry with
  `rev = "main"` added gives exactly that diagnostic and
  `agentstacks.toml: packs.core.rev: not a commit id` (spec §7: only the `git, path` refusal, plus
  malformed field values).
- `source_fields_refuses_git_only_fields_on_a_path_source` (source_fields.rs) — a `path` entry
  with `tag`, one with `rev`, one with `subdir`, and one with all three each give exactly one
  diagnostic `agentstacks.toml: packs.team-lint: tag, rev and subdir apply only to git sources`.
- `source_fields_locates_field_errors_at_their_key_path` (source_fields.rs) — table-driven, one
  diagnostic each: `git = "-x"` → `agentstacks.toml: packs.core.git: URL must not start with
  "-"`; `rev = "main"` → `agentstacks.toml: packs.core.rev: not a commit id`; `subdir = "../x"` →
  `agentstacks.toml: packs.core.subdir: must stay inside the repository`; `path = "/abs"` →
  `agentstacks.toml: packs.team-lint.path: absolute paths are not portable`; `path = "../x"` →
  `agentstacks.toml: packs.team-lint.path: must stay inside the project`; `tag = ""` →
  `agentstacks.toml: packs.core.tag: empty`; `git = " "` → `agentstacks.toml: packs.core.git:
  empty`; `tag = 5` → `agentstacks.toml: packs.core.tag: expected string, got integer`.
- `exactly_one_of_tag_rev_refuses_neither_or_both` (source_fields.rs) — under
  `ExactlyOneOfTagRev`, a git entry with neither `tag` nor `rev`, and one with both, each give
  exactly one diagnostic `agentstacks.toml: packs.core: needs exactly one of tag, rev`.
- `rev_required_keeps_tag_optional` (source_fields.rs) — under `RevRequired` with file
  `agentstacks.lock` and key path `pack[1]`: `rev` alone and `tag` plus `rev` read cleanly, and
  `tag()` returns the tag in the second case; `tag` alone and neither each give exactly one
  diagnostic `agentstacks.lock: pack[1].rev: required` (spec §8 row text).
- `source_fields_reports_every_error_in_one_entry` (source_fields.rs) — an entry with
  `git = "-x"`, `tag = "v1"` and `rev = "main"` under `ExactlyOneOfTagRev` gives exactly the three
  diagnostics for the URL, the commit id and `needs exactly one of tag, rev`, compared as a set
  of `Display` strings.
- `source_from_fields_pins_a_tag_or_a_rev` (source.rs) — tag-only git fields become `Source::Git`
  with `Pin::Tag` and the subdir kept; rev-only become `Pin::Rev`; path fields become
  `Source::Path` with the same `RelPath`.
- `source_from_fields_prefers_rev_over_tag` (source.rs) — git fields read under `RevRequired`
  with both `tag` and `rev` become `Pin::Rev` holding that commit id.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): read pack and extension sources`.

### Task 3 — Parse `agentstacks.toml`

**Files:**
- Create `crates/agentstacks/src/model/project_error.rs` — `ProjectError`.
- Create `crates/agentstacks/src/model/pack_entry.rs` — `PackEntry`.
- Create `crates/agentstacks/src/model/project.rs` — `ProjectFile` and `ProjectFile::parse`.
- Modify `crates/agentstacks/src/model/format_error.rs` — add the `Project(ProjectError)` variant.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export the three items.

**Change:** `ProjectError` variants and their `Display` text, copied from spec §7:
`UnsupportedApi { found }` → `api <found> not supported (this agentstacks speaks 0)`;
`NothingToInstall` → `nothing to install`; `InvalidName` → `not a valid name` (a refusal from
plan 02's `ToolName::new` / `PackName::new`, whose own `InvalidName` error displays
`"<value>" is not a valid name`, is mapped to this variant, because spec §7 drops the quoted
value). `PackEntry`
holds a `Source` and the raw `options` table, with accessors only (no logic, no test).
`ProjectFile::parse` takes the path used in locations and the text, and returns the
`ProjectFile` or every `Diagnostic`:

- the text goes through `parse_toml_document`; a syntax error is the file's only diagnostic,
  located `<file>:<line>` (§10);
- top-level keys are `api`, `tools`, `packs`; any other is `<key>: unknown key`;
- `api` is required and an integer (generic shapes, at `api`); a value other than 0 is
  `UnsupportedApi`, located at the file with no key path, matching the §7 text;
- `tools` and `packs` must be tables of tables (generic wrong type at `tools`, `tools.<key>`,
  `packs`, `packs.<key>`); when either is absent or has no entries, one `NothingToInstall`
  diagnostic located at the file alone; when either has the wrong type, only the wrong-type
  diagnostic;
- every entry key must parse as `ToolName` / `PackName` (plan 02), else `InvalidName` at
  `tools.<key>` / `packs.<key>`; the entry's fields are still checked so every error is reported;
- each entry is read with `SourceFields::read(…, PinRule::ExactlyOneOfTagRev)` and converted with
  `Source::from_fields`;
- a tool entry allows the keys `git`, `path`, `tag`, `rev`, `subdir`; a pack entry allows those
  plus `options`; any other key (including `branch`, and `options` under a tool) is
  `<entry>.<key>: unknown key`;
- `options`, when present, must be a table (generic wrong type at `packs.<key>.options`); its
  values are kept raw for plan 10.

Entries iterate in name order because a raw `toml::Table` is sorted by key (spec §1.1).

**Proof:**
- `project_error_messages_match_the_spec` (project_error.rs) — `UnsupportedApi { found: 1 }`
  displays `api 1 not supported (this agentstacks speaks 0)`, `NothingToInstall` displays
  `nothing to install`, `InvalidName` displays `not a valid name`.
- `project_file_parses_the_spec_example` (project.rs) — the §7 example parses with no
  diagnostics: tool `claude` is a git source pinned by tag `packs-v0.1.0` with subdir
  `extensions/claude`; pack `core` is a git source pinned by tag with subdir `packs/core` and raw
  option `archive_keep = 50`; pack `team-lint` is a path source `packs/team-lint` with no options.
- `project_file_refuses_a_syntax_error_with_its_line` (project.rs) — `api = 0`, a blank line and
  `x = [1,` give exactly one diagnostic, `agentstacks.toml:3: unclosed array, expected ']'` (the
  parser message is the spec §1.1 probe text).
- `project_file_refuses_api_rules` (project.rs) — table-driven, each file otherwise valid:
  `api = 1` → `agentstacks.toml: api 1 not supported (this agentstacks speaks 0)`; no `api` →
  `agentstacks.toml: api: required`; `api = "0"` → `agentstacks.toml: api: expected integer, got
  string`; `api = 1979-05-27` → `agentstacks.toml: api: expected integer, got datetime`.
- `project_file_refuses_nothing_to_install` (project.rs) — table-driven, each case containing
  exactly the diagnostic `agentstacks.toml: nothing to install`: tools but no `packs`; packs but
  no `tools`; empty `[tools]` and `[packs]` tables. Empty text gives exactly
  `agentstacks.toml: api: required` and `agentstacks.toml: nothing to install` (§10: empty TOML
  is refused field by field).
- `project_file_refuses_invalid_table_keys` (project.rs) — `[packs.Core]` gives
  `agentstacks.toml: packs.Core: not a valid name`; `[tools.Claude]` gives
  `agentstacks.toml: tools.Claude: not a valid name`.
- `project_file_refuses_unknown_keys` (project.rs) — `branch = "main"` in `[packs.core]` gives
  `agentstacks.toml: packs.core.branch: unknown key`; `options = { x = 1 }` in `[tools.claude]`
  gives `agentstacks.toml: tools.claude.options: unknown key`; top-level `foo = 1` gives
  `agentstacks.toml: foo: unknown key`.
- `project_file_refuses_wrong_typed_tables` (project.rs) — `tools.claude = "x"` gives
  `agentstacks.toml: tools.claude: expected table, got string`; `options = 5` in `[packs.core]`
  gives `agentstacks.toml: packs.core.options: expected table, got integer`.
- `project_file_applies_source_rules_to_every_entry` (project.rs) — table-driven through whole
  files, one diagnostic each: `[packs.core]` with both `git` and `path` →
  `agentstacks.toml: packs.core: needs exactly one of git, path`; with `git` and both `tag` and
  `rev` → `agentstacks.toml: packs.core: needs exactly one of tag, rev`; with `rev = "main"` →
  `agentstacks.toml: packs.core.rev: not a commit id`; `[packs.team-lint]` with `path` and
  `tag` → `agentstacks.toml: packs.team-lint: tag, rev and subdir apply only to git sources`;
  with `path = "/abs"` → `agentstacks.toml: packs.team-lint.path: absolute paths are not
  portable`; `[tools.claude]` with `git = "-x"` → `agentstacks.toml: tools.claude.git: URL must
  not start with "-"`; with `subdir = "/x"` → `agentstacks.toml: tools.claude.subdir: must stay
  inside the repository`.
- `project_file_reports_every_error` (project.rs) — one file with `api = 1`, `[packs.Core]` (a
  valid git source), `branch = "main"` in a valid `[packs.core]`, and `options` under
  `[tools.claude]` gives exactly those four diagnostics, compared as a set of `Display` strings.
- `project_file_keeps_option_values_raw` (project.rs) — `options = { archive_keep = "many",
  ratio = 1.5 }` in `[packs.core]` parses with no diagnostics and both values are kept as given
  (typing them is plan 10, spec §4.4).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done
(`cargo make dod`) is green. Commit `feat(agentstacks): parse agentstacks.toml`.

---

## Risks

- Task 2 is the riskiest step: plan 09 reads lock sources through `SourceFields::read` with
  `PinRule::RevRequired`. A change to the reader for `agentstacks.toml` reaches the lock too; the
  `rev_required_keeps_tag_optional` test pins the lock behaviour here so such a change fails in
  this file first.
- Option values are not typed here: §7 checks `options` "per §4.4", which needs the pack's
  declarations, so `ratio = 1.5` passes this file and is refused by plan 10 (spec §9 item 3).
  Option not taken: a file-level type check, which would report the same value twice with two
  messages.

## Proof

- `cargo make dod-crate agentstacks` green, then `cargo make dod` green.
- Every file-local §7 row has a case asserting the full `Diagnostic` `Display`: `api`
  (`project_file_refuses_api_rules`), nothing to install (`project_file_refuses_nothing_to_install`),
  table keys (`project_file_refuses_invalid_table_keys`), `git`/`path`, `tag`/`rev`, `rev` shape,
  `-` URL, `subdir`, git-only fields, both `git` and `path` getting only that refusal
  (`source_fields_both_git_and_path_gets_only_that_refusal`), `path` resolving inside, decided
  lexically (`in_project_*`), `path` (`source_fields_*`, `exactly_one_of_tag_rev_*`,
  `project_file_applies_source_rules_to_every_entry`), `options` placement and unknown keys
  including `branch` (`project_file_refuses_unknown_keys`), empty strings
  (`source_fields_locates_field_errors_at_their_key_path`), syntax line
  (`project_file_refuses_a_syntax_error_with_its_line`).
- The remaining §7 rows (key = fetched name, `requires.packs` listed, cycles, option typing) are
  covered by plan 10.

## Review findings

Built after the 2026-11-21 redesign. Crate-wide review rounds: review #1 (~70 findings) fixed in two waves; final review (0🔴 13🟡 17🔵 3❓). The 13🟡 and Q1–Q3 are fixed in the final fix wave (see the PR); the 17🔵 are listed in the PR as follow-ups.

## Deviations

- 2026-11-21 — **Redesign:** the spec was redesigned (`spec-superseded-2026-11-21.md` → `spec.md`) after plans 01–05 were built. This plan was built against the new spec, not the text above: code lives under `crates/agentstacks/src/types/<group>/` (domain types and logic, no I/O) with the read-only `types::port::FileTree` port and `fs::StdFileTree` as the only `std::fs` caller; loaders take `&impl FileTree`; file names never repeat their folder word. File paths and type names in the tasks above are superseded by the built tree.
- 2026-11-21 — Built in parallel with sibling plans by separate coders; consistency fixed afterwards by crate-wide reviews (one error collection type `Diagnostics`, `[i]` on element refusals, own error type per validating newtype, one api constant).
