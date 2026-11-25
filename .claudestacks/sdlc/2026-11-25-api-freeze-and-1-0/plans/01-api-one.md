---
status: approved
created: 2026-11-25
built-on: c187d693fbea
---

# api = 1 Implementation Plan

**Goal:** Every agentstacks reader, writer and text speaks `api = 1`.

**Architecture:** `types/api.rs` stays the one home of the number. It gains the api decision
(current, legacy 0, unsupported) and its error, so every reader asks one rule. `report/phrase.rs`
holds the two phrases. The project and lock readers return which api they read, and
`app/plan/read.rs`, the one place `plan` and `apply` read the project, refuses a legacy project.
`doctor` is untouched (D18). The number cannot change without every valid fixture, pack and
extension changing with it, so the flip is one task. The texts that name api 0 follow in a
second task.

**Tech Stack:** Rust (agentstacks, agentstacks-cli), Lua under airsl 0.1.5 (extensions, packs), TOML fixtures.

**Content authority:** spec §3.1, §3.2, §3.3, D4, D10, D17, D18, F7, F15, F18, E1, E1b, E4, E5, §12 rows `types/api.rs`, `types/project/file.rs`, `lock/file.rs`, `app/plan`, `app/apply`, `app/doctor.rs`.

---

## Files that change

```
crates/agentstacks/src/types/api.rs                    — modify: API = 1; the api decision and its error
crates/agentstacks/src/types/report/phrase.rs          — modify: legacy and unsupported phrases
crates/agentstacks/src/types/manifest/pack/file.rs     — modify: pack api via the decision
crates/agentstacks/src/types/extension/manifest.rs     — modify: extension api via the decision
crates/agentstacks/src/types/extension/loaded.rs       — modify: doc example text
crates/agentstacks/src/types/project/file.rs           — modify: reads api 0 or 1; exposes which
crates/agentstacks/src/types/lock/file.rs              — modify: reads api 0 or 1
crates/agentstacks/src/types/lock/write.rs             — modify: doc line; renders API
crates/agentstacks/src/app/plan/read.rs                — modify: refuses a legacy project
crates/agentstacks-cli/src/command/apply.rs            — modify: test that apply refuses a legacy project
crates/agentstacks/src/host/payload.rs                 — modify: test payload literals api 1
packs/{core,sdlc,journal,guideline-rust,guideline-python}/pack.toml — modify: api = 1
extensions/{claude,opencode}/extension.toml            — modify: api = 1
extensions/{claude,opencode}/lib/values_test.lua       — modify: payload api 1
crates/agentstacks/tests/fixtures/** and crates/agentstacks-cli/tests/** (F18) — modify: every valid api = 0 → 1
crates/agentstacks/tests/fixtures/extensions/invalid/api-one/ — rename: api-two/ with api = 2; create api-zero/ with api = 0
the 49 .rs files of F18, tests/common/hook.rs, crates/agentstacks-cli/src/command/testing.rs — modify: inline api 1
the F18 refused-case tests                             — modify: test api 2 unsupported and api 0 legacy
crates/agentstacks/src/types/hooks/invalid.rs          — modify: pure refusal from API
crates/agentstacks/src/types/pack/error.rs             — modify: reserved refusal from API
crates/agentstacks/src/types/hooks/{action,grants,lua,message,neutral,run,policy}.rs — modify: doc comments say api = 1
crates/agentstacks/assets/runtime.lua                  — modify: header says api 1 (D17)
extensions/claude/lib/translate.lua, extensions/opencode/lib/{translate,plugin}.lua — modify: headers say api 1
extensions/opencode/lib/{translate,plugin}_test.lua    — modify: header expectations
crates/agentstacks/tests/fixtures/{claude,opencode}/*/expected/** — modify: translate.lua / agentstacks.js headers
crates/agentstacks/tests/opencode.rs                   — modify: literal header text
```

## Order of work

### Task 1 — Flip to api 1

**Files:**
- Modify `crates/agentstacks/src/types/api.rs` — `API = 1`; `ApiCheck` (`Current`, `Legacy`, `Unsupported(i64)`) and `fn check(found: i64) -> ApiCheck`; the module doc names both
- Modify `crates/agentstacks/src/types/report/phrase.rs` — the unsupported phrase reads `API`; the legacy phrases `<found> is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)` and `api 0 is from agentstacks 0.x (run agentstacks update)` (§3.3 exact texts)
- Modify `crates/agentstacks/src/types/manifest/pack/file.rs`, `crates/agentstacks/src/types/extension/manifest.rs` — legacy and unsupported refusals at today's locations (F7)
- Modify `crates/agentstacks/src/types/extension/loaded.rs` — the doc example's text
- Modify `crates/agentstacks/src/types/project/file.rs` — accepts api 0 and 1; `ProjectFile::api()` returns the decision; any other value is unsupported
- Modify `crates/agentstacks/src/types/lock/file.rs` — accepts api 0 and 1; `Lockfile::api()` returns the decision (spec §3.2); any other value is unsupported
- Modify `crates/agentstacks/src/types/lock/write.rs` — renders `API`; doc line says `api = 1`
- Modify `crates/agentstacks/src/app/plan/read.rs` — a legacy project is the refusal `agentstacks.toml: api 0 is from agentstacks 0.x (run agentstacks update)`; `doctor` is not touched (D18)
- Modify every valid fixture and test text of F18 to `api = 1`: the five `pack.toml`, the two `extension.toml`, `values_test.lua` ×2, the `.toml` and `.lock` fixtures under both crates' `tests/`, the 49 `.rs` files with inline TOML, `host/payload.rs` test literals, `tests/common/hook.rs`, `crates/agentstacks-cli/src/command/testing.rs`, `crates/agentstacks-cli/tests/opencode-check/pack/pack.toml`
- Rename `crates/agentstacks/tests/fixtures/extensions/invalid/api-one/` → `api-two/` (`api = 2`); create `api-zero/` (`api = 0`)
- Modify the F18 refused-case tests (`types/set/check.rs:176-179`, `tests/pack.rs:256`, `tests/set.rs:201`, `types/lock/file.rs:562`, `types/project/file.rs:360`, `types/manifest/pack/file.rs:429`, `types/lock/error.rs:121`, `types/manifest/error.rs:254`, the extension fixture tests) — each tests `api = 2` as unsupported; the pack, extension and project ones also test `api = 0` as legacy

**Change:** §3.1–§3.3 as one atomic flip. Every valid file is at api 1, and a 0.x file is named legacy.

**Proof:**
- `api_is_one` — `API` equals 1.
- `zero_is_legacy`, `two_and_minus_one_are_unsupported` — `check(0)`, `check(2)` and `check(-1)`.
- `legacy_phrase_names_api_one`, `legacy_project_names_update` — the §3.3 texts exactly.
- `pack_api_zero_is_legacy` — `packs/demo/pack.toml: demo: api 0 is from agentstacks 0.x (this agentstacks speaks 1; set api = 1, or use a newer release of it)`.
- `pack_api_zero_without_name_is_legacy_at_pack_api` — `pack.api: 0 is from …`.
- `pack_api_two_is_unsupported` — `demo: api 2 not supported (this agentstacks speaks 1)`.
- `extension_load_refuses_api_zero_as_legacy`, `extension_load_refuses_api_two` — the two fixtures give the §3.3 extension texts.
- `project_reads_api_zero_as_legacy`, `project_refuses_api_two` — parse succeeds and reports legacy; `agentstacks.toml: api 2 not supported (this agentstacks speaks 1)`.
- `lock_reads_api_zero` (and `Lockfile::api()` reports legacy), `lock_refuses_api_two`, `lock_writes_api_one`.
- `legacy_project_refused` in the `app/plan` tests — a `FakeWorkspace` with an `api = 0` project gives exactly the §3.3 project refusal; nothing planned.
- `apply_refuses_a_legacy_project` in `crates/agentstacks-cli/src/command/apply.rs` tests — `apply --yes` on an `api = 0` project prints the §3.3 project refusal, exits 1 and stages nothing. `app::apply::apply` takes a `Plan` and never reads `agentstacks.toml` (`app/apply/flow.rs:35`), so the apply side of E1 is proven at the command, which reads through `app/plan/read.rs`.
- `legacy_lock_reports_as_today` in the `app/doctor.rs` tests — a lock with `api = 0` and matching files gives no finding (E1b).
- every existing integration suite (`tests/claude.rs`, `opencode.rs`, `pack.rs`, `set.rs`, `extension.rs`, `runtime.rs`, the CLI `tests/commands.rs`, `tests/packs.rs`) passes at api 1, and `cargo make lua` passes.

**Done when:** `cargo make dod`, `cargo make claudevs-check` and `cargo make opencode-check` pass. Commit `feat(repo): agentstacks speaks api 1`.

### Task 2 — Every text that names api 0 says api 1

**Files:**
- Modify `crates/agentstacks/src/types/hooks/invalid.rs` — the pure refusal reads `API`: `"pure" is not supported in api 1 (airsl has no require under pure, airsstack/airsl#12)` (§3.1); module doc
- Modify `crates/agentstacks/src/types/pack/error.rs` — `reserved, not supported in api 1`, number from `API` (§3.1)
- Modify `crates/agentstacks/src/types/hooks/{action,grants,lua,message,neutral,run,policy}.rs` — doc comments and examples say `api = 1`
- Modify `crates/agentstacks/assets/runtime.lua` — header `-- agentstacks hook runtime, api 1.`
- Modify `extensions/claude/lib/translate.lua`, `extensions/opencode/lib/translate.lua`, `extensions/opencode/lib/plugin.lua` — headers say api 1 (D17)
- Modify `extensions/opencode/lib/translate_test.lua`, `extensions/opencode/lib/plugin_test.lua` — header expectations
- Modify `crates/agentstacks/tests/fixtures/{claude,opencode}/*/expected/**` — the expected `translate.lua` and `agentstacks.js` headers
- Modify `crates/agentstacks/tests/opencode.rs` — any literal header text

**Change:** No text written by the core or an official extension names api 0. Each written file changes once (D17).

**Proof:**
- `pure_policy_refusal_names_api_one` — the §3.1 text exactly.
- `reserved_dir_refusal_names_api_one` — `<pack>/commands: reserved, not supported in api 1`.
- the runtime test of `RUNTIME_LUA` asserts the first line `-- agentstacks hook runtime, api 1.`
- the Lua tests in `translate_test.lua` and `plugin_test.lua` assert the api 1 headers.
- `tests/claude.rs` and `tests/opencode.rs` expected-fixture comparisons pass with the new headers.
- The api-0 gate (below, "Kept lines") prints only kept lines that exist at this point.

**Done when:** `cargo make dod` and `cargo make opencode-check` pass. Commit `feat(repo): written headers and refusals name api 1`.

---

## Risks

- Task 1 is large by necessity (F18: about 280 files), because the number and every valid fixture must change in one green commit. A missed valid fixture shows as a refused load in an integration test, so the suites are the net. Task 2's grep lists what is left.
- Changing the D17 headers changes the hash of `runtime.lua`, `translate.lua` and `agentstacks.js`. Expected fixtures and any test that pins those hashes move in Task 2, or `tests/claude.rs` and `tests/opencode.rs` fail.

## Proof

- `cargo make dod`, `cargo make claudevs-check` and `cargo make opencode-check` are green after each task.
- The api-0 gate prints only kept lines.

## Kept lines

The spec §8 gate is `git grep -nE 'api *= *0|api 0|speaks 0' -- crates extensions packs book README.md CLAUDE.md RELEASING.md`.
After plan 08 it prints only lines in the files below, each for the reason given. A line in any
other file, or a line in a listed file that is not of the listed kind, is a failure. Line numbers
are not given, because the plans before 08 move them. Each later plan's proof runs this gate over
the files it has touched.

| File | Kept because | Added or kept by |
|---|---|---|
| `packs/CHANGELOG.md` (the 0.1.0 and 0.2.0 entries, today `:12` and `:31`) | 0.x history | kept |
| `README.md` (the roadmap line `M1  claude extension (api = 0) ✓ released (0.1.0)`, today `:183`) | 0.x history | kept |
| `crates/agentstacks/src/types/report/phrase.rs` | the legacy phrase `api 0 is from agentstacks 0.x` itself | 01 |
| `crates/agentstacks/src/types/api.rs`, `types/project/file.rs`, `types/lock/file.rs`, `types/manifest/pack/file.rs`, `types/extension/manifest.rs`, `types/lock/error.rs`, `types/manifest/error.rs`, `types/set/check.rs`, `app/plan/*` tests, `app/doctor.rs` tests | legacy-case tests and docs (`check(0)`, the §3.3 cases, E1b) | 01 |
| `crates/agentstacks/tests/fixtures/extensions/invalid/api-zero/extension.toml`, `crates/agentstacks/tests/{pack,set,extension}.rs` | legacy-case fixture and tests | 01 |
| `crates/agentstacks-cli/src/command/apply.rs` tests | `apply_refuses_a_legacy_project` | 01 |
| `crates/agentstacks/src/types/project/upgrade.rs`, `app/update.rs`, `crates/agentstacks-cli/src/{command,show}/update.rs` tests, `crates/agentstacks-cli/tests/fixtures/legacy/**`, `crates/agentstacks-cli/tests/commands.rs` | the 0 → 1 move and its tests | 03 |
| `crates/agentstacks/src/app/ext/add.rs` tests | `legacy_project_refused` | 05 |
| `book/src/ch03-01-from-0x.md`, `book/src/ch04-00-cli.md` (`update`'s `api: 0 → 1` and the legacy messages), `book/src/ch05-00-plan-and-apply.md` (the lock reads api 0), `book/src/ch06-06-stability.md` (the lock reads api 0) | the 0.x move, documented | 07 |
| `crates/agentstacks/CHANGELOG.md`, `crates/agentstacks-cli/CHANGELOG.md`, `packs/CHANGELOG.md` (the 1.0.0 entries) | release notes for the move | 08 |
