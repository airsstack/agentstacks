---
status: done
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

## Review findings

Reviewer: no blocking findings; every "built" claim checked against the tree (`crates/claudevs{,-cli}` exist with tests, `packs/` holds exactly the five packs, `extensions/` only claude, no `book/`; `Makefile.toml` `dod` depends on `lua`; `claudevs-check` not in `dod`). Greps: `grep -n 'have not landed\|still lives in' CLAUDE.md` prints nothing (line 24 reads "`book/` has not landed"); `grep -n 'claudevs-check' CLAUDE.md` → `133:cargo make claudevs-check      # claudevs fixture corpus; deliberately not part of dod`; `grep -n cmux CLAUDE.md` prints nothing. Workspace `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` exit 0 (24 `test result: ok`).

- 🟡 open — `CLAUDE.md:129`: the `dod` comment lists five steps without `lua`, while `:134` says `lua` is part of `dod`.
- 🟡 open — `CLAUDE.md:20,138`: CI is said to run `dod` and `cargo deny check`; `ci.yml` also has a `claudevs-check` job (lines predate the diff).
- 🔵 open — `CLAUDE.md:130`: "dod-crate … the same steps" — `dod-crate` runs no `lua` (predates the diff).
- 🔵 open — `CLAUDE.md:23`: "not a description of existing files" no longer fits the Porting paragraph and the "(built)" marks.
- 🔵 open — `CLAUDE.md:106`: "claude is built" added beyond the plan (accurate).
- 🔵 open — `CLAUDE.md:121`: the "(spec §13)" cite does not mention archiving claudestacks (copied from the plan).
- 🔵 open — outside scope: `README.md:140-141` still lists `claudevs`/`claudevs-cli` as planned.

## Deviations

- 2026-11-24 — Started while plan 15 (its `depends-on`) was in review, after its test was green; it touches only `CLAUDE.md`.
- 2026-11-24 — The `cargo make lua` line says "(part of dod)" (`Makefile.toml` `dod` depends on `lua`); the existing `dod` comment was left as is (see the open 🟡).
- 2026-11-24 — No org docs touched (spec §12: status moves at release).
- 2026-11-24 — Not committed.
