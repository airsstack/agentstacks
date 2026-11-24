---
status: done
created: 2026-11-23
depends-on: [10, 11]
built-on: 0f89fdb3ebb3
---

# sdlc pack Implementation Plan

**Goal:** Ship the claudestacks-sdlc plugin as pack `sdlc`.

**Architecture:**
- `packs/sdlc/` follows spec §6.1 and holds:
  - 11 skills and 3 agents;
  - the root `references/` and `scripts/`;
  - the `status.sh` and `fingerprint.sh` the skills run, carried unchanged (spec §1:
    they hold no plugin-root or namespaced text).
- `requires.packs = ["core"]`: the skills name core's agents and skill through tokens, and the
  agent-stop hook runs core's `scripts/handoff_check_hook.lua` (`{{root core}}`, spec §1
  `probes/crosspack-results.md` Q1; §9).
- The attention hook is a module. The board name in its first line comes from `args[1]`, which is
  `{{skill status}}`, rendered `/sdlc-status`. The product label `claudestacks-sdlc:` stays (spec
  §6.4).
- `lib/status.lua` and the libraries it loads take the §3 idiom, because the hook loads them. The
  idiom also holds when `status.lua` loads them as a skill-run script: the entry's own requires stay
  plain.
- `cargo make lua` gains step 4, the fingerprint gate (spec §6.5).

**Tech Stack:** pack format, Lua under airsl 0.1.5, POSIX sh, cargo-make.

**Content authority:** spec §3 (idiom, `requires.airsl`), §6.1, §6.2 (sdlc manifest), §6.3 rows H2
and H13, §6.4, §6.5 (`status_test` row, new `attention` test, `lua` step 4), §9 (sdlc rows).

---

## Files that change

```
packs/sdlc/pack.toml                     — create: §6.2 text
packs/sdlc/README.md                     — create: plugin README with the install block replaced
packs/sdlc/agents/{artifact-reviewer,chain-reader,task-briefer}.md — create: converted
packs/sdlc/skills/{adr,design,distill,execute,intent,milestone,plan,roadmap,setup,status,triage}/SKILL.md — create: converted
packs/sdlc/references/{artifact-chain,artifact-review,review-policy,templates}.md — create: converted
packs/sdlc/scripts/{status,fingerprint}.sh  — create: carried unchanged
packs/sdlc/scripts/status.lua            — create: carried unchanged (it does not call attention)
packs/sdlc/scripts/attention.lua         — create: module
packs/sdlc/scripts/attention_test.lua    — create: new
packs/sdlc/scripts/lib/{adr,fingerprint,roadmap,status}.lua — create: carried; idiom; attention takes the board name
packs/sdlc/scripts/{adr,fingerprint,roadmap,status}_test.lua — create: moved; status-line strings
Makefile.toml                            — modify: `lua` step 4, the fingerprint gate
```

## Order of work

### Task 1 — Scripts, attention module and the fingerprint gate

**Files:**
- Create `packs/sdlc/pack.toml` (header and `[requires]` only), `scripts/**`.
- Modify `Makefile.toml`.

**Change:**
- `pack.toml` holds the §6.2 sdlc text up to `[requires]`: `exclude`; `[pack]` with name `sdlc`,
  version `0.1.0`, api 0 and the §6.2 description; `[requires] packs = ["core"]`,
  `airsl = ">=0.1.5"`. Task 3 adds the hooks.
- `lib/status.lua`: `M.attention(rootdir, board)` takes the board name. Its first line is
  `claudestacks-sdlc: %d open, %d need attention — full board: <board>` (`:825`).
- `lib/status.lua` requires its siblings `adr`, `fingerprint` and `roadmap` as `base .. "<name>"`,
  and `lib/adr.lua` requires `base .. "fingerprint"` (§3 idiom).
- `scripts/status.lua` keeps its plain requires.
- `attention.lua` returns `function(event)`:
  - the chain root is `event.cwd` joined with `status.ROOT`, or the process directory when `cwd` is
    absent, as today;
  - it returns `{ context = text }` when `status.attention(root, event.args[1])` is not empty, else
    `nil`;
  - its require is `base .. "lib.status"`;
  - its usage comment showing an `airsl run` command line (`attention.lua:3-4`) is dropped (spec
    §6.4 comment row).
- `status_test.lua`: the six status-line strings are built with the board name argument
  `/sdlc-status`. Each call passes it.
- `Makefile.toml` `[tasks.lua]` gains step 4: the fingerprint gate of
  `../claudestacks/Makefile.toml:176-228`, with `FP_SH=packs/sdlc/scripts/fingerprint.sh`. It runs
  8 cases, then repeats them with a `PATH` holding only `sha256sum`.

**Proof:**
- `attention_test.lua`:
  - `attention_returns_context_for_open_chains`: a temp chain root with one open chain and
    `args = {"/sdlc-status"}` gives `{ context = … }`, whose first line ends
    `full board: /sdlc-status`;
  - `attention_returns_nil_without_a_chain_root`.
- `status_test`, `adr_test`, `fingerprint_test` and `roadmap_test` pass.
- `cargo make lua` runs the fingerprint gate: 8 cases twice, all pass.

**Done when:** `cargo make lua` passes and `cargo make dod` is green. Commit
`feat(repo): sdlc pack scripts and attention hook module; fingerprint gate`.

### Task 2 — Agents, references and skills

**Files:**
- Create `packs/sdlc/agents/*.md`, `skills/*/SKILL.md`, `references/*.md`, `README.md`.

**Change:** Converted by the spec §6.4 table:
- `claudestacks-sdlc:<x>` → `{{skill <x>}}` or `{{agent <x>}}`;
- `claudestacks:<x>` → `{{skill core/<x>}}` or `{{agent core/<x>}}`;
- in a `SKILL.md`, `${CLAUDE_PLUGIN_ROOT}/<x>` → `{{var project_dir}}/{{root}}/<x>`;
- in agents and `references/*.md`, `${CLAUDE_PLUGIN_ROOT}/<x>` → `{{root}}/<x>`;
- `$ARGUMENTS` → `{{var arguments}}`;
- the examples at `design/SKILL.md:128` and `plan/SKILL.md:68` become the prose "the stack's
  guideline skill (for Rust, the guideline-rust pack's skill)";
- frontmatter by the §6.4 rules; names unchanged (no sdlc item is renamed);
- backticked bare names (spec §6.4 own-pack row; `probes/selfref-results.md` grep 4):
  `execute/SKILL.md:50` `` `task-briefer` `` → `` `{{agent task-briefer}}` ``, and
  `execute/SKILL.md:61` `` `coder` `` → `` `{{agent core/coder}}` `` (core's agent).

Product names, the handoff file names (`claudestacks-sdlc-<chain>-spec-01.md`) and the attention
label stay. The README install block becomes `agentstacks init --tool claude --pack core --pack sdlc`.

**Proof:**
- In a temp git project with copies of `packs/core`, `packs/sdlc` and `extensions/claude`,
  `agentstacks init --tool ./extensions/claude --pack ./packs/core --pack ./packs/sdlc --yes` exits
  0 and writes `.claude/skills/sdlc-<s>/SKILL.md` for all 11 skills and `.claude/agents/sdlc-*.md`
  for the 3 agents.
- the three spec §6.4 gate greps, with `packs/` narrowed to `packs/sdlc`, print nothing (the third applies to journal only).

**Done when:** both checks pass and `cargo make dod` is green. Commit `feat(repo): sdlc pack
skills, agents and references`.

### Task 3 — sdlc hooks

**Files:**
- Modify `packs/sdlc/pack.toml`.

**Change:** Append the two §6.2 sdlc `[[hooks]]` tables:
- `session.start` running `scripts/attention.lua`, `args = ["{{skill status}}"]`, `read = ["/"]`;
- `agent.stop`, `agents = ["task-briefer", "chain-reader", "artifact-reviewer"]`, running
  `{{root core}}/scripts/handoff_check_hook.lua`, `may_block = true`, with grants `env = ["HOME",
  "TMPDIR"]` and `read = ["/"]`.

The file now equals the §6.2 sdlc text.

**Proof:**
- After the Task 2 `init`, `settings.json` holds:
  - a `SubagentStop` group with matcher
    `sdlc-task-briefer|sdlc-chain-reader|sdlc-artifact-reviewer`, whose command runs module
    `core.scripts.handoff_check_hook`;
  - a `SessionStart` group running `sdlc.scripts.attention` with the argument `/sdlc-status`.
- Piping a `SessionStart` payload whose `cwd` holds a chain root into that rendered command prints a
  `hookSpecificOutput.additionalContext` whose first line ends `full board: /sdlc-status`.

**Done when:** both checks pass and `cargo make dod` is green. Commit `feat(repo): sdlc pack
hooks`.

---

## Risks

- `lib/status.lua` is large and shared by the hook and the skill-run `status.lua`. Only its
  requires and `attention`'s signature change. Every other `status_test` case must pass unchanged.
- The fingerprint gate copies claudestacks shell. Only `FP_SH` changes, and the 8 expected
  fingerprints stay as they are.

## Proof

- `cargo make dod` green, including the fingerprint gate. `packs/sdlc/pack.toml` equals the §6.2
  text.

## Review findings

Reviewer: no blocking findings. `cargo make lua` exit 0 (`640 passed, 0 failed (25 files)`; `fingerprint gate: 8 cases passed`, `… with sha256sum`); `pack.toml` byte-identical to the spec §6.2 sdlc block; gate code matches `../claudestacks/Makefile.toml:176-228` except `FP_SH`, 8 fingerprints unchanged; carried scripts unchanged; 37 `${CLAUDE_PLUGIN_ROOT}` lines → 37 `{{root}}`; 21 namespaced references → 19 tokens + 2 guideline-prose rewrites; §6.4 greps 1–2 print nothing; no other frontmatter with an unquoted `: `. Orchestrator: workspace `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` exit 0 (23 `test result: ok`, lua `559 passed, 0 failed (18 files)`, fingerprint gate 8+8); `init` core (HEAD) + sdlc in a temp git project exit 0, `59 written`, 1 skip; 11 `sdlc-*` skills, 3 `sdlc-*` agents; no `{{` or `CLAUDE_PLUGIN_ROOT` under `.claude`; `SubagentStop` matcher `sdlc-task-briefer|sdlc-chain-reader|sdlc-artifact-reviewer` → module `core.scripts.handoff_check_hook`; `SessionStart` → `sdlc.scripts.attention` `/sdlc-status`, piped with a chain root → `{"hookSpecificOutput":{"additionalContext":"claudestacks-sdlc: 1 open, 0 need attention — full board: /sdlc-status",…}}`; `plan` 0 to write; `doctor` healthy.

- 🟡 open — `scripts/attention.lua:10-12`, `lib/status.lua:13-17`, `lib/adr.lua:12-14`: nothing catches the §3 idiom being reverted (the tests load with an empty prefix; reverting `base .. "adr"` keeps 264 passed while the hook fails "module `lib.adr` not found"). Same class as plan 11's open finding.
- ❓ open — backticked stage names (`adr`, `plan`, `setup`, `execute`, e.g. `design/SKILL.md:24,37,95,123`, `review-policy.md:10`, `templates.md:146,219`) stay plain; the plan lists only `execute/SKILL.md:50,61`. Read as stage labels.
- 🔵 open — `README.md:39`: `[LICENSE](./LICENSE)` is dead; core uses `../../LICENSE`.
- 🔵 open — "plugin" wording left in `agents/artifact-reviewer.md:5,64`, `references/artifact-chain.md:11,58`, `references/templates.md:30`, `skills/distill/SKILL.md:46` (`cargo make plugins` does not exist here).
- 🔵 open — `scripts/attention.lua:1-2`: names Claude Code in a tool-neutral pack.
- 🔵 open — `attention_test.lua`: no case for the process-directory fallback when `cwd` is absent.
- 🔵 open — `lib/status.lua:808-810`: a nil `command` would print "full board: nil" (harmless: `args` is always present, `docs/hooks.md`).
- 🔵 open — `design/SKILL.md:128`, `plan/SKILL.md:68`: prose reads "(the stack's guideline skill; for Rust, …)", redundant in design.
- 🔵 open — `plan/SKILL.md:3`: the quoting fix (see Deviations) is guarded only by plan 15's `init`.

## Deviations

- 2026-11-24 — Tasks ran as two parallel coders on disjoint files (Tasks 1+3 owning `pack.toml`, `scripts/**`, `Makefile.toml`; Task 2 owning agents, skills, references, README), in parallel with plan 12. Coders ran file-local checks; the orchestrator ran `dod` and the Task 2/3 `init` proof once.
- 2026-11-24 — Task 2 `init` first refused `./packs/sdlc/skills/plan/SKILL.md:3: mapping values are not allowed in this context`: the carried description holds `spec: skipped` unquoted. The orchestrator double-quoted it; `init` then exits 0.
- 2026-11-24 — Task 1: the `attention` parameter is `command` (a local `board` already exists in `lib/status.lua`); the attention test expects `1 open, 0 need attention`; `lib/status.lua` header drops the spec-section parenthetical; moved test usage comments say `cargo make lua`; `scripts/status.lua` keeps its `airsl run` usage comment (carried unchanged). `Makefile.toml` `[tasks.lua]` description and the file header comment were updated with the gate.
- 2026-11-24 — Task 2: `design/SKILL.md:127` "guideline plugin's" → "guideline pack's"; README drops the `claudestacks-sdd` paragraph and says "the `core` pack"; agent frontmatter `tools` → `[read, grep, glob, shell, write]`, models `deep`/`fast`/`balanced`; `setup` `disable-model-invocation` → `invocation: user`.
- 2026-11-24 — Not committed (per-task commits held for the author).
