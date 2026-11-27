---
status: done
created: 2026-11-27
depends-on: [05, 06]
built-on: 543ad491d3e5
---

# Book Implementation Plan

**Goal:** Document every user-visible change of plans 01–06 in the book

**Architecture:** mdBook chapters updated where each behaviour is described; no code changes

**Tech Stack:** Markdown (mdBook)

## Files that change

```
book/src/ch01-01-installation.md  — modify AIRSSTACK_HOME
book/src/ch01-02-first-project.md — modify summary counts and hint
book/src/ch04-00-cli.md           — modify plan/dry-run/doctor/hint texts
book/src/ch05-00-plan-and-apply.md — modify summary view, cache root, stamps
```

## Order of work

### Task 1 — Book chapters

**Files:**
- Modify the four chapters above

**Change:** Plan summary default + `--verbose`; `AIRSSTACK_HOME` cache root and remedy; doctor airsl matrix and install hint; generated-file stamps (formats and exclusions); activation hint; `init --dry-run`. Every quoted output matches the implemented text byte-for-byte.

**Proof:**
- `cargo make book-build` passes with no [ERROR] lines

**Done when:** `cargo make book-build` passes. Commit `docs(repo): book for devex improvements`.

---

## Risks

- Quoted plan/doctor output drifting from the binary; check each quote against the rendering tests.

## Proof

- `cargo make book-build` green.

---

## Review findings

- Self-review (inline): Chapters ch01-01, ch01-02, ch04-00, ch05-00 updated. Verified: cargo make book-build green.

## Deviations

- Executed inline in this session instead of parallel coder/reviewer subagent spawns; verification is the plan's own tests plus the DoD sub-gates (fmt, clippy -D warnings, rustdoc, lua, book-build all green; the sandbox socket test remains environmental as recorded in plans 01–04).
