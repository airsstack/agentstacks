---
status: done
created: 2026-11-23
depends-on: [07, 08, 11]
built-on: 0f89fdb3ebb3
---

# Enforcement Implementation Plan

**Goal:** Point Claude at the guideline skill of every installed pack that ships `enforcement.json`.

**Architecture:**
- The enforce hook gets `{{root}}`, its own pack root (`.claude/agentstacks/core`), as `args[1]`.
  The candidates are the directories beside it (the subdirectories of its parent), in name order,
  whose root `enforcement.json` loads (spec §7 "Discovery"). The doctor takes the same pack root. The plugin's registry machinery goes:
  - `registry_path`, `read_registry` and the `@claudestacks` filter;
  - GATE 1;
  - the parity section.

  GATE 2, GATE 3, `load_manifest`, `toplevel` (GATE 3's `path_for_matching`) and the sentinels
  stay.
- Listing the parent uses `airsstack.fs.list`, which is gated by the read grant
  (`~/.cargo/registry/src/*/airsl-0.1.4/src/modules/fs.rs:288-302`). The hook's `read = ["/"]`
  covers it (spec §7 "Grants").
- `explain` moves into `hooks/lib/enforce.lua` as `M.explain`, beside a new `M.doctor` that owns
  the usage refusal. `hooks/doctor.lua` is a thin script the enforce-doctor skill runs. The hook and
  `rearm` become modules (spec §6.3 H4, H8; §6.5 `doctor`).
- Every module reads `airsstack.env` when called, never into a module-level local, so tests swap
  in a fake env table and need no `--allow-env` (spec §6.3 "Environment", §1
  `probes/envfake-results.md`).
- `pack.toml` gains the `rearm` table at index 1 and the `enforce` table at index 5. With plan 11's
  eight tables, the file is the §6.2 text.

**Tech Stack:** Lua under airsl 0.1.5, pack format, the `agentstacks` binary.

**Content authority:** spec §1 (enforce rows: `agent_id` sentinel, registry, GATE 1, sentinel
life, `toplevel`/`common_dir`, `fs.list`, env fake, doctor usage), §3 (idiom), §6.2 (core `rearm`
and `enforce` tables), §6.3 rows H4, H8, enforce-doctor and "Environment", §6.5 (`enforce_test`
row; new `enforce`, `rearm`, `doctor` cases), §7, §8 (doctor usage), §9 (enforce rows).

---

## Files that change

```
packs/core/hooks/lib/enforce.lua      — create: from the plugin lib; sibling discovery; registry, GATE 1, parity removed; explain and doctor; idiom
packs/core/hooks/lib/globs.lua        — create: carried unchanged
packs/core/hooks/globs_test.lua       — create: moved unchanged
packs/core/hooks/enforce.lua          — create: module; event fields; returns { context = pointers }
packs/core/hooks/enforce_test.lua     — create: moved; sibling-pack cases; plugins_root anchor; skill string
packs/core/hooks/doctor.lua           — create: thin script: M.doctor(arg, airsstack.env), out to stdout, err to stderr
packs/core/hooks/rearm.lua            — create: module; event.session_id
packs/core/hooks/rearm_test.lua       — create: new
packs/core/skills/enforce-doctor/SKILL.md — create: converted; new command line and read-back list
packs/core/pack.toml                  — modify: rearm table (index 1), enforce table (index 5)
```

## Order of work

### Task 1 — Sibling-pack discovery in the enforce library

**Files:**
- Create `packs/core/hooks/lib/enforce.lua`, `lib/globs.lua`, `globs_test.lua`, `enforce_test.lua`.

**Change:**
- `resolve` takes the pack root (`args[1]`) in place of the registry path:
  - it lists the directories beside it (its parent's subdirectories, `airsstack.fs.list`) in name
    order;
  - a directory with no `enforcement.json` is not listed;
  - one whose file does not load (`load_manifest`, unchanged) gives `<key>: no usable
    enforcement.json` and is skipped, while the others still apply;
  - the candidate key is the directory name.
- Removed: `registry_path`, `read_registry`, `MARKETPLACE_SUFFIX` and the `@claudestacks` filter,
  `AIRSSTACK_ENFORCE_REGISTRY`, `project_key`, `common_dir` and `select_record` (GATE 1), the
  `installPath` re-read, and `parity_report`, `tree_files` (the parity section). `toplevel` stays:
  GATE 3's `path_for_matching` calls it (spec §1, §7).
- Trace lines change exactly as the spec §7 table says:
  - `packs: N with enforcement.json under <packs root>` replaces the registry count;
  - `STOP: no pack ships a usable enforcement.json` replaces the zero-manifests STOP;
  - the `@claudestacks` STOP, `project key:`, GATE 1 and `using <installPath>` lines are gone.

  Every other line stays as in `lib/enforce.lua:533-605`.
- The sentinel prefix `claudestacks-enforce-` and the sentinel key (session, agent, stack, phase;
  24 h) are unchanged (spec §1, §6.4).
- The library's require of `globs` takes the §3 idiom: `base .. "globs"`.
- `enforce_test.lua`:
  - registry cases become sibling-pack cases: fixtures are directories holding `enforcement.json`
    under a temp packs root, with the pack root `<packs root>/core`, and assert the new trace
    lines;
  - GATE 1 and parity cases are removed;
  - `plugins_root()` (`:84-100`) anchors on `packs/core/pack.toml`;
  - the "no driver emits `permissionDecision`" sweep (`:521-540`) runs over `packs/`;
  - the `skill` string at `:18` is `/guideline-rust-guidelines`.

**Proof:**
- `two_sibling_packs_both_apply`: a packs root holding `core/`, `guideline-rust/` and
  `guideline-python/` (the latter two with an `enforcement.json`) and pack root `<root>/core` gives
  both guideline packs as candidates, in name order.
- `directory_without_manifest_is_not_listed`: a sibling with no file is absent from the trace.
- `unloadable_manifest_is_skipped_and_others_apply`: invalid JSON in one sibling traces
  `<key>: no usable enforcement.json`, and the other still yields its pointer.
- `no_manifest_anywhere_stops`: the trace ends `STOP: no pack ships a usable enforcement.json`.
- `trace_counts_packs_under_root`: the first trace line is
  `packs: 2 with enforcement.json under <root>`.
- The moved GATE 2 / GATE 3 (including the `path_for_matching` cases at the plugin's
  `enforce_test.lua:404-438`) and sentinel cases pass unchanged; `globs_test` passes.

**Done when:** `cargo make lua` passes and `cargo make dod` is green. Commit
`feat(repo): enforce finds guideline packs beside core`.

### Task 2 — Enforce and rearm hook modules

**Files:**
- Create `packs/core/hooks/enforce.lua`, `hooks/rearm.lua`, `hooks/rearm_test.lua`.
- Modify `packs/core/pack.toml`, `hooks/enforce_test.lua`.

**Change:**
- `enforce.lua` returns `function(event)`:
  - it builds its settings inside the call from `airsstack.env` (the plugin's module-level
    `local env = airsstack.env`, `enforce.lua:23`, moves inside);
  - it reads `event.path`, `event.cwd` and `event.session_id`;
  - the sentinel agent key is `event.agent_id`, or `"main"` when nil, as today (spec §1:
    `enforce.lua:137`);
  - the pack root is `event.args[1]`;
  - it returns `{ context = <pointers joined by "\n"> }`, or `nil` when there are none.

  The `--explain` branch and `explain` leave the file (Task 3). The usage comments showing an
  `airsl run` command line (`enforce.lua:11`, `rearm.lua:9`) are dropped (spec §6.4 comment row). The require of the library is
  `base .. "lib.enforce"`.
- `rearm.lua` returns `function(event)`. It clears the sentinels of `event.session_id` in
  `enforce.sentinel_dir(airsstack.env)`, read when called, and returns `nil`. Its require is
  `base .. "lib.enforce"`.
- `pack.toml`: the §6.2 `rearm` table (`session.start` `match = ["compact"]`; grants `env =
  ["TMPDIR"]`, `read` and `write` `["{{var tmpdir}}"]`) goes at index 1. The §6.2 `enforce` table
  (`tool.before` `match = ["read", "edit", "write"]`, `args = ["{{root}}"]`; grants `env = ["HOME",
  "TMPDIR", "AIRSSTACK_HOME"]`, `read = ["/"]`, `write = ["{{var tmpdir}}"]`) goes at index 5.
  The file now equals the §6.2 core text.

**Proof:**
- New cases in `enforce_test.lua`:
  - `hook_returns_pointer_for_a_rust_file`: with `airsstack.env` swapped for a fake whose `TMPDIR`
    is a fresh temp directory (restored after), a temp packs root holding `core/` and
    `guideline-rust/enforcement.json`, `args = {"<packs root>/core"}`, a project with `Cargo.toml`
    and a `tool.before` event for `src/main.rs` give `{ context = … }` containing
    `/guideline-rust-guidelines`;
  - `hook_keys_sentinel_by_agent_id`: two events, `agent_id` `a1` and `a2`, each get the pointer
    once;
  - `nil_agent_id_uses_main` (same fake env).
- `rearm_test.lua`, `rearm_clears_this_session_only`: under a fake env whose `TMPDIR` is a temp
  directory, sentinels of sessions `s1` and `s2` exist, and after an event for `s1`, only `s2`'s
  remain.
- `agentstacks plan` on a core-only temp project lists exactly one skip,
  `core/hook/3: claude shows no hook output to the model at session end`.

**Done when:** `cargo make lua` passes and `cargo make dod` is green. Commit
`feat(repo): core enforce and rearm as pack hooks`.

### Task 3 — Enforce doctor

**Files:**
- Create `packs/core/hooks/doctor.lua`, `packs/core/skills/enforce-doctor/SKILL.md`.
- Modify `hooks/lib/enforce.lua`, `hooks/enforce_test.lua`.

**Change:**
- `lib/enforce.lua` gains:
  - `M.explain(pack_root, file_path, env)`: the plugin's `explain` (`enforce.lua:47-112`) without
    the parity block. Its header is the plugin's, except `registry: <path>` becomes
    `packs:    <parent of pack_root>`. Then the trace and the `outcome:` lines, unchanged;
  - `M.doctor(args, env)`: returns `nil, "usage: doctor.lua <pack-root> <path>\n"` when `args[1]` or
    `args[2]` is missing or empty, else `M.explain(args[1], args[2], env), nil`.
- `doctor.lua` is a script, `doctor.lua <pack-root> <path>`. It requires `lib.enforce` plainly,
  calls `M.doctor(arg, airsstack.env)`, writes `out` to stdout and `err` to stderr, and returns, so
  it exits 0 either way (spec §7, §8; §1 `probes/doctor-usage-results.md`).
- `enforce-doctor/SKILL.md`, converted by §6.4. Its command is the spec §7 block:
  `airsl run --policy confined --allow-env HOME --allow-env TMPDIR --allow-env AIRSSTACK_HOME
  --allow-read / --allow-write "{{var tmpdir}}" "{{var project_dir}}/{{root}}/hooks/doctor.lua"
  "{{var project_dir}}/{{root}}" "{{var arguments}}"`.
- The read-back list (`:26-58`):
  - the `no @claudestacks plugins in the registry` item goes;
  - the `zero manifests loaded` item reads "`no pack ships a usable enforcement.json`: no installed
    pack carries a readable `enforcement.json`. Add a guideline pack to `agentstacks.toml`, then
    run `agentstacks apply`";
  - the GATE 1 item, the `using <installPath>` paragraph and point 3 (parity) go.
- The guard paragraph keeps its `${TMPDIR:-/tmp}` advice and drops `${CLAUDE_PLUGIN_ROOT}`. The
  description's `/claudestacks:enforce-doctor <path>` becomes `{{skill enforce-doctor}} <path>`.

**Proof:**
- `doctor_prints_packs_header_and_trace` (in `enforce_test.lua`): under a fake env,
  `M.doctor({"<packs root>/core", "<project>/src/main.rs"}, env)` returns text holding
  `packs:    <packs root>`, then the trace and `outcome:` lines, no line containing `parity`, and
  `nil` error.
- `doctor_usage_without_arguments`: `M.doctor({}, env)` and `M.doctor({"/r", ""}, env)` each return
  `nil` and `"usage: doctor.lua <pack-root> <path>\n"`.
- `airsl check packs` compiles `doctor.lua`.
- After `init` with core and guideline-rust in a temp project, the rendered
  `.claude/skills/core-enforce-doctor/SKILL.md` command, run by hand with `src/main.rs`, prints a
  trace that names `guideline-rust`.
- The spec §6.4 gate greps over `packs/core` print nothing.

**Done when:** `cargo make lua` passes and `cargo make dod` is green. Commit
`feat(repo): core enforce doctor over sibling packs`.

---

## Risks

- `lib/enforce.lua` is the largest file converted. Task 1 removes code by the named functions only.
  Every trace line not in the §7 table must print as before, and the moved GATE 2 / GATE 3 tests
  guard that.
- `args[1]` is project-relative. The hook runs with the project as its working directory
  (`docs/hooks.md` §3), so the parent `.claude/agentstacks` resolves. Tests pass absolute temp
  roots.
- A test that swaps `airsstack.env` must restore it even when an assertion fails (wrap the body in
  `pcall`, restore, then re-raise), or later tests see the fake.

## Proof

- `cargo make dod` green. `packs/core/pack.toml` equals the spec §6.2 core text byte for byte (`diff`
  against the spec block).

## Review findings

Reviewer re-ran `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` exit 0 (23 `test result: ok`; lua `11 file(s) compiled` + `67 file(s) compiled`, 0 failed; `71 passed, 0 failed` and `640 passed, 0 failed (25 files)`; `enforce_test` 34, `globs_test` 12, `rearm_test` 2). `packs/core/pack.toml` diffed against the spec §6.2 core block: identical. Orchestrator proofs: core-only `plan` → exactly one skip, `core/hook/3: claude shows no hook output to the model at session end`; core + guideline-rust `init` exit 0, 50 written; the rendered `PreToolUse` `Read|Edit|Write` command piped a `Read` of `src/main.rs` → `{"hookSpecificOutput":{"additionalContext":"rust work is in play. The /guideline-rust-guidelines skill is MANDATORY …"}}`; the same event again → no output (sentinel); `SessionStart` `compact` (rearm) → the next `Read` points again; the rendered `.claude/skills/core-enforce-doctor/SKILL.md` command run by hand → `packs:    <project>/.claude/agentstacks`, `packs: 1 with enforcement.json under …`, `guideline-rust: EMIT /guideline-rust-guidelines`, `outcome: 1 pointer(s)`, exit 0; the §6.4 gate greps over `packs/core` print nothing.

- 🟡 declined — `skills/enforce-doctor/SKILL.md:14,19` (named blocking): "`${CLAUDE_PROJECT_DIR}` is empty in the Bash tool, so the command runs `/.claude/…`". Spec §1 proves Claude Code substitutes `${CLAUDE_PROJECT_DIR}` in a project skill's text with the absolute path (`probes/claude-projdir-results.md`), and that the Bash tool lacks the variable (`probes/claude-handback-results.md` R3); the §6.4 SKILL.md row is built on exactly that. The model receives an absolute literal, so the guard paragraph's `${TMPDIR:-/tmp}` advice covers the only variable left. No change.
- 🟡 open — `hooks/enforce_test.lua:296`: the trace labels enforce-doctor reads back (`GATE 2 no detect marker`, `already claimed`, `STOP: no manifest declares phase`, `sentinel claimed concurrently`, `match path:`) are not asserted; only `GATE 3 no match glob hit` is.
- 🔵 open — `hooks/enforce.lua:9`, `hooks/lib/enforce.lua:79`: rationale names "a worktree-isolated Claude Code session" in a tool-neutral pack.
- 🔵 open — `hooks/enforce.lua:26`: the `args[1]` missing/empty guard has no test.
- 🔵 open — `hooks/enforce_test.lua:168,703`: history narrative ("an earlier version of this guard", "a previous version of this test").
- 🔵 open — `hooks/enforce_test.lua:543`: no test now covers `toplevel`/`path_for_matching` in a linked worktree (`.git` file).
- 🔵 open — `hooks/lib/enforce.lua:83`: `dot_git` now returns the directory holding `.git`; `toplevel` is an alias.
- 🔵 open — `hooks/lib/enforce.lua:353`: `table.sort(names)` repeats the sort airsl 0.1.5 `fs.list` already does.
- 🔵 open — `hooks/lib/enforce.lua:458`: the `M.home` `HOME` fallback is untested.
- 🔵 open — `hooks/lib/enforce.lua:468`: "The framework" is plugin wording.

## Deviations

- 2026-11-24 — Tasks 1–3 share files, so one coder ran them in order; plan 13 ran in parallel on disjoint files. The coder ran file-local checks; the orchestrator ran `dod` and the proofs once.
- 2026-11-24 — Task 1: `globs.lua` ("plugin authors" → "pack authors") and `globs_test.lua` (usage line → `cargo make lua`) are not byte-identical to the plugin; comment-only.
- 2026-11-24 — Task 1: dropped the lib tests of removed code (`every_worktree_of_one_repository_shares_a_key`, `a_hand_built_linked_worktree_shares_the_key_without_git`, `a_malformed_dot_git_file_stops_the_ascent`, and the registry-only cases); the plugin's `enforce_test.lua:404-438` range holds two of them, which test `project_key`, removed by this plan. `dot_git` returns only the root (its `.git` path fed only `common_dir`).
- 2026-11-24 — Task 1: the `packs: N` count includes directories whose `enforcement.json` fails to load (they also get the `no usable` line). Extra tests: `trace_has_no_registry_or_gate_one_lines`, `a_missing_packs_root_resolves_to_nothing`, `a_manifest_missing_required_fields_skips_that_pack`, `hook_returns_nil_when_no_pack_matches`, `hook_ignores_an_event_without_a_path`, `doctor_trace_names_the_stop_when_no_pack_ships_a_manifest`, `rearm_without_a_session_id_leaves_other_sessions_alone`.
- 2026-11-24 — Task 3: `M.home(env)` added to the lib so the hook and the doctor share the `AIRSSTACK_HOME` / `HOME` rule.
- 2026-11-24 — Comments naming the plugin layout (registry, cache, `scripts/lib` paths, the plugin protocol path) were rewritten.
- 2026-11-24 — Not committed (per-task commits held for the author).
