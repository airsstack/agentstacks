---
status: approved
created: 2026-11-20
depends-on: [02, 03, 04, 05]
built-on: 3162a314af79
---

# Pack Loading Implementation Plan

**Goal:** `Pack::load(dir)` turns a pack directory into a checked `Pack` or every refusal it
contains.

**Architecture:** `Pack::load` (`model/pack.rs`) only orchestrates. Each rule family lives in its
own file with its own colocated tests:

```text
walk(dir)                       plan 03: files + ignored dot entries, or walk refusals (load stops)
PackLayout::classify            §2.1/§2.2 layout: pack.toml, reserved dirs, skills/, agents/, native/
PackManifest::parse             plan 05: pack.toml (a syntax or field failure stops only that file)
NativeHooks::parse              plan 05: each native/<tool>/hooks.toml
HookTargets::collect / check    §6.2 own-pack run and agents
InstallSet::resolve             §4.2 exclude and render; which files install and which render
check_airsl_declared            §4.2 requires.airsl when a non-excluded .lua exists
check_something_to_install      §2.1 at least one skill, agent, hook or native hook entry
SkillMeta / AgentMeta::parse    plan 04: frontmatter, its tokens, name = dir/stem, body start line
rendered_text + scan_body       §3.4 UTF-8 rule; body and render scans (plan 02's scan)
check_native_references         §2.2 no root token followed by /native/ in neutral content
check_local_tokens              §3.2 arguments placement; §3.5 own-pack targets
```

Every token found is kept on the `Pack` as a `LocatedToken` (plan 02): frontmatter tokens from
plan 04, hook-field tokens from plan 05 (`pack.toml` and `native/<tool>/hooks.toml`), and body and
`render` tokens from this plan. Each text is scanned once (§3.5): the loader scans only Markdown
bodies (after the closing `---` for `SKILL.md` and `agents/<name>.md`, at the body start line plan
04 returns; from line 1 for every other `.md`, native `.md` included, since §2.2 exempts native
files from §5) and `render` files. The pack-local rules run over every kept token.

Boundaries. Own-pack token targets (no pack prefix, or this pack's name) are checked here (§3.5);
targets in other packs, option-token existence (§9 item 3) and other-pack hook `run` (§9 item 4)
belong to `pack_set::check` (plan 10). §3.2's other option rows (`"a-b" is not a valid option
name`, `option tokens name this pack's options only`) are syntax refusals from plan 02's `scan`.
The `native/cluade: not for claude` line is printed by `plan` (§2.2); this plan supplies the data
through `Pack::unused_native`.

When `pack.toml` is refused, the layout rules, frontmatter and the bodies of `SKILL.md` and agent
files (which `exclude` can never remove, §4.2) still run, so every refusal in those files is
reported. Everything that needs the manifest (native hooks, `exclude`, `render`, hook targets, the
airsl rule, "nothing to install", other bodies, pack-local token rules) is skipped, because without
`exclude` it would report on files the manifest may exclude.

`PackError` is this module's `thiserror` enum and joins `FormatError` (plan 01). A file that cannot
be read is reported through `FormatError`'s single I/O variant (`<path>: <io error>`, §10). The
result is `Ok(Pack)` when the run found no refusal, else every `Diagnostic` sorted per §10 through
plan 01's `Diagnostics`.

`<pack>` and `<dir>` in spec messages are the directory passed to `Pack::load` (§10:
`<pack>/pack.toml: <key.path>`).

**Tech Stack:** Rust 2024; `globset` 0.4 (added by plan 05), `tempfile` 3 (dev-dependency, added
by plan 03). No new dependency.

**Content authority:** spec §2 and §2.1 (layout rules), §2.2 (`native/<tool>/`), §3.2 (`arguments`
placement row; own-pack rows of `pack "core" has no skill "coder"`), §3.3 (`pack "core" has no
agent "codr"` for own-pack targets), §3.4 (where tokens render; UTF-8 rule), §3.5 (scan once; tokens
kept on `Pack`; pack-local rules), §4.2 (`render`, `exclude`, `requires.airsl` rows; excluded files
not scanned), §6.2 (own-pack `run`, `agents`), §10 (locations, sort order), §12 (fixtures,
coverage rule).

---

## Files that change

```
crates/agentstacks/src/model/mod.rs                  — modify  declare and re-export the new items
crates/agentstacks/src/model/format_error.rs         — modify  FormatError gains a Pack variant
crates/agentstacks/src/model/pack_error.rs           — create  PackError (thiserror), spec messages verbatim
crates/agentstacks/src/model/pack_layout.rs          — create  PackLayout::classify: §2.1/§2.2 layout rules
crates/agentstacks/src/model/hook_targets.rs         — create  HookTargets: own-pack run paths and agents
crates/agentstacks/src/model/install_set.rs          — create  InstallSet::resolve: exclude, render, protection
crates/agentstacks/src/model/native_dir.rs           — create  NativeDir: one native/<tool>/ (files, hooks)
crates/agentstacks/src/model/airsl_requirement.rs    — create  check_airsl_declared
crates/agentstacks/src/model/nothing_to_install.rs   — create  check_something_to_install
crates/agentstacks/src/model/rendered_text.rs        — create  rendered_text: bytes → UTF-8 text or refusal
crates/agentstacks/src/model/body_scan.rs            — create  scan_body: scan from a start line → LocatedTokens
crates/agentstacks/src/model/native_reference.rs     — create  check_native_references
crates/agentstacks/src/model/local_token_rules.rs    — create  check_local_tokens: placement, own-pack targets
crates/agentstacks/src/model/pack_skill.rs           — create  PackSkill: one skill (meta, files)
crates/agentstacks/src/model/pack.rs                 — create  Pack, Pack::load, Pack::unused_native
crates/agentstacks/tests/pack_load.rs                — create  integration test over the fixtures
crates/agentstacks/tests/fixtures/packs/**           — create  one valid pack per feature, one invalid pack per refusal
```

## Order of work

### Task 1 — Classify the pack layout

**Files:**
- Create `crates/agentstacks/src/model/pack_error.rs` — `PackError` with one variant per refusal
  this plan owns, each message copied from the spec: `not a pack (no pack.toml)`; `reserved, not
  supported in api 0`; `missing SKILL.md`; `only skill directories belong in skills/`; `agents/
  holds only <name>.md files`; `only tool directories belong in native/`; `"<path>" does not
  exist`; `"<path>" is also excluded`; `"<glob>" excludes <path>`; `required (the pack contains .lua
  files)`; `pack declares nothing to install`; `not valid UTF-8, cannot
  render`; `neutral content must not reference native/`; `pack "<pack>" has no <skill|agent> "<item>"`. The invalid-name refusal is not
  redeclared: it wraps the error plan 02's name newtypes return (`"<value>" is not a valid name`).
- Create `crates/agentstacks/src/model/pack_layout.rs` — `PackLayout` and
  `PackLayout::classify(dir, files)`, taking the walked relative paths (plan 03) and returning the
  layout plus its `Diagnostic`s.
- Modify `crates/agentstacks/src/model/format_error.rs` — `FormatError::Pack(#[from] PackError)`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** `classify` sorts every walked file into exactly one group: `pack.toml`; skills
(`ItemName` → the skill's files, `SKILL.md` included); agents (`ItemName` → `agents/<name>.md`);
native (`ToolName` → its files, with `native/<tool>/hooks.toml` held apart as the tool's hook file,
never an installed file); root (every other file, installed under `{{root}}`, §2). Refusals, each
located at a path under `dir` with no line:
- no `pack.toml` at the root → `<dir>: not a pack (no pack.toml)`;
- any file under `commands/` or `rules/` → `<dir>/commands: reserved, not supported in api 0`
  (same for `rules`), once per directory;
- a file directly in `skills/` → `<dir>/skills/<file>: only skill directories belong in skills/`;
- a `skills/<name>/` whose name fails `ItemName` → `<dir>/skills/<name>: "<name>" is not a valid
  name`; that directory is not classified further, so it gets no follow-on refusal;
- a `skills/<name>/` without `SKILL.md` → `<dir>/skills/<name>: missing SKILL.md`;
- a file in `agents/` that does not end in `.md`, or any subdirectory of `agents/` →
  `<dir>/agents/<entry>: agents/ holds only <name>.md files` (a subdirectory once, at its own
  path);
- an `agents/<stem>.md` whose stem fails `ItemName` → `<dir>/agents/<stem>.md: "<stem>" is not a
  valid name`;
- a file directly in `native/` → `<dir>/native/<file>: only tool directories belong in native/`;
- a `native/<tool>/` whose name fails `ToolName` → `<dir>/native/<tool>: "<tool>" is not a valid
  name`.
A skill and an agent with the same name are both kept (§2.1).

**Proof:**
- `pack_error_messages_match_the_spec` (in `pack_error.rs`) — table: each variant's `Display`
  equals its spec text, e.g. `not a pack (no pack.toml)`, `"enforcment.json" does not exist`,
  `"x.json" is also excluded`, `"**/*.md" excludes skills/discuss/SKILL.md`, `required (the pack
  contains .lua files)`, `pack "core" has no skill "coder"`, `pack "core" has no agent
  "codr"`, `pack "core" has no agent "task-briefer"`, and every other message listed under Files.
- `classify_sorts_files_into_skills_agents_native_and_root` — `pack.toml`, `skills/a/SKILL.md`,
  `skills/a/references/r.md`, `agents/b.md`, `native/claude/x.json`, `native/claude/hooks.toml`,
  `scripts/s.lua`, `README.md` give skill `a` with `SKILL.md` and `references/r.md`, agent `b`,
  native `claude` with `x.json` as a file and `hooks.toml` as its hook file, root `README.md` and
  `scripts/s.lua`; `pack.toml` is in no installable group; no diagnostic.
- `classify_refuses_a_directory_without_pack_toml` — `skills/a/SKILL.md` alone gives exactly
  `<dir>: not a pack (no pack.toml)`.
- `classify_reports_each_reserved_directory_once` — `commands/a.md`, `commands/b/c.md` and
  `rules/x.md` give exactly two diagnostics: `<dir>/commands: reserved, not supported in api 0`
  and `<dir>/rules: reserved, not supported in api 0`.
- `classify_refuses_a_skill_directory_without_skill_md` — `skills/a/references/r.md` gives
  `<dir>/skills/a: missing SKILL.md`.
- `classify_refuses_files_directly_in_skills` — `skills/README.md` gives
  `<dir>/skills/README.md: only skill directories belong in skills/`.
- `classify_refuses_invalid_skill_directory_names` — `skills/My_Skill/notes.md` gives only
  `<dir>/skills/My_Skill: "My_Skill" is not a valid name` (no `missing SKILL.md`).
- `classify_refuses_non_md_files_and_subdirectories_in_agents` — `agents/notes.txt` gives
  `<dir>/agents/notes.txt: agents/ holds only <name>.md files`; `agents/x/y.md` and `agents/x/z.md`
  give one `<dir>/agents/x: agents/ holds only <name>.md files`.
- `classify_refuses_invalid_agent_stems` — `agents/Coder.md` gives `<dir>/agents/Coder.md:
  "Coder" is not a valid name`.
- `classify_refuses_files_directly_in_native` — `native/README.md` gives
  `<dir>/native/README.md: only tool directories belong in native/`.
- `classify_refuses_invalid_native_tool_names` — `native/Claude/x.json` gives
  `<dir>/native/Claude: "Claude" is not a valid name`.
- `classify_keeps_a_skill_and_an_agent_with_the_same_name` — `skills/x/SKILL.md` and
  `agents/x.md` give skill `x` and agent `x`, no diagnostic.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): classify pack layout`.

### Task 2 — Check own-pack hook targets

**Files:**
- Create `crates/agentstacks/src/model/hook_targets.rs` — `HookTargets`:
  `HookTargets::collect(pack name, manifest, native hooks)` gathers every own-pack `run` (with its
  file and key path) and every `agents` entry; `run_paths()` lists the resolved own-pack run files
  (Task 3 protects them from `exclude`); `check(layout)` returns the refusals.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** a `run` (plan 05 has already checked its form, §6.2) is own-pack when it has no
leading token (pack-relative `hooks/x.lua`), or its leading `root` token (the hook-field
`LocatedToken` at that `run` key path, plan 05) names no pack or this pack. Its path is the
pack-relative text, or the text after the token's `}}/`. `{{root}}` holds only the root files
(§2), so the path must be an existing, non-excluded root file of the layout (not `pack.toml`, not
under `skills/`, `agents/` or `native/`, §6.2); otherwise
`<dir>/pack.toml: hooks[i].run: "<run as written>" does not exist` (for a native hook,
`<dir>/native/<tool>/hooks.toml: hooks[i].run: …`). A `run` whose `root` token names another pack
is left to plan 10. An excluded `run` gets only Task 3's `exclude` refusal (§4.2 protects run
targets from `exclude`); the file exists, so no `does not exist` refusal is added, matching the
`exclude-protected-run` fixture. Each `agents` entry of a
`pack.toml` hook must name an agent of the layout:
`<dir>/pack.toml: hooks[i].agents: pack "<this pack>" has no agent "<name>"`.

**Proof:**
- `own_runs_resolve_in_all_three_forms` — in pack `core`, `hooks/a.lua`, `{{root}}/hooks/a.lua`
  and `{{root core}}/hooks/a.lua` all resolve to `hooks/a.lua` in `run_paths()`; with that root
  file present, no diagnostic.
- `other_pack_runs_are_left_to_the_set_check` — `{{root guideline}}/hooks/x.lua` in pack `core`
  is absent from `run_paths()` and gives no diagnostic though no such file exists.
- `missing_own_run_is_refused` — table: `hooks/missing.lua` as the second hook gives
  `<dir>/pack.toml: hooks[1].run: "hooks/missing.lua" does not exist`; `{{root}}/hooks/missing.lua`
  as the first gives `<dir>/pack.toml: hooks[0].run: "{{root}}/hooks/missing.lua" does not exist`.
- `run_naming_a_skill_file_does_not_exist_under_root` — `skills/a/x.lua` exists in skill `a`;
  `run = "skills/a/x.lua"` gives `<dir>/pack.toml: hooks[0].run: "skills/a/x.lua" does not exist`.
- `run_naming_a_native_file_does_not_exist_under_root` — `native/claude/x.lua` exists;
  `run = "native/claude/x.lua"` gives `<dir>/pack.toml: hooks[0].run: "native/claude/x.lua" does
  not exist`.
- `native_hook_runs_are_checked_as_own_pack_runs` — a missing run in `native/claude/hooks.toml`
  gives `<dir>/native/claude/hooks.toml: hooks[0].run: "hooks/missing.lua" does not exist`.
- `hook_agents_must_name_own_agents` — pack `core` with agent `coder`; `agents = ["coder",
  "task-briefer"]` on the fourth hook gives exactly `<dir>/pack.toml: hooks[3].agents: pack "core"
  has no agent "task-briefer"`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): check own-pack hook targets`.

### Task 3 — Apply `exclude` and `render`

**Files:**
- Create `crates/agentstacks/src/model/install_set.rs` — `InstallSet` and
  `InstallSet::resolve(manifest file, layout, manifest, protected run paths)`, returning the set
  of excluded files, the files to install and the files to render, plus refusals.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** each `exclude` pattern (plan 05 compiled and validated it) is matched against every
walked pack-relative path. Excluded files are not installed and not scanned (§4.2). Rendered files
are every non-excluded `.md` plus every `render` entry (§3.4); `pack.toml` and
`native/<tool>/hooks.toml` are never installed. Refusals, located at `<dir>/pack.toml` with key
path `render` or `exclude`:
- a `render` entry that is not a walked file → `render: "<path>" does not exist`;
- a `render` entry that an `exclude` pattern matches → `render: "<path>" is also excluded`;
- a pattern matching `pack.toml`, a `SKILL.md`, an agent file or a `run_paths()` file →
  `exclude: "<glob>" excludes <path>`, once per pattern and file — except for a file that is also
  listed in `render`, which gets only the `render` refusal (§4.2).

**Proof:**
- `exclude_drops_matching_files_from_install_and_render` — `**/*_test.lua` drops
  `scripts/a_test.lua` and `skills/a/scripts/b_test.lua` and keeps `scripts/a.lua`; `docs/**`
  drops `docs/x.md` from the rendered set.
- `rendered_set_holds_each_installed_md_and_each_render_file` — `SKILL.md`, `agents/b.md`,
  `README.md`, `native/claude/x.md` and `render = ["enforcement.json"]` are rendered;
  `scripts/a.lua` and `native/claude/hooks.toml` are not.
- `render_entry_must_exist` — `render = ["enforcment.json"]` with no such file gives
  `<dir>/pack.toml: render: "enforcment.json" does not exist`.
- `render_entry_must_not_be_excluded` — `render = ["x.json"]`, `exclude = ["*.json"]` gives
  exactly `<dir>/pack.toml: render: "x.json" is also excluded`.
- `exclude_must_not_match_protected_files` — table, each asserting the one diagnostic:
  `"pack.toml"` → `<dir>/pack.toml: exclude: "pack.toml" excludes pack.toml`; `"**/*.md"` over a
  pack whose only `.md` is `skills/discuss/SKILL.md` → `exclude: "**/*.md" excludes
  skills/discuss/SKILL.md`; `"agents/*"` → `exclude: "agents/*" excludes agents/coder.md`;
  `"hooks/**"` with `hooks/enforce.lua` in `run_paths()` → `exclude: "hooks/**" excludes
  hooks/enforce.lua`.
- `exclude_matching_a_rendered_protected_file_reports_only_the_render_refusal` —
  `render = ["agents/coder.md"]`, `exclude = ["agents/*"]` gives exactly
  `<dir>/pack.toml: render: "agents/coder.md" is also excluded` and no `exclude` refusal.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): apply pack exclude and render rules`.

### Task 4 — Check what the pack installs

**Files:**
- Create `crates/agentstacks/src/model/native_dir.rs` — `NativeDir`: one `native/<tool>/`, its
  installed files and its parsed `NativeHooks` (plan 05) when `hooks.toml` exists and is not
  excluded; `hook_count()`.
- Create `crates/agentstacks/src/model/airsl_requirement.rs` — `check_airsl_declared(pack name,
  manifest, install set)`.
- Create `crates/agentstacks/src/model/nothing_to_install.rs` —
  `check_something_to_install(dir, layout, manifest, native dirs)`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** a pack with any installed (non-excluded) `.lua` file anywhere — root, skill or native
— and no `requires.airsl` is refused at `<dir>/pack.toml`, key path `requires.airsl`:
`<dir>/pack.toml: requires.airsl: required (the pack contains .lua files)` (§4.2, location per §10). A pack with no
skill, no agent, no `pack.toml` hook and no entry in a non-excluded `native/<tool>/hooks.toml` is
refused: `<dir>: pack declares nothing to install` (§2.1). Skills and agents always count:
`exclude` cannot remove `SKILL.md` or an agent file (Task 3).

**Proof:**
- `native_dir_counts_hook_entries` (in `native_dir.rs`) — no hook file gives 0; a hook file with
  two `[[hooks]]` gives 2.
- `installed_lua_requires_requires_airsl` — pack `sdlc` with `scripts/x.lua` and no
  `requires.airsl` gives exactly `<dir>/pack.toml: requires.airsl: required (the pack contains .lua
  files)`.
- `lua_in_a_skill_or_native_dir_counts` — the only `.lua` is `skills/a/scripts/x.lua` (then,
  separately, `native/claude/x.lua`): the same refusal.
- `excluded_lua_needs_no_airsl` — the only `.lua` files are excluded: no diagnostic; with
  `requires.airsl = ">=0.1.4"` and an installed `.lua`: no diagnostic.
- `pack_with_only_root_files_has_nothing_to_install` — `pack.toml` and `scripts/x.md` give
  `<dir>: pack declares nothing to install`.
- `each_installable_kind_counts` — table: one skill; one agent; one `pack.toml` hook; one entry in
  `native/claude/hooks.toml`; each gives no diagnostic.
- `excluded_or_empty_native_hooks_do_not_count` — a `native/claude/hooks.toml` that is excluded,
  and one with no `[[hooks]]`, each in an otherwise empty pack give
  `<dir>: pack declares nothing to install`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): require installable content and airsl for lua`.

### Task 5 — Scan bodies and apply the pack-local token rules

**Files:**
- Create `crates/agentstacks/src/model/rendered_text.rs` — `rendered_text(file, bytes)`: the
  UTF-8 text, or `<file>: not valid UTF-8, cannot render`.
- Create `crates/agentstacks/src/model/body_scan.rs` — `scan_body(file, text, start_line)`: runs
  plan 02's `scan` and returns `LocatedToken`s and token-error `Diagnostic`s located at
  `<file>:<line>`, the line counted from the top of the file.
- Create `crates/agentstacks/src/model/native_reference.rs` — `check_native_references(dir,
  tokens)`: every kept `LocatedToken` whose file is not under `<dir>/native/` and whose
  `token().reaches_native()` (plan 02: a `root` token followed by `/native/`, recorded by the
  scanner) is true gives `neutral content must not reference native/` through
  `LocatedToken::refuse`, at the token's own location: `<file>:<line>` in bodies and `render`
  files, `<file>: <key.path>` in frontmatter values and hook fields (§2.2, §10).
- Create `crates/agentstacks/src/model/local_token_rules.rs` — `check_local_tokens(pack name,
  layout, tokens)`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** `check_local_tokens` runs over `LocatedToken`s from any source: `{{var arguments}}`
located in a file not named `SKILL.md` (Markdown line or TOML key path alike) gives
`var "arguments" is only valid in SKILL.md` (§3.2); a `skill` or `skill_dir` token with no pack or
this pack's name must name a skill of the layout, an `agent` token an agent, else
`pack "<this pack>" has no skill "<item>"` / `has no agent "<item>"` (§3.2, §3.3), at the token's
location. Tokens naming another pack are left to plan 10.

**Proof:**
- `rendered_text_refuses_invalid_utf8` — bytes `0x66 0xFF` for `<dir>/data.json` give
  `<dir>/data.json: not valid UTF-8, cannot render`; `"a\n"` gives the text.
- `scan_body_counts_lines_from_the_file_top` — a body starting at line 5 with
  `{{skill status}}` on its second line gives a `LocatedToken` at `<file>:6`.
- `scan_body_locates_token_errors` — `{{skil x}}` on the first body line, start line 6, gives
  `<file>:6: unknown token kind "skil"`.
- `native_reference_is_refused_after_a_root_token` — table: `{{root}}/native/claude/x` and
  `{{root core}}/native/x` on line 3 give `<file>:3: neutral content must not reference native/`;
  `{{root}}/natives/x`, `{{root}}/hooks/native/x` and `{{skill_dir a}}/native/x` give nothing.
- `native_reference_in_frontmatter_keeps_its_key_path` — a skill `description: "see
  {{root}}/native/claude/x"` gives `<dir>/skills/a/SKILL.md: description: neutral content must not
  reference native/`.
- `native_reference_in_a_hook_field_keeps_its_key_path` — a `pack.toml` arg
  `{{root}}/native/claude/x` gives `<dir>/pack.toml: hooks[0].args[0]: neutral content must not
  reference native/`.
- `arguments_var_is_refused_outside_skill_md` — table: a token at `<dir>/agents/b.md:6` gives
  `<dir>/agents/b.md:6: var "arguments" is only valid in SKILL.md`; one at
  `<dir>/pack.toml: hooks[0].args[0]` gives `<dir>/pack.toml: hooks[0].args[0]: var "arguments"
  is only valid in SKILL.md`.
- `arguments_var_is_accepted_in_skill_md` — tokens at `<dir>/skills/a/SKILL.md:7` and at
  `<dir>/skills/a/SKILL.md: argument_hint` give nothing.
- `own_pack_targets_must_exist` — table in pack `core` with skill `discuss` and agent `coder`:
  `{{skill_dir core/coder}}` gives `pack "core" has no skill "coder"`; `{{agent codr}}` gives
  `pack "core" has no agent "codr"`; `{{skill status}}` gives `pack "core" has no skill
  "status"`; each prefixed with its `<file>:<line>`.
- `existing_own_targets_and_other_pack_targets_pass` — `{{skill discuss}}`,
  `{{skill_dir core/discuss}}`, `{{agent coder}}`, `{{root}}` and `{{skill sdlc/status}}` give
  nothing.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): scan pack bodies and apply pack-local token rules`.

### Task 6 — Load the pack

**Files:**
- Create `crates/agentstacks/src/model/pack_skill.rs` — `PackSkill`: one skill's `SkillMeta`
  (plan 04) and its installed files, listed relative to the skill directory.
- Create `crates/agentstacks/src/model/pack.rs` — `Pack` (directory, manifest, skills, agents,
  native dirs, root files, rendered files, tokens, ignored dot entries) and `Pack::load(dir)`;
  `Pack::unused_native(tools)` lists the `native/<tool>/` names no given tool claims.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** `Pack::load(dir)`, in order: `walk` (a walk refusal returns those refusals and stops);
`PackLayout::classify` (no `pack.toml` stops); `PackManifest::parse` on `pack.toml`; each
`SKILL.md` and agent file through `rendered_text`, then `SkillMeta::parse` / `AgentMeta::parse`
with the directory name or file stem as the expected name (plan 04 refuses `<file>: name "x" does
not match "y"`), then `scan_body` from the body start line; a file whose frontmatter is refused is
not body-scanned. With a manifest: `NativeHooks::parse` on each `native/<tool>/hooks.toml`;
`HookTargets`; `InstallSet::resolve`; `check_airsl_declared`; `check_something_to_install`;
`rendered_text` + `scan_body` from line 1 on every other rendered file; `check_native_references`
over every kept token (it reads the flag plan 02 records, so no text is re-read); `check_local_tokens` over all
tokens. Every diagnostic from every step is collected and returned sorted (§10); with none, the
`Pack` holds every `LocatedToken` from frontmatter, hook fields, bodies and `render` files, and
`walk`'s ignored dot entries for `plan`.

**Proof:**
- `pack_skill_lists_files_relative_to_its_directory` (in `pack_skill.rs`) — files
  `skills/a/SKILL.md` and `skills/a/references/r.md` are listed as `SKILL.md`,
  `references/r.md`.
- `load_returns_every_part_of_a_valid_pack` (tempdir) — a skill, an agent, `native/claude/x.json`,
  `scripts/s.lua` with a lua hook running it and `requires.airsl` give a `Pack` whose skills,
  agents, native dirs and root files hold exactly those; `pack.toml` is in none.
- `load_keeps_tokens_from_every_source` — tokens in a skill's quoted `description`, its body
  (line 6), a `render` file (line 1), a `pack.toml` hook arg and a `native/claude/hooks.toml` arg
  all appear in `Pack::tokens` at `<file>: description`, `<file>:6`, `<render file>:1`,
  `<dir>/pack.toml: hooks[0].args[0]` and `<dir>/native/claude/hooks.toml: hooks[0].args[0]`.
- `load_scans_each_text_once` — a quoted `description: "{{skil x}}"` gives exactly one
  diagnostic, the frontmatter one plan 04 locates by key; none from the body scan.
- `load_does_not_scan_excluded_files` — `{{bad}}` inside an excluded `docs/x.md` gives no
  diagnostic, and `docs/x.md` is not among the rendered files.
- `load_does_not_hold_native_md_to_frontmatter_rules` — `native/claude/agents/x.md` with
  frontmatter `model: sonnet` loads.
- `load_allows_native_content_to_reference_native` — `{{root}}/native/claude/x` inside
  `native/claude/x.md` gives no diagnostic.
- `load_stops_at_a_missing_pack_toml` — gives exactly `<dir>: not a pack (no pack.toml)`.
- `load_continues_past_a_refused_pack_toml` — `pack.toml` holding only `a = [1,` (spec §1.1), an
  unknown frontmatter key `disable-model-invocation` in `skills/a/SKILL.md` and `{{skil x}}` on
  line 6 of `agents/b.md` give exactly three diagnostics, in this order:
  `<dir>/agents/b.md:6: unknown token kind "skil"`, `<dir>/pack.toml:1: unclosed array, expected
  ']'`, `<dir>/skills/a/SKILL.md: unknown key "disable-model-invocation"`; no manifest-dependent
  refusal appears.
- `load_skips_the_body_of_a_refused_frontmatter` — a `SKILL.md` with a YAML syntax error and
  `{{skil x}}` in its body gives only the YAML diagnostic.
- `load_refuses_a_non_utf8_render_file` — `render = ["data.json"]` holding bytes `0x66 0xFF`
  gives `<dir>/data.json: not valid UTF-8, cannot render`.
- `load_passes_walk_refusals_through` (unix only) — a symlink in the pack gives the plan 03
  refusal `<path>: symlinks are not allowed in a pack` and nothing else; a Unix socket bound with
  `std::os::unix::net::UnixListener` gives `<path>: only regular files and directories are
  allowed` and nothing else.
- `load_ignores_empty_directories` — a valid pack plus empty `skills/b/`, `agents/x/`,
  `native/claude/` and `commands/` loads with no skill `b`, no native dir and no diagnostic
  (§2.1: empty directories are never a refusal).
- `load_keeps_ignored_dot_entries` — `.DS_Store` is in `Pack::ignored` and not a root file.
- `unused_native_lists_unclaimed_tool_dirs` — `native/claude/` and `native/cluade/` with tools
  `[claude]` give `[cluade]`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): load and check pack directories`.

### Task 7 — Prove every rule end to end on fixtures

**Files:**
- Create `crates/agentstacks/tests/fixtures/packs/**` — one directory per case below; each
  invalid fixture is valid except for its one refusal. Markdown fixtures put frontmatter on lines
  1–4 (`---`, `name`, `description`, `---`), a blank line 5 and the case's token on line 6.
  `render-not-utf8/data.json` is written as raw bytes `0x66 0xFF`.
- Create `crates/agentstacks/tests/pack_load.rs` — drives `Pack::load` on each fixture; expected
  strings prefix the fixture path passed to `Pack::load`.

**Change:** no library change. Valid fixtures (§12): `valid-hooks` (a message hook and a lua hook
with an existing `run`), `valid-native-hooks`, `valid-options`, `valid-native-files`,
`valid-render-exclude`, `valid-tokens` (own `skill`, `agent`, `skill_dir` and `root` tokens,
`{{var arguments}}` in `SKILL.md`), `valid-shared-name` (skill and agent `x`).

**Proof:**
- `load_accepts_each_feature_fixture` — each valid fixture returns `Ok(Pack)`.
- `load_refuses_each_fixture` — table: fixture → exactly its `Diagnostic` `Display`:
  `no-pack-toml` → `<fx>: not a pack (no pack.toml)`; `reserved-commands` →
  `<fx>/commands: reserved, not supported in api 0`; `reserved-rules` →
  `<fx>/rules: reserved, not supported in api 0`; `skill-missing-skill-md` →
  `<fx>/skills/a: missing SKILL.md`; `skills-stray-file` → `<fx>/skills/README.md: only skill
  directories belong in skills/`; `skill-bad-dir-name` → `<fx>/skills/My_Skill: "My_Skill" is not
  a valid name`; `agents-stray-file` → `<fx>/agents/notes.txt: agents/ holds only <name>.md
  files`; `agents-subdirectory` → `<fx>/agents/x: agents/ holds only <name>.md files`;
  `agent-bad-stem` → `<fx>/agents/Coder.md: "Coder" is not a valid name`; `skill-name-mismatch` →
  `<fx>/skills/a/SKILL.md: name "b" does not match "a"`; `agent-name-mismatch` →
  `<fx>/agents/a.md: name "b" does not match "a"`; `native-stray-file` →
  `<fx>/native/README.md: only tool directories belong in native/`; `native-bad-tool` →
  `<fx>/native/Claude: "Claude" is not a valid name`; `nothing-to-install` →
  `<fx>: pack declares nothing to install`; `nothing-but-excluded-native-hooks` → the same;
  `lua-without-airsl` → `<fx>/pack.toml: requires.airsl: required (the pack contains .lua files)`;
  `render-missing` → `<fx>/pack.toml: render: "enforcment.json" does not exist`;
  `render-excluded` → `<fx>/pack.toml: render: "x.json" is also excluded`;
  `exclude-protected-pack-toml` → `<fx>/pack.toml: exclude: "pack.toml" excludes pack.toml`;
  `exclude-protected-skill-md` → `<fx>/pack.toml: exclude: "**/*.md" excludes
  skills/discuss/SKILL.md`; `exclude-protected-agent` → `<fx>/pack.toml: exclude: "agents/*"
  excludes agents/coder.md`; `exclude-protected-run` → `<fx>/pack.toml: exclude: "hooks/**"
  excludes hooks/enforce.lua`; `render-not-utf8` → `<fx>/data.json: not valid UTF-8, cannot
  render`; `arguments-outside-skill` → `<fx>/agents/b.md:6: var "arguments" is only valid in
  SKILL.md`; `neutral-references-native` → `<fx>/skills/a/SKILL.md:6: neutral content must not
  reference native/`; `own-skill-dir-is-an-agent` (pack `core`, `{{skill_dir core/coder}}`) →
  `<fx>/skills/a/SKILL.md:6: pack "core" has no skill "coder"`; `own-agent-missing` →
  `<fx>/skills/a/SKILL.md:6: pack "core" has no agent "codr"`; `hook-run-missing` →
  `<fx>/pack.toml: hooks[1].run: "hooks/missing.lua" does not exist`; `native-hook-run-missing` →
  `<fx>/native/claude/hooks.toml: hooks[0].run: "hooks/missing.lua" does not exist`;
  `hook-agents-unknown` → `<fx>/pack.toml: hooks[3].agents: pack "core" has no agent
  "task-briefer"`.
- `load_reports_errors_from_every_file_in_one_run` — fixture `many-errors`:
  `pack.description = ""`, an unknown key `disable-model-invocation` in `skills/a/SKILL.md` and
  `{{skil x}}` on line 6 of `agents/b.md` give, in this order, `<fx>/agents/b.md:6: unknown token
  kind "skil"`, `<fx>/pack.toml: pack.description: empty`, `<fx>/skills/a/SKILL.md: unknown key
  "disable-model-invocation"`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done
(`cargo make dod`, the whole workspace) is green. Commit
`test(agentstacks): cover pack loading with fixtures`.

---

## Risks

- When `walk` refuses (symlink, special file, non-UTF-8 path), load stops with those refusals.
  Option not taken: classifying the partial file list, which would add false `missing SKILL.md` or
  `does not exist` refusals for the entries `walk` refused.
- The fixture set is large (38 directories); keep each fixture minimal and named after its case.

## Proof

- `cargo make dod` green.
- Coverage map, every refusal in this plan's scope → its tests (exact `Display` asserted):

| Spec row | Tests |
|---|---|
| §2.1 no `pack.toml` | `classify_refuses_a_directory_without_pack_toml`, `load_stops_at_a_missing_pack_toml`, fixture `no-pack-toml` |
| §2.1 reserved `commands/`, `rules/` | `classify_reports_each_reserved_directory_once`, fixtures `reserved-commands`, `reserved-rules` |
| §2.1 missing `SKILL.md` | `classify_refuses_a_skill_directory_without_skill_md`, fixture `skill-missing-skill-md` |
| §2.1 `skills/` only valid-named directories | `classify_refuses_files_directly_in_skills`, `classify_refuses_invalid_skill_directory_names`, fixtures `skills-stray-file`, `skill-bad-dir-name` |
| §2.1 `agents/` only `<name>.md`, valid stem | `classify_refuses_non_md_files_and_subdirectories_in_agents`, `classify_refuses_invalid_agent_stems`, fixtures `agents-stray-file`, `agents-subdirectory`, `agent-bad-stem` |
| §2.1 `name` = directory / stem (rule in plan 04) | fixtures `skill-name-mismatch`, `agent-name-mismatch` |
| §2.1 nothing to install | `pack_with_only_root_files_has_nothing_to_install`, `each_installable_kind_counts`, `excluded_or_empty_native_hooks_do_not_count`, fixtures `nothing-to-install`, `nothing-but-excluded-native-hooks` |
| §2.1 symlinks, special files, non-UTF-8 paths (rules in plan 03) | `load_passes_walk_refusals_through` |
| §2.1 empty directories ignored | `load_ignores_empty_directories` |
| §2.1 dot entries kept for `plan`; shared skill/agent name | `load_keeps_ignored_dot_entries`; `classify_keeps_a_skill_and_an_agent_with_the_same_name`, fixture `valid-shared-name` |
| §2.2 `native/` only valid-named tool directories | `classify_refuses_files_directly_in_native`, `classify_refuses_invalid_native_tool_names`, fixtures `native-stray-file`, `native-bad-tool` |
| §2.2 unused `native/<tool>/` listed by `plan` | `unused_native_lists_unclaimed_tool_dirs` |
| §2.2 `hooks.toml` never installed; native `.md` not held to §5 | `classify_sorts_files_into_skills_agents_native_and_root`, `rendered_set_holds_each_installed_md_and_each_render_file`; `load_does_not_hold_native_md_to_frontmatter_rules` |
| §2.2 neutral content must not reference `native/` | `native_reference_is_refused_after_a_root_token`, `native_reference_in_frontmatter_keeps_its_key_path`, `native_reference_in_a_hook_field_keeps_its_key_path`, `load_allows_native_content_to_reference_native`, fixture `neutral-references-native` |
| §3.2 `arguments` only in `SKILL.md` | `arguments_var_is_refused_outside_skill_md`, `arguments_var_is_accepted_in_skill_md`, fixture `arguments-outside-skill` |
| §3.2 / §3.3 own-pack item missing | `own_pack_targets_must_exist`, fixtures `own-skill-dir-is-an-agent`, `own-agent-missing` |
| §3.4 rendered file not UTF-8 | `rendered_text_refuses_invalid_utf8`, `load_refuses_a_non_utf8_render_file`, fixture `render-not-utf8` |
| §3.4 / §4.2 excluded files not scanned | `exclude_drops_matching_files_from_install_and_render`, `load_does_not_scan_excluded_files` |
| §3.5 scanned once, body after `---`, tokens kept | `scan_body_counts_lines_from_the_file_top`, `load_scans_each_text_once`, `load_keeps_tokens_from_every_source` |
| §4.2 `render` exists / not excluded | `render_entry_must_exist`, `render_entry_must_not_be_excluded`, fixtures `render-missing`, `render-excluded` |
| §4.2 `exclude` protection; render-only refusal on overlap | `exclude_must_not_match_protected_files`, `exclude_matching_a_rendered_protected_file_reports_only_the_render_refusal`, fixtures `exclude-protected-*` |
| §4.2 `requires.airsl` for non-excluded `.lua` | `installed_lua_requires_requires_airsl`, `lua_in_a_skill_or_native_dir_counts`, `excluded_lua_needs_no_airsl`, fixture `lua-without-airsl` |
| §6.2 own-pack `run` exists | `own_runs_resolve_in_all_three_forms`, `missing_own_run_is_refused`, `run_naming_a_skill_file_does_not_exist_under_root`, `run_naming_a_native_file_does_not_exist_under_root`, `native_hook_runs_are_checked_as_own_pack_runs`, fixtures `hook-run-missing`, `native-hook-run-missing` |
| §6.2 `agents` name own agents | `hook_agents_must_name_own_agents`, fixture `hook-agents-unknown` |
| §10 all errors per run, sorted | `load_continues_past_a_refused_pack_toml`, `load_reports_errors_from_every_file_in_one_run` |
