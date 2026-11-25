---
status: done
created: 2026-11-25
depends-on: [01]
built-on: 63f62977694c
---

# OpenCode Check Journal Export Implementation Plan

**Goal:** `cargo make opencode-check` proves the OpenCode behaviour the journal's OpenCode
`session.md` relies on.

**Architecture:** Two scenarios, S8a and S8b, are added to
`crates/agentstacks-cli/tests/opencode-check/check.mjs`, after S7, on the existing helpers
(`startMock`/`withOpencode`, `makeHome`, `isolatedEnv`, `runArgs`, `copyProject`, `sessionId`).
The mock model's scripted `shell` steps run the commands in the session's shell and write their
output to files in the project. A new helper gives OpenCode a `PATH` on which `opencode` resolves.
They need no pack change: they test OpenCode, not the journal text. Depends on plan 01, so the
final opencode-check run (S1-S8b) is on the tree with the new agent permissions.

**Tech Stack:** Node (ES modules), the pinned OpenCode v2.4.1, the mock model `mock.mjs`.

**Content authority:** spec §10 (rows S8a, S8b and the three bullets under the table), F13-F17,
F19, F25, E4, E5, K2.

---

## Files that change

```
crates/agentstacks-cli/tests/opencode-check/check.mjs — modify: header comment, PATH helper, s8a, s8b, scenarios list
```

## Order of work

### Task 1 — Prove a session with no service sees `stopped` and is not restarted

**Files:** Modify `crates/agentstacks-cli/tests/opencode-check/check.mjs` — header comment
(line 5), a new helper, a new `s8a`, the `scenarios` list in `main`.

**Change:**
- New helper `pathWithOpencode(binDir)`: creates `binDir`, puts a symlink named `opencode` to
  `OPENCODE` in it, and returns `binDir` followed by `process.env.PATH`. Its comment says why: the
  check runs OpenCode by absolute path, and a session's shell finds `opencode` only on `PATH`
  (spec F19).
- New helper (or inline code) to read a session after its run, the way spec §10 says: in the same
  isolated env, `opencode service stop`, then `opencode session export --standalone <id>`, parse
  the JSON, and collect its messages whose text starts with `The server restarted`.
- `s8a` (`journal export, no service`): a copied project; two turns, as the probes behind F15-F17 ran them
  (`probes/raw/o9-restart-cause.txt`, `o12-standalone-status-stopped.txt`): turn 1 is a text
  reply; turn 2 (`--session <id>`, as S6 runs its second turn) is a `shell` tool call whose command
  runs `opencode service status`, writing its stdout to `status.txt` and its exit
  status to `rc.txt` in the project, then a text reply. Both runs use `runArgs` (`--standalone --auto`) and `homeExtra: { PATH:
  pathWithOpencode(...) }`. Asserts: both runs exit 0;
  `status.txt` trimmed is `stopped`; `rc.txt` trimmed is `0`; the session read after the run has no
  message whose text starts with `The server restarted`.
- `main`'s list gains `["S8a", s8a]` after S7; the header comment says S1-S8b.
- Every service the scenario may start is stopped in a `finally` with `opencode service stop`
  in the same env, so no process is left (spec F25).

**Proof:** written first, seen failing, then passing — `S8a` — `cargo make opencode-check` prints
`ok S8a`. Seen failing first: without the `PATH` helper the shell's `rc.txt` reads `127` and the
scenario prints `FAIL S8a`.

**Done when:** `cargo make opencode-check` prints `ok S1` to `ok S8a` and exits 0, and
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`test(repo): opencode-check proves a session without the service is not restarted`.

### Task 2 — Prove an export with the service running

**Files:** Modify `crates/agentstacks-cli/tests/opencode-check/check.mjs` — a new `s8b`, the
`scenarios` list in `main`.

**Change:**
- `s8b` (`journal export, running service`): a copied project; before the run, in the same
  isolated env (the `PATH` from `pathWithOpencode`), `opencode service start` must exit 0. It is
  run with `spawnSync` and a timeout (as `main` runs `--version`), not through `run()`: `run()`
  kills the child's whole process group on close (`check.mjs:41-44,109-131`), which could take the
  service with it. Then `opencode service status` in the same env must print a URL starting
  `http://`, not `stopped`, before the run starts. Two
  turns, as in S8a: turn 1 a text reply; turn 2 (`--session <id>`) a `shell` tool call running
  `opencode session export "$OPENCODE_SESSION_ID" > export.json; echo $? > rc.txt`, then a text
  reply. Both runs use `runArgs` (`--standalone --auto`), in that same env (`homeExtra: { PATH:
  pathWithOpencode(...) }`, so `withOpencode` hands it to every `oc` call and the run). Asserts: both runs exit 0; `rc.txt`
  trimmed is `0`; `export.json` parses as JSON with keys `info` and `messages`, and `info.id`
  equals the run's session id (`sessionId`); the session read after the run (Task 1's reader) has
  no message whose text starts with `The server restarted`.
- `opencode service stop` runs in a `finally`, whether or not an assertion failed.
- `main`'s list gains `["S8b", s8b]` after S8a.

**Proof:** written first, seen failing, then passing — `S8b` — `cargo make opencode-check` prints
`ok S8b`. Seen failing first: with the `service start` step left out, the pre-run status assertion reads
`stopped` and the scenario prints `FAIL S8b`.

**Done when:** `cargo make opencode-check` prints `ok S1` to `ok S8b` and exits 0, and
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`test(repo): opencode-check proves the session export with the service running`.

## Risks

- OpenCode V2 may change `session export`, its JSON or the restart behaviour; S8a and S8b fail at
  the next pin bump, which is their purpose (spec K2).
- A service left running would outlive the check; both scenarios stop it in `finally`, and the
  isolated HOME keeps it apart from the user's.
- Riskiest step: Task 2's service start on CI (`ubuntu-latest`); the check already runs there in
  the `opencode-check` job, and a failure there shows as `FAIL S8b`.
- S8a and S8b pass `--auto` (from `runArgs`), which the probes did not. F20 shows these shell steps run
  without `--auto` too, so no assertion rests on the flag. If it changes the result, the
  scenario fails and execute records it as a deviation.

## Proof

- `cargo make opencode-check` prints `ok S1` … `ok S8b` and exits 0.
- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` green.

## Review findings

One reviewer pass over the diff of plans 04-06. The reviewer read `check.mjs`, and the orchestrator ran `cargo make opencode-check`: `ok S1` … `ok S8b`. The reviewer named nothing as blocking. All seven findings were fixed on the author's word.

After the fixes:

- `cargo make opencode-check` → `ok S1` … `ok S8b`, `Build Done`
- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` → exit 0

- 🟡 fixed — A signal between S8b's start and its stop left the service running. `requireNoService` now records the scenario's env in `serviceEnv`, and `cleanup()` runs `opencode service stop` there until a stop succeeds. Seen: in the mutated run below, S8b failed with its service still up, and no OpenCode service process was left after the check exited.
- 🟡 fixed — `.messages ?? []` could pass empty. `sessionMessages` now requires a non-empty `messages` array: `the session export has no messages: …`.
- 🟡 / ❓ fixed — A developer's own service could be seen or stopped. S8a and S8b first run `requireNoService`. It refuses with `an OpenCode background service already runs at <url>; stop it and run the check again` and stops nothing. Whether the port is shared across HOMEs is still unproven; the guard holds either way.
- 🔵 fixed — The header now says a scenario's background service is not in a group the check kills: the scenario stops it, and so does the cleanup at exit or on a signal.
- 🔵 fixed — Every status read goes through `serviceStatus`, which reports a non-zero exit with its code: `opencode service status exited <code>: …`.
- 🔵 fixed — Nothing caught a missing stop. Before the fix, `sessionMessages` stopped the service itself, so removing the `finally` stop still passed. Now `sessionMessages` does not stop it: the `finally` (`stopService`) is the only stop, `requireStopped` checks that none is left, and the session is read after that. Seen: with S8b's `stopService` removed, `FAIL S8b: a background service is left running at http://127.0.0.1:49374`; restored, `ok S8b`.
- 🔵 fixed — S8a's in-session command now writes stdout only (`opencode service status > status.txt`), as the plan says.

## Deviations

- 2026-11-25 — One `claudestacks:coder` built this plan with no task-briefer brief; it read the plan's tasks directly. It built Tasks 1 and 2 together and made no per-task commit.
- 2026-11-25 — Seen failing:
  - Without the `PATH` helper: `FAIL S8a: opencode service status in the session exited 127: zsh:1: command not found: opencode`.
  - With `service start` replaced by `--version`: `FAIL S8b: opencode service status printed "stopped", not a URL`.
  - `--auto` did not change either result.
- 2026-11-25 — Review fixes moved the after-run session read out of the `try`: the read now runs after the `finally` stop and `requireStopped`. The plan's order of assertions is otherwise kept.
