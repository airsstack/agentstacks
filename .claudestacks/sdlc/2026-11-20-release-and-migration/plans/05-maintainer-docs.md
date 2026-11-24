---
status: approved
created: 2026-11-24
built-on: a72a2ee643bb
---

# Maintainer docs Implementation Plan

**Goal:** Give the maintainer the release runbook in the repository.

**Architecture:** `RELEASING.md` (new, at the repository root) carries the author's release steps
of spec §9, including the claudestacks steps (§8.2, §8.3), which are the author's and not plan
work. `CLAUDE.md` points to it and records the release setup. Steps the spec gives as commands
keep the spec's text.

**Tech Stack:** Markdown.

**Content authority:** spec §4.3 (tag table), §7.4, §7.5, §8.2, §8.3, §9, §10 (rows: more than 100
releases, release or smoke failure, partial publish, more than 3 tags).

---

## Files that change

```
RELEASING.md   — create: the release runbook (spec §7.4, §9, §4.3, §8.2, §8.3)
CLAUDE.md      — modify: status heading, layout, dist line, CI sentence, archive sentence (spec §7.5)
```

## Order of work

### Task 1 — RELEASING.md

**Files:**
- Create `RELEASING.md`.

**Change:** The runbook, in this order:
- A short intro: binaries release before crates.io, and every step before the crates.io publish
  can be undone (spec §9 lead).
- The spec §4.3 tag table, plus its rules: annotated tags, each pushed alone (more than three in
  one push create no event), and later independent releases.
- Spec §9 steps 1-12 as a numbered list, with each step's commands verbatim from the spec. That
  includes the step 5 subshell block and the step 9 forwarder check, and the recovery text of
  step 4 (delete the GitHub Release and the tag, fix in a pull request, re-tag the same version)
  and of step 6 (publish one crate at a time, confirm on timeout, continue from the failed crate,
  bump on a code fix).
- Step 8 links to a "claudestacks changes" section holding spec §8.2 in full: the README banner
  text, the forwarder's three behaviours with their exact messages, the `sh -n` check, and the
  rule that it merges and deploys before the archive.
- Step 10 holds spec §8.3: never delete; airsclaude is frozen until airsdk; unarchive, release,
  re-archive for a fix.
- Two notes: `cargo publish` is irreversible (a version can be yanked, never replaced), and the
  installers' release lookup reads one page of 100 releases, to be revisited when the repository
  passes 100 releases (spec §10).

**Proof:** checks written first and seen failing:
- `steps-complete` — `RELEASING.md` has twelve numbered steps whose order matches spec §9.
- `commands-verbatim` — every fenced or inline command of spec §9 (the step 5 block, step 6's four
  `cargo publish -p` lines, the step 9 one-liner, the `git tag -d` and
  `git push origin :refs/tags/<tag>` recovery) appears in `RELEASING.md` unchanged.
- `forwarder-texts` — the three spec §8.2 messages appear exactly.
- `banner-text` — the spec §8.2 README banner appears exactly.
- `tag-table` — the five tag rows of spec §4.3 appear with their "starts a release" values.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(repo): release runbook`.

### Task 2 — CLAUDE.md

**Files:**
- Modify `CLAUDE.md`.

**Change:** The spec §7.5 edits, exactly:
- The status heading becomes
  `## Status: core, CLI commands, claude extension, packs and claudevs built; first release configured`.
- The layout block under the status gains `dist-workspace.toml`, `install/` and `RELEASING.md`.
- In Commands, the "When dist is added, set …" sentence becomes the spec's replacement sentence.
- The CI sentence under Commands names the `installers` job next to `cargo make dod`,
  `cargo make claudevs-check` and `cargo deny check`. The layout block's `.github/workflows/ci.yml`
  entry names it the same way, so the two places agree.
- The "Archiving claudestacks …" sentence in the porting section becomes the spec's replacement
  sentence.

**Proof:** checks written first and seen failing:
- `status-heading` — the first `## Status:` line equals the spec text.
- `dist-sentence` — `grep -n 'When dist is added' CLAUDE.md` prints nothing; the replacement
  sentence is present.
- `archive-sentence` — the replacement sentence is present, and
  `grep -n 'release-and-migration. work' CLAUDE.md` prints nothing.
- `layout-block` — the layout block lists `dist-workspace.toml`, `install/` and `RELEASING.md`.
- `ci-named-twice` — `grep -c 'installers' CLAUDE.md` is at least 2: the Commands sentence and the
  layout entry for `ci.yml`.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(repo): CLAUDE.md for the release setup`.

---

## Risks

- `RELEASING.md` copies spec §9. If the spec changes after approval, the runbook drifts;
  `commands-verbatim` catches it only at build time.
- The runbook's steps run against live services (GitHub, crates.io). Nothing in this plan runs
  them.

## Proof

- Every check above passes; `cargo make dod` green.
