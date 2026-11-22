---
status: done
created: 2026-11-22
depends-on: [02, 04]
built-on: f39b2a814319
---

# Steps Implementation Plan

**Goal:** decide what happens to each path.

**Architecture:** everything here is a pure rule in `types/plan` (spec §2, D13): plain values in,
one answer out, no port and no I/O. The disk facts arrive as values that `Workspace` (plan 09) will
produce: a `Resolution` (inside the project with a `DiskState`, or outside; either way whether the
path itself is a symlink, §6.3, §16 R15) and a read record (`Reads`, what `agentstacks.read`
returned, §4.3). Two decide functions turn a proposed path or a locked-but-no-longer-proposed path
into a `Step` or a located refusal (§6.2, §16 R15). A filter keeps only a removed tool's shared
writes (§6.4). One builder turns the steps into the new lock's `LockContents`, either after a full
apply (§7 "Lock content") or after a swap failure with only the landed paths (§16 R17, called by plan
14). `Plan` is the one canonical value the flows hand around (§6.1, modularity Rule 2), with its
notes worded per §9.1 and §16 R20. The doctor rule (§8, §16 R23) maps one lock entry plus its
`Resolution`, and one airsl answer, to `Finding`s. Refusals are `StepError` variants (plan 02)
wrapped in a `Diagnostic` located at the project-relative path (`Location::file`, §13 row `<path>`).
Operation values (`Operation`, `Proposal`, `Origin`, `OpPath`, owner derivation) are plan 04's and
are used, never copied. This module imports nothing from `crate::port` (§2 dependency rule, §16 X2).
No step is decided after a refusal in §6.1 steps 1–7 (§16 R7); plan 13 enforces that order, and the
decide functions here only answer for the inputs they are given.

**Tech Stack:** Rust 2024, `crates/agentstacks` only; existing types `ContentHash`
(`src/types/tree/hash/content.rs`), `FileEntry`, `LockContents`, `LockedEntry`, `Lockfile`,
`Owner`, `check_airsl_presence` (`src/types/lock/*`), `SubPath` (`src/types/project/path/sub.rs`),
`PackVersion`, `PackManifest`, `PackName`, `ToolName`, `BinaryName`, `Diagnostic`, `Location`; no new
dependency.

**Content authority:** spec §6.2 (both tables and the checked examples), §6.3 (the decision given a
resolution), §6.4, §7 step 3 and "Lock content", §8, §13 (locations), §16 R1, R2, R7, R8, R15, R16,
R17, R18, R20, R21, R23, R27.

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6): `mod-rs-export-only.md` (the
`types/plan/mod.rs` and `types/plan/decide/mod.rs` hold only docs, `mod` and `pub use`);
`modularity.md` Rule 1 (one job per file below) and Rule 2 (`Step`, `Plan` exist only here);
`strong-types.md` (no `bool` parameter for a decision: `Resolution` carries a `Link` enum and the
builder a `Landed` enum; paths are `OpPath`/`SubPath`, hashes `ContentHash`); `unit-test-mandate.md`
(every logic file carries `#[cfg(test)] mod tests`); `doc-comment-discipline.md` (doc comments never
name specs, plans, chains or agents; doctests build plain values and never use a `#[cfg(test)]` fake,
§16 R24 plan-level note); `static-dispatch.md` (no trait objects; nothing here is generic over a
port).

---

## Files that change

```
crates/agentstacks/src/types/plan/mod.rs               — modify: add mod / pub use lines for the files below (docs stay)
crates/agentstacks/src/types/plan/disk.rs              — create: DiskFile, DiskState, Link, Resolution
crates/agentstacks/src/types/plan/action.rs            — create: Action
crates/agentstacks/src/types/plan/reads.rs             — create: ReadOutcome, Reads, the shared rule
crates/agentstacks/src/types/plan/step.rs              — create: Step
crates/agentstacks/src/types/plan/decide/mod.rs        — create: export-only
crates/agentstacks/src/types/plan/decide/proposed.rs   — create: decide a proposed path (§6.2 first table, §6.3, R15)
crates/agentstacks/src/types/plan/decide/unproposed.rs — create: decide a locked path no longer proposed (§6.2 second table, R15)
crates/agentstacks/src/types/plan/removed.rs           — create: removed-tool filter (§6.4)
crates/agentstacks/src/types/plan/lock.rs              — create: lock-content builder (§7, R17) and the airsl pin rule
crates/agentstacks/src/types/plan/binary.rs            — create: BinaryCheck
crates/agentstacks/src/types/plan/note.rs              — create: Note (§9.1, R20), AirslPin (§10)
crates/agentstacks/src/types/plan/built.rs             — create: Plan
crates/agentstacks/src/types/plan/airsl.rs             — create: AirslAnswer (R1, R2)
crates/agentstacks/src/types/plan/finding.rs           — create: Finding (§8, R23 texts)
crates/agentstacks/src/types/plan/drift.rs             — create: the doctor rules (§8, R23)
crates/agentstacks/src/types/plan/race.rs              — create: Race, the apply step-3 comparison (§7, R15, R21)
```

`src/types/plan/error.rs` (`StepError`) belongs to plan 02 and is not touched here.

## Order of work

### Task 1 — Model the disk facts a step is decided on

**Files:**
- Create `crates/agentstacks/src/types/plan/disk.rs` — the disk-state and resolution values.
- Create `crates/agentstacks/src/types/plan/action.rs` — the step actions.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod disk; mod action;` plus `pub use`.

**Change:** `DiskFile` holds a regular file's bytes, its `ContentHash` (computed with
`ContentHash::of_bytes`, so hash and bytes cannot disagree; the constructor takes only bytes and
mode) and its permission mode (kept for apply, §7 "Modes"). `DiskState` is `Absent`,
`File(DiskFile)` or `NotRegular` (§6.2 "Disk states"). `Link` is `Direct` or `Symlink`: whether the
path itself is a symlink. `Resolution` is `Inside { link: Link, state: DiskState }` (the state of
the canonical target; a dangling symlink is `Inside { link: Symlink, state: NotRegular }`, §6.3 step
1) or `Outside { link: Link }` (§6.3 step 3). `Outside` keeps `link` because a locked path no longer
proposed that is now a symlink pointing outside is `forget`, checked first (§16 R15). `Action` is
`Create`, `Update`, `Restore`, `Unchanged`, `Delete`, `Forget` (§6.2 step names), with
`Action::writes()` true for create, update and restore (the three apply stages, §7 step 2) and
`Action::touches_disk()` true for those three and `Delete` (§16 R17: forget and unchanged never touch
disk).

**Proof:** written first, seen failing, then passing.
- `disk_file_hash_matches_its_bytes` — a `DiskFile` built from `b"a\n"` reports the
  `ContentHash::of_bytes(b"a\n")` hash and the given mode.
- `action_writes_only_for_create_update_restore` — `writes()` is true for exactly `Create`,
  `Update`, `Restore`.
- `action_touches_disk_except_unchanged_and_forget` — `touches_disk()` is false for exactly
  `Unchanged` and `Forget`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): disk state and step actions`.

### Task 2 — Record what `read` returned and decide "shared"

**Files:**
- Create `crates/agentstacks/src/types/plan/reads.rs` — the read record and the shared rule.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod reads;` plus `pub use`.

**Change:** `ReadOutcome` is `Absent` or `Bytes(ContentHash)`: what one `agentstacks.read` returned
(§4.3 "The core records what each read returned (the hash, or "absent")"). `Reads` is one tool's
record for one run, keyed by `OpPath` (the normalised path, §3.3 "Normalisation"); a second read of
the same path keeps the first outcome. `Reads::shared(path, declared)` returns the recorded outcome
when the path is in the tool's declared `reads` (given as normalised `OpPath`s) **and** was read in
this run, else `None` (§6.2 definition of **Shared**).

**Proof:**
- `shared_needs_declared_and_read` — declared and read → `Some(outcome)`; declared but not read →
  `None`; read record present but path not declared → `None`.
- `reads_keeps_the_first_outcome_of_a_path` — recording `Absent` then `Bytes(h)` for one path keeps
  `Absent`.
- `reads_key_is_the_normalised_path` — a read of `./.claude//settings.json` is found by
  `.claude/settings.json` (both built through `OpPath`'s constructor, plan 04).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): read record and the shared rule`.

### Task 3 — Decide the step for a proposed path

**Files:**
- Create `crates/agentstacks/src/types/plan/step.rs` — `Step`.
- Create `crates/agentstacks/src/types/plan/decide/mod.rs` — export-only.
- Create `crates/agentstacks/src/types/plan/decide/proposed.rs` — the first-table decision.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod step; mod decide;` plus `pub use`.

**Change:** `Step` holds `path: OpPath`, `action: Action`, `owner: Option<Owner>` (`None` for a
shared file, which has no `by`, §3.1), `origin: Option<Origin>` (`None` for a delete or forget, which
the core makes from the lock), `new: Option<Vec<u8>>` (the bytes to install; `None` for delete and
forget), `old: Option<Vec<u8>>` (a shared file's disk bytes, kept for the diff, §6.1), and
`decided_on: Option<DiskState>` (the state apply re-checks, §7 step 3; for a shared file it equals
what `read` returned, because only that case is accepted, §16 R15). It is `None` for a `forget`,
including a forget of an outside or dangling symlink: a forget never touches disk, so apply never
re-checks it.

`decide_proposed(ProposedPath) -> Result<Step, Diagnostic>` where `ProposedPath` gathers: the
`OpPath`, the new bytes, the `Origin`, the owner already derived by plan 04's owner rule, the
`Option<ReadOutcome>` from `Reads::shared`, the lock hash if the path has a `[[file]]` entry, and
the `Resolution`. Rows, in this order (§6.3, then §6.2 first table with the §16 R15 addition):

1. `Outside` → refused `<path>: resolves outside the project` (§6.3 step 3).
2. `NotRegular` (any sharing, any lock) → refused `<path>: is not a regular file` (§6.2 row 1).
3. shared, disk absent and read returned absent → `create`, owner `None` (row 2).
4. shared, disk file whose hash equals what read returned → `update`, or `unchanged` when the new
   bytes equal the disk bytes; `old` = disk bytes; owner `None` (row 3).
5. shared, any other combination (disk absent but read returned bytes, disk present but read
   returned absent, or disk hash ≠ read hash) → refused
   `<path>: changed after it was read; run plan again` (§16 R15).
6. not shared, locked, absent → `restore` (row 4).
7. not shared, locked, disk hash = lock hash → `update`, or `unchanged` when new = disk (row 5).
8. not shared, locked, disk hash ≠ lock hash, disk bytes = new bytes → `unchanged` (row 6).
9. not shared, locked, disk hash ≠ lock hash, disk bytes ≠ new bytes → refused
   `<path>: changed since the last apply` (row 7).
10. not shared, not locked, absent → `create` (row 8).
11. not shared, not locked, disk bytes = new bytes → `unchanged` (row 9, adopted).
12. not shared, not locked, disk bytes ≠ new bytes → refused
    `<path>: exists and was not written by agentstacks` (row 10).

Every refusal is a `Diagnostic` at `Location::file(<normalised path>)` carrying the matching
`StepError` variant (plan 02), so it displays exactly as quoted. A proposed path that is an inside
symlink is decided on its canonical target's state (written through, §6.3); `link` plays no part
here.

**Proof:** one test per row; each refusal test asserts the whole `Diagnostic` display verbatim.
- `proposed_outside_is_refused` — `Outside` → `.claude/x: resolves outside the project`.
- `proposed_not_regular_is_refused` — `NotRegular`, tried shared, locked and unlocked →
  `.claude/x: is not a regular file` each time.
- `shared_absent_read_absent_creates` — action `Create`, owner `None`, `old` `None`.
- `shared_equal_to_read_updates_with_old_bytes` — disk = read, new differs → `Update`, `old` = disk
  bytes, owner `None`.
- `shared_equal_to_read_and_new_is_unchanged` — new = disk → `Unchanged`, owner `None`.
- `shared_changed_after_read_is_refused` — three inputs (absent vs read bytes, file vs read absent,
  file hash ≠ read hash) each give `.claude/settings.json: changed after it was read; run plan again`.
- `locked_absent_restores` — `Restore` with the derived owner.
- `locked_hash_equal_updates` — `Update`; and `Unchanged` when new = disk.
- `locked_hash_differs_bytes_equal_new_is_unchanged` — `Unchanged`.
- `locked_hash_differs_bytes_differ_is_refused` — `.claude/x: changed since the last apply`.
- `unlocked_absent_creates` — `Create`.
- `unlocked_equal_bytes_is_adopted_unchanged` — `Unchanged`, owner kept (it will be locked, §6.2
  row 9 "adopted into the lock").
- `unlocked_different_bytes_is_refused` — `.claude/x: exists and was not written by agentstacks`.
- `proposed_inside_symlink_uses_the_target_state` — `Inside { link: Symlink, state: Absent }`, not
  locked → `Create`.
- `step_keeps_disk_state_it_was_decided_on` — `decided_on` is `Some` of the input state for every
  accepted row.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): decide the step for a proposed path`.

### Task 4 — Decide the step for a locked path no longer proposed

**Files:**
- Create `crates/agentstacks/src/types/plan/decide/unproposed.rs` — the second-table decision.
- Modify `crates/agentstacks/src/types/plan/decide/mod.rs` — `mod unproposed;` plus `pub use`.

**Change:** `decide_unproposed(path: OpPath, entry: &FileEntry, resolution: &Resolution) ->
Result<Step, Diagnostic>` (§6.2 second table with §16 R15). The caller (plan 13) passes the entry's
path as an `OpPath`: a lock `[[file]].path` never contains `\` (§16 R16, refused when the lock is read,
plan 03), so it is a valid op path shape. The step's owner is the entry's `by`, origin `None`, `new`
`None`; `decided_on` is `Some(state)` for a `delete` and `None` for a `forget`. Rows, in this order:

1. `link` is `Symlink`, whether `Inside` (dangling included) or `Outside` → `forget` (§16 R15: "now a
   symlink … is `forget`, checked before any other row"; §6.2 row 2).
2. `Outside` (a direct path) → refused `<path>: resolves outside the project` (§6.3).
3. `Absent` → `forget` (row 1).
4. file hash = lock hash → `delete` (row 3).
5. file hash ≠ lock hash → refused `<path>: changed since the last apply; not deleting it` (row 4).
6. `NotRegular` → refused `<path>: is not a regular file` (row 5).

**Proof:**
- `unproposed_symlink_forgets_first` — a symlink to an inside regular file whose hash matches, a
  dangling symlink (`Inside { Symlink, NotRegular }`) and a symlink pointing outside
  (`Outside { Symlink }`) each give `Forget` with `decided_on` `None`.
- `unproposed_absent_forgets` — `Forget`.
- `unproposed_hash_equal_deletes` — `Delete`, owner = the entry's `by`, `decided_on` `Some` of the
  file state.
- `unproposed_hash_differs_is_refused` — `.claude/x: changed since the last apply; not deleting it`.
- `unproposed_not_regular_is_refused` — `.claude/x: is not a regular file`.
- `unproposed_outside_is_refused` — `Outside { Direct }` → `.claude/x: resolves outside the project`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): decide the step for a path no longer proposed`.

### Task 5 — Prove the checked examples

**Files:**
- Test `crates/agentstacks/src/types/plan/decide/proposed.rs` — the proposed-path examples.
- Test `crates/agentstacks/src/types/plan/decide/unproposed.rs` — the removed-pack example.

**Change:** none beyond tests: each row of §6.2 "Checked examples" becomes a test built with the
example's own path and situation, through `decide_proposed` / `decide_unproposed`. (The same rows run
end to end through the flow in plan 13.)

**Proof:**
- `example_first_apply_creates` — `.claude/skills/status/SKILL.md` absent, not locked → `Create`.
- `example_user_edit_is_refused` — locked, disk edited, pack unchanged →
  `.claude/skills/status/SKILL.md: changed since the last apply`.
- `example_user_delete_restores` — locked, absent → `Restore`.
- `example_removed_pack_deletes` — `team-lint`'s locked file, untouched, no longer proposed →
  `Delete`.
- `example_settings_shared_update` — `.claude/settings.json` holds the user's permissions, read
  returned those bytes, proposal adds hooks → `Update`, `old` = the user's bytes, owner `None`.
- `example_settings_absent_shared_create` — `.claude/settings.json` absent, read returned absent →
  `Create`, owner `None`, `decided_on` `Absent` (new-file mode is apply's, §7).
- `example_foreign_agent_is_refused` — `.claude/agents/coder.md` exists, unlocked, different
  content → `.claude/agents/coder.md: exists and was not written by agentstacks`.
- `example_symlink_to_etc_hosts_is_refused` — proposed path resolving `Outside { Symlink }` →
  `<path>: resolves outside the project` with the example path filled in.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `test(agentstacks): checked step examples`.

### Task 6 — Keep only a removed tool's shared writes

**Files:**
- Create `crates/agentstacks/src/types/plan/removed.rs` — the removed-tool filter.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod removed;` plus `pub use`.

**Change:** `keep_shared_writes(proposals: Vec<Proposal>, reads: &Reads, declared: &[OpPath]) ->
Vec<Proposal>` keeps a proposal only when its `Operation` is a `Write` whose path is shared
(`Reads::shared` is `Some`); every other proposal (`Copy`, `Skip`, `RequireBinary`, a write to a
path not read) is dropped (§6.4 bullet 2), before the duplicate check sees it (§16 R18; plan 13
calls this filter first). The core writes for the removed tool's root are never made: that is the
flow's (plan 13), and this module documents it as a non-responsibility.

**Proof:**
- `removed_tool_keeps_only_shared_writes` — a write to a read path is kept; a write to an unread
  path, a copy, a skip and a require_binary are dropped.
- `removed_tool_without_reads_keeps_nothing` — an empty `Reads` drops every proposal.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): removed-tool filter`.

### Task 7 — Build the new lock contents, full or partial

**Files:**
- Create `crates/agentstacks/src/types/plan/lock.rs` — the lock-content builder and airsl pin rule.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod lock;` plus `pub use`.

**Change:** `airsl_pin(locked: impl IntoIterator<Item = &PackManifest>, installed:
Option<&PackVersion>) -> Option<PackVersion>` returns the installed version exactly when a locked
pack declares `requires.airsl` (§7 "Lock content" third bullet, §10 "The installed airsl version is
written to `[airsl] version`").

`Landed<'a>` is `All` (apply finished, §7 step 5) or `Only { paths: &'a BTreeSet<OpPath>, old:
&'a [FileEntry] }` (a step-4 failure, §16 R17: the paths whose rename or delete was performed, and the
old `[[file]]` entries of the lock the plan was made against, which `Plan::old_files` carries, Task
8). `lock_contents(tools, packs, steps: &[Step], airsl: Option<PackVersion>, landed:
Landed) -> Result<LockContents, InvalidSubPath>`:

- `tools`, `packs` (`LockedEntry` values with rev and tree, built by the flow) and `airsl` are
  carried as given, in both modes (§7 bullets 1 and 3; §16 R17 "`[[tool]]`, `[[pack]]` and `[airsl]`
  take the new values");
- **new entry** of a step: a `FileEntry` when the owner is `Some` and the action is `Create`,
  `Update`, `Restore` or `Unchanged`, with `by` = the owner and `hash` = `ContentHash::of_bytes(new)`
  (§7 bullet 2; a user who made the same edit gives the lock the new hash, §6.2 row 6, and an adopted
  file enters the lock, row 9). A shared step (owner `None`), a `Delete` and a `Forget` have no new
  entry, so a shared path's old lock entry is dropped (§6.2 row 3) and a removed tool's files leave
  the lock (§6.4 bullet 4);
- `Landed::All`: every step contributes its new entry;
- `Landed::Only` (§16 R17): `tools`, `packs` and `airsl` still take the new values; every `Forget`
  and every `Unchanged` step applies (a forget drops the entry, an unchanged step gives its new
  entry), because none touches disk; a step that touches disk (`Action::touches_disk`) contributes
  its new entry when its path is in `paths`, otherwise the `old` entry for that path, if any ("new
  entries for files that landed and old entries for those that did not");
- each `OpPath` becomes a `SubPath` through `SubPath::new`; `OpPath`'s shape rule (plan 04) is
  stricter than `SubPath`'s, so the error is unreachable for a valid `OpPath`, but it is returned,
  never unwrapped (workspace lint `unwrap_used`);
- the struct literal ends with `..LockContents::default()`, so it compiles before and after plan 03
  removes the `merges` field.

The flow builds a `Lockfile` from the result through `Lockfile::new` and runs
`check_airsl_presence` on that new lock only (§16 R27).

**Proof:**
- `lock_records_written_and_unchanged_steps` — create, update, restore and unchanged steps each
  give a `[[file]]` with the owner and the hash of the new bytes.
- `lock_leaves_out_shared_delete_and_forget` — those three give no entry, and a shared path's old
  entry does not survive.
- `lock_takes_the_new_hash_for_the_same_user_edit` — a §6.2 row-6 step records
  `ContentHash::of_bytes(new)`, not the old lock hash.
- `lock_contents_build_a_lockfile` — the contents (one tool, one pack, two files owned by them) pass
  `Lockfile::new`, and `write_lock` of it reparses with `Lockfile::parse`.
- `airsl_pinned_exactly_when_a_locked_pack_requires_it` — with a manifest declaring
  `requires.airsl` and installed `0.1.4` → `Some(0.1.4)`, and `check_airsl_presence` passes on the
  built lock; with no such manifest → `None`, and `check_airsl_presence` passes too.
- `partial_lock_keeps_old_entries_for_what_did_not_land` — steps update `a` (landed), update `b`
  (not landed, old entry kept with its old hash), create `c` (not landed, no old entry → absent),
  delete `d` (landed → absent), delete `e` (not landed → old entry kept).
- `partial_lock_applies_forget_and_unchanged` — with an empty `paths` set, a `Forget` of `f` drops its
  old entry, an adopted `Unchanged` of `g` gets its new entry, and a §6.2 row-6 `Unchanged` of `h`
  gets the hash of its new bytes instead of its old entry.
- `partial_lock_takes_new_tools_packs_and_airsl` — the old lock's tool rev, pack rev and airsl
  version are replaced by the given ones.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): new lock contents from the steps`.

### Task 8 — Hold the whole plan

**Files:**
- Create `crates/agentstacks/src/types/plan/binary.rs` — `BinaryCheck`.
- Create `crates/agentstacks/src/types/plan/note.rs` — `Note`, `AirslPin`.
- Create `crates/agentstacks/src/types/plan/built.rs` — `Plan`.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod binary; mod note; mod built;` plus `pub use`.

**Change:** `BinaryCheck { name: BinaryName, origin: Origin }` is one passed binary check; one per
requirement, so a binary required by a pack and by an extension appears once for each (§10). A
failed check is a refusal (plan 02 / plan 13), never a `BinaryCheck`. `Note` displays each plan note
verbatim:

| Variant | Display |
|---|---|
| `Ignored { root: PathBuf, path: PathBuf }` | `<pack>/<path>: ignored (dot entry)` (§16 R20; `<pack>` is the loaded directory's label, `Pack::dir()` or the extension's, `path` from `Pack::ignored()`) |
| `UnusedNative { dir: ToolName, tool: ToolName }` | `native/cluade: not for claude` (§16 R20, formats §2.2 wording) |
| `SourceChanged { location: Location }` | `agentstacks.toml: packs.core: source changed, resolved again` (§9.1 row 3) |

The `SourceChanged` wording lives here because `Plan` holds the note; the stale classification is
plan 07's. `AirslPin { locked: Option<PackVersion>, installed: PackVersion }` is the data behind "the
plan shows `airsl 0.1.4 → 0.1.5`" (§10); rendering it is `cli-commands`' (§17). `Plan` holds
(§6.1): `steps` (kept in path order, so the plan reads the same every run), `skips` (the `Skip`
proposals, the core's missing-value skips included, plan 04), `binaries`, `notes`, `airsl:
Option<AirslPin>`, `lock: Lockfile` (the new lock) and `old_files: Vec<FileEntry>` (the `[[file]]`
entries of the lock the plan was made against, empty when there was none), so apply can build the
§16 R17 partial lock from the `Plan` alone (Task 7, plan 14). Each step carries the disk state it was
decided on (Task 3). `Plan::new` takes the parts and sorts the steps by path.

**Proof:**
- `note_texts_match_the_spec` — `Ignored` with root `packs/core` and path `.DS_Store` displays
  `packs/core/.DS_Store: ignored (dot entry)`; `UnusedNative { dir: cluade, tool: claude }` displays
  `native/cluade: not for claude`; `SourceChanged` at key `packs.core` displays
  `agentstacks.toml: packs.core: source changed, resolved again`.
- `plan_orders_steps_by_path` — steps given as `b`, `a` come back `a`, `b`.
- `plan_keeps_every_part` — skips, binaries, notes, airsl pin, lock and old file entries read back as
  given.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): the plan value`.

### Task 9 — Report drift against the lock

**Files:**
- Create `crates/agentstacks/src/types/plan/airsl.rs` — `AirslAnswer`.
- Create `crates/agentstacks/src/types/plan/finding.rs` — `Finding`.
- Create `crates/agentstacks/src/types/plan/drift.rs` — the doctor rules.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod airsl; mod finding; mod drift;` plus `pub use`.

**Change:** `AirslAnswer` is `NotFound`, `Printed(String)`, `Failed(String)` or
`SpawnFailed(std::io::Error)`: what the `Binary` port (plan 09, `Binary::airsl_version`) learned from
`airsl --version`. `SpawnFailed` carries the I/O error of a spawn that failed for a reason other than
the binary being absent from `PATH`; its text is `agentstacks.toml: airsl: <io::Error Display>`
(§16 R1), given by `AirslAnswer::spawn_refusal()` as a `Diagnostic` at
`Location::file("agentstacks.toml")` with the R1 I/O variant of plan 02, so plans 13 and 15 reach the
same text. `AirslAnswer::from_output(stdout: &[u8], status: Exit)`
(`Exit` is `Success` or `Failure`) decodes the output as lossy UTF-8 and trims surrounding whitespace
(§16 R2); a non-zero exit gives `Failed`, which counts as unparseable (§16 R2). The version parse
is plan 04's `parse_airsl_version` (`src/types/op/airsl.rs`, §10 row 3, §16 R2), imported here and
never redefined (modularity Rule 2); plan 13 uses the same parse for the §10 refusal. `Finding` displays the §8 and §16 R23 texts verbatim:

| Variant | Display |
|---|---|
| `LockMissing` | `agentstacks.lock: not found (run apply)` (§8 row 1) |
| `LockIo(std::io::Error)` | `agentstacks.lock: <io::Error Display>` (§16 R1: the lock read failed; nothing else runs after it, like `LockMissing`) |
| `Lock(Diagnostic)` | the lock's own refusal, as its `Diagnostic` displays (§8 row 2) |
| `Missing(SubPath)` | `missing: <path>` (§8 row 3) |
| `NotRegular(SubPath)` | `not a regular file: <path>` (§8 row 4) |
| `Changed(SubPath)` | `changed: <path>` (§8 row 5) |
| `Outside(SubPath)` | `outside the project: <path>` (§16 R23) |
| `Io { path: SubPath, error: std::io::Error }` | `<path>: <io::Error Display>` (§16 R23) |
| `AirslNotFound { pin }` | `airsl: not found on PATH (lock pins 0.1.4)` (§8 row 6) |
| `AirslUnparsed { text }` | `airsl: --version printed "<text>", expected "airsl <version>"` (§8 row 7, `<text>` trimmed per R2) |
| `AirslMismatch { installed, pin }` | `airsl: 0.1.5 installed, lock pins 0.1.4` (§8 row 8) |
| `AirslSpawn(std::io::Error)` | `agentstacks.toml: airsl: <io::Error Display>` (§16 R1 spawn text) |

`file_finding(entry: &FileEntry, resolution: &Resolution) -> Option<Finding>`: `Outside` (any link)
→ `Outside`; inside and `Absent` → `Missing`; inside and `NotRegular` → `NotRegular`; inside `File` (a
symlink to a regular file inside the project counts, §8 row 4) whose hash ≠ the entry's → `Changed`;
equal → `None`. An I/O failure while reading the entry is the port's error; the flow (plan 15) turns
it into `Finding::Io`. `airsl_finding(pin: &PackVersion, answer: &AirslAnswer) -> Option<Finding>`:
`NotFound` → `AirslNotFound`; `SpawnFailed(error)` → `AirslSpawn(error)`; `Failed(text)` or an
unparseable `Printed(text)` → `AirslUnparsed` with the trimmed text; parsed ≠ pin → `AirslMismatch`;
equal → `None`. Since `AirslAnswer` and `Finding` hold `std::io::Error`, `airsl_finding` takes the
answer by value. "A lock exists", "the lock reads" (`LockIo`), "lock parses", "then no other check" and the order of findings (the lock's path order, then airsl, §16 R8) are the
flow's (plan 15); this task gives them their `Finding` values only.

**Proof:**
- `finding_texts_match_the_spec` — each variant's display equals its text with the example values
  (`0.1.4`, `0.1.5`, a sample path, printed text, and an `io::Error` of kind `PermissionDenied` with
  its own Display).
- `lock_finding_shows_the_lock_refusal` — `Lock` of the diagnostic `Lockfile::parse` gives for a
  `[[pack]]` with a `branch` key (`agentstacks.lock: pack[0].branch: unknown key`, formats §8 shape)
  displays that line unchanged.
- `drift_missing_file` — `Absent` → `missing: .claude/x`.
- `drift_not_a_regular_file` — `NotRegular` → `not a regular file: .claude/x`.
- `drift_outside_the_project` — `Outside { Symlink }` and `Outside { Direct }` →
  `outside the project: .claude/x`.
- `drift_symlink_inside_counts_as_regular` — `Inside { link: Symlink, state: File }` with the
  entry's hash → `None`.
- `drift_changed_file` — hash ≠ → `changed: .claude/x`.
- `drift_matching_file_is_healthy` — hash = → `None`.
- `airsl_answer_decodes_and_trims` — stdout `b"airsl 0.1.4\n"` with `Success` → `Printed("airsl 0.1.4")`;
  invalid UTF-8 bytes become U+FFFD; `Failure` → `Failed`.
- `airsl_not_found_on_path` — `airsl: not found on PATH (lock pins 0.1.4)`.
- `airsl_version_unparsed` — `Printed("oops")` →
  `airsl: --version printed "oops", expected "airsl <version>"`; `Failed("airsl 0.1.4")` gives the
  same shape with `airsl 0.1.4`.
- `airsl_version_mismatch` — `Printed("airsl 0.1.5")`, pin `0.1.4` →
  `airsl: 0.1.5 installed, lock pins 0.1.4`.
- `airsl_version_match_is_healthy` — `Printed("airsl 0.1.4")`, pin `0.1.4` → `None`.
- `airsl_spawn_failure_is_reported` — `SpawnFailed` of a `PermissionDenied` error →
  `airsl_finding` gives `AirslSpawn`, and both it and `spawn_refusal()` display
  `agentstacks.toml: airsl: <that error's Display>`.
- `lock_io_finding_text` — `LockIo` of a `PermissionDenied` error displays
  `agentstacks.lock: <that error's Display>`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): doctor findings against the lock`.

### Task 10 — Compare a fresh disk state with what a step was decided on

**Files:**
- Create `crates/agentstacks/src/types/plan/race.rs` — `Race` and the comparison.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod race;` plus `pub use`.

**Change:** `Race` is `Same`, `Changed` or `Outside`. `race(step: &Step, fresh: &Resolution) ->
Option<Race>` is apply's step-3 check (§7 step 3), defined only for a step that touches disk
(`Action::touches_disk`: create, update, restore, delete): any other step (`forget`, `unchanged`)
gives `None`, so a forget of an outside or dangling symlink never blocks apply. For a step that
touches disk: `Outside` (any link) → `Outside` (§16 R21: a path that resolves outside at apply time is
refused); otherwise the fresh `state` is compared with `step.decided_on` (always `Some` for these
steps):
both `Absent`, or both `File` with the same hash → `Same`; anything else (absent vs file, file vs
`NotRegular`, a different hash) → `Changed`. For a shared step `decided_on` equals what `read`
returned (Task 3, §16 R15), so this is the "for a shared file: what read returned" comparison.
`Race::refusal(&self, path: &OpPath) -> Option<Diagnostic>` gives, at `Location::file(path)`, the
`StepError` variants of plan 02: `Changed` → `<path>: changed after plan; run plan again` (§7 step 3),
`Outside` → `<path>: resolves outside the project` (§16 R21); `Same` → `None`. Plan 14 calls it for
every step it stages or deletes, and skips the rest.

**Proof:**
- `race_same_file_and_same_absence` — a file with the decided hash, and absent against absent → `Same`.
- `race_changed_hash_or_presence` — a different hash, a file where absent was decided, absent where a
  file was decided, and `NotRegular` → `Changed`, each refusing with
  `.claude/x: changed after plan; run plan again`.
- `race_outside_is_refused` — `Outside { Symlink }` →
  `.claude/x: resolves outside the project`.
- `race_shared_step_compares_with_the_read` — a shared `Create` decided on `Absent` against a fresh
  file → `Changed`.
- `race_skips_steps_that_do_not_touch_disk` — a forget of a dangling symlink (decided `None`) against
  a fresh `Inside { Symlink, NotRegular }`, and against `Outside { Symlink }`, gives `None`, so it
  passes apply; an `Unchanged` step gives `None` too.

**Done when:** `cargo make dod` passes (end of plan) and the Definition of Done is green.
Commit `feat(agentstacks): apply race check`.

---

## Risks

- `types/plan` uses `Origin`, `Proposal`, `Operation`, `OpPath` from plan 04 and `StepError` from plan
  02 (including its §16 R15 `changed after it was read` variant); a rename there breaks this plan's
  code, so execute builds 02 and 04 first.
- `decide_unproposed` takes an `OpPath` the caller built from the lock path. The lock reader (plan
  03, §16 R16 as extended) refuses a `[[file]].path` containing `\` or naming `agentstacks.toml` or
  `agentstacks.lock`, so every lock path this plan sees is a valid `OpPath`.
- `Step.decided_on` stores a whole `DiskFile` (bytes included) per step; a project with very large
  files holds them in memory twice (decided-on and new). Accepted: the spec's `Plan` already holds
  the new and old bytes (§6.1).
- `Finding::Io` holds a `std::io::Error`, which is neither `Clone` nor `PartialEq`; `Finding` derives
  only `Debug`, and tests compare displays.
- Option not taken: deciding sharing inside `decide_proposed` from the raw record. Keeping
  `Reads::shared` separate lets plan 13 and the removed-tool filter use one rule.
- Option not taken: a separate builder for the partial lock. One builder with `Landed` keeps the
  "new entry of a step" rule in one place for plans 13 and 14.

- `race` compares presence and content hash only: a mode-only change between plan and apply is not
  "changed after plan" (settled by the author; §7 "Modes" copies the old permissions at swap time).

## Proof

- `cargo make dod` is green at the end of the plan.
- Spec rows covered, each by a named test above: §6.2 "Disk states" (Task 1); §6.2 **Shared**
  definition (Task 2); §6.2 first table rows 1-10 and §16 R15 shared row (Task 3); §6.2 second table
  rows 1-5 and §16 R15 symlink-first rule (Task 4); §6.2 checked examples rows 1-8 (Task 5); §6.3
  outside decision for proposed and deleted paths (Tasks 3, 4); §6.4 bullets 2 and 4, §16 R18
  dropped proposals (Tasks 6, 7); §7 "Lock content" bullets 1-3 and §16 R17 partial lock (Task 7);
  §9.1 stale note text and §16 R20 notes (Task 8); §10 one check per requirement and pin data (Task
  8); §8 rows 1-8, §16 R23, §16 R1 lock-read and airsl spawn texts, §16 R2 airsl decoding and
  trimming (Task 9); §7 step 3 race and §16 R21 outside at apply time (Task 10). §6.3 resolution itself
  (symlink follow, canonicalise) is `Workspace`'s (plan 10); §6.4 bullets 1 and 3, §16 R7, R8 order
  and R27 are the flows' (plans 13, 14, 15).
- `grep -rn "spec\|chain" crates/agentstacks/src/types/plan` finds nothing in doc comments.
- `grep -rn "Fake" crates/agentstacks/src/types/plan` finds nothing (no doctest uses a fake).

## Review findings

Reviewer, one round: 0🔴, 3🟡 (code), 13🔵, 1❓, 1🟡 (plan). Blocking: `plan/built.rs:21` (skips). Reviewer `cargo test -p agentstacks types::plan`: 79 passed.

- fixed 🟡 (blocking) `plan/built.rs:21`: `Plan.skips: Vec<CoreSkip>` dropped an extension skip's `Origin`, so with two tools the plan could not say which tool skipped an item.
  - Fix: added the `Skipped` enum in `plan/skipped.rs`: `Extension { item, reason, origin }` or `Core(CoreSkip)`, with accessors `subject`, `reason`, `origin` and `tool`.
  - `plan_keeps_every_part` now asserts that the tools read back as `[Some("opencode"), None]`. It failed to compile before the fix and passes after.
  - Two tests were added: `an_extension_skip_keeps_its_origin` and `a_core_skip_has_no_origin`.
  - `cargo make dod` exit 0: 1429 tests passed, 0 failed.
- open 🟡 `decide/proposed.rs:33`: `ProposedPath.owner: Option<Owner>` lets an unshared write go unlocked when `None`.
- open 🟡 `plan/disk.rs:40`: `DiskFile` mode is a bare `u32`.
- open ❓ `plan/race.rs:39`: `race` ignores `link`, so a delete target that becomes an inside symlink after `plan` still passes the check and gets deleted.
- open 🔵 `Finding` also derives `thiserror::Error`; the plan's Risks said it derives only `Debug`.
- open 🔵 A core missing-value skip has no `Origin`, so `Skipped::tool()` is `None`; only its reason text names the tool. Structural attribution would need a tool field on `types::op::CoreSkip`.
- open 🔵 There are 12 further nits in `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/handoff/13-reviewer-plan06.md`.

## Deviations

- 2026-11-22: `lock_contents` lists all four fields instead of ending with `..LockContents::default()`. Plan 03 removed `merges`, so clippy's `needless_update` rejects the update under `-D warnings`.
- 2026-11-22: `Plan.skips` is `Vec<Skipped>`, a new type. No plan 04 type carries both an extension skip with its `Origin` and a core `SkipSubject::RootFile` skip. Plan 13 builds `Skipped::Extension` from a `Skip` proposal and `Skipped::Core` from a `CoreSkip`.
- 2026-11-22: `Plan` fields are private with accessors. `Plan::new` takes seven arguments and sorts steps by `OpPath`.
- 2026-11-22: `AirslAnswer::spawn_refusal` rebuilds the `io::Error` from its kind and Display, because `io::Error` is not `Clone`. The text is `agentstacks.toml: airsl: <err>`.
- 2026-11-22: `Reads::shared` returns a cloned `Option<ReadOutcome>`. `ProposedPath` has pub fields. `Exit` lives in `plan/airsl.rs`.
- 2026-11-22: `step.rs` and `binary.rs` are plain data; each got one small test instead of citing an exemption.
