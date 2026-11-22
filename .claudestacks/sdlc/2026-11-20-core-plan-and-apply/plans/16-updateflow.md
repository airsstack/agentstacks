---
status: done
created: 2026-11-22
depends-on: [07, 09]
built-on: f39b2a814319
---

# Update flow Implementation Plan

**Goal:** Find the newest released tag for every git source in `agentstacks.toml`.

**Architecture:** `app/update` is a flow and holds no rule (spec §2, `modularity.md` Rule 1). It
reads `agentstacks.toml` through `Workspace::project_text` and parses it with the built
`ProjectFile::parse`. For each source, in key order (spec §16 R8), it asks plan 07's
`UpdateQuery::for_source`, which settles path sources, rev pins and tags with no version. For an
`UpdateQuery` it asks `Git::list_tags` with the query's URL and prefix, then hands the tags to
`UpdateQuery::finish`, which yields the `UpdateEntry`. The flow orders those calls, collects
entries and refusals, and assembles the one git-not-found refusal. It is generic over
`W: Workspace` and `G: Git` (spec §2 static-dispatch row). It never writes: writing a new tag into
`agentstacks.toml` belongs to `cli-commands` (spec §9.4 last paragraph, §17, §18).

**Tech Stack:** Rust 2024, crate `agentstacks`; `Workspace`, `Git`, `FakeWorkspace`, `FakeGit`,
`WorkspaceCall`, `GitCall` from plan 09; `UpdateQuery`, `UpdateEntry`, `UpdateOutcome`,
`UpdateNote`, `SourceKey` from plan 07; `FetchError` (including `GitNotFound { needed_by }`) from
plan 02; the built `ProjectFile`, `Source`, `Pin` (`src/types/project/`), `Diagnostics`,
`Location`.

**Content authority:** spec §9.4 (every case row and the `app/update` output), §9.2 (the list-tags
step and its failure rows), §16 R1, R2, R8, §13 (`agentstacks.toml` locations), §14 `app/update`
row.

---

## Files that change

```
crates/agentstacks/src/app/update.rs   — modify: replace the docs-only stub with the update flow and its colocated tests
```

`src/app/mod.rs` already declares `update` (plan 01) and is not edited here: callers reach the flow
as `app::update::update`.

## Order of work

### Task 1 — Read the project and settle the sources that need no git

**Files:**
- Modify `crates/agentstacks/src/app/update.rs` — the module doc (what, why, responsibilities,
  non-responsibilities: no rule, no write to `agentstacks.toml`, no shorthand expansion) and the
  function `update`.

**Change:** `update<W: Workspace, G: Git>(workspace: &W, git: &G) -> Result<Vec<UpdateEntry>,
Diagnostics>` returns one `UpdateEntry` per git source (spec §9.4: "its key, the current tag, and
the new tag or the note"). Refusals are collected, never first-only, and handed back sorted by the
built `Diagnostics` (spec §13); any refusal makes the result `Err`. Its rustdoc describes inputs
and output in prose and carries no example that uses a fake: the fakes are `#[cfg(test)]` and a
doctest cannot see them. This task adds:

- `Workspace::project_text` fails (the file is missing or unreadable) → the refusal
  `agentstacks.toml: <io::Error Display>` (spec §16 R1), and nothing else runs;
- the text is parsed with `ProjectFile::parse` at `agentstacks.toml`; its refusals are returned
  unchanged, and nothing else runs;
- sources are visited in key order, every `packs.<name>` then every `tools.<name>`, names in order
  (spec §16 R8 "update entries follow key order (`packs.*`, then `tools.*`)");
- each source goes to `UpdateQuery::for_source`: `None` (a `path` source) → no entry (spec §9.4
  row 4); `Err(entry)` → that entry, with no git call: `agentstacks.toml: packs.core: pinned to rev
  <rev>, not updated` for a rev pin (spec §9.4 row 3) and `agentstacks.toml: packs.core: tag
  "main-latest" has no version to update from` for a tag with no split (spec §9.4 row 1).

**Proof:** written first, seen failing, then passing.
- `update_reports_a_missing_project_file` — `FakeWorkspace` with no project: the error holds
  exactly `agentstacks.toml: ` followed by the Display of the `NotFound` error `project_text`
  returned, and `FakeGit::calls()` is empty.
- `update_reports_an_unreadable_project_file` — `FakeWorkspace::fail(WorkspaceCall::ProjectText)`:
  the error holds exactly `agentstacks.toml: ` followed by the Display of
  `FakeWorkspace::denied()`.
- `update_passes_project_refusals_through` — a project text `api = 0` and nothing else: the error
  holds `agentstacks.toml: nothing to install` (formats §7), and `FakeGit::calls()` is empty.
- `update_gives_no_entry_for_a_path_source` — a project whose only pack and tool are `path`
  sources: the result is an empty list.
- `update_notes_a_rev_pin` — `packs.core` pinned to a 40-hex rev: one entry whose note displays
  `agentstacks.toml: packs.core: pinned to rev <rev>, not updated` with that rev; no git call.
- `update_notes_a_tag_with_no_version` — `packs.core` at tag `main-latest`: one entry whose note
  displays `agentstacks.toml: packs.core: tag "main-latest" has no version to update from`; no git
  call.
- `update_orders_entries_by_key` — rev-pinned `tools.claude`, `packs.lint` and `packs.core`: three
  entries, keys `packs.core`, `packs.lint`, `tools.claude`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): update reads the project sources`.

### Task 2 — Ask git for newer tags

**Files:**
- Modify `crates/agentstacks/src/app/update.rs` — the tag step and its tests.

**Change:** for an `UpdateQuery`, the flow calls `Git::list_tags` with the query's URL and prefix
(spec §9.4 step 1, the §9.2 list-tags row) and hands the answer to `UpdateQuery::finish` (spec §9.4
steps 2-3; the R8 tie-break on equal precedence is plan 07's), which yields either:

- `UpdateOutcome::NewTag` → the entry carries the current tag and the new tag;
- the note `agentstacks.toml: packs.core: no newer tag than packs-v0.1.0` (current tag filled in)
  when the list is empty, holds only pre-releases or non-semver suffixes, or nothing newer than the
  current version (spec §9.4 row 2).

`subdir` plays no part in listing tags.

**Proof:** written first, seen failing, then passing.
- `update_asks_git_with_the_tag_prefix` — `packs.core` at `packs-v0.1.0` and `tools.claude` at
  `x-1.2.3-4.5.6`: `FakeGit::calls()` is exactly `ListTags` for `packs.core`'s URL with prefix
  `packs-v`, then `ListTags` for `tools.claude`'s URL with prefix `x-`.
- `update_offers_the_highest_released_tag` — `FakeGit::tags` lists `packs-v0.1.0`,
  `packs-v0.10.0`, `packs-v0.2.0`, `packs-v1.0.0-rc.1`, `packs-vnext`: the entry for `packs.core`
  has current tag `packs-v0.1.0` and new tag `packs-v0.10.0` (semver order, not lexical;
  pre-release and non-semver dropped).
- `update_notes_no_newer_tag` — `FakeGit::tags` lists only `packs-v0.1.0`: the note
  `agentstacks.toml: packs.core: no newer tag than packs-v0.1.0`.
- `update_notes_no_newer_tag_when_nothing_matches` — `FakeGit::tags` lists nothing under the
  prefix: the same note.
- `update_leaves_the_project_untouched` — after a run with a new tag, `FakeWorkspace::calls()` is
  exactly `[ProjectText]` (no stage, swap, delete or lock write).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): update finds the latest tag`.

### Task 3 — Pass git failures through

**Files:**
- Modify `crates/agentstacks/src/app/update.rs` — the failure collection and its tests.

**Change:** a `Git::list_tags` error is a refusal located at `agentstacks.toml` (spec §13), and the
flow carries on with the other sources so every refusal is collected:

- git's own failures pass through located with the source's key: a non-zero exit as
  `agentstacks.toml: packs.core: git ls-remote failed: <git's stderr, trimmed>` (spec §9.2 failure
  row 2); a spawn failure other than "not found" as `agentstacks.toml: packs.core: git: <io::Error
  Display>` (spec §16 R1); an `ls-remote` line that does not parse as `agentstacks.toml: packs.core:
  git ls-remote printed output agentstacks cannot read: "<first line>"` (spec §16 R2). The flow adds
  only the location; the `FetchError` and its text come from the port;
- "git not found on `PATH`" answers are not reported one by one: the flow gathers the key of every
  source whose `list_tags` gave that answer, in visiting order, and adds one refusal at
  `agentstacks.toml` with no key, `FetchError::GitNotFound { needed_by }`, displayed
  `agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)` (spec §9.2 failure
  row 1, "once"; §13 row "`agentstacks.toml`, no key").

**Proof:** written first, seen failing, then passing.
- `update_passes_a_git_failure_through` — `FakeGit::fail` on `packs.core`'s URL with git's stderr
  `fatal: could not read Username for 'https://github.com': terminal prompts disabled`: the error
  holds exactly `agentstacks.toml: packs.core: git ls-remote failed: fatal: could not read Username
  for 'https://github.com': terminal prompts disabled`.
- `update_reports_missing_git_once` — `FakeGit::git_missing()` with `packs.core` and
  `tools.claude` at version tags: the error holds exactly one refusal, `agentstacks.toml: git not
  found on PATH (needed by packs.core, tools.claude)`.
- `update_reports_missing_git_only_for_sources_that_asked` — `FakeGit::git_missing()`, `packs.core`
  at `packs-v0.1.0` and `tools.claude` pinned by rev: exactly `agentstacks.toml: git not found on
  PATH (needed by packs.core)`.
- `update_collects_every_git_failure` — `packs.core` and `packs.lint` both fail with different
  stderr: both refusals, sorted, and no entries.

**Done when:** `cargo make dod-crate agentstacks` passes, then `cargo make dod` passes, and the
Definition of Done is green. Commit `feat(agentstacks): update passes git failures through`.

---

## Risks

- The tag split, the latest-tag choice, the "newer than current" comparison, the R8 tie-break and
  the note texts live in plan 07's `UpdateQuery` and `UpdateNote`. The flow never compares versions
  itself, or the rule is duplicated in `app/` (`modularity.md` Rule 2). Task 2 is the riskiest step
  for that.
- The git-not-found refusal is assembled here and in `app/plan` (plan 13) from per-source answers
  into the one plan 02 `FetchError::GitNotFound { needed_by }`. The adapter (plan 11) only answers
  per call; neither flow builds the text itself.
- `tests/isolation.rs` refuses adapter imports and `std::process` inside `src/app`. Tests use
  `FakeGit` only; no repository, no network (spec §14).
- Not done here: writing the new tag, shorthand expansion with prefix `packs-v` and its refusal
  when `latest_tag` is `None` (spec §9.4 last paragraph, §17 `cli-commands`).

## Proof

- Spec §9.4 rows covered: "no such split" Task 1; "`None`, or nothing newer" Task 2 (both
  halves); "pinned by `rev`" Task 1; "`path` source" Task 1; steps 1-3 and the split examples
  (`packs-v0.1.0`, `x-1.2.3-4.5.6`) Task 2; the `app/update` output (key, current tag, new tag or
  note, one per git source) Tasks 1-2.
- Spec §9.2 failure rows met by listing tags: git missing (once) and git non-zero, Task 3.
- Spec §16 rows covered: R1 (`agentstacks.toml` missing or unreadable, Task 1; git spawn, Task 3);
  R2 (unreadable `ls-remote` output, Task 3); R8 (key order, Task 1).
- Spec §14 `app/update` row: fakes only.
- No rustdoc example uses a fake (`grep -n "Fake" crates/agentstacks/src/app/update.rs` shows hits
  only inside `#[cfg(test)] mod tests`).
- `cargo test -p agentstacks --test isolation` passes.
- `cargo make dod` is green at the end of the plan.

## Review findings

Reviewer, one round, shared with plans 14 and 15: code 0🔴 1🟡 1🔵, plan 1🔵. Blocking: none, so there was no fix round. Reviewer `cargo test -p agentstacks --lib app::`: 92 passed, 0 failed. Coder: 18 update tests green (16 named plus 2 extra). The full `cargo make dod` gate is recorded at the end of the run.

- open 🟡 `src/app/update.rs:89-95`: no test covers locating a `FetchError::CacheIo` at its carried path. The same choice is made in `app/plan/sources.rs:299`.
- open 🔵 `update.rs:97-102`: `single` duplicates `Diagnostics::from`.
- open 🔵 Locating `CacheIo` at its carried path is not written into the plan. It matches the `FetchError::CacheIo` docs.

## Deviations

- 2026-11-22: Tests configure `FakeGit::tags(url, vec![])`, because an unconfigured URL answers git-missing.
- 2026-11-22: Two extra tests cover the git spawn and unreadable-output passthrough and the tool-key refusal.
- 2026-11-22: A git `CacheIo` is located at its carried path; other `FetchError`s are located at the source key.
