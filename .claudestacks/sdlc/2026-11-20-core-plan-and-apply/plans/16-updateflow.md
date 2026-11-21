---
status: draft
created: 2026-11-22
depends-on: [07, 09]
built-on: ea104af7451e
---

# Update flow Implementation Plan

**Goal:** Find the newest released tag for every git source in `agentstacks.toml`.

**Architecture:** `app/update` is a flow and holds no rule (spec §2, `modularity.md` Rule 1). It
reads `agentstacks.toml` through the `Workspace` port and parses it with the built
`ProjectFile::parse`. For each git source it hands the current tag to plan 07's tag split, asks the
`Git` port to list tags under the prefix, and hands the listing and the current tag to plan 07's
latest-tag selection, which yields the `UpdateEntry` (new tag or note). The flow orders those calls
and collects entries and refusals. It is generic over `W: Workspace` and `G: Git` (spec §2
static-dispatch row). It never writes: writing a new tag into `agentstacks.toml` belongs to
`cli-commands` (spec §9.4 last paragraph, §16, §17).

**Tech Stack:** Rust 2024, crate `agentstacks`; `FakeWorkspace` and `FakeGit` from plan 09; the
built `ProjectFile`, `Source` and `Pin` (`src/types/project/`); the tag split, latest-tag selection
and `UpdateEntry` from plan 07; the §9.2 git refusals (`FetchError`) from plan 02.

**Content authority:** spec §9.4 (every case row and the `app/update` output), §9.2 (the list-tags
step and its failure rows), §13 (`agentstacks.toml` with key / no key locations), §14 `app/update`
row.

---

## Files that change

```
crates/agentstacks/src/app/update.rs   — modify: replace the docs-only stub with the update flow and its colocated tests
```

`src/app/mod.rs` already declares `update` (plan 01). If the flow is exported from `app`, the one
`pub use` line it needs is the only edit there, and it stays export-only (`mod-rs-export-only.md`).

## Order of work

### Task 1 — Read the project and settle the sources that need no git

**Files:**
- Modify `crates/agentstacks/src/app/update.rs` — the module doc (what, why, responsibilities,
  non-responsibilities: no rule, no write to `agentstacks.toml`, no shorthand expansion) and the
  function `update`.

**Change:** `update<W: Workspace, G: Git>(workspace: &W, git: &G) -> Result<Vec<UpdateEntry>,
Diagnostics>` returns one `UpdateEntry` per git source (spec §9.4: "its key, the current tag, and
the new tag or the note"). Refusals are collected, never first-only, and handed back sorted by the
built `Diagnostics` (spec §13). This task adds:

- the project text is read through `Workspace` and parsed with `ProjectFile::parse` at
  `agentstacks.toml`; its refusals are returned unchanged;
- sources are visited in key order, every `packs.<name>` then every `tools.<name>`, names in order
  (`ProjectFile` holds both in `BTreeMap`s; the same order as the §9.2 example `needed by
  packs.core, tools.claude`);
- `Source::Path` → no entry (spec §9.4 row 4, "nothing");
- `Source::Git` with `Pin::Rev` → the entry's note `agentstacks.toml: packs.core: pinned to rev
  <rev>, not updated`, and `Git` is not asked (spec §9.4 row 3);
- `Source::Git` with `Pin::Tag` whose tag has no split (plan 07's tag split returns none) → the
  note `agentstacks.toml: packs.core: tag "main-latest" has no version to update from`, and `Git`
  is not asked (spec §9.4 row 1).

**Proof:** written first, seen failing, then passing.
- `update_passes_project_refusals_through` — a project text `api = 0` and nothing else: the error
  holds `agentstacks.toml: nothing to install` (formats §7), and `FakeGit` recorded no call.
- `update_gives_no_entry_for_a_path_source` — a project whose only pack and tool are `path`
  sources: the result is empty.
- `update_notes_a_rev_pin` — `packs.core` pinned to a 40-hex rev: one entry whose note is
  `agentstacks.toml: packs.core: pinned to rev <rev>` with that rev, then `, not updated`; no git
  call.
- `update_notes_a_tag_with_no_version` — `packs.core` at tag `main-latest`: one entry whose note
  is `agentstacks.toml: packs.core: tag "main-latest" has no version to update from`; no git call.
- `update_orders_entries_by_key` — rev-pinned `tools.claude` and `packs.core`: two entries, keys
  `packs.core` then `tools.claude`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): update reads the project sources`.

### Task 2 — Ask git for newer tags

**Files:**
- Modify `crates/agentstacks/src/app/update.rs` — the tag step and its tests.

**Change:** for a `Pin::Tag` source with a split, the flow asks the `Git` port to list tags for the
source's URL under the split's prefix (spec §9.4 step 1, the §9.2 list-tags row), then hands the
listing and the current tag to plan 07's latest-tag selection (spec §9.4 steps 2-3), which yields
either the new tag or the note:

- a newer tag exists → the entry carries the current tag and the new tag;
- the listing is empty, every suffix is a pre-release or not semver, or nothing is newer than the
  current version → the note `agentstacks.toml: packs.core: no newer tag than packs-v0.1.0`
  (current tag filled in; spec §9.4 row 2).

`subdir` plays no part in listing tags.

**Proof:** written first, seen failing, then passing.
- `update_asks_git_with_the_tag_prefix` — `packs.core` at `packs-v0.1.0` and `tools.claude` at
  `x-1.2.3-4.5.6`: `FakeGit` recorded one list call per source, with each source's URL and the
  prefixes `packs-v` and `x-`.
- `update_offers_the_highest_released_tag` — `FakeGit` lists `packs-v0.1.0`, `packs-v0.10.0`,
  `packs-v0.2.0`, `packs-v1.0.0-rc.1`, `packs-vnext`: the entry for `packs.core` has current tag
  `packs-v0.1.0` and new tag `packs-v0.10.0` (semver order, not lexical; pre-release and
  non-semver dropped).
- `update_notes_no_newer_tag` — `FakeGit` lists only `packs-v0.1.0`: the note `agentstacks.toml:
  packs.core: no newer tag than packs-v0.1.0`.
- `update_notes_no_newer_tag_when_nothing_matches` — `FakeGit` lists nothing: the same note.
- `update_leaves_the_project_untouched` — after any of the runs above, the `FakeWorkspace` holds
  the same `agentstacks.toml` bytes and recorded no stage, swap, delete or lock write.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): update finds the latest tag`.

### Task 3 — Pass git failures through

**Files:**
- Modify `crates/agentstacks/src/app/update.rs` — the failure collection and its tests.

**Change:** a `Git` error is a refusal located at `agentstacks.toml` (spec §13), and the flow
carries on with the other sources so every refusal is collected:

- git exits non-zero → `agentstacks.toml: packs.core: git ls-remote failed: <git's stderr,
  trimmed>`, located with the source's key (spec §9.2 failure row 2);
- git is not on `PATH` → one refusal for the whole run, `agentstacks.toml: git not found on PATH
  (needed by packs.core, tools.claude)`, listing every source whose list call met it, in visiting
  order (spec §9.2 failure row 1, "once"). The flow gathers the keys and builds the single
  plan 02 `FetchError` for it.

Any refusal makes the result `Err`; no entries are returned alongside it.

**Proof:** written first, seen failing, then passing.
- `update_passes_a_git_failure_through` — `FakeGit` fails the list call for `packs.core` with the
  stderr `fatal: could not read Username for 'https://github.com': terminal prompts disabled`: the
  error holds `agentstacks.toml: packs.core: git ls-remote failed: fatal: could not read Username
  for 'https://github.com': terminal prompts disabled`.
- `update_reports_missing_git_once` — `FakeGit` answers "git not found" for both `packs.core` and
  `tools.claude`: the error holds exactly one refusal, `agentstacks.toml: git not found on PATH
  (needed by packs.core, tools.claude)`.
- `update_collects_every_git_failure` — `packs.core` fails and `packs.lint` fails with another
  stderr: both refusals, sorted.

**Done when:** `cargo make dod-crate agentstacks` passes, then `cargo make dod` passes, and the
Definition of Done is green. Commit `feat(agentstacks): update passes git failures through`.

---

## Risks

- The tag split, the latest-tag selection, the "newer than current" comparison and the note
  wording live in plan 07's types (`UpdateEntry`). The flow must never compare versions itself, or
  the rule is duplicated in `app/` (`modularity.md` Rule 2). Task 2 is the riskiest step for that.
- The "git not found, once" refusal is also met by `app/plan` (plan 13) and the git adapter
  (plan 11). All three must build it from the one plan 02 `FetchError` variant; if plan 11 already
  aggregates the keys, this flow passes the adapter's answer through and does not aggregate twice.
- The exact `Git` and `Workspace` method names are plan 09's; the tasks name the behaviour.
- `tests/isolation.rs` refuses adapter imports and `std::process` inside `src/app`. Tests use
  `FakeGit` only; no repository, no network (spec §14).
- Not done here: writing the new tag, shorthand expansion with prefix `packs-v` and its refusal
  when `latest_tag` is `None` (spec §9.4 last paragraph, `cli-commands`).

## Proof

- Spec §9.4 rows covered: "no such split" Task 1; "`None`, or nothing newer" Task 2 (both
  halves); "pinned by `rev`" Task 1; "`path` source" Task 1; steps 1-3 and the split examples
  (`packs-v0.1.0`, `x-1.2.3-4.5.6`) Task 2; the `app/update` output (key, current tag, new tag or
  note, one per git source) Tasks 1-2.
- Spec §9.2 failure rows met by listing tags: git missing (once) and git non-zero, Task 3.
- Spec §14 `app/update` row: fakes only.
- `cargo test -p agentstacks --test isolation` passes.
- `cargo make dod` is green at the end of the plan.
