---
status: done
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

## Review findings

One `claudestacks:reviewer` pass over the plans 03 and 04 diff. It re-ran `cargo make dod`,
`dod-crate agentstacks`, `dod-crate agentstacks-cli` and `cargo make deny`; all exited 0.

- 🟡 blocking, CLAUDE.md:16 — "Everything below the Status section describes the agreed design, not
  existing files" contradicted the present-tense Commands section. **Fixed**: the sentence now
  reads "…below the Status section except Commands…".
- 🔵 CLAUDE.md:11 — "answers --version only" left out `--help`. **Fixed**: it now reads "answers --version and --help only".
- 🔵 CLAUDE.md:12 — the `deny.toml` row read as a run-on phrase. **Fixed**: the files are now
  separated by a comma and aligned.
- 🔵 CLAUDE.md:108 — "every push to `main`" overstated the trigger and left out manual dispatch. **Fixed**: the line now lists push to
  `main`, pull requests and manual dispatch.
- 🔵 CLAUDE.md:13 — it lists ci.yml as existing while the file was untracked. This is resolved by
  committing plan 03 before plan 04.
- 🔵 open — the org root `CLAUDE.md:106` still says `cargo make fmt` is "the only task that rewrites
  the tree". That is an org-root follow-up outside this repo.
- Verified after the fixes: `cargo make dod` → exit 0. `grep -nE 'pre-code|None work yet|no Cargo
  workspace' CLAUDE.md` → no output (exit 1). `cargo make dod-crate agentstacks`, `cargo make fmt`
  (no source diff) and `cargo make deny` → exit 0 each, from the pre-review run.

## Deviations

- 2026-11-20 — The plan said "the command block stays". The `cargo make fmt` comment changed anyway,
  from "the only task that rewrites the tree" to "rewrite formatting (clippy-fix is the other task
  that edits sources)". The old claim was false: `Makefile.toml` also has `clippy-fix`. The reviewer
  confirmed the new wording is accurate.
- 2026-11-20 — Built inline by the main thread rather than by a `claudestacks:coder` spawn. It is a
  docs-only edit.
- 2026-11-20 — The chain is complete, so `intent.md` was flipped to `done`. The milestone roll-up (§12)
  was **not** applied to `foundation/01`. This chain is that milestone's only member, so the
  mechanical rule would flip it to `done`. But the milestone's goal (claude extension, packs,
  claudevs move, a 0.x release) is unmet, and milestone `done` is final. More chains have to join
  `foundation/01` before it can roll up.
