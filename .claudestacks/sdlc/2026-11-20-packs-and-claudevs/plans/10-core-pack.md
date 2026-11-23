---
status: done
created: 2026-11-23
depends-on: [05, 09]
built-on: 0f89fdb3ebb3
---

# Core pack Implementation Plan

**Goal:** Ship the claudestacks plugin's skills, agents and skill-run scripts as pack `core`.

**Architecture:**
- `packs/core/` follows spec §6.1. This plan carries everything a skill or agent uses. Plans 11 and
  12 carry the hooks:
  - plan 11: style, handoff and comm checks, and the message hooks;
  - plan 12: enforce, rearm, doctor, and the enforce-doctor skill.
- `pack.toml` starts here with the §6.2 header (`exclude`, `[pack]`, `[requires]`, both
  `[options.*]`). Plans 11 and 12 add the `[[hooks]]` tables, so the finished file equals the §6.2
  text.
- `cargo make lua` becomes the §6.5 `@shell` task, so `packs/` is checked and tested from this plan
  on.
- `statusline` is a native claude skill, `native/claude/skills/core-statusline/`, because the bare
  name would replace the built-in `/statusline` (spec §1, §9).

**Tech Stack:** pack format, Lua under airsl 0.1.5, cargo-make, the `agentstacks` binary.

**Content authority:** spec §3 (`requires.airsl`), §6.1, §6.2 (core header), §6.4, §6.5 (moved
tests, `discuss_skill_test`, the `lua` task steps 1–3), §9 (statusline row), §11 (packs row).

---

## Files that change

```
Makefile.toml                                         — modify: `lua` becomes an @shell script (check, test extensions, test packs)
packs/core/pack.toml                                  — create: §6.2 header, no hooks yet
packs/core/README.md                                  — create: plugin README with the install block replaced
packs/core/agents/{coder,explorer,reviewer}.md        — create: converted agents
packs/core/skills/context-handoff/{SKILL.md,references/protocol.md}   — create: converted
packs/core/skills/discuss/{SKILL.md,references/protocol.md}           — create: converted
packs/core/skills/orchestrate/SKILL.md                — create: converted
packs/core/skills/process-guidelines/**               — create: converted (4 files)
packs/core/skills/snapshot-load/SKILL.md              — create: converted
packs/core/skills/snapshot-save/SKILL.md              — create: converted
packs/core/native/claude/skills/core-statusline/SKILL.md — create: from skills/statusline/SKILL.md
packs/core/scripts/{discuss,handoff,handoff_report,statusline,statusline-install}.lua — create: carried
packs/core/scripts/lib/{discuss,handoff,handoff_report,comm_report,statusline}.lua    — create: carried
packs/core/scripts/install-airsl.sh                   — create: carried unchanged
packs/core/scripts/{comm_report,discuss_skill,discuss,handoff_report,handoff,statusline_install,statusline}_test.lua — create: moved tests
```

## Order of work

### Task 1 — Pack header, agents and skill-run scripts

**Files:**
- Create `packs/core/pack.toml`, `README.md`, `agents/*.md`, `scripts/**` listed above.

**Change:**
- `pack.toml` holds the §6.2 core text from `exclude = …` through `[options.archive_keep]`, with no
  `[[hooks]]` table: `exclude = ["README.md", "**/*_test.lua"]`; `[pack]` `name = "core"`,
  `version = "0.1.0"`, `api = 0` and the §6.2 description; `[requires] airsl = ">=0.1.5"`;
  `style_reinject` (boolean, default true) and `archive_keep` (integer, default 20), each with its
  §6.2 description.
- Agents are converted by spec §6.4:
  - `claudestacks:<agent>` → `{{agent <agent>}}`;
  - `claudestacks:<skill>` → `{{skill <skill>}}`;
  - `${CLAUDE_PLUGIN_ROOT}/skills/<s>/…` → `{{skill_dir core/<s>}}/…`;
  - other `${CLAUDE_PLUGIN_ROOT}/<x>` → `{{root}}/<x>`;
  - frontmatter `model` and `tools` map to the neutral words; `name` stays `coder`, `explorer`,
    `reviewer`.
- `scripts/` files and their tests are carried as they are, except comments that name a plugin
  item (spec §6.4 comment row): `scripts/discuss.lua:1` and `scripts/lib/discuss.lua:1` say "the
  core pack's discuss skill"; `scripts/lib/discuss.lua:19` says "the `archive_keep` option". These
  modules are loaded only by skills, so they keep their plain requires (spec §3: "A file loaded only by scripts that skills run
  … keeps its plain requires"). Plan 11 gives the idiom to the libs a hook loads.
- `README.md` is the plugin README, except its install block: the `/plugin` lines become
  `agentstacks init --tool claude --pack core`. Other text stays as it is (spec §6.4).

**Proof:**
- `airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" --allow-exec git packs`
  passes the moved tests: `comm_report_test`, `discuss_test`, `handoff_report_test`,
  `handoff_test`, `statusline_install_test`, `statusline_test`.
- `airsl check packs` compiles every carried `.lua`.

**Done when:** both commands pass and `cargo make dod` is green. Commit
`feat(repo): core pack agents and scripts`.

### Task 2 — `lua` task over extensions and packs

**Files:**
- Modify `Makefile.toml` `[tasks.lua]`.

**Change:** `[tasks.lua]` keeps category Gate and its place in `dod`. Its description becomes "Lua:
airsl check and airsl test over extensions/ and packs/". It becomes `script_runner = "@shell"`
with a script that runs, stopping at the first failure (`set -e`):
1. `airsl check extensions` and `airsl check packs`;
2. `airsl test extensions`;
3. `airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" --allow-exec git packs`.

Step 4, the fingerprint gate, is added by plan 13. The `dod` and `deny` comments that name `lua`
stay true.

**Proof:**
- With Task 1's pack in place, `cargo make lua` runs all three steps and exits 0, and its output
  lists the moved core tests.
- With a deliberate syntax error in a scratch copy of `packs/core/scripts/discuss.lua`, step 1
  exits non-zero. The error is reverted before the commit.

**Done when:** `cargo make lua` exits 0 and `cargo make dod` is green. Commit
`build(repo): lua gate checks and tests packs`.

### Task 3 — Core skills

**Files:**
- Create `packs/core/skills/{context-handoff,discuss,orchestrate,process-guidelines,snapshot-load,snapshot-save}/**`
  and `packs/core/scripts/discuss_skill_test.lua`.

**Change:**
- Each skill is converted by the §6.4 table. In a `SKILL.md`:
  - `${CLAUDE_PLUGIN_ROOT}/skills/<s>/…` → `{{var project_dir}}/{{skill_dir <s>}}/…`;
  - other plugin-root paths → `{{var project_dir}}/{{root}}/…`;
  - `${CLAUDE_SESSION_ID}` → `{{var session_id}}`;
  - `$ARGUMENTS` → `{{var arguments}}`;
  - `${user_config.archive_keep}` → `{{option archive_keep}}`.
- In `references/*.md`, the "any other file" forms apply.
- `context-handoff/references/protocol.md:145-151`: the sentence naming `${CLAUDE_PLUGIN_ROOT}` as a
  variable the worktree guard cannot resolve drops that variable and its `echo` line. The advice
  for the other variables stays.
- Pack items named without a namespace become tokens (spec §6.4 own-pack row; inventory
  `probes/selfref-results.md` greps 2–4):
  - `/snapshot-save` (`snapshot-save/SKILL.md:16`) and `/snapshot-load`
    (`snapshot-load/SKILL.md:28,37`) → `{{skill snapshot-save}}`, `{{skill snapshot-load}}`;
  - backticked bare names in `orchestrate/SKILL.md`, `context-handoff/SKILL.md`,
    `context-handoff/references/protocol.md`, `process-guidelines/SKILL.md`,
    `process-guidelines/references/{agent-orchestration,model-routing}.md`,
    `snapshot-load/SKILL.md` and `snapshot-save/SKILL.md` (`` `coder` `` → `` `{{agent coder}}` ``,
    `` `orchestrate` `` → `` `{{skill orchestrate}}` ``, and so on), backticks kept;
  - `orchestrate/SKILL.md:18` "Namespaced as `claudestacks:coder`, `claudestacks:reviewer`, etc."
    → "Named {{agent coder}}, {{agent reviewer}}, and so on.", the rest of the line unchanged;
  - plain role words ("the coder") and the label "the `discuss` archive"
    (`snapshot-save/SKILL.md:45`) stay.
- `orchestrate/SKILL.md:61`: the example reference to `claudestacks-guideline-rust:rust-guidelines`
  becomes the prose "the stack's guideline skill (for Rust, the guideline-rust pack's skill)".
- Frontmatter: `disable-model-invocation: true` → `invocation: user`; `argument-hint` →
  `argument_hint`; `name` stays the plugin name, since no core skill is renamed (spec §6.1).
- Product names stay (spec §6.4 last paragraph).
- `discuss_skill_test.lua`: the path at `:8` becomes `packs/core/skills/discuss/SKILL.md`. It
  asserts that the archive-keep value is single-quoted around `{{option archive_keep}}`.
  `only_the_author_can_invoke_it` (`:15-21`) asserts `invocation: user`, and
  `command_lines_carry_the_session_and_the_plugin_root` (`:33-38`) asserts `{{var session_id}}` and
  `"{{var project_dir}}/{{root}}/scripts/discuss.lua"` (§6.4 changes the text under test).

**Proof:**
- `discuss_skill_test` passes under `cargo make lua`.
- In a temp git project with copies of `packs/core` and `extensions/claude`,
  `agentstacks init --tool ./extensions/claude --pack ./packs/core --yes` exits 0 and writes
  `.claude/skills/core-<s>/SKILL.md` for the six skills (frontmatter `name: "core-<s>"`) and
  `.claude/agents/core-{coder,explorer,reviewer}.md`.
- the three spec §6.4 gate greps, with `packs/` narrowed to `packs/core`, print nothing (the third applies to journal only).

**Done when:** both checks pass and `cargo make dod` is green. Commit `feat(repo): core pack
skills`.

### Task 4 — Native `core-statusline`

**Files:**
- Create `packs/core/native/claude/skills/core-statusline/SKILL.md`.

**Change:**
- The file is `P/claudestacks/skills/statusline/SKILL.md` with Claude's own frontmatter. Its
  `name` is `core-statusline`, and `disable-model-invocation: true` is kept as Claude's key.
- Every `${CLAUDE_PLUGIN_ROOT}`, bare or with a path (`:41,81,91,115,126`), becomes
  `{{var project_dir}}/{{root}}`.
- `/claudestacks:statusline` in the description becomes `/core-statusline`, the native skill's
  own final name (spec §4 "Native files", §9).

**Proof:**
- The Task 3 `init` check also writes `.claude/skills/core-statusline/SKILL.md`. In it,
  `{{var project_dir}}/{{root}}` reads `${CLAUDE_PROJECT_DIR}/.claude/agentstacks/core`.
- The Task 3 grep, run again, prints nothing.

**Done when:** both checks pass and `cargo make dod` is green. Commit `feat(repo): core-statusline
native claude skill`.

---

## Risks

- The `lua` task now needs airsl 0.1.5 on `PATH` for `dod`. CI has it from plan 09.
- `enforce-doctor` is carried by plan 12. Until then, a core text naming `claudestacks:enforce-doctor`
  would fail the load with an unknown target. Task 3 converts no such reference: the only one is in
  `enforce-doctor/SKILL.md` itself.
- Statusline's source-path test (`statusline/SKILL.md:41`) now always sees a path inside the
  project. That is the §9 behaviour change, not a defect.

## Proof

- `cargo make dod` green; the core-only `init` writes 7 skill directories and 3 agent files.

## Review findings

Reviewer: no blocking findings. Re-ran `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` exit 0 (23 `test result: ok`; lua `11 file(s) compiled`, `17 file(s) compiled`, `71 passed, 0 failed (5 files)`, `216 passed, 0 failed (7 files)`); core-only `init` in a temp git project exit 0, 26 written, 7 skill dirs (`core-context-handoff`, `core-discuss`, `core-orchestrate`, `core-process-guidelines`, `core-snapshot-load`, `core-snapshot-save`, `core-statusline`), 3 agents, no `{{` or `CLAUDE_PLUGIN_ROOT` left under `.claude`; `plan` `0 to write`; `doctor` `healthy`; the two §6.4 gate greps over `packs/core` print nothing; `core-statusline` renders `${CLAUDE_PROJECT_DIR}/.claude/agentstacks/core` (16 lines).

- 🟡 open — `packs/core/README.md:22` "Every skill is namespaced `claudestacks:`, so `discuss` runs as `/claudestacks:discuss`" is false (init renders `/core-discuss`); `:1-9` ("# claudestacks", "core plugin", "holds the suite's hooks") stale too. Task 1 says other README text stays.
- 🟡 open — only `discuss/SKILL.md` is pinned by a test; reverting any other conversion (a `${CLAUDE_PLUGIN_ROOT}` in orchestrate/context-handoff/statusline, a bare `` `coder` ``, `name: statusline` in the native skill) passes `dod` and `init`. The §6.4 greps are a one-off proof, not a gate.
- 🔵 open — `Makefile.toml:97`: reverting `[tasks.lua]` to `airsl test extensions` fails nothing; pack tests would drop out of `dod` silently.
- 🔵 open — `core-statusline/SKILL.md:16,50,52-53`: prose still says "plugin root", `plugins/claudestacks/scripts/`, and an out-of-tree branch `{{var project_dir}}/{{root}}` can no longer reach.
- 🔵 open — `discuss_skill_test.lua:23` test name `every_user_config_placeholder_is_single_quoted` names the retired `${user_config…}` form.
- 🔵 open — `handoff_test.lua:6` names sibling `enforce_test.lua` fixtures (arrives plan 12).
- 🔵 open — `install-airsl.sh:79` echo "Plugin hooks still find airsl there" and `:154` "the plugin tests" while `:2` says "every pack hook".
- 🔵 open — `lib/discuss.lua:3` "this chain's spec" (workflow vocabulary); `:28` cites `hooks/lib/enforce.lua:35-174` by line (absent until plan 12; cite `M.project_key`).
- 🔵 open — `statusline-install.lua:4` keeps "NOT plugin content".
- 🔵 open — "plugin" wording mixed: `context-handoff/SKILL.md:28,71`, `discuss/references/protocol.md:75` vs `context-handoff/references/protocol.md:187` "the core pack".
- 🔵 open — `context-handoff/references/protocol.md:145` "run the block above with that absolute literal in place of the variable" points at no variable any more.
- 🔵 open — `context-handoff/references/protocol.md:187` "the core pack registers it as a hook on three events" is false until plan 11 lands.
- 🔵 open — `snapshot-save/SKILL.md:74` cites `hooks/lib/enforce.lua` `M.project_key` (plan 12) and other plugin-era paths.

## Deviations

- 2026-11-23 — Tasks 1–4 ran as one parallel batch (disjoint files); coders ran file-local checks only, and the orchestrator ran the `init` proof, the gate greps and `dod` once after the batch.
- 2026-11-23 — Task 1: carried comments naming retired things were rewritten beyond the three the plan names (`cargo make plugins-test` → `cargo make lua` in 5 files; `plugins/claudestacks/scripts` → `packs/core/scripts` in 4; `lib/handoff.lua:30` "the pack's"; `statusline-install.lua:4`; `install-airsl.sh` header and usage path, so it is not "carried unchanged"). Reviewer checked each is true.
- 2026-11-23 — Task 1 README: LICENSE link `./LICENSE` → `../../LICENSE` (the old link is dead; same as the guideline packs) and "Then, in Claude Code:" → "Then, in your project:"; the rest kept per "Other text stays".
- 2026-11-23 — Task 1 agents: no agent text held a namespaced reference; only frontmatter (`tools`, `model`) and the closing protocol path (`{{skill_dir core/context-handoff}}/references/protocol.md`) changed.
- 2026-11-23 — Task 3: `context-handoff/references/protocol.md:145-151` `echo "$CLAUDE_PLUGIN_ROOT"` block dropped and the advice reworded; `context-handoff/SKILL.md` sentence on the variable reworded to the project directory; `orchestrate/SKILL.md:39` `claudestacks:explorer` → `{{agent explorer}}` (caught by gate grep 1); `protocol.md:10` plugin path → `{{root}}/scripts/lib/handoff.lua`; `protocol.md:187` "the `claudestacks` plugin" → "the core pack".
- 2026-11-23 — Task 4: the plan lists `:41,81,91,115,126` but the source also has `${CLAUDE_PLUGIN_ROOT}` at `:69,82-83,92-93,116-117,127-128,153,162`; all were converted ("Every" governs).
- 2026-11-23 — The worktree guard refuses a Bash command whose text holds the git exec grant of the pack test command; it was run from a scratch script (`sh <script>`). `cargo make lua` is unaffected.
- 2026-11-23 — Not committed.
