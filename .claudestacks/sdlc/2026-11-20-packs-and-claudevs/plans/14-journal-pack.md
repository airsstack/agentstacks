---
status: approved
created: 2026-11-23
depends-on: [03, 10, 11, 13]
built-on: 0f89fdb3ebb3
---

# journal pack Implementation Plan

**Goal:** Ship the claudestacks-journal plugin as pack `journal`.

**Architecture:**
- `packs/journal/` follows spec §6.1. The journal skills and agents lose their `journal-` prefix,
  because the extension adds the pack prefix: skill `capture` renders `/journal-capture` (spec §6.1
  "Renamed items"). Skill `setup` shares its name with sdlc's `setup`, which plan 03 allows. The two
  render as `/journal-setup` and `/sdlc-setup`.
- `requires.packs = ["core"]`: a skill names `{{skill core/context-handoff}}`, and the agent-stop
  hook runs core's `handoff_check_hook.lua` (spec §6.2).
- `session-start.lua` becomes a module. The libraries it loads take the §3 idiom: `index`,
  `orientation` and `vault`. Libraries only skill-run scripts load keep their plain requires.

**Tech Stack:** pack format, Lua under airsl 0.1.5.

**Content authority:** spec §3, §6.1 (journal rows), §6.2 (journal manifest), §6.3 rows H1 and H13,
§6.4 (including `[[wiki links]]` unchanged and the guard-prose row), §6.5 (journal tests, new
`session-start` test), §9 (rename row).

---

## Files that change

```
packs/journal/pack.toml                    — create: §6.2 text
packs/journal/README.md                    — create: plugin README with the install block replaced
packs/journal/agents/{capture,curator,note,recall}.md — create: from agents/journal-*.md, converted
packs/journal/skills/{capture,helped,link,note,recall,review,setup}/SKILL.md — create: from skills/journal-*/, converted
packs/journal/references/note-schema.md    — create: carried, converted
packs/journal/scripts/{build-index,bump-helped,daily-link,graph-health,journal-backup,orientation,project-key,provision,transcript-path}.lua — create: carried unchanged
packs/journal/scripts/session-start.lua    — create: module
packs/journal/scripts/session-start_test.lua — create: new
packs/journal/scripts/lib/{index,orientation,vault}.lua — create: carried; idiom for their requires
packs/journal/scripts/lib/{backup,daily,health,notes}.lua — create: carried unchanged
packs/journal/scripts/{daily,health,index,notes,orientation,vault}_test.lua — create: moved unchanged
```

## Order of work

### Task 1 — Scripts and the session-start module

**Files:**
- Create `packs/journal/pack.toml` (header and `[requires]` only), `scripts/**`.

**Change:**
- `pack.toml` holds the §6.2 journal text up to `[requires]`: `exclude`; `[pack]` with name
  `journal`, version `0.1.0`, api 0 and the §6.2 description; `[requires] packs = ["core"]`,
  `airsl = ">=0.1.5"`.
- `session-start.lua` returns `function(event)` and does what the script does today: provision,
  rebuild a stale index, build the orientation card. It returns `{ context = card }`, or `nil` when
  there is no card. Its requires are `base .. "lib.index"`, `base .. "lib.orientation"` and
  `base .. "lib.vault"`.
- `lib/index.lua` and `lib/orientation.lua` require `base .. "vault"`. `lib/vault.lua` keeps its
  content; it requires nothing.
- `session-start.sh` is not carried (spec §6.1). The usage comment in `session-start.lua:7-11`
  showing an `airsl run` command line is dropped (spec §6.4 comment row).

**Proof:**
- `session-start_test.lua`:
  - `session_start_provisions_an_empty_vault`: with `airsstack.env` swapped for a fake whose
    `AIRSSTACK_HOME` is a temp directory (restored after), a `session.start` event creates `daily`,
    `sessions`, `notes`, `mocs` and `.index` under `<AIRSSTACK_HOME>/journal`
    (`lib/vault.lua:124-131`);
  - `session_start_on_an_empty_vault_returns_nil`: the same call returns `nil`, because no session
    or note matches (spec §1, §6.5). The card itself stays covered by `orientation_test`.
- The six moved tests pass.

**Done when:** `cargo make lua` passes and `cargo make dod` is green. Commit
`feat(repo): journal pack scripts and session-start hook module`.

### Task 2 — Agents, references and skills

**Files:**
- Create `packs/journal/agents/*.md`, `skills/*/SKILL.md`, `references/note-schema.md`, `README.md`.

**Change:**
- Renames as spec §6.1: skill `journal-<x>` → `<x>`, agent `journal-<x>` → `<x>`. Frontmatter
  `name` is the new pack name (`capture`).
- Converted by §6.4:
  - `claudestacks-journal:journal-helped` → `{{skill helped}}`;
  - `claudestacks-journal:journal-setup` → `{{skill setup}}`;
  - `claudestacks:context-handoff` → `{{skill core/context-handoff}}`;
  - in a `SKILL.md`, `${CLAUDE_PLUGIN_ROOT}/<x>` → `{{var project_dir}}/{{root}}/<x>`;
  - in agents and `references/`, `${CLAUDE_PLUGIN_ROOT}/<x>` → `{{root}}/<x>`;
  - `${CLAUDE_SESSION_ID}` → `{{var session_id}}`;
  - `$ARGUMENTS` → `{{var arguments}}`.
- In `journal-{capture,link,note,recall,review}/SKILL.md`, the prose naming `${CLAUDE_PLUGIN_ROOT}`
  as a variable the worktree guard cannot resolve drops that variable and its `echo` line.
- Journal items named without a namespace become tokens (spec §6.4 own-pack row;
  `probes/selfref-results.md` greps 2–4): `/journal-<x>` in descriptions and bodies and in
  `references/note-schema.md:167` → `{{skill <x>}}`; `subagent_type: journal-<x>`, the backticked
  `` `journal-<x>` `` subagent and the unbackticked "journal-<x> subagent" → `{{agent <x>}}`.
  Headings (`# journal-capture`) and the file name `journal-curator-review.md` stay.
- `[[wiki links]]`, "the claudestacks-journal vault" and other product names stay.
- The README install block becomes
  `agentstacks init --tool claude --pack core --pack journal`.

**Proof:**
- In a temp git project with copies of `packs/core`, `packs/journal` and `extensions/claude`,
  `agentstacks init --tool ./extensions/claude --pack ./packs/core --pack ./packs/journal --yes`
  exits 0. It writes `.claude/skills/journal-{capture,helped,link,note,recall,review,setup}/SKILL.md`
  and `.claude/agents/journal-{capture,curator,note,recall}.md`, each with frontmatter `name` equal
  to its file or directory name.
- the three spec §6.4 gate greps, with `packs/` narrowed to `packs/journal`, print nothing (the third applies to journal only).

**Done when:** both checks pass and `cargo make dod` is green. Commit `feat(repo): journal pack
skills, agents and references`.

### Task 3 — journal hooks

**Files:**
- Modify `packs/journal/pack.toml`.

**Change:** Append the two §6.2 journal `[[hooks]]` tables:
- `session.start` `match = ["startup", "resume", "clear"]` running `scripts/session-start.lua`;
  grants `env = ["AIRSSTACK_HOME", "HOME"]`, `read = ["{{var airsstack_home}}",
  "{{var project_dir}}"]`, `write = ["{{var airsstack_home}}"]`;
- `agent.stop`, `agents = ["curator"]`, running `{{root core}}/scripts/handoff_check_hook.lua`,
  `may_block = true`, with grants `env = ["HOME", "TMPDIR"]` and `read = ["/"]`.

The file now equals the §6.2 journal text.

**Proof:**
- After the Task 2 `init`, `settings.json` holds a `SubagentStop` group with matcher
  `journal-curator` and a `SessionStart` group with matcher `startup|resume|clear` running
  `journal.scripts.session-start`.
- Piping a `SessionStart` payload into that rendered command, with `AIRSSTACK_HOME` set to a temp
  directory, prints a `hookSpecificOutput.additionalContext` holding the card. This shows the
  idiom loads `journal.scripts.lib.index` under the runtime.
- With core, sdlc and journal together, `init` exits 0. Both `.claude/skills/sdlc-setup/` and
  `.claude/skills/journal-setup/` are written.

**Done when:** these checks pass and `cargo make dod` is green. Commit `feat(repo): journal pack
hooks`.

---

## Risks

- The module name `journal.scripts.session-start` has a hyphen. airsl accepts ASCII letters,
  digits, underscores, hyphens and dots in a `require` target
  (`~/.cargo/registry/src/*/airsl-0.1.4/src/types/require_target.rs`, `RequireTarget::new`). The
  idiom's pattern drops everything after the last dot, hyphens included, so `base` is
  `journal.scripts.`. The Task 3 piped run is the runtime check.
- `lib/vault.lua` notes that every script in `scripts/` requires it. Its comment stays true.

## Proof

- `cargo make dod` green; a core + sdlc + journal `init` exits 0.
