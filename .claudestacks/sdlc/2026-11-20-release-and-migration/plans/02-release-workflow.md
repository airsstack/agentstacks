---
status: approved
created: 2026-11-24
depends-on: [01]
built-on: a72a2ee643bb
---

# Release workflow Implementation Plan

**Goal:** Release the `agentstacks` and `claudevs` binaries through dist, each on its own tag, with every archive smoke-tested before it is published.

**Architecture:** dist 0.33.1 owns `release.yml`, which it generates from `dist-workspace.toml`.
The trigger is the one hand edit; `allow-dirty = ["ci"]` keeps it. `smoke.yml` is hand-written and
reached through dist's `global-artifacts-jobs` hook, so `host` publishes only after it succeeds.
The smoke matrix is derived from the plan manifest per archive, so one workflow serves both apps.
Depends on plan 01: the tag checks below name the release versions (agentstacks-cli 0.1.0,
claudevs-cli 0.1.2), which plan 01 sets, and every recorded dist run used `publish = true`
(spec probes `dist-results.md:4`).

**Tech Stack:** dist (`cargo-dist`) 0.33.1, GitHub Actions YAML, bash, jq.

**Content authority:** spec §4 (config, trigger, tags), §6 (smoke), §10 (rows: version mismatch,
release or smoke failure, library and packs tags, pull request, Windows), §12 (dist config and
archives rows).

---

## Files that change

```
dist-workspace.toml               — create: spec §4.1, keys and comments exactly
.github/workflows/release.yml     — create: dist-generated, then the spec §4.2 `on:` block
.github/workflows/smoke.yml       — create: per-archive smoke for both apps (spec §6)
```

## Order of work

### Task 1 — dist config and the release workflow

**Files:**
- Create `dist-workspace.toml` — exactly the spec §4.1 `toml` block.
- Create `.github/workflows/release.yml` — produced by the spec §4.2 procedure: write §4.1 without
  the `allow-dirty` line, run `dist generate`, replace the generated `on:` block with the spec §4.2
  `yaml` block (comment included), then restore the `allow-dirty` line in `dist-workspace.toml`.
  No other byte of the generated file changes.

**Change:** A push of `agentstacks-cli-v*` or `claudevs-cli-v*` releases that one app with its
shell installer and six archives. `packs-v*`, `agentstacks-v*` and `claudevs-v*` tags start
nothing. A pull request runs only dist's plan job (`pr-run-mode = "plan"`). The workflow calls
`./.github/workflows/smoke.yml`, which Task 2 creates.

**Proof:** checks run first and seen failing (no config yet):
- `config-exact` — the spec §4.1 `toml` block and §4.2 `yaml` block, extracted from the spec with
  the awk `block` function of `probes/scripts/spec-probe.sh`, compare equal (`cmp`) to
  `dist-workspace.toml` and to the `on:` block plus its four comment lines in `release.yml`.
- `plan-agentstacks-tag` — `dist plan --tag=agentstacks-cli-v0.1.0 --output-format=json` exits 0;
  its `.releases` holds only `agentstacks-cli` `0.1.0`, with `agentstacks-cli-installer.sh` and six
  `executable-zip` artifacts.
- `plan-claudevs-tag` — the same for `claudevs-cli-v0.1.2`: only `claudevs-cli` `0.1.2`.
- `plan-untagged` — `dist plan` exits 0 (what a pull request runs).
- `plan-mismatch` — `dist plan --tag=agentstacks-cli-v0.1.9` exits non-zero with
  `claims we're releasing agentstacks-cli 0.1.9`.
- `generate-keeps-trigger` — copy `release.yml` aside, run `dist generate`, then `cmp` the file
  with the copy: identical.
- `smoke-hook` — `release.yml` contains `uses: ./.github/workflows/smoke.yml` in its
  `custom-smoke` job.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`ci(repo): dist release workflow for agentstacks and claudevs`.

### Task 2 — Smoke workflow

**Files:**
- Create `.github/workflows/smoke.yml` — per spec §6.

**Change:** The workflow runs `on: workflow_call` with input `plan` (string), with
`permissions: contents: read`. Its header comment, adapted from claudestacks' smoke workflow, says
it serves two apps, is never triggered alone, and runs `aarch64-apple-darwin` on `macos-15`.

- `targets` job (`ubuntu-22.04`): sets `runners` to claudestacks' six-row JSON table (from
  `/Users/hiraq/Projects/airsstack/claudestacks/.github/workflows/claudevs-cli-smoke.yml:51-58`)
  and builds the matrix with exactly the spec §6 jq program
  (`jq -c --argjson runners "$runners" '<program>' <<<"$PLAN"`). An empty matrix fails with
  `the plan lists no archives to smoke-test`; a target with no runner fails with
  `no smoke runner for: <targets>` (claudestacks' texts). It outputs only `matrix`.
- `smoke` job: one run per row, `fail-fast: false`, bash, `timeout-minutes: 15`, env `ARCHIVE`,
  `BIN_NAME` (row `bin`) and `VERSION` (row `version`). Steps: checkout
  (`persist-credentials: false`); download `artifacts-build-local-*` into `.smoke/artifacts`;
  checksum (`sha256sum -c`, else `shasum -a 256 -c`); unpack the `.tar.xz`; find the file named
  `$BIN_NAME` (missing → `no $BIN_NAME binary inside $ARCHIVE`); on `alpine` rows write the
  `alpine:3.20` docker wrapper as claudestacks does; export the binary or wrapper path as `BIN`.
- Step `version` (always): `"$BIN" --version` must equal `$BIN_NAME $VERSION`, else
  `$BIN_NAME --version printed '<got>', expected '$BIN_NAME $VERSION'`.
- The per-app gates are step-level `if:` expressions on the matrix row: the claudevs doctor and
  wiring steps carry `if: ${{ matrix.bin == 'claudevs' }}`, the claudevs test stage carries
  `if: ${{ matrix.bin == 'claudevs' && !matrix.alpine }}`, and the agentstacks step carries
  `if: ${{ matrix.bin == 'agentstacks' }}`.
- Steps for claudevs: claudestacks' doctor, wiring-fixture and test-stage-fixture steps, copied
  with only two changes: `$CLAUDEVS` → `$BIN`, and the `if:` lines above.
- Step for agentstacks: make `.smoke/project/ext` and `.smoke/project/packs`, copy
  `extensions/claude` to `.smoke/project/ext/claude` and `packs/guideline-rust` to
  `.smoke/project/packs/guideline-rust`, then in `.smoke/project` run
  `init --tool ext/claude --pack packs/guideline-rust --yes` (exit 0, a line matching
  `^applied: [0-9]+ written, 0 deleted$`), `plan` (exit 0, a line matching
  `^0 to write, 0 to delete, [0-9]+ unchanged, 0 skipped$`) and `doctor` (exit 0, a line equal to
  `healthy`). A non-zero exit fails with `agentstacks <command> exited <code>, expected 0`; a
  missing line fails with `agentstacks <command> did not print '<expected>'`.

**Proof:** checks run first and seen failing (no workflow yet):
- `matrix-from-plan` — the `targets` script body, run locally with `PLAN` set to
  `dist plan --tag=agentstacks-cli-v0.1.0 --output-format=json` output, emits six rows with
  `bin` `agentstacks`, `version` `0.1.0`, a `runner` on every row and `alpine` true only on the two
  musl rows. The same for `claudevs-cli-v0.1.2` gives `claudevs`, `0.1.2`.
- `matrix-empty` — the same script fed a plan with no `executable-zip` artifact exits 1 with
  `the plan lists no archives to smoke-test`.
- `matrix-no-runner` — the same script fed a plan whose archive has a target outside the runner
  table exits 1 with `no smoke runner for: <that target>`.
- `agentstacks-steps-local` — with `BIN` set to the host binary from
  `cargo build --release -p agentstacks-cli`, `BIN_NAME=agentstacks` and `VERSION=0.1.0`, the
  `version` and agentstacks-project step bodies, run locally under bash with `PATH=/nonexistent`
  for the binary (as Alpine has no git or airsl), all exit 0.
- `agentstacks-steps-fail` — the agentstacks-project body with `BIN` pointing at a stub that exits
  2 fails with `agentstacks init exited 2, expected 0`. With a stub that exits 0 and prints nothing,
  it fails with `agentstacks init did not print '…'`.
- `claudevs-steps-local` — with `BIN` set to the host binary from
  `cargo build --release -p claudevs-cli`, `BIN_NAME=claudevs` and `VERSION=0.1.2`, the `version`
  step and the copied claudevs step bodies, run locally, exit 0.
- `binary-missing` — the find step body run over an empty unpack directory fails with
  `no agentstacks binary inside <archive>`.
- `release-calls-smoke` — the `custom-smoke` job of `release.yml` passes `plan`, the only input
  `smoke.yml` declares.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`ci(repo): smoke-test every release archive of both apps`.

---

## Risks

- The first full run of `release.yml` and `smoke.yml` happens on a real tag, because pull requests
  build nothing (spec §6). The local step runs above are the closest check; the spec's runbook
  (§9 step 4) makes a failed tag recoverable before anything reaches crates.io.
- A future `dist generate` with `allow-dirty` removed overwrites the trigger. The comment above
  `on:` says how to reapply it.
- The Alpine wrapper path, the archive download, the checksum and the unpack of a real `.tar.xz`
  are exercised only in CI. The local runs use host binaries from `cargo build --release`; the
  `PATH=/nonexistent` run stands in for "no tools", not for musl itself.
- Task 1's commit has a `release.yml` that calls `smoke.yml`, which only Task 2 creates. Both
  commits ship in one pull request, so GitHub never sees the first alone.

## Proof

- `dist plan` exits 0 untagged and for both binary tags; `cmp` shows both spec blocks unchanged.
- `cargo make dod` green.
