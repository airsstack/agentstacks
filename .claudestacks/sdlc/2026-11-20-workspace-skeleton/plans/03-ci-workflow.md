---
status: approved
created: 2026-11-20
depends-on: [01]
built-on: 5c6b28af8487
---

# CI Workflow Implementation Plan

**Goal:** GitHub Actions gates every push to `main` and every pull request with the same commands a
developer runs locally.

**Architecture:** One workflow, `.github/workflows/ci.yml`, copied from claudestacks' CI keeping
only what agentstacks uses: the header (triggers, concurrency, permissions, env), the `dod` job
and the `deny` job. The `dod` job runs `cargo make dod`; the `deny` job runs `cargo deny check`
independently, outside the gate, as in claudestacks. The pinned toolchain comes from
`rust-toolchain.toml` via `rustup show`.

**Tech Stack:** GitHub Actions — `actions/checkout@v5`, `Swatinem/rust-cache@v2`,
`taiki-e/install-action@v2` (as used at `../claudestacks/.github/workflows/ci.yml`).

**Content authority:** `intent.md` (spec skipped) — Desired outcome bullet 1 (CI on push to `main`
and on pull requests), Affected systems (CI workflow), Problem ("same … CI pattern").

---

## Files that change

```
.github/workflows/ci.yml   — create  CI: dod job + deny job
```

## Order of work

### Task 1 — Add the CI workflow

**Files:**
- Create `.github/workflows/ci.yml` — copied from `../claudestacks/.github/workflows/ci.yml`:
  - Lines 1-24 as-is: `name: CI`; `on:` push to `main`, `pull_request`, `workflow_dispatch`;
    `concurrency` with `cancel-in-progress`; `permissions: contents: read`; the `env` block.
  - The `dod` job (lines 26-62): checkout, "Install the pinned Rust toolchain" (`rustup show` and
    version prints), rust-cache, install cargo-make, `cargo make dod`. The rust-cache step drops
    `shared-key: workspace` and its comment, which exist only to share the cache with
    claudestacks' `claudevs-check` job; only one job here builds.
  - The `deny` job (lines 117-142) as-is: pinned toolchain via `rustup show`, install cargo-deny,
    `cargo deny check`.
  - Not copied: `claudevs-check` (lines 64-91), `install-pointer` (lines 93-115), `book`
    (lines 144-162).

**Change:** Pushes to `main` and pull requests run the Definition of Done and the supply-chain
check on `ubuntu-latest`.

**Proof:** No Rust tests (workflow configuration only; no workflow linter is installed locally).
Checks:
- `grep -nE '^  [a-z][a-z_-]*:$' .github/workflows/ci.yml` lists exactly the three triggers
  (`push`, `pull_request`, `workflow_dispatch`) and the two jobs (`dod`, `deny`).
- `grep -nE 'claudevs|install-check|mdbook|shared-key' .github/workflows/ci.yml` prints nothing.
- The workflow's first run on this branch's pull request is green on both jobs.

**Done when:** the two `grep` checks hold and `cargo make dod` (the stack's Definition of Done) is
green locally. Commit `ci(repo): add definition-of-done and supply-chain workflow`.

---

## Risks

- **Riskiest step: the first real CI run.** The workflow cannot be executed locally, so a YAML or
  action-version mistake shows only on the pull request. The fix lands on the same branch before
  merge.
- The `deny` job answers to a moving advisory database, so it can turn red on a commit that
  changed nothing; that is by design and why it is not part of `dod`.
- Options not taken: a Lua CI workflow and `pages.yml` (intent Non-goals); dist/release workflows
  (0.x release chain).

## Proof

- The pull request for this chain shows the `Definition of Done` and `Supply chain` checks green.
