---
status: done
created: 2026-11-24
built-on: d3463785d0cb
---

# Installers Implementation Plan

**Goal:** Let a user without Rust install `agentstacks` (with airsl) or `claudevs` with one command.

**Architecture:** Two POSIX sh pointer scripts under `install/`, in the shape of airsl's pointer
(`/Users/hiraq/Projects/airsstack/airsl/install/install.sh`). Each resolves a version from the
public releases API and runs dist's own installer for that release; `install.sh` then makes sure
airsl is present. A CI job checks that they parse and that their first refusals come out exactly.
sh, not Lua, because they run before airsl exists (spec §5.2). Serving them is plan 10's
`pages.yml`, which copies both into the built book. Supersedes plan 03 without its Pages workflow.

**Tech Stack:** POSIX sh (checked with `/bin/sh` and `/bin/dash`), GitHub Actions YAML.

**Content authority:** spec §5.2-§5.5 (including messages M1-M10; §5.1 is plan 10), §7.1 (`installers` job),
§10 (every installer row: empty/latest/`v`/bad/unreleased version, pre-release, no downloader,
rate limit, no release, 100 releases, airsl present/old/elsewhere, install-dir override, re-run,
agentstacks installer failure), §12 (installer row).

---

## Files that change

```
install/install.sh            — create: agentstacks pointer + the airsl step (spec §5.2, §5.3, §5.5)
install/claudevs.sh           — create: claudevs pointer (spec §5.2, §5.4, §5.5)
.github/workflows/ci.yml      — modify: add the `installers` job (spec §7.1)
```

## Order of work

### Task 1 — `install/install.sh`

**Files:**
- Create `install/install.sh`.

**Change:** A POSIX sh script with prefix `agentstacks-install`, `<VAR>` `AGENTSTACKS_VERSION`,
`<app>` `agentstacks-cli`, `<name>` `agentstacks`. It has a header comment with the two spec §5.3
usage lines and why it is sh, then `set -eu`. Its helpers are `say` (`<p>: <text>` to stderr),
`fail` (`say "error: <text>"`, exit 1) and `download` (copied from airsl's pointer: curl with
`--proto '=https' --tlsv1.2 -sSfL`, else `wget -qO-`). `main "$@"` is the last line. `main`
follows spec §5.2 steps 1-7 in order:
1. The curl/wget check with `command -v`, before anything else (M1).
2. `mktemp -d` with a trap.
3. Version from `${AGENTSTACKS_VERSION:-latest}`, one leading `v` stripped.
4. latest: M3, then the API body redirected to `$tmp/releases.json` in the current shell, never
   inside `$( )` (M4 on failure). The spec P18 grep | sed | sort | tail pipeline runs over that
   file (empty → M5). The bad-version `case` gives M2; anything else is used as given.
5. M6, then the dist installer downloaded to `$tmp/agentstacks-cli-installer.sh` (M7 on failure).
6. `sh` on it with `"$@"`. Its exit status ends the script when non-zero.
7. The spec §5.3 airsl step: `command -v airsl`, else an executable `$HOME/.local/bin/airsl` → M8;
   else M9, then airsl's pointer downloaded to `$tmp/airsl-install.sh` and run with `sh` (M10 on
   any failure).

Every message is the exact spec §5.5 text. The script never replaces or upgrades an airsl it
finds, and passes arguments only to the agentstacks installer.

**Proof:** checks written first and seen failing (the script is absent), each run under `/bin/sh`
and `/bin/dash`:
- `parses` — `sh -n install/install.sh` exits 0.
- `no-downloader-m1-only` — `env -i PATH=/nonexistent /bin/sh install/install.sh` exits 1 and its
  stderr is exactly `agentstacks-install: error: neither curl nor wget is installed`.
- `bad-version-m2` — for each of `a/b`, `0.1 5` and `v`:
  `env -i PATH=/usr/bin:/bin HOME="$(mktemp -d)" AGENTSTACKS_VERSION='<v>' /bin/sh install/install.sh`
  exits 1 and its stderr is exactly the M2 line with `'<v>'`.
- `latest-no-release-m5` — with `AGENTSTACKS_VERSION` unset (the live API, where agentstacks has
  no release yet), stderr is M3 then M5, exit 1. If the API refuses (rate limit), M3 then M4 is the
  expected pair instead, and the check says which it saw.
- `pinned-unreleased-m7` — `AGENTSTACKS_VERSION=v0.1.0` prints M6 with `0.1.0` (the `v` stripped),
  then M7, exit 1.
The stub checks below run the real, unmodified `install/install.sh` with
`PATH=<stubs>:/usr/bin:/bin`, so no real `airsl` on the machine is found. A fake `curl` answers by URL: the API URL gets a local `releases.json`, the installer URL a
stub dist installer, and the airsl pointer URL a stub airsl installer. Each stub records its
arguments and the values of `AGENTSTACKS_CLI_INSTALL_DIR` and `AIRSL_VERSION` in a file, and
`HOME` is a fresh temp directory:
- `airsl-found-m8` — `releases.json` holds `"tag_name": "agentstacks-cli-v0.1.0"`, and a fake
  executable `airsl` is on `PATH`. The script prints M8 with that path and exits 0.
- `airsl-home-m8` — no `airsl` on `PATH`, an executable `$HOME/.local/bin/airsl`. M8 names that
  path, exit 0.
- `airsl-missing-m9` — no airsl anywhere. The script prints M9, runs the airsl stub and exits 0.
- `airsl-download-fails-m10` — the fake `curl` fails for the airsl pointer URL: M9 then M10,
  exit 1.
- `airsl-install-fails-m10` — the airsl stub exits 1: M9 then M10, exit 1.
- `dist-installer-fails` — the dist installer stub exits 3: the script exits 3 and prints neither
  M8 nor M9.
- `args-to-agentstacks-only` — `sh install/install.sh --flag-a` with airsl missing: the dist
  installer stub recorded `--flag-a`, and the airsl stub recorded no argument.
- `prerelease-not-latest` — `releases.json` holds `agentstacks-cli-v0.2.0-rc.1` and
  `agentstacks-cli-v0.1.0`: M6 names `agentstacks 0.1.0`.
- `api-fails-m4` — the fake `curl` fails for the API URL: stderr is M3 then exactly M4, exit 1.
- `empty-and-latest` — with `AGENTSTACKS_VERSION=''` and then `AGENTSTACKS_VERSION=latest`, each
  run prints M3, then M6 for `agentstacks 0.1.0` from the stubbed `releases.json`.
- `env-passes-through` — with `AGENTSTACKS_CLI_INSTALL_DIR=/x` and `AIRSL_VERSION=0.1.5` set and
  airsl missing, the dist installer stub and the airsl stub each recorded both values unchanged
  (spec §5.3).

**Done when:** the checks pass and `cargo make dod` is green. Commit
`feat(repo): install.sh installs agentstacks and airsl`.

### Task 2 — `install/claudevs.sh`

**Files:**
- Create `install/claudevs.sh`.

**Change:** The same structure as Task 1's script (spec §5.2 steps 1-6), written out in full in
this file. Prefix `claudevs-install`, `<VAR>` `CLAUDEVS_VERSION`, `<app>` `claudevs-cli`,
`<name>` `claudevs`, the two spec §5.4 usage lines, and no airsl step (spec §5.4). Every message is
the exact spec §5.5 text with these substitutions.

**Proof:** checks written first and seen failing, each under `/bin/sh` and `/bin/dash`:
- `parses` — `sh -n install/claudevs.sh` exits 0.
- `no-downloader-m1-only` — `env -i PATH=/nonexistent /bin/sh install/claudevs.sh` exits 1 with
  exactly `claudevs-install: error: neither curl nor wget is installed`.
- `bad-version-m2` — `a/b`, `0.1 5`, `v` each give exactly the M2 line with
  `CLAUDEVS_VERSION='<v>'`, exit 1.
- `latest-no-release-m5` — unset `CLAUDEVS_VERSION` against the live API: M3 then M5 (no
  `claudevs-cli` release in agentstacks yet), exit 1, or M3 then M4 under a rate limit, as in Task 1.
- `pinned-unreleased-m7` — `CLAUDEVS_VERSION=0.1.2` prints M6 then M7, exit 1.
- `stub-run` — with stub tools built as in Task 1 (the fake `curl` serving
  `"tag_name": "claudevs-cli-v0.1.2"` and a stub installer), the real script exits 0 after M3 and
  M6 for `claudevs 0.1.2`, and prints nothing about airsl.
- `dist-installer-fails` — the stub installer exits 3: the script exits 3 (spec §5.2 step 6).
- `api-fails-m4` — the fake `curl` fails for the API URL: M3 then exactly M4 with
  `CLAUDEVS_VERSION`, exit 1.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`feat(repo): claudevs.sh installs claudevs from agentstacks releases`.

### Task 3 — CI job

**Files:**
- Modify `.github/workflows/ci.yml` — add the job `installers` (`runs-on: ubuntu-latest`,
  `timeout-minutes: 5`). It checks out, then runs the three spec §7.1 checks for each of
  `install/install.sh` and `install/claudevs.sh`: `sh -n` exits 0; with `env -i PATH=/nonexistent`
  the exit is 1 and stderr is exactly the M1 line; with
  `env -i PATH=/usr/bin:/bin HOME="$RUNNER_TEMP" <VAR>='a/b'` (`RUNNER_TEMP`, spec P53) the exit is
  1 and stderr is exactly the M2 line. No other job changes.

**Change:** CI catches an installer that does not parse or whose first refusals drift.

**Proof:** checks written first and seen failing:
- `installers-job-local` — the job's `run:` script, extracted and run locally under `bash -e`
  from the repository root with `RUNNER_TEMP="$(mktemp -d)"`, exits 0. A copy of
  `install/install.sh` with a syntax error makes it exit non-zero.
- `ci-other-jobs-unchanged` — written first as a script over `git diff .github/workflows/ci.yml`
  that fails unless the diff is non-empty, has no removed line, and adds only lines inside the
  `installers` job; seen failing on the untouched file (empty diff), then passing.

**Done when:** the checks pass and `cargo make dod` is green. Commit
`ci(repo): check the installers`.

---

## Risks

- The `latest-*` checks call the live GitHub API. A rate limit turns M5 into M4. Both are spec
  messages, and the check names which one it saw instead of failing on the limit.
- The fake-`curl` stubs (Task 1) prove the airsl step's branching, not airsl's real installer. The
  real end-to-end run is the author's §9 step 5.
- The scripts are served only once plan 10's `pages.yml` deploys and the author sets the Pages
  source (spec §9 step 2).

## Proof

- Both scripts pass `sh -n` under `/bin/sh` and `/bin/dash`; the `installers` job body passes
  locally.
- `cargo make dod` green.

## Review findings

- F1 🟡 `install/install.sh:11` header said agentstacks needs airsl "to run extensions" (extensions
  run on the embedded airsl library; the binary is for pack hooks and `plan`) — fixed: reworded;
  `shellcheck install/install.sh` clean, `sh task1.sh` → `FAILS=0`.
- F2 🔵 the scripts' header cites the Pages workflow `pages.yml`, created by plan 10 — open until
  plan 10 lands.
- F3 🟡 CI checks only parse, M1 and M2 (what this plan asks); M3–M10 and the airsl branching have
  no committed reversion guard — open.

## Deviations

- 2026-11-24 — One coder ran all three tasks; the task-briefer was not used. Commits per task with
  this plan's messages, made by the orchestrator after review.
- 2026-11-24 — The live checks (`latest-no-release-m5`, `pinned-unreleased-m7`) compare only the
  script's own prefixed stderr lines: real `curl -sS` adds its own `curl: (56) … 404` line. The stub
  checks and the CI job compare stderr exactly.
- 2026-11-24 — `dist-installer-fails` asserts no `airsl` text at all in stderr, a stricter form of
  "no M8/M9".
- 2026-11-24 — The scripts carry no executable bit (run with `sh`, as airsl's pointer).
