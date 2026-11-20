---
status: approved
created: 2026-11-20
depends-on: [01, 02, 03]
built-on: 5c6b28af8487
---

# CLAUDE.md Update Implementation Plan

**Goal:** `CLAUDE.md` describes the workspace as it now runs.

**Architecture:** A docs-only change to two sections of the project `CLAUDE.md`. The status section
stops claiming the repo is pre-code; the Commands section lists the commands that now work. All
other sections (design, invariants, planned layout, porting notes) are unchanged. Progress stays
on the sdlc board, not in prose.

**Tech Stack:** Markdown.

**Content authority:** `intent.md` (spec skipped) — Affected systems (`CLAUDE.md` Commands and
"Status: pre-code" sections), Desired outcome bullet 5.

---

## Files that change

```
CLAUDE.md   — modify  status section and Commands section
```

## Order of work

### Task 1 — Update the status and Commands sections of `CLAUDE.md`

**Files:**
- Modify `CLAUDE.md` —
  - "Status: pre-code" section (heading at `CLAUDE.md:5`, body through line 11): retitled to a
    status that no longer says pre-code. Body states what exists after plans 01-03: a Cargo
    workspace with `crates/agentstacks` (empty seams) and `crates/agentstacks-cli` (answers
    `--version` only), the cargo-make gate, cargo-deny policy and CI. It keeps the facts that
    remain true: everything in the design sections is still the contract to build toward,
    `extensions/`, `packs/`, `book/` and the claudevs move have not landed, and the code to move
    still lives in claudestacks. It drops "no `.claudestacks/sdlc/` chain yet" (`CLAUDE.md:8`),
    which is already false, and points to `/claudestacks-sdlc:status` for progress.
  - "Commands" section (heading at `CLAUDE.md:86`): the "None work yet. When the workspace exists,
    it uses…" lead-in (`CLAUDE.md:88`) becomes a present-tense statement that these commands run
    today; the command block stays; the gate-source and lint sentences stay. Add one line that CI
    runs `cargo make dod` and `cargo deny check` on push to `main` and on pull requests.

**Change:** No reader of `CLAUDE.md` is told the workspace or commands do not exist.

**Proof:** No Rust tests (docs only). Checks:
- `grep -nE 'pre-code|None work yet|no Cargo workspace' CLAUDE.md` prints nothing.
- Each command listed in the Commands block runs: `cargo make dod`, `cargo make dod-crate
  agentstacks`, `cargo make fmt` (no diff), `cargo make deny`.

**Done when:** the checks hold and `cargo make dod` (the stack's Definition of Done) is green.
Commit `docs(repo): describe the working workspace in CLAUDE.md`.

---

## Risks

- Over-editing: rewriting design sections that are still "planned". Only the two named sections
  change.
- The org-level docs are outside this repository and are not touched by this chain. The status
  column stays "Planned" (`/Users/hiraq/Projects/airsstack/README.md:102`), so the org rule to
  update on a status change does not fire. Two descriptive lines do go stale once this chain
  lands — the workspace `README.md:93` ("no code yet") and the org `CLAUDE.md` project map —
  and are an org-root follow-up after merge, outside this plan.

## Proof

- `CLAUDE.md` reads consistently with `git ls-files` on the branch: every file it says exists is
  tracked.
