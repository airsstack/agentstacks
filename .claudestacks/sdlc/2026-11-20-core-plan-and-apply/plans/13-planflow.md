---
status: approved
created: 2026-11-22
depends-on: [03, 06, 07, 08, 09]
built-on: f39b2a814319
---

# Plan flow Implementation Plan

**Goal:** Build a reviewable `Plan` from the project through the ports, in the ten steps of spec §6.1.

**Architecture:** `app/plan` becomes a folder of flow files, one per group of §6.1 steps. Each file
calls a port (plan 09: `Workspace`, `Git`, `Binary`, `Adapter`) and passes what comes back to a
`types` rule. The rules are:
- the built formats: `check`, `Pack::load`, `Extension::load`, `scan`, `TokenValues`, `render`,
  `tree_hash`, `AirslRange`, `Lockfile::new`, `check_airsl_presence`;
- `types/op` (plan 04), `types/plan` (plan 06), `types/resolve` (plan 07) and `types/protocol`
  (plan 08).

No file in this folder decides anything by itself (spec §2: "`app/` holds no rule";
`docs/architecture.md` "Where logic lives"). Ports are generic parameters, never `dyn`
(`static-dispatch.md`). `mod.rs` holds only exports (`mod-rs-export-only.md`). Every file has its
own colocated tests, which use only the fakes (`unit-test-mandate.md`, spec §14 app row). The fakes
are `#[cfg(test)]`, so no doc comment here carries an example that uses them. The end-to-end unit
tests in Task 8 take the place of such examples. File names are one word each.

**Tech Stack:** Rust 2024 workspace crate `agentstacks`, with the plan 09 fakes (`FakeWorkspace`, `FakeGit`, `FakeBinary`, `FakeAdapter`) and the built `FakeFileTree`.

**Content authority:**
- spec §6.1, §6.3 (last paragraph), §6.4;
- §4.4 (flow order, rendering, missing values);
- §3.1 (core writes and core `require_binary`), §3.2/§3.3 (where the flow calls them);
- §9.1, §9.2 (passing failures through), §10;
- §7 "Lock content" (as the plan's new lock);
- §16 rules R1, R2, R6, R7, R8, R10, R11, R15, R18, R20, R22, R27, R28(b), R28(g);
- §14 app row.

---

## Files that change

```
crates/agentstacks/src/app/plan.rs            — delete (plan 01's docs-only stub; replaced by the folder)
crates/agentstacks/src/app/plan/mod.rs        — create: four-question module doc; `mod` lines; `pub use flow::plan`
crates/agentstacks/src/app/plan/read.rs       — create: step 1, reading agentstacks.toml and agentstacks.lock
crates/agentstacks/src/app/plan/sources.rs    — create: steps 2–3, reconciling, fetching, tree checks, the §6.3 path-source check
crates/agentstacks/src/app/plan/load.rs       — create: step 4, the set check plus each removed tool's locked extension; dot-entry and unused-native notes
crates/agentstacks/src/app/plan/render.rs     — create: step 5 render, filling tokens from the values result; missing-value skips
crates/agentstacks/src/app/plan/tools.rs      — create: steps 5–6, values → core writes → plan for each tool, removed tools, coverage and translate.lua
crates/agentstacks/src/app/plan/binaries.rs   — create: step 7, binary and airsl checks through `Binary`
crates/agentstacks/src/app/plan/steps.rs      — create: steps 8–9, resolving each path and deciding its step
crates/agentstacks/src/app/plan/flow.rs       — create: `plan`, steps 1–10 in order, building the `Plan`, collecting refusals
crates/agentstacks/src/app/plan/testing.rs    — create (#[cfg(test)]): a scenario builder over the plan 09 fakes, used by every test here
```

`src/app/mod.rs` (plan 01) already declares `pub mod plan;`, and a folder satisfies it. This plan
does not edit that file.

## Order of work

### Task 1 — Read the project and the lock

**Files:**
- Delete `crates/agentstacks/src/app/plan.rs` — replaced by the folder.
- Create `crates/agentstacks/src/app/plan/mod.rs` — module doc (what: the flow that builds a `Plan`; why: the §6.1 order lives in `app`, rules in `types`; responsibilities per file; non-responsibilities: no rule, no disk, no Lua). `mod` lines for the files that exist so far, with `#[cfg(test)] mod testing;`.
- Create `crates/agentstacks/src/app/plan/testing.rs` — `Scenario` builds:
  - a project text and an optional lock text;
  - packs and extensions as `FakeFileTree`s, served by `FakeGit::tree` (git sources) or `FakeWorkspace::source` (path sources);
  - a scripted `FakeAdapter`, one `ToolScript` per tool;
  - `FakeWorkspace` files;
  - `FakeBinary` PATH entries and airsl text.

  It also has a `shown(Diagnostics) -> Vec<String>` helper that returns the sorted `Display` lines.
- Create `crates/agentstacks/src/app/plan/read.rs` — step 1.

**Change:** `read_inputs<W: Workspace>(&W) -> Result<Inputs, Diagnostics>` reads two texts
through `Workspace` (spec §6.1 step 1): `project_text()` and `lock_text()`. It parses them with the
built `ProjectFile::parse` and `Lockfile::parse`. A lock that does not exist (`lock_text()` gives
`None`) means "no lock": plan 07's `reconcile` then makes every toml entry `New` (spec §9.1 row 1),
as on the first apply (spec §6.2 checked example 1). `Inputs` is a flow-internal carrier,
documented as such, that holds the `ProjectFile` and an `Option<Lockfile>`. A read failure is `<path>: <io::Error
Display>` at the file involved (spec §16 R1; built `FormatError::Io`). Parse refusals are passed
through unchanged. When both files are refused, both sets of refusals are returned. The lock read
here is never checked with `check_airsl_presence` (R27).

**Proof:** written first, seen failing, then passing.
- `read_inputs_parses_the_project_and_the_lock` — with both texts on `FakeWorkspace`, `Inputs` holds the toml's packs and tools and the lock's entries.
- `read_inputs_without_a_lock_has_no_lock` — with no lock on `FakeWorkspace`, `Inputs` holds `None` and no refusal.
- `read_inputs_reports_an_unreadable_project_file_with_the_io_error` — `FakeWorkspace::fail(ProjectText)` gives exactly `agentstacks.toml: <FakeWorkspace::denied() Display>` (R1).
- `read_inputs_returns_project_and_lock_refusals_together` — a toml with an unknown key and a lock with `merge` give both refusals. The lock refusal reads `agentstacks.lock: merge: unknown key` (spec §12 row 2).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow reads the project and the lock`.

### Task 2 — Resolve and fetch every source

**Files:**
- Create `crates/agentstacks/src/app/plan/sources.rs` — steps 2–3.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod sources;`.

**Change:** `fetch_sources<W: Workspace, G: Git>(&Inputs, &W, &G) -> Fetched` starts by calling
plan 07's `reconcile(project, lock)`. That gives one `Reconciled` per `SourceKey` (spec §6.1 step
2, §9.1; R9's normalised comparison is plan 07's). It then visits the keys in `SourceKey` order:
packs, then tools, each in name order.

- **`New`, or `Stale` (resolved again as new).**
  - A tag pin calls `Git::fetch_tag`, which returns the rev, then `Git::tree(url, rev, subdir,
    pin)` (spec §9.2).
  - A rev pin calls `Git::tree` directly.
  - A path source is described under "Path sources" below.
  - Each `Stale` entry adds plan 06's `Note::SourceChanged` at `SourceKey::location()`, which
    displays `agentstacks.toml: packs.core: source changed, resolved again` (spec §9.1 row 3).
- **`Locked { rev, tree }`.** The flow calls `Git::tree` at the lock's rev and never
  `list_tags` or `fetch_tag`. A moved remote tag therefore cannot move the entry (spec §9.1 last
  paragraph). The flow hashes the tree with the built `tree_hash` and hands it to plan 07's
  `check_tree`. A git mismatch is refused with `agentstacks.toml: packs.core: tree sha256:<a> at
  rev <rev> differs from the lock's sha256:<b>` (§9.1 row 2). For a path source a changed tree is
  a normal change.
- **`RemovedTool`.** Its locked source (`LockedSource::to_source`, built) is fetched at its lock
  rev, or read from its path, so the extension can run (spec §6.4 bullet 1). Its git tree goes
  through `check_tree` like any locked entry (R18).
- **`RemovedPack`.** Nothing is fetched. Its files fall to the deciding step (spec §9.1 row 4).
- **Path sources.** These use `Workspace::source_tree(dir)` (spec §6.3 last paragraph). `None`
  becomes plan 02's path-source refusal, `agentstacks.toml: packs.team-lint.path: resolves outside
  the project`, located at that entry's key path. The key is `packs.<key>.path` or
  `tools.<key>.path` (spec §13 "agentstacks.toml with key").

Each pack's tree hash is `tree_hash` over its tree. A tool's tree hash comes from
`Extension::load`, in Task 3.

The `Git` port returns `FetchError`s with no location (plan 02, plan 09), so the flow places each
variant (spec D3, §9.2 failure table, §13):

| Variant | Location | Shown as |
|---|---|---|
| `GitSpawn` | `SourceKey::location()` | `agentstacks.toml: packs.core: git: <io::Error Display>` (R1) |
| `GitFailed` | `SourceKey::location()` | `agentstacks.toml: packs.core: git <subcommand> failed: <git's stderr, trimmed>` (§9.2 row 2) |
| `GitUnreadable` | `SourceKey::location()` | `agentstacks.toml: packs.core: git <subcommand> printed output agentstacks cannot read: "<first line>"` (R2) |
| `SubdirNotFound`, `SubdirIsFile` | `SourceKey::subdir_location()` (plan 07, as `check_subdir_at` places it) | `agentstacks.toml: packs.core.subdir: "packs/nope" not found at <tag or rev>` / `agentstacks.toml: packs.core.subdir: "packs/core/pack.toml" is a file, not a directory` (§9.2 rows 4–5) |
| `CacheIo { path, .. }` | `Location::file(path)`, its own path (the `<rev>.git` target for a failed rename) | `<path>: <io::Error Display>` (R1, R28(b)) |
| `HomeUnset` | `agentstacks.toml`, no key, one refusal however many sources met it | `agentstacks.toml: HOME is not set, so the cache (~/.airsstack/cache) has no home` (§9.3, R10 "reported once") |
| `GitNotFound` | `agentstacks.toml`, no key: the per-source answers, each with an empty `needed_by`, are merged into one `FetchError::GitNotFound { needed_by }` listing each key in visiting order | `agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)` (§9.2 row 1, "once") |

`Fetched` is a flow-internal carrier. It holds, for each `SourceKey`, either the tree, the resolved
rev (git) and the tree hash, or that key's refusals. It also holds the `Note::SourceChanged` notes
and the fetched removed tools.

**Proof:** written first, seen failing, then passing.
- `fetch_sources_resolves_a_new_git_tag_to_its_rev` — with no lock and a tag pin, `FakeGit` logs `FetchTag` then `Tree` at the returned rev, and `Fetched` holds that rev and the snapshot's `tree_hash`.
- `fetch_sources_keeps_the_locked_rev_when_the_tag_moved` — the lock has rev R1 for tag T, and `FakeGit` maps T to R2. `FakeGit::calls()` holds only `Tree` at R1, with no `ListTags` or `FetchTag`, and `Fetched` holds R1 (spec §14 "moved tag").
- `fetch_sources_resolves_a_stale_entry_again_with_a_note` — the toml tag differs from the lock's tag. The tag is fetched as new, and the notes hold one `Note::SourceChanged` that displays exactly `agentstacks.toml: packs.core: source changed, resolved again` (spec §14 "stale lock").
- `fetch_sources_refuses_a_locked_git_tree_that_differs` — `FakeGit` serves other files at the lock's rev, which gives the §9.1 row 2 message with both hashes and the rev.
- `fetch_sources_accepts_a_changed_path_tree` — a locked path source whose files changed gives no refusal and the new hash.
- `fetch_sources_refuses_a_path_source_outside_the_project` — `FakeWorkspace::source_outside("packs/team-lint")` gives exactly `agentstacks.toml: packs.team-lint.path: resolves outside the project`.
- `fetch_sources_reports_git_missing_once_for_every_source` — `FakeGit::git_missing()` with one git pack and one git tool gives exactly one refusal: `agentstacks.toml: git not found on PATH (needed by packs.core, tools.claude)`.
- `fetch_sources_reports_home_unset_once` — two git sources whose fetches both answer `HomeUnset` give exactly one `agentstacks.toml: HOME is not set, so the cache (~/.airsstack/cache) has no home` (R10).
- `fetch_sources_places_git_failed_at_the_key` — a queued `GitFailed` for `fetch` with stderr `fatal: couldn't find remote ref refs/tags/packs-v9.9.9` gives exactly `agentstacks.toml: packs.core: git fetch failed: fatal: couldn't find remote ref refs/tags/packs-v9.9.9` (spec §9.2 row 2).
- `fetch_sources_places_git_spawn_at_the_key` — a queued `GitSpawn(denied)` for `tools.claude` gives exactly `agentstacks.toml: tools.claude: git: <denied Display>` (R1).
- `fetch_sources_places_git_unreadable_at_the_key` — a queued `GitUnreadable { subcommand: "rev-parse", first_line: "nope" }` gives exactly `agentstacks.toml: packs.core: git rev-parse printed output agentstacks cannot read: "nope"` (R2).
- `fetch_sources_places_subdir_refusals_at_the_subdir_key` — a queued `SubdirNotFound` for `packs/nope` at `packs-v0.1.0` gives exactly `agentstacks.toml: packs.core.subdir: "packs/nope" not found at packs-v0.1.0`. A queued `SubdirIsFile` for `packs/core/pack.toml` gives exactly `agentstacks.toml: packs.core.subdir: "packs/core/pack.toml" is a file, not a directory` (§9.2 rows 4–5).
- `fetch_sources_places_cache_io_at_its_own_path` — a queued `CacheIo { path: "/h/.airsstack/cache/git/<hex>/<rev>.git", error: denied }` gives exactly `/h/.airsstack/cache/git/<hex>/<rev>.git: <denied Display>`, with no `agentstacks.toml` prefix (R1, R28(b)).
- `fetch_sources_fetches_a_removed_tool_by_its_locked_rev` — a tool that is only in the lock is fetched at the lock's rev and listed among the removed tools.
- `fetch_sources_checks_a_removed_tools_tree_against_the_lock` — the removed tool's served tree differs from its lock `tree`, which gives the §9.1 row 2 message at `tools.<name>` (R18).
- `fetch_sources_fetches_nothing_for_a_removed_pack` — a pack that is only in the lock adds no `FakeGit` call.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow resolves and fetches sources`.

### Task 3 — Load and check the set

**Files:**
- Create `crates/agentstacks/src/app/plan/load.rs` — step 4.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod load;`.

**Change:** `load_set(&Inputs, Fetched) -> Result<Loaded, Diagnostics>` calls the built `check`
(spec §6.1 step 4, formats §9).
- Its pack loader runs `Pack::load` over the fetched tree for that key, and its tool loader runs
  `Extension::load` the same way.
- A key whose fetch was refused makes its loader return that key's fetch refusals. `check` then
  reports them with every other refusal and runs no second check on that entry
  (`src/types/set/check.rs` responsibilities).
- Each removed tool's fetched tree is loaded with `Extension::load` on its own, because the toml
  no longer lists it and `check` cannot see it (spec §6.4).

Two kinds of plan 06 `Note` are added (spec §6.1 "notes", formats §2.1, §2.2, §16 R20):
- `Note::Ignored { root, path }` for each dot entry in `Pack::ignored` and `Extension::ignored`.
  `root` is the loaded directory's label, `Pack::dir()` or the extension's `dir()`. For a path
  source that is its path (`packs/core`). For a git source it is the tree's root label, the
  `subdir` as written or `.` without one (R22). It displays as `<pack>/<path>: ignored (dot
  entry)`.
- `Note::UnusedNative { dir, tool }` for each `Pack::unused_native(tools)`, with the toml's tool
  names as `tools`. It displays as `native/cluade: not for claude`.

`Loaded` is a flow-internal carrier that holds:
- the `ValidSet`;
- the removed tools' `Extension`s;
- the trees, revs and tree hashes;
- the `Note`s.

**Proof:** written first, seen failing, then passing.
- `load_set_returns_the_valid_set_of_fetched_sources` — a valid pack and a valid extension give a `ValidSet` with one pack and one extension.
- `load_set_reports_fetch_and_load_refusals_together` — one pack whose fetch was refused and one whose `pack.toml` says `api = 1` give both refusals and no set.
- `load_set_loads_a_removed_tool_outside_the_set` — the removed tool's extension is in `Loaded` and absent from `ValidSet::extensions`.
- `load_set_notes_dot_entries_and_unused_native_dirs` — a path-source pack at `packs/core` with `.DS_Store` and `native/cluade/`, in a project listing only `claude`, gives notes that display exactly `packs/core/.DS_Store: ignored (dot entry)` and `native/cluade: not for claude` (R20).
- `load_set_labels_a_git_dot_entry_by_its_subdir` — a git pack with `subdir = "packs/core"` and a `.DS_Store` gives `packs/core/.DS_Store: ignored (dot entry)`. Without a `subdir` it gives `./.DS_Store: ignored (dot entry)` (R20, R22).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow loads and checks the set`.

### Task 4 — Render a tool's packs with its values

**Files:**
- Create `crates/agentstacks/src/app/plan/render.rs` — the render part of step 5.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod render;`.

**Change:** The `values` payload is plan 04's `ValuesInput::new(tool, packs)`, called by Task 5
with the tool's packs. Plan 04 derives and sorts the `vars` (spec §16 R8), so this folder computes
none.

`render_packs(&Loaded, &ToolName, &ValuesResult) -> Rendered` renders every text formats §3.4 lists:
- `.md` files and `render` files, as whole files;
- frontmatter string values;
- hook `text`, `args`, `run` and grant paths in `pack.toml`;
- the same fields in this tool's `native/<tool>/hooks.toml`;
- this tool's native `.md` files.

Each text goes through four calls:
1. scan it with the built `scan`;
2. ask plan 04's `ValuesResult::value(target, pack)` for each token's value. `Given(s)` is used as
   is, and `FromOption` takes `ValidSet::option` (formats §4.4);
3. put each value in with `TokenValues::insert`;
4. splice with `render` (spec §4.4 "Every token is filled through `TokenValues::insert` and
   `render`").

An item with no value is skipped through plan 04's missing-value rule and left out of the plan
payload (spec §4.4 "Missing values"). R8 fixes which reason is used:
- a skill with no `skills[...]` entry gets `skip_skill_without_location` first, which reads
  `claude gave no install location for skill core/status`;
- otherwise the first `Missing` token in text order names the reason, for example `claude gave no
  value for {{skill core/status}}`. This goes through `skip_for_missing_value` (a skill is skipped
  as its whole group; an agent, hook, native file or native hook as itself) or, for a root file,
  `skip_root_file_for_missing_value`.

A `copy` installs a file's final bytes, kept by plan 04 item id (spec §3.1 `copy` row):
- a rendered file: the whole rendered file, including the neutral frontmatter of `SKILL.md` and
  agents;
- any other file: its raw bytes from the source tree.

A lua hook's `command` argv is plan 08's `HookCommand` over the rendered hook fields (spec §4.4,
§11.2). `HookCommand` takes the root as a `SubPath`, so the flow converts `ValuesResult::root()`
(an `OpPath`) with `SubPath::new(root.as_str())`. Both types are normalised project-relative paths,
and `OpPath`'s shape rule is stricter, so the conversion cannot fail for a valid `OpPath`. The
`Result` is still propagated, never unwrapped (workspace lint `unwrap_used`). The rendered parts go to plan 04's item-id assignment and `PlanInput` construction. That
includes R11's native-hook shape, which is plan 04's. `Rendered` is a flow-internal carrier that
holds:
- the `PlanInput`;
- the final bytes by item id;
- the rendered root files;
- the `CoreSkip`s.

**Proof:** written first, seen failing, then passing.
- `render_packs_fills_every_token_kind_from_the_values` — one `SKILL.md` holds `{{root}}`, `{{root core}}`, `{{skill core/status}}`, `{{skill_dir core/status}}`, `{{agent core/coder}}`, `{{var tmpdir}}` and `{{option archive_keep}}`. With the §4.4 example values result it renders to `.claude/agentstacks/core`, `.claude/agentstacks/core`, `/status`, `.claude/skills/status`, `coder`, `$TMPDIR` and the option's value.
- `render_packs_skips_a_skill_with_a_missing_value` — with no value for `{{skill core/status}}`, the whole `core/skill/status` group is skipped with exactly `claude gave no value for {{skill core/status}}`, and it is absent from `PlanInput`.
- `render_packs_names_the_first_missing_token_in_text_order` — an agent body missing `{{var home}}` and then `{{skill core/x}}` gets the reason naming `{{var home}}` (R8).
- `render_packs_prefers_a_missing_skill_entry_over_a_missing_token` — a skill with no `skills["core/status"]` entry that also holds a token with no value gets exactly `claude gave no install location for skill core/status` (R8).
- `render_packs_skips_only_the_agent_hook_or_native_item_missing_a_value` — an agent, a hook `run`, a native file and a native hook with a missing value are each skipped as themselves, and the items beside them stay in `PlanInput`.
- `render_packs_makes_a_core_skip_for_a_root_file_missing_a_value` — a rendered root file with a missing value becomes a `CoreSkip` of that file.
- `render_packs_keeps_raw_bytes_for_unrendered_files` — a binary `logo.png` keeps its bytes and has no `text` in `PlanInput`. A rendered `SKILL.md`'s final bytes include its rendered frontmatter.
- `render_packs_builds_the_hook_command_argv` — a lua hook's `command` equals the `HookCommand` argv for its rendered `run` and grants.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow renders packs with tool values`.

### Task 5 — Run each tool's extension and check its operations

**Files:**
- Create `crates/agentstacks/src/app/plan/tools.rs` — steps 5–6.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod tools;`.

**Change:** `run_tools<W: Workspace, A: Adapter>(&Loaded, &W, &A) -> Ran` runs the listed tools in
name order, then the removed tools in name order (spec §16 R6). For each listed tool:
1. `Adapter::open(extension, files)` with the tree the extension was loaded from.
2. `Adapter::values(session, &ValuesInput::new(tool, packs))` returns a `ValuesResult` or that
   tool's refusals.
3. `render_packs` (Task 4).
4. The tool's core writes are recorded with plan 04's `Proposals::record_core(tool, proposal,
   "agentstacks.toml")`, just before the `plan` dispatch (R6):
   - plan 04's `root_file_write` for each pack root file, to `<root>/<pack>/<rel>` with its
     rendered or raw bytes (spec D7, §3.1 row 5);
   - plan 04's `runtime_write` with plan 08's `RUNTIME_LUA`, when plan 04's `runtime_needed(packs,
     tool)` holds. That is true when a pack has any lua hook, neutral or native, whether or not the
     extension later skips it (R11).

   A core write that meets an earlier tool's path is refused by `record_core` with R6's
   `agentstacks.toml: tools.opencode: path "<p>" is already proposed by claude`.
5. `Adapter::plan(session, &plan_input, workspace, &mut proposals)` returns the tool's `Reads`
   (spec §4.3). Inside it, the call-time duplicate rule (spec §3.3, plan 04 `Proposals`) can
   refuse with any of:
   - `path ".claude/x" is already proposed at adapter.lua:9`
   - `… already proposed by opencode`
   - `… already proposed by core (root file of core)`
   - R6's `path "<p>" is already proposed by core (runtime.lua of claude)`
6. Plan 04's `check_coverage` (spec §3.2) can refuse with `claude/adapter.lua:
   core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a
   reason)`.
7. Plan 04's `check_translate` applies when `runtime.lua` was written and at least one lua hook is
   covered by a write (R11). It can refuse with `claude/adapter.lua: lua hooks need
   <root>/translate.lua (write it)`.

Each removed tool goes through these steps with four differences (spec §6.4 bullets 1–2, R18,
R28(g)):
- its payload is `ValuesInput::new(tool, &[])`, with an empty `packs` list;
- nothing is rendered, and no core write is made for its root;
- `Adapter::plan` is given a separate, empty `Proposals`, so its ops are still checked against each
  other (R28(g));
- plan 06's `keep_shared_writes` then keeps only its writes to paths it read in this run, and only
  those kept writes are put into the run's `Proposals` with `Proposals::propose`. Its dropped
  proposals never meet the other tools' proposals (R18, R28(g)).

Adapter refusals are collected (spec §4.6 "Any extension error refuses the whole plan"). Examples
are `claude/adapter.lua: registers no values handler`, `claude/adapter.lua: registers no plan
handler`, §3.3 op refusals and Lua failures. A tool whose `open` or `values` failed gets nothing
rendered and no `plan` call. `Ran` is a flow-internal carrier that holds:
- the `Proposals`;
- each tool's `Reads` and its `extension.reads` (plan 03);
- the skips (the first reason for each, spec §3.2) and the `CoreSkip`s;
- the final bytes by item id.

**Proof:** written first, seen failing, then passing.
- `run_tools_runs_tools_in_name_order` — for tools `opencode` and `claude`, `FakeAdapter::calls()` holds `claude` open, values and plan, then `opencode` open, values and plan.
- `run_tools_runs_removed_tools_after_listed_tools_in_name_order` — listed `claude` and removed `aider` give `claude` calls first, then `aider` calls (R6).
- `run_tools_dispatches_plan_with_the_rendered_payload` — the `PlanInput` in the `Plan` call log holds the rendered text, not the raw token.
- `run_tools_records_core_writes_just_before_the_plan_dispatch` — the `Proposals` handed to `FakeAdapter::plan` already holds the tool's core writes, recorded through `record_core`. The previous tool's plan proposals come before them (R6).
- `run_tools_writes_root_files_under_the_values_root` — a pack root file `hooks/enforce.lua` becomes a core write to `.claude/agentstacks/core/hooks/enforce.lua`, owned by `core`.
- `run_tools_writes_runtime_lua_even_when_the_lua_hook_is_skipped` — a pack with one lua hook, which the extension skips, still gets the `RUNTIME_LUA` write to `.claude/agentstacks/runtime.lua`, owned by the tool (R11).
- `run_tools_writes_no_runtime_lua_without_lua_hooks` — a pack with only message hooks gets no `runtime.lua` proposal.
- `run_tools_refuses_a_path_an_earlier_tool_proposed` — `opencode` writing a path `claude` already wrote gives the refusal ending `is already proposed by claude`, at `opencode`'s call line.
- `run_tools_refuses_an_extension_write_onto_a_root_file` — an extension write to `.claude/agentstacks/core/hooks/enforce.lua` gives the refusal ending `is already proposed by core (root file of core)`.
- `run_tools_refuses_an_extension_write_onto_runtime_lua` — a `claude` write to `.claude/agentstacks/runtime.lua` gives, at its call line, `path ".claude/agentstacks/runtime.lua" is already proposed by core (runtime.lua of claude)` (R6).
- `run_tools_refuses_a_core_write_onto_an_earlier_tools_path` — `claude` writes `.agents/core/hooks/enforce.lua`, and `opencode`'s values root is `.agents`. This gives exactly `agentstacks.toml: tools.opencode: path ".agents/core/hooks/enforce.lua" is already proposed by claude` (R6).
- `run_tools_refuses_an_uncovered_item` — leaving `core/skill/status/references/x.md` uncovered gives exactly `claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a reason)`.
- `run_tools_refuses_lua_hooks_without_translate_lua` — a lua hook covered by a write, with no write to `.claude/agentstacks/translate.lua`, gives exactly `claude/adapter.lua: lua hooks need .claude/agentstacks/translate.lua (write it)`.
- `run_tools_needs_no_translate_lua_when_every_lua_hook_is_skipped` — `runtime.lua` is written but every lua hook is skipped, so there is no translate refusal (R11).
- `run_tools_collects_adapter_refusals_from_every_tool` — `claude` with no values handler and `opencode` with no plan handler give both refusals, and `claude` gets no `Plan` call.
- `run_tools_runs_a_removed_tool_with_zero_packs` — the removed tool's `ValuesInput` has an empty `packs` list. Only its write to a path it read is kept, its other write is dropped, and no core write is made for its root (spec §14 "removed tool").
- `run_tools_keeps_a_removed_tools_dropped_write_out_of_the_duplicate_check` — the removed tool writes, without reading it, a path `claude` already wrote. There is no refusal, and the write is dropped (R18).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow runs tool extensions`.

### Task 6 — Check binaries and the airsl pin

**Files:**
- Create `crates/agentstacks/src/app/plan/binaries.rs` — step 7.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod binaries;`.

**Change:** `check_binaries<B: Binary>(&Loaded, &Ran, &B) -> Binaries` starts from plan 04's
`collect_binaries(packs, proposals, tools)`. That gives each pack's `requires.binaries` (spec §3.1
row 6), then each tool's `require_binary` names, once per tool (spec §3.3 "checked once"). Each
requirement is asked with `Binary::on_path` (spec §6.1 step 7). A miss gives
`BinaryRequirement::not_found`, once for each requirer (spec §10):
- a pack: `agentstacks.toml: packs.core: binary "rg" not found on PATH`;
- an extension: `claude/adapter.lua:7: require_binary: "node" not found on PATH`.

A hit becomes a plan 06 `BinaryCheck`.

When `ValidSet::airsl()` is `Some`, the flow asks `Binary::airsl_version()` (spec §10 row 3):
- `NotFound` gives plan 04's `airsl_not_found`: `agentstacks.toml: airsl not found on PATH
  (needed by core, sdlc)`;
- `Printed(text)` that plan 04's `parse_airsl_version` cannot parse gives `airsl_unparseable`:
  `agentstacks.toml: airsl --version printed "<text>", expected "airsl <version>"`. Here `<text>`
  is trimmed, and a non-zero airsl exit counts as unparseable (R2). An airsl spawn failure shows
  R1's `agentstacks.toml: airsl: <io::Error Display>`;
- a parsed version outside the range gives the built `AirslRange::installed_refusal`:
  `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4`.

The installed version is kept for plan 06's `airsl_pin` (spec §7 "Lock content", §10). A plan 06
`AirslPin { locked, installed }` records the move that `plan` shows as `airsl 0.1.4 → 0.1.5`
(spec §10). With no range, airsl is never asked. `Binaries` is a flow-internal carrier holding the
`BinaryCheck`s, the installed version, the `AirslPin` and the refusals.

**Proof:** written first, seen failing, then passing.
- `check_binaries_refuses_a_missing_pack_binary` — `rg` missing from `FakeBinary` gives exactly `agentstacks.toml: packs.core: binary "rg" not found on PATH`.
- `check_binaries_refuses_a_missing_extension_binary` — `require_binary{name="node"}` at `adapter.lua:7` with `node` missing gives exactly `claude/adapter.lua:7: require_binary: "node" not found on PATH`.
- `check_binaries_reports_a_binary_once_for_the_pack_and_once_for_the_extension` — `rg` required by `core` and by `claude` gives two refusals, one of each shape.
- `check_binaries_checks_a_repeated_name_once` — two `require_binary{name="node"}` ops from one tool give one `OnPath(node)` in `FakeBinary::asked()`.
- `check_binaries_refuses_airsl_not_on_path` — `core` and `sdlc` declare `requires.airsl` with no airsl configured, which gives exactly `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)`.
- `check_binaries_refuses_unparseable_airsl_version_text` — `FakeBinary::airsl("airsl-dev\n")` gives exactly `agentstacks.toml: airsl --version printed "airsl-dev", expected "airsl <version>"` (R2, trimmed).
- `check_binaries_refuses_airsl_out_of_range` — `airsl 0.1.3` against `>=0.1.4` gives exactly `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4`.
- `check_binaries_moves_the_pin` — lock `[airsl] version = "0.1.4"` with `airsl 0.1.5` installed gives installed version `0.1.5` and `AirslPin { locked: Some(0.1.4), installed: 0.1.5 }`.
- `check_binaries_skips_airsl_when_no_pack_declares_it` — `FakeBinary::asked()` holds no `AirslVersion`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow checks binaries and airsl`.

### Task 7 — Decide every path's step

**Files:**
- Create `crates/agentstacks/src/app/plan/steps.rs` — steps 8–9.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod steps;`.

**Change:** `decide_steps<W: Workspace>(&Inputs, &Ran, &W) -> Decided` covers two kinds of path
(spec §6.1 steps 8–9, §6.3).

- **Every proposed path** (core writes, `write`, `copy`, in `Proposals` order): the flow asks
  `Workspace::resolve` and gets a `Resolution` with the disk state. It builds plan 06's
  `ProposedPath` from:
  - the new bytes (a `copy`'s are the final bytes of its file id, Task 4);
  - the origin;
  - the plan 04 owner;
  - `Reads::shared(path, extension.reads)` for the proposing tool (spec §6.2 "Shared");
  - the lock hash, when the path has a `[[file]]`;
  - the `Resolution`.

  That goes to `decide_proposed`.
- **Every lock `[[file]]` whose path is not proposed in this plan** gets a step from the §6.2
  second table, whoever owns it. That covers files of removed packs (spec §9.1 row 4) and of
  removed tools (spec §6.4 bullet 3). It also covers files a still-listed pack or tool stopped
  proposing, for example a skill a new pack version dropped. Each path is resolved the same way
  and goes to `decide_unproposed(path, entry, resolution)`, with the entry's path as an `OpPath`.

The refusals come from plans 06/02 and are not reworded here:
- `<path>: is not a regular file`
- `<path>: changed since the last apply`
- `<path>: exists and was not written by agentstacks`
- `<path>: changed since the last apply; not deleting it`
- `<path>: resolves outside the project`
- `<path>: changed after it was read; run plan again` (R15)

A `Workspace::resolve` I/O failure gives `<path>: <io::Error Display>` (R1). `Decided` is a
flow-internal carrier holding the `Step`s and the refusals. Each `Step` carries its `decided_on:
Option<DiskState>`, which is `None` for `forget` (spec §6.1 `Plan` bullet 5, plan 06).

**Proof:** written first, seen failing, then passing.
- `decide_steps_forgets_an_absent_locked_path` — a locked path that is no longer proposed and is absent gives `forget`.
- `decide_steps_forgets_a_locked_path_now_a_dangling_symlink` — `FakeWorkspace::symlink` to a missing target gives `forget`, not `is not a regular file` (R15, checked before any other row).
- `decide_steps_refuses_to_delete_a_changed_locked_path` — a hash that differs from the lock gives exactly `<path>: changed since the last apply; not deleting it`.
- `decide_steps_refuses_a_proposed_path_that_is_not_a_regular_file` — `FakeWorkspace::not_regular(".claude/x")` gives exactly `.claude/x: is not a regular file`.
- `decide_steps_refuses_a_shared_file_changed_after_it_was_read` — the `Reads` record one hash and `FakeWorkspace` now holds other bytes, which gives exactly `.claude/settings.json: changed after it was read; run plan again` (R15).
- `decide_steps_keeps_old_bytes_for_a_shared_file` — a path that is in `extension.reads` and was read gives a step holding the disk bytes as `old`.
- `decide_steps_reports_a_resolve_failure_with_the_io_error` — `FakeWorkspace::fail(Resolve(".claude/x"))` gives `.claude/x: <denied() Display>` (R1).
- `decide_steps_records_the_disk_state_each_step_was_decided_on` — every non-`forget` `Step`'s `decided_on` is `Some` of the state `FakeWorkspace` returned, and a `forget` step's is `None`.
- `decide_steps_decides_a_locked_path_a_listed_pack_stopped_proposing` — a `core`-owned `[[file]]` that nobody proposes now, with `core` still listed, gets a second-table step (`delete` when its hash equals the lock).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow decides each path's step`.

### Task 8 — Assemble the plan flow

**Files:**
- Create `crates/agentstacks/src/app/plan/flow.rs` — the public flow.
- Modify `crates/agentstacks/src/app/plan/mod.rs` — `mod flow;` and `#[doc(inline)] pub use flow::plan;`.

**Change:** `plan<W: Workspace, G: Git, B: Binary, A: Adapter>(&W, &G, &B, &A) -> Result<Plan,
Diagnostics>` runs Tasks 1–6 in the order of spec §6.1 steps 1–7. It collects every refusal.

- **Before steps 8–9.** If steps 1–7 raised any refusal, the flow returns them without running
  steps 8–9: no path is resolved, no step is decided, and so no false delete appears (spec §16
  R7). A step whose input does not exist is skipped on the way: with no `ValidSet` there are no
  tools to run, and a tool whose `values` was refused gets nothing rendered.
- **Steps 8–9.** Otherwise Task 7 runs. Its refusals are returned, all of them, in one
  `Diagnostics`, which the caller sorts with `into_sorted` (spec §6.1 "Refusals are collected,
  never first-only, and sorted").
- **Step 10.** With no refusal, the flow builds the new lock and calls plan 06's `Plan::new`:
  1. plan 06's `lock_contents(tools, packs, steps, airsl_pin(…), Landed::All)` gives the contents
     (spec §7 "Lock content"). The tools and packs are built `LockedEntry` values with the revs
     and tree hashes from Task 2. Nothing from a removed tool is locked (spec §6.4 bullet 4);
  2. `Lockfile::new` turns them into the new `Lockfile`;
  3. `check_airsl_presence` runs on that new lock only, with the set's pack manifests (R27);
  4. `Plan::new` takes these parts:
     - the steps;
     - the skips: the extensions' skips and the `CoreSkip`s;
     - the `BinaryCheck`s;
     - the `Note`s (`Note::SourceChanged`, `Note::Ignored`, `Note::UnusedNative`);
     - the `AirslPin`;
     - the new `Lockfile`;
     - `old_files`: the step-1 lock's `[[file]]` entries as read, empty when there was no lock.
       Plan 14 needs them for the R17 lock.

The doc comment states the steps in prose and carries no example, because the fakes are
`#[cfg(test)]`. The tests below stand in for it.

**Proof:** written first, seen failing, then passing. These tests drive every spec §6.2 checked
example through `plan` with fakes only (spec §14 app row).
- `plan_creates_an_absent_skill_file_on_first_apply` — no lock, `.claude/skills/status/SKILL.md` absent and a `copy` of `core/skill/status/SKILL.md` give `create`. The new lock has its `[[file]]` with `by = "core"`.
- `plan_refuses_a_skill_file_edited_after_apply` — a disk hash that differs from the lock and from the new bytes gives exactly `.claude/skills/status/SKILL.md: changed since the last apply`.
- `plan_restores_a_deleted_skill_file` — locked and absent gives `restore`.
- `plan_deletes_the_untouched_file_of_a_removed_pack` — `team-lint` is in the lock but not the toml, and its file hash equals the lock. The step is `delete`, and the new lock has no `team-lint` pack and no file of it (spec §14 "removed pack").
- `plan_updates_a_shared_settings_file_without_locking_it` — `.claude/settings.json` is in `extension.reads`, was read with the user's permissions and is proposed with hooks added. The step is `update` with the old bytes kept, and there is no `[[file]]` for it.
- `plan_creates_an_absent_shared_settings_file` — `read` returned absent and the extension proposes the file, which gives `create`, with no `[[file]]`.
- `plan_refuses_an_agent_file_not_written_by_agentstacks` — the file exists with different bytes and is not locked, which gives exactly `.claude/agents/coder.md: exists and was not written by agentstacks`.
- `plan_refuses_a_proposed_symlink_to_etc_hosts` — `FakeWorkspace::outside(path)` gives exactly `<that path>: resolves outside the project`.
- `plan_removes_a_tool_by_running_its_locked_extension` — `opencode` is only in the lock. Its extension runs with zero packs, its shared write is a step, and its locked files get `delete`. The new lock has no `opencode` `[[tool]]` and no file of it (spec §14 "removed tool").
- `plan_refuses_while_a_removed_tool_source_is_unreachable` — a queued `FakeGit` failure on the removed tool's rev returns the git refusal and no `Plan`, so nothing is removed (spec §9.2 last row).
- `plan_skips_deciding_after_an_earlier_refusal` — `claude`'s plan handler is missing while the lock holds `claude`-written files. Only `claude/adapter.lua: registers no plan handler` is returned. `FakeWorkspace::calls()` holds no `Resolve`, and no delete step or `not deleting it` refusal appears (R7).
- `plan_deletes_a_skill_a_new_pack_version_dropped` — `core` stays listed, but its new version no longer has skill `old`. The untouched, locked `.claude/skills/old/SKILL.md` gets `delete`, and the new lock has no `[[file]]` for it.
- `plan_locks_resolved_revs_trees_and_airsl` — the new lock holds each tool's and pack's rev and tree from Task 2, and `[airsl] version` equal to the installed version.
- `plan_carries_the_old_file_entries` — `Plan::old_files` equals the step-1 lock's `[[file]]` entries, and is empty with no lock.
- `plan_checks_airsl_presence_on_the_new_lock_only` — a step-1 lock with `[airsl]` while no pack declares `requires.airsl` gives no refusal, and the new lock has no `[airsl]` (R27).
- `plan_collects_every_refusal_from_the_deciding_step_sorted` — two `changed since the last apply` refusals on two paths are both returned, in `Diagnostics::into_sorted` order.
- `plan_with_any_refusal_returns_no_plan` — any refusal makes the result `Err`, so no `Plan` that holds a refusal can reach `apply` (spec §7 step 1).

**Done when:** `cargo make dod` passes and the Definition of Done is green. Commit `feat(agentstacks): plan flow from project to reviewable plan`.

---

## Risks

- **Names from other plans.** Port names follow plan 09: `project_text`, `lock_text`,
  `resolve`, `source_tree`, `fetch_tag`, `tree`, `on_path`, `airsl_version`, `open`/`Session`,
  `values`, `plan`. Rule names follow plans 04, 06 and 07 as they read now (`ValuesResult::value`,
  `ValuesInput::new`, `skip_*`, `Proposals::propose`, `Proposals::record_core`, `check_coverage`,
  `check_translate`, `root_file_write`, `runtime_write`, `runtime_needed`, `collect_binaries`,
  `parse_airsl_version`, `decide_proposed`, `decide_unproposed`, `keep_shared_writes`,
  `lock_contents` with `Landed`, `airsl_pin`, `Note`, `Plan::new`, `reconcile`, `check_tree`,
  `SourceKey::location`/`subdir_location`). If a plan renames one while aligning to §16, the coder follows that
  plan's name. The behaviour cited here stays.
- **Riskiest task: Task 4, rendering.** Tokens are scanned again per text, so their spans index
  the exact text being rendered. A token missed by this scan would render silently as its literal
  text. The tests cover every token kind and every text source formats §3.4 lists.
- **Not done here:** turning `Plan` into text, including the diff (cli-commands, spec §17).
  Writing to disk (plan 14). The fs, process and host contract tests (plans 10–12).

## Proof

- Spec rows covered:

  | Rows | Tasks |
  |---|---|
  | §6.1 steps 1–10 | 1–8 |
  | §6.1 `Plan` contents | 8 |
  | §6.2 every checked example | 8 |
  | §6.2 second-table rows and R15 | 7 |
  | §6.3 last paragraph | 2 |
  | §6.3 resolution of every proposed and deleted path | 7 |
  | §6.4 all bullets | 2, 5, 7, 8 |
  | §4.4 order, token table, missing values, `command`, tool order | 4–5 |
  | §4.6 collecting extension refusals | 5 |
  | §3.1 core writes and core `require_binary` | 5–6 |
  | §3.2 coverage and `translate.lua`; §3.3 duplicates | 5 |
  | §9.1 rows 1–5 and the moved tag | 2 |
  | §9.2 failures, git-missing once, removed-tool source row | 2, 8 |
  | §9.3 HOME | 2 |
  | §10 all rows | 6 |
  | §7 "Lock content" | 8 |
  | §16 R1 | 1, 2, 6, 7 |
  | §16 R2 | 2, 6 |
  | §16 R6 | 5 |
  | §16 R7 | 8 |
  | §16 R8 (missing-value order; vars are plan 04's) | 4 |
  | §16 R10 | 2 |
  | §16 R11 | 5 |
  | §16 R15 | 7 |
  | §16 R18, R28(g) | 2, 5 |
  | §16 R20, R22 | 3 |
  | §16 R28(b) | 2 |
  | §13 location of every `FetchError` variant | 2 |
  | every locked path not proposed gets a second-table step | 7, 8 |
  | §16 R27 | 1, 8 |
  | §14 app row: checked examples, removed pack, removed tool, stale lock, moved tag | 2, 5, 8 |

- Every test uses only `FakeWorkspace`, `FakeGit`, `FakeBinary`, `FakeAdapter` and `FakeFileTree`. `tests/isolation.rs` (plan 01) keeps `src/app` free of `std::fs`, `std::process` and adapter imports.
- `cargo make dod` is green at the end of Task 8.
