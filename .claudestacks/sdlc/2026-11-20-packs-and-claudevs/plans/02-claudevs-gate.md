---
status: done
created: 2026-11-23
depends-on: [01]
built-on: 0f89fdb3ebb3
---

# claudevs gate Implementation Plan

**Goal:** Run the claudevs fixture-corpus check from this repo's cargo-make and CI.

**Architecture:**
- Three cargo-make tasks copied from `../claudestacks/Makefile.toml`. None joins `dod` (spec §2
  "Gate tasks").
- One new CI job runs `cargo make claudevs-check`. The runner has no `claude` binary, so the
  validate stage reports skipped (spec §2 "CI").

**Tech Stack:** cargo-make, GitHub Actions, POSIX sh.

**Content authority:** spec §1 (bad-matcher row), §2 "Gate tasks" and "CI", §11 (claudevs row).

---

## Files that change

```
Makefile.toml               — modify: tasks claudevs-check, corpus-fetch, corpus-check
.github/workflows/ci.yml    — modify: job claudevs-check
```

## Order of work

### Task 1 — Add the three claudevs tasks

**Files:**
- Modify `Makefile.toml`: a new section `# claudevs` after the Gate section.

**Change:**
- `[tasks.claudevs-check]` is `../claudestacks/Makefile.toml` lines 232–391, the script verbatim.
  The `category` becomes `"claudevs"`.
- `[tasks.corpus-fetch]` is lines 393–417 (needs the network).
- `[tasks.corpus-check]` is lines 419–458 (the `#[ignore]` corpus lane).
- The script's text is unchanged. These comments change:
  - the comment that says `bad-matcher-plugin` fails Claude's validate step now says it passes
    validate (`PreToolUseX` is a warning with `claude` 2.1.330) and fails `wiring` (spec §1;
    `probes/claudevs-results.md` step g);
  - the same claim inside the script (before `expect_no_fail minimal-plugin`) now says validate
    passes `bad-matcher-plugin` and still fails `hooks-array-plugin` (step g);
  - the `corpus-fetch` comment that names `crates/claudevs/tests/corpus_fetch.rs` (`:411-412`)
    names `crates/claudevs/tests/fetch.rs` (plan 01);
  - a comment naming a claudestacks-only task (`plugins`) names this repo's `dod` instead.
  - Only comments change; every command line of the script is verbatim.
- `tasks.dod.dependencies` is not changed.

**Proof:**
- `cargo make claudevs-check` exits 0 and prints the 8 fixture outcomes it printed in claudestacks
  (spec §1: "the fixture check passes 8/8").
- `cargo make --list-all-steps` lists `claudevs-check`, `corpus-fetch` and `corpus-check`.
  `dod`'s dependencies are still `fmt-check, clippy, doc, test, test-doc, lua`.

**Done when:** `cargo make claudevs-check` exits 0 and `cargo make dod` is green. Commit
`build(repo): claudevs-check, corpus-fetch and corpus-check tasks`.

### Task 2 — Add the CI job

**Files:**
- Modify `.github/workflows/ci.yml`: a job `claudevs-check` after `dod`.

**Change:**
- `name: claudevs fixture corpus`, `runs-on: ubuntu-latest`, `timeout-minutes: 30`.
- Steps:
  1. `actions/checkout@v5`;
  2. the pinned toolchain, with the same `run` block as the `dod` job (`rustup toolchain install`,
     then the version lines);
  3. `Swatinem/rust-cache@v2`, as in `dod`;
  4. cargo-make through `taiki-e/install-action@v2`;
  5. `cargo make claudevs-check`, with the comment "No `claude` binary on the runner: the validate
     stage reports itself skipped and the deterministic stages gate."
- Shape: `../claudestacks/.github/workflows/ci.yml:64-91`.

**Proof:**
- The job's last step runs exactly `cargo make claudevs-check`, the command Task 1 proved locally.
- After push, GitHub accepts the workflow, and the job `claudevs fixture corpus` is green on the
  pull request with its validate stage reported skipped.

**Done when:** `cargo make dod` is green locally and the pushed job passes. Commit
`ci(repo): claudevs-check job`.

---

## Risks

- `claudevs-check` reads `crates/claudevs/tests/fixtures/` by path. If the import moved those paths,
  the script fails. Plan 01 keeps them.
- A local machine with `claude` installed runs the validate stage that CI skips. The corrected
  comment states what it reports there.

## Proof

- `cargo make claudevs-check` and `cargo make dod` green locally; CI `dod`, `deny` and
  `claudevs-check` green on the pull request.

## Review findings

- 🔵 open — `Makefile.toml:270` carried comment "The only step in this repository that touches the network" is false here (`deny` fetches advisories); the plan allows only the briefed comment edits.
- 🔵 open — category `claudevs` is lowercase and sorts after `Tools`; other categories are capitalized.
- 🔵 accepted — unbriefed comment at `ci.yml:90` ("Same step as in the `dod` job …"); accurate.
- Verification: `cargo make claudevs-check` exit 0, 8/8 (`ok minimal-plugin`, `dead-script-plugin`, `escape-plugin (FAIL wiring)`, `bad-matcher-plugin (FAIL wiring)`, `hooks-array-plugin (FAIL wiring)`, `project-branch-plugin`, `exec-args-plugin`, `matcher-routing-plugin (FAIL test)`); with `claude`/`airsl` off PATH it still passes 8/8 and prints `skip  validate`. `cargo make --list-all-steps` lists the three tasks; `dod` deps unchanged. `ci.yml` parses (`ruby -ryaml`), jobs `["dod","claudevs-check","deny"]`. Workspace `cargo make dod` exit 0.

## Deviations

- 2026-11-23 — `category = "claudevs"` set on all three tasks, not only `claudevs-check`: the source category "Plugin suite" names a suite this repo does not have.
- 2026-11-23 — The validate stage ran locally (a `claude` binary is installed): `bad-matcher-plugin` gave `ok validate`, `FAIL wiring`. The Task 2 proof (CI job green with validate skipped) can only be checked after a push.
- 2026-11-23 — Tasks 1 and 2 ran in one coder; no per-task commits (execute does not commit unless told).
