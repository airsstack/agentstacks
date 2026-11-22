---
status: done
created: 2026-11-22
depends-on: [09]
built-on: f39b2a814319
---

# Workspace Implementation Plan

**Goal:** give the flows the real project disk.

**Architecture:** `src/fs/` is the only `std::fs` caller (spec §2). This plan adds two adapters
beside the built `StdFileTree` (`fs/disk.rs`, unchanged):

- `StdWorkspace` implements plan 09's `Workspace` over a project directory: the §6.3 resolution
  (follow a symlink, else canonicalise the deepest existing ancestor, then `starts_with` the
  canonical root), the disk state of what a path resolves to, and the §7 apply mechanics (temp
  beside the target, mode copy, rename, delete, lock via temp + rename).
- `Cache` is the git cache directory of §9.3: its home from `$HOME`, the per-URL directory, a
  temporary repository directory, and the rename into `<rev>.git` that settles a race with
  `DirectoryNotEmpty`. `process/git` (plan 11) drives `git` and calls `Cache` for every disk step,
  so `std::fs` stays in `fs` and `std::process` stays in `process`. This is the one allowed edge from
  one adapter to another (spec §2, X3: `process` may use `fs::cache` and nothing else from `fs`). The
  process id in a temp name is therefore passed in by the caller, never read here.

It is translation only (`docs/architecture.md` "Where logic lives"): every decision about a step is
plan 06's, taken on the `Resolution` this adapter returns. The workspace is a folder, one job per
file; the cache is one file because it has one job.

```text
src/fs/
├─ mod.rs            docs + mod / pub use (export-only)
├─ disk.rs           StdFileTree (built, unchanged)
├─ cache.rs          Cache: the git cache directory (§9.3)
└─ workspace/
   ├─ mod.rs         export-only
   ├─ project.rs     StdWorkspace: the canonical root, the two project files, path-source trees; impl Workspace
   ├─ resolve.rs     a project path → its canonical target and whether the path itself is a symlink (§6.3)
   ├─ state.rs       the DiskState of a canonical target (§6.2 "Disk states")
   ├─ stage.rs       the temp name beside a target; write and remove a temp (§7 step 2)
   └─ swap.rs        rename a temp onto its target with the old mode; delete; write the lock (§7 steps 4-5)
```

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6): `mod-rs-export-only.md`
(`fs/mod.rs`, `fs/workspace/mod.rs`); `modularity.md` Rule 1 (one job per file above) and Rule 2
(no fs copy of `Resolution`, `DiskState`, `DiskFile`, `Link`, `FetchError`); `static-dispatch.md`
(concrete adapters, no trait objects; `StdWorkspace::Tree` is `StdFileTree`); `strong-types.md`
(paths are `OpPath` / `LocalPath` / `GitUrl` / `CommitId`, never bare strings at the boundary);
`unit-test-mandate.md` (every file has colocated tests; disk tests on `tempfile` dirs, spec §14 fs
row); `doc-comment-discipline.md`.

**Tech Stack:** Rust 2024, crate `agentstacks`; `std::fs`, `std::os::unix::fs::{symlink,
PermissionsExt}` under `cfg(unix)`; `tempfile` (dev-dependency, unchanged, spec §15); the built
`ContentHash::of_bytes`, `StdFileTree`, `utf8_text`; plan 06's `DiskFile::new(bytes, mode)`; plan
07's cache path rule; plan 02's `FetchError`. No new dependency.

**Content authority:** spec §6.3 (all), §6.2 "Disk states" and the symlink row of the second table,
§7 (steps 2, 4, 5, "Modes", failure rows as I/O errors handed back), §9.3 (all), §14 fs row, §15
(`tempfile` stays a dev-dependency), §16 R1, R10, R17 (what apply locks after a failure, not
done here), R21, X3.

---

## Files that change

```
crates/agentstacks/src/fs/mod.rs                 — modify: docs (it now writes, and holds the cache); mod + pub use StdWorkspace, Cache
crates/agentstacks/src/fs/cache.rs               — create: Cache
crates/agentstacks/src/fs/workspace/mod.rs       — create: export-only
crates/agentstacks/src/fs/workspace/project.rs   — create: StdWorkspace, impl Workspace
crates/agentstacks/src/fs/workspace/resolve.rs   — create: the §6.3 resolution
crates/agentstacks/src/fs/workspace/state.rs     — create: the disk state of a target
crates/agentstacks/src/fs/workspace/stage.rs     — create: temp naming, stage, discard
crates/agentstacks/src/fs/workspace/swap.rs      — create: swap with mode copy, delete, lock write
```

`fs/disk.rs` is not changed. Its module doc's line "It is the only `std::fs` caller in the crate"
stays true of the `fs` module, and `fs/mod.rs`'s doc is updated to say `fs` now also writes.

## Order of work

### Task 1 — Resolve a project path by the one inside rule

**Files:**
- Create `crates/agentstacks/src/fs/workspace/resolve.rs` — the resolution.
- Create `crates/agentstacks/src/fs/workspace/project.rs` — `StdWorkspace::open` (only the root for now).
- Create `crates/agentstacks/src/fs/workspace/mod.rs` — export-only.
- Modify `crates/agentstacks/src/fs/mod.rs` — `pub mod workspace;` plus `pub use workspace::StdWorkspace`.

**Change:** `StdWorkspace::open(root)` canonicalises the project directory once and keeps it as
the canonical root (§6.3 step 3); a root that cannot be canonicalised is the platform's error.
`resolve_target(root, relative)` (crate-private, in `resolve.rs`) returns either "outside" or the
canonical target plus `Link` (`Direct` / `Symlink`, plan 06), in the order of §6.3:

1. When the path itself is a symlink (`symlink_metadata`), it is followed with `canonicalize`; the
   link is `Link::Symlink`. A dangling link (`canonicalize` gives `NotFound`) is reported as
   dangling, which the state step turns into `NotRegular` (§6.3 step 1).
2. Otherwise the deepest existing ancestor is canonicalised: `canonicalize` is tried on the path,
   then on each parent while it answers `NotFound` (`probes/round3-results.md` S5), and the
   components below it are appended unchanged. Any other error is returned as is.
3. The result must `starts_with` the canonical root, else "outside" (§6.3 step 3; the
   `canonicalize` + `starts_with` check of `probes/round2-results.md`).

Paths compare by bytes; nothing is case-folded (§18).

**Proof:** written first, seen failing, then passing (all on `tempfile` dirs, symlink cases under
`cfg(unix)`).
- `a_plain_file_resolves_to_itself` — `a/b.md` → `Direct`, target `<canonical root>/a/b.md`.
- `a_missing_path_resolves_through_its_deepest_existing_ancestor` — `new/dir/x.md` with only the
  root existing → `Direct`, target `<canonical root>/new/dir/x.md`.
- `an_inside_symlink_is_followed` — `link.md` → `real.md` → `Symlink`, target `<root>/real.md`.
- `escape_is_refused_for_a_symlinked_file` — `link` → a file in another temp dir → outside (§6.2
  checked example "a symlink to `/etc/hosts`", with a temp file standing in for `/etc/hosts`).
- `escape_is_refused_through_a_symlinked_directory` — `d` → another temp dir; both `d/x.md`
  (existing) and `d/new.md` (missing, via the deepest-ancestor step) → outside.
- `a_dangling_symlink_is_reported_dangling` — `gone` → a missing target → dangling, `Symlink`.
- `a_symlinked_root_is_canonicalised_once` — a workspace opened through a symlink to the project
  resolves `a.md` inside (the root comparison uses the canonical root).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): resolve project paths inside the project`.

### Task 2 — Report the disk state and the project files

**Files:**
- Create `crates/agentstacks/src/fs/workspace/state.rs` — the disk state.
- Modify `crates/agentstacks/src/fs/workspace/project.rs` — `impl Workspace for StdWorkspace` for
  `project_text`, `lock_text`, `resolve`, `source_tree` (the writing methods land in Tasks 3-4).
- Modify `crates/agentstacks/src/fs/workspace/mod.rs`.

**Change:** `disk_state(target)` (crate-private) reads what a canonical target holds (§6.2 "Disk
states"): nothing there → `DiskState::Absent`; a regular file (following links) →
`DiskState::File(DiskFile::new(bytes, mode))`, the mode being the Unix permission bits (§7 "Modes");
anything else (directory, socket, device) → `DiskState::NotRegular`. The mode is kept so `swap`
can restore it; a mode-only change is not "changed" (R28(i): content hash only), which is the
comparing flow's rule, not this adapter's. `StdWorkspace::resolve(path)`
combines Task 1 and this: outside → `Resolution::Outside`; dangling → `Inside { Symlink,
NotRegular }`; else `Inside { link, state }`. A read error other than `NotFound` is returned.

`project_text` reads `<root>/agentstacks.toml` through `utf8_text` (so non-UTF-8 is the shared
`not valid UTF-8` error); `lock_text` reads `<root>/agentstacks.lock`, `NotFound` → `None` (§8 row
1). `source_tree(dir)` resolves `<root>/<dir>` with the Task 1 rule (§6.3 last paragraph): outside →
`None`, else `Some(StdFileTree::new(<root>/<dir>))`, so the pack's messages keep the declared path
under the project root (formats `<root>/<rel>` form) and `StdFileTree` reads through a symlinked
root as it already does (`a_symlinked_root_is_still_read`, `fs/disk.rs`).

**Proof:**
- `resolve_reports_absent` — a missing path → `Inside { Direct, Absent }`.
- `resolve_reports_a_file_with_hash_and_mode` — bytes `b"a\n"` with mode 755 → `File` whose hash is
  `ContentHash::of_bytes(b"a\n")` and mode 755.
- `resolve_reports_a_directory_as_not_regular` — a directory → `NotRegular`.
- `resolve_reports_a_dangling_link_as_not_regular` — §14 "dangling link": `Inside { Symlink,
  NotRegular }`.
- `resolve_reads_through_an_inside_link` — `Inside { Symlink, File }` with the target's bytes; this
  is what makes doctor count "a symlink to one inside the project" as regular (§8 row 4).
- `resolve_marks_a_file_replaced_by_a_symlink` — §14 "symlink delete forgotten": a locked file
  replaced by a link to another inside file resolves with `Link::Symlink`, the input plan 06 turns
  into `forget` (§6.2 second table, row 2).
- `resolve_refuses_an_escape` — §14 "escape refused": a link out of the project → `Outside`.
- `lock_text_is_none_without_a_lock` — and the text when one exists.
- `project_text_refuses_bytes_that_are_not_utf8` — `not valid UTF-8`.
- `source_tree_inside_reads_the_directory` — a pack directory gives a tree whose `pack.toml`
  reads back; `source_tree` of a directory symlinked out of the project → `None`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): project disk state through the workspace`.

### Task 3 — Stage and discard temps

**Files:**
- Create `crates/agentstacks/src/fs/workspace/stage.rs` — temp naming, stage, discard.
- Modify `crates/agentstacks/src/fs/workspace/project.rs` — `stage`, `discard`.
- Modify `crates/agentstacks/src/fs/workspace/mod.rs`.

**Change:** `temp_of(target)` (crate-private, pure) names the temp `.<name>.agentstacks-tmp` in the
target's directory (§7 step 2). The target is the canonical one (Task 1; R21 "Apply temps: beside
the canonical target"), so the later rename stays inside one directory and a link inside the
project is written through (§6.3 "the write goes to its canonical target"). `stage(path, bytes)`
re-resolves the path. A path that resolves outside the project at apply time is refused (R21): the
`io::Error` it returns displays `resolves outside the project`, so apply reports exactly
`<path>: resolves outside the project`, and no temp is written outside the project. Otherwise it
creates the parents (`create_dir_all`, §7 step 2 "parents
created") and writes the bytes with `std::fs::write`, which truncates a leftover temp of the same
name (§7 step 2 "a leftover temp of the same name is overwritten") and gives a new temp the mode
`std::fs::write` creates (§7 "Modes", `probes/round3-results.md` S2). `discard(path)` removes that
temp; `NotFound` is success. Directories created while staging are left in place (§7 failure row "2
or 3": "Directories already created are left").

**Proof:**
- `temp_sits_beside_the_target_with_a_dot_name` — `<dir>/x.md` → `<dir>/.x.md.agentstacks-tmp`.
- `stage_creates_parents` — staging `a/b/c.md` in an empty project creates `a/b/` and the temp.
- `stage_overwrites_a_leftover_temp` — an existing longer temp is replaced byte for byte.
- `stage_through_an_inside_link_writes_beside_the_link_target` — the temp is in the real target's
  directory.
- `stage_refuses_a_path_that_now_resolves_outside` — the Display is `resolves outside the project`
  and the outside directory holds no temp.
- `discard_removes_the_temp_and_tolerates_none`.
- `discard_leaves_created_directories` — after staging `a/b/c.md` and discarding it, `a/b/` exists.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): stage apply temps beside their targets`.

### Task 4 — Swap, delete and write the lock

**Files:**
- Create `crates/agentstacks/src/fs/workspace/swap.rs` — swap, delete, lock write.
- Modify `crates/agentstacks/src/fs/workspace/project.rs` — `swap`, `delete`, `write_lock`.
- Modify `crates/agentstacks/src/fs/workspace/mod.rs`.

**Change:** `swap(path)` re-resolves (outside → the Task 3 refusal), and when the canonical target
is an existing regular file copies its permissions onto the temp (`set_permissions` with the old
`Permissions`), then renames the temp onto the target (§7 step 4; "Modes": a rename installs the
temp's mode, `S3 mode after rename over 755 file: 644`, so the old mode is copied first). A new or
restored file keeps the temp's `std::fs::write` mode (§7 "Modes" first bullet). Renaming onto the
canonical target keeps an inside link in place (`S4 rename onto canonical target: link2
is_symlink=true real content="B"`). `delete(path)` re-resolves (outside → the same refusal) and
removes the file; the directories it leaves empty stay (§7 "Directories a delete leaves empty are
left in place", §18). `write_lock(text)` writes `<root>/.agentstacks.lock.agentstacks-tmp` and
renames it onto `<root>/agentstacks.lock`, copying an existing lock's permissions first (§7 step 5
"via temp + rename"; R21: "The lock temp is `.agentstacks.lock.agentstacks-tmp`, and the lock keeps
its mode"). Every other failure is the platform's `io::Error`, handed back unchanged; the flows show
it as `<path>: <io::Error Display>` (§7 failure table, R1). Deciding what to remove and what to lock
after a failure is apply's (plan 14, §7 failure table, R17).

**Proof:**
- `swap_keeps_the_old_mode` — §14 "mode kept": a 755 file updated through stage + swap holds the new
  bytes and is still 755.
- `swap_gives_a_new_file_the_write_mode` — a created file has the same mode as a file freshly made
  with `std::fs::write` in the same directory (S2; compared, not hard-coded, because the umask
  decides it).
- `swap_writes_through_an_inside_symlink` — §14 "symlink written through": the link is still a
  symlink and its target holds the new bytes (S4).
- `swap_without_a_temp_fails_with_not_found` — the platform's `NotFound` comes back.
- `delete_removes_the_file_and_leaves_its_directory` — the parent directory stays, empty.
- `delete_refuses_a_path_that_now_resolves_outside` — the outside file still exists.
- `write_lock_replaces_the_lock_and_leaves_no_temp` — the lock holds the text byte for byte; no
  `.agentstacks.lock.agentstacks-tmp` remains; an existing 600 lock stays 600.
- `staged_then_swapped_matches_the_fake` — the sequence of the plan 09 fake test
  `stage_then_swap_installs_bytes_and_keeps_mode` gives the same bytes and modes on disk (the
  contract test: the real adapter behaves like its fake, `docs/architecture.md` "Testing").

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): swap staged files and write the lock`.

### Task 5 — Fill the git cache safely

**Files:**
- Create `crates/agentstacks/src/fs/cache.rs` — `Cache`.
- Modify `crates/agentstacks/src/fs/mod.rs` — `pub mod cache;` plus `pub use cache::Cache`.

**Change:** `Cache` is the directory of §9.3:

- `Cache::from_home(home: Option<&OsStr>)` → `Result<Cache, FetchError>`: calls plan 07's
  `cache_home(home)` and keeps its answer; the root is `<home>/.airsstack/cache` (D10). `cache_home`
  owns the rule: a `HOME` that is unset or empty is "not set" (R10) and gives the unlocated
  `FetchError::HomeUnset`, which displays (after its `agentstacks.toml` location)
  `HOME is not set, so the cache (~/.airsstack/cache) has no home`. Reporting it once per run is the
  flows' (R10 "reported once"). `Cache::from_env()` passes `std::env::var_os("HOME")`.
- `repo(url, rev)` → `Result<Option<PathBuf>, FetchError>`: `<rev>.git` under the URL directory when
  that directory exists, so a locked rev resolves without network (§9.3 "Layout"). The paths come
  from plan 07's `url_dir` and `rev_dir`; `Cache` re-implements none of plan 07's rules.
- `temp(url, name)` → `Result<PathBuf, FetchError>`: creates the URL directory and returns
  `<url dir>/<name>`, where `name` is plan 07's `temp_name(pid, n)` (`tmp-<pid>-<n>`) built by the
  caller from its process id and counter (§9.3 "Filling it" step 1). A leftover entry of that name
  is removed and the name reused (R21 "A leftover cache temp of the same name is removed and
  reused").
- `install(temp, url, rev)` → `Result<PathBuf, FetchError>`: renames the temp to `<rev>.git` (§9.3
  step 2). `ErrorKind::DirectoryNotEmpty` means another run cached it first (`S1 rename dir onto
  non-empty dir: ERR kind=DirectoryNotEmpty`): the temp is removed and the existing directory is
  returned.
- `discard(temp)` → `Result<(), FetchError>`: removes a temp after a failed fetch; `NotFound` is
  success. Temps of a killed process are never cleaned (§9.3 last bullet, §18).

Every other I/O failure here is R1: plan 02's cache I/O `FetchError` carrying the path involved and
the `io::Error`, shown as `<path>: <io::Error Display>` at that path. The path is the directory being
created for `temp`, the temp being removed for `temp`, `install` and `discard`, and the
`<rev>.git` target for a failed rename (R28(b)). `process/git` (plan 11) passes it on unchanged (X3:
`process` may use `fs::cache` and nothing else from `fs`).

**Proof:**
- `no_home_is_refused` — `from_home(None)` and `from_home(Some(""))` both give `HomeUnset`; its
  Display is exactly `HOME is not set, so the cache (~/.airsstack/cache) has no home` (R10).
- `a_cache_io_failure_names_its_path` — on a `tempfile` home whose `.airsstack` is a regular file,
  `temp` fails with the R1 cache variant whose path is the URL directory and whose Display is
  `<that path>: <io::Error Display>`.
- `a_leftover_temp_is_removed_and_reused` — an existing `tmp-<pid>-<n>` directory holding a file is
  gone after `temp` returns the same path (R21).
- `layout_follows_the_url_hash_and_rev` — on a `tempfile` home, `repo` of an installed rev is
  `<home>/.airsstack/cache/git/<hex sha256 of url>/<rev>.git`; an uninstalled rev is `None`.
- `temp_creates_the_url_directory` — and returns `<url dir>/tmp-<pid>-<n>` for the given name.
- `install_renames_the_temp_into_place` — the temp is gone, `<rev>.git` holds its files.
- `install_race_uses_the_existing_directory` — §14 "cache rename race": with `<rev>.git` already
  holding a file, installing a second temp returns the existing path, keeps the existing file and
  removes the second temp.
- `install_onto_an_empty_directory_replaces_it` — `S1 rename dir onto empty dir: OK`.
- `discard_removes_a_temp_and_tolerates_none`.

**Done when:** `cargo make dod` passes (end of plan) and the Definition of Done is green.
Commit `feat(agentstacks): git cache directory with race-safe install`.

---

## Risks

- **`DirectoryNotEmpty` only.** POSIX also allows `EEXIST` for a rename onto a non-empty directory.
  Linux and macOS give `ENOTEMPTY` (S1 is macOS). The spec names `DirectoryNotEmpty`, and the plan
  follows it; another errno would surface as an R1 I/O failure.
- **Time of check versus time of use.** Resolution, staging and renaming are separate system calls;
  a link swapped in between is caught by apply's state check (§7 step 3) except in the instant
  between that check and the rename. The spec accepts this (§7 order); the module docs say so, as
  `fs/disk.rs` already does for reads.
- **Riskiest task: Task 1.** The deepest-ancestor walk with `canonicalize` decides every
  inside/outside answer. A dangling symlink as an ancestor directory resolves as missing below its
  parent; creating parents through it then fails with the platform's error at staging, which apply
  reports (§7 row "2 or 3").
- **Unix only for modes.** Mode bits are read with `PermissionsExt` under `cfg(unix)`; CI is
  `ubuntu-latest` and development is macOS. Copying `Permissions` in `swap` is portable.
- Option not taken: canonical paths in the lock. The lock keeps the proposed path (§6.3); the
  canonical target never leaves this module.

## Proof

- `cargo make dod` is green at the end of the plan.
- `tempfile` stays in `[dev-dependencies]` (`crates/agentstacks/Cargo.toml`, §15).
- `grep -rn "std::process" crates/agentstacks/src/fs` finds nothing.
- Spec rows covered: §6.3 steps 1-3, write-through, path-source paragraph (Tasks 1, 2); §6.2 "Disk
  states" and the second table's symlink input (Task 2); §7 step 2 incl. parents and leftover temp
  (Task 3), step 4 and "Modes" both bullets (Task 4), step 5 (Task 4), "Directories a delete leaves
  empty are left in place" (Task 4), failure rows as I/O errors handed back (Tasks 3, 4); §8 row 4's
  input (Task 2); §9.3 layout, filling steps 1-2, race, HOME, killed-process temps (Task 5); §14 fs
  row: mode kept (`swap_keeps_the_old_mode`), symlink written through
  (`swap_writes_through_an_inside_symlink`), symlink delete forgotten
  (`resolve_marks_a_file_replaced_by_a_symlink`), escape refused (`resolve_refuses_an_escape`,
  Task 1 escapes), dangling link (`resolve_reports_a_dangling_link_as_not_regular`), cache rename
  race (`install_race_uses_the_existing_directory`). §16: R1 (`a_cache_io_failure_names_its_path`,
  I/O errors handed back in Tasks 2-4), R10 (`no_home_is_refused`), R21 (temps beside the canonical
  target, Task 3; lock temp name and mode, `write_lock_replaces_the_lock_and_leaves_no_temp`;
  outside at apply, `stage_refuses_a_path_that_now_resolves_outside` and
  `delete_refuses_a_path_that_now_resolves_outside`; leftover cache temp,
  `a_leftover_temp_is_removed_and_reused`), X3 (only `fs::cache` is used by `process`).

## Review findings

Reviewer, one round: code 2🔴 3🟡 10🔵, plan 1🔴 1🟡 4🔵. The reviewer probed with a throwaway crate on this worktree. Reviewer `cargo test -p agentstacks fs::`: 70 passed. Blocking: `resolve.rs:66`, `project.rs:110`, `project.rs:87`, plan Task 2. The fix coder also took `project.rs:162`.

- fixed 🔴 (blocking, security) `src/fs/workspace/resolve.rs:66`: `..` in the existing prefix was resolved by name, so with `d -> <outside>/sub`, `source_tree("d/../packs")` read `<outside>/packs`.
  - Fix: the existing prefix is canonicalised through the filesystem. Only the missing suffix is applied by name, and a suffix that climbs above the root is Outside. `source_tree` hands `StdFileTree` the checked canonical target.
  - A second hole of the same kind was closed: a link reached as `d/../ghost` is now Outside, via `link_location` and `Located::Dangling { at }`.
  - Tests:
    - `a_parent_component_is_resolved_through_the_real_directory`
    - `a_missing_suffix_that_climbs_above_the_root_is_outside`
    - `source_tree_refuses_a_parent_component_that_climbs_out_through_a_link`
    - `a_symlink_reached_through_an_outside_directory_is_outside`
- fixed 🔴 (blocking) `project.rs:110`: a missing `agentstacks.toml` printed `entity not found`.
  - Fix: `project_text` passes the error from `std::fs::read` through.
  - The test now asserts the Display `No such file or directory (os error 2)`.
- fixed 🟡 (blocking) `project.rs:87`: a link to the project root staged a temp file beside the root, which is outside the project.
  - Fix: `target_of` refuses with InvalidInput "resolves to the project directory", and `discard` touches nothing.
  - Test: `a_link_to_the_project_root_is_never_staged_swapped_or_discarded`.
- fixed 🟡 `project.rs:162`: `delete` followed a final symlink and removed the user's target file.
  - Fix: the link itself is removed, matching `FakeWorkspace::delete`.
  - Tests:
    - `delete_removes_a_link_and_never_its_target`
    - `delete_removes_a_dangling_link`
    - `delete_refuses_a_link_that_now_resolves_outside_by_its_directory`
- fixed 🟡 `project.rs:85-93`: the dangling-link NotFound branch was untested. Test: `a_dangling_link_is_refused_by_stage_and_swap_with_not_found`.
- fixed 🔵 The symlink test modules are now gated `#[cfg(all(test, unix))]`.
- Fix verification:
  - `cargo test -p agentstacks --lib fs::`: 81 passed, 0 failed. `fs::` doctests: 4 passed.
  - clippy `--all-targets -D warnings`: clean. rustfmt: clean.
  - All of this ran in a scratch copy, because the live crate was red in `src/host`, which was mid-edit.
  - With the fixes reverted, 7 tests failed.
- open 🔵 Nits:
  - `state.rs:45-48`: the non-unix `mode_of` invents 0o644.
  - `swap.rs:28`: `if let Ok(..)` hides a non-NotFound metadata error.
  - `cache.rs:142-147`: untested branches. `cache.rs:184`: a wrong helper doc.
  - `lib.rs:19`: still says fs only reads.
  - `stage.rs:22`: `unwrap_or_default` on `file_name`.
  - `a_symlinked_root_is_canonicalised_once` repeats the plain-file case.
  - Task 1's `resolve_target` is named `locate`.
  - `StdWorkspace::root()` is public without a plan basis.
- open 🔵 Cross-plan: `fs/mod.rs:4` says fs is the only module calling `std::fs`, but `process/binary.rs:79` (plan 11) calls `std::fs::metadata`.

## Deviations

- 2026-11-22: A dangling symlink at stage or swap returns io `NotFound` "symlink leads nowhere". `discard` of a dangling or outside path is `Ok`.
- 2026-11-22: Crate-private items use `pub(super)`, because clippy `redundant_pub_crate` rejects `pub(crate)` inside private modules.
- 2026-11-22: The coder's "`..` resolved lexically" was rejected by review, and the fix replaced it with filesystem canonicalisation of the existing prefix.
- 2026-11-22: A link to the project root is refused at stage and swap with InvalidInput "resolves to the project directory". The plan is silent on this.
- 2026-11-22: `delete` removes a final-component symlink itself, dangling or not.

### Crate-wide cross-check (after plans 09-17)

- fixed 🟡 (security) `src/fs/cache.rs`: the public `Cache::discard(&Path)` let any library caller delete any directory recursively.
  - Fix: `Cache` and its methods are `pub(crate)`, with no public re-export. The dead `Cache::from_env` was removed.
  - `discard` is now an associated function, because clippy `unused_self` fires once the type is crate-private. Its doctest was dropped, since unit tests cover it.
  - `cargo make dod`: green.
- open ❓ `fs/workspace/project.rs:135-143`: a path source's tree is rooted at the canonical absolute path, so its load messages show absolute paths, where `FakeWorkspace` shows relative ones.
- open ❓ `port/workspace/seam.rs:24-26`: paths compare by bytes, so on case-insensitive APFS two proposals that differ only in case share one target.
