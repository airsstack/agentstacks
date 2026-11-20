---
status: draft
created: 2026-11-20
depends-on: [01, 02, 03, 08]
built-on: e665589a43a9
---

# Lockfile Implementation Plan

**Goal:** `agentstacks.lock` round-trips through one strict, deterministically ordered `Lockfile`
type.

**Architecture:** `lock/lockfile.rs` holds `Lockfile { api, airsl, tools, packs, files, merges }`
with `Lockfile::parse(text)` and `Lockfile::to_toml_string()`. Entries are one type per file:
`LockedSource` (`name`, `version`, the `Source` from plan 08, `rev`, `tree`) used for both tools
and packs; `FileEntry` (`path`, `hash`, `by`); `MergeEntry` (`file`, `key`, `hash`, `by`).
Hashes reuse `ContentHash` (plan 03); names reuse the name newtypes. Serialisation sorts tools
and packs by name, files by path, merges by (file, key), and writes the generated-file comment.
`check_airsl_presence(needs_airsl)` is the hook for the "present iff a locked pack needs airsl"
rule, which can only be decided with the packs in hand; it is tested here and called by the core
later. `LockError` joins `FormatError`.

**Tech Stack:** Rust 2024, `toml` 1, `semver` 1.

**Content authority:** spec §8 (all rows, sorted writer rule, missing lock not an error), §8.1
(`ContentHash` form).

---

## Files that change

```
crates/agentstacks/src/lock/mod.rs            — modify  declare and re-export
crates/agentstacks/src/lock/locked_source.rs  — create  LockedSource for [[tool]] and [[pack]]
crates/agentstacks/src/lock/file_entry.rs     — create  FileEntry for [[file]]
crates/agentstacks/src/lock/merge_entry.rs    — create  MergeEntry for [[merge]]
crates/agentstacks/src/lock/lockfile.rs       — create  Lockfile: parse, sorted serialise, airsl presence
crates/agentstacks/src/lock/lock_error.rs     — create  LockError (thiserror)
crates/agentstacks/src/model/diagnostic.rs    — modify  FormatError gains a Lock variant
```

## Order of work

### Task 1 — Parse lock entries

**Files:**
- Create `locked_source.rs`, `file_entry.rs`, `merge_entry.rs`, `lock_error.rs`; modify
  `lock/mod.rs`, `model/diagnostic.rs`.

**Change:** `LockedSource`: `name`, `version`, `tree` required; exactly one of `git`/`path`; git
entries need `rev` (a `CommitId`) and keep `tag` when present; the §7 source rules apply.
`FileEntry`: `path` (relative, `/`-separated, no `..`), `hash`, `by` required. `MergeEntry`:
`file`, non-empty `key`, `hash`, `by` required. Hashes must parse as `ContentHash`
(`agentstacks.lock:<line>: bad hash`).

**Proof:**
- `locked_source_parses_git_and_path_entries` — the §8 `core` and `team-lint` entries parse.
- `lock_entries_refuse_each_rule` — table-driven: git entry without `rev` (`pack core: missing
  rev`), both `git` and `path` (`needs exactly one of git, path`), missing `tree`, `hash =
  "sha256:XYZ"` (`bad hash`), file path `../x`, empty merge `key`, `git = "-x"`; exact messages.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse lock entries`.

### Task 2 — Parse and write the whole lockfile

**Files:**
- Create `lockfile.rs`; modify `lock_error.rs`, `lock/mod.rs`.

**Change:** `api` must be 0; `[airsl] version` is an exact semver version when present; tool
names unique and pack names unique (`agentstacks.lock: pack "core" listed twice`); file paths
unique (`file ".claude/x" listed twice`); merges unique by (file, key) (`merge
".claude/settings.json" "<key>" listed twice`); every `by` names a locked pack or tool (`file …:
by "kore" is not a locked pack or tool`); unknown keys and syntax errors refused with their line.
`to_toml_string` writes the generated-file comment and sorted entries.
`check_airsl_presence(needs_airsl)` refuses `[airsl]` missing when needed and present when not.
There is no "missing lock" error: the caller treats an absent file as no lock.

**Proof:**
- `lockfile_round_trips_the_spec_example` — parse the §8 example, serialise, parse again: equal
  values; serialising twice gives identical text.
- `lockfile_serialises_sorted` — entries built in reverse order serialise tools and packs by
  name, files by path, merges by (file, key), with the generated-file comment first.
- `lockfile_refuses_each_rule` — table-driven over the §8 rows not covered in Task 1: `api = 1`,
  `airsl.version = "0.1"`, duplicate pack, duplicate file, duplicate merge, unknown `by`, unknown
  key, a syntax error with its line; exact messages.
- `airsl_presence_matches_need` — `[airsl]` absent with need, and present without need, are both
  refused; the two consistent cases pass.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): read and write agentstacks.lock`.

---

## Risks

- `toml` serialisation of arrays of tables must keep the field order shown in §8 for readable
  diffs; the round-trip and determinism tests catch reorderings between runs, not cosmetic order,
  so field order follows struct declaration order.
- The meaning of `[[merge]].key`, stale-lock reconciliation and tree mismatches belong to
  `core-plan-and-apply`; this plan stores and validates shapes only.

## Proof

- `cargo make dod-crate agentstacks` green; every §8 row has a case; serialisation is
  byte-identical across runs.
