---
status: approved
created: 2026-11-25
depends-on: [04]
built-on: c1dbedb148db
---

# musecode-check Implementation Plan

**Goal:** `cargo make musecode-check` runs the musecode extension inside the pinned real Muse Code against a mock model.

**Architecture:** Test tooling under `crates/agentstacks-cli/tests/musecode-check/` (a Node mock of
Muse Code's model endpoint, a Node scenario runner, a check pack), a cargo-make task that downloads
and verifies the pinned binary, and a CI job that runs the task; none of it is installed into a
project (spec §8.3, §8.4). It mirrors `crates/agentstacks-cli/tests/opencode-check/`,
`[tasks.opencode-check]` and the `opencode-check` CI job.

**Tech Stack:** Node 22 (`node:http`, `node:child_process`, `node:fs`, no packages), cargo-make,
GitHub Actions; Muse Code 1.4.25-R9361.1; airsl 0.1.5.

**Content authority:** spec §1 D9-D10, §8.3, §8.4.

---

## Files that change

```
crates/agentstacks-cli/tests/musecode-check/mock.mjs     — create: the mock model endpoint
crates/agentstacks-cli/tests/musecode-check/pack/…       — create: the check pack of spec §8.3
crates/agentstacks-cli/tests/musecode-check/check.mjs    — create: scratch project, scenarios S1-S6, ok/FAIL lines
Makefile.toml                                            — modify: [tasks.musecode-check]
.github/workflows/ci.yml                                 — modify: the musecode-check job; the airsl pin comments
```

## Order of work

### Task 1 — Mock model and check pack

**Files:**
- Create `crates/agentstacks-cli/tests/musecode-check/mock.mjs`.
- Create `crates/agentstacks-cli/tests/musecode-check/pack/` — `pack.toml` (name `check`,
  `requires.airsl = ">=0.1.5"`, hooks 0-6 exactly as the spec §8.3 pack table defines them),
  `skills/hello/SKILL.md`, `skills/hello/references/note.md`, `skills/ask/SKILL.md`,
  `agents/helper.md`, and one Lua module per Lua hook: `hooks/start.lua` (hook 1),
  `hooks/prompt.lua` (hook 2), `hooks/before.lua` (hook 3), `hooks/after.lua` (hook 4) and
  `hooks/stop.lua` (hook 6, returns nothing). Hook 6 needs its file even though the extension skips
  it: a Lua hook whose `run` file is missing is refused when the pack loads
  (`crates/agentstacks/src/types/pack/targets.rs:242`); the opencode-check pack ships `hooks/stop.lua`
  for the same reason.

**Change:** `mock.mjs <port> <script.json> <logDir>`: the `probes/harness/mock.mjs` logic
(`GET <base>/muse-code/models` with one model `fake-model`; `POST <base>/responses` answering
Responses SSE: a scripted `response.function_call_arguments.done` for the first unserved step whose
`when` substrings all appear and whose `unless` substrings do not, else a
`response.output_text.delta` text; every request body logged to `<logDir>/req-NNN.json`), listening
on `<port>` (`0` = any free port) on `127.0.0.1` and printing `listening <port>` once bound (spec
§8.3, D9). The scripted call shapes are those of the probes: `read_skill {name}`
(`probes/items-results.md` B6), `subagent_spawn` (B4), `bash {command, description}` (B5),
`write_file {path, content}` and `read_file {path, offset, limit}` (`probes/hooks-results.md` P6, P7).

**Proof:** written first, seen failing, then passing, with a mock instance and log directory of its
own (not one a scenario uses):
- `node mock.mjs 0 <file holding []> <dir>` prints `listening <n>`;
  `curl -s http://127.0.0.1:<n>/muse-code/models` returns the one-model list; `curl -s -X POST
  http://127.0.0.1:<n>/responses -d '{}'` returns SSE ending in `response.completed` and creates
  `<dir>/req-001.json`.
- In a scratch directory holding copies of `extensions/musecode` (as `ext/musecode`) and `pack/` (as
  `packs/check`), `target/debug/agentstacks init --tool ext/musecode --pack packs/check --yes` exits 0
  and prints skip lines for hooks 5 and 6 with the spec §4.4 H2 and H3 reasons.

**Done when:** both checks above pass and the Definition of Done is green (`cargo make dod`). Commit
`test(agentstacks-cli): musecode-check mock and pack`.

### Task 2 — Scenario runner and cargo-make task

**Files:**
- Create `crates/agentstacks-cli/tests/musecode-check/check.mjs`.
- Modify `Makefile.toml` — add `[tasks.musecode-check]` after `[tasks.opencode-check]`.

**Change:**
- `check.mjs <muse-binary> <agentstacks-binary>`: per scenario, a scratch project with
  `extensions/musecode` copied to `ext/musecode` and `pack/` to `packs/check`; the mock started on
  port `0` with a fresh log directory and its `listening` line awaited (none within 10 s → print
  `FAIL mock did not start`, exit 1); a scratch `HOME` seeded with `.config/muse/settings.json` and
  `auth.json` as spec §8.3 gives them; Muse Code run as `<muse> exec --trust-workspace --yolo
  "<prompt>"` from the project root with `HOME` and `TMPDIR` in the scratch dir, `XDG_CONFIG_HOME`
  unset and `TBH_CREDENTIAL_BACKEND=file`; scenarios S1-S6 asserted exactly as the spec §8.3 table
  says (`<project>` is the realpath); one `ok S<n>` or `FAIL S<n> <why>` line each; exit 1 on any
  failure.
- `[tasks.musecode-check]`, category `Muse Code`, not a dependency of `dod`; script per spec §8.4:
  pick `file` and SHA256 from `uname -s`/`uname -m` (`Linux x86_64` → `muse-x86-linux`,
  `c1b728fb9732503c5e1869e3ad2f2c9bdb5b2638fb4cf398d6e2c46aa6d8f94a`; `Darwin arm64` →
  `muse-aarch64-macos`, `d8faeec8b3a2507125d97f1a79ae18781d254e8cd376f26531587fb6e043c27d`; else
  `musecode-check: no pinned Muse Code build for <os> <arch>`, exit 1); `mkdir -p
  target/musecode-check/1.4.25-R9361.1`; when `…/muse` is missing, `curl -fsSL -o …/muse
  'https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=1.4.25-R9361.1&file=<file>'`;
  on every run verify the file's SHA256 (`sha256sum`, else `shasum -a 256`), as `opencode-check`
  does (`Makefile.toml:496-507`): a mismatch removes the file and exits 1 with
  `musecode-check: <file>: checksum mismatch`; `chmod +x`; `cargo build -p agentstacks-cli`; `node
  crates/agentstacks-cli/tests/musecode-check/check.mjs target/musecode-check/1.4.25-R9361.1/muse
  target/debug/agentstacks`. The pin is the version string plus the two hashes, changed together.

**Proof:** written first, seen failing, then passing:
- `cargo make musecode-check` on macOS arm64 prints `ok S1` … `ok S6` and exits 0; each scenario is
  first seen failing (an assertion inverted, or the scenario run before its check is written).
- With the stored file truncated, `cargo make musecode-check` prints
  `musecode-check: muse-aarch64-macos: checksum mismatch`, exits 1 and the file is gone; the next run
  downloads it again and passes.

**Done when:** `cargo make musecode-check` passes and the Definition of Done is green. Commit
`test(agentstacks-cli): musecode-check scenarios and task`.

### Task 3 — CI job

**Files:**
- Modify `.github/workflows/ci.yml` — a `musecode-check` job after `opencode-check`, and the airsl
  pin comments.

**Change:** `name: Muse Code check`, `runs-on: ubuntu-latest`, `timeout-minutes: 30`; the same
checkout, pinned-toolchain, rust-cache, cargo-make and airsl steps as the `opencode-check` job;
`actions/setup-node@v4` with `node-version: 22`; `actions/cache@v4` on
`target/musecode-check/1.4.25-R9361.1` keyed `musecode-1.4.25-R9361.1-linux-x64`; then
`cargo make musecode-check`; same triggers as the other jobs (spec §8.4). The comment at
`.github/workflows/ci.yml:59` ("The `opencode-check` job carries a second copy of the version and
hash: bump both jobs") and the matching comment in the `opencode-check` job name all three jobs.

**Proof:** `grep -n "musecode-check" .github/workflows/ci.yml` finds the job and its
`cargo make musecode-check` step; no comment in the file still says "bump both jobs". The Linux run
itself is spec §10 K1: it happens on the first push of the branch, and execute reports its result.

**Done when:** those greps hold and the Definition of Done is green. Commit `ci(repo): musecode-check job`.

## Risks

- K1: the Linux binary has not run here; the first CI run proves it. No local check can, so Task 3's
  done-check is a file check and the CI result is reported after the push.
- K2: the versioned download URL may stop answering after stable moves past 1.4.25; the check fails
  loudly and the fix is a pin bump.
- K4: `TBH_CREDENTIAL_BACKEND` is undocumented; local macOS runs may show the Keychain dialog again
  in a later Muse Code build.
- Riskiest step: Task 2's S3 (agent spawn) and S4 (hook ordering), which depend on how the mock
  scripts the calls; their shapes are the probes' (Task 1 Change).

## Proof

- `cargo make musecode-check` → `ok S1` … `ok S6` locally; the CI job green on the branch after the
  push; `cargo make dod` green.
