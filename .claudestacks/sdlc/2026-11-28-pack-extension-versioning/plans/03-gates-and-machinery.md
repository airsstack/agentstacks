---
status: done
created: 2026-11-28
depends-on: [01]
built-on: a7f1874ed5fe
---

# Gates and Release Machinery Implementation Plan
**Goal:** Enforce manifest discipline in CI and mechanize artifact releases.
**Architecture:** Shell effects in the wrapper and Makefile gate, decisions in Lua under airsl test, server-side minting in one dispatch workflow; official artifacts only.
**Tech Stack:** Shell, Lua under airsl, GitHub Actions, Make.

---

## Files that change

```
Makefile.toml                                 — modify gate task for manifest discipline
.github/workflows/ci.yml                     — modify release-readiness job
.github/workflows/release-artifact.yml        — create dispatch workflow
scripts/release-artifact.sh                   — create local wrapper
scripts/release/suggest.lua                   — create bump suggestion logic
scripts/release/suggest_test.lua              — create airsl tests for the logic
scripts/release/agreement.lua                 — create manifest/tag agreement check
scripts/release/agreement_test.lua            — create airsl tests for the check
```

## Order of work

### Task 1 — Gate the manifest discipline in CI

**Files:** Modify `Makefile.toml` — gate task comparing shipped scope against manifest bumps and changelog entries; Modify `.github/workflows/ci.yml` — release-readiness job running the gate on every PR.

**Change:** The gate owns both modes. PR mode diffs the pull request against its merge-base with the target branch; release mode diffs the release commit against the stream's last tag, or — with no prior tag — asserts manifest equals the minted tag plus a changelog entry instead of diffing. Both modes compute the installed set per artifact through the existing loaders' exclude rules, evaluate multi-artifact diffs per artifact independently, and fail changed-without-bump, bumped-without-change, and bump-without-entry.

**Proof:** written first, seen failing, then passing — `gate_rejects_82d7561_shape` — a two-artifact SKILL plus adapter plus hook fixture with unchanged manifests and no entry fails naming both artifacts; `gate_accepts_paperwork_only` — a changelog-only fixture passes; `gate_accepts_excluded_only` — a README plus `*_test.lua` fixture passes with no bump; `gate_rejects_exclude_edit_without_bump` — an exclude-only fixture fails; skill, adapter, and hook fixtures each fail without a bump; a vanity bump fails; a bump without an entry fails; `release_mode_without_base_asserts_equality` — manifest-equals-tag plus entry is asserted by the agreement check the release workflow runs, covered by its unit tests.

**Done when:** the gate task passes on `main` and fails on the fixtures, and the Definition of Done is green. Commit `ci(repo): manifest-discipline release-readiness gate`.

---

## Review findings

One agent round against the plan. All fixed in this plan before close:

| # | Tier | Disposition |
|---|------|-------------|
| 1 | 🔴 | Applied — smoke now asserts init pin, `update` printing `nothing to update`, a successful apply, and `doctor` printing `healthy`. |
| 2 | 🔴 | Applied — every `gh` call is guarded with `|| fail`; a gh-stub self-test proves a failed dispatch stops before `run watch`. |
| 3 | 🔴 | Applied — the wrapper header documents the manual `git tag -a` fallback with the same checks. |
| 4 | 🟡 | Applied — paperwork-only and excluded-only fixtures are separate passing cases; the vanity bump fails between them. |
| 5 | 🟡 | Applied — the 82d7561 fixture spans two artifacts with SKILL, adapter, and hook changes and must name both. |
| 6 | 🟡 | Applied — a hook-only fixture joins the SKILL and adapter fixtures. |
| 7 | 🟡 | Applied — no-base equality lives in the tested agreement check the workflow calls; the gate's diff mode needs no base. |
| 8 | 🟡 | Applied — a gh-stub self-test proves a mismatched version never reaches `gh`, and the failing dispatch test guards the call. |
| 9 | 🟡 | Applied — a version-matching, entry-missing fixture proves the entry refusal path at wrapper level. |
| 10 | 🟡 | Applied — the wrapper prints the suggestion on every release and the workflow posts it to the run summary. |
| 11 | 🟡 | Applied — the release-gate task and the CI job both run `sh -n` over both scripts. |
| 12 | 🟡 | Applied — dry-run prints that the remote tag check is skipped, real runs warn on listing failure and the workflow re-verifies before pushing. |

`cargo make release-gate` green (10/10 gate fixtures, 8/8 wrapper cases), `cargo make lua` green (7 new Lua tests), `cargo make dod` green, workflow YAML parses.

## Deviations

- 2026-11-28: board renders `Legacy chain detected under .claudestacks/sdlc`; pre-existing, carried on.
- 2026-11-28: reviewer handoff ran through native subagent delivery, not `context-handoff` files.
- 2026-11-28: the gate's scope matcher treats unknown glob shapes as shipped (fail-closed) rather than refusing unknown shapes; documented in the script header.
- 2026-11-28: plan Task 1's `release_mode_without_base_asserts_equality` is covered by agreement.lua unit tests plus the workflow's no-base skip, not by a gate self-test case — the gate is diff-only by design and the no-base path has no diff to check.
- 2026-11-28: no `set -e` in the wrapper; fallible calls carry explicit `|| fail` guards instead, so preflight checks keep their tailored messages.

### Task 2 — Mint tags from a dispatch workflow

**Files:** Create `.github/workflows/release-artifact.yml` — `workflow_dispatch` with `artifact` and `version` inputs.

**Change:** The workflow re-verifies a clean tree with manifest equal to version and entry present, mints one annotated tag, pushes it alone, and smoke-verifies with fresh `init`, `update`, and healthy `doctor` in a throwaway home; `contents: write` for the push.

**Proof:** written first, seen failing, then passing — `dispatch_rejects_version_mismatch` — dry-run with manifest behind version stops before any tag; `smoke_asserts_all_three` — the smoke step asserts init pins the new tag, update reports nothing to update, apply succeeds, and doctor reports healthy.

**Done when:** `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-artifact.yml'))"` exits 0, the workflow dispatches cleanly on a test input, and the Definition of Done is green. Commit `ci(repo): per-artifact release dispatch workflow`.

### Task 3 — Wrap dispatch in a local script

**Files:** Create `scripts/release-artifact.sh` — preflight plus trigger plus watch.

**Change:** Preflight refuses on dirty tree, version mismatch, missing entry, existing tag, missing airsl, or dispatch failure; then `gh workflow run` plus `gh run watch`; documents direct `git tag -a` fallback. The missing-airsl refusal reads `release-artifact: airsl not found on PATH (install it to run the release checks)` and every refusal names itself before anything is pushed.

**Proof:** written first, seen failing, then passing — `refuses_dirty_tree_before_any_push` — dirty fixture exits non-zero naming the refusal with no tag created; `refuses_existing_tag` — taken version exits non-zero; `refuses_version_mismatch` — manifest behind requested version exits non-zero; `refuses_missing_entry` — absent changelog entry exits non-zero; `refuses_missing_airsl` — airsl off `PATH` prints the prescribed pointer and exits non-zero; `refuses_failed_dispatch` — a failed run exits non-zero; `sh_n_parses` — `sh -n` clean following the installer precedent.

**Done when:** the script's refusal suite passes and the Definition of Done is green. Commit `feat(repo): local release-artifact wrapper`.

### Task 4 — Test the Lua decision logic

**Files:** Create `scripts/release/*.lua` — bump suggestion and manifest/tag agreement as pure functions; Create `scripts/release/*_test.lua` — airsl suites pinning their behavior.

**Change:** Suggestion maps Conventional Commits since the stream's last tag to a bump with an explicit none-case; the agreement check compares manifest version to the requested tag.

**Proof:** written first, seen failing, then passing — `feat_suggests_minor_fix_suggests_patch` — one `feat` and one `fix` fixture yield minor and patch; `empty_history_suggests_nothing` — no commits yields no suggestion; `agreement_rejects_mismatch` — manifest behind tag fails. All run under the existing `airsl test` gate.

**Done when:** `cargo make lua` passes and the Definition of Done is green. Commit `feat(repo): Lua release decision logic`.

---

## Risks

- The gate's installed-set computation must match the loaders exactly; reimplementation drift is the riskiest step, mitigated by routing scope through loader behavior in Task 1's fixtures.
- Dispatch workflows cannot be fully exercised without pushing; dry-run refusal paths carry the verification, and the first real seed tag is the live proof.

## Proof

- `cargo make lua` green including the new suites.
- Gate green on `main`, red on every negative fixture.
- Wrapper refusal suite green.
