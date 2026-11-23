---
status: approved
created: 2026-11-23
depends-on: [02, 04, 15]
built-on: 0f89fdb3ebb3
---

# Project docs Implementation Plan

**Goal:** Make `CLAUDE.md` describe the packs and the claudevs crates as built.

**Architecture:** Only `CLAUDE.md` changes here. The API docs change with the code they describe:
- `docs/hooks.md` in plan 07;
- `docs/extensions.md`, `docs/authoring.md`, `docs/cli.md:120` and `docs/plan-and-apply.md` in
  plans 05 and 06;
- the official pack lists in plan 04.

The org docs do not change (spec §12: the profile names no individual packs, and the status
changes at release).

**Tech Stack:** Markdown.

**Content authority:** spec §12 (`CLAUDE.md` bullets, org docs bullet), §13.

---

## Files that change

```
CLAUDE.md   — modify: Status, Planned layout, the porting section
```

## Order of work

### Task 1 — Status and layout

**Files:**
- Modify `CLAUDE.md`.

**Change:**
- `## Status` becomes "core, CLI commands, claude extension, packs and claudevs built".
  - The tree gains `crates/claudevs/` ("deterministic Claude Code setup test engine"),
    `crates/claudevs-cli/` and `packs/` ("the five official packs").
  - The paragraph that says `packs/` and the claudevs move have not landed now names only `book/`
    as not landed. The "code that will move here still lives in claudestacks" sentence goes.
- `## Planned layout`: `crates/claudevs{,-cli}/` and `packs/` are marked built. The pack list stays
  the five from plan 04.
- `## Porting claudestacks plugins into packs`: the table is replaced by one paragraph. It says the
  plugins are converted, and that the mapping, renames, hook table and text rules are in
  `.claudestacks/sdlc/2026-11-20-packs-and-claudevs/spec.md` §6–§7. The archive sentence stays, as
  `release-and-migration` work (spec §13).
- `## Commands` gains `cargo make claudevs-check` (fixture corpus, not in `dod`) and notes that `cargo
  make lua` now checks and tests `extensions/` and `packs/` and runs the fingerprint gate.

**Proof:**
- `grep -n 'have not landed\|still lives in' CLAUDE.md` prints at most the `book/` line.
- `grep -n 'claudevs-check' CLAUDE.md` prints the Commands line.
- `grep -n cmux CLAUDE.md` prints nothing.

**Done when:** the greps hold and `cargo make dod` is green. Commit
`docs(repo): CLAUDE.md status for packs and claudevs`.

---

## Risks

- `CLAUDE.md` says to update the org profile when status changes. Spec §12 decides no org doc
  changes in this chain (status moves at release). The commit body says so, so a reader does not
  take it as an omission.

## Proof

- `cargo make dod` green.
