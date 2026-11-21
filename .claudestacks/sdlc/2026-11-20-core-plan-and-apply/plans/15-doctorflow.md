---
status: draft
created: 2026-11-22
depends-on: [06, 09]
built-on: ea104af7451e
---

# Doctor flow Implementation Plan

**Goal:** Report drift between the project disk and `agentstacks.lock`.

**Architecture:** `app/doctor` is a flow and holds no rule (spec §2, `modularity.md` Rule 1). It
reads the lock text through the `Workspace` port, parses it with the built `Lockfile::parse`, asks
`Workspace` for the §6.3 resolution and the `DiskState` of every `[[file]]` path, asks the `Binary`
port about airsl when the lock has `[airsl]`, and hands each answer to plan 06's drift rule, which
returns `Finding` values. The flow only orders those calls and collects the findings. It is
generic over `W: Workspace` and `B: Binary` (spec §2 static-dispatch row). It takes no `Git` and
no `Adapter`: doctor is offline and needs no extension (spec §8 first and last lines, §17 "Network
access in `doctor`").

**Tech Stack:** Rust 2024, crate `agentstacks`; `FakeWorkspace` and `FakeBinary` from plan 09; the
built `Lockfile` (`src/types/lock/file.rs`); `Finding` and its drift rule from plan 06.

**Content authority:** spec §8 (every row), §6.3 (what counts as a regular file), §2 (layer rule),
§14 `app/doctor` row.

---

## Files that change

```
crates/agentstacks/src/app/doctor.rs   — modify: replace the docs-only stub with the doctor flow and its colocated tests
```

`src/app/mod.rs` already declares `doctor` (plan 01). If the flow is exported from `app`, the one
`pub use` line it needs is the only edit there, and it stays export-only (`mod-rs-export-only.md`).

## Order of work

### Task 1 — Check the lock is present and parses

**Files:**
- Modify `crates/agentstacks/src/app/doctor.rs` — the module doc (what, why, responsibilities,
  non-responsibilities: no rule, no network, no extension, no writes) and the function `doctor`.

**Change:** `doctor<W: Workspace, B: Binary>(workspace: &W, binary: &B) -> Vec<Finding>` returns
every finding, and an empty list means healthy (spec §8 "none means healthy"). This task adds the
first two §8 rows:

- the lock text is read through `Workspace`; when no lock exists the result is exactly one
  `Finding`, displayed `agentstacks.lock: not found (run apply)`, and no other check runs (spec §8
  row 1, "then no other check"; `Binary` is never asked);
- the text is parsed with `Lockfile::parse` at `agentstacks.lock`; when it is refused, the lock's
  own refusals (formats §8) are the findings, carried by plan 06's `Finding` for that row, and no
  file or airsl check runs, because there is no lock to check against (spec §8 row 2).

**Proof:** written first, seen failing, then passing.
- `doctor_reports_a_missing_lock_alone` — a `FakeWorkspace` with no lock, and a `FakeBinary` that
  would answer "not found": the result is the single finding `agentstacks.lock: not found (run
  apply)`.
- `doctor_reports_the_lock_refusals` — a lock text holding a `[[merge]]` table: the findings are the
  lock's refusals, among them `agentstacks.lock: merge: unknown key` (spec §12 row 2), and nothing
  else.
- `doctor_finds_nothing_for_an_empty_healthy_lock` — a lock with no `[[file]]` and no `[airsl]`:
  the result is empty, and the `FakeBinary` recorded no question.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): doctor reads and checks the lock`.

### Task 2 — Check every locked file against the disk

**Files:**
- Modify `crates/agentstacks/src/app/doctor.rs` — the file loop and its tests.

**Change:** for each `FileEntry` of the lock, in the lock's order (`Lockfile` keeps its lists
sorted), the flow asks `Workspace` to resolve the path (spec §6.3) and read its `DiskState`, then
hands the entry and that state to plan 06's drift rule. The rule yields at most one finding per
file (spec §8 rows 3-5):

- absent → `missing: <path>`;
- not a regular file (a directory, a dangling symlink, spec §6.3 step 1) → `not a regular file:
  <path>`;
- a regular file, or a symlink to one inside the project (spec §8 row 4, §6.3), whose hash differs
  from the lock's → `changed: <path>`;
- a regular file whose hash equals the lock's → no finding.

Every file is checked; findings are collected, never first-only. Shared files are not in the lock
(spec §6.2), so nothing here reads an extension (spec §8 last line).

**Proof:** written first, seen failing, then passing.
- `doctor_reports_a_missing_file` — a locked `.claude/skills/status/SKILL.md` absent from the
  `FakeWorkspace`: the finding `missing: .claude/skills/status/SKILL.md`.
- `doctor_reports_a_directory_as_not_a_regular_file` — the locked path is a directory: `not a
  regular file: <path>` with the path filled in.
- `doctor_reports_a_dangling_symlink_as_not_a_regular_file` — the locked path is a symlink whose
  target is missing: `not a regular file: <path>`.
- `doctor_reports_a_changed_file` — the disk bytes hash differently from the lock: `changed:
  <path>`.
- `doctor_accepts_a_symlink_to_a_matching_file_inside` — the locked path is a symlink to a regular
  file inside the project whose hash equals the lock's: no finding.
- `doctor_reports_a_symlink_to_a_changed_file_inside` — the same symlink, target bytes changed:
  `changed: <path>`.
- `doctor_reports_every_drifted_file` — three locked files, one missing, one changed, one intact:
  exactly the two findings, in the lock's path order.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): doctor checks locked files`.

### Task 3 — Check the airsl pin

**Files:**
- Modify `crates/agentstacks/src/app/doctor.rs` — the airsl step and its tests.

**Change:** when `Lockfile::airsl()` is `Some(version)`, the flow asks `Binary` whether `airsl` is
on `PATH` and, only when it is, for its `airsl --version` text; it hands the pinned version and
those answers to plan 06's drift rule (spec §8 rows 6-8). One finding at most:

- not on `PATH` → `airsl: not found on PATH (lock pins 0.1.4)` (pinned version filled in);
- the text does not parse as `airsl <semver>` → `airsl: --version printed "<text>", expected
  "airsl <version>"` (text filled in);
- installed version ≠ pinned version → `airsl: 0.1.5 installed, lock pins 0.1.4` (both filled in);
- equal → no finding.

When the lock has no `[airsl]`, `Binary` is never asked. The airsl step runs whatever the file
step found; both sets of findings are returned.

**Proof:** written first, seen failing, then passing.
- `doctor_reports_airsl_missing_from_path` — lock pins `0.1.4`, `FakeBinary` has no airsl: the
  finding `airsl: not found on PATH (lock pins 0.1.4)`, and no version question was asked.
- `doctor_reports_unparseable_airsl_version` — `FakeBinary` answers `airsl version 0.1.4`: the
  finding `airsl: --version printed "airsl version 0.1.4", expected "airsl <version>"`.
- `doctor_reports_a_different_airsl_version` — `FakeBinary` answers `airsl 0.1.5`: the finding
  `airsl: 0.1.5 installed, lock pins 0.1.4`.
- `doctor_accepts_the_pinned_airsl` — `FakeBinary` answers `airsl 0.1.4`: no finding.
- `doctor_skips_airsl_without_a_pin` — a lock with no `[airsl]` and a `FakeBinary` with no airsl:
  no finding, and the `FakeBinary` recorded no question.
- `doctor_reports_files_and_airsl_together` — one changed file and a missing airsl: both findings.

**Done when:** `cargo make dod-crate agentstacks` passes, then `cargo make dod` passes, and the
Definition of Done is green. Commit `feat(agentstacks): doctor checks the airsl pin`.

---

## Risks

- The `Finding` type, its `Display` wording and the drift rule are plan 06's. This flow must hand
  it raw answers (lock entry, `DiskState`, the `Binary` answers) and never re-derive a finding
  itself, or the rule is duplicated in `app/` (`modularity.md` Rule 2). The riskiest step is
  Task 3, where parsing `airsl <semver>` is tempting to write inline; it belongs to the rule.
- The exact `Workspace` and `Binary` method names are plan 09's; the tasks name the behaviour and
  follow whatever plan 09 declares.
- `tests/isolation.rs` refuses `crate::fs`, `crate::process`, `crate::host` and `std::fs` inside
  `src/app`. Test fixtures must come from the fakes, never from a temp directory.
- Not done here: rendering findings and exit codes (spec §16, `cli-commands`); calling
  `check_airsl_presence`, which needs pack manifests that doctor, being offline and
  extension-free, does not load (spec §8 row 2 limits the lock check to the lock's own refusals).

## Proof

- Spec §8 rows covered: row 1 (lock exists) Task 1; row 2 (lock parses) Task 1; row 3 (file
  exists) Task 2; row 4 (regular file, symlink inside counts, §6.3) Task 2; row 5 (hash) Task 2;
  row 6 (airsl on `PATH`) Task 3; row 7 (`--version` parses) Task 3; row 8 (version equal) Task 3;
  "Shared files are not in the lock, so doctor needs no extension" — the signature takes no
  `Adapter` or `Git` (Task 1).
- Spec §14 `app/doctor` row: every doctor row is a unit test on fakes.
- `cargo test -p agentstacks --test isolation` passes (no adapter import in `src/app`).
- `cargo make dod` is green at the end of the plan.
