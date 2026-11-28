---
status: approved
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

**Proof:** written first, seen failing, then passing — `gate_rejects_82d7561_shape` — a fixture diff with 26 shipped files, unchanged manifest, no entry fails both directions; `gate_accepts_paperwork_only` — a manifest-plus-changelog-only fixture passes; `gate_accepts_excluded_only` — a README plus `*_test.lua` fixture passes with no bump; `gate_rejects_exclude_edit_without_bump` — a fixture editing only `exclude` fails for missing bump; `gate_rejects_skill_change_without_bump` — a `SKILL.md`, adapter, and hook fixture each fail without a bump; `release_mode_without_base_asserts_equality` — a no-prior-tag fixture passes on manifest-equals-tag plus entry and fails without the entry.

**Done when:** the gate task passes on `main` and fails on the fixtures, and the Definition of Done is green. Commit `ci(repo): manifest-discipline release-readiness gate`.

### Task 2 — Mint tags from a dispatch workflow

**Files:** Create `.github/workflows/release-artifact.yml` — `workflow_dispatch` with `artifact` and `version` inputs.

**Change:** The workflow re-verifies a clean tree with manifest equal to version and entry present, mints one annotated tag, pushes it alone, and smoke-verifies with fresh `init`, `update`, and healthy `doctor` in a throwaway home; `contents: write` for the push.

**Proof:** written first, seen failing, then passing — `dispatch_rejects_version_mismatch` — dry-run with manifest behind version stops before any tag; `smoke_asserts_all_three` — the smoke step asserts init resolves, update discovers, doctor is healthy.

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
