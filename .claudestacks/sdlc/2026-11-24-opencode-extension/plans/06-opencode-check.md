---
status: approved
created: 2026-11-25
depends-on: [05]
built-on: d54f8422f1ff
---

# OpenCode Check Implementation Plan

**Goal:** `cargo make opencode-check` runs the opencode extension inside pinned OpenCode v2.4.1 against a local mock model, the same way on a developer machine as in CI.

**Architecture:** Test tooling under `crates/agentstacks-cli/tests/opencode-check/` (never installed
into a project): `mock.mjs` (the proven `probes/harness/mock.mjs` logic), `check.mjs` (scratch project,
isolated OpenCode runs, assertions) and the check pack. A cargo-make task outside `dod` fetches and
verifies the pinned binary and runs `check.mjs`; a CI job runs the task on `ubuntu-latest`.

**Tech Stack:** Node 22 (test tooling), shell (cargo-make script), GitHub Actions, OpenCode v2.4.1, airsl 0.1.5.

**Content authority:** spec §8.3, §8.4, D5, D6, D16.

---

## Files that change

```
crates/agentstacks-cli/tests/opencode-check/mock.mjs        — create: mock OpenAI-compatible chat server
crates/agentstacks-cli/tests/opencode-check/check.mjs       — create: scenarios S1-S7 and assertions
crates/agentstacks-cli/tests/opencode-check/pack/           — create: the check pack (spec §8.3 table)
Makefile.toml                                               — modify: [tasks.opencode-check]
.github/workflows/ci.yml                                    — modify: opencode-check job
```

## Order of work

### Task 1 — Mock model and check pack

**Files:**
- Create `crates/agentstacks-cli/tests/opencode-check/mock.mjs` — `node mock.mjs <port> <scenario.json> <log.jsonl>`: the `probes/harness/mock.mjs` behaviour (one scripted step per request that offers tools; title requests answered `title`; every request body appended to the log).
- Create `crates/agentstacks-cli/tests/opencode-check/pack/` — `pack.toml` (name `check`, `requires.airsl = ">=0.1.5"`, hooks 0-6 exactly as the spec §8.3 table), `skills/hello/SKILL.md` + `skills/hello/references/note.md`, `skills/ask/SKILL.md`, `agents/helper.md`, `hooks/*.lua` modules for hooks 1-4 and 6.

**Change:** The fixed inputs of the check.

**Proof:**
- `airsl check crates/agentstacks-cli/tests/opencode-check/pack` compiles every hook module.
- In a scratch git repository holding copies at `ext/opencode` and `packs/check` (plan 05 is done, so `extensions/opencode` exists): `target/debug/agentstacks init --tool ext/opencode --pack packs/check --yes` exits 0 and prints `applied:`; then `target/debug/agentstacks plan` exits 0 and prints `0 to write` and `2 skipped`, with the hook 5 and hook 6 skip lines carrying the spec §4.6 reasons `opencode has no hook when a session ends` and `opencode has no hook when a subagent stops`. (`plan` takes no flags; without `--yes`, `init` with no terminal writes `agentstacks.toml` and exits 1, `probes/review-results.md` R3.)

**Done when:** both checks above pass and `cargo make dod` is green. Commit `test(repo): opencode check pack and mock model`.

### Task 2 — Scenarios

**Files:**
- Create `crates/agentstacks-cli/tests/opencode-check/check.mjs` — `node check.mjs <opencode> <agentstacks>`.

**Change:** Builds a scratch project by copying `extensions/opencode` to `<scratch>/ext/opencode` and
`pack/` to `<scratch>/packs/check`, passing relative paths. Isolates every OpenCode run exactly as
`probes/harness/run.sh` (HOME and XDG dirs in the scratch dir, `--standalone --auto`,
`OPENCODE_DISABLE_MODELS_FETCH=1`, `OPENCODE_DISABLE_AUTOUPDATE=1`, global `opencode.json` with
provider `mock`, package `@opencode/ai/providers/openai-compatible`, `baseURL`
`http://127.0.0.1:<port>/v1`, model `mock/m`). Runs S1-S7 of spec §8.3, prints one `ok <S>` or
`FAIL <S>: <reason>` line each, exits 1 on any failure, and kills every mock and server it started.
For S7 it starts one `opencode serve --port <p>` in the scratch project with `OPENCODE_PASSWORD`
set and a `PATH` with every directory holding an `airsl` file removed, then drives it with
`opencode run --server http://127.0.0.1:<p> --auto --format json` clients (the shape of
`probes/review-results.md` R4); the mock runs under `process.execPath`, so it does not
depend on that `PATH`.

**Proof:** (the scenarios are the tests; each asserts what spec §8.3 lists)
- S1 — `init`, `plan` (`0 to write`), `doctor` (`healthy`), `update` (`nothing to update`), skip lines for hooks 5 and 6.
- S2 — `skill` call for `check-hello`: `HELLO-BODY`, `Base directory for this skill:`, the `references/note.md` path; the system prompt lists `check-hello`, not `check-ask`.
- S3 — `subagent` call for `check-helper`: a later request holds `HELPER-BODY` and offers only `grep, read, skill`.
- S4 — first user message holds `CHECK-WELCOME`, `CHECK-START startup`, `CHECK-PROMPT`; `blocked.txt` absent and its error carries `CHECK-BLOCK blocked.txt`; the `ok.txt` result carries `CHECK-BEFORE ok.txt`; the read result carries `CHECK-AFTER`; the shell prints the scratch project root.
- S5 — `mock/gpt-5`: first patch blocked with `CHECK-BLOCK blocked.txt` and nothing written; second patch result carries `CHECK-BEFORE ok2.txt`.
- S6 — `opencode api --standalone session.command` with `check-ask` and `a b`, then `opencode run --session <id>`: the model receives `Load the skill \`check-ask\`` and `a b`.
- S7 — airsl off the server's `PATH`: prompt 1 `hello`, in which the model writes `blocked.txt`, then prompt 2 `again` with `--session <id>`. Prompt 1's user message holds `CHECK-WELCOME` and ends with `agentstacks: airsl is not on PATH, so the agentstacks Lua hooks are not running. Install airsl, then run agentstacks doctor.` exactly once, without `CHECK-START` or `CHECK-PROMPT`; `blocked.txt` exists (the guard is off); prompt 2's user message holds no notice; both clients exit 0.

**Done when:** `node crates/agentstacks-cli/tests/opencode-check/check.mjs ~/.opencode/bin/opencode target/debug/agentstacks` prints seven `ok` lines on the developer machine (macOS arm64, v2.4.1), and `cargo make dod` is green. Commit `test(repo): opencode check scenarios`.

### Task 3 — Task and CI job

**Files:**
- Modify `Makefile.toml` — `[tasks.opencode-check]`, category `OpenCode`, not a dependency of `dod`, script per spec §8.4 steps 1-4 (platform pick with both SHA256 values, `curl -sSfL` into `target/opencode-check/2.4.1/`, checksum with `sha256sum` else `shasum -a 256`, `tar -xzf` / `unzip -o -q`, the two exact refusal messages, `cargo build -p agentstacks-cli`, `node … check.mjs target/opencode-check/2.4.1/opencode target/debug/agentstacks`). A comment beside the pins says that bumping OpenCode changes the version and both hashes together (spec §8.4).
- Modify `.github/workflows/ci.yml` — job `opencode-check` on `ubuntu-latest`, `timeout-minutes: 30` (as `claudevs-check`), with the `dod` job's checkout, toolchain, cache, cargo-make and airsl steps, `actions/setup-node@v4` with `node-version: 22`, then `cargo make opencode-check`.

**Change:** The check runs the same way locally and in CI.

**Proof:**
- `cargo make opencode-check` on the developer machine downloads, verifies, unpacks and prints seven `ok` lines.
- `cargo make --list-all-steps` lists `opencode-check` under `OpenCode`; `dod`'s dependencies are unchanged.
- The CI job's first run on the pull request proves the Linux build and Linux `/bin/sh` exiting 127 for a missing airsl (S7) (spec D6, D17, K1).

**Done when:** `cargo make opencode-check` passes locally and `cargo make dod` is green. Commit `ci(repo): run opencode-check in CI`.

---

## Risks

- K1: the Linux binary has never run here; the first CI run is its proof. If it fails, execute reports
  it with the job log; the local macOS run stays green.
- `check.mjs` depends on OpenCode internals that the mock reads (request shapes); a pin bump re-runs it.
- Riskiest step: Task 2's S6, which drives a command through `opencode api` and a second `run`
  (spec §8.3, from `probes/config-results.md` P1b).

## Proof

- `cargo make opencode-check` seven `ok` lines locally; the CI job green on the pull request.
- `cargo make dod` green and unchanged in scope.
