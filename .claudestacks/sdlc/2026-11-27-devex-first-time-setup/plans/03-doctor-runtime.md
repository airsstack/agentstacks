---
status: done
created: 2026-11-27
built-on: 543ad491d3e5
---

# Doctor Runtime Implementation Plan

**Goal:** Report airsl runtime state at doctor time on every lock path

**Architecture:** `airsl_finding(Option<&pin>, answer)` rule in types; doctor flow always asks the Binary port

**Tech Stack:** Rust

## Files that change

```
crates/agentstacks/src/types/plan/drift.rs   — modify Option pin matrix
crates/agentstacks/src/types/plan/finding.rs — modify NotFound suffix, add AirslMissing
crates/agentstacks/src/app/doctor.rs         — modify always ask Binary
```

## Order of work

### Task 1 — Finding rule matrix

**Files:**
- Modify `crates/agentstacks/src/types/plan/finding.rs` — texts per spec §3
- Modify `crates/agentstacks/src/types/plan/drift.rs` — `airsl_finding(Option<&PackVersion>, AirslAnswer)`

**Change:** Implement the §3 matrix verbatim: NotFound with/without pin (install suffix), parseable equal/different/no-pin, Unparsed and Spawn unchanged.

**Proof:**
- `airsl_missing_without_pin` — exact unpinned text
- `airsl_not_found_with_pin_names_install` — exact pinned text
- `airsl_parseable_without_pin_is_healthy` — no finding

**Done when:** `cargo test -p agentstacks types::plan` passes and the Definition of Done is green. Commit `feat(agentstacks): doctor airsl runtime findings`.

### Task 2 — Doctor flow always checks

**Files:**
- Modify `crates/agentstacks/src/app/doctor.rs` — ask Binary on missing/unreadable/refused lock too, airsl finding last

**Change:** Remove early returns that skipped the binary; collect lock findings, then append the airsl finding.

**Proof:**
- `doctor_reports_airsl_with_missing_lock` — two findings in order
- `doctor_reports_airsl_with_refused_lock` — lock refusals then airsl

**Done when:** `cargo test -p agentstacks app::doctor` passes and the Definition of Done is green. Commit `feat(agentstacks): doctor checks airsl without a lock`.

---

## Risks

- CLI doctor tests asserting `healthy`/single-finding output need the binary fake configured; update them to the matrix.

## Proof

- `cargo make dod-crate agentstacks` green except the known sandbox socket failure.

---

## Review findings

- Self-review (inline): airsl_finding(Option<pin>) matrix with AirslMissing; doctor always asks Binary on every lock path. Verified with plan 02 run. Fixed on review: fmt, clippy `map_or_else`/`String::len`, test `panic!` removal.

## Deviations

- Executed inline in this session instead of parallel coder/reviewer subagent spawns; verification is the plan's own tests plus `cargo make dod` sub-gates.
- `cargo make dod` test stage failed in the sandbox only on `fs::disk::tests::a_unix_socket_is_other`: the sandbox blocks `AF_UNIX bind` everywhere (proven with a Python probe in three directories). Re-run outside the sandbox on 2026-11-27 at the author's direction: the test passes and the full core suite is 1864 passed, 0 failed — environmental, no code fix needed. fmt, clippy -D warnings, rustdoc and doctests (312+1) passed.
- Plans 05–07 of the spec (activation hint, init --dry-run, book) were not drafted or executed in this run; the chain stays open.
