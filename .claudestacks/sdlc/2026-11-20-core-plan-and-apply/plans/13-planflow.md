---
status: draft
created: 2026-11-22
depends-on: [03, 06, 07, 08, 09]
built-on: ea104af7451e
---

# Plan flow Implementation Plan

**Goal:** Build a reviewable `Plan` from the project through the ports, in the ten steps of spec §6.1.

**Architecture:** `app/plan` becomes a folder of flow files, one per group of §6.1 steps. Each file
calls a port (`Workspace`, `Git`, `Binary`, `Adapter`, plan 09) and passes what comes back to a
`types` rule. The rules are: the built formats (`check`, `Pack::load`, `Extension::load`, `scan`,
`TokenValues`, `render`, `tree_hash`, `AirslRange`), `types/op` (plan 04), `types/plan` (plan 06),
`types/resolve` (plan 07) and `types/protocol` (plan 08). No file in this folder decides anything
by itself (spec §2: "`app/` holds no rule"; `docs/architecture.md` "Where logic lives"). Ports are
generic parameters, never `dyn` (`static-dispatch.md`). `mod.rs` holds only exports
(`mod-rs-export-only.md`). Every file has its own colocated tests, which use only the fakes
(`unit-test-mandate.md`, spec §14 app row). File names are one word each.

**Tech Stack:** Rust 2024 workspace crate `agentstacks`, with the plan 09 port fakes (`FakeWorkspace`, `FakeGit`, `FakeBinary`, `FakeAdapter`) and the built `FakeFileTree`.

**Content authority:** spec §6.1, §6.3 (last paragraph), §6.4, §4.4 (flow order, rendering,
missing values), §3.1 (core writes and core `require_binary`), §3.2/§3.3 (where the flow calls
them), §9.1, §9.2 (passing failures through), §10, §7 "Lock content" (as the plan's new lock), §14
app row.

---

## Files that change

```
crates/agentstacks/src/app/plan.rs            — delete (plan 01's docs-only stub; replaced by the folder)
crates/agentstacks/src/app/plan/mod.rs        — create: four-question module doc; `mod` lines; `pub use flow::plan`
crates/agentstacks/src/app/plan/read.rs       — create: step 1, reading agentstacks.toml and agentstacks.lock
crates/agentstacks/src/app/plan/sources.rs    — create: steps 2–3, reconciling, fetching, tree hashes, the §6.3 path-source check
crates/agentstacks/src/app/plan/load.rs       — create: step 4, the set check plus each removed tool's locked extension; notes
crates/agentstacks/src/app/plan/render.rs     — create: step 5 render, filling tokens from the values result; missing-value skips
crates/agentstacks/src/app/plan/tools.rs      — create: steps 5–6, values → render → plan for each tool, removed tools, core writes, op checks
crates/agentstacks/src/app/plan/binaries.rs   — create: step 7, binary and airsl checks through `Binary`
crates/agentstacks/src/app/plan/steps.rs      — create: steps 8–9, resolving and reading disk state, deciding each path's step
crates/agentstacks/src/app/plan/flow.rs       — create: `plan`, steps 1–10 in order, building the `Plan`, collecting refusals
crates/agentstacks/src/app/plan/testing.rs    — create (#[cfg(test)]): a scenario builder over the plan 09 fakes, used by every test here
```

`src/app/mod.rs` (plan 01) already declares `pub mod plan;`, and a folder satisfies it. This plan
does not edit that file.

## Order of work

### Task 1 — Read the project and the lock

**Files:**
- Delete `crates/agentstacks/src/app/plan.rs` — replaced by the folder.
- Create `crates/agentstacks/src/app/plan/mod.rs` — module doc (what: the flow that builds a `Plan`; why: §6.1 order lives in `app`, rules in `types`; responsibilities per file; non-responsibilities: no rule, no disk, no Lua). `mod` lines for every file below, with `#[cfg(test)] mod testing;`. While `flow.rs` does not exist yet, declare only the files that do.
- Create `crates/agentstacks/src/app/plan/testing.rs` — `Scenario`: builds a project text, an optional lock text, packs and extensions as `FakeFileTree`s served by `FakeGit` (git sources) or `FakeWorkspace` (path sources), a scripted `FakeAdapter` (a values result plus ops for each tool), `FakeWorkspace` disk states and `FakeBinary` PATH entries. It also has a `shown(&Diagnostics) -> Vec<String>` helper that returns the sorted `Display` lines.
- Create `crates/agentstacks/src/app/plan/read.rs` — step 1.

**Change:** `read_inputs<W: Workspace>(&W) -> Result<Inputs, Diagnostics>` reads the
`agentstacks.toml` text and the `agentstacks.lock` text through `Workspace` (spec §6.1 step 1). It
parses them with the built `ProjectFile::parse` and `Lockfile::parse`. A lock that does not exist
means "no lock": every toml entry is then new (spec §9.1 row 1), as on the first apply (spec §6.2
checked example 1). `Inputs` is a flow-internal carrier, documented as such, that holds the
`ProjectFile` and an `Option<Lockfile>`. A read failure is the built I/O shape `<path>: <the
std::io::Error Display>` (formats §10, `FormatError::Io`). Parse refusals are passed through
unchanged. When both files are refused, both sets of refusals are returned.

**Proof:** written first, seen failing, then passing.
- `read_inputs_parses_the_project_and_the_lock` — with both texts on `FakeWorkspace`, `Inputs` holds the toml's packs and tools and the lock's entries.
- `read_inputs_without_a_lock_has_no_lock` — when `FakeWorkspace` has no lock, `Inputs` holds `None` and no refusal.
- `read_inputs_reports_an_unreadable_project_file_with_the_io_error` — a fake I/O error on `agentstacks.toml` gives exactly `agentstacks.toml: <that error's Display>`.
- `read_inputs_returns_project_and_lock_refusals_together` — a toml with an unknown key and a lock with `merge` give both refusals, with the lock one reading `agentstacks.lock: merge: unknown key` (spec §12 row 2).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow reads the project and the lock`.

### Task 2 — Resolve and fetch every source

**Files:**
- Create `crates/agentstacks/src/app/plan/sources.rs` — steps 2–3.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod sources;`.

**Change:** `fetch_sources<W: Workspace, G: Git>(&Inputs, &W, &G) -> Fetched` does three things.

1. It hands the project and the lock to the plan 07 reconciliation and gets one `Reconciled` per
   pack and tool key (spec §6.1 step 2, §9.1).
2. It acts on each `Reconciled` value, visiting packs in name order and then tools in name order:
   - **New**, or **Stale** (resolved again as new): a git tag pin goes through the `Git` port, tag
     → rev (spec §9.2 "fetch a tag"); a rev pin goes through `Git` "fetch a rev". Each Stale entry
     adds the plan 07 note `agentstacks.toml: packs.core: source changed, resolved again` (spec
     §9.1 row 3).
   - **Locked**: fetches the lock's `rev` only, and never lists or fetches a tag. A moved remote
     tag therefore does not move the entry (spec §9.1 last paragraph). The fetched tree hash and
     the lock's `tree` go to the plan 07 tree rule (spec §9.1 row 2), which may refuse with
     `agentstacks.toml: packs.core: tree sha256:<a> at rev <rev> differs from the lock's
     sha256:<b>`. A path source whose tree changed is a normal change, not a refusal (same row).
   - **RemovedTool**: its locked source (`LockedSource::to_source`, built) is fetched by its lock
     `rev`, or read from its path, so the extension can run (spec §6.4 first bullet).
   - **RemovedPack**: nothing is fetched. Its files fall to the deciding step (spec §9.1 row 4).
3. It builds the source trees. A git source's tree is the snapshot `Git` returns (spec §9.2
   "list files", "read blobs"). A path source first goes through `Workspace` resolution (spec
   §6.3). An outside answer becomes the plan 02 refusal `agentstacks.toml:
   packs.team-lint.path: resolves outside the project` (spec §6.3 last paragraph), located at
   that entry's key path (`packs.<key>.path` or `tools.<key>.path`, spec §13 "agentstacks.toml
   with key"). An inside answer is read through the `Workspace` tree for that directory. Each
   pack's tree hash is the built `tree_hash` over its tree. A tool's tree hash comes from
   `Extension::load`, in Task 3.

Every `Git` failure is passed through unchanged (spec D3, §9.2 failure table). This covers `git
<subcommand> failed: …`, the `subdir` refusals and the HOME refusal (spec §9.3). The exception is
"git not found": the keys of every source that got this answer, in visiting order, go into one
plan 02 git-missing refusal, `agentstacks.toml: git not found on PATH (needed by packs.core,
tools.claude)` (spec §9.2 row 1, "once"). `Fetched` is a flow-internal carrier, documented as
such. For each key it holds the tree snapshot, the resolved rev (git) and the tree hash, or that
key's refusals. It also holds the notes and the fetched removed tools.

**Proof:** written first, seen failing, then passing.
- `fetch_sources_resolves_a_new_git_tag_to_its_rev` — with no lock and a tag pin, `FakeGit` is asked for the tag, and `Fetched` holds the rev it returned plus the `tree_hash` of the snapshot.
- `fetch_sources_keeps_the_locked_rev_when_the_tag_moved` — the lock has rev R1 for tag T, and `FakeGit` now maps T to R2. Only R1 is fetched, `FakeGit` records no tag call, and `Fetched` holds R1 (spec §14 "moved tag").
- `fetch_sources_resolves_a_stale_entry_again_with_a_note` — the toml tag differs from the lock's tag. The tag is fetched as new, and the notes hold exactly `agentstacks.toml: packs.core: source changed, resolved again` (spec §14 "stale lock").
- `fetch_sources_refuses_a_locked_git_tree_that_differs` — `FakeGit` serves different bytes at the lock's rev, which gives the §9.1 row 2 message with both hashes and the rev.
- `fetch_sources_accepts_a_changed_path_tree` — a locked path source whose files changed gives no refusal and the new hash.
- `fetch_sources_refuses_a_path_source_outside_the_project` — `FakeWorkspace` resolves `packs/team-lint` outside, which gives exactly `agentstacks.toml: packs.team-lint.path: resolves outside the project`.
- `fetch_sources_reports_git_missing_once_for_every_source` — `FakeGit` answers "not found" for a pack and a tool, which gives exactly one refusal: `agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)`.
- `fetch_sources_passes_a_git_failure_through` — `FakeGit` fails a fetch with stderr `fatal: couldn't find remote ref refs/tags/packs-v9.9.9`, which gives `agentstacks.toml: packs.core: git fetch failed: fatal: couldn't find remote ref refs/tags/packs-v9.9.9` (spec §9.2 row 2).
- `fetch_sources_fetches_a_removed_tool_by_its_locked_rev` — a tool that is only in the lock is fetched at the lock's rev and listed among the removed tools.
- `fetch_sources_fetches_nothing_for_a_removed_pack` — a pack that is only in the lock makes no `Git` call.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow resolves and fetches sources`.

### Task 3 — Load and check the set

**Files:**
- Create `crates/agentstacks/src/app/plan/load.rs` — step 4.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod load;`.

**Change:** `load_set(&Inputs, Fetched) -> Result<Loaded, Diagnostics>` calls the built `check`
(spec §6.1 step 4, formats §9). Its pack loader runs `Pack::load` over the fetched tree for that
key, and its tool loader runs `Extension::load` the same way. A key whose fetch was refused makes
its loader return that key's fetch refusals, so `check` reports them with every other refusal and
runs no second check on that entry (`src/types/set/check.rs` responsibilities). Each removed
tool's fetched tree is loaded with `Extension::load` on its own, because the toml no longer lists
it and `check` cannot see it (spec §6.4). The notes from step 2 stay, and two kinds are added:
- each pack's `Pack::ignored` dot entries and each extension's `Extension::ignored` dot entries;
- each pack's `Pack::unused_native(tools)`, with the toml's tool names as `tools`

(spec §6.1 "notes", formats §2.1, §2.2). These go into the plan 06 `Plan` notes. `Loaded` is a
flow-internal carrier holding the `ValidSet`, the removed tools' `Extension`s, the trees, revs and
tree hashes, and the notes.

**Proof:** written first, seen failing, then passing.
- `load_set_returns_the_valid_set_of_fetched_sources` — a valid pack and a valid extension give a `ValidSet` with one pack and one extension.
- `load_set_reports_fetch_and_load_refusals_together` — one pack whose fetch was refused and one whose `pack.toml` says `api = 1` give both refusals and no set.
- `load_set_loads_a_removed_tool_outside_the_set` — a removed tool's extension is loaded, is in `Loaded`, and is absent from `ValidSet::extensions`.
- `load_set_notes_dot_entries_and_unused_native_dirs` — a pack with `.DS_Store` and `native/opencode/`, in a project listing only `claude`, gets one dot-entry note and one unused-native note.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow loads and checks the set`.

### Task 4 — Render a tool's packs with its values

**Files:**
- Create `crates/agentstacks/src/app/plan/render.rs` — the render part of step 5.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod render;`.

**Change:** `render_packs(&Loaded, &ToolName, &ValuesResult) -> Rendered` renders every text
formats §3.4 lists for each pack of the set:
- `.md` files and `render` files, as whole files;
- frontmatter string values;
- hook `text`, `args`, `run` and grant paths in `pack.toml`;
- the same fields in this tool's `native/<tool>/hooks.toml`;
- this tool's native `.md` files.

Each text goes through four steps:
1. scan it with the built `scan`;
2. get each token's value from the plan 04 §4.4 token rule over the `ValuesResult` (`{{root}}`,
   `{{root <pack>}}` → `<root>/<pack>`; `{{skill p/s}}` → `invoke`; `{{skill_dir p/s}}` → `dir`;
   `{{agent p/a}}`; `{{var v}}`). An `{{option o}}` token takes `ValidSet::option` (formats §4.4);
3. put each value in with `TokenValues::insert`;
4. splice with `render` (spec §4.4 "Every token is filled through `TokenValues::insert` and
   `render`").

A token with no value hands its item to the plan 04 missing-token skip rule (spec §4.4 "Missing
values"), and the item is left out of the plan payload:
- a skill is skipped as a whole group, with `claude gave no value for {{skill core/status}}`. A
  skill with no `skills[...]` entry at all gets `claude gave no install location for skill
  core/status`;
- an agent, a hook (any field), a native file or a native hook is skipped as the item itself;
- a root file becomes a core skip of that file.

Every file's final bytes, keyed by its plan 04 item id, are what a `copy` installs (spec §3.1
`copy` row): the whole rendered file for a rendered file (the neutral frontmatter of `SKILL.md`
and agents included), and the raw bytes from the source tree otherwise. A lua hook's `command`
argv is the plan 08 `HookCommand` over the rendered hook fields (spec §4.4 "`command` is the argv
from §11.2"). The rendered parts go to the plan 04 item-id assignment and `PlanInput`
construction. `Rendered` is a flow-internal carrier holding the `PlanInput`, the final bytes by
item id, the rendered root files and the core skips with their reasons.

**Proof:** written first, seen failing, then passing.
- `render_packs_fills_every_token_kind_from_the_values` — one `SKILL.md` holding `{{root}}`, `{{root core}}`, `{{skill core/status}}`, `{{skill_dir core/status}}`, `{{agent core/coder}}`, `{{var tmpdir}}` and `{{option archive_keep}}` renders, with the §4.4 example values result, to `.claude/agentstacks/core`, `.claude/agentstacks/core`, `/status`, `.claude/skills/status`, `coder`, `$TMPDIR` and the option's value.
- `render_packs_skips_a_skill_with_a_missing_value` — with no value for `{{skill core/status}}`, the whole `core/skill/status` group is skipped with exactly `claude gave no value for {{skill core/status}}` and is absent from `PlanInput`.
- `render_packs_skips_a_skill_with_no_install_location` — with no `skills["core/status"]` entry at all, the skip reason is exactly `claude gave no install location for skill core/status`.
- `render_packs_skips_only_the_agent_hook_or_native_item_missing_a_value` — an agent, a hook `run`, a native file and a native hook with a missing value are each skipped as themselves, and the items beside them stay in `PlanInput`.
- `render_packs_makes_a_core_skip_for_a_root_file_missing_a_value` — a rendered root file with a missing value becomes a core skip of that file.
- `render_packs_keeps_raw_bytes_for_unrendered_files` — a binary `logo.png` keeps its bytes, has no `text` in `PlanInput`, and a rendered `SKILL.md`'s final bytes include its rendered frontmatter.
- `render_packs_builds_the_hook_command_argv` — a lua hook's `command` equals the `HookCommand` argv for its rendered `run` and grants.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow renders packs with tool values`.

### Task 5 — Run each tool's extension and check its operations

**Files:**
- Create `crates/agentstacks/src/app/plan/tools.rs` — steps 5–6.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod tools;`.

**Change:** `run_tools<W: Workspace, A: Adapter>(&Loaded, &W, &A) -> Ran` runs the listed tools in
name order (spec §4.4 "Tools run in name order"). For each tool it does the following:
1. calls `Adapter::values` with the plan 04 `ValuesInput` for this tool's packs and vars, and gets
   back a `ValuesResult` or that tool's refusals;
2. records the tool's core proposals (`Origin::Core`), using the plan 04 §3.1 core-write rule and
   the `<root>` from the values result:
   - each pack root file goes to `<root>/<pack>/<rel>`, with its rendered or raw bytes and its
     pack as owner (spec D7, §3.1 row 5);
   - `<root>/runtime.lua` gets the plan 08 `RUNTIME_LUA` bytes and the tool as owner, when a pack
     of this tool has a `lua` hook or a native hook that is not skipped;
3. renders with Task 4;
4. calls `Adapter::plan` with the `PlanInput`, the workspace (for `read`) and the `Proposals` so
   far. The `Proposals` hold the core writes and every earlier tool's ops, so the call-time
   duplicate check (spec §3.3, plan 04 `Proposals`) can refuse with `path ".claude/x" is already
   proposed at adapter.lua:9` / `… already proposed by opencode` / `… already proposed by core
   (root file of core)`. The call returns the proposals and the reads record (spec §4.3, what
   each `read` returned);
5. runs the plan 04 coverage check over the `PlanInput` and the tool's proposals, which may refuse
   with `claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write
   it, or skip it with a reason)` (spec §3.2). It then runs the plan 04 `translate.lua` rule,
   which may refuse with `claude/adapter.lua: lua hooks need <root>/translate.lua (write it)`.

Then the flow runs each removed tool, in name order, after the listed tools (spec §6.1 step 5,
second line). The steps are the same, with these differences: the `ValuesInput` has an empty
`packs` list, nothing is rendered, no core write is made for its root, and the plan 06
removed-tool filter keeps only its writes to paths it read in this run (spec §6.4 bullets 1–2).

Adapter refusals are collected (spec §4.6 "Any extension error refuses the whole plan"), for
example `claude/adapter.lua: registers no values handler` and `claude/adapter.lua: registers no
plan handler`, along with §3.3 op refusals and Lua failures. A tool whose `values` failed is not
rendered and gets no `plan` call. `Ran` is a flow-internal carrier holding the `Proposals`, each
tool's reads record and `extension.reads` (plan 03), the extension skips with their reasons
(spec §3.2 "the plan shows the first reason"), the core skips, and the final bytes by item id.

**Proof:** written first, seen failing, then passing.
- `run_tools_runs_tools_in_name_order` — for tools `opencode` and `claude`, `FakeAdapter` records `claude` values, `claude` plan, `opencode` values, `opencode` plan, in that order.
- `run_tools_dispatches_plan_with_the_rendered_payload` — the `PlanInput` given to `FakeAdapter::plan` holds the rendered text, not the raw token.
- `run_tools_writes_root_files_under_the_values_root` — a pack root file `hooks/enforce.lua` becomes a core write to `.claude/agentstacks/core/hooks/enforce.lua`, owned by `core`.
- `run_tools_writes_runtime_lua_for_a_lua_hook` — a pack with a `lua` hook gets a core write of `RUNTIME_LUA` to `.claude/agentstacks/runtime.lua`, owned by the tool.
- `run_tools_writes_no_runtime_lua_when_every_hook_is_skipped_or_absent` — with no lua hook and only skipped native hooks, there is no `runtime.lua` proposal.
- `run_tools_refuses_a_path_an_earlier_tool_proposed` — `opencode` writing a path `claude` already wrote gives the refusal ending `is already proposed by claude`, at `opencode`'s call line.
- `run_tools_refuses_an_extension_write_onto_a_core_write` — an extension write to `.claude/agentstacks/core/hooks/enforce.lua` gives the refusal ending `is already proposed by core (root file of core)`.
- `run_tools_refuses_an_uncovered_item` — an extension that never covers `core/skill/status/references/x.md` gives exactly `claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a reason)`.
- `run_tools_refuses_lua_hooks_without_translate_lua` — lua hooks covered by a write, with no write to `.claude/agentstacks/translate.lua`, give exactly `claude/adapter.lua: lua hooks need .claude/agentstacks/translate.lua (write it)`.
- `run_tools_collects_adapter_refusals_from_every_tool` — `claude` with no values handler and `opencode` with no plan handler give both refusals.
- `run_tools_runs_a_removed_tool_with_zero_packs` — the removed tool's `ValuesInput` has an empty `packs` list. Only its write to a path it read is kept, its other write is dropped, and no core write is made for its root (spec §14 "removed tool").

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow runs tool extensions`.

### Task 6 — Check binaries and the airsl pin

**Files:**
- Create `crates/agentstacks/src/app/plan/binaries.rs` — step 7.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod binaries;`.

**Change:** `check_binaries<B: Binary>(&Loaded, &Ran, &B) -> Binaries` gathers what to check with
the plan 04 §10 requirement collection. That covers each pack's `requires.binaries` as core
`require_binary` (spec §3.1 row 6) and each extension `require_binary` op, where the same name
from one source is checked once (spec §3.3 "Accepted without refusal"). Each name is looked up
through `Binary` (spec §6.1 step 7). A missing binary is refused once for each requirer (spec §10
"reported once for each"): `agentstacks.toml: packs.core: binary "rg" not found on PATH` for a
pack, and `claude/adapter.lua:7: require_binary: "node" not found on PATH` for an extension, at
the op's `Origin` location.

When `ValidSet::airsl` holds a range (a pack of the set declares `requires.airsl`), the flow
checks airsl:
- it asks `Binary` for the `airsl --version` text;
- not on PATH gives `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)`;
- text the plan 04 §10 version rule cannot parse gives `agentstacks.toml: airsl --version printed
  "<text>", expected "airsl <version>"`;
- a parsed version outside the range gives the built `AirslRange::installed_refusal`,
  `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4` (spec §10).

The installed version is kept for `[airsl] version` (spec §7 "Lock content", §10). When it
differs from the lock's `[airsl]`, the plan 06 note `airsl 0.1.4 → 0.1.5` is added (spec §10). If
no pack declares `requires.airsl`, `Binary` is never asked about airsl. `Binaries` is a
flow-internal carrier holding the binary checks for the `Plan`, the installed airsl version and
the refusals.

**Proof:** written first, seen failing, then passing.
- `check_binaries_refuses_a_missing_pack_binary` — `rg` absent from `FakeBinary` gives exactly `agentstacks.toml: packs.core: binary "rg" not found on PATH`.
- `check_binaries_refuses_a_missing_extension_binary` — `require_binary{name="node"}` at `adapter.lua:7` with `node` absent gives exactly `claude/adapter.lua:7: require_binary: "node" not found on PATH`.
- `check_binaries_reports_a_binary_once_for_the_pack_and_once_for_the_extension` — `rg` required by `core` and by `claude` gives two refusals, one of each shape.
- `check_binaries_checks_a_repeated_name_once` — two `require_binary{name="node"}` ops lead to one `FakeBinary` lookup and at most one refusal.
- `check_binaries_refuses_airsl_not_on_path` — `core` and `sdlc` declare `requires.airsl` and airsl is absent, which gives exactly `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)`.
- `check_binaries_refuses_unparseable_airsl_version_text` — `FakeBinary` prints `airsl-dev`, which gives exactly `agentstacks.toml: airsl --version printed "airsl-dev", expected "airsl <version>"`.
- `check_binaries_refuses_airsl_out_of_range` — `airsl 0.1.3` against `>=0.1.4` gives exactly `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4`.
- `check_binaries_moves_the_pin_with_a_note` — lock `[airsl] version = "0.1.4"` and `airsl 0.1.5` installed give installed version `0.1.5` and the note `airsl 0.1.4 → 0.1.5`.
- `check_binaries_skips_airsl_when_no_pack_declares_it` — `FakeBinary` records no airsl query.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow checks binaries and airsl`.

### Task 7 — Decide every path's step

**Files:**
- Create `crates/agentstacks/src/app/plan/steps.rs` — steps 8–9.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod steps;`.

**Change:** `decide_steps<W: Workspace>(&Inputs, &Ran, &W) -> Decided` covers two kinds of path.

- **Every proposed path** (core writes, `write`, `copy`): the flow asks `Workspace` for its
  `Resolution` and `DiskState` (spec §6.1 step 8, §6.3). A `copy`'s new bytes are the final bytes
  of its file id (Task 4). The plan 06 shared rule gets the path, the proposing tool's
  `extension.reads` and its reads record (spec §6.2 "Shared"). The plan 06 decide fn for proposed
  paths then gets the `Resolution`, the `DiskState`, the lock entry and the new bytes, and
  returns a `Step` or a refusal. For a shared file the `Step` also keeps the old bytes (spec §6.1
  `Plan` bullet 1).
- **Every lock `[[file]]` path no longer proposed**: this covers files of removed packs (spec
  §9.1 row 4) and of removed tools (spec §6.4 bullet 3). Each is resolved and read the same way,
  then decided by the plan 06 decide fn for locked-not-proposed paths.

The refusals come from plan 06/02 and are not reworded here:
- `<path>: is not a regular file`
- `<path>: changed since the last apply`
- `<path>: exists and was not written by agentstacks`
- `<path>: changed since the last apply; not deleting it`
- `<path>: resolves outside the project`

`Decided` is a flow-internal carrier holding the `Step`s, each with its `decided_on` `DiskState`
(spec §6.1 `Plan` bullet 5), and the refusals.

**Proof:** written first, seen failing, then passing. One test per row of the second §6.2 table
whose input the flow supplies.
- `decide_steps_forgets_an_absent_locked_path` — a locked path that is no longer proposed and is absent on disk gives `forget`.
- `decide_steps_forgets_a_locked_path_replaced_by_a_symlink` — the path is now a symlink, so `forget`.
- `decide_steps_refuses_to_delete_a_changed_locked_path` — the hash differs from the lock, which gives exactly `<path>: changed since the last apply; not deleting it`.
- `decide_steps_refuses_a_proposed_path_that_is_not_a_regular_file` — `FakeWorkspace` reports a directory at `.claude/x`, which gives exactly `.claude/x: is not a regular file`.
- `decide_steps_keeps_old_bytes_for_a_shared_file` — a path that is in `extension.reads` and was read gives a step holding the old bytes.
- `decide_steps_records_the_disk_state_each_step_was_decided_on` — every `Step`'s `decided_on` equals what `FakeWorkspace` returned.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow decides each path's step`.

### Task 8 — Assemble the plan flow

**Files:**
- Create `crates/agentstacks/src/app/plan/flow.rs` — the public flow.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod flow;` and `pub use flow::plan;` with `#[doc(inline)]`.

**Change:** `plan<W: Workspace, G: Git, B: Binary, A: Adapter>(&W, &G, &B, &A) -> Result<Plan,
Diagnostics>` runs Tasks 1–7 in the order of spec §6.1 steps 1–9 (map ownership: plan 13 builds
this signature). For step 10 it either returns the plan 06 `Plan` or every refusal collected in
one `Diagnostics`, which the caller sorts with `into_sorted` (spec §6.1 "Refusals are collected,
never first-only, and sorted"). The `Plan` holds:
- the steps;
- the skips with their reasons (the core's missing-value skips and the extensions' skips);
- the binary checks;
- the notes;
- each step's `decided_on`;
- the new lock contents.

The new lock contents come from the plan 06 lock-content builder (spec §7 "Lock content"). It
takes the set's tools and packs with their resolved revs and tree hashes, a `[[file]]` for each
non-shared create, update, restore or unchanged step together with its derived owner (plan 04
owner rule, spec §3.1 `by`), and `[airsl] version` from Task 6 exactly when a pack declares
`requires.airsl`. Nothing from a removed tool is locked (spec §6.4 bullet 4). A step whose input
does not exist is skipped: with no `ValidSet` there are no tools to run, and a tool whose `values`
was refused gets nothing rendered. The doc comment carries a runnable example over the fakes.

**Proof:** written first, seen failing, then passing. These tests drive every spec §6.2 checked
example through `plan` with fakes only (spec §14 app row).
- `plan_creates_an_absent_skill_file_on_first_apply` — with no lock, absent `.claude/skills/status/SKILL.md` and a `copy` of `core/skill/status/SKILL.md`, the step is `create`, and the new lock has its `[[file]]` with `by = "core"`.
- `plan_refuses_a_skill_file_edited_after_apply` — the disk hash differs from the lock and from the new bytes, which gives exactly `.claude/skills/status/SKILL.md: changed since the last apply`.
- `plan_restores_a_deleted_skill_file` — locked and absent gives `restore`.
- `plan_deletes_the_untouched_file_of_a_removed_pack` — `team-lint` is in the lock but not the toml, and its file hash equals the lock. The step is `delete`, and the new lock has no `team-lint` pack and no file of it (spec §14 "removed pack").
- `plan_updates_a_shared_settings_file_without_locking_it` — `.claude/settings.json` is in `extension.reads`, was read with the user's permissions and is proposed with hooks added. The step is `update` with the old bytes kept, and there is no `[[file]]` for it.
- `plan_creates_an_absent_shared_settings_file` — `read` returned absent and the extension proposes the file, which gives `create`, with no `[[file]]`.
- `plan_refuses_an_agent_file_not_written_by_agentstacks` — the file exists with different bytes and is not in the lock, which gives exactly `.claude/agents/coder.md: exists and was not written by agentstacks`.
- `plan_refuses_a_proposed_symlink_to_etc_hosts` — `FakeWorkspace` resolves the path outside, which gives exactly `<that path>: resolves outside the project`.
- `plan_removes_a_tool_by_running_its_locked_extension` — `opencode` is only in the lock. Its extension runs with zero packs, its shared write is a step, its locked files get `delete`, and the new lock has no `opencode` `[[tool]]` and no file of it (spec §14 "removed tool").
- `plan_refuses_while_a_removed_tool_source_is_unreachable` — `FakeGit` fails the removed tool's rev fetch. The git refusal is returned and no `Plan` is built, so nothing is removed (spec §9.2 last row).
- `plan_locks_resolved_revs_trees_and_airsl` — the new lock holds each tool's and pack's rev and tree from Task 2, and `[airsl] version` equal to the installed version.
- `plan_collects_refusals_from_several_steps_sorted` — a coverage refusal from one tool and a `changed since the last apply` refusal are both returned, in the order `Diagnostics::into_sorted` gives.
- `plan_with_any_refusal_returns_no_plan` — any refusal makes the result `Err`, so no `Plan` that holds a refusal can reach `apply` (spec §7 step 1).

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow from project to reviewable plan`.

---

## Risks

- **Port and rule seams with plans 04, 06, 07 and 09.** This plan depends on several of their
  calls. Plan 09: a `Workspace` tree for a path-source directory, and `Adapter::plan` taking the
  `Proposals` so far plus the workspace for `read`. Plan 04: the §4.4 token-value rule, the §3.1
  core-write rule, `ValuesInput` and `PlanInput` construction, the §10 requirement collection and
  the airsl version parse. Plan 06: the shared rule, both decide fns, the removed-tool filter and
  the lock-content builder. Plan 07: the tree rule and the stale note. If one of them names a
  call differently, the coder follows that plan's names. The behaviour cited here stays.
- **Riskiest task: Task 4, rendering.** Tokens are scanned again per text, so their spans index
  the exact text being rendered. A token already found at pack load but missed by this scan
  would render silently as its literal text. Its tests check every token kind and every text
  source formats §3.4 lists.
- **Not done here:** turning `Plan` into text, including the diff (cli-commands, spec §16).
  Writing to disk (plan 14). The fs, process and host contract tests (plans 10–12).

## Proof

- Spec rows covered:
  - §6.1 steps 1–10 (Tasks 1–8);
  - §6.1 `Plan` contents (Task 8);
  - §6.2 every checked example (Task 8), plus the second-table rows that need flow input
    (Task 7);
  - §6.3 last paragraph (Task 2) and the resolution of every proposed and deleted path (Task 7);
  - §6.4 all four bullets (Tasks 2, 5, 7, 8);
  - §4.4 flow order, token table, missing values, `command` argv, tool name order (Tasks 4–5);
  - §4.6 collecting extension refusals (Task 5);
  - §3.1 core writes and core `require_binary` (Tasks 5–6), `by` (Task 8);
  - §3.2 coverage and the `translate.lua` row, §3.3 duplicate messages (Task 5);
  - §9.1 rows 1–5 and the moved tag (Task 2);
  - §9.2 failures passed through, git-missing once, the removed-tool source row (Tasks 2, 8);
  - §10 all rows, "once for each", the pin move (Task 6);
  - §7 "Lock content" (Task 8);
  - §14 app row: checked examples, removed pack, removed tool, stale lock, moved tag (Tasks 2, 5,
    8).
- Every test uses only `FakeWorkspace`, `FakeGit`, `FakeBinary`, `FakeAdapter` and `FakeFileTree`. `tests/isolation.rs` (plan 01) keeps `src/app` free of `std::fs`, `std::process` and adapter imports.
- `cargo make dod` is green at the end of Task 8.
