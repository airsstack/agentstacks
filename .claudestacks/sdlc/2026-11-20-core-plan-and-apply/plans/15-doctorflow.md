---
status: done
created: 2026-11-22
depends-on: [06, 09]
built-on: f39b2a814319
---

# Doctor flow Implementation Plan

**Goal:** Report drift between the project disk and `agentstacks.lock`.

**Architecture:** `app/doctor` is a flow and holds no rule (spec §2, `modularity.md` Rule 1). It
reads the lock text through `Workspace::lock_text`, parses it with the built `Lockfile::parse`,
asks `Workspace::resolve` for the `Resolution` of every `[[file]]` path, asks
`Binary::airsl_version` when the lock has `[airsl]`, and hands each answer to plan 06's drift rules
(`file_finding`, `airsl_finding`), which return `Finding` values. The flow only orders those calls
and collects the findings in the R8 order. It is generic over `W: Workspace` and `B: Binary`
(spec §2 static-dispatch row). It takes no `Git` and no `Adapter`: doctor is offline and needs no
extension (spec §8 first and last lines, §18 "Network access in `doctor`").

**Tech Stack:** Rust 2024, crate `agentstacks`; `Workspace`, `Binary`, `FakeWorkspace`,
`FakeBinary`, `WorkspaceCall`, `BinaryCall` from plan 09; `Resolution`, `AirslAnswer`, `Finding`,
`file_finding`, `airsl_finding` from plan 06; `OpPath` from plan 04; the built `Lockfile`
(`src/types/lock/file.rs`).

**Content authority:** spec §8 (every row), §16 R1, R8, R16, R23, §6.3 (what counts as a regular
file), §2 (layer rule), §14 `app/doctor` row.

---

## Files that change

```
crates/agentstacks/src/app/doctor.rs   — modify: replace the docs-only stub with the doctor flow and its colocated tests
```

`src/app/mod.rs` already declares `doctor` (plan 01) and is not edited here: callers reach the flow
as `app::doctor::doctor`.

## Order of work

### Task 1 — Check the lock is present and parses

**Files:**
- Modify `crates/agentstacks/src/app/doctor.rs` — the module doc (what, why, responsibilities,
  non-responsibilities: no rule, no network, no extension, no writes) and the function `doctor`.

**Change:** `doctor<W: Workspace, B: Binary>(workspace: &W, binary: &B) -> Vec<Finding>` returns
every finding, and an empty list means healthy (spec §8 "none means healthy"). Its rustdoc
describes inputs and output in prose and carries no example that uses a fake: the fakes are
`#[cfg(test)]` and a doctest cannot see them. This task adds the lock steps:

- `Workspace::lock_text` answers `Ok(None)` → exactly one `Finding::LockMissing`, displayed
  `agentstacks.lock: not found (run apply)`; no other check runs and `Binary` is never asked
  (spec §8 row 1, "then no other check");
- `Workspace::lock_text` fails → exactly one I/O finding, displayed `agentstacks.lock: <io::Error
  Display>` (spec §16 R23 second message, R1 "the lock"); no other check runs, because there is no
  lock to check against;
- the text is parsed with `Lockfile::parse` at `agentstacks.lock`; when it is refused, each of the
  lock's own refusals (formats §8, plus the R16 separator refusal) is a `Finding::Lock`, and no
  file or airsl check runs (spec §8 row 2). `check_airsl_presence` is not called (spec §16 R27).

**Proof:** written first, seen failing, then passing.
- `doctor_reports_a_missing_lock_alone` — a `FakeWorkspace` with no lock and a `FakeBinary` with
  no airsl: the result is the single finding `agentstacks.lock: not found (run apply)`, and
  `FakeBinary::asked()` is empty.
- `doctor_reports_an_unreadable_lock_alone` — `FakeWorkspace::fail(WorkspaceCall::LockText)`: the
  result is the single finding `agentstacks.lock: ` followed by the Display of
  `FakeWorkspace::denied()`.
- `doctor_reports_the_lock_refusals` — a lock text holding a `[[merge]]` table: the findings are the
  lock's refusals, among them `agentstacks.lock: merge: unknown key` (spec §12 row 2), and no
  `Resolve` call was made.
- `doctor_reports_a_backslash_lock_path` — a lock whose `file[0].path` is `a\b`: the finding
  `agentstacks.lock: file[0].path: "a\b" must use / as the separator` (spec §16 R16).
- `doctor_finds_nothing_for_an_empty_healthy_lock` — a lock with no `[[file]]` and no `[airsl]`:
  the result is empty, and `FakeBinary::asked()` is empty.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): doctor reads and checks the lock`.

### Task 2 — Check every locked file against the disk

**Files:**
- Modify `crates/agentstacks/src/app/doctor.rs` — the file loop and its tests.

**Change:** for each `FileEntry` of the lock, in the lock's path order (spec §16 R8; `Lockfile`
keeps its lists sorted), the flow turns the entry's path into the `OpPath` the port takes (every
locked path passes, because R16 refuses a `\` at lock read), asks `Workspace::resolve`, and hands
the entry and the `Resolution` to `file_finding`. One finding at most per file:

- inside, absent → `missing: <path>` (spec §8 row 3);
- inside, not a regular file (a directory, a dangling symlink, spec §6.3 step 1) → `not a regular
  file: <path>` (spec §8 row 4);
- inside, a regular file or a symlink to one inside the project, hash ≠ lock hash → `changed:
  <path>` (spec §8 rows 4-5);
- inside, hash = lock hash → no finding;
- `Resolution::Outside` (for example a symlink to `/etc/hosts`) → `outside the project: <path>`
  (spec §16 R23 first message);
- `Workspace::resolve` fails → `<path>: <io::Error Display>` (spec §16 R23 second message, R1).

Every file is checked; findings are collected, never first-only. Shared files are not in the lock
(spec §6.2), so nothing here reads an extension (spec §8 last line).

**Proof:** written first, seen failing, then passing.
- `doctor_reports_a_missing_file` — a locked `.claude/skills/status/SKILL.md` with no entry in the
  `FakeWorkspace`: the finding `missing: .claude/skills/status/SKILL.md`.
- `doctor_reports_a_directory_as_not_a_regular_file` — `FakeWorkspace::not_regular` at the locked
  path: `not a regular file: .claude/x`.
- `doctor_reports_a_dangling_symlink_as_not_a_regular_file` — `FakeWorkspace::symlink` to a target
  with no entry: `not a regular file: .claude/x`.
- `doctor_reports_a_changed_file` — the disk bytes hash differently from the lock: `changed:
  .claude/x`.
- `doctor_accepts_a_symlink_to_a_matching_file_inside` — a symlink to an inside file whose hash
  equals the lock's: no finding.
- `doctor_reports_a_symlink_to_a_changed_file_inside` — the same symlink, target bytes changed:
  `changed: .claude/x`.
- `doctor_reports_a_file_resolving_outside` — `FakeWorkspace::outside(".claude/x")`: the finding
  `outside the project: .claude/x`.
- `doctor_reports_an_unreadable_file` — `FakeWorkspace::fail(WorkspaceCall::Resolve(".claude/x"))`:
  the finding `.claude/x: ` followed by the Display of `FakeWorkspace::denied()`, and the next
  locked file is still checked.
- `doctor_reports_every_drifted_file_in_path_order` — locked `a.md` (missing), `b.md` (intact),
  `c.md` (changed): exactly `missing: a.md` then `changed: c.md`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): doctor checks locked files`.

### Task 3 — Check the airsl pin

**Files:**
- Modify `crates/agentstacks/src/app/doctor.rs` — the airsl step and its tests.

**Change:** when `Lockfile::airsl()` is `Some(pin)`, the flow asks `Binary::airsl_version` once and
hands the pin and the `AirslAnswer` to `airsl_finding` (spec §8 rows 6-8). One finding at most:

- `NotFound` → `airsl: not found on PATH (lock pins 0.1.4)` (pin filled in);
- `SpawnFailed(error)` (airsl found but will not run) → `agentstacks.toml: airsl: <io::Error
  Display>` (spec §16 R1);
- `Failed(text)` (airsl exited non-zero, which R2 counts as unparseable) → the unparseable finding
  below;
- printed text that does not parse as `airsl <semver>` → `airsl: --version printed "<text>",
  expected "airsl <version>"` (text filled in; the trimming of `<text>` and "airsl exiting non-zero
  counts as unparseable" are spec §16 R2, applied by plans 06 and 11, not by this flow);
- installed version ≠ pin → `airsl: 0.1.5 installed, lock pins 0.1.4` (both filled in);
- equal → no finding.

When the lock has no `[airsl]`, `Binary` is never asked. The airsl finding comes after every file
finding (spec §16 R8 "doctor findings follow the lock's path order, then airsl"); both sets are
returned.

**Proof:** written first, seen failing, then passing.
- `doctor_reports_airsl_missing_from_path` — lock pins `0.1.4`, `FakeBinary` with no `airsl(…)`:
  the finding `airsl: not found on PATH (lock pins 0.1.4)`.
- `doctor_reports_an_airsl_spawn_failure` — `FakeBinary::airsl_spawn_failure()`: the finding
  `agentstacks.toml: airsl: ` followed by the Display of the `PermissionDenied` error the fake
  carries (spec §16 R1).
- `doctor_reports_a_failing_airsl_as_unparseable` — `FakeBinary::airsl_exit_failure("boom")`: the
  finding `airsl: --version printed "boom", expected "airsl <version>"` (spec §16 R2).
- `doctor_reports_unparseable_airsl_version` — `FakeBinary::airsl("airsl version 0.1.4")`: the
  finding `airsl: --version printed "airsl version 0.1.4", expected "airsl <version>"`.
- `doctor_reports_a_different_airsl_version` — `FakeBinary::airsl("airsl 0.1.5")`: the finding
  `airsl: 0.1.5 installed, lock pins 0.1.4`.
- `doctor_accepts_the_pinned_airsl` — `FakeBinary::airsl("airsl 0.1.4")`: no finding, and
  `asked()` is exactly one `BinaryCall::AirslVersion`.
- `doctor_skips_airsl_without_a_pin` — a lock with no `[airsl]`: no finding, and `asked()` is
  empty.
- `doctor_puts_airsl_after_the_files` — one changed file and a missing airsl: `changed: <path>`
  first, then `airsl: not found on PATH (lock pins 0.1.4)`.

**Done when:** `cargo make dod-crate agentstacks` passes, then `cargo make dod` passes, and the
Definition of Done is green. Commit `feat(agentstacks): doctor checks the airsl pin`.

---

## Risks

- `Finding`, its `Display` texts (including the two R23 findings and the lock-read I/O finding) and
  the drift rules are plan 06's. This flow hands them raw answers (`FileEntry`, `Resolution`, the
  `io::Error`, `AirslAnswer`) and never builds a finding text itself, or the rule is duplicated in
  `app/` (`modularity.md` Rule 2). The riskiest step is Task 3, where parsing `airsl <semver>` is
  tempting to write inline; it is plan 06's `airsl_finding`.
- `tests/isolation.rs` refuses `crate::fs`, `crate::process`, `crate::host` and `std::fs` inside
  `src/app`. Test fixtures come from the fakes, never from a temp directory.
- Not done here: rendering findings and exit codes (spec §17, `cli-commands`).

## Proof

- Spec §8 rows covered: row 1 (lock exists) Task 1; row 2 (lock parses) Task 1; row 3 (file
  exists) Task 2; row 4 (regular file, symlink inside counts, §6.3) Task 2; row 5 (hash) Task 2;
  row 6 (airsl on `PATH`) Task 3; row 7 (`--version` parses) Task 3; row 8 (version equal) Task 3;
  "doctor needs no extension" — the signature takes no `Adapter` or `Git` (Task 1).
- Spec §16 rows covered: R1 and R23 (lock-read and file-resolve I/O, outside the project) Tasks 1-2; R1 (airsl spawn
  failure) Task 3;
  R8 (lock path order, then airsl) Tasks 2-3; R16 (backslash lock path) Task 1; R27 (no
  `check_airsl_presence` on the read lock) Task 1. R2 (non-zero airsl exit as unparseable, Task 3; trimming) is met through plan 06's
  `airsl_finding`.
- Spec §14 `app/doctor` row: every doctor row is a unit test on fakes.
- No rustdoc example uses a fake (`grep -n "Fake" crates/agentstacks/src/app/doctor.rs` shows hits
  only inside `#[cfg(test)] mod tests`).
- `cargo test -p agentstacks --test isolation` passes (no adapter import in `src/app`).
- `cargo make dod` is green at the end of the plan.

## Review findings

Reviewer, one round, shared with plans 14 and 16: code 0🔴 1🟡 1🔵, plan 1🔵. Blocking: none, so there was no fix round. Reviewer `cargo test -p agentstacks --lib app::`: 92 passed, 0 failed. `cargo test -p agentstacks --test isolation`: 17 passed. Coder: 22 doctor tests green. The full `cargo make dod` gate is recorded at the end of the run.

- open 🟡 `src/app/doctor.rs:67-72`: a locked path that cannot become an `OpPath` (the lock accepts `"."` and `"./"`, which `OpPath::new` refuses as empty) is reported as `Finding::NotRegular`. That choice is made in `app/` and no test covers it.
- open 🔵 `doctor.rs:48-52`: `into_sorted().into_iter()` is redundant.
- open 🔵 The plan's premise that every locked path forms an `OpPath` is wrong for `"."`.

## Deviations

- 2026-11-22: A locked path that cannot form an `OpPath` reports `NotRegular` (the project directory). The plan is silent on this.
- 2026-11-22: This plan and plan 16 were built by one coder. Task briefs were read from the plan directly.
