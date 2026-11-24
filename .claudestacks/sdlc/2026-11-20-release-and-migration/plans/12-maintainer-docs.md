---
status: done
created: 2026-11-24
depends-on: [08, 10]
built-on: d3463785d0cb
---

# Maintainer docs Implementation Plan

**Goal:** Give the maintainer the release runbook in the repository.

**Architecture:** `RELEASING.md` (new, at the repository root) carries the author's release steps
of spec §9, including the claudestacks steps (§8.2, §8.3), which are the author's and not plan
work. `CLAUDE.md` points to it, records the release setup, and names the book wherever it names
`docs/` today. Steps the spec gives as commands keep the spec's text. Depends on plan 10: the
`CLAUDE.md` links name `book/src/` files plan 10 creates. Depends on plan 08: `CLAUDE.md` states
what `dist-workspace.toml` and `smoke.yml` set, and plan 08 creates them. Supersedes plan 05.

**Tech Stack:** Markdown.

**Content authority:** spec §4.3 (tag table), §7.4, §7.5, §8.2, §8.3, §9, §10 (rows: more than 100
releases, release or smoke failure, partial publish, more than 3 tags, relative link in a pack
README), P44.

---

## Files that change

```
RELEASING.md   — create: the release runbook (spec §7.4, §9, §4.3, §8.2, §8.3)
CLAUDE.md      — modify: status heading, layout, dist line, CI sentence, archive sentence, docs/ → book, book commands, the chapter rule (spec §7.5)
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
  text, the book pointer blockquote and where it goes (under the `ch05-00` title and above
  `introduction.md` line 53), the forwarder's four cases (unset or `latest`; an explicit version, with its forward fallback; a
  bad version; a failed forwarded download) with their exact messages, the
  `sh -n` and `mdbook build book` checks, and the rule that it merges and deploys before the
  archive.
- Step 10 holds spec §8.3: never delete; airsclaude is frozen until airsdk; unarchive, release,
  re-archive for a fix.
- Two notes: `cargo publish` is irreversible (a version can be yanked, never replaced), and the
  installers' release lookup reads one page of 100 releases, to be revisited when the repository
  passes 100 releases (spec §10).
- One sentence saying `RELEASING.md` stays at the root, outside the book (spec §7.4).
- Under step 5's block (kept verbatim), one line giving the text its `# prints M9` comment means:
  the spec §5.5 M9 text, exactly.

**Proof:** checks written first and seen failing:
- `steps-complete` — `RELEASING.md` has twelve numbered steps whose order matches spec §9.
- `commands-verbatim` — every fenced or inline command of spec §9 (the step 5 block, step 6's four
  `cargo publish -p` lines, the step 9 one-liner, the `git tag -d` and
  `git push origin :refs/tags/<tag>` recovery) appears in `RELEASING.md` unchanged.
- `forwarder-texts` — the three spec §8.2 messages appear exactly.
- `banner-text` — the spec §8.2 README banner (with the book migration URL) and the book pointer
  blockquote appear exactly.
- `book-step-2` — step 2 names the four URLs of spec §9 step 2 (`…/agentstacks/`,
  `…/ch03-00-migration.html`, `…/install.sh`, `…/claudevs.sh`).
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
  `cargo make claudevs-check` and `cargo deny check`, and the `book` job (spec §7.5). The layout
  block's `.github/workflows/ci.yml` entry names the same jobs, so the two places agree.
- The "Archiving claudestacks …" sentence in the porting section becomes the spec's replacement
  sentence.
- The book replaces `docs/` everywhere (spec §7.5 bullet and P44): the layout line 18, the
  "`book/` has not landed." sentence (removed), line 73's contract link, lines 95-96 and the
  "keep it in step" sentence, the planned layout's line 103, line 108 (removed) and the `book/`
  line (`(built)`), each exactly as spec §7.5 gives.
- Commands gain the two spec §7.5 lines for `cargo make book` and `cargo make book-build`, and the
  CI sentence names the `book` job as well as `installers`.
- The spec §7.5 paragraph on chapters and pack README links goes after the Architecture bullets,
  exactly.

**Proof:** checks written first and seen failing:
- `status-heading` — the first `## Status:` line equals the spec text.
- `dist-sentence` — `grep -n 'When dist is added' CLAUDE.md` prints nothing; the replacement
  sentence is present.
- `archive-sentence` — the replacement sentence is present, and
  `grep -n 'release-and-migration. work' CLAUDE.md` prints nothing.
- `layout-block` — the layout block lists `dist-workspace.toml`, `install/` and `RELEASING.md`.
- `ci-named-twice` — the Commands CI sentence and the layout entry for `ci.yml` each name the
  `installers` job and the `book` job.
- `no-docs-paths` — `grep -n 'docs/' CLAUDE.md` prints no line naming this repository's `docs/`.
- `book-links-exist` — every `book/src/<file>.md` that `CLAUDE.md` names exists.
- `chapter-rule` — the spec §7.5 paragraph appears exactly.
- `book-commands` — both spec §7.5 `cargo make book…` lines appear exactly.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`docs(repo): CLAUDE.md for the release setup and the book`.

---

## Risks

- `RELEASING.md` copies spec §9. If the spec changes after approval, the runbook drifts;
  `commands-verbatim` catches it only at build time.
- The runbook's steps run against live services (GitHub, crates.io). Nothing in this plan runs
  them.

## Proof

- Every check above passes; `cargo make dod` green.

## Review findings

- 🔵 `CLAUDE.md:23` the layout block names only `ci.yml` of the workflows — open.
- 🔵 `CLAUDE.md:75` line past the ~100-column wrap — fixed: rewrapped; `check.py all` → 15 PASS,
  0 FAIL.
- 🔵 `RELEASING.md:22` "tags the merged release commit with all five" vs step 4's recovery — open
  (spec §4.3 text).
- 🔵 `RELEASING.md:48,59` "M9" is a spec identifier in a shipped doc — open (spec text, explained
  at :59).
- 🔵 `RELEASING.md:80` steps 11–12 name the org root and the sdlc milestone command — open (spec
  text).
- 🔵 `RELEASING.md:90-94` only the banner's first line carries `>` — declined: it sits in a `text`
  block, shown as the literal text to paste.
- 🔵 `RELEASING.md:97` "above line 53" cites a line number in claudestacks — open (spec text; the
  quoted line also anchors it).

## Deviations

- 2026-11-24 — `RELEASING.md` drops the spec's "(this chain's work)", the `P<n>` citations and the
  "(§8.1)" pointer, and inlines or links §8.2/§8.3 as its own sections: planning vocabulary does not
  ship in the runbook. Step lead words and every command are unchanged (`steps-complete` passes).
- 2026-11-24 — The packs-update reference links `book/src/ch05-00-plan-and-apply.md` instead of the
  spec's `docs/plan-and-apply.md:336` (moved by plan 10).
- 2026-11-24 — The layout entries for `dist-workspace.toml` and `install/` carry short descriptions
  of my choosing; the spec only says the block gains them.
- 2026-11-24 — Ran in parallel with plan 11 (disjoint files). One coder for both tasks; no
  task-briefer. Commits per task with this plan's messages, by the orchestrator.
- 2026-11-24 — `cargo make dod` run once by the orchestrator after the wave: exit 0.
