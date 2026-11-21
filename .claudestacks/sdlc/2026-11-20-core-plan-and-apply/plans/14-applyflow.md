---
status: draft
created: 2026-11-22
depends-on: [06, 09]
built-on: ea104af7451e
---

# Apply flow Implementation Plan

**Goal:** Apply a shown `Plan` through `Workspace` as the only writer.

**Architecture:** `app/apply` becomes a folder of flow files, one per group of spec §7 steps. Each
file calls the `Workspace` port (plan 09) and passes what comes back to a `types/plan` rule (plan
06). Apply never re-plans (spec §7). Its signature takes only `W: Workspace` and a `&Plan`, so it
cannot reach `Git`, `Binary` or `Adapter`. No rule lives here (spec §2, `docs/architecture.md`
"Where logic lives"). Static dispatch only (`static-dispatch.md`). `mod.rs` holds only exports
(`mod-rs-export-only.md`). Every file has its own colocated tests, which use only `FakeWorkspace`
(`unit-test-mandate.md`, spec §14 app row).

**Tech Stack:** Rust 2024 workspace crate `agentstacks`, with the plan 09 `FakeWorkspace` (scripted disk states and an I/O failure that can be injected at any call) and the built `write_lock`.

**Content authority:** spec §7 (steps 1–5, the failure table, "Lock content" as the plan carries it), §13 rows "`<path>` apply race, I/O" and "`agentstacks.lock` lock write", §14 app row.

---

## Files that change

```
crates/agentstacks/src/app/apply.rs           — delete (plan 01's docs-only stub; replaced by the folder)
crates/agentstacks/src/app/apply/mod.rs       — create: four-question module doc; `mod` lines; `pub use flow::apply`
crates/agentstacks/src/app/apply/stage.rs     — create: steps 2–3, staging temps, the race check, removing temps on failure
crates/agentstacks/src/app/apply/swap.rs      — create: step 4, renames and deletes, stopping at the first failure
crates/agentstacks/src/app/apply/lock.rs      — create: step 5, writing agentstacks.lock last
crates/agentstacks/src/app/apply/flow.rs      — create: `apply`, steps 1–5 in order
crates/agentstacks/src/app/apply/testing.rs   — create (#[cfg(test)]): builds `Plan` values over plan 06 constructors for the tests here
```

`src/app/mod.rs` (plan 01) already declares `pub mod apply;`, and a folder satisfies it. This plan
does not edit that file.

## Order of work

### Task 1 — Stage every write and check for a race

**Files:**
- Delete `crates/agentstacks/src/app/apply.rs` — replaced by the folder.
- Create `crates/agentstacks/src/app/apply/mod.rs` — module doc (what: applying a `Plan`; why: the order of §7 is a flow and the only path to a write; responsibilities per file; non-responsibilities: no planning, no rule, no `std::fs`). `mod` lines for the files that exist, with `#[cfg(test)] mod testing;`.
- Create `crates/agentstacks/src/app/apply/testing.rs` — builds `Plan` values with steps of every `Action` (create, update, restore, unchanged, delete, forget), a shared step, and lock contents built with the built `LockContents`/`Lockfile::new`. It also has a `shown(&Diagnostics) -> Vec<String>` helper.
- Create `crates/agentstacks/src/app/apply/stage.rs` — steps 2–3.

**Change:** `stage<W: Workspace>(&Plan, &W) -> Result<Staged, Diagnostics>` does two things.

1. **Stage (spec §7 step 2).** In plan order, each `create`, `update` or `restore` step is staged
   through `Workspace`, which writes `<dir>/.<name>.agentstacks-tmp` beside the step's (canonical)
   target. Creating parents, overwriting a leftover temp and copying the mode are `fs` work (plan
   10, spec §7 "Modes"). `unchanged`, `delete` and `forget` steps stage nothing.
2. **Check (spec §7 step 3).** The flow reads each step's current disk state through `Workspace`,
   for every step in the `Plan`. It compares that state with the step's `decided_on` using the
   plan 06 race rule. For a shared file, `decided_on` is what `read` returned. Each step that
   differs gives the plan 02 refusal `<path>: changed after plan; run plan again`, and every such
   step is reported, not only the first. Then every staged temp is removed through `Workspace`.

Failure-table row "2 or 3" (spec §7): if staging, checking or reading fails with an I/O error,
every temp written so far is removed. Nothing is swapped and the lock is not written. The refusal
is the built I/O shape `<path>: <io::Error Display>` (`FormatError::Io`). Directories already
created are left alone. `Staged` is a flow-internal carrier, documented as such, that lists the
temps written in plan order.

**Proof:** written first, seen failing, then passing.
- `stage_writes_a_temp_only_for_create_update_and_restore` — over a `Plan` holding all six actions, `FakeWorkspace` records staging for exactly the create, update and restore paths, in plan order.
- `stage_refuses_every_target_changed_after_plan` — two steps whose disk state changed give exactly `<first path>: changed after plan; run plan again` and the same for the second. Every temp is removed, nothing is renamed, and the lock is not written.
- `stage_checks_a_shared_file_against_what_read_returned` — a shared `update` step passes when the disk equals the bytes `read` returned, and is refused with `.claude/settings.json: changed after plan; run plan again` when the disk differs.
- `stage_checks_steps_that_write_nothing` — an `unchanged` step and a `delete` step whose disk changed are both refused with the race message.
- `stage_io_failure_at_the_second_write_removes_the_first_temp` — a fake I/O error on the second staging gives exactly `<second path>: <the error's Display>`. The first temp is removed, nothing is renamed, and the lock is not written (spec §7 row "2 or 3").
- `stage_io_failure_while_checking_removes_every_temp` — a fake I/O error reading a disk state at step 3 gives `<path>: <the error's Display>`, and every temp is removed.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): apply flow stages writes and checks for a race`.

### Task 2 — Swap temps into place and perform deletes

**Files:**
- Create `crates/agentstacks/src/app/apply/swap.rs` — step 4.
- Modify `crates/agentstacks/src/app/apply/mod.rs` — `mod swap;`.

**Change:** `swap<W: Workspace>(&Plan, Staged, &W) -> Swapped` renames each staged temp onto its
(canonical) target through `Workspace`, in plan order. After that it performs each `delete` step
through `Workspace`, in plan order (spec §7 step 4, in the order the spec writes it). A symlink
inside the project is written through to its canonical target, and a delete leaves empty
directories in place. Both are `fs` behaviour (plan 10, spec §6.3, §7).

Failure-table row 4 (spec §7): at the first I/O failure the swap stops. Every temp not yet renamed
is removed through `Workspace`, and the refusal is `<path>: <io::Error Display>` (`FormatError::Io`).
`Swapped` is a flow-internal carrier. It records which steps landed and holds the failure, if
there is one. Task 3 uses it to choose between the plan's lock and the "only what landed" lock.

**Proof:** written first, seen failing, then passing.
- `swap_renames_every_temp_then_performs_every_delete` — `FakeWorkspace` records every rename in plan order first, then every delete in plan order.
- `swap_stops_at_the_first_failed_rename` — a fake I/O error at the second of three renames gives exactly `<second path>: <the error's Display>`. The third temp is removed, it is never renamed, and no delete is performed.
- `swap_stops_at_a_failed_delete` — a fake I/O error on a delete gives `<path>: <the error's Display>`. Every rename landed, and the later deletes are not performed.
- `swap_records_exactly_the_steps_that_landed` — after a failure at the second rename, `Swapped` lists only the first step as landed.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): apply flow swaps temps and performs deletes`.

### Task 3 — Write the lock last

**Files:**
- Create `crates/agentstacks/src/app/apply/lock.rs` — step 5.
- Modify `crates/agentstacks/src/app/apply/mod.rs` — `mod lock;`.

**Change:** `write<W: Workspace>(&Plan, &Swapped, &W) -> Result<(), Diagnostics>` picks the lock
contents and writes them. After a full swap, the contents are the `Plan`'s new lock contents (spec
§7 "Lock content", built by plan 06 and carried in the `Plan`). After a step-4 failure, they are
what the plan 06 lock-content builder gives for the landed steps alone (spec §7 row 4):
- renamed files carry their new hash;
- every other file keeps its old lock entry;
- a delete that was not performed keeps its entry.

The flow turns the contents into text with the built `write_lock` (`src/types/lock/write.rs:58`)
and writes the text through `Workspace`'s lock write, a temp plus rename (spec §7 step 5).
Failure-table row 5 (spec §7): an I/O failure gives `agentstacks.lock: <io::Error Display>`, and
the files stay where they landed.

**Proof:** written first, seen failing, then passing.
- `write_lock_writes_the_plans_lock_text_after_a_full_swap` — the text `FakeWorkspace` receives equals `write_lock` of the `Plan`'s lock contents.
- `write_lock_after_a_failed_second_rename_holds_only_the_first` — the swap landed the first of two updates and failed the second. The written lock has the first file's new hash and the second file's old entry (spec §14: "a fake I/O error at the second rename leaves a lock holding only the first").
- `write_lock_keeps_the_entry_of_a_delete_not_performed` — after a swap that stopped before a delete, that file's old `[[file]]` entry is still in the written lock.
- `write_lock_reports_the_lock_io_error` — a fake I/O error on the lock write gives exactly `agentstacks.lock: <the error's Display>`, and the renames already recorded by `FakeWorkspace` stay in place.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): apply flow writes the lock last`.

### Task 4 — Assemble the apply flow

**Files:**
- Create `crates/agentstacks/src/app/apply/flow.rs` — the public flow.
- Modify `crates/agentstacks/src/app/apply/mod.rs` — `mod flow;` and `pub use flow::apply;` with `#[doc(inline)]`.

**Change:** `apply<W: Workspace>(&Plan, &W) -> Result<(), Diagnostics>` runs spec §7 steps 1–5 in
order:
1. **Step 1, "refuse a Plan that holds refusals".** This holds by type: plan 06's `Plan` has no
   refusal field, and `app::plan` returns `Err` on any refusal (plan 13 test
   `plan_with_any_refusal_returns_no_plan`).
2. Stage and check (Task 1). Any refusal returns before a swap.
3. Swap (Task 2).
4. Write the lock (Task 3). It always runs after step 4, including after a step-4 failure.
5. Return every refusal collected. A step-4 I/O refusal and a lock-write refusal are both
   returned when both happen.

The doc comment carries a runnable example over `FakeWorkspace`.

**Proof:** written first, seen failing, then passing.
- `apply_runs_stage_check_swap_then_lock_in_order` — `FakeWorkspace`'s call log holds every staging, then every disk-state read, then every rename and delete, and finally the one lock write.
- `apply_touches_nothing_on_a_race` — one target changed after plan gives `<path>: changed after plan; run plan again`. The log holds no rename, no delete and no lock write.
- `apply_writes_a_partial_lock_after_a_failed_rename` — a failure at the second rename returns `<second path>: <the error's Display>`, and the lock written holds only the first file's new hash (spec §14 app row).
- `apply_reports_a_swap_failure_and_a_lock_failure_together` — a rename failure followed by a lock-write failure returns both refusals.
- `apply_leaves_unchanged_and_forget_steps_alone_on_disk` — those steps cause no staging, rename or delete, and the lock text still follows the `Plan` (an `unchanged` file has its `[[file]]`, a `forget` path has none).

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit `feat(agentstacks): apply flow as the only writer`.

---

## Risks

- **Seams with plans 06 and 09.** This plan assumes these calls exist:
  - plan 09 `Workspace`: staging a step, reading a step's disk state, renaming a temp, removing a
    temp, deleting a path, writing the lock text;
  - plan 06: the race comparison of a `DiskState` with `decided_on`, and the lock-content builder
    for the "only what landed" case.

  If either plan names these calls differently, the coder follows that plan's names. The
  behaviour cited here stays.
- **Riskiest task: Task 2.** Stopping mid-swap must leave no temp behind and must hand Task 3 the
  exact set that landed. The tests inject the failure at each position: first rename, middle
  rename, delete.
- **Not done here:**
  - modes, symlink write-through, parent creation, overwriting a leftover temp, and empty
    directories after a delete: `fs::StdWorkspace` contract tests (plan 10, spec §14 fs row);
  - showing the plan and choosing to run apply: cli-commands (spec §16).

## Proof

- Spec rows covered:
  - §7 step 1 (by type; Task 4, plus plan 13 Task 8);
  - §7 step 2 (Task 1), step 3 including shared files and every differing path (Task 1);
  - §7 step 4 in written order (Task 2), step 5 last via `write_lock` (Task 3);
  - §7 failure rows "2 or 3" (Task 1), "4" (Tasks 2–3), "5" (Task 3);
  - §7 "Lock content", as carried by the `Plan` (Task 3);
  - §13 apply race and I/O locations (Tasks 1–3);
  - §14 app row: the apply race, each failure row, the second-rename partial lock (Tasks 1–4).
- Every test uses only `FakeWorkspace`. `apply` takes no `Git`, `Binary` or `Adapter`, so it cannot re-plan.
- `cargo make dod` is green at the end of Task 4.
