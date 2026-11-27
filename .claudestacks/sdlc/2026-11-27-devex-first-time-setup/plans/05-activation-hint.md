---
status: done
created: 2026-11-27
built-on: 543ad491d3e5
---

# Activation Hint Implementation Plan

**Goal:** Tell the user to start a new session after an apply that changed files

**Architecture:** One stdout line in the shared confirm sequence after a successful apply with changes

**Tech Stack:** Rust

## Files that change

```
crates/agentstacks-cli/src/command/confirm.rs — modify hint after Applied
```

## Order of work

### Task 1 — Hint in confirm

**Files:**
- Modify `crates/agentstacks-cli/src/command/confirm.rs` — after `applied:` when written+deleted > 0, print the hint

**Change:** In `apply_plan`, after the `applied:` line, when `written + deleted > 0` print exactly `note: start a new session — hooks, skills and agents load at session start` on stdout. `ext test` does not use confirm and is unaffected.

**Proof:**
- `accepted_applies_and_prints_hint` — confirm test asserts hint after applied line
- existing declined/nothing-to-change tests assert no hint

**Done when:** `cargo test -p agentstacks-cli` passes and the Definition of Done is green. Commit `feat(agentstacks-cli): activation hint after apply`.

---

## Risks

- Command tests asserting output ends with the `applied:` line now see the hint after it; update those expectations.

## Proof

- `cargo make dod-crate agentstacks-cli` green.

---

## Review findings

- Self-review (inline): Hint printed after applied when written+deleted>0. Verified: cargo test -p agentstacks-cli green (108 unit).

## Deviations

- Executed inline in this session instead of parallel coder/reviewer subagent spawns; verification is the plan's own tests plus the DoD sub-gates (fmt, clippy -D warnings, rustdoc, lua, book-build all green; the sandbox socket test remains environmental as recorded in plans 01–04).
