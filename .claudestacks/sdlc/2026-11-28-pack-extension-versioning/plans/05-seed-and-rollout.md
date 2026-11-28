---
status: executing
created: 2026-11-28
depends-on: [03, 04]
built-on: a7f1874ed5fe
---

# Seed Release and Rollout Implementation Plan
**Goal:** Ship the first stream tags and the dual-scheme binary without breaking existing pins.
**Architecture:** One release commit, eight solo tag pushes, then the minor binary release; every outward step carries an explicit approval checkpoint.
**Tech Stack:** Git tags, GitHub releases, crates.io, the release machinery from plan 03.

---

## Files that change

```
packs/sdlc/pack.toml                          — modify version 1.0.0 to 1.1.0 for the seed (on main, post-merge)
packs/sdlc/CHANGELOG.md                       — date the Unreleased seed entry as 1.1.0 (on main, post-merge)
crates/agentstacks/Cargo.toml                 — modify version 1.2.0 to 1.3.0
crates/agentstacks-cli/Cargo.toml             — modify version 1.2.0 to 1.3.0
Cargo.lock                                    — modify updated crate versions
crates/agentstacks/CHANGELOG.md               — modify 1.3.0 entry
crates/agentstacks-cli/CHANGELOG.md           — modify 1.3.0 entry
```

## Order of work

### Task 1 — Cut the seed release commit

**Files:** On `main` after the merge — Modify `packs/sdlc/pack.toml` — version to 1.1.0; date the `CHANGELOG.md` Unreleased seed entry as `1.1.0`.

**Change:** On `main`, after the merge and before any stream tag exists: one commit bumps the sdlc manifest to 1.1.0 and dates its staged changelog entry. The bump cannot ride a PR diff — its shipped content predates the gate (shipped on `main` by #27), so a PR-range check would misread it as a vanity bump. The release workflow's no-base path validates it instead: manifest declares the version and the changelog names it. This commit is the seed the eight tags point at.

**Proof:** written first, seen failing, then passing — `seed_versions_match_stipulation` — the agreement check passes for all eight artifacts against their stipulated versions on the release commit; `no_base_needs_no_gate` — the release workflow skips the diff gate with no prior tag and validates manifest-equals-version plus changelog entry instead.

**Done when:** the stipulation checks pass and the Definition of Done is green. Approval checkpoint: author confirms the commit before any tag. Commit `chore(packs): seed per-artifact streams`.

### Task 2 — Mint and push the eight stream tags

**Files:** No files change; eight annotated tags are created and pushed one at a time via the plan-03 workflow.

**Change:** `packs-core-v1.0.1`, `packs-sdlc-v1.1.0`, `packs-journal-v1.0.0`, `packs-guideline-rust-v1.0.0`, `packs-guideline-python-v1.0.0`, `ext-claude-v1.1.0`, `ext-musecode-v1.1.0`, `ext-opencode-v1.1.0`; no `packs-v*` tag minted or moved.

**Proof:** written first, seen failing, then passing — `all_eight_tags_resolve` — `git ls-remote` lists all eight; `smoke_init_update_doctor` — fresh `init` resolves a stream tag, `update` discovers it, `doctor` is healthy.

**Done when:** all eight tags verify on the remote. Approval checkpoint: author confirms each push batch (one at a time per the runbook rule). No commit (tags only).

### Task 3 — Release the 1.3.0 binary

**Files:** Modify both crate manifests and changelogs plus `Cargo.lock` — 1.2.0 to 1.3.0.

**Change:** Release PR, green gates, `agentstacks-cli-v1.3.0` tag through the existing release workflow, crates.io publishes with the human keeping the publish step.

**Proof:** written first, seen failing, then passing — `release_workflow_green` — the tag's workflow run succeeds; `published_versions_match` — crates.io shows 1.3.0 for both crates; `runbook_corpora_green` — `claudevs-check`, `opencode-check`, `musecode-check`, the installers, and `book-build` all pass before the publish approval is sought.

**Done when:** the release workflow is green, the corpora are green, crates are published, and the Definition of Done is green. Approval checkpoint: author approves the publish. Commit `chore(repo): release agentstacks 1.3.0`.

---

## Risks

- Tags are user-visible the moment they land and crates.io publishes cannot be undone; the riskiest steps are the pushes and the publish, each gated by an explicit author checkpoint in this plan.
- The window between seed tags and the 1.3.0 binary leaves new-binary users unable to discover streams; the plan orders Task 2 before Task 3 and keeps the window to a single session.
- Pushing more than three tags at once loses GitHub events; pushes go strictly one at a time.

## Proof

- All eight tags listed on the remote.
- Release workflow green and crates published at 1.3.0.

---

## Review findings

One agent round on release readiness before any outward step: no findings. The
seed content verifies clean (manifests equal the stipulated versions, every
changelog carries its entry, no stream tags exist yet so the workflow takes
its no-base path).

## Deviations

- 2026-11-28: board renders `Legacy chain detected under .claudestacks/sdlc`; pre-existing, carried on.
- 2026-11-28: reviewer handoff ran through native subagent delivery, not `context-handoff` files.
- 2026-11-28: no commit, no tag, no push, no publish performed in this run. The execute
  instruction did not authorize them, and Tasks 2-3 each require the author's explicit word at
  their checkpoints. The seed content is assembled and verified in the worktree; the release
  itself is staged, not started.
- 2026-11-28: plan stays `executing` — Tasks 1 (verification) is complete, Tasks 2-3 await the
  author checkpoints above.
