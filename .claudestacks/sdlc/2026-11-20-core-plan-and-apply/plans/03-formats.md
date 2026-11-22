---
status: done
created: 2026-11-22
depends-on: [01]
built-on: f39b2a814319
---

# Formats Implementation Plan

**Goal:** Amend the built formats to the api 0 changes of spec §12.

**Architecture:** Five independent edits to formats already built in `src/types`, one task each.
Each refusal is a variant of the format's own error enum, displayed after the built `Location`, so
the new messages read like the old ones of the same file:

- `extension.reads` is read by `ExtensionManifest` (`src/types/extension/manifest.rs`) with the
  lock's `[[file]].path` rule (formats §8, built in `read_path`,
  `src/types/lock/entry/written.rs:243`): an absolute entry is `MustBeRelative`, a `..` segment is
  `OutsideProject`, a blank one is `empty`, and a `\` is refused as in §16 R16 (R28(n)); naming
  agentstacks' own files is allowed there (R28(n)). Entries are kept as `SubPath`
  (`src/types/project/path/sub.rs`), whose `normalized()` is the spec §3.3 normalisation, and a
  repeat after normalising is the shared `ListedTwice` phrase, quoting the repeated entry as written
  (§16 R24).
- The lock's `[[merge]]` goes: `MergeEntry`, `MergeFile`, `MergeKey`, `EntryKey::Merge` and the
  `merge` top-level key are deleted, so `merge` falls to the built unknown-key refusal.
- The lock's `[[file]].path` rule, when a lock is read, also refuses a `\` and agentstacks' own two
  files (§16 R16, R28(e)).
- `Policy` (`src/types/hooks/policy.rs`) keeps only `Confined`; `pure` gets its own refusal and the
  unknown-value refusal names `confined` alone.
- `HookRun::read` (`src/types/hooks/run.rs`), which both `pack.toml` hooks and
  `native/<tool>/hooks.toml` hooks use (`src/types/hooks/native/hook.rs` module doc), refuses the
  first path segment that `require` cannot name (spec §11.1, §16 R24).

Guideline rules applied: `modularity.md` Rule 2 (no new phrase type: the built `MustBeRelative`,
`OutsideProject`, `ListedTwice<Quoted>` and `FieldError` are reused; the lock's dead merge types
are deleted rather than kept beside the new design); `mod-rs-export-only.md` (`lock/mod.rs`,
`lock/entry/mod.rs`, `hooks/mod.rs` and `types/mod.rs` change only `mod` / `pub use` lines and doc
bullets); `unit-test-mandate.md` (every changed rule gets its colocated test); `strong-types.md`
(`reads` is `Vec<SubPath>`, not `Vec<String>`); `doc-comment-discipline.md` (docs describe the new
behaviour; no issue number in a doc comment; the issue reference lives only in the user-facing
message the spec fixes). No doc-comment example uses a fake (fakes are `#[cfg(test)]`, plan-set
rule); `FakeFileTree` appears only in unit tests.

**Tech Stack:** Rust 2024, crate `agentstacks`, `toml 1`, `thiserror 2`.

**Content authority:** spec §12 rows 1-4, D14, §11.1 (why a segment must be require-able), §3.3
"Normalisation" (for `reads`), §16 R16, R24, R28(e), R28(n).

---

## Files that change

```
crates/agentstacks/src/types/extension/manifest.rs      — modify: read `reads`; accessor; docs; tests
crates/agentstacks/src/types/extension/error.rs         — modify: variants MustBeRelative, OutsideProject, ListedTwice, Separator
crates/agentstacks/src/types/extension/loaded.rs        — modify: `Extension::reads` forwarding accessor; test
crates/agentstacks/tests/extension.rs                   — modify: a fixture with reads loads; one invalid reads fixture
crates/agentstacks/tests/fixtures/extensions/valid/reads/extension.toml        — create
crates/agentstacks/tests/fixtures/extensions/valid/reads/adapter.lua           — create (`return {}`)
crates/agentstacks/tests/fixtures/extensions/invalid/reads-absolute/extension.toml — create
crates/agentstacks/tests/fixtures/extensions/invalid/reads-absolute/adapter.lua    — create (`return {}`)
crates/agentstacks/src/types/lock/merge.rs              — delete
crates/agentstacks/src/types/lock/entry/written.rs      — modify: drop MergeEntry, ReadMerge, MERGE_KEYS and their tests; R16 checks in read_path; docs
crates/agentstacks/src/types/lock/entry/key.rs          — modify: drop EntryKey::Merge; test case; docs
crates/agentstacks/src/types/lock/entry/locked.rs       — modify: module doc no longer links MergeEntry
crates/agentstacks/src/types/lock/entry/mod.rs          — modify: drop the MergeEntry re-export; docs
crates/agentstacks/src/types/lock/file.rs               — modify: drop merges everywhere; TOP_LEVEL_KEYS loses "merge"; tests
crates/agentstacks/src/types/lock/write.rs              — modify: drop the merge table; tests
crates/agentstacks/src/types/lock/error.rs              — modify: drop the merge test case; variants Separator, Managed; docs
crates/agentstacks/src/types/lock/owner.rs              — modify: docs say "file", not "file or merge"
crates/agentstacks/src/types/lock/testing.rs            — modify: the example lock loses its [[merge]] table
crates/agentstacks/src/types/lock/mod.rs                — modify: drop `pub mod merge` and three re-exports; docs
crates/agentstacks/src/types/mod.rs                     — modify: drop `pub use lock::{MergeEntry, MergeFile, MergeKey}` (three lines)
crates/agentstacks/src/types/hooks/policy.rs            — modify: Policy::Pure removed; `pure` refused; tests; doctest
crates/agentstacks/src/types/hooks/invalid.rs           — modify: InvalidPolicy::PureUnsupported; Unknown text; test
crates/agentstacks/src/types/hooks/grants.rs            — modify: module doc and doctest drop `pure`; test cases
crates/agentstacks/src/types/hooks/mod.rs               — modify: the Policy doc bullet
crates/agentstacks/src/types/hooks/run.rs               — modify: segment rule in HookRun::read; tests
crates/agentstacks/src/types/hooks/native/hook.rs       — modify: two cases in native_hook_keeps_the_neutral_rules
crates/agentstacks/src/types/manifest/error.rs          — modify: ManifestError::RunSegment; test case table updated
```

## Order of work

### Task 1 — Read `extension.reads`

**Files:**
- Modify `crates/agentstacks/src/types/extension/manifest.rs` — `ExtensionManifest` gets a
  `reads: Vec<SubPath>` field and `pub fn reads(&self) -> &[SubPath]`; `read_section` reads `reads`;
  `"reads"` joins the allowed keys of `[extension]`; module doc lists `reads`; tests.
- Modify `crates/agentstacks/src/types/extension/error.rs` — `ExtensionError` gains
  `MustBeRelative(#[from] MustBeRelative)`, `OutsideProject(#[from] OutsideProject)`,
  `ListedTwice(#[from] ListedTwice<Quoted>)` (all transparent, the phrase module's naming rule) and
  `Separator { text }` displaying `"<text>" must use / as the separator`; module doc; the display
  test gets one case each. A blank entry uses the built `FieldError::Empty`
  (`empty`), not a new variant.
- Modify `crates/agentstacks/src/types/extension/loaded.rs` — `pub fn reads(&self) -> &[SubPath]`
  forwarding to the manifest, so the plan and host code that holds an `Extension` reads it in one
  call; test.
- Create the two fixture directories listed above. `valid/reads/extension.toml` is the `minimal`
  fixture's manifest plus `reads = [".claude/settings.json", "./opencode.json"]`;
  `invalid/reads-absolute/extension.toml` is the `minimal` manifest plus `reads = ["/x"]`.
- Modify `crates/agentstacks/tests/extension.rs` — one new test and one new row (below).

**Change:** `reads` is an optional list of project paths (spec §12 row 1); absent means empty, an
empty array is accepted. It is read with the built `optional_string_list`
(`src/types/format/fields.rs:174`), which already words a non-array value and a non-string item.
Each string item, located at `extension.reads[<i>]` (0-based TOML index, as every built list), is
checked in the order of the lock path rule's existing checks (`read_path`,
`src/types/lock/entry/written.rs:243-260`): a leading `/` → `MustBeRelative`; then `SubPath::new` —
`InvalidSubPath::OutsideRepository` → `OutsideProject`, `InvalidSubPath::Empty` →
`FieldError::Empty`; then a `\` → `Separator`, the R16 check Task 3 adds to the lock (§16 R28(n)).
The own-file check of Task 3 does not apply: `agentstacks.toml` and `agentstacks.lock` are allowed in
`reads` (R28(n)). A valid entry whose `normalized()` equals an earlier valid entry's is refused
with `ListedTwice` quoting the repeated entry as written, at the repeated index (spec §12 row 1
"duplicates refused after normalising"; §16 R24 "A repeated `extension.reads` entry is quoted as
written"). Every refusal is collected with the other field refusals, as today (`parse` reports all
at once). The manifest keeps the entries as written; callers compare `normalized()` (spec §3.3
"Normalisation").

**Proof:** written first, seen failing, then passing. In `manifest.rs` (each refusal test asserts
the full sorted diagnostics, as the existing tests do):
- `extension_manifest_reads_default_to_empty` — the documented example (no `reads`) parses with
  `reads()` empty; `reads = []` parses with `reads()` empty.
- `extension_manifest_parses_reads` — `reads = [".claude/settings.json", "./opencode.json"]` parses;
  `reads()` holds both as written, and their `normalized()` forms are `.claude/settings.json` and
  `opencode.json`.
- `extension_manifest_refuses_reads_that_is_not_an_array` — `reads = "x"`:
  `extension.toml: extension.reads: expected array, got string`.
- `extension_manifest_refuses_a_reads_item_that_is_not_a_string` — `reads = [1]`:
  `extension.toml: extension.reads[0]: expected string, got integer`.
- `extension_manifest_refuses_an_absolute_reads_entry` — `reads = ["/x"]`:
  `extension.toml: extension.reads[0]: "/x" must be relative`.
- `extension_manifest_refuses_a_reads_entry_outside_the_project` — `reads = ["../x"]`:
  `extension.toml: extension.reads[0]: "../x" must stay inside the project`.
- `extension_manifest_refuses_a_blank_reads_entry` — `reads = [""]` and `reads = ["  "]`:
  `extension.toml: extension.reads[0]: empty` (the lock rule's blank refusal,
  `agentstacks.lock: file[0].path: empty`, `src/types/lock/entry/written.rs:391`).
- `extension_manifest_refuses_a_reads_entry_with_a_backslash` — `reads = ['a\b']` (one backslash):
  `extension.toml: extension.reads[0]: "a\b" must use / as the separator` (§16 R28(n)).
- `extension_manifest_allows_agentstacks_own_files_in_reads` —
  `reads = ["agentstacks.toml", "agentstacks.lock"]` parses with both entries kept (§16 R28(n)).
- `extension_manifest_refuses_a_reads_entry_listed_twice` —
  `reads = [".claude/settings.json", ".claude/settings.json"]`:
  `extension.toml: extension.reads[1]: ".claude/settings.json" listed twice`.
- `extension_manifest_refuses_one_reads_path_written_two_ways` — `reads = [".claude/x", "./.claude/x"]`:
  `extension.toml: extension.reads[1]: "./.claude/x" listed twice`; `reads = ["./a", "a"]`:
  `extension.toml: extension.reads[1]: "a" listed twice`; `reads = ["a//b", "a/b"]`:
  `extension.toml: extension.reads[1]: "a/b" listed twice` (§16 R24).
- `extension_manifest_refuses_unknown_keys` (existing) still passes unchanged: `foo` stays an
  unknown key, and `reads` no longer is.
- `extension_error_displays_each_message` (existing, `error.rs`) gains the four new variants:
  `"/x" must be relative`, `"../x" must stay inside the project`,
  `".claude/settings.json" listed twice`, `"a\b" must use / as the separator`.

In `loaded.rs`:
- `extension_reads_are_the_manifests` (unit test) — an extension loaded from a `FakeFileTree` whose
  manifest has `reads = [".claude/settings.json"]` returns the same slice from `Extension::reads()`
  and `manifest().reads()`.

In `tests/extension.rs`:
- `extension_fixture_with_reads_loads` — `valid/reads` loads; `reads()` normalised is
  `[".claude/settings.json", "opencode.json"]`.
- `each_invalid_extension_fixture_reports_its_refusal` gains the row
  `("reads-absolute", false, ": extension.reads[0]: \"/x\" must be relative")`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): extension.reads`.

### Task 2 — Remove the lock's `[[merge]]`

**Files:**
- Delete `crates/agentstacks/src/types/lock/merge.rs`.
- Modify `crates/agentstacks/src/types/lock/entry/written.rs` — remove `MERGE_KEYS`, `MergeEntry`
  (struct, `new`, accessors, `read`), `ReadMerge`, the merge imports, and the tests
  `merge_entry_parses_a_merge_entry`, `merge_entry_refuses_each_rule`,
  `merge_entry_new_keeps_its_parts` with their helpers `read_merge` and `merge_text`; the module doc
  describes files only.
- Modify `crates/agentstacks/src/types/lock/entry/key.rs` — remove `EntryKey::Merge`, its `Display`
  arm, the merge import and the `Merge` case of
  `entry_key_shows_the_array_word_then_each_value_quoted`; doc names three arrays.
- Modify `crates/agentstacks/src/types/lock/entry/locked.rs` — the module doc's second sentence links
  only `FileEntry` (today it also links `MergeEntry`, which no longer resolves).
- Modify `crates/agentstacks/src/types/lock/entry/mod.rs` — drop `pub use written::MergeEntry;` and
  the merge words of the doc.
- Modify `crates/agentstacks/src/types/lock/file.rs` — `TOP_LEVEL_KEYS` becomes
  `["api", "airsl", "tool", "pack", "file"]` (5 keys); remove `LockContents::merges`,
  `Lockfile::merges` and `merges()`, the `merge` array read, `Identities::merges`, the merge `by`
  owners, the merge repeat check and the merge sort; module and item docs drop merges.
- Modify `crates/agentstacks/src/types/lock/write.rs` — remove the `[[merge]]` loop; docs list
  three arrays.
- Modify `crates/agentstacks/src/types/lock/error.rs` — the `ListedTwice` variant doc says "tool,
  pack or file"; remove the `EntryKey::Merge` case of `lock_error_displays_each_refusal_text` and its
  imports.
- Modify `crates/agentstacks/src/types/lock/owner.rs` — docs say a pack or tool wrote a file (three
  mentions of "file or merge").
- Modify `crates/agentstacks/src/types/lock/testing.rs` — `example_lock()` ends after the
  `[[file]]` table (its last line `by   = "core"\n`).
- Modify `crates/agentstacks/src/types/lock/mod.rs` — drop `pub mod merge;`,
  `pub use entry::MergeEntry;`, `pub use merge::MergeFile;`, `pub use merge::MergeKey;` and their
  doc bullets.
- Modify `crates/agentstacks/src/types/mod.rs` — drop `pub use lock::MergeEntry;`,
  `pub use lock::MergeFile;`, `pub use lock::MergeKey;` and nothing else.

**Change:** A lock holds `api`, `[airsl]`, `[[tool]]`, `[[pack]]` and `[[file]]` only (spec §12
row 2, D1, D2). A lock that still has `[[merge]]` is refused by the built unknown-key rule at the
top level: `agentstacks.lock: merge: unknown key` (the shape of `agentstacks.lock: foo: unknown
key`, `src/types/lock/file.rs:649`).

**Proof:** written first, seen failing, then passing. In `file.rs`:
- `lockfile_refuses_a_merge_table_as_an_unknown_key` (new) — `api = 0` followed by one `[[merge]]`
  table with `file`, `key`, `hash`, `by`: the only refusal is `agentstacks.lock: merge: unknown key`.
- `lockfile_parses_the_documented_example` — the merges assertion is removed; tools, packs and files
  are asserted as today.
- `lockfile_refuses_duplicates` — the merge case (`merge[1]: merge ".claude/settings.json" "hooks"
  listed twice`) is removed; the other cases are unchanged.
- `lockfile_accepts_distinct_namespaces` — the two texts that use `[[merge]]` are removed; the tool
  and pack text stays.
- `lockfile_checks_by_against_tools_and_packs` — the `merge[0].by` case is removed; the `file` cases
  stay.
- `lockfile_sorts_parsed_entries` — the two merges and their assertion are removed.
- The `merge` text helper of the test module is removed.

In `write.rs`:
- `write_lock_reproduces_the_documented_example` and `write_lock_round_trips_through_parse` pass on
  the shortened `example_lock()`.
- `write_lock_sorts_entries` and `write_lock_round_trips_an_in_memory_lock_with_every_path_shape` —
  the `merges` arguments are removed (the `lock` helper loses its `merges` parameter and the `merge`
  helper is removed).
- `write_lock_escapes_strings` — the escaped string moves from a merge key to a file path: a
  `[[file]]` whose path is `a"b` + tab + `d` + newline + `e` + U+0001 is written as
  `path = "a\"bd\ne\u0001"` with the tab escaped as `\t` (the path holds no `\`, which Task 3
  refuses on read), reads back to the same path, and the reread lock equals the written one.

In `key.rs` and `error.rs`: the existing tests pass without their merge case.

**Done when:** `cargo make dod-crate agentstacks` passes, `grep -rni "merge" crates/agentstacks/src/types/lock`
finds only the new test's `[[merge]]` text and its name, and the Definition of Done is green.
Commit `feat(agentstacks): remove the lock's [[merge]]`.

### Task 3 — Refuse a `\` and agentstacks' own files in a lock file path

**Files:**
- Modify `crates/agentstacks/src/types/lock/error.rs` — `LockError::Separator { text }` displaying
  `"<text>" must use / as the separator` and `LockError::Managed { text }` displaying
  `"<text>" is managed by agentstacks`; one case each in `lock_error_displays_each_refusal_text`.
- Modify `crates/agentstacks/src/types/lock/entry/written.rs` — `read_path` refuses, after its
  existing checks pass, a path containing `\` with `Separator`, and a path whose `normalized()` is
  `agentstacks.lock` or `agentstacks.toml` with `Managed`, quoting the path as written; module doc;
  tests.

**Change:** §16 R16 and R28(e): when a lock is read
(`Lockfile::parse`), a `[[file]].path` containing `\` is refused with
`agentstacks.lock: file[0].path: "a\b" must use / as the separator`, and one naming agentstacks'
own files is refused with `agentstacks.lock: file[0].path: "agentstacks.lock" is managed by
agentstacks` (same shape for `agentstacks.toml`). `Lockfile::new` takes typed `FileEntry` values and
is unchanged: §16 R16 scopes the rule to reading, and the op path rule (§3.3) already refuses both
before a path can reach the lock.

**Proof:** written first, seen failing, then passing. In `written.rs`, `file_entry_refuses_each_rule`
gains three cases (the shape of its existing `"/x" must be relative` case):
- path `a\b` (one backslash): `agentstacks.lock: file[0].path: "a\b" must use / as the separator`.
- path `agentstacks.lock`: `agentstacks.lock: file[0].path: "agentstacks.lock" is managed by agentstacks`.
- path `agentstacks.toml`: `agentstacks.lock: file[0].path: "agentstacks.toml" is managed by agentstacks`.

In `error.rs`, `lock_error_displays_each_refusal_text` gains `Separator { text: "a\b" }` →
`"a\b" must use / as the separator` and `Managed { text: "agentstacks.lock" }` →
`"agentstacks.lock" is managed by agentstacks`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): refuse backslashes and own files in lock paths`.

### Task 4 — Refuse the `pure` hook policy in api 0

**Files:**
- Modify `crates/agentstacks/src/types/hooks/policy.rs` — `Policy` has one variant, `Confined`
  (still the default); `Policy::new("pure")` returns `InvalidPolicy::PureUnsupported`; `trusted`
  stays `InvalidPolicy::NotAllowed`; any other text is `InvalidPolicy::Unknown`; module doc and the
  `Policy` doctest drop `pure`; tests.
- Modify `crates/agentstacks/src/types/hooks/invalid.rs` — add `PureUnsupported`; `Unknown` displays
  `"<text>" (expected confined)`; module doc; `invalid_policy_displays_each_message`.
- Modify `crates/agentstacks/src/types/hooks/grants.rs` — module doc: the policy is `confined`
  (api 0 allows no other); the `Grants` doctest's `policy = "pure"` becomes `policy = "confined"` and
  asserts `Policy::Confined`; `grants_refuse_each_rule` cases (below).
- Modify `crates/agentstacks/src/types/hooks/mod.rs` — the `Policy` bullet: "the sandbox policy,
  `confined`".
- Modify `crates/agentstacks/src/types/manifest/error.rs` — the `InvalidPolicy::Unknown` case in
  `value_cases` expects `"foo" (expected confined)`.

**Change:** Spec §12 row 3 and D14: a hook (in `pack.toml` or `native/<tool>/hooks.toml`, both read
by `Grants::read`) with `grants.policy = "pure"` is refused at pack load with
`"pure" is not supported in api 0 (airsl has no require under pure, airsstack/airsl#12)`; an unknown
policy reads `"<text>" (expected confined)`. The `trusted` refusal is unchanged (not part of §12).

**Proof:** written first, seen failing, then passing.
- `policy_refuses_pure_in_api_0` (new, `policy.rs`) — at
  `Location::at_key("packs/core/pack.toml", "hooks[1].grants.policy")`:
  `packs/core/pack.toml: hooks[1].grants.policy: "pure" is not supported in api 0 (airsl has no require under pure, airsstack/airsl#12)`.
- `policy_refuses_an_unknown_value_naming_the_allowed_set` (changed) — for `foo`, `open` and the
  empty string: `packs/core/pack.toml: hooks[1].grants.policy: "<text>" (expected confined)`; for
  `foo` this is the spec line `hooks[1].grants.policy: "foo" (expected confined)`.
- `policy_parses_confined` (renamed from `policy_parses_confined_and_pure`) — `confined` parses,
  is the default and displays `confined`.
- `policy_refuses_trusted_as_not_allowed` — unchanged.
- `policy_builds_alike_through_new_from_str_and_try_from` — its `pure` assertion becomes "refused".
- `invalid_policy_displays_each_message` — `PureUnsupported` displays the `pure` text above;
  `Unknown { text: "foo" }` displays `"foo" (expected confined)`.
- `grants_refuse_each_rule` — the `policy = "foo"` case expects
  `hooks[1].grants.policy: "foo" (expected confined)`; a new case `policy = "pure"` expects
  `hooks[1].grants.policy: "pure" is not supported in api 0 (airsl has no require under pure, airsstack/airsl#12)`.
- `native_hook_keeps_the_neutral_rules` (`native/hook.rs`) — a new case with `[grants]
  policy = "pure"` expects `hooks[0].grants.policy: "pure" is not supported in api 0 (airsl has no require under pure, airsstack/airsl#12)`.
- `manifest_error_messages_match_their_wording` (`manifest/error.rs`) passes with the changed case.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): refuse the pure hook policy in api 0`.

### Task 5 — Require every `run` segment to be require-able

**Files:**
- Modify `crates/agentstacks/src/types/manifest/error.rs` — `ManifestError::RunSegment { run, segment }`
  displaying `"<run>" segment "<segment>" may use only letters, digits, _ and -`; a case in
  `hook_cases`.
- Modify `crates/agentstacks/src/types/hooks/run.rs` — the segment rule in `HookRun::read`; module
  doc lists the rule and why (the segment becomes a dotted module name under the hook runtime, spec
  §11.1); tests.
- Modify `crates/agentstacks/src/types/hooks/native/hook.rs` — a case in
  `native_hook_keeps_the_neutral_rules`.

**Change:** Spec §12 row 4, D14 and §16 R24. When the `run` form splits and `run` ends in `.lua`
(other values are already refused by the `.lua` row and get no segment refusal), each `/` segment of
the pack-relative path that follows the form, with `.lua` removed from the last one, must match
`^[A-Za-z0-9_-]+$`. The first segment that does not is refused at `run` with `RunSegment`, quoting
`run` as written; later bad segments are not reported, and no `HookRun` is returned. Example:
`{{root core}}/hooks/enforce.lua` becomes the module `core.hooks.enforce` (§11.1), so `my.hook`
cannot be one segment. `PackPath` already refuses `.`, `..`, empty segments and `\`
(`src/types/manifest/pack/path.rs`), so the segments this rule sees are non-empty and plain. The
rule applies to `native/<tool>/hooks.toml` through the same `HookRun::read`.

**Proof:** written first, seen failing, then passing.
- `hook_run_refuses_a_segment_that_cannot_be_required` (`run.rs`) — `run = 'hooks/my.hook.lua'`:
  the only refusal is
  `packs/core/pack.toml: hooks[1].run: "hooks/my.hook.lua" segment "my.hook" may use only letters, digits, _ and -`
  (§12 row 4), and no `HookRun` is returned.
- `hook_run_quotes_a_token_form_as_written` (`run.rs`) — `run = '{{root core}}/hooks/my.hook.lua'`:
  the only refusal is
  `packs/core/pack.toml: hooks[1].run: "{{root core}}/hooks/my.hook.lua" segment "my.hook" may use only letters, digits, _ and -`
  (§16 R24).
- `hook_run_reports_only_the_first_bad_segment` (`run.rs`) — `run = 'a.b/c.d.lua'`: the only
  refusal is `packs/core/pack.toml: hooks[1].run: "a.b/c.d.lua" segment "a.b" may use only letters, digits, _ and -`
  (§16 R24).
- `hook_run_checks_segments_of_lua_runs_only` (`run.rs`) — `run = 'hooks/my.hook.sh'`: the only
  refusal is `packs/core/pack.toml: hooks[1].run: "hooks/my.hook.sh" is not a .lua file` (§16 R24).
- `hook_run_accepts_letters_digits_underscore_and_hyphen` (`run.rs`) — `hooks/handoff-check_2.lua`,
  `Hooks/A9.lua` and `{{root core}}/hooks/enforce.lua` are accepted with no refusal.
- `hook_run_accepts_the_three_forms` (existing) — unchanged and passing.
- `native_hook_keeps_the_neutral_rules` (`native/hook.rs`) — a new case
  `run = "hooks/my.hook.lua"` expects
  `hooks[0].run: "hooks/my.hook.lua" segment "my.hook" may use only letters, digits, _ and -`.
- `manifest_error_messages_match_their_wording` — `RunSegment { run: "hooks/my.hook.lua", segment:
  "my.hook" }` displays `"hooks/my.hook.lua" segment "my.hook" may use only letters, digits, _ and -`.

**Done when:** `cargo make dod-crate agentstacks` passes, then `cargo make dod`, and the
Definition of Done is green. Commit `feat(agentstacks): require-able hook run segments`.

---

## Risks

- **Riskiest step: Task 2.** It deletes three public types and a public field. Inside the crate,
  `grep -rn -i merge crates/agentstacks/src crates/agentstacks/tests` before the task lists every
  user (the lock group and `types/mod.rs`); `src/types/report/diagnostic.rs:109` uses the string
  `merge[0].key` only as an example key path in a `Location` test and stays. `agentstacks-cli` uses
  none of them.
- Task 2 edits `src/types/mod.rs`, which the skeleton plan owns; this plan depends on it and changes
  only the three `Merge*` re-export lines.
- Task 1 copies the lock's path decision rather than sharing one function, because the two enums
  that carry the refusal differ (`LockError`, `ExtensionError`); the wording cannot drift, since
  both use the shared phrase types. `extension.reads` takes Task 3's `\` check but not its own-file
  check (§16 R28(n)).
- `LockError::Separator` and `LockError::Managed` repeat wording that the errors plan's `OpError`
  and the built `InvalidPackPath::Backslash` also print; the final cross-check may fold them into
  one shared phrase type in `report/phrase.rs`.
- `extension_reads_are_the_manifests` uses `crate::port::FakeFileTree` in a `#[cfg(test)]` unit test
  under `src/types`, which §16 R28(k) allows and the skeleton plan's isolation guard accepts.

## Proof

- `cargo make dod` is green at the end of Task 5.
- Spec rows covered: §12 row 1 and §16 R24 (reads quoting) (Task 1: every listed refusal, normalised
  duplicates, optional list, `reads` exposed, R28(n) separator and own files), §12 row 2 (Task 2),
  §16 R16 and R28(e) (Task 3), §12 row 3 and D14 first sentence (Task 4), §12 row 4, D14 second sentence and
  §16 R24 (run segment), incl. `native/<tool>/hooks.toml` (Task 5).

## Review findings

Reviewer (one round, shared batch with plan 02): plan 03 share was 0🔴 4🟡 2🔵 plus 1 plan-hygiene 🔵. Blocking: `lock/file.rs:13`. Batch `cargo make dod` exit 0 (804 unit, 31 integration, 233 doctests).

- fixed 🟡 (blocking) `lock/file.rs:13`, `project/path/sub.rs:9-11`. The module doc claimed that a lock built with `Lockfile::new` is held to the parse rules, but `new` accepts `\` and agentstacks' own files. The docs were narrowed to say that the operation path rule keeps those paths out before a lock is built. `cargo test -p agentstacks --lib types::lock` gave 43 passed, 0 failed. `cargo fmt --all -- --check` was clean.
- fixed 🟡 `lock/entry/written.rs:150`: the own-file check was unguarded for other spellings. Added the `./agentstacks.toml` and `./agentstacks.lock` cases to `file_entry_refuses_each_rule`. Same run: 43 passed.
- fixed 🟡 `lock/write.rs:342`: the `'\\'` arm of `quote` was unguarded. Added `quote_escapes_a_backslash`. Same run: 43 passed.
- open 🟡 `extension/manifest.rs:247`: `reads = ["."]` / `["./"]` normalise to `""` (the project root) and are accepted. Lock `[[file]].path = "."` is the same. This is a plan gap, because only blank entries were listed.
- open 🔵 `extension/manifest.rs:449`: a redundant `use` under `use super::*`.
- open 🔵 `tests/fixtures/packs/hook-message-grants/pack.toml:11`: still `policy = "pure"`. The test passes only because the message-hook refusal comes first.
- open 🔵 `tests/pack.rs` was edited but is not in this plan's file list. The edit is accepted; see Deviations.

## Deviations

- 2026-11-22: `tests/pack.rs` was not in the file list. Its `hook-grants-policy-unknown` row expected `(expected confined, pure)` and now expects `(expected confined)`, because Task 4 changed the Unknown text.
- 2026-11-22: Task 2's expected text for `write_lock_escapes_strings` was garbled. It was built to the stated input: path `a"b<TAB>d<LF>e<U+0001>` is written as `path = "a\"b\td\ne\u0001"`.
- 2026-11-22: The coder ran `cargo make fmt` once mid-task. It rewrites the whole tree, including plan 02's files in progress. The final fmt-check was clean.
