---
status: approved
created: 2026-11-24
built-on: a72a2ee643bb
---

# Org docs Implementation Plan

**Goal:** Make the org docs show agentstacks as pre-release and claudestacks as archived.

**Architecture:** Three files outside this repository, in the org root
`/Users/hiraq/Projects/airsstack`: `.github/profile/README.md` (its own repository),
`README.md` and `CLAUDE.md` (the local-only org repository). The coder edits them in place and
commits nothing. They state that claudestacks is archived, so the author commits and pushes them
only after the archive (spec §9 step 11). Every edit is the exact spec §8.1 text.

**Tech Stack:** Markdown, a Mermaid block, text diagrams.

**Content authority:** spec §8.1.

---

## Files that change

```
/Users/hiraq/Projects/airsstack/.github/profile/README.md — modify: diagram node and style, projects table, airsl row, heading text, lines 103 and 111
/Users/hiraq/Projects/airsstack/README.md                  — modify: map line 44, line 55, tree lines 92-94, three table rows, lines 116 and 156
/Users/hiraq/Projects/airsstack/CLAUDE.md                  — modify: line 24, lines 26-28
```

## Order of work

### Task 1 — Org profile

**Files:**
- Modify `/Users/hiraq/Projects/airsstack/.github/profile/README.md`.

**Change:** The spec §8.1 profile edits, exactly:
- Line 25 becomes the `SDLC[...]` node text; line 46 becomes the amber style.
- The agentstacks row is inserted above the claudestacks row. The claudestacks row's text and
  status change to the spec's.
- The airsl row's last sentence changes.
- "Nothing below has moved yet." is replaced.
- Line 103's `claudestacks-sdlc` sentence start changes.
- Line 111's regrouping phrase changes.

**Proof:** checks written first and seen failing:
- `profile-edits` — each spec §8.1 replacement string appears in the file, and each replaced
  original string (`claudestacks → agentstacks<br/>`, `Nothing below has moved yet.`,
  `the agentstacks and airsdk regrouping`, ``moves through `claudestacks-sdlc` ``,
  `the claudestacks hooks`) no longer does.
- `table-shape` — every row of the Projects table has the header's column count.

**Done when:** the checks pass. No commit (spec §9 step 11 is the author's).

### Task 2 — Org README and CLAUDE.md

**Files:**
- Modify `/Users/hiraq/Projects/airsstack/README.md` — the eight spec §8.1 edits: map line 44,
  line 55, tree lines 92-94 (the exact three-line block), the agentstacks, claudestacks and airsl
  table rows, line 116, line 156.
- Modify `/Users/hiraq/Projects/airsstack/CLAUDE.md` — line 24 and lines 26-28 (the exact
  three-line block).

**Change:** The org README and the org CLAUDE.md map agree with the profile: agentstacks is
pre-release, hosts claudevs and has moved; claudestacks is archived and still hosts airsclaude
until airsdk; the airsstack and airsdk entries stay planned.

**Proof:** checks written first and seen failing:
- `map-width` — line 44 of the org README has the same character length as line 43 and line 45
  (the box stays closed), and its first cell is `│ agentstacks   tool setup: packs │`.
- `blocks-exact` — the README lines 92-94 and the CLAUDE.md lines 26-28 equal the spec §8.1 fenced
  blocks.
- `readme-edits` — each other replacement string appears, and `nothing has moved yet`,
  `claudestacks hooks ►`, ``moves through `claudestacks-sdlc` `` and
  `agentstacks** (private; from claudestacks)` no longer do.
- `table-shape` — every Projects row has the header's column count.

**Done when:** the checks pass. No commit.

---

## Risks

- These files sit outside the worktree, and a worktree-isolated session may be refused writes
  there. No git operation in that repository is part of this plan, so execute's commit step skips
  these files. Fallback if a write is refused: leave the file untouched, record a deviation, and
  hand the author the spec §8.1 edits for that file to apply at §9 step 11. The plan still counts
  as done once every other edit is in.
- The org README and profile may have changed since the spec read them (2026-11-24). An edit
  whose original text is no longer at the stated line is applied by matching the text, not the
  line number. If the text is gone, that edit is reported as a deviation.

## Proof

- Every check above passes. The three files are left uncommitted for the author.
