---
status: approved
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
