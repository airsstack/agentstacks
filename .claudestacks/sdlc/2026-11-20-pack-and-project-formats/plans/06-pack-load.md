---
status: draft
created: 2026-11-20
depends-on: [03, 04, 05]
built-on: e665589a43a9
---

# Pack Loading Implementation Plan

**Goal:** `Pack::load(dir)` turns a pack directory into a checked `Pack` or every refusal it
contains.

**Architecture:** `Pack::load` walks the directory once (`walk`), parses `pack.toml`
(`PackManifest`), classifies every file as skill, agent, native or "anything else", parses
frontmatter for skills and agents, parses each `native/<tool>/hooks.toml`, applies `exclude`,
then scans every non-excluded `.md` and every `render` file for tokens. It applies only rules
that need this pack's own files; targets in other packs, option tokens, hook `run` targets and
`agents` are checked in the set check (plan 10), which needs every pack. The result `Pack` holds
the manifest, skills (name, meta, files), agents, native files per tool, installable files with
their tokens, and the ignored dot entries for `plan`. `PackError` joins `FormatError`. Fixtures
live under `crates/agentstacks/tests/fixtures/packs/`, one directory per case, driven by the
integration test `crates/agentstacks/tests/pack_load.rs`.

**Tech Stack:** Rust 2024, `globset` 0.4 (from plan 05), `tempfile` 3 (dev, from plan 03).

**Content authority:** spec §2 (layout, §2.1 rules, §2.2 native rules), §3.4 (where tokens are
rendered; `render` UTF-8 rule), §3.1 (var `arguments` placement), §4.2 (`render`/`exclude`
existence and protected files; `requires.airsl` for non-excluded `.lua`), §12 (fixtures).

---

## Files that change

```
crates/agentstacks/src/model/mod.rs         — modify  declare and re-export
crates/agentstacks/src/model/pack.rs        — create  Pack, Pack::load
crates/agentstacks/src/model/pack_layout.rs — create  classify(files): skills, agents, native, reserved, other
crates/agentstacks/src/model/pack_files.rs  — create  exclude/render checks; token scan of rendered files
crates/agentstacks/src/model/pack_error.rs  — create  PackError (thiserror)
crates/agentstacks/src/model/diagnostic.rs  — modify  FormatError gains a Pack variant
crates/agentstacks/tests/pack_load.rs       — create  integration test over the fixtures
crates/agentstacks/tests/fixtures/packs/**  — create  one valid pack per feature, one invalid pack per refusal
```

## Order of work

### Task 1 — Classify the pack layout

**Files:**
- Create `pack_layout.rs`, `pack_error.rs`; modify `mod.rs`, `diagnostic.rs`.

**Change:** from the walked file list: `pack.toml` must exist (`<dir>: not a pack (no
pack.toml)`); any file under `commands/` or `rules/` is refused once per directory (`<pack>/commands:
reserved, not supported in api 0`); each `skills/<name>/` must hold `SKILL.md` (`<pack>/skills/<name>:
missing SKILL.md`) and its name must be a valid `ItemName`; `agents/<name>.md` files are agents;
each `native/<tool>/` must have a valid `ToolName` (`native: "Claude" is not a valid tool name`);
everything else is "other" and goes under the install root. Skill and agent names come from the
directory or file stem and are passed to `SkillMeta::parse` / `AgentMeta::parse`.

**Proof:**
- `layout_classifies_skills_agents_native_and_other` — a fixture with `skills/a/SKILL.md`,
  `skills/a/references/r.md`, `agents/b.md`, `native/claude/x.json`, `scripts/s.lua` is
  classified accordingly; `references/r.md` belongs to skill `a`.
- `layout_refuses_each_rule` — table-driven: missing `pack.toml`, `commands/x.md`, `rules/x.md`,
  `skills/a/` without `SKILL.md`, `native/Claude/`; exact messages.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): classify pack layout`.

### Task 2 — Apply `exclude` and `render`

**Files:**
- Create `pack_files.rs`; modify `pack_error.rs`, `mod.rs`.

**Change:** `exclude` may not match `pack.toml`, any `SKILL.md`, any agent file, a hook `run`
target in this pack, or a `render` file (`exclude: "**/*.md" excludes skills/discuss/SKILL.md`).
Each `render` path must exist (`render: "enforcment.json" does not exist`) and not be excluded
(`render: "x.json" is also excluded`). Excluded files are dropped from the installable set and
are not scanned. A pack with any non-excluded `.lua` file and no `requires.airsl` is refused
(`<pack>: contains .lua files but declares no requires.airsl`). After exclusion, a pack with no
skill, agent or hook is refused (`<pack>: pack declares nothing to install`).

**Proof:**
- `exclude_drops_files_from_install_and_scan` — a fixture excluding `**/*_test.lua` has no test
  file in its installable set; a `{{bad}}` token inside an excluded `.md` produces no error.
- `exclude_and_render_refuse_each_rule` — table-driven over the §4.2 `render`/`exclude` rows and
  the protected-file rule: excluding `pack.toml`, a `SKILL.md`, an agent file, a hook `run`
  target, a `render` file; a missing `render` path; exact messages.
- `airsl_requirement_counts_only_installed_lua` — a pack whose only `.lua` files are excluded
  loads without `requires.airsl`; the same pack with one installed `.lua` is refused.
- `pack_with_nothing_to_install_is_refused` — only `scripts/x.md` and `pack.toml`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): apply pack exclude and render rules`.

### Task 3 — Scan rendered files and load the pack

**Files:**
- Create `pack.rs`; modify `pack_files.rs`, `pack_error.rs`, `mod.rs`.

**Change:** `Pack::load(dir)` runs Tasks 1–2, parses frontmatter for every skill and agent,
parses each `native/<tool>/hooks.toml` with `NativeHooks::parse`, and scans every non-excluded
`.md` and `render` file (neutral and native alike) with `scan`, keeping tokens with their file and
line. Pack-local token rules apply here: a rendered file that is not valid UTF-8 is refused
(`<file>: not valid UTF-8, cannot render`); `{{var arguments}}` outside a `SKILL.md` is refused
(`<file>:<line>: var "arguments" is only valid in SKILL.md`); in neutral content (outside
`native/`) a `root` token followed by `/native/` is refused (`<file>:<line>: neutral content must
not reference native/`). Native `.md` files are scanned but not parsed as §5 frontmatter.
`hooks[].agents` must name agents of this pack (`hooks[3].agents: pack "core" has no agent
"task-briefer"`). Ignored dot entries are kept on the `Pack` for `plan`. All diagnostics from all
steps are returned together, sorted.

**Proof:**
- `load_accepts_each_feature_fixture` — valid fixtures for: both hook kinds, native hooks,
  options, a `native/claude/` file, `render` + `exclude`, `skill_dir` and `root` tokens, a skill
  and an agent sharing a name; each loads without diagnostics.
- `load_refuses_each_pack_local_token_rule` — fixtures for non-UTF-8 render content,
  `{{var arguments}}` in an agent file, `{{root}}/native/claude/x` in a neutral skill, an unknown
  own agent in `hooks[].agents`; exact messages and lines.
- `load_does_not_hold_native_md_to_frontmatter_rules` — `native/claude/agents/x.md` with Claude
  keys (`model: sonnet`) loads.
- `load_reports_errors_from_every_file_in_one_run` — a fixture with a bad `pack.toml` field, a bad
  frontmatter key and a bad token returns three sorted diagnostics.
- `load_keeps_ignored_dot_entries` — `.DS_Store` appears in `Pack::ignored`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): load and check pack directories`.

---

## Risks

- The exclude-protection check needs hook `run` targets that are pack-relative or
  `{{root}}/…`; targets in other packs (`{{root core}}/…`) are skipped here and resolved in plan
  10.
- Fixture count is large; keep one directory per case with a short name matching its test row.

## Proof

- `cargo make dod-crate agentstacks` green; every §2.1, §2.2 and pack-local §3/§4.2 refusal has a
  fixture or table case asserting its message.
