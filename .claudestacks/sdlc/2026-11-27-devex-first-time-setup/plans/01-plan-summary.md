---
status: done
created: 2026-11-27
built-on: 543ad491d3e5
---

# Plan Summary Implementation Plan

**Goal:** Show per-owner plan counts by default with the full list under `--verbose`

**Architecture:** `PlanView::{Summary, Full}` in the CLI show layer; `Full` is today's `plan_text` unchanged, `Summary` groups steps by owner

**Tech Stack:** Rust, clap 4.6.9

## Files that change

```
crates/agentstacks-cli/src/cli.rs        — modify plan flags
crates/agentstacks-cli/src/show/plan.rs  — modify PlanView and summary rendering
crates/agentstacks-cli/src/command/plan.rs — modify pass view
crates/agentstacks-cli/src/command/confirm.rs — modify show summary by default
```

## Order of work

### Task 1 — PlanView and summary rendering

**Files:**
- Modify `crates/agentstacks-cli/src/show/plan.rs` — `PlanView`, `plan_text_in(view)`, per-owner rows

**Change:** Add `PlanView::{Summary, Full}`. Keep existing rendering as Full. Summary: header, owner rows (`{owner:<width}  {w} to write, {d} to delete, {u} unchanged`, byte-sort, `shared` last, Forget excluded per-owner), then skips, airsl/notes sections, final summary line. Blank line before each non-empty section.

**Proof:**
- `summary_renders_per_owner_counts` — fixture from spec §1 renders byte-exact owner rows and final line
- `summary_excludes_forget_from_per_owner_delete` — Forget only in final suffix
- `full_view_is_unchanged` — existing spec-example test passes unmodified

**Done when:** `cargo test -p agentstacks-cli show::plan` passes and the Definition of Done is green. Commit `feat(agentstacks-cli): plan summary view`.

### Task 2 — CLI flags and command wiring

**Files:**
- Modify `crates/agentstacks-cli/src/cli.rs` — `Plan { summary, verbose }` conflicting flags
- Modify `crates/agentstacks-cli/src/command/plan.rs` — select view
- Modify `crates/agentstacks-cli/src/command/confirm.rs` — default Summary in confirm

**Change:** Neither flag = Summary; `--summary` = Summary; `--verbose` = Full; both = clap exit 2. Confirm sequence shows Summary (ext test unchanged).

**Proof:**
- `plan_summary_and_verbose_conflict` — clap exit 2 (cli tests)
- `plan_prints_summary_by_default` — command test output has owner rows, no per-file rows

**Done when:** `cargo test -p agentstacks-cli` passes and the Definition of Done is green. Commit `feat(agentstacks-cli): plan --summary and --verbose flags`.

---

## Risks

- Golden plan tests for apply/init/update change because confirm now shows Summary; update expectations to summary text, keep Full assertions in show tests.

## Proof

- `cargo make dod-crate agentstacks-cli` green; book text for plan output updated in plan 07 (deferred with plans 05–07).

---

## Review findings

- Self-review (inline): PlanView Summary/Full implemented; summary per-owner counts with Forget excluded per-owner. Verified: cargo test -p agentstacks-cli (106+18+20+1 green after fix). Fixed on review: fmt, clippy `map_or_else`/`String::len`, test `panic!` removal.

## Deviations

- Executed inline in this session instead of parallel coder/reviewer subagent spawns; verification is the plan's own tests plus `cargo make dod` sub-gates.
- `cargo make dod` test stage failed in the sandbox only on `fs::disk::tests::a_unix_socket_is_other`: the sandbox blocks `AF_UNIX bind` everywhere (proven with a Python probe in three directories). Re-run outside the sandbox on 2026-11-27 at the author's direction: the test passes and the full core suite is 1864 passed, 0 failed — environmental, no code fix needed. fmt, clippy -D warnings, rustdoc and doctests (312+1) passed.
- Plans 05–07 of the spec (activation hint, init --dry-run, book) were not drafted or executed in this run; the chain stays open.
