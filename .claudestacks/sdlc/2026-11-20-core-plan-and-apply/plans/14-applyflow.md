---
status: approved
created: 2026-11-22
depends-on: [06, 09]
built-on: f39b2a814319
---

# Apply flow Implementation Plan

**Goal:** Apply a shown `Plan` through `Workspace` as the only writer.

**Architecture:** `app/apply` becomes a folder of flow files, one per group of spec §7 steps.
- **Calls.** Each file calls the plan 09 `Workspace` port (`resolve`, `stage`, `discard`, `swap`,
  `delete`, `write_lock`) and hands what comes back to a `types/plan` rule (plan 06).
- **Never re-plans** (spec §7). The signature takes only `W: Workspace` and a `&Plan`, so apply
  cannot reach `Git`, `Binary` or `Adapter`.
- **No rule here** (spec §2, `docs/architecture.md` "Where logic lives").
- **Guidelines.** Static dispatch only (`static-dispatch.md`). `mod.rs` holds only exports
  (`mod-rs-export-only.md`).
- **Tests.** Every file has its own colocated tests, which use only `FakeWorkspace`
  (`unit-test-mandate.md`, spec §14 app row). `FakeWorkspace` is `#[cfg(test)]`, so no doc comment
  here carries an example that uses it.

**Tech Stack:** Rust 2024 workspace crate `agentstacks`, with the plan 09 `FakeWorkspace` (files, `fail(WorkspaceCall)`, `set_file`/`remove` for edits between plan and apply, `calls()`, `temps()`, `lock_written()`) and the built `write_lock`.

**Content authority:** spec §7 (steps 1–5, the failure table, "Lock content" as the plan carries it), §16 R1, R17, R21, R28(f), R28(i), §13 rows "`<path>` apply race, I/O" and "`agentstacks.lock` lock write", §14 app row.

---

## Files that change

```
crates/agentstacks/src/app/apply.rs           — delete (plan 01's docs-only stub; replaced by the folder)
crates/agentstacks/src/app/apply/mod.rs       — create: four-question module doc; `mod` lines; `pub use flow::apply`
crates/agentstacks/src/app/apply/stage.rs     — create: steps 2–3: stage temps, check for a race, discard temps on failure
crates/agentstacks/src/app/apply/swap.rs      — create: step 4: swaps and deletes, stopping at the first failure
crates/agentstacks/src/app/apply/lock.rs      — create: step 5: write agentstacks.lock last
crates/agentstacks/src/app/apply/flow.rs      — create: `apply`, steps 1–5 in order
crates/agentstacks/src/app/apply/testing.rs   — create (#[cfg(test)]): builds `Plan` values with plan 06 constructors, for the tests here
```

`src/app/mod.rs` (plan 01) already declares `pub mod apply;`, and a folder satisfies it. This plan
does not edit that file.

## Order of work

### Task 1 — Stage every write and check for a race

**Files:**
- Delete `crates/agentstacks/src/app/apply.rs` — replaced by the folder.
- Create `crates/agentstacks/src/app/apply/mod.rs` — module doc. What: applying a `Plan`. Why: the §7 order is a flow, and it is the only path to a write. Responsibilities per file. Non-responsibilities: no planning, no rule, no `std::fs`. `mod` lines for the files that exist, with `#[cfg(test)] mod testing;`.
- Create `crates/agentstacks/src/app/apply/testing.rs` — builds `Plan` values (plan 06 `Plan::new`, `Step`, `Action`). They hold steps of every `Action`, a shared step, the decided-on `DiskState`s, the old lock entries, and new lock contents from the built `LockContents`/`Lockfile::new`. It also has a `shown(Diagnostics) -> Vec<String>` helper.
- Create `crates/agentstacks/src/app/apply/stage.rs` — steps 2–3.

**Change:** `stage<W: Workspace>(&Plan, &W) -> Result<Staged, Diagnostics>` does two things.

1. **Stage (spec §7 step 2).** In plan order, each step whose `Action::writes()` is true (create,
   update, restore) goes through `Workspace::stage(path, new bytes)`. That writes the temp
   `.<name>.agentstacks-tmp` beside the canonical target (spec §16 R21). Parent creation,
   overwriting a leftover temp and mode copying belong to the adapter (plan 10, spec §7 "Modes").
   Steps whose action is `unchanged`, `delete` or `forget` stage nothing.
2. **Check (spec §7 step 3).** For every step that apply stages or deletes (create, update,
   restore, delete), the flow calls `Workspace::resolve(path)` and hands the answer to plan 06's
   `race(step, fresh)`. `forget` and `unchanged` steps are not checked: they never touch disk
   (plan 06's `race` covers only steps that do; a `forget`'s `decided_on` is `None`). The answers
   become refusals as follows:
   - `Race::Outside` is refused with R21's `<path>: resolves outside the project`;
   - `Race::Changed` is refused with plan 02's `<path>: changed after plan; run plan again`. For a
     shared file, `decided_on` is what `read` returned (spec §7 step 3). A mode-only change is not
     "changed" (R28(i)).

   `Race::refusal` (plan 06) words both.

   Every such step is reported, not only the first. Then every staged temp is removed with
   `Workspace::discard`.

Failure-table row "2 or 3" (spec §7): an I/O failure in `stage` or `resolve` causes four things:
- every temp written so far is discarded;
- nothing is swapped;
- the lock is not written;
- the refusal is `<path>: <io::Error Display>` (R1; built `FormatError::Io`).

Directories already created are left alone. `Staged` is a flow-internal carrier, documented as
such, that lists the staged paths in plan order.

**Proof:** written first, seen failing, then passing.
- `stage_writes_a_temp_only_for_create_update_and_restore` — over a `Plan` with all six actions, `FakeWorkspace::calls()` holds `Stage` for exactly the create, update and restore paths, in plan order.
- `stage_refuses_every_target_changed_after_plan` — two steps changed with `set_file`/`remove` after the `Plan` was built give exactly `<first path>: changed after plan; run plan again` and the same line for the second path. `temps()` is empty, `calls()` holds no `Swap` or `Delete`, and `lock_written()` is `None`.
- `stage_checks_a_shared_file_against_what_read_returned` — a shared `update` step passes when the disk equals the bytes `read` returned. It is refused with `.claude/settings.json: changed after plan; run plan again` when the disk differs.
- `stage_checks_delete_steps_and_skips_unchanged` — a `delete` step whose disk changed after the plan is refused with the race message; an `unchanged` step whose disk changed causes no `Resolve` call and no refusal (it never touches disk, matching plan 06's `race`).
- `stage_skips_the_race_check_for_forget` — a `forget` step whose path is now a dangling symlink, and whose disk changed after the plan, causes no `Resolve` call and no refusal. The apply goes on, and the written lock has no `[[file]]` for it.
- `stage_ignores_a_mode_only_change` — an `update` target whose mode changed after the plan, with its bytes unchanged, passes the check (R28(i)).
- `stage_refuses_a_target_that_now_resolves_outside` — `FakeWorkspace::outside(path)` set after the plan gives exactly `<path>: resolves outside the project`, and every temp is discarded (R21).
- `stage_io_failure_at_the_second_write_discards_the_first_temp` — `fail(Stage(second))` gives exactly `<second path>: <denied() Display>`. `temps()` is empty, there is no `Swap`, and the lock is not written (spec §7 row "2 or 3").
- `stage_io_failure_while_checking_discards_every_temp` — `fail(Resolve(path))` gives `<path>: <denied() Display>`, and `temps()` is empty.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): apply flow stages writes and checks for a race`.

### Task 2 — Swap temps into place and perform deletes

**Files:**
- Create `crates/agentstacks/src/app/apply/swap.rs` — step 4.
- Modify `crates/agentstacks/src/app/apply/mod.rs` — `mod swap;`.

**Change:** `swap<W: Workspace>(&Plan, Staged, &W) -> Swapped` does the swaps first, then the
deletes (spec §7 step 4, in the order the spec writes it):
1. it calls `Workspace::swap(path)` for each staged path, in plan order;
2. it calls `Workspace::delete(path)` for each `delete` step, in plan order.

Three behaviours belong to the adapter (plan 10, spec §6.3, §7, R21):
- a symlink inside the project is written through to its canonical target;
- an updated file keeps its mode;
- a delete leaves empty directories in place.

Failure-table row 4 (spec §7): at the first I/O failure the swap stops. Every temp not yet swapped
is removed with `Workspace::discard`, and the refusal is `<path>: <io::Error Display>` (R1).
`Swapped` is a flow-internal carrier that records which steps landed and holds the failure, if any.
Task 3 uses it to choose the R17 lock.

**Proof:** written first, seen failing, then passing.
- `swap_swaps_every_temp_then_deletes` — `FakeWorkspace::calls()` holds every `Swap` in plan order first, then every `Delete` in plan order.
- `swap_stops_at_the_first_failed_swap` — `fail(Swap(second))` of three gives exactly `<second path>: <denied() Display>`. The third temp is discarded and never swapped, no `Delete` is called, and `temps()` is empty.
- `swap_stops_at_a_failed_delete` — `fail(Delete(path))` gives `<path>: <denied() Display>`. Every swap landed, and later deletes are not called.
- `swap_records_exactly_the_steps_that_landed` — after a failure at the second swap, `Swapped` lists only the first write as landed.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): apply flow swaps temps and performs deletes`.

### Task 3 — Write the lock last

**Files:**
- Create `crates/agentstacks/src/app/apply/lock.rs` — step 5.
- Modify `crates/agentstacks/src/app/apply/mod.rs` — `mod lock;`.

**Change:** `write<W: Workspace>(&Plan, &Swapped, &W) -> Result<(), Diagnostics>` picks the lock
contents.

- **After a full swap**, they are the `Plan`'s new `Lockfile` (spec §7 "Lock content", built by
  plan 06).
- **After a step-4 failure**, they are the R17 lock (spec §16 R17, R28(f)). Plan 06 builds it
  with `lock_contents(tools, packs, steps, airsl, Landed::Only { paths: <landed paths>, old:
  plan.old_files() })`, taking tools, packs and airsl from the `Plan`'s new `Lockfile`:
  - `[[tool]]`, `[[pack]]` and `[airsl]` take the new values;
  - `[[file]]` holds the new entry for each write that landed, and the old entry for each create,
    update, restore or delete that did not;
  - every `forget` and every `unchanged` entry applies, because those steps never touch disk
    (R28(f)).

  The flow passes `Lockfile::new` the result.

The flow turns the lock into text with the built `write_lock` (`src/types/lock/write.rs:58`) and
calls `Workspace::write_lock(text)` (spec §7 step 5). The adapter writes the temp
`.agentstacks.lock.agentstacks-tmp`, renames it, and keeps the lock's mode (R21, plan 10).
Failure-table row 5 (spec §7): an I/O failure gives `agentstacks.lock: <io::Error Display>`, and
the files stay where they landed.

**Proof:** written first, seen failing, then passing.
- `write_lock_writes_the_plans_lock_after_a_full_swap` — `lock_written()` equals `write_lock` of the `Plan`'s `Lockfile`.
- `write_lock_after_a_failed_second_swap_holds_only_the_first` — of two updates, the first landed and the second failed. The written lock has the first file's new hash and the second file's old entry, and its `[[pack]]` revs are the new ones (spec §14 "a fake I/O error at the second rename leaves a lock holding only the first"; R17).
- `write_lock_keeps_the_entry_of_a_delete_not_performed` — after a swap that stopped before a delete, that file's old `[[file]]` is in the written lock.
- `write_lock_applies_forget_and_unchanged_after_a_failure` — after a step-4 failure, a `forget` path has no `[[file]]`, and both an adopted `unchanged` path and a locked `unchanged` path have their new entries (R17, R28(f)).
- `write_lock_keeps_the_old_entry_of_a_restore_not_landed` — a `restore` whose swap was never reached keeps its entry from `Plan::old_files` (R17).
- `write_lock_reports_the_lock_io_error` — `fail(WriteLock)` gives exactly `agentstacks.lock: <denied() Display>`, and the swapped files keep their new bytes in `FakeWorkspace`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): apply flow writes the lock last`.

### Task 4 — Assemble the apply flow

**Files:**
- Create `crates/agentstacks/src/app/apply/flow.rs` — the public flow.
- Modify `crates/agentstacks/src/app/apply/mod.rs` — `mod flow;` and `#[doc(inline)] pub use flow::apply;`.

**Change:** `apply<W: Workspace>(&Plan, &W) -> Result<(), Diagnostics>` runs spec §7 steps 1–5 in
order.
1. **Step 1, "refuse a Plan that holds refusals".** This holds by type: plan 06's `Plan` has no
   refusal field, and `app::plan` returns `Err` on any refusal (plan 13 test
   `plan_with_any_refusal_returns_no_plan`).
2. **Steps 2–3**: Task 1. Any refusal returns before a swap.
3. **Step 4**: Task 2.
4. **Step 5**: Task 3. It always runs after step 4, including after a step-4 failure.
5. Every refusal collected is returned. A step-4 I/O refusal and a lock-write refusal are both
   returned when both happen.

The doc comment states the order in prose and carries no example, because `FakeWorkspace` is
`#[cfg(test)]`. The tests below stand in for it.

**Proof:** written first, seen failing, then passing.
- `apply_runs_stage_check_swap_then_lock_in_order` — `FakeWorkspace::calls()` holds every `Stage`, then a `Resolve` for every step except `forget`, then every `Swap` and `Delete`, then one `WriteLock`.
- `apply_applies_a_forget_of_a_dangling_symlink` — a `Plan` whose only step is a `forget` of a path that is now a dangling symlink applies without a refusal. The symlink is left in place, and the lock written has no `[[file]]` for that path.
- `apply_touches_nothing_on_a_race` — a target changed after the plan gives `<path>: changed after plan; run plan again`. `calls()` holds no `Swap`, `Delete` or `WriteLock`.
- `apply_writes_the_r17_lock_after_a_failed_swap` — a failure at the second swap returns `<second path>: <denied() Display>`. The written lock holds the first file's new hash and the second file's old entry (spec §14 app row, R17).
- `apply_reports_a_swap_failure_and_a_lock_failure_together` — `fail(Swap(…))` plus `fail(WriteLock)` return both refusals.
- `apply_leaves_unchanged_and_forget_steps_alone_on_disk` — those steps cause no `Stage`, `Swap` or `Delete`. The written lock follows the `Plan`: an `unchanged` file has its `[[file]]`, and a `forget` path has none.

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit `feat(agentstacks): apply flow as the only writer`.

---

## Risks

- **Seams with plans 06 and 09.** Port names follow plan 09: `resolve`, `stage`, `discard`,
  `swap`, `delete`, `write_lock`. Rule names follow plan 06: `race`, `Race`, `Race::refusal`,
  `lock_contents` with `Landed::Only`, `Plan::old_files`, and `Step.decided_on: Option<DiskState>`
  (`None` for `forget`). If plan 06 renames one, the coder follows plan 06. The behaviour cited
  here stays.
- **Riskiest task: Task 2.** Stopping mid-swap must leave no temp behind and must hand Task 3 the
  exact landed set. The tests inject the failure at the first swap, a middle swap and a delete.
- **Not done here:**
  - fs contract tests (plan 10, spec §14 fs row): modes, write-through, parent creation, leftover
    temps, the lock temp name and mode (R21), empty directories after a delete;
  - showing the plan and choosing to apply (cli-commands, spec §17).

## Proof

- Spec rows covered:

  | Rows | Tasks |
  |---|---|
  | §7 step 1 (by type) | 4; plan 13 Task 8 |
  | §7 step 2, with temps beside the canonical target (R21) | 1 |
  | §7 step 3, including shared files, every differing path, outside at apply time (R21), no check for `forget`, mode-only change (R28(i)) | 1, 4 |
  | §7 step 4, in the order written | 2 |
  | §7 step 5, last via `write_lock` | 3 |
  | §7 failure row "2 or 3" | 1 |
  | §7 failure row "4", with the R17 lock | 2–3 |
  | §7 failure row "5" | 3 |
  | §7 "Lock content" | 3 |
  | §16 R1 | 1–3 |
  | §16 R17, R28(f) | 3–4 |
  | §16 R21 | 1, 3 |
  | §13 apply race and I/O locations | 1–3 |
  | §14 app row: the apply race, each failure row, the second-rename partial lock | 1–4 |

- Every test uses only `FakeWorkspace`. `apply` takes no `Git`, `Binary` or `Adapter`, so it cannot re-plan.
- `cargo make dod` is green at the end of Task 4.
