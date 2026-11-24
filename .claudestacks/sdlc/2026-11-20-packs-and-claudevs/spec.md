---
status: approved
created: 2026-11-23
built-on: 5b1e71d67f7d
---

# Spec: Claudestacks plugins to packs, and the claudevs move

This spec turns five claudestacks plugins (core, sdlc, journal, guideline-rust,
guideline-python) into tool-neutral packs under `packs/`. Rendered by the `claude` extension, they
behave as the plugins do today, apart from the changes listed in §9. It moves `claudevs` and
`claudevs-cli` into this workspace under the same names, with behaviour unchanged and nine files
renamed to the module naming rule. Four supporting changes make that possible:

- airsl passes a module's name to a required chunk;
- the claude extension namespaces skills and agents by pack;
- the neutral event gains an optional `agent_id`;
- the core drops its cross-pack name clash rule.

This spec supersedes `spec-superseded-2026-11-23.md`. It closes the gaps the plan-set review found:
`toplevel` is kept for GATE 3; the doctor's usage refusal is stated; hook tests use a fake
`airsstack.env`; the `agent_id` mapping is tested through the Rust translator tests; and the text
rules cover Lua comments, bare own-pack references and the deny.toml entry.

Proof notation: `Proof:` is a probe in `probes/` (raw output, pasted verbatim), a `file:line` opened
in this design session, or a fetched document at `<file>:<line>`. `P/` is
`../claudestacks/plugins/` (`/Users/hiraq/Projects/airsstack/claudestacks/plugins/`), `C/` is
`../claudestacks/crates/`, `W/` is this worktree's root.

## 1. Premises

| Fact | Proof |
|---|---|
| claudevs and claudevs-cli build and pass all five DoD steps in this workspace unchanged (airsl 0.1.4, serde_json `preserve_order`, workspace lints); `cargo deny check` passes; the fixture check passes 8/8. The only manifest edit is three `[workspace.dependencies]` entries: `claudevs` (path), `regex = "1"`, `walkdir = "2.5"` | `probes/claudevs-results.md` (steps a–h, diff in step 2) |
| `regex` 1.13.1 and `walkdir` 2.5.1 are already in this workspace's `Cargo.lock`; the move adds only `claudevs` 0.1.0 and `claudevs-cli` 0.1.1, and no crate gains a second version | `W/Cargo.lock` (`name = "regex"` `version = "1.13.1"`, `name = "walkdir"` `version = "2.5.1"`); `probes/claudevs-results.md` step h |
| With `claude` 2.1.330 installed, `bad-matcher-plugin` passes Claude's validate stage (`PreToolUseX` is a warning); the fixture script still passes because it checks only its own `FAIL  wiring` line | `probes/claudevs-results.md` step g |
| Both workspaces pin toolchain 1.94.1 | `W/rust-toolchain.toml:8`, `../claudestacks/rust-toolchain.toml` `channel = "1.94.1"` |
| Nine claudevs Rust files have compound names: `src/types/{case_name,hook_event,marketplace_name,plugin_name,plugin_version}.rs`, `src/harness/{hooks_file,t_module}.rs`, `tests/{corpus_fetch,verify_core}.rs` (56 `.rs` files in total) | `find claudevs claudevs-cli -name '*.rs' -not -path '*/fixtures/*' \| grep '_'` in `C/` → those nine; `… \| wc -l` → `56` |
| `types/` and `harness/` re-export every item flat (`pub use case_name::{CaseName, InvalidCaseName}` …) | `C/claudevs/src/types/mod.rs` (`pub use` block), `C/claudevs/src/harness/mod.rs` (`pub use hooks_file::…`, `pub use t_module::TModule`) |
| `HookEvent` (`types/hook_event.rs`) is the set claudevs can simulate, distinct from the documented catalogue in `contract/event.rs` | `C/claudevs/src/types/hook_event.rs:1-6`, `C/claudevs/src/contract/event.rs:1-6` |
| The plugins' Lua suites pass under airsl 0.1.4 with the plugins-test grants: claudestacks 265, sdlc 262, journal 79 (606 in total); the guideline plugins have no test files, and `airsl test` exits 1 on a directory with none | `probes/plugins-lua-results.md` |
| `airsl test` takes one `[PATH]` | `airsl test --help` → `Usage: airsl test [OPTIONS] [PATH]` |
| airsl 0.1.4 resolves every `require` against the entry script's directory, for the whole run | `probes/require-results-2.md`: `module \`lib.x\` not found under \`…/rq/R\`` when `core.hooks.enforce` requires `lib.x` |
| airsl 0.1.4 passes no arguments to a required chunk: `(...)` is nil | `probes/require-results.md`: `enforce.lua:1: attempt to index a nil value`; `~/.cargo/registry/src/*/airsl-0.1.4/src/require_loader.rs:199-203` `lua.load(&source)…​.eval::<Value>()` |
| Lua 5.4's `require` calls the loader with the module name | `probes/lua54-manual.html:8702-8704` "require calls the loader with two arguments: modname and an extra value" |
| Changing that one call to `.call::<Value>(target.as_str())` makes the `(...)` prefix idiom load sibling libs both under the runtime (`core.hooks.enforce`) and under `airsl test` (`enforce`); airsl's whole lib suite still passes (553) | `probes/airsl-modname-results.md` (patched binary named by path; `cargo test -q -p airsl --lib` → `553 passed`) |
| The latest published airsl is 0.1.4; the change is filed as airsstack/airsl#17 | `cargo search airsl --limit 3` → `airsl = "0.1.4"`, `airsl-cli = "0.1.4"`; `gh issue create -R airsstack/airsl …` → `https://github.com/airsstack/airsl/issues/17` |
| Claude Code 2.1.330 accepts pack-prefixed project skills and agents: `/sdlc-status`, `/guideline-python-guidelines` and subagent `core-coder` all ran | `probes/claude-names-results.md` Q1–Q4 |
| Four plugin skill names match Claude Code built-ins: `/design` (bundled skill), `/plan`, `/status`, `/statusline` | `probes/claude-commands-doc.md:79,116,148,149` |
| No built-in command or bundled skill starts with a pack prefix; the built-in subagents are Explore, Plan, General-purpose and the "Other" tab | `grep -nE '\`/(core\|sdlc\|journal\|guideline)' probes/claude-commands-doc.md` → no output, control `grep -n '\`/status' …` → line 148; `…/cli-and-claude-extension/probes/docs/sub-agents.md:35,51,61,71` |
| A project skill replaces a built-in or bundled command of the same name | `W/.claudestacks/sdlc/2026-11-20-cli-and-claude-extension/probes/docs/slash-commands.md:216-217` |
| The skill's command name is its directory name, or the frontmatter `name` when set | same file `:118` |
| Claude Code substitutes `${CLAUDE_PROJECT_DIR}` in a project skill's text with the absolute project path | `probes/claude-projdir-results.md` (`DIR=/private/tmp/…/claude-names`); docs `slash-commands.md:496,500` |
| In the main session a Bash `cd` carries over to later commands; subagent sessions never carry it over | `…/cli-and-claude-extension/probes/docs/tools-reference.md:153-154` |
| Hook input carries `agent_id` (unique per subagent) and `agent_type` (its name) | `…/cli-and-claude-extension/probes/docs/hooks.md:255,764-768`; `…/probes/claude-results.md:194` |
| The plugin enforce hook keys its sentinels by `agent_id`, one per subagent instance | `P/claudestacks/hooks/enforce.lua:137`, `P/claudestacks/hooks/lib/enforce.lua:394-403` |
| The plugin enforce hook finds guideline data by reading `~/.claude/plugins/installed_plugins.json` | `P/claudestacks/hooks/lib/enforce.lua:199-205,240-325`; `probes/hooks-map.md` §3 |
| Claude Code discards SessionEnd hook output | `…/cli-and-claude-extension/probes/docs/hooks.md:3707` |
| On `PreToolUse`, exit 2 blocks the tool call, and `permissionDecision: "deny"` prevents it | same `hooks.md:1036`, `:2015` |
| Inside a subagent, a `deny` on `SubagentHandback` and the plugin's exit 2 behave the same: the call is blocked, the reason comes back as an error result (`PreToolUse:SubagentHandback hook error: PROBE-HANDBACK-DENIED fix the report`), and the subagent retries; nothing is swallowed. The hook's `tool_input` is `{"message":"<report>"}` | `probes/claude-handback-results.md` R1, R2 |
| `CLAUDE_PROJECT_DIR` is not set in the Bash tool's environment (hooks get it) | `probes/claude-handback-results.md` R3 (`PD=`) |
| A `deny` on a main-session `Read` in `-p` mode returns the reason to the model ("PreToolUse:Read hook error: probe says no"); the swallow airsl recorded was seen with `defer`, not `deny` | `…/cli-and-claude-extension/probes/claude-results.md:267-291,541`; `~/.cargo/registry/src/*/airsl-0.1.4/src/modules/hook.rs:13-22` |
| The 14 plugin hook entries, their payload fields, outputs, grants and wrapper exit rules | `probes/hooks-map.md` §1–§2 |
| A pack hook may `run` `{{root core}}/…` from another pack. `{{root}}` in args and grants means the declaring pack. The module loads from `.claude/agentstacks/core/` with no read grant on core. A `{block}` result on `SubagentStop` prints `{"decision":"block",…}` | `probes/crosspack-results.md` Q1, Q3 |
| `render` replaces `{{skill …}}` in a root JSON file | `probes/crosspack-results.md` Q2 |
| The §6 manifests load and apply through the real loader and the shipped claude extension: 52 writes and exactly one skip, `core/hook/3: claude shows no hook output to the model at session end` (run with `">=0.1.4"` because 0.1.5 is unreleased, and two stub skills renamed past the clash rule) | `probes/manifests-results.md:271,275`; the stubs are made by `probes/manifests-probe.sh` |
| The core refuses two packs that ship a skill (or agent) of the same name | `probes/manifests-results.md` § "The cross-pack name clash rule (today)" (`agentstacks.toml: skill "setup" is shipped by both c and d`); `W/crates/agentstacks/src/types/set/clash.rs`, `check.rs:129` |
| Two proposals of one install path are refused at the later call, naming the earlier site | `probes/manifests-results.md` § "Two proposals of one install path" (`claude/adapter.lua:135: copy: path ".claude/skills/x/SKILL.md" is already proposed at adapter.lua:41`); `W/crates/agentstacks/src/types/op/proposals.rs:405-424` |
| The exact airsl refusals: `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4` and `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)` | `W/crates/agentstacks/src/types/op/airsl.rs:110,132` |
| The plugin suite's gate runs three things: `airsl check plugins` (compiles every Lua file, including hook entries no test loads), `airsl test` with grants, and a fingerprint gate of 8 cases through `fingerprint.sh` (repeated with a sha256sum-only `PATH`) | `../claudestacks/Makefile.toml:124-143` (`plugins-check`), `:155-230` (`plugins-test`; the fingerprint gate is `:176-228`) |
| The plugin style hook always injects at session start; only the per-prompt (`--turn`) run reads `style_reinject` | `P/claudestacks/scripts/style.lua:3-5,19-24`; `P/claudestacks/.claude-plugin/plugin.json` userConfig ("When off, they are still sent at every session start, /clear and compaction") |
| Enforce's registry parts: GATE 1 binds a registry record to the project (`select_record`), the doctor prints a repo↔cache parity section, and the trace has registry STOP lines | `P/claudestacks/hooks/lib/enforce.lua:275-292,503-532,574-576`; `P/claudestacks/hooks/enforce.lua:47-112` |
| `enforce_test.lua` locates its suite through `claudestacks/.claude-plugin/plugin.json`; `discuss_skill_test.lua:8` and `style_test.lua:69` read `plugins/claudestacks/…` paths relative to the run directory | `P/claudestacks/hooks/enforce_test.lua:84-100,521-540`; `P/claudestacks/scripts/discuss_skill_test.lua:8`; `P/claudestacks/scripts/style_test.lua:69` |
| An enforce sentinel is a file that lives 24 h, keyed by session, agent, stack and phase | `P/claudestacks/hooks/lib/enforce.lua:28,394-403` |
| The carried `.sh` files hold no plugin-root or namespaced text | `grep -n 'CLAUDE_PLUGIN_ROOT\|claudestacks[a-z-]*:[a-z]' claudestacks-sdlc/scripts/{status,fingerprint}.sh claudestacks/scripts/install-airsl.sh` in `P/` → no output |
| The claude extension maps `fast/balanced/deep` to `haiku/sonnet/opus`, `shell` to `Bash`, `invocation: user` to `disable-model-invocation: true` | `W/extensions/claude/lib/frontmatter.lua:15,19,93-94` |
| The claude extension's `tmpdir` var is `$TMPDIR`; `sh` expands it to an empty string when TMPDIR is unset, while `${TMPDIR:-/tmp}` gives `/tmp` | `W/extensions/claude/lib/values.lua` `VARS.tmpdir`; `probes/tmpdir-results.md` |
| The agent-stop matcher is the hook's `agents` joined with `\|` | `W/extensions/claude/lib/hooks.lua:40-45,63-64`; `probes/manifests-results.md` (`"matcher": "task-briefer\|chain-reader\|artifact-reviewer"`) |
| The official pack list is `OFFICIAL_PACKS` and holds `cmux` | `W/crates/agentstacks/src/types/shorthand.rs:39-46` |
| The CLI end-to-end tests copy a fixture project and `extensions/claude` into a temp dir and skip without airsl unless `AGENTSTACKS_REQUIRE_AIRSL` is set | `W/crates/agentstacks-cli/tests/commands.rs:3-6,17-44,68-73` |
| `toplevel` reads `.git` (it runs no git), and GATE 3's `path_for_matching` calls it. `common_dir` has one caller, `project_key` (GATE 1) | `P/claudestacks/hooks/lib/enforce.lua:103-104,181-185`; `grep -n common_dir P/claudestacks/hooks/lib/enforce.lua` → `:45` (comment), `:118` (definition), `:157` (in `project_key`) |
| `airsstack.fs.list` lists a directory and is gated by the read grant; the plugin enforce library already uses it, and its suite passes (row "plugins' Lua suites") | `~/.cargo/registry/src/*/airsl-0.1.4/src/modules/fs.rs:285-302`; `P/claudestacks/hooks/lib/enforce.lua:430-480` |
| A `require` module name may hold hyphens (`team-lint.hooks.my-file` loads through `runtime.lua`) | `W/.claudestacks/sdlc/2026-11-20-core-plan-and-apply/probes/hookreq-results.md:7-9` |
| Under the `lua` task's test grants (no `--allow-env`), `airsstack.env.get` raises `env.get denied: … is not granted`; a test can replace `airsstack.env` with a fake table, and a required module that reads `airsstack.env` when called sees the fake | `probes/envfake-results.md` |
| The plugin enforce entry binds `local env = airsstack.env` when it loads; `vault.root()` reads `airsstack.env.get` when called | `P/claudestacks/hooks/enforce.lua:23-35`; `P/claudestacks-journal/scripts/lib/vault.lua:125-128` |
| The journal session-start emits nothing when the orientation card is empty. On a fresh vault it rebuilds the index, which writes `.index/summaries.tsv` (empty when there are no notes), and the card is empty because no session or note matches the project | `P/claudestacks-journal/scripts/session-start.lua:31-34,57-72`; `P/claudestacks-journal/scripts/lib/index.lua:300-330`; `P/claudestacks-journal/scripts/lib/orientation.lua:84-90` |
| A `{{skill}}` or `{{agent}}` token in a `SKILL.md` or agent frontmatter value renders, as in the body | `probes/fmtoken-results.md` (`description: "Recall notes. Use when the user says \"/recall X\"."`) |
| The plugin `enforce.lua --explain` with no path, or an empty one, writes `usage: enforce.lua --explain <path>` to stderr, nothing to stdout, and exits 0 | `probes/doctor-usage-results.md` |
| The claude translator is shipped as text (`M.text()`); its mapping is tested in Rust by running fixture hooks through the real runtime. `payloads/hooks/echo.lua` prints a fixed `FIELDS` list per event, and only `input/subagent.json` carries an `agent_id` (`"a1"`) | `W/extensions/claude/lib/translate.lua:6,113`, `lib/translate_test.lua:13-30`; `W/crates/agentstacks/tests/claude.rs:530-550`; `W/crates/agentstacks/tests/fixtures/claude/payloads/hooks/echo.lua:3`; `grep -o '"agent_id":"[^"]*"' …/payloads/input/*.json` → `subagent.json:"agent_id":"a1"` only |
| Plugin Lua files that are carried and name a plugin item or variable: comments at `scripts/discuss.lua:1`, `scripts/lib/discuss.lua:1,19`, `scripts/lib/style.lua:34`, `scripts/style.lua:8`; code at `scripts/style.lua:20` and `lib/status.lua:825` | `probes/selfref-results.md`, first grep |
| Plugin Markdown that names a plugin skill or agent without a namespace: `/snapshot-save`, `/snapshot-load`, `/journal-<x>` (second grep); `subagent_type: journal-<x>`, the `journal-<x>` subagent, and `orchestrate/SKILL.md:18` "Namespaced as" (third grep); backticked bare names such as `coder`, `explorer`, `reviewer`, `task-briefer`, `orchestrate`, `snapshot-save` in core and sdlc skills and references, including sdlc `execute/SKILL.md:61` `coder`, which is core's agent (fourth grep). One backticked hit is a label, not a reference: `snapshot-save/SKILL.md:45` "the `discuss` archive" | `probes/selfref-results.md`, second to fourth grep |
| `deny.toml` exempts airsl's own tree from the duplicate-version rule: `skip-tree = [{ crate = "airsl@0.1.4", reason = "airsl 0.1.4 pins sha2 0.10 and toml 0.8; …" }]` | `W/deny.toml:69-81` |

## 2. claudevs move

- **Import.** `C/claudevs/` and `C/claudevs-cli/` are copied whole into `crates/claudevs/` and
  `crates/claudevs-cli/`, together with `../claudestacks/scripts/corpus-fetch.sh` →
  `scripts/corpus-fetch.sh` (`tests/corpus_fetch.rs` drives it, `probes/claudevs-results.md` step 1).
  This is one commit with no claudestacks history, and nothing refers to claudestacks commits.
  README, CHANGELOG, LICENSE, `docs/`, `examples/`, `tests/fixtures/` and `tests/corpus/` move
  unchanged. Crate versions stay `0.1.0` (claudevs) and `0.1.1` (claudevs-cli).
- **Workspace.** `[workspace.dependencies]` gains `claudevs = { path = "crates/claudevs", version =
  "0.1.0" }`, `regex = "1"` and `walkdir = "2.5"`. The members glob `crates/*` already covers both
  crates (`W/Cargo.toml:3`). `repository` is inherited and becomes agentstacks'. `deny.toml` does
  not change (§1).
- **Renames** (the approved architecture, §10). `git mv` only. Each moved file keeps its content
  except module paths. `types/mod.rs` and `harness/mod.rs` keep their re-exports, so the public API
  is unchanged.

  | From | To |
  |---|---|
  | `src/types/case_name.rs` | `src/types/name/case.rs` |
  | `src/types/marketplace_name.rs` | `src/types/name/marketplace.rs` |
  | `src/types/plugin_name.rs` | `src/types/name/plugin.rs` |
  | (new) | `src/types/name/mod.rs`, which only declares and re-exports, with the four-question `//!` block |
  | `src/types/plugin_version.rs` | `src/types/version.rs` |
  | `src/types/hook_event.rs` | `src/types/event.rs` |
  | `src/harness/t_module.rs` | `src/harness/module.rs` |
  | `src/harness/hooks_file.rs` | `src/harness/hooks.rs` |
  | `tests/corpus_fetch.rs` | `tests/fetch.rs` |
  | `tests/verify_core.rs` | `tests/engine.rs` |

  Doc comments and Makefile comments that name an old path are updated with it. `types/mod.rs`'s
  prose ("this file", "Four of the five types this module exports") is reworded to the new layout.
- **Gate tasks.** `Makefile.toml` gains three tasks, copied from `../claudestacks/Makefile.toml`:
  - `claudevs-check`: the script verbatim (lines 232–391);
  - `corpus-fetch`: lines 393–417, needs the network, not in `dod`;
  - `corpus-check`: lines 419–458, the `#[ignore]` corpus lane, not in `dod`.

  The `claudevs-check` comment that says `bad-matcher-plugin` fails Claude's validate step is
  corrected: it now passes validate and fails `wiring` (§1). No task is added to `dod`.
- **CI.** `.github/workflows/ci.yml` gains a job `claudevs-check` (ubuntu-latest): checkout, pinned
  toolchain, rust-cache, cargo-make, then `cargo make claudevs-check`. There is no `claude` binary
  on the runner, so the validate stage reports skipped, as in claudestacks
  (`../claudestacks/.github/workflows/ci.yml:64-91`).
- **Not moved here:** the claudevs-cli release and smoke workflows (`release-and-migration`).

## 3. airsl 0.1.5 (precondition)

- airsl's `require` passes the module name to the required chunk as its first argument, as Lua
  5.4 does (§1). It is the one-line change in `probes/airsl-modname-results.md`, filed as
  airsstack/airsl#17. airsl and airsl-cli are released together as 0.1.5. That work happens in the
  airsl repository; it is not built by this chain.
- After that release, this chain:
  - sets `airsl = "0.1.5"` in `[workspace.dependencies]`;
  - bumps the CI binary pin to 0.1.5 with its new SHA-256. The CI comment requires the version, the
    hash and the library to move together (`.github/workflows/ci.yml`, step "Install the airsl
    binary");
  - sets `requires.airsl = ">=0.1.5"` on every pack holding `.lua` files (core, sdlc, journal; §6);
  - moves the `deny.toml` `skip-tree` entry to `crate = "airsl@0.1.5"`, with a reason naming the
    `sha2` and `toml` versions 0.1.5 pins (`cargo tree -p airsl@0.1.5 --depth 1 -e normal`). If
    0.1.5 pins no older `sha2` or `toml` than agentstacks, the entry is removed, and the `[bans]`
    comment that describes it ("The one exemption is the `skip-tree` entry for `airsl` …") goes
    with it. Either way `cargo make deny` is green (§1 deny.toml row).
- Work order: §2, §4 and §5 do not need airsl 0.1.5. §6, §7, the Lua step and the end-to-end test
  of §11 do.
- claudevs is proven against airsl 0.1.4 (§1). After the bump, `cargo make dod` re-proves it on
  0.1.5. If it does not, the bump plan fixes claudevs before it lands.
- **The idiom.** Every Lua file that a hook loads, directly or through another module, starts its
  sibling requires from its own name:

  ```lua
  local base = (...):match("^(.-)[^.]*$")
  local lib = require(base .. "lib.x")
  ```

  Under the runtime `...` is `core.hooks.enforce` and `base` is `core.hooks.`. Under `airsl test`
  `...` is `enforce` and `base` is empty. Proof: `probes/airsl-modname-results.md` prints
  `lib=ok via core.hooks.lib.x base=[core.hooks.]` and `1 passed`. A file loaded only by scripts
  that skills run (never by a hook) keeps its plain requires.

## 4. claude extension

All of these changes are in `extensions/claude/`. The core stays tool-agnostic.

| Change | Rule |
|---|---|
| Skill names | `values.skills["<p>/<s>"] = { invoke = "/<p>-<s>", dir = ".claude/skills/<p>-<s>" }` |
| Agent names | `values.agents["<p>/<a>"] = "<p>-<a>"`; the agent file is `.claude/agents/<p>-<a>.md` |
| Frontmatter `name` | The written `SKILL.md` and agent frontmatter `name` is the rendered name (`sdlc-status`, `core-coder`), because the frontmatter name sets the command (§1) |
| Agent-stop matcher | The pack's `agents`, each rendered as `<p>-<a>`, joined with `\|` (`sdlc-task-briefer\|sdlc-chain-reader\|sdlc-artifact-reviewer`) |
| `tmpdir` var | `${TMPDIR:-/tmp}` (was `$TMPDIR`; §1) |
| Translator | `M.event` sets `e.agent_id = p.agent_id` when it is a non-empty string, on every event, beside `agent` |
| Native files | unchanged: `native/claude/<rel>` → `.claude/<rel>`. A native skill carries its own final name (`core-statusline`) |

- The extension's Lua tests (`lib/*_test.lua`) are updated for each row, except the translator
  row. The translator is shipped as text, and its mapping is tested in Rust (§1). For `agent_id`:
  - `payloads/hooks/echo.lua` adds `agent_id` to the end of `FIELDS`, so every `expected/echo/*.err`
    line gains ` agent_id=<value>`: `a1` for `subagent`, `nil` for the rest;
  - `input/noagent.json` gains `"agent_id":""`, and its line shows `agent_id=nil` (an empty id is
    absent). The test `empty_agent_type_maps_to_nil_agent` becomes
    `empty_agent_type_and_id_map_to_nil`;
  - `tests/fixtures/claude/full/expected/.claude/agentstacks/translate.lua` is the new translator
    text byte for byte.
- **Collisions.** The prefixed forms of pack `a-b` skill `c` and pack `a` skill `b-c` are the same
  name. The second proposal of `.claude/skills/a-b-c/SKILL.md` is refused by the existing rule:
  `claude/adapter.lua:<n>: write: path ".claude/skills/a-b-c/SKILL.md" is already proposed at adapter.lua:<m>`.
  §1 shows the same rule for a `copy`. No new rule.
- **Length.** There is no length rule. The docs give none, and `/guideline-python-guidelines`
  (27 characters) runs (§1).
- **Docs.** `docs/hooks.md` §5 adds the `agent_id` row: "`agent_id` | optional; the tool's id for
  the agent instance, `nil` when the tool has none". `docs/extensions.md` §6 (the `values` example)
  and `docs/authoring.md` (the worked example) show the prefixed names.

## 5. Core: name clash rule removed, official packs

- `types/set/clash.rs` and its call (`types/set/check.rs:129`) are removed, along with:
  - the `set/error.rs` variants and their tests;
  - the doc lines that name the rule (`types/set/check.rs:13`, `types/set/mod.rs:15`,
    `types/pack/load.rs:44`);
  - in `tests/set.rs`, the cases `skill-clash`, `skill-clash-three` and `agent-clash` and their
    fixtures. A new test loads the `skill-clash` and `agent-clash` fixtures, kept under the names
    `shared-skill` and `shared-agent`, and asserts no diagnostic;
  - in `check_skips_follow_on_errors_from_failed_packs` (`tests/set.rs:195-213`): the clash line
    leaves the expected list, so the test asserts exactly the one `core: api 1 not supported` line.
    It still shows that no follow-on error from the failed pack is reported.

  Two packs may now ship a skill or agent of the same name. Each extension names the tool's items. A tool whose
  extension gives two items the same path hits "already proposed" (§4). This drops a refusal; it
  adds none.
- `OFFICIAL_PACKS` becomes `["core", "sdlc", "journal", "guideline-rust", "guideline-python"]`
  (`[&str; 5]`). Its test at `shorthand.rs:231` and every doc list that names `cmux` are updated:
  `docs/cli.md:59`, `README.md:102`, `CLAUDE.md:105`.

## 6. Packs

### 6.1 Layout and mapping

```text
packs/<pack>/
├─ pack.toml                    §6.2 (never installed)
├─ README.md                    the plugin's README, adapted; excluded from install
├─ skills/<name>/SKILL.md       + references/ etc., moved with the skill
├─ agents/<name>.md
├─ native/claude/…              core only (§6.2)
└─ hooks/ scripts/ references/ enforcement.json …   root files → {{root}}
```

| Plugin source | Pack destination |
|---|---|
| `P/<plugin>/skills/<s>/**` | `packs/<pack>/skills/<s'>/**`, where `<s'>` is the name with any pack prefix dropped (table below) |
| `P/<plugin>/agents/<a>.md` | `packs/<pack>/agents/<a'>.md` |
| `P/<plugin>/{hooks,scripts,references}/**`, `enforcement.json` | same relative path under `packs/<pack>/`, except the dropped files below |
| `P/claudestacks/skills/statusline/` | `packs/core/native/claude/skills/core-statusline/` |
| `.claude-plugin/`, `LICENSE`, `hooks/hooks.json`, every hook wrapper `.sh` (`hooks/{comm-check,enforce,handoff-check,preflight,rearm,style}.sh`, `sdlc scripts/attention.sh`, `journal scripts/session-start.sh`) | not carried. The hooks are declared in `pack.toml` (§6.2) |
| `P/claudestacks-sdlc/scripts/{status,fingerprint}.sh`, `P/claudestacks/scripts/install-airsl.sh` | carried unchanged (skills run them; they are not hooks) |

Renamed items:

| Pack | Plugin name → pack name | Renders (claude) |
|---|---|---|
| journal | skills `journal-{capture,helped,link,note,recall,review,setup}` → `{capture,helped,link,note,recall,review,setup}` | `/journal-capture` … |
| journal | agents `journal-{capture,curator,note,recall}` → `{capture,curator,note,recall}` | `journal-capture` … |
| guideline-rust | skill `rust-guidelines` → `guidelines` | `/guideline-rust-guidelines` |
| guideline-python | skill `python-guidelines` → `guidelines` | `/guideline-python-guidelines` |
| core | (none renamed) | `/core-<s>`, `core-coder` … |
| sdlc | (none renamed) | `/sdlc-<s>`, `sdlc-chain-reader` … |

### 6.2 Manifests

These texts are byte for byte the probed ones (`probes/manifests-results.md`). The probe ran them
with `">=0.1.4"` in place of `">=0.1.5"`, because 0.1.5 is unreleased.

`packs/core/pack.toml`:

```toml
exclude = ["README.md", "**/*_test.lua"]

[pack]
name        = "core"
version     = "0.1.0"
api         = 0
description = "Core workflow: coder, explorer and reviewer agents, orchestration, handoff and communication checks, guideline enforcement"

[requires]
airsl = ">=0.1.5"

[options.style_reinject]
type        = "boolean"
default     = true
description = "Re-inject the reply rules on every prompt; they are always sent at session start, /clear and compaction"

[options.archive_keep]
type        = "integer"
default     = 20
description = "Discussions kept per project besides the current one"

[[hooks]]
kind  = "message"
event = "session.start"
match = ["startup", "resume", "clear"]
text  = "Reminder: run {{skill snapshot-load}} for the current git branch to rehydrate relevant project memory before starting work."

[[hooks]]
kind  = "lua"
event = "session.start"
match = ["compact"]
run   = "hooks/rearm.lua"
[hooks.grants]
env   = ["TMPDIR"]
read  = ["{{var tmpdir}}"]
write = ["{{var tmpdir}}"]

[[hooks]]
kind  = "lua"
event = "session.start"
run   = "scripts/style.lua"
args  = ["{{skill_dir discuss}}/references/protocol.md"]
[hooks.grants]
read = ["{{skill_dir discuss}}"]

[[hooks]]
kind  = "message"
event = "session.end"
text  = "Reminder: run {{skill snapshot-save}} to flush durable session learnings into project memory (durability gate applies — thin sessions write nothing)."

[[hooks]]
kind  = "lua"
event = "prompt.submit"
run   = "scripts/style.lua"
args  = ["{{skill_dir discuss}}/references/protocol.md", "{{option style_reinject}}"]
[hooks.grants]
read = ["{{skill_dir discuss}}"]

[[hooks]]
kind  = "lua"
event = "tool.before"
match = ["read", "edit", "write"]
run   = "hooks/enforce.lua"
args  = ["{{root}}"]
[hooks.grants]
env   = ["HOME", "TMPDIR", "AIRSSTACK_HOME"]
read  = ["/"]
write = ["{{var tmpdir}}"]

[[hooks]]
kind      = "lua"
event     = "tool.after"
match     = ["write"]
run       = "scripts/handoff_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]

[[hooks]]
kind      = "lua"
event     = "tool.after"
match     = ["write"]
run       = "scripts/comm_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]

[[hooks]]
kind      = "lua"
event     = "agent.stop"
agents    = ["coder", "explorer", "reviewer"]
run       = "scripts/handoff_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]

[[hooks]]
kind      = "lua"
event     = "agent.stop"
run       = "scripts/comm_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]
```

`packs/core/native/claude/hooks.toml`:

```toml
[[hooks]]
kind      = "lua"
event     = "PreToolUse"
match     = ["^SubagentHandback$"]
run       = "scripts/handoff_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]

[[hooks]]
kind      = "lua"
event     = "PreToolUse"
match     = ["^SubagentHandback$"]
run       = "scripts/comm_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]
```

`packs/sdlc/pack.toml`:

```toml
exclude = ["README.md", "**/*_test.lua"]

[pack]
name        = "sdlc"
version     = "0.1.0"
api         = 0
description = "Intent, spec, plan and execute chain with a status board, roadmaps and ADRs"

[requires]
packs = ["core"]
airsl = ">=0.1.5"

[[hooks]]
kind  = "lua"
event = "session.start"
run   = "scripts/attention.lua"
args  = ["{{skill status}}"]
[hooks.grants]
read = ["/"]

[[hooks]]
kind      = "lua"
event     = "agent.stop"
agents    = ["task-briefer", "chain-reader", "artifact-reviewer"]
run       = "{{root core}}/scripts/handoff_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]
```

`packs/journal/pack.toml`:

```toml
exclude = ["README.md", "**/*_test.lua"]

[pack]
name        = "journal"
version     = "0.1.0"
api         = 0
description = "Session journal: capture, atomic notes, recall and review over a local vault"

[requires]
packs = ["core"]
airsl = ">=0.1.5"

[[hooks]]
kind  = "lua"
event = "session.start"
match = ["startup", "resume", "clear"]
run   = "scripts/session-start.lua"
[hooks.grants]
env   = ["AIRSSTACK_HOME", "HOME"]
read  = ["{{var airsstack_home}}", "{{var project_dir}}"]
write = ["{{var airsstack_home}}"]

[[hooks]]
kind      = "lua"
event     = "agent.stop"
agents    = ["curator"]
run       = "{{root core}}/scripts/handoff_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]
```

`packs/guideline-rust/pack.toml` (`guideline-python` is the same, with `name =
"guideline-python"` and `description = "Python engineering rules and the Definition of Done gate"`):

```toml
render  = ["enforcement.json"]
exclude = ["README.md"]

[pack]
name        = "guideline-rust"
version     = "0.1.0"
api         = 0
description = "Rust engineering rules and the Definition of Done gate"
```

`enforcement.json` keeps its keys `stack`, `detect`, `match`, `skill` and `phase`
(`probes/hooks-map.md` §3). `skill` becomes `"{{skill guidelines}}"`, which renders to
`/guideline-rust-guidelines` (render behaviour: §1).

### 6.3 Hooks: plugin → pack

| Plugin entry (`probes/hooks-map.md` §1) | Pack hook (§6.2) | Code change |
|---|---|---|
| H3 preflight echo | core `message` `session.start` `[startup,resume,clear]` | The echo text becomes `text`, with `{{skill snapshot-load}}`. The airsl-missing STATUS/FIX text is dropped: `requires.airsl` makes `plan` and `doctor` check airsl (§9) |
| H4 rearm | core `hooks/rearm.lua` | module; `event.session_id` |
| H5, H7 style | core `scripts/style.lua` on `session.start` (args: the protocol path) and `prompt.submit` (args: the protocol path, then the option) | module; protocol path = `event.args[1]`. On `session.start` it always returns `{ context = rules }`. On `prompt.submit` it returns them unless `event.args[2]` is `"false"` or `"0"` (as `lib/style.lua:37-43`). No env read. `--turn` is dropped, because `event.event` names the event. Same behaviour as the plugin (§1) |
| H6 SessionEnd echo | core `message` `session.end` | none. The claude extension skips it with `claude shows no hook output to the model at session end`, the same effect as today (§1) |
| H8 enforce | core `hooks/enforce.lua`, `args = ["{{root}}"]` | module; reads `event.path`, `event.cwd`, `event.session_id`, `event.agent_id` (sentinel key; `"main"` when nil, as today); discovery and trace per §7; returns `{ context = pointers }` |
| (enforce-doctor `--explain`) | new script `core/hooks/doctor.lua`, run by the skill (§7) | the `explain` function moves out of `enforce.lua` into `doctor.lua`, a top-level script that takes the pack root and the file path |
| H9, H10 SubagentHandback | core native `PreToolUse` `^SubagentHandback$` | module; native event: reads `event.payload.tool_input.message` and `event.payload.cwd`; a violation returns `{ block = reason }` (deny with reason, §1) |
| H11, H12 PostToolUse Write | core `tool.after` `[write]` | module; reads `event.path`, `event.cwd`; returns `{ block = reason }` |
| H13 SubagentStop (named agents) | `agent.stop` in core `[coder,explorer,reviewer]`, sdlc `[task-briefer,chain-reader,artifact-reviewer]`, journal `[curator]`, all running core's `scripts/handoff_check_hook.lua` | module; reads `event.last_message`, `event.cwd`; returns `{ block = reason }` |
| H14 SubagentStop (any agent) | core `agent.stop`, no `agents` | as H13, with `comm_check_hook.lua` |
| H2 sdlc attention | sdlc `scripts/attention.lua`, `args = ["{{skill status}}"]` | module; `event.cwd`; returns `{ context = text }`. The board name in `lib/status.lua:825` comes from `args[1]` |
| H1 journal session-start | journal `scripts/session-start.lua` | module; returns `{ context = card }` |

**Module shape.** Each hook file returns `function(event) … end`. It returns `nil`,
`{ context = … }` or `{ block = … }` (`docs/hooks.md` §5), and never calls
`airsstack.hook.context`, `airsstack.hook.emit`, `airsstack.hook.payload` or `airsstack.stdio`. A
library that today switches on `payload.hook_event_name` switches on `event.event` (`tool.after`,
`agent.stop`), or on `event.payload ~= nil` for the native case. This includes
`lib/handoff_report.lua` `file_from_payload` and `lib/comm_report.lua` `hook_output`. The §3 idiom
applies to every file a hook loads.

**Environment.** A hook module and every library it calls read `airsstack.env` when called, never
into a module-level local. The plugin enforce entry's `local env = airsstack.env` (§1) moves inside
the function that builds its settings, and so does `scripts/lib/handoff_report.lua:11`, which both
check hooks load. A test swaps `airsstack.env` for a fake table and restores
it (§1, `probes/envfake-results.md`), so no test needs an `--allow-env` grant.

### 6.4 Text conversion

| Plugin text | Pack text |
|---|---|
| `<plugin>:<skill>` (e.g. `claudestacks-sdlc:status`, `/claudestacks:discuss`) | `{{skill <pack>/<skill'>}}` (own pack: `{{skill <skill'>}}`); the leading `/` goes, because the token renders it |
| `<plugin>:<agent>` (e.g. `claudestacks:coder`) | `{{agent <pack>/<agent'>}}` |
| The three example references to `claudestacks-guideline-rust:rust-guidelines` (`P/claudestacks-sdlc/skills/design/SKILL.md:128`, `plan/SKILL.md:68`, `P/claudestacks/skills/orchestrate/SKILL.md:61`) | prose: "the stack's guideline skill (for Rust, the guideline-rust pack's skill)". No token, because these packs do not require the guideline packs |
| `${CLAUDE_PLUGIN_ROOT}/skills/<s>/…`, `$CLAUDE_PLUGIN_ROOT/skills/<s>/…` | in a `SKILL.md` (neutral or native): `{{var project_dir}}/{{skill_dir <s'>}}/…`; in any other file: `{{skill_dir <pack>/<s'>}}/…` |
| `${CLAUDE_PLUGIN_ROOT}/<other>`, `$CLAUDE_PLUGIN_ROOT/<other>` | in a `SKILL.md`: `{{var project_dir}}/{{root}}/<other>`; in any other file (agents, `references/*.md`): `{{root}}/<other>`. Skills run in the main session, whose working directory can drift, and Claude substitutes `${CLAUDE_PROJECT_DIR}` in skill text while the Bash tool has no such variable (§1), so only skills can get the absolute form. Subagents always start at the project directory (§1) |
| a bare `${CLAUDE_PLUGIN_ROOT}` (no path after it), e.g. a grant `--allow-read "${CLAUDE_PLUGIN_ROOT}"` or `case "${CLAUDE_PLUGIN_ROOT}" in …` (`statusline/SKILL.md:41,81,91,115,126`) | `{{var project_dir}}/{{root}}` in a `SKILL.md`; `{{root}}` elsewhere |
| prose naming `${CLAUDE_PLUGIN_ROOT}` as a variable the worktree guard cannot resolve (`enforce-doctor/SKILL.md:21-23`, `journal-{capture,link,note,recall,review}/SKILL.md`, `context-handoff/references/protocol.md:145-151`) | that variable is dropped from the sentence or list, including its `echo "$CLAUDE_PLUGIN_ROOT"` line; the advice for the other variables stays |
| `${CLAUDE_SESSION_ID}` | `{{var session_id}}` |
| `$ARGUMENTS` | `{{var arguments}}` |
| `${user_config.archive_keep}` | `{{option archive_keep}}` |
| A skill or agent of this pack or a required pack, named in Markdown (body or frontmatter) without a namespace: as a command (`/snapshot-load`, `/journal-recall`), a spawn target (`subagent_type: journal-capture`), a backticked bare name (`` `coder` ``, `` `task-briefer` ``, `` `orchestrate` ``), or a hyphenated journal item name in prose (the isolated journal-capture subagent). The inventory is `probes/selfref-results.md` greps 2–4 (§1) | `{{skill <s'>}}` / `{{agent <a'>}}` for its own pack, `{{skill <pack>/<s'>}}` / `{{agent <pack>/<a'>}}` for a required pack (sdlc `execute/SKILL.md:61` `coder` → `{{agent core/coder}}`). Backticks around a name stay around the token. Plain role words without backticks ("the coder", "the reviewer") are prose and stay. Labels stay: headings (`# journal-capture`), file names (`journal-curator-review.md`) and "the `discuss` archive" (`snapshot-save/SKILL.md:45`) |
| `orchestrate/SKILL.md:18` "Namespaced as `claudestacks:coder`, `claudestacks:reviewer`, etc." | "Named {{agent coder}}, {{agent reviewer}}, and so on." The rest of the line stays |
| A plugin item or variable named in a comment of a carried `.lua` or `.sh` file (not rendered; §1 row "Plugin Lua files") | plain prose naming the pack item: `/claudestacks:discuss` → "the core pack's discuss skill"; `${user_config.archive_keep}` → "the `archive_keep` option"; `CLAUDE_PLUGIN_OPTION_STYLE_REINJECT` → "the `style_reinject` option (args[2])". A usage comment that shows a top-level `airsl run` command line for a file that becomes a hook module (`scripts/style.lua:5-9`, and the same block in each converted entry) is dropped: a module has no command line. Code lines change by §6.3 (`style.lua:20`, `status.lua:825`) |
| A literal `{{` (none today; §1 of the formats spec) | `\{{` |
| `[[wiki links]]` (journal) | unchanged |

Frontmatter:

- `model` maps `haiku`/`sonnet`/`opus` → `fast`/`balanced`/`deep`.
- `tools` maps `Read Edit Write Grep Glob Bash Skill` → `read edit write grep glob shell skill`.
- `effort` is unchanged.
- `disable-model-invocation: true` → `invocation: user`.
- `argument-hint` → `argument_hint`.
- `name` is the pack name (`capture`), never the rendered one.
- The native `core-statusline/SKILL.md` keeps Claude's own frontmatter, `name: core-statusline`.

Each plan converts its files by this table and greps the result. After conversion:

- no pack file holds `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PLUGIN_OPTION`, `user_config` or a namespaced
  reference:
  `grep -rnE 'CLAUDE_PLUGIN_(ROOT|OPTION)|user_config|claudestacks(-[a-z]+)*:[a-z]' packs/ --exclude=README.md --exclude='*_test.lua'`
  gives no output;
- no pack Markdown names a pack item without a token:
  ``grep -rnE '(^|[ `"(])/(snapshot-(load|save)|journal-[a-z]+)\b|subagent_type: [a-z]|Namespaced as|`(coder|explorer|reviewer|task-briefer|chain-reader|artifact-reviewer|journal-[a-z]+|context-handoff|enforce-doctor|orchestrate|process-guidelines|snapshot-load|snapshot-save|statusline)`' packs/ --include='*.md' --exclude=README.md``
  gives no output;
- no journal Markdown names a journal item by its plugin name outside a label:
  `grep -rnE 'journal-(capture|curator|note|recall|helped|link|review|setup)' packs/journal --include='*.md' --exclude=README.md | grep -vE ':[0-9]+:# |journal-curator-review\.md'`
  gives no output.

Test files (`*_test.lua` and fixtures) are exempt. So are `README.md` files, which describe the old
plugin names on purpose: each README is the plugin's, with its install block replaced by the
`agentstacks init --tool claude --pack …` line for the pack and the packs it requires.

Product names are not namespaced references and stay as they are: "claudestacks", "the
claudestacks-journal vault", the sentinel prefix `claudestacks-enforce-`
(`hooks/lib/enforce.lua:25`), the attention label `claudestacks-sdlc: N open, …`
(`lib/status.lua:825`, where only the board name after "full board:" comes from `args[1]`), the
handoff file names `claudestacks-sdlc-<chain>-spec-01.md`, and the snapshot key example. Renaming
the product is not this chain's work (§14).

### 6.5 Lua tests

- Every `*_test.lua` moves with its code (20 files: 10 core, 4 sdlc, 6 journal;
  `probes/plugins-lua-results.md`). Its assertions change only where §6.3, §6.4 or §7 changes the
  code under test, or where a test anchors on the plugin layout:

  | Test | Change |
  |---|---|
  | `core/hooks/enforce_test.lua` | Its registry cases become sibling-pack cases: fixtures are directories holding `enforcement.json` under a temp packs root, asserting the §7 trace lines. GATE 1 and parity cases are removed with the code. `plugins_root()` (`:84-100`) anchors on `packs/core/pack.toml` instead of `claudestacks/.claude-plugin/plugin.json`. Its sweep "no driver emits `permissionDecision`" (`:521-540`) keeps running over `packs/`. The `skill` string at `:18` becomes the rendered form `/guideline-rust-guidelines` |
  | `core/scripts/discuss_skill_test.lua:8` | path becomes `packs/core/skills/discuss/SKILL.md`. It asserts that the archive-keep value is single-quoted around `{{option archive_keep}}`. By §6.4, `:15-21` asserts `invocation: user` and `:33-38` asserts `{{var session_id}}` and `"{{var project_dir}}/{{root}}/scripts/discuss.lua"` |
  | `core/scripts/style_test.lua:69` | path becomes `packs/core/skills/discuss/references/protocol.md`; the env case becomes the `args[2]` case of §6.3 |
  | `sdlc/scripts/status_test.lua` (the six status-line strings) | the board name comes from the argument (§6.3 H2) |
  | all others | unchanged, except the §3 idiom where the module under test now uses it |

- **New unit tests.** Each converted hook entry module gets cases in the `*_test.lua` beside it (a
  new file, or the existing one where it already exists: `hooks/enforce_test.lua`,
  `scripts/style_test.lua`). The test calls the returned function with a neutral event and asserts
  the result. A module that reads the environment runs under a fake `airsstack.env` (§6.3
  "Environment"):
  - `style`: session start always returns context; prompt submit with `"false"` returns nil;
  - `handoff_check_hook` and `comm_check_hook`: a `tool.after`, an `agent.stop` and a native event,
    each with a bad report, give `{ block = … }`;
  - `enforce`: with a fake env whose `TMPDIR` is a fresh temp directory, a temp packs root holding
    `core/` and `guideline-rust/enforcement.json`, `args = {"<packs root>/core"}` and a project with
    `Cargo.toml`, a `tool.before` event for `src/main.rs` returns `{ context = … }` holding the
    pack's `skill`;
  - `rearm`: with a fake env, it clears that session's sentinels and keeps another session's;
  - `attention`: a chain root with an open chain gives context whose first line ends with
    `full board: <args[1]>`;
  - `session-start`: with a fake env whose `AIRSSTACK_HOME` is a temp directory, it creates
    `daily`, `sessions`, `notes`, `mocs` and `.index` under the vault root `<AIRSSTACK_HOME>/journal`
    (`lib/vault.lua:124-131`) and returns
    `nil`, because no session or note matches in an empty vault (§1). The card itself stays covered by the
    moved `orientation_test`;
  - `doctor`: `explain` moves into `hooks/lib/enforce.lua` as `M.explain(pack_root, file_path,
    env)`, which returns the text and lists the directories beside `pack_root`. `M.doctor(args,
    env)` returns `out, err`: `nil` and `"usage: doctor.lua <pack-root> <path>\n"` when `args[1]` or
    `args[2]` is missing or empty, else `M.explain(args[1], args[2], env)` and `nil`.
    `hooks/doctor.lua` is a script that calls `M.doctor(arg, airsstack.env)`, writes `out` to
    stdout and `err` to stderr, and returns (exit 0). The tests call `M.doctor`: a pack root inside
    a temp packs root gives the header and trace, and `{}` and `{"/r", ""}` each give the usage
    text. `airsl check packs` compiles `doctor.lua`.
- **`cargo make lua`** becomes an `@shell` script task (the `${TMPDIR:-/tmp}` grant needs a shell)
  that runs, in order:
  1. `airsl check extensions` and `airsl check packs` (the `plugins-check` gate, §1);
  2. `airsl test extensions`;
  3. `airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" --allow-exec git packs`
     (the plugins-test grants, §1). `packs/` holds tests, so the guideline packs having none does
     not fail the run;
  4. the fingerprint gate, copied from `../claudestacks/Makefile.toml:176-228` with
     `FP_SH=packs/sdlc/scripts/fingerprint.sh`: 8 cases, repeated with a sha256sum-only `PATH`.

## 7. Enforcement discovery

- **Discovery.** The enforce hook gets `{{root}}`, its own pack root, as `args[1]`. It reads
  `enforcement.json` at the root of each directory beside it, and the guideline packs ship that
  file as a root file. Any pack that ships a root `enforcement.json` takes part, third-party ones
  included.
- **What the registry did, and what replaces it** (`P/claudestacks/hooks/lib/enforce.lua`,
  `hooks/enforce.lua`):

  | Today | After |
  |---|---|
  | `registry_path`, `read_registry`, the `@claudestacks` filter (`MARKETPLACE_SUFFIX`), `AIRSSTACK_ENFORCE_REGISTRY` | removed. The candidate list is the directories beside `args[1]` (the subdirectories of its parent; core itself ships no `enforcement.json`), in name order, whose `enforcement.json` loads (`load_manifest`, unchanged). The candidate key is the pack directory name |
  | GATE 1 (`project_key`, `select_record`, and `common_dir`, whose one caller is `project_key`): a registry record must be bound to this project | removed. Every pack under the root is installed in this project by construction. GATE 2, GATE 3 and their labels are unchanged |
  | the bound manifest re-read from `record.installPath` | the candidate's manifest is used directly |
  | doctor parity section (`parity_report`, `tree_files`): compares the plugin repo with the install cache | removed. There is no cache, because the rendered files are the installed files. `toplevel` stays: GATE 3's `path_for_matching` uses it (§1) |

- **Trace lines.** `resolve` and its trace change only in these lines. Every other line stays as
  it is today (`lib/enforce.lua:533-605`).

  | Today | After |
  |---|---|
  | `registry: N @claudestacks plugin(s)` | `packs: N with enforcement.json under <packs root>` |
  | `STOP: no @claudestacks plugins in the registry` | (none; the next STOP covers it) |
  | `<key>: no usable enforcement.json` | unchanged, for a directory whose `enforcement.json` exists but does not load; a directory with none is not listed |
  | `STOP: zero manifests loaded (delivery failure — run the parity check)` | `STOP: no pack ships a usable enforcement.json` |
  | `project key: <key>` | (none) |
  | `<key>: GATE 1 no record bound to this project` | (none) |
  | `<key>: using <installPath>` | (none) |

- **Doctor.** `doctor.lua <pack-root> <path>` takes core's own pack root, as the hook's `args[1]`
  does, and finds the packs beside it the same way. It prints the same header as today, except
  that `registry: <path>` becomes `packs:    <parent of pack-root>`. It then prints the trace and
  the `outcome:` lines unchanged, and no parity section. With either argument missing or empty it
  writes `usage: doctor.lua <pack-root> <path>` to stderr, nothing to stdout, and exits 0, as the
  plugin's `--explain` does today (§1). `M.doctor` owns both outcomes (§6.5). The enforce-doctor
  skill runs:

  ```text
  airsl run --policy confined \
    --allow-env HOME --allow-env TMPDIR --allow-env AIRSSTACK_HOME \
    --allow-read / --allow-write "{{var tmpdir}}" \
    "{{var project_dir}}/{{root}}/hooks/doctor.lua" "{{var project_dir}}/{{root}}" "{{var arguments}}"
  ```

  The skill's read-back list (`enforce-doctor/SKILL.md:26-58`) changes as follows:
  - the `no @claudestacks plugins in the registry` item is removed;
  - the `zero manifests loaded` item becomes "`no pack ships a usable enforcement.json`: no installed
    pack carries a readable `enforcement.json`. Add a guideline pack to `agentstacks.toml`, then run
    `agentstacks apply`";
  - the GATE 1 item, the `using <installPath>` paragraph and the whole parity item (point 3) are
    removed.

  The guard paragraph keeps its `${TMPDIR:-/tmp}` advice and drops `${CLAUDE_PLUGIN_ROOT}`, which
  the skill no longer holds.
- **Pack sets.** A project without core gets guideline skills and no enforcement, the same as
  installing a guideline plugin alone today. A missing, unreadable or undecodable
  `enforcement.json` is skipped and the others still apply, as the plugin's `load_manifest` does
  today (`P/claudestacks/hooks/lib/enforce.lua:294-325`, `return nil -- absent or unreadable: skip
  this plugin, keep the rest`).
- **Grants.** `read = ["/"]` already covers sibling packs (§6.2).

## 8. Errors and refusals

- **New:** none.
- **Removed:** `skill "<n>" is shipped by both <a> and <b>` and its agent form (§5).
- **Reached through existing rules:**
  - airsl too old: `agentstacks.toml: airsl 0.1.4 installed, packs need >=0.1.5`;
  - airsl missing: `agentstacks.toml: airsl not found on PATH (needed by core, journal, sdlc)`
    (shape: `op/airsl.rs:110`);
  - duplicate install path: `claude/adapter.lua:<n>: write: path ".claude/skills/<name>/SKILL.md" is already proposed at adapter.lua:<m>`
    (shape: §1, the probed `copy:` case).
- **Doctor usage** (carried, renamed): `usage: doctor.lua <pack-root> <path>` on stderr, exit 0
  (§7).
- **Hooks.** They keep `--fail-open` (`docs/hooks.md` §3): a hook error never blocks Claude Code, as
  the wrappers' `exit 0` paths do today (`probes/hooks-map.md` §1, "Exit behaviour").

## 9. Behaviour changes

| Change | Why |
|---|---|
| Command and agent names change: `/claudestacks:x` → `/core-x`; `/claudestacks-sdlc:x` → `/sdlc-x`; `/claudestacks-journal:journal-x` → `/journal-x`; `/claudestacks-guideline-rust:rust-guidelines` → `/guideline-rust-guidelines`; `claudestacks:coder` → `core-coder`, `claudestacks-sdlc:chain-reader` → `sdlc-chain-reader`, `claudestacks-journal:journal-curator` → `journal-curator` | Native install has no plugin namespace, and the extension namespaces by pack (§4) |
| Installed per project into `.claude/`, never machine-wide | intent constraint |
| `statusline` is `/core-statusline` and lives in core's `native/claude/` | It configures a Claude-only key, and the bare name would replace the built-in `/statusline` |
| airsl must be on `PATH` and ≥0.1.5. The hook wrappers' fallback lookup (`$AIRSL_BIN`, `~/.local/bin`, `~/.cargo/bin`) and the preflight STATUS/FIX text are gone. `plan` and `doctor` report a missing or old airsl | hooks run the core's argv (`docs/hooks.md` §3) |
| The enforce hook finds guideline packs installed in the same project, not plugins in the registry | §7 |
| `style_reinject` is set in `agentstacks.toml` (`[packs.core.options]`), not in the plugin's `userConfig` | options reach hooks through args (§6.3) |
| A `SubagentHandback` violation is a `deny` with a reason, where it used to be exit 2 with the text on stderr. Both block the call and return the reason to the subagent, which retries. Only the error text differs: it no longer shows the hook command in brackets (§1, `probes/claude-handback-results.md`) | the runtime's `block` result |
| The SessionEnd reminder appears in `plan` as a skip. Its output was discarded before too (§1) | the claude extension's existing skip |
| The enforce pointer names `/guideline-rust-guidelines`, where it used to say `claudestacks-guideline-rust:rust-guidelines` | rendered `{{skill}}` |
| The enforce doctor has no GATE 1 and no parity section; its "no manifests" stop reads `no pack ships a usable enforcement.json` | §7 |
| `sdlc` requires `core`; the sdlc plugin declared no dependency | Its skills name core's agents and skill through tokens, and its agent-stop hook runs core's check. Both need `requires.packs` (formats spec §3.3) |
| `/core-statusline` always detects its source as inside the project and recommends project scope; the plugin's source sat in the install cache | `statusline/SKILL.md:41` tests whether the source path is under `$PWD` |
| `cmux` is not converted | intent |

## 10. Architecture (approved by the author on 2026-11-23)

Approved in two steps on 2026-11-23: first the tree below without `clash.rs`, then the removal of
the core clash rule (§5), keeping claudevs' `error.rs`, and the strict §3 idiom (no nil-safe
variant; pack work waits for airsl 0.1.5).

```text
crates/agentstacks/       types/set/clash.rs removed; types/shorthand.rs: 5 official packs
crates/agentstacks-cli/   + tests/packs.rs (§11)
crates/claudevs/          imported; renames of §2
crates/claudevs-cli/      imported unchanged
extensions/claude/        §4
packs/{core,sdlc,journal,guideline-rust,guideline-python}/   §6
scripts/corpus-fetch.sh   moved with claudevs

claudevs-cli → claudevs → airsl      agentstacks-cli → agentstacks → airsl   (no edge between the two)
```

claudevs after the renames (`src/`):

```text
lib.rs check.rs doctor.rs error.rs suite.rs validate.rs
case/{mod,model,yaml,lua,migrate,discover,runner}.rs
contract/{mod,event,handler,matcher,site}.rs
harness/{mod,environment,hooks,module,payload,project,semantics,spawn,verdict}.rs
layout/{mod,installed,manifest}.rs   native/{mod,declared}.rs   report/{mod,render}.rs
types/{mod,event,ident,version}.rs   types/name/{mod,case,marketplace,plugin}.rs
wiring/{mod,finding,invocations,matchers,refs,run}.rs
```

Rules applied (`claudestacks-guideline-rust:rust-guidelines`):

- `modularity.md` Rule 1: one concept per file. The three name newtypes share one concept family,
  so they get one folder.
- `modularity.md` Rule 2: `types/event.rs` and `contract/event.rs` stay two types. Their docs state
  the different invariant (§1).
- `mod-rs-export-only.md`: `types/name/mod.rs` holds only `//!`, `mod` and `pub use`.
- `static-dispatch.md`, `strong-types.md`: no new types or traits, so nothing new to apply.
- `workspace.md`: centralised `[workspace.dependencies]` and workspace lints (§2).
- The author's naming rule: one-word files; files sharing a word go into a folder named by it; a
  file never repeats its folder's word.

In `crates/agentstacks`:
- `types/set/clash.rs` is removed;
- `types/set/{mod,check,error}.rs` lose its declaration, call, variants and docs (§5);
- `types/pack/load.rs` loses a doc line;
- `types/shorthand.rs` changes.

No other module changes.

claudevs keeps its crate-wide `src/error.rs` (one `Error` enum). The approved adjustment is the
nine renames, and the move keeps behaviour unchanged. Splitting errors by context, under the
module-layout rule "errors live with their context", would restructure every module that uses it
(`grep -rn 'crate::error' crates/claudevs/src`). It is a follow-up chain if wanted, not part of
this one.

## 11. Testing

| What | How |
|---|---|
| claudevs | Its existing unit, integration and doc tests and the cli tests, unchanged except the two renamed test files, under `cargo make dod` (all green today, `probes/claudevs-results.md` steps d–e). `cargo make claudevs-check` in CI (§2) |
| claude extension | The updated `lib/*_test.lua`: `values_test` (prefixed invoke, dir, agent), `hooks_test` (prefixed agent-stop matcher), `frontmatter_test` (rendered `name`). `agent_id` through the Rust translator tests in `tests/claude.rs`: the echo hook prints it (`a1` for `subagent`, `nil` elsewhere, `nil` for the empty id in `noagent`) (§4) |
| core | `clash.rs` tests removed. A set test loads `shared-skill` (two packs ship skill `status`) and `shared-agent` (two packs ship agent `coder`) with no diagnostic (§5). `shorthand.rs` test lists 5 packs |
| packs (Lua) | The 20 moved tests and the new hook-module cases under `cargo make lua` (§6.5), with a fake `airsstack.env` where a module reads the environment; test-only fixtures beside them |
| end to end | New `crates/agentstacks-cli/tests/packs.rs`, built like `commands.rs`: it copies `packs/` and `extensions/claude` into a temp git project, runs `init --tool ./… --pack ./…` for all five packs with `--yes`, and asserts the points below. Skipped without airsl ≥0.1.5 unless `AGENTSTACKS_REQUIRE_AIRSL` is set, as in `commands.rs` |
| gate | `cargo make dod` green; `cargo make deny` green |

`packs.rs` asserts:

- exit 0;
- exactly one skip, `core/hook/3: claude shows no hook output to the model at session end`;
- 28 skill directories (`core-*` ×7, `core-statusline`, `sdlc-*` ×11, `journal-*` ×7,
  `guideline-rust-guidelines`, `guideline-python-guidelines`) and 10 agent files;
- the `SubagentStop` matchers `core-coder|core-explorer|core-reviewer`,
  `sdlc-task-briefer|sdlc-chain-reader|sdlc-artifact-reviewer` and `journal-curator`;
- running the rendered `PreToolUse` `Read|Edit|Write` command with `CLAUDE_PROJECT_DIR` set to the
  temp project and `TMPDIR` set to a fresh directory inside the test's temp dir (so no sentinel
  from an earlier run is seen, §1), on stdin
  `{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"<project>/src/main.rs"},"session_id":"s1","cwd":"<project>"}`
  in a project holding `Cargo.toml` prints a `hookSpecificOutput.additionalContext` that contains
  `/guideline-rust-guidelines`.

## 12. Docs

- `docs/hooks.md` §5: `agent_id`.
- `docs/extensions.md` §6 and `docs/authoring.md`: the prefixed names. `docs/extensions.md:150`
  shows `tmpdir = "${TMPDIR:-/tmp}"`, and `:219` shows the prefixed `dir`.
- `docs/cli.md:120` and `docs/plan-and-apply.md:117-123`: example paths become
  `.claude/skills/core-status/…` and `.claude/agents/core-coder.md`.
- Org docs (`.github/profile/README.md`, the workspace `README.md`): no change. The profile names
  no individual packs (`grep -n -i 'cmux\|pack' .github/profile/README.md` → lines 25 and 79, both
  generic), and agentstacks' status changes at release (`release-and-migration`).
- `docs/cli.md:59`, `README.md:102`, `CLAUDE.md`: the five official packs. `CLAUDE.md` also gets:
  - its Status updated: `packs/` and the claudevs move landed;
  - the "Porting claudestacks plugins into packs" table replaced by a pointer to this spec;
  - `crates/claudevs{,-cli}` listed as built.

## 13. Handed to other chains

| Chain | Receives |
|---|---|
| airsl (airsstack/airsl#17) | pass the module name to a required chunk; release airsl and airsl-cli 0.1.5 (§3) |
| `release-and-migration` | publishing the claudevs crates from here; claudevs-cli release/smoke workflows; the migration doc, which lists the §9 changes; tagging `packs-v*` |

## 14. Non-goals

- Converting `claudestacks-cmux`.
- claudevs checking agentstacks-rendered `.claude/` config, or any new claudevs feature.
- New skills, agents or hooks beyond what the five plugins ship.
- OpenCode rendering (`foundation/02`).
- Editing or archiving claudestacks.
- Publishing crates, tagging releases.
- A length limit or other new refusals for rendered names.
- Renaming the product names (claudestacks, claudestacks-sdlc, …) in pack prose and literals.
- Splitting claudevs' crate-wide `error.rs`.
