---
status: draft
created: 2026-11-22
depends-on: [02, 04]
built-on: ea104af7451e
---

# Steps Implementation Plan

**Goal:** decide what happens to each path.

**Architecture:** everything here is a pure rule in `types/plan` (spec §2, D13): plain values in,
one answer out, no port and no I/O. The disk facts arrive as values that `Workspace` (plan 09) will
produce: a `Resolution` (inside the project with a `DiskState`, or outside, §6.3) and a read record
(`Reads`, what `agentstacks.read` returned, §4.3). Two decide functions turn a proposed path or a
locked-but-no-longer-proposed path into a `Step` or a located refusal (§6.2). A filter keeps only a
removed tool's shared writes (§6.4). A builder turns the accepted steps into the new lock's
`LockContents` (§7 "Lock content"). `Plan` is the one canonical value the flows hand around (§6.1,
modularity Rule 2). The doctor rule (§8) maps one lock entry plus its `Resolution`, and one airsl
answer, to `Finding`s. Refusals are `StepError` variants (plan 02) wrapped in a `Diagnostic` located
at the project-relative path (`Location::file`, §13 row `<path>`). Operation values (`Operation`,
`Proposal`, `Origin`, `OpPath`, owner derivation) are plan 04's and are used, never copied.

**Tech Stack:** Rust 2024, `crates/agentstacks` only; existing types `ContentHash`
(`src/types/tree/hash/content.rs`), `FileEntry`, `LockContents`, `LockedEntry`, `Lockfile`,
`Owner`, `check_airsl_presence` (`src/types/lock/*`), `SubPath` (`src/types/project/path/sub.rs`),
`PackVersion`, `PackManifest`, `BinaryName`, `Diagnostic`, `Location`; no new dependency.

**Content authority:** spec §6.2 (both tables and the checked examples), §6.3 (the decision given a
resolution), §6.4, §7 "Lock content", §8, §13 (locations).

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6): `mod-rs-export-only.md` (the
`types/plan/mod.rs` and `types/plan/decide/mod.rs` hold only docs, `mod` and `pub use`);
`modularity.md` Rule 1 (one job per file below) and Rule 2 (`Step`, `Plan` exist only here);
`strong-types.md` (no `bool` parameter for a decision: `Resolution` carries a `Link` enum, not a
flag; paths are `OpPath`/`SubPath`, hashes `ContentHash`); `unit-test-mandate.md` (every logic file
carries `#[cfg(test)] mod tests`); `doc-comment-discipline.md` (doc comments never name specs,
plans, chains or agents); `static-dispatch.md` (no trait objects; nothing here is generic over a
port).

---

## Files that change

```
crates/agentstacks/src/types/plan/mod.rs              — modify: add mod / pub use lines for the files below (docs stay)
crates/agentstacks/src/types/plan/disk.rs             — create: DiskFile, DiskState, Link, Resolution
crates/agentstacks/src/types/plan/action.rs           — create: Action
crates/agentstacks/src/types/plan/reads.rs            — create: ReadOutcome, Reads, the shared rule
crates/agentstacks/src/types/plan/step.rs             — create: Step
crates/agentstacks/src/types/plan/decide/mod.rs       — create: export-only
crates/agentstacks/src/types/plan/decide/proposed.rs  — create: decide a proposed path (§6.2 first table, §6.3)
crates/agentstacks/src/types/plan/decide/unproposed.rs — create: decide a locked path no longer proposed (§6.2 second table)
crates/agentstacks/src/types/plan/removed.rs          — create: removed-tool filter (§6.4)
crates/agentstacks/src/types/plan/lock.rs             — create: lock-content builder and the airsl pin rule (§7)
crates/agentstacks/src/types/plan/binary.rs           — create: BinaryCheck
crates/agentstacks/src/types/plan/note.rs             — create: SourceNote (§9.1 plan note), AirslPin (§10)
crates/agentstacks/src/types/plan/built.rs            — create: Plan
crates/agentstacks/src/types/plan/airsl.rs            — create: AirslAnswer, parse of `airsl <semver>`
crates/agentstacks/src/types/plan/finding.rs          — create: Finding (§8 texts)
crates/agentstacks/src/types/plan/drift.rs            — create: the doctor rules (§8)
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
`File(DiskFile)` or `NotRegular` (§6.2 "Disk states"). `Link` is `Direct` or `Symlink`: whether
the path itself is a symlink (needed by the second table's symlink row, §6.2). `Resolution` is
`Inside { link: Link, state: DiskState }` (the state of the canonical target; a dangling symlink is
`NotRegular`, §6.3 step 1) or `Outside` (§6.3 step 3). `Action` is `Create`, `Update`, `Restore`,
`Unchanged`, `Delete`, `Forget` (§6.2 step names), with `Action::writes()` true for create, update
and restore (the three apply stages, §7 step 2).

**Proof:** written first, seen failing, then passing.
- `disk_file_hash_matches_its_bytes` — a `DiskFile` built from `b"a\n"` reports the
  `ContentHash::of_bytes(b"a\n")` hash and the given mode.
- `action_writes_only_for_create_update_restore` — `writes()` is true for exactly `Create`,
  `Update`, `Restore`.

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
`decided_on: DiskState` (the state apply re-checks, §7 step 3; for a shared file it equals what
`read` returned, because only that case is accepted).

`decide_proposed(ProposedPath) -> Result<Step, Diagnostic>` where `ProposedPath` gathers: the
`OpPath`, the new bytes, the `Origin`, the owner already derived by plan 04's owner rule, the
`Option<ReadOutcome>` from `Reads::shared`, the lock hash if the path has a `[[file]]` entry, and
the `Resolution`. Rows, in this order (§6.3 then §6.2 first table):

1. `Outside` → refused `<path>: resolves outside the project` (§6.3 step 3; `StepError` variant).
2. `NotRegular` (any sharing, any lock) → refused `<path>: is not a regular file` (§6.2 row 1).
3. shared, disk absent and read returned absent → `create`, owner `None` (row 2).
4. shared, disk file whose hash equals what read returned → `update`, or `unchanged` when the new
   bytes equal the disk bytes; `old` = disk bytes; owner `None` (row 3).
5. not shared, locked, absent → `restore` (row 4).
6. not shared, locked, disk hash = lock hash → `update`, or `unchanged` when new = disk (row 5).
7. not shared, locked, disk hash ≠ lock hash, disk bytes = new bytes → `unchanged` (row 6).
8. not shared, locked, disk hash ≠ lock hash, disk bytes ≠ new bytes → refused
   `<path>: changed since the last apply` (row 7).
9. not shared, not locked, absent → `create` (row 8).
10. not shared, not locked, disk bytes = new bytes → `unchanged` (row 9, adopted).
11. not shared, not locked, disk bytes ≠ new bytes → refused
    `<path>: exists and was not written by agentstacks` (row 10).

Every refusal is a `Diagnostic` at `Location::file(<normalised path>)` carrying the matching
`StepError` variant (plan 02), so it displays exactly as quoted. A shared path whose disk state
disagrees with what `read` returned is not covered by the spec's table and is held for the author
(see Risks, gap GAP-1); this task does not add a row for it.

**Proof:** one test per row; each refusal test asserts the whole `Diagnostic` display verbatim.
- `proposed_outside_is_refused` — `Outside` → `.claude/x: resolves outside the project`.
- `proposed_not_regular_is_refused` — `NotRegular`, tried shared, locked and unlocked →
  `.claude/x: is not a regular file` each time.
- `shared_absent_read_absent_creates` — action `Create`, owner `None`, `old` `None`.
- `shared_equal_to_read_updates_with_old_bytes` — disk = read, new differs → `Update`, `old` = disk
  bytes, owner `None`.
- `shared_equal_to_read_and_new_is_unchanged` — new = disk → `Unchanged`, owner `None`.
- `locked_absent_restores` — `Restore` with the derived owner.
- `locked_hash_equal_updates` — `Update`; and `Unchanged` when new = disk.
- `locked_hash_differs_bytes_equal_new_is_unchanged` — `Unchanged`.
- `locked_hash_differs_bytes_differ_is_refused` — `.claude/x: changed since the last apply`.
- `unlocked_absent_creates` — `Create`.
- `unlocked_equal_bytes_is_adopted_unchanged` — `Unchanged`, owner kept (it will be locked, §6.2
  row 9 "adopted into the lock").
- `unlocked_different_bytes_is_refused` — `.claude/x: exists and was not written by agentstacks`.
- `step_keeps_disk_state_it_was_decided_on` — `decided_on` equals the input state for every
  accepted row.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): decide the step for a proposed path`.

### Task 4 — Decide the step for a locked path no longer proposed

**Files:**
- Create `crates/agentstacks/src/types/plan/decide/unproposed.rs` — the second-table decision.
- Modify `crates/agentstacks/src/types/plan/decide/mod.rs` — `mod unproposed;` plus `pub use`.

**Change:** `decide_unproposed(entry: &FileEntry, resolution: &Resolution) -> Result<Step, Diagnostic>`
(§6.2 second table). The step's path is the entry's path as an `OpPath`, owner the entry's `by`,
origin `None`, `new` `None`. Rows:

1. inside, state `Absent` → `forget` (row 1).
2. inside, `Link::Symlink` (the path itself is a symlink), any state → `forget` (row 2).
3. inside, direct, file hash = lock hash → `delete` (row 3).
4. inside, direct, file hash ≠ lock hash → refused
   `<path>: changed since the last apply; not deleting it` (row 4).
5. inside, direct, `NotRegular` → refused `<path>: is not a regular file` (row 5).
6. `Outside` → refused `<path>: resolves outside the project` (§6.3: "every … deleted path … is
   resolved"). The order of rows 2 and 6 for a symlink that points outside, and of rows 2 and 5 for
   a dangling symlink, is held for the author (gap GAP-2); the tests for those two inputs are written
   after the author settles it.

**Proof:**
- `unproposed_absent_forgets` — `Forget`.
- `unproposed_symlink_forgets` — a symlink to an inside regular file whose hash matches → `Forget`
  (the user replaced it, so it is left alone).
- `unproposed_hash_equal_deletes` — `Delete`, owner = the entry's `by`.
- `unproposed_hash_differs_is_refused` — `.claude/x: changed since the last apply; not deleting it`.
- `unproposed_not_regular_is_refused` — `.claude/x: is not a regular file`.
- `unproposed_outside_is_refused` — a direct path resolving outside →
  `.claude/x: resolves outside the project`.

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
- `example_symlink_to_etc_hosts_is_refused` — proposed path resolving `Outside` →
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
path not read) is dropped (§6.4 bullet 2). The core writes for the removed tool's root are never
made: that is the flow's (plan 13), and this module documents it as a non-responsibility.

**Proof:**
- `removed_tool_keeps_only_shared_writes` — a write to a read path is kept; a write to an unread
  path, a copy, a skip and a require_binary are dropped.
- `removed_tool_without_reads_keeps_nothing` — an empty `Reads` drops every proposal.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): removed-tool filter`.

### Task 7 — Build the new lock contents

**Files:**
- Create `crates/agentstacks/src/types/plan/lock.rs` — the lock-content builder and airsl pin rule.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod lock;` plus `pub use`.

**Change:** `airsl_pin(locked: impl IntoIterator<Item = &PackManifest>, installed:
Option<&PackVersion>) -> Option<PackVersion>` returns the installed version exactly when a locked
pack declares `requires.airsl` (§7 "Lock content" third bullet, §10 "The installed airsl version is
written to `[airsl] version`"). `lock_contents(tools, packs, steps: &[Step], airsl:
Option<PackVersion>) -> Result<LockContents, InvalidSubPath>` (§7 "Lock content"):

- `tools` and `packs` (`LockedEntry` values with rev and tree, built by the flow) are carried as
  given (bullet 1);
- one `FileEntry` per step whose owner is `Some` and whose action is `Create`, `Update`, `Restore`
  or `Unchanged`, with `by` = the owner and `hash` = `ContentHash::of_bytes(new)` (bullet 2; so a
  user who made the same edit gives the lock the new hash, §6.2 row 6, and an adopted file enters the
  lock, row 9);
- a shared step (owner `None`), a `Delete` and a `Forget` give no entry, so a shared path's old lock
  entry is dropped (§6.2 row 3) and a removed tool's files leave the lock (§6.4 bullet 4);
- each `OpPath` becomes a `SubPath` through `SubPath::new`; `OpPath`'s shape rule (plan 04) is
  stricter than `SubPath`'s, so the error is unreachable for a valid `OpPath`, but it is returned,
  never unwrapped (workspace lint `unwrap_used`).

The result builds a `Lockfile` through `Lockfile::new` (the flow does that and stores the
`Lockfile` in `Plan`, Task 8).

**Proof:**
- `lock_records_written_and_unchanged_steps` — create, update, restore and unchanged steps each
  give a `[[file]]` with the owner and the hash of the new bytes.
- `lock_leaves_out_shared_delete_and_forget` — those three give no entry.
- `lock_takes_the_new_hash_for_the_same_user_edit` — a §6.2 row-6 step records
  `ContentHash::of_bytes(new)`, not the old lock hash.
- `lock_contents_build_a_lockfile` — the contents (one tool, one pack, two files owned by them) pass
  `Lockfile::new`, and `write_lock` of it reparses with `Lockfile::parse`.
- `airsl_pinned_exactly_when_a_locked_pack_requires_it` — with a manifest declaring
  `requires.airsl` and installed `0.1.4` → `Some(0.1.4)`, and `check_airsl_presence` passes on the
  built lock; with no such manifest → `None`, and `check_airsl_presence` passes too.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): new lock contents from the steps`.

### Task 8 — Hold the whole plan

**Files:**
- Create `crates/agentstacks/src/types/plan/binary.rs` — `BinaryCheck`.
- Create `crates/agentstacks/src/types/plan/note.rs` — `SourceNote`, `AirslPin`.
- Create `crates/agentstacks/src/types/plan/built.rs` — `Plan`.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod binary; mod note; mod built;` plus `pub use`.

**Change:** `BinaryCheck { name: BinaryName, origin: Origin }` is one passed binary check; one per
requirement, so a binary required by a pack and by an extension appears once for each (§10). A
failed check is a refusal (plan 02 / plan 13), never a `BinaryCheck`. `SourceNote` holds the
`Location` of an entry the flow resolved again (`agentstacks.toml` at key `packs.core`) and displays
exactly `agentstacks.toml: packs.core: source changed, resolved again` (§9.1 row 3); the wording
lives here because `Plan` holds the note, and the stale classification is plan 07's. `AirslPin {
locked: Option<PackVersion>, installed: PackVersion }` is the data behind "the plan shows `airsl
0.1.4 → 0.1.5`" (§10); rendering it is `cli-commands`' (§16). `Plan` holds (§6.1): `steps` (kept in
path order, so the plan reads the same every run), `skips` (the `Skip` proposals, the core's
missing-value skips included, plan 04), `binaries`, `ignored` (formats' dot-entry listing, paths as
`Pack::ignored` gives them), `unused_native` (pack and tool pairs from `Pack::unused_native`), `notes`
(`SourceNote`s), `airsl: Option<AirslPin>` and `lock: Lockfile` (the new lock). Each step carries the
disk state it was decided on (Task 3). `Plan::new` takes the parts and sorts the steps by path.

**Proof:**
- `source_note_displays_the_spec_text` — `SourceNote` at key `packs.core` displays
  `agentstacks.toml: packs.core: source changed, resolved again`.
- `plan_orders_steps_by_path` — steps given as `b`, `a` come back `a`, `b`.
- `plan_keeps_every_part` — skips, binaries, ignored, unused-native pairs, notes, airsl pin and lock
  read back as given.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): the plan value`.

### Task 9 — Report drift against the lock

**Files:**
- Create `crates/agentstacks/src/types/plan/airsl.rs` — `AirslAnswer` and the version parse.
- Create `crates/agentstacks/src/types/plan/finding.rs` — `Finding`.
- Create `crates/agentstacks/src/types/plan/drift.rs` — the doctor rules.
- Modify `crates/agentstacks/src/types/plan/mod.rs` — `mod airsl; mod finding; mod drift;` plus `pub use`.

**Change:** `AirslAnswer` is `NotFound` or `Printed(String)`: what the `Binary` port (plan 09) learned
from `airsl --version`. `parse_airsl_version(text) -> Option<PackVersion>` accepts exactly
`airsl <semver>` (§10 "parsed as `airsl <semver>`"; the probe printed `airsl 0.1.4`,
`probes/hookreq-results.md`); whether trailing whitespace is trimmed is held for the author (gap
GAP-3). Plan 13 reuses this parse for the §10 refusal. `Finding` displays the §8 texts verbatim:

| Variant | Display (§8) |
|---|---|
| `LockMissing` | `agentstacks.lock: not found (run apply)` |
| `Lock(Diagnostic)` | the lock's own refusal, as its `Diagnostic` displays |
| `Missing(SubPath)` | `missing: <path>` |
| `NotRegular(SubPath)` | `not a regular file: <path>` |
| `Changed(SubPath)` | `changed: <path>` |
| `AirslNotFound { pin }` | `airsl: not found on PATH (lock pins 0.1.4)` |
| `AirslUnparsed { text }` | `airsl: --version printed "<text>", expected "airsl <version>"` |
| `AirslMismatch { installed, pin }` | `airsl: 0.1.5 installed, lock pins 0.1.4` |

`file_finding(entry: &FileEntry, resolution: &Resolution) -> Option<Finding>`: inside and `Absent`
→ `Missing`; inside and `NotRegular` → `NotRegular`; inside `File` (a symlink to a regular file
inside the project counts, §8 row 4) whose hash ≠ the entry's → `Changed`; equal → `None`. A
`[[file]]` that resolves `Outside` is not covered by §8 and is held for the author (gap GAP-4).
`airsl_finding(pin: &PackVersion, answer: &AirslAnswer) -> Option<Finding>`: `NotFound` →
`AirslNotFound`; unparseable → `AirslUnparsed` with the printed text; parsed ≠ pin →
`AirslMismatch`; equal → `None`. "A lock exists", "lock parses" and "then no other check" are the
flow's order (plan 15); this task gives them their `Finding` values only.

**Proof:**
- `finding_texts_match_the_spec` — each variant's display equals its §8 text with the example
  values (`0.1.4`, `0.1.5`, a sample path and printed text).
- `lock_finding_shows_the_lock_refusal` — `Lock` of the diagnostic `Lockfile::parse` gives for a
  `[[pack]]` with a `branch` key (`agentstacks.lock: pack[0].branch: unknown key`, formats §8 shape)
  displays that line unchanged.
- `drift_missing_file` — `Absent` → `missing: .claude/x`.
- `drift_not_a_regular_file` — `NotRegular` → `not a regular file: .claude/x`.
- `drift_symlink_inside_counts_as_regular` — `Inside { link: Symlink, state: File }` with the
  entry's hash → `None`.
- `drift_changed_file` — hash ≠ → `changed: .claude/x`.
- `drift_matching_file_is_healthy` — hash = → `None`.
- `airsl_version_parses_airsl_semver` — `airsl 0.1.4` → `0.1.4`; `airsl` alone, `0.1.4`,
  `airsl x` → `None`.
- `airsl_not_found_on_path` — `airsl: not found on PATH (lock pins 0.1.4)`.
- `airsl_version_unparsed` — `Printed("oops")` → `airsl: --version printed "oops", expected "airsl <version>"`.
- `airsl_version_mismatch` — `Printed("airsl 0.1.5")`, pin `0.1.4` →
  `airsl: 0.1.5 installed, lock pins 0.1.4`.
- `airsl_version_match_is_healthy` — `Printed("airsl 0.1.4")`, pin `0.1.4` → `None`.

**Done when:** `cargo make dod` passes (end of plan) and the Definition of Done is green.
Commit `feat(agentstacks): doctor findings against the lock`.

---

## Risks

- **GAP-1 (open, author):** §6.2 has no row for a shared path whose disk state differs from what
  `read` returned (the file changed between the extension's read and the disk read). No message
  exists; Task 3 adds no row until the author settles it.
- **GAP-2 (open, author):** for a locked path no longer proposed, the symlink row (`forget`, §6.2)
  and §6.3 ("every … deleted path … is resolved"; outside → refused, dangling → not a regular file)
  both match a symlink that points outside or dangles. Task 4 holds those two tests.
- **GAP-3 (open, author):** `airsl --version` output almost certainly ends with a line break; whether the
  parse trims it, and whether `<text>` in the finding is trimmed, is not stated.
- **GAP-4 (open, author):** §8 has no finding for a `[[file]]` that resolves outside the project.
- **GAP-5 (open, author):** a `[[file]]` path the lock accepts (`SubPath` rule) may still fail
  `OpPath`'s stricter rule (for example a hand-edited `a\b`). `decide_unproposed` needs an `OpPath`
  for the step; the spec gives no refusal for this case.
- `LockContents` loses its `merges` field in plan 03, which is not upstream of this plan. The builder
  fills the fields it owns and ends the struct literal with `..LockContents::default()`, so it
  compiles before and after plan 03 lands.
- `types/plan` uses `Origin`, `Proposal`, `Operation`, `OpPath` from plan 04 and `StepError` from plan
  02; a rename there breaks this plan's code, so execute builds 02 and 04 first.
- `Step.decided_on` stores a whole `DiskFile` (bytes included) per step; a project with very large
  files holds them in memory twice (decided-on and new). Accepted: the spec's `Plan` already holds
  the new and old bytes (§6.1).
- Option not taken: deciding sharing inside `decide_proposed` from the raw record. Keeping
  `Reads::shared` separate lets plan 13 and the removed-tool filter use one rule.
- The partial lock written when apply's swap fails (§7 failure row 4) is apply's (plan 14); this
  plan's builder covers the normal lock only.

## Proof

- `cargo make dod` is green at the end of the plan.
- Spec rows covered, each by a named test above: §6.2 "Disk states" (Task 1); §6.2 **Shared**
  definition (Task 2); §6.2 first table rows 1-10 (Task 3); §6.2 second table rows 1-5 (Task 4);
  §6.2 checked examples rows 1-8 (Task 5); §6.3 outside decision for proposed and deleted paths
  (Tasks 3, 4); §6.4 bullets 2 and 4 (Tasks 6, 7); §7 "Lock content" bullets 1-3 (Task 7); §9.1
  stale note text (Task 8); §10 one check per requirement and pin data (Task 8); §8 rows 1-8 as
  `Finding` texts and the file and airsl rules (Task 9). §6.3 resolution itself (symlink follow,
  canonicalise) is `Workspace`'s (plan 10); §6.4 bullet 1 and 3 and the §8 check order are the flows'
  (plans 13, 15).
- `grep -rn "spec\|plan 0\|chain" crates/agentstacks/src/types/plan` finds nothing in doc comments.
