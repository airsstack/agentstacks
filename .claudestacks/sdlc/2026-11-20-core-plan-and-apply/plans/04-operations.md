---
status: done
created: 2026-11-22
depends-on: [02]
built-on: f39b2a814319
---

# Operations Implementation Plan

**Goal:** Validate what an extension proposes.

**Architecture:** `types/op` is the one home of `Operation` and its rules (spec §2 table row
`types/op`, modularity Rule 2). It is pure: values in, an answer or a refusal out. It never reads
Lua, disk, git or `PATH`. The host (plan 12) reads Lua arguments strictly and builds these values
through the constructors here. The plan flow (plan 13) builds the payloads, renders, and calls the
collection and coverage rules. The step rules (plan 06) consume `Proposal`, `Origin`, `OpPath` and
the owner rule.

```text
crates/agentstacks/src/types/op/
├─ mod.rs        docs + mod / pub use only
├─ error.rs      OpError                                   (plan 02, not edited here)
├─ word.rs       LuaType, ApiFunction                      (plan 02, not edited here)
├─ shape.rs      PathShape                                 (plan 02, not edited here)
├─ path.rs       OpPath: shape, normalisation, own-file rule (§3.3)
├─ id.rs         ItemId: the §3.2 id formats, pack and skill group of an id
├─ origin.rs     Origin: who proposed an op, and where
├─ proposal.rs   Operation + Proposal (§3.1)
├─ skip.rs       SkipReason, SkipSubject, CoreSkip
├─ owner.rs      owner: the derived `by` of a proposal (§3.1)
├─ root.rs       the core writes under <root>: root files, runtime.lua (§3.1 rows 5, D7)
├─ proposals.rs  Proposals: duplicate-path and skip/cover-conflict rules, in call order (§3.2, §3.3)
├─ coverage.rs   the coverage check at the end of a tool's plan (§3.2, D6)
├─ translate.rs  the translate.lua rule (§3.2 last row)
├─ missing.rs    the missing-value skip rule (§4.4 "Missing values")
├─ binaries.rs   BinaryRequirement: what to look up on PATH (§10 rows 1–2, §3.1 row 6)
├─ airsl.rs      the `airsl --version` rule (§10 row 3)
├─ payload/      the two payloads the core hands an extension, as domain values (§4.4)
│  ├─ mod.rs     docs + mod / pub use only
│  ├─ values.rs  ValuesInput, ValuesPack
│  ├─ plan.rs    PlanInput, PlanPack, PlanAgent, PlanNative: id lookup, items to cover
│  ├─ skill.rs   PlanSkill
│  ├─ file.rs    PlanFile
│  └─ hook.rs    PlanHook, PlanNativeHook, PlanAction
└─ answer/       the values result, read and used (§4.4)
   ├─ mod.rs     docs + mod / pub use only
   ├─ read.rs    ValuesResult, SkillValues: the values-table rules
   └─ lookup.rs  TokenValue: the token → value table
```

**Dependency direction:**
- `types/op` uses `types::{name, tokens, pack, manifest, set, tree, lock::Owner, report, api}` and
  `serde_json` (for the values object only).
- It imports no adapter, no `app`, and nothing from `port` (spec §2 dependency rule, §16 X2: `types`
  may import only `port::{FileTree, EntryKind}`, and `types/op` needs neither). Its unit tests load
  packs from `FakeFileTree` under `#[cfg(test)]`, as the built `set/testing.rs` does. No doc
  example uses a fake. `tests/isolation.rs` (plan 01) enforces the rule.
- Nothing in `types/op` imports `types/plan`, `types/json` or `types/protocol`. The runtime.lua
  bytes are passed in by the caller.

**Boundaries with other plans** (referenced, never copied):
- **Plan 02:** `OpError` (every §3.3, §3.2, §4.4, §10 and §16 R4/R6/R12/R13 message, including
  `OpPath { function, field, shape }` and `ValuesPath { key, shape }`), `PathShape`
  (`src/types/op/shape.rs`), `LuaType` and `ApiFunction`. Every refusal below is the plan 02 value
  for the spec row cited, and plan 02's tests pin the Display.
- **Plan 12 (host):** argument reading (§3.3 rows 1–6: one table, required, exact type, list,
  unknown key, UTF-8), the phase rule (§3.3 last row), `read` (§3.3 `read` row, §4.3), the
  `values` dispatch outcome (no handler, `nil`, non-object; §4.4 rows 1–2) and the JSON payloads.
  `ValuesResult::read` takes the host's JSON→`LuaType` word function as a parameter, so the word
  rule exists once.
- **Plan 13 (flow):** builds `ValuesInput` and `PlanInput`, renders, records core writes in the
  §16 R6 order, and calls the rules here.
- **Plan 09 (ports):** the source of truth for port signatures. `Adapter::values` returns
  `Result<ValuesResult, Diagnostics>`, and `Adapter::plan` takes `&mut Proposals`. `ValuesInput`,
  `PlanInput` and `ValuesResult` derive `Clone` for its fake.
- **Plan 06 (steps):** consumes `Proposal`, `Origin`, `OpPath`, `owner` and `CoreSkip`.

**Rust guideline rules applied** (claudestacks-guideline-rust 0.1.6):
- `modularity.md` Rule 1: one job per file, as in the tree above. `Proposals` holds only the
  call-order rules; coverage and translate are their own files.
- `modularity.md` Rule 2: `Operation`, `OpPath`, `ItemId`, `Origin` exist only here. The owner is
  the built `Owner` (`src/types/lock/owner.rs`), the binary name the built `BinaryName`
  (`src/types/name/binary.rs`), the token the built `Token`/`TokenTarget`, and the out-of-range
  airsl refusal the built `AirslRange::installed_refusal` (`src/types/set/airsl.rs`).
- `mod-rs-export-only.md`: `op/mod.rs`, `payload/mod.rs` and `answer/mod.rs` hold only the
  four-question `//!` block, `mod` lines and `pub use` lines. Callers use `types::op::…` (they are
  not flat re-exported from `types/mod.rs`, plan-set map plan 01).
- `strong-types.md`: `OpPath`, `ItemId` and `SkipReason` are newtypes, valid on construction, with
  `# Errors` docs and a doctest of the happy and refused path. No `bool` parameter.
- `static-dispatch.md`: the only abstraction is the word-function parameter of
  `ValuesResult::read`, a generic `F: Fn(&serde_json::Value) -> LuaType`. There is no `dyn`.
- `unit-test-mandate.md`: every non-`mod.rs` file has a colocated `#[cfg(test)] mod tests`.
- `doc-comment-discipline.md`: tool names in docs are labelled examples ("example: claude
  extension"). No plan, spec or agent words appear in source.
- Naming: one-word files and folders, no `_`, no repeated folder word (`op/path.rs`, not
  `op/oppath.rs`).
- The core is tool-agnostic: no tool shape appears here; `claude` appears only as test data.

**Tech Stack:** Rust 2024; built `agentstacks::types` modules; `serde_json` 1.0.154 (workspace,
plan 01) for the values object. No new dependency.

---

## Files that change

```
crates/agentstacks/src/types/op/mod.rs            — modify: module docs, mod / pub use lines for every file below
crates/agentstacks/src/types/op/path.rs           — create: OpPath
crates/agentstacks/src/types/op/id.rs             — create: ItemId
crates/agentstacks/src/types/op/origin.rs         — create: Origin
crates/agentstacks/src/types/op/proposal.rs       — create: Operation, Proposal
crates/agentstacks/src/types/op/skip.rs           — create: SkipReason, SkipSubject, CoreSkip
crates/agentstacks/src/types/op/owner.rs          — create: owner
crates/agentstacks/src/types/op/root.rs           — create: root_file_write, runtime_write, runtime_needed
crates/agentstacks/src/types/op/proposals.rs      — create: Proposals
crates/agentstacks/src/types/op/coverage.rs       — create: check_coverage
crates/agentstacks/src/types/op/translate.rs      — create: check_translate
crates/agentstacks/src/types/op/missing.rs        — create: the missing-value skip rule
crates/agentstacks/src/types/op/binaries.rs       — create: BinaryRequirement, collect_binaries
crates/agentstacks/src/types/op/airsl.rs          — create: airsl --version parse and refusals
crates/agentstacks/src/types/op/payload/mod.rs    — create: export-only
crates/agentstacks/src/types/op/payload/values.rs — create: ValuesInput, ValuesPack
crates/agentstacks/src/types/op/payload/plan.rs   — create: PlanInput, PlanPack, PlanAgent, PlanNative (id rules, items to cover)
crates/agentstacks/src/types/op/payload/skill.rs  — create: PlanSkill
crates/agentstacks/src/types/op/payload/file.rs   — create: PlanFile
crates/agentstacks/src/types/op/payload/hook.rs   — create: PlanHook, PlanNativeHook, PlanAction
crates/agentstacks/src/types/op/answer/mod.rs     — create: export-only
crates/agentstacks/src/types/op/answer/read.rs    — create: ValuesResult, SkillValues
crates/agentstacks/src/types/op/answer/lookup.rs  — create: TokenValue, ValuesResult::value
```

## Order of work

### Task 1 — Parse and normalise an operation path

**Files:**
- Create `crates/agentstacks/src/types/op/path.rs` — `OpPath`.
- Modify `crates/agentstacks/src/types/op/mod.rs` — `pub mod path; pub use path::OpPath;` and the
  four-question docs. The responsibilities list grows with each later task.

**Change:** `OpPath` is a project-relative path held in its normalised form (§3.3
"Normalisation": `.` and empty segments removed, `./a` = `a`, `a//b` = `a/b`). Equality, ordering
and hashing are by the normalised text in bytes (§18: case-sensitive), so the duplicate check, the
lock lookup and `[[file]].path` all see one form.

One private shape check runs the §3.3 path-shape row (the lock's `[[file]].path` rule plus `/` as
separator) in the cell order of that row. The first failing cell is the refusal (§16 R5):
1. empty, whitespace-only, or normalising to nothing (`.`, `./`) → empty (§16 R25 for an op path,
   §16 R4 for a values path);
2. a leading `/` → must be relative;
3. any `..` segment → must stay inside the project;
4. any `\` → must use / as the separator.

The shape check's result is plan 02's `PathShape` (`src/types/op/shape.rs`): the four cases, each
holding the text as written, with no field word. Two public constructors word it for their two
places. Each refusal shows the text quoted here:
- `OpPath::proposed(text: &str, function: ApiFunction) -> Result<OpPath, OpError>` is for every op
  `path`. It wraps the shape as `OpError::OpPath { function, field: "path", shape }`, which the
  host locates at the call (§3.3, §13 row 1):
  - `path: empty`;
  - `path "/x" must be relative`;
  - `path "../x" must stay inside the project`;
  - `path "a\b" must use / as the separator`.

  It then applies the §3.3 own-file row. A normalised path equal to `agentstacks.lock` or
  `agentstacks.toml` gives `path "agentstacks.lock" is managed by agentstacks` (same for
  `agentstacks.toml`). `function` is plan 02's `ApiFunction` (`write`, `copy`), which starts the
  message.
- `OpPath::new(text: &str) -> Result<OpPath, PathShape>` is for a path whose location key already
  names the field: `values.root` and each `dir` (§16 R4), and the normalised `reads` key plan 06
  builds. It returns the bare `PathShape`, whose Display carries no field word: `empty`,
  `"/x" must be relative`, `"../x" must stay inside the project`, `"a\b" must use / as the
  separator`. Task 6 wraps it as `OpError::ValuesPath { key, shape }` under the key, as in
  `claude/adapter.lua: values.root: empty` and `… values.root: "../x" must stay inside the project`
  (§16 R4). The managed-file rule does not apply here (§16 R4).
- `OpPath::join(&self, child: &str) -> OpPath` appends the `/`-joined segments of an already-valid
  relative child (a pack name, a walked `TreePath`, `runtime.lua`, `translate.lua`), normalising
  the result. It is infallible because both parts satisfy the shape. It is used for `<root>/<pack>`,
  `<root>/<pack>/<rel>`, `<root>/runtime.lua` and `<root>/translate.lua`.
- `OpPath::as_str` gives the normalised text, and `Display` shows the same. There is no
  `FromStr`/`TryFrom`, so each caller names the form it needs.

**Proof:** written first, seen failing, then passing. Each refusal test asserts the plan 02
`OpError` Display.
- `op_path_normalises_dot_and_empty_segments` — `./a`, `a//b`, `a/./b/` give `a`, `a/b`, `a/b`;
  `.claude/x` stays `.claude/x` (§3.3 "Normalisation").
- `op_path_refuses_empty` — `proposed` with `""`, `"  "`, `"."` and `"./"` gives `write: path:
  empty` (§3.3 path row, §16 R25).
- `op_path_refuses_a_leading_slash` — `proposed("/x", write)` gives `write: path "/x" must be
  relative`.
- `op_path_refuses_a_dot_dot_segment` — `"../x"` and `"a/../b"` give `write: path "../x" must stay
  inside the project` / `write: path "a/../b" must stay inside the project`.
- `op_path_refuses_a_backslash` — `"a\b"` gives `write: path "a\b" must use / as the separator`.
- `op_path_first_failing_cell_wins` — `"/a\b"` gives the must-be-relative refusal, and `"../a\b"`
  gives the stay-inside refusal (§16 R5, cell order).
- `op_path_proposed_refuses_agentstacks_own_files` — `agentstacks.lock`, `./agentstacks.lock` and
  `agentstacks.toml` give `write: path "agentstacks.lock" is managed by agentstacks` (and the
  `.toml` form). `new` accepts all three (§3.3 own-file row, §16 R4).
- `op_path_new_returns_the_bare_shape` — `new` with `"."`, `"../x"` and `"a\b"` gives the
  `PathShape` cases displayed `empty`, `"../x" must stay inside the project` and `"a\b" must use /
  as the separator` (§16 R4).
- `op_path_equal_after_normalising` — `OpPath::new("./a//b")` equals `OpPath::new("a/b")`, and
  paths order by bytes (`A.md` < `a.md`, two paths, §18).
- `op_path_join_builds_root_paths` — `.claude/agentstacks` joined with `core`, then with
  `hooks/enforce.lua`, is `.claude/agentstacks/core/hooks/enforce.lua`. Joined with
  `translate.lua`, it is `.claude/agentstacks/translate.lua`.

**Done when:** `cargo test -p agentstacks types::op::path` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): operation path rule`.

### Task 2 — Assign item ids

**Files:**
- Create `crates/agentstacks/src/types/op/id.rs` — `ItemId`.
- Modify `crates/agentstacks/src/types/op/mod.rs` — `pub mod id; pub use id::ItemId;`.

**Change:** `ItemId` is the id string the plan payload hands out (§3.2). It holds its text and the
parts it was built from, so the pack and skill group are read without re-parsing. It is built only
by the core, from typed parts. There is no parse from arbitrary text, because an extension copies
ids and never builds one (§3.2). Constructors give exactly the §3.2 formats:
- `ItemId::skill(&PackName, &ItemName)` → `core/skill/status`;
- `ItemId::skill_file(&PackName, &ItemName, &TreePath)` → `core/skill/status/SKILL.md`,
  `core/skill/status/references/x.md` (the path relative to the skill directory);
- `ItemId::agent(&PackName, &ItemName)` → `core/agent/coder`;
- `ItemId::hook(&PackName, usize)` → `core/hook/0` (index in `[[hooks]]`);
- `ItemId::native_file(&PackName, &ToolName, &TreePath)` → `core/native/claude/x.md` (path
  relative to `native/<tool>/`);
- `ItemId::native_hook(&PackName, usize)` → `core/native-hook/0` (index in
  `native/<tool>/hooks.toml`).

Queries:
- `pack() -> &PackName` (for the owner rule, §3.1);
- `group() -> Option<ItemId>` (the skill id of a skill file; `None` otherwise, §3.2 "skip of it or
  of its skill");
- `is_skill_group()`;
- `is_file()` (skill file or native file: the ids `copy` names, §3.1 `copy` row);
- `as_str`, `Display`.

`Eq`, `Ord` and `Hash` are by the text, so "id order" (§3.2 row 1) is the byte order of the id
text, so `core/hook/10` sorts before `core/hook/2` (§16 R28(h)).

**Proof:** written first, seen failing, then passing.
- `item_id_formats_follow_the_spec` — each constructor's `Display` equals the matching §3.2 id
  example above, all seven of them.
- `item_id_knows_its_pack_and_group` — `core/skill/status/SKILL.md` has pack `core` and group
  `core/skill/status`. A skill group, agent, hook, native file and native hook have no group.
- `item_id_knows_which_ids_are_files` — skill files and native files are files. Skill groups,
  agents, hooks and native hooks are not.
- `item_id_orders_by_text` — sorting `core/skill/status`, `core/agent/coder`, `core/hook/0` gives
  `core/agent/coder`, `core/hook/0`, `core/skill/status`.

**Done when:** `cargo test -p agentstacks types::op::id` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): item ids`.

### Task 3 — Model operations, origins and skips

**Files:**
- Create `crates/agentstacks/src/types/op/origin.rs` — `Origin`.
- Create `crates/agentstacks/src/types/op/skip.rs` — `SkipReason`, `SkipSubject`, `CoreSkip`.
- Create `crates/agentstacks/src/types/op/proposal.rs` — `Operation`, `Proposal`.
- Modify `crates/agentstacks/src/types/op/mod.rs` — the three `pub mod` lines and their `pub use`.

**Change:**
- `Origin` (§3.1 "Proposed by", §3.3 duplicate row):
  - `Extension { tool: ToolName, file: TreePath, line: NonZeroUsize }` is an extension op at
    `file:line`, where `file` is the path inside the extension (`adapter.lua`,
    `lib/settings.lua`, §13 last paragraph);
  - `Core { pack: PackName }` is a core write of that pack's root file, or that pack's
    `requires.binaries`;
  - `Tool { tool: ToolName }` is the core write of `<root>/runtime.lua` for that tool.

  `Origin::location()` gives `Some(Location::at_line("<tool>/<file>", line))` for an extension and
  `None` for core origins (§13 row 1). `Origin::tool()` gives the tool of an `Extension` or `Tool`
  origin. `Origin::site()` is the `file:line` text (`adapter.lua:9`) used inside messages (§3.2
  row 2, §3.3 duplicate row).
- `SkipReason::new(text: &str) -> Result<SkipReason, OpError>` refuses empty or whitespace-only
  text with `reason: empty` (§3.3 `skip.reason` row). The text is kept as written.
  `SkipSubject` is `Item(ItemId) | RootFile { pack: PackName, rel: TreePath }`. A root file is not
  an item (§3.2), but the core can skip it (§4.4 "Root file: a core skip of that file").
  `CoreSkip { subject, reason }` is one skip the core made (Task 7).
- `Operation` (§3.1, D1):
  - `Write { path: OpPath, content: Vec<u8>, covers: Vec<ItemId> }`, where an empty `covers` means
    none (§3.3 "Accepted");
  - `Copy { path: OpPath, file: ItemId }`;
  - `Skip { item: ItemId, reason: SkipReason }`;
  - `RequireBinary { name: BinaryName }`;
  - `Delete { path: OpPath }`, made only by the step rules from the lock (§3.1 last row, plan 06),
    never proposed by an extension.

  There is no merge and no run-command operation (D1, §18). `Operation::path()` gives the path of
  `Write`/`Copy`/`Delete`. `Operation::covered()` lists the ids an op covers: a `Copy`'s `file`, a
  `Write`'s `covers`, a `Skip`'s `item`. `Proposal { op: Operation, origin: Origin }`.
- `require_binary.name` is built with the built `BinaryName::new`. Its `InvalidName` becomes the
  `OpError` value for §3.3 row "`require_binary.name` follows the binary name rule"
  (`name: "Git" is not a valid name`). That conversion is a `From`/constructor in this task, so the
  host does not word it.

**Proof:** written first, seen failing, then passing.
- `skip_reason_refuses_empty_and_blank` — `""`, `"   "` and `"\t\n"` give the `reason: empty`
  refusal. `"not for this tool"` is kept as written.
- `binary_name_refusal_maps_to_the_op_refusal` — `"Git"` gives the `OpError` whose Display is
  `name: "Git" is not a valid name`. `"git"` is accepted.
- `origin_locates_an_extension_op` — `Extension{claude, adapter.lua, 12}` locates as
  `claude/adapter.lua:12`, and its site is `adapter.lua:12`. `Extension{claude, lib/settings.lua,
  3}` locates as `claude/lib/settings.lua:3` (§13).
- `origin_of_the_core_has_no_location` — `Core{core}` and `Tool{claude}` give `None`.
- `operation_lists_what_it_covers` — a copy covers its file, a write its `covers`, a skip its item,
  a `require_binary` nothing. An empty `covers` covers nothing (§3.3 "Accepted").
- `operation_path_of_each_kind` — `Write`, `Copy` and `Delete` have a path. `Skip` and
  `RequireBinary` have none.

**Done when:** `cargo test -p agentstacks types::op` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): operations, origins and skips`.

### Task 4 — Derive the owner of a proposal

**Files:**
- Create `crates/agentstacks/src/types/op/owner.rs` — `owner`.
- Modify `crates/agentstacks/src/types/op/mod.rs` — `pub mod owner; pub use owner::owner;`.

**Change:** `owner(proposal: &Proposal) -> Option<Owner>` derives `by` (§3.1 "`by` … is derived,
never passed"), using the built `Owner::from(&PackName)` / `Owner::from(&ToolName)`:
- a `Copy` is owned by the pack of its `file`;
- a `Write` whose non-empty `covers` all belong to one pack is owned by that pack;
- a `Write` with origin `Core { pack }` (a root file) is owned by that pack;
- any other `Write` is owned by the tool: no `covers`, an empty `covers`, `covers` spanning several
  packs, or the `Tool { tool }` runtime.lua write. The tool is the origin's tool;
- `Skip`, `RequireBinary` and `Delete` have no derived owner. A delete's owner is the lock entry's
  `by`, which plan 06 reads from the lock.

Whether a path is shared, and so has no `by` at all, is plan 06's decision (§3.1 last line, §6.2).
This function is not asked for a shared path.

**Proof:** written first, seen failing, then passing.
- `copy_is_owned_by_the_pack_of_its_file` — a copy of `core/skill/status/SKILL.md` → `core`.
- `write_covering_one_pack_is_owned_by_that_pack` — covers `core/agent/coder` and
  `core/hook/0` → `core`.
- `write_without_covers_is_owned_by_the_tool` — `covers` absent and `covers` empty → `claude`.
- `write_covering_several_packs_is_owned_by_the_tool` — covers `core/agent/coder` and
  `sdlc/agent/planner` → `claude`.
- `core_root_file_is_owned_by_its_pack` — origin `Core{team-lint}` → `team-lint`.
- `runtime_write_is_owned_by_the_tool` — origin `Tool{claude}` → `claude`.
- `skip_require_binary_and_delete_have_no_derived_owner` — each → `None`.

**Done when:** `cargo test -p agentstacks types::op::owner` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): derived owner of a proposal`.

### Task 5 — Model the values and plan payloads

**Files:**
- Create `crates/agentstacks/src/types/op/payload/mod.rs` — export-only, four-question docs.
- Create `crates/agentstacks/src/types/op/payload/values.rs` — `ValuesInput`, `ValuesPack`.
- Create `crates/agentstacks/src/types/op/payload/file.rs` — `PlanFile`.
- Create `crates/agentstacks/src/types/op/payload/skill.rs` — `PlanSkill`.
- Create `crates/agentstacks/src/types/op/payload/hook.rs` — `PlanHook`, `PlanNativeHook`, `PlanAction`.
- Create `crates/agentstacks/src/types/op/payload/plan.rs` — `PlanInput`, `PlanPack`, `PlanAgent`, `PlanNative`.
- Modify `crates/agentstacks/src/types/op/mod.rs` — `pub mod payload;` and `pub use` of the types.

**Change:** These are the domain values behind the §4.4 payloads. The host turns them into JSON
key for key (plan 12). Plan 13 builds them from the rendered packs. `ValuesInput`, `PlanInput` and
every type they hold derive `Clone`, because plan 09's `FakeAdapter` keeps copies in its call log.
The built `SkillMeta` and `AgentMeta` already derive `Clone`.
- **`ValuesInput { tool: ToolName, packs: Vec<ValuesPack>, vars: Vec<Var> }`** (§4.4 `values`
  payload). `ValuesPack { name: PackName, version: PackVersion, skills: Vec<ItemName>, agents:
  Vec<ItemName> }` lists skills and agents in `Pack::skills()` / `Pack::agents()` order.
  `ValuesInput::new(tool, packs: &[&Pack])` builds it as follows:
  - each pack's manifest name and version, skills and agents;
  - `vars`: every var that a `{{var v}}` token in `Pack::tokens()` of these packs names, once
    each, sorted by name (the var's word in byte order, §16 R8: "the vars used by this tool's
    packs, sorted by name");
  - a removed tool gets `new(tool, &[])`, so both lists are empty (§6.4 bullet 1, D8).

  Queries `has_skill(&PackName, &ItemName)`, `has_agent(&PackName, &ItemName)` and `has_var(Var)`
  serve Task 6. The `api` value is the built `types::API`, which the host reads directly.
- **`PlanFile { id: ItemId, rel: TreePath, text: Option<String> }`**. `text` is `Some` for a
  UTF-8 file and `None` for a binary one, which is installed with `copy` (§4.4 first bullet). For
  `SKILL.md` it is the body only.
- **`PlanSkill { id: ItemId, name: ItemName, dir: OpPath, meta: SkillMeta, files: Vec<PlanFile>
  }`**:
  - `meta` is the built `SkillMeta` of the rendered file (formats §5); picking this tool's
    `native` entry and spelling the JSON keys are the host's translation;
  - `dir` is `values.skills[…].dir`, always present for a skill in the payload, because an entry
    needs both `invoke` and `dir` (§16 R12) and a skill with no entry is skipped (§4.4 "Missing
    values").
- **`PlanAgent { id, name: ItemName, meta: AgentMeta, text: String }`**, where `text` is the body.
- **Hooks:**
  - `PlanAction` is `Lua { may_block: bool, command: Vec<String> } | Message { text: String }`.
    `command` is the §11.2 argv built by plan 08, and `may_block` is the hook's declared value
    (formats §6.1). The host derives the payload `kind` (`lua` / `message`) from the variant;
  - `PlanHook { id, event: HookEvent, matches: Option<Vec<MatchValue>>, agents:
    Option<Vec<ItemName>>, action: PlanAction }` (§4.4 hooks, "`match` and `agents` are given as in
    formats §6.1");
  - `PlanNativeHook { id, event: String, matches: Option<Vec<String>>, action: PlanAction }` keeps
    the native event and `match` opaque, as written (§16 R11: native hooks use the hook shape
    `kind`, `event`, `match` as written, `may_block`, `command` or `text`).
- **`PlanNative { files: Vec<PlanFile>, hooks: Vec<PlanNativeHook> }`** holds this tool's
  `native/<tool>/` only (§3.2 native rows). `PlanPack { name, version, skills, agents, hooks,
  native }`. `PlanInput { tool: ToolName, root: OpPath, packs: Vec<PlanPack> }`.
- **`PlanInput` id rules** (§3.3 row "`file`, `item`, `covers[i]` name an id in this run's
  payload", then §16 R13). Each returns the found `ItemId` or one `OpError` (§16 R5):
  - `copy_file(text) -> Result<ItemId, OpError>`:
    - an id not in the payload → `copy: file "core/skill/x/SKILL.md" is not in this plan`;
    - an id that is not a file → `copy: file "core/agent/coder" is not a file` (§16 R13);
  - `skip_item(text) -> Result<ItemId, OpError>`: not in the payload →
    `skip: item "core/skill/x/SKILL.md" is not in this plan`. Any id, skill groups included, may
    be skipped (§3.2);
  - `cover(index: usize, text) -> Result<ItemId, OpError>`, where `index` is the Lua position,
    1-based as the spec examples show:
    - not in the payload → `write: covers[1] "core/skill/x/SKILL.md" is not in this plan`;
    - a skill group → `covers[1]: "core/skill/status" is a skill; list its files` (§16 R13), after
      the `write: ` function word.
- **`PlanInput` queries:**
  - `items()` returns every id that needs a cover, sorted by `ItemId` order (§3.2 table: skill
    files, agents, hooks, native files, native hooks; not skill groups);
  - `lua_hooks()` returns the ids of every hook, neutral or native, whose action is `Lua` (§16 R11,
    Task 9).

  Items the core skipped (§4.4 "Missing values") are never added, so they need no cover (§3.2
  "Items the core already skipped … are left out of the payload").

**Proof:** written first, seen failing, then passing. Packs are loaded in the tests from
`FakeFileTree` (`#[cfg(test)]`, as `set/testing.rs` does), never in a doc example (fakes exist only
under `#[cfg(test)]`). The doctests build `ItemId`s and `PlanInput` values from typed parts.
- `values_input_lists_each_pack_with_its_skills_and_agents` — a pack `core` with skill `status`
  and agent `coder` gives one `ValuesPack` with name `core`, its version, skills `["status"]`,
  agents `["coder"]`.
- `values_input_lists_used_vars_sorted_by_name` — packs whose texts hold `{{var tmpdir}}`,
  `{{var session_id}}` and `{{var tmpdir}}` again give `vars` `["session_id", "tmpdir"]`, the §4.4
  example list (§16 R8).
- `values_input_for_a_removed_tool_is_empty` — `ValuesInput::new(claude, &[])` has empty `packs`
  and `vars`.
- `values_input_answers_membership` — for that input, `has_skill(core, status)`,
  `has_agent(core, coder)` and `has_var(Tmpdir)` are true, and they are false for `core/x`,
  `core/y` and `Home`.
- `plan_input_resolves_every_payload_id` — a `PlanInput` holding the §4.4 example's items (skill
  `core/skill/status` with `SKILL.md` and `logo.png`, agent `core/agent/coder`, hooks
  `core/hook/0` (lua) and `core/hook/1` (message), native file `core/native/claude/x.md`, native
  hook `core/native-hook/0`) gives each id through `skip_item`. `copy_file` resolves both files,
  and `cover` resolves every non-group id.
- `plan_input_refuses_an_id_not_in_the_payload` — `copy_file`, `skip_item` and `cover(1, …)` with
  `core/skill/x/SKILL.md` give `copy: file "core/skill/x/SKILL.md" is not in this plan`, `skip:
  item "core/skill/x/SKILL.md" is not in this plan` and `write: covers[1] "core/skill/x/SKILL.md"
  is not in this plan` (§3.3).
- `plan_input_refuses_copying_a_non_file` — `copy_file("core/agent/coder")` gives `copy: file
  "core/agent/coder" is not a file` (§16 R13).
- `plan_input_refuses_covering_a_skill_group` — `cover(1, "core/skill/status")` gives the refusal
  displayed `write: covers[1]: "core/skill/status" is a skill; list its files` (§16 R13).
- `plan_input_items_exclude_skill_groups_and_are_sorted` — `items()` of that input is the seven
  non-group ids in byte order.
- `plan_input_lists_neutral_and_native_lua_hooks` — with `core/native-hook/0` holding a `Lua`
  action, `lua_hooks()` is `[core/hook/0, core/native-hook/0]`. The message hook `core/hook/1` is
  left out (§16 R11).
- `plan_file_text_is_absent_for_a_binary_file` — `logo.png` built with `text: None` keeps `None`,
  and `SKILL.md` keeps its body text.
- `payloads_clone_equal` — a cloned `ValuesInput` and a cloned `PlanInput` keep every field: the
  same ids, root, vars and pack names.

**Done when:** `cargo test -p agentstacks types::op::payload` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): values and plan payload models`.

### Task 6 — Read the values result

**Files:**
- Create `crates/agentstacks/src/types/op/answer/mod.rs` — export-only, four-question docs.
- Create `crates/agentstacks/src/types/op/answer/read.rs` — `ValuesResult`, `SkillValues`.
- Modify `crates/agentstacks/src/types/op/mod.rs` — `pub mod answer;` and `pub use`.

**Change:** `ValuesResult::read<F>(object: &serde_json::Map<String, serde_json::Value>, input:
&ValuesInput, entry: &Path, word: F) -> Result<ValuesResult, Diagnostics>` takes
`F: Fn(&Value) -> LuaType`. It applies the §4.4 `values` rule table, rows 3–7, to the object the
host received. Rows 1–2 (no handler, a non-table result) are the host's, before this call.
- **Location:** every refusal is a `Diagnostic` at `Location::at_key(entry, <key path>)`. `entry`
  is `<tool>/<entry file>`, e.g. `claude/adapter.lua`, and the key path starts at `values` (§13
  row 2).
- **Collected:** the values result is not a Lua call, so §16 R5 (one refusal per call) does not
  apply. Its refusals are collected, never only the first (§6.1).

The rows:
- **Row 3, `root`:**
  - absent or `null` → `values.root: required`;
  - not a string → `values.root: expected string, got <word>`;
  - a string goes through `OpPath::new`, and its `PathShape` is wrapped as
    `OpError::ValuesPath { key: "values.root", shape }` at `values.root` (§16 R4):
    `claude/adapter.lua: values.root: "/x" must be relative`, `… values.root: empty` (for `""` and
    `.`), `… values.root: "../x" must stay inside the project`, `… values.root: "a\b" must use / as
    the separator`;
  - the accepted root is held normalised (§3.3 "Normalisation").
- **Row 4, `skills`, `agents`, `vars`:** each may be absent. §4.4 marks only `root` required, and
  an absent table has no entries. When present, each must be an object, else `values.<key>:
  expected table, got <word>` (e.g. `values.agents: expected table, got list`). A Lua list arrives
  as a JSON array, and an empty Lua table as an object (§4.4).
- **Row 5, keys:**
  - a `skills` key must be `<pack>/<skill>` for a skill in `input`, else
    `values.skills."core/x": not a skill in this run`;
  - an `agents` key must be `<pack>/<agent>` for an agent in `input`, else
    `values.agents."core/x": not an agent in this run` (§16 R4);
  - a `vars` key must name a var in `input.vars`, else `values.vars.home: not a var in this run`.
- **Row 6, values:**
  - a `skills` entry must be an object (`values.skills."core/status": expected table, got
    <word>`);
  - it needs both `invoke` and `dir` (§16 R12): `values.skills."core/status".dir: required` (same
    shape for `invoke`);
  - both must be strings (`values.skills."core/status".dir: expected string, got number`);
  - `dir` goes through `OpPath::new`, and its `PathShape` is wrapped as `OpError::ValuesPath` at
    `values.skills."core/status".dir` (§16 R4);
  - an agent value and a var value must be strings (`values.agents."core/coder": expected string,
    got <word>`).
- **Row 7, unknown keys:** any key other than `root`, `skills`, `agents`, `vars` at the top, or
  other than `invoke`, `dir` in a skill entry, gives `values.<…>.<key>: unknown key`
  (`values.foo: unknown key`, `values.skills."core/status".icon: unknown key`).
- **Key-path display:** `skills` and `agents` keys appear double-quoted and every other key bare,
  exactly as the §4.4 and §16 R4/R12 messages show them.

The type word of every wrong-type refusal is `word(value)` (`LuaType`, plan 02: object → `table`,
array → `list`, null → `nil`, number, string, boolean; §4.4). The word rule lives in the host
(plan 12, `json_word`) and is passed in, so it is written once.

`ValuesResult` derives `Clone` (plan 09's fake returns scripted copies) and holds:
- `root: OpPath`;
- `skills: BTreeMap<(PackName, ItemName), SkillValues>`, with `SkillValues { invoke: String, dir:
  OpPath }`;
- `agents: BTreeMap<(PackName, ItemName), String>`;
- `vars: BTreeMap<Var, String>`.

Accessors: `root()`, `skill(&PackName, &ItemName) -> Option<&SkillValues>`, `agent(…)`,
`var(Var)`.

**Proof:** written first, seen failing, then passing. Each test passes a word function equivalent
to the §4.4 mapping. Each refusal test asserts the full `Diagnostic` Display text (location plus
plan 02's message).
- `values_example_result_is_accepted` — read the object `probes/examples-results.txt` printed for
  the §4.4 example
  (`{"agents":{"core/coder":"coder"},"root":".claude/agentstacks","skills":{"core/status":{"dir":".claude/skills/status","invoke":"/status"}},"vars":{"tmpdir":"$TMPDIR"}}`)
  against the §4.4 `values` payload (`core` with skill `status`, agent `coder`, vars
  `session_id`, `tmpdir`). It gives:
  - root `.claude/agentstacks`;
  - skill `core/status` → invoke `/status`, dir `.claude/skills/status`;
  - agent `core/coder` → `coder`;
  - var `tmpdir` → `$TMPDIR`.
- `values_root_is_required` — `{}` gives exactly `claude/adapter.lua: values.root: required`
  (row 3).
- `values_root_must_be_a_valid_path` — gives exactly these (row 3, §16 R4):
  - root `/x` → `claude/adapter.lua: values.root: "/x" must be relative`;
  - root `.` → `claude/adapter.lua: values.root: empty`;
  - root `../x` → `claude/adapter.lua: values.root: "../x" must stay inside the project`;
  - root `a\b` → `claude/adapter.lua: values.root: "a\b" must use / as the separator`.
- `values_root_may_be_an_own_file_name` — root `agentstacks.lock` is accepted, because the
  managed-file rule does not apply (§16 R4).
- `values_root_must_be_a_string` — `{"root":5}` gives `claude/adapter.lua: values.root: expected
  string, got number`.
- `values_paths_are_normalised` — root `./.claude//agentstacks` and dir `.claude/skills/./status`
  are held as `.claude/agentstacks` and `.claude/skills/status` (§3.3 "Normalisation").
- `values_tables_refuse_a_list` — `{"root":"r","agents":["x"]}` gives exactly
  `claude/adapter.lua: values.agents: expected table, got list` (row 4).
- `values_tables_may_be_absent` — `{"root":"r"}` is accepted, with no skills, agents or vars.
- `values_refuses_a_skill_not_in_this_run` — key `core/x` gives exactly `claude/adapter.lua:
  values.skills."core/x": not a skill in this run` (row 5).
- `values_refuses_an_agent_not_in_this_run` — key `core/x` under `agents` gives exactly
  `claude/adapter.lua: values.agents."core/x": not an agent in this run` (row 5, §16 R4).
- `values_refuses_a_var_not_in_this_run` — key `home` gives exactly `claude/adapter.lua:
  values.vars.home: not a var in this run` (row 5).
- `values_skill_entry_needs_invoke_and_dir` — `{"invoke":"/status"}` gives exactly
  `claude/adapter.lua: values.skills."core/status".dir: required`, and `{"dir":"d"}` gives the same
  shape for `invoke` (§16 R12).
- `values_skill_fields_must_be_strings` — `dir = 5` gives exactly `claude/adapter.lua:
  values.skills."core/status".dir: expected string, got number` (row 6).
- `values_skill_dir_must_be_a_valid_path` — `dir = "../x"` gives exactly `claude/adapter.lua:
  values.skills."core/status".dir: "../x" must stay inside the project` (§16 R4).
- `values_agent_and_var_values_must_be_strings` — agent value `true` and var value `{}` give
  `expected string, got boolean` and `expected string, got table` at their keys.
- `values_refuses_unknown_keys_at_any_level` — `foo` at the top and `icon` in a skill entry give
  exactly `claude/adapter.lua: values.foo: unknown key` and `claude/adapter.lua:
  values.skills."core/status".icon: unknown key` (row 7).
- `values_collects_every_refusal` — an object with a bad root, an unknown key and a stray var gives
  all three diagnostics.

**Done when:** `cargo test -p agentstacks types::op::answer` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): values result rules`.

### Task 7 — Look up token values and skip what has none

**Files:**
- Create `crates/agentstacks/src/types/op/answer/lookup.rs` — `TokenValue`, `ValuesResult::value`.
- Create `crates/agentstacks/src/types/op/missing.rs` — the missing-value skip rule.
- Modify `crates/agentstacks/src/types/op/answer/mod.rs`, `crates/agentstacks/src/types/op/mod.rs`.

**Change:**
- **The token table (§4.4).** `ValuesResult::value(&self, target: &TokenTarget, pack: &PackName)
  -> TokenValue`. `pack` is the pack the token sits in; it resolves an `ItemRef` with no pack, and
  `Root(None)`. `TokenValue` is `Given(String) | Missing | FromOption`:
  - `Root(None)` gives `<root>/<pack>`, and `Root(Some(p))` gives `<root>/<p>` (`OpPath::join`);
  - `Skill(r)` gives `skills[r].invoke`, and `SkillDir(r)` gives `skills[r].dir`;
  - `Agent(r)` gives `agents[r]`;
  - `Var(v)` gives `vars[v]`;
  - `Option(_)` gives `FromOption`. The core resolves options (formats §4.4, `ValidSet::option`);
    plan 13 does that.

  An absent entry is `Missing`. A skill entry always holds both fields (§16 R12), so `SkillDir` is
  `Missing` only when the whole entry is. Filling `TokenValues` and calling `render` is plan 13's
  (§4.4 "Every token is filled through `TokenValues::insert` and `render`").
- **The missing-value rule** (§4.4 "Missing values", tie-breaks §16 R8). The reason texts are the
  spec's, verbatim.
  - `first_missing<'a>(tokens: impl IntoIterator<Item = &'a Token>, values: &ValuesResult, pack:
    &PackName) -> Option<&'a Token>` returns the first token in the order given whose value is
    `Missing`. Plan 13 passes an item's tokens in text order, so the reason names "the first
    missing token in text order" (§16 R8).
  - `skip_for_missing_value(tool: &ToolName, item: &ItemId, token: &Token) -> CoreSkip`:
    - a skill file id becomes its skill group, and a skill group stays itself ("the item is the
      whole skill");
    - an agent, hook, native file or native hook is skipped as itself;
    - the reason is `<tool> gave no value for <token>`, with the token in the built `Token`
      `Display`, e.g. `claude gave no value for {{skill core/status}}`.
  - `skip_root_file_for_missing_value(tool, pack: &PackName, rel: &TreePath, token) -> CoreSkip`
    gives the subject `RootFile { pack, rel }` with the same reason ("Root file: a core skip of
    that file").
  - `skip_skill_without_location(tool, pack, skill: &ItemName) -> CoreSkip` gives the subject
    `ItemId::skill(pack, skill)` and the reason `<tool> gave no install location for skill
    <pack>/<skill>`, e.g. `claude gave no install location for skill core/status`. A missing
    skill entry comes before any token (§16 R8), so plan 13 asks `ValuesResult::skill` first and
    uses this skip when it is `None`.

**Proof:** written first, seen failing, then passing.
- `root_token_is_root_slash_pack` — with root `.claude/agentstacks`, `{{root}}` in pack `core` and
  `{{root core}}` both give `.claude/agentstacks/core`.
- `skill_and_skill_dir_tokens_read_invoke_and_dir` — with the §4.4 example result:
  - `{{skill core/status}}` gives `/status`;
  - `{{skill_dir core/status}}` gives `.claude/skills/status`;
  - `{{skill status}}` inside pack `core` gives `/status`.
- `agent_and_var_tokens_read_their_maps` — `{{agent core/coder}}` gives `coder`, and `{{var
  tmpdir}}` gives `$TMPDIR`.
- `option_token_is_left_to_the_core` — `{{option archive_keep}}` gives `FromOption`.
- `absent_values_are_missing` — `{{skill core/other}}`, `{{skill_dir core/other}}`, `{{agent
  core/x}}` and `{{var home}}` each give `Missing`.
- `first_missing_is_the_first_in_text_order` — tokens `{{var tmpdir}}`, `{{agent core/x}}`,
  `{{var home}}` give `{{agent core/x}}` (§16 R8).
- `missing_value_skips_the_whole_skill` — a missing token in `core/skill/status/SKILL.md` gives
  subject `core/skill/status` and reason exactly `claude gave no value for {{skill core/status}}`.
- `missing_value_skips_other_items_as_themselves` — `core/agent/coder`, `core/hook/0`,
  `core/native/claude/x.md` and `core/native-hook/0` each give themselves as subject.
- `missing_value_in_a_root_file_is_a_core_skip_of_that_file` — pack `core` and rel `hooks/x.lua`
  give `RootFile{core, hooks/x.lua}` with the same reason shape.
- `skill_without_entry_has_no_install_location` — gives subject `core/skill/status` and reason
  exactly `claude gave no install location for skill core/status`.

**Done when:** `cargo test -p agentstacks types::op` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): token values and missing-value skips`.

### Task 8 — Collect proposals in call order

**Files:**
- Create `crates/agentstacks/src/types/op/proposals.rs` — `Proposals`.
- Modify `crates/agentstacks/src/types/op/mod.rs` — `pub mod proposals; pub use proposals::Proposals;`.

**Change:** `Proposals` is the run's record of accepted proposals, in the order received.
`Proposals::new()` gives an empty record, and `Default` is the same. Plans 09, 12 and 13 start a run
with it.
- **Order** (§16 R6): listed tools in name order, then removed tools in name order. Each listed
  tool's core writes are recorded just before its `plan` dispatch. The flow (plan 13) drives that
  order, and the host adds that tool's ops during the dispatch (plan 09 `Adapter::plan` takes
  `&mut Proposals`). So a check made when an op arrives sees every earlier proposal.
- **Removed tools:** a removed tool's dropped (non-shared) proposals never reach the duplicate
  check (§16 R18). The flow keeps them out of this record (plan 13 with plan 06's filter).
  `Proposals` stays one record and applies its rules to whatever it is given.

One Lua call raises one refusal: the first failing check, in table order (§16 R5). So each method
returns at most one refusal, and a refused proposal is not recorded (§3.3 "the refused op is not
recorded").

- **`check_path(&self, path: &OpPath, origin: &Origin) -> Result<(), OpError>`** is the §3.3 row
  "`path` not proposed twice in one plan". It is public, so the host runs it at its place in the
  §3.3 table: after the shape and own-file rows (Task 1), before the id rows (Task 5). The new
  extension op is worded by the earlier proposer:
  - the same tool's extension op → `path ".claude/x" is already proposed at adapter.lua:9`;
  - another tool's extension op → `… already proposed by opencode`;
  - a core root-file write → `… already proposed by core (root file of core)`;
  - the core's runtime.lua write → `path "<p>" is already proposed by core (runtime.lua of
    claude)` (§16 R6).

  The function word comes first in each refusal (plan 02's `ApiFunction`).
- **`propose(&mut self, proposal: Proposal) -> Result<(), OpError>`** records one extension op.
  It runs the checks in this order and returns the first that fails:
  1. `check_path` for a `Write` / `Copy` (§3.3 table row, which comes before §3.2's conflict row);
  2. for a `Write` / `Copy`, its covered ids in `covers` order: the first id that this tool already
     skipped, or whose skill group it skipped, gives `covers "core/agent/coder", which
     adapter.lua:9 skipped` (§3.2 row 2, the same shape for `write` and `copy`);
  3. for a `Skip`: an id this tool already covered by a `Write`/`Copy`, or a skill group with any of
     its files covered, gives `skip: item "core/skill/status" is already covered at adapter.lua:9`
     (§3.2 row 2). The site named is the first covering op recorded.
- **`record_core(&mut self, tool: &ToolName, proposal: Proposal, project_file: &Path) -> Result<(),
  Diagnostic>`** records a core write (Task 10) made for `tool`. The tool is stored beside the
  proposal, so a later clash can name it. A path that an earlier proposal already holds is refused
  at `Location::set_level(project_file, Some("tools.<tool>"))` with `path "<p>" is already proposed
  by <earlier tool>`, e.g. `agentstacks.toml: tools.opencode: path "<p>" is already proposed by
  claude` (§16 R6). The earlier tool is the earlier proposal's tool, whether it came from an
  extension op or from a core write made for that tool.
- **Accepted** (§3.2 rows 3–4, §3.3 "Accepted without refusal"):
  - the same item skipped twice (the first reason is kept for the plan);
  - the same file copied to two paths;
  - two writes covering the same item;
  - a repeated id in one `covers`;
  - the same `require_binary` name more than once.
- **`Delete`** never reaches `propose`. Deletes are step decisions from the lock (plan 06), and
  `propose` documents that.

Queries:
- `iter()` gives all proposals in order;
- `for_tool(&ToolName)` gives that tool's extension proposals;
- `skip_reason(&ToolName, &ItemId) -> Option<&SkipReason>` gives the first reason (§3.2 row 3);
- `binaries(&ToolName)` gives each `require_binary` name once per tool, with its first origin
  (§3.3 "checked once");
- `wrote(&ToolName, &OpPath)` tells whether that tool proposed a `Write` to the path (Task 9);
- `has_runtime(&ToolName)` tells whether the core's runtime.lua write for that tool was recorded
  (Task 9, §16 R11).

The per-tool state (what each tool covered and skipped, with the first site) lives in a private
part of the same file. It is one concept: the call-order record.

**Proof:** written first, seen failing, then passing. Each refusal test asserts the plan 02
`OpError` Display, or the full `Diagnostic` for `record_core`.
- `same_tool_duplicate_names_the_earlier_site` — claude writes `.claude/x` at `adapter.lua:9`,
  then copies to `./.claude//x` at line 12. The copy is refused with `copy: path ".claude/x" is
  already proposed at adapter.lua:9`, and it is not recorded.
- `other_tool_duplicate_names_the_tool` — claude writes `.claude/x`, then opencode writes it:
  refused with `write: path ".claude/x" is already proposed by claude`. This is the spec example
  `… already proposed by opencode` with the tools in that order.
- `extension_op_on_another_tools_root_file_names_the_pack` — `record_core` for claude of a
  `Core{core}` root-file write to `.claude/agentstacks/core/x.md`, then an opencode write of that
  path: refused with `write: path ".claude/agentstacks/core/x.md" is already proposed by core (root
  file of core)`, not `by claude`.
- `proposals_new_is_empty_and_default` — `Proposals::new()` and `Proposals::default()` hold no
  proposal, and `iter()` yields nothing.
- `core_root_file_duplicate_names_the_pack` — a `Core{core}` write of
  `.claude/agentstacks/core/hooks/enforce.lua`, then a claude write of it: refused with
  `write: path ".claude/agentstacks/core/hooks/enforce.lua" is already proposed by core (root file
  of core)`.
- `runtime_duplicate_names_the_tool_runtime` — claude's runtime.lua write at
  `.claude/agentstacks/runtime.lua`, then a claude write of it: refused with `write: path
  ".claude/agentstacks/runtime.lua" is already proposed by core (runtime.lua of claude)` (§16 R6).
- `core_write_meeting_an_earlier_tool_is_refused_at_the_tool_entry` — claude writes
  `.shared/runtime.lua`, then `record_core` for opencode of a runtime.lua write to the same path.
  It gives exactly `agentstacks.toml: tools.opencode: path ".shared/runtime.lua" is already
  proposed by claude`. The same holds when the earlier proposal is claude's own core write
  (§16 R6).
- `skip_after_copy_is_refused_at_the_skip` — copy of `core/skill/status/SKILL.md` at line 9, then
  skip of `core/skill/status` at line 14: refused with `skip: item "core/skill/status" is already
  covered at adapter.lua:9`.
- `write_after_skip_is_refused_at_the_write` — skip `core/agent/coder` at line 9, then a write
  covering it: refused with `write: covers "core/agent/coder", which adapter.lua:9 skipped`.
- `copy_after_group_skip_is_refused` — skip `core/skill/status`, then copy
  `core/skill/status/SKILL.md`: refused with `copy: covers "core/skill/status/SKILL.md", which
  adapter.lua:9 skipped`.
- `duplicate_path_wins_over_a_conflict` — a write that duplicates a path and covers a skipped id
  returns only the duplicate-path refusal (§16 R5).
- `skipping_twice_keeps_the_first_reason` — two skips of `core/agent/coder`, with reasons `a` then
  `b`: both accepted, and `skip_reason` is `a`.
- `copying_one_file_to_two_paths_is_accepted` — two copies of one file id to two paths are both
  recorded.
- `two_writes_covering_one_item_are_accepted` — both are recorded, and a `covers` of `[x, x]` is
  accepted.
- `require_binary_twice_is_checked_once` — two `node` requirements give one entry in
  `binaries(claude)`, with the first origin.
- `conflicts_are_per_tool` — claude skips `core/agent/coder`, then opencode writes covering
  `core/agent/coder`: accepted, since each tool has its own payload.

**Done when:** `cargo test -p agentstacks types::op::proposals` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): proposal record with duplicate and conflict rules`.

### Task 9 — Check coverage and translate.lua at the end of a tool's plan

**Files:**
- Create `crates/agentstacks/src/types/op/coverage.rs` — `check_coverage`.
- Create `crates/agentstacks/src/types/op/translate.rs` — `check_translate`.
- Modify `crates/agentstacks/src/types/op/mod.rs`.

**Change:**
- **`check_coverage(tool: &ToolName, entry: &Path, payload: &PlanInput, proposals: &Proposals) ->
  Diagnostics`** (§3.2, D6). An item from `payload.items()` is covered by any one of:
  - a `Copy` of it;
  - a `Write` listing it in `covers`;
  - a `Skip` of it;
  - for a skill file, a `Skip` of its skill group.

  Only this tool's proposals count. Each uncovered item gives one refusal, in id order, at
  `Location::file(entry)` with no line (§13 row 3), e.g. `claude/adapter.lua:
  core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a
  reason)`. The end-of-plan check is not one Lua call, so every uncovered item is reported (§3.2
  row 1 "one per item").
- **`check_translate(tool, entry, payload, proposals) -> Option<Diagnostic>`** (§3.2 last row as
  §16 R11 states it). It applies only when `proposals.has_runtime(tool)` (runtime.lua is written)
  and at least one id in `payload.lua_hooks()` (neutral or native) is covered by one of this tool's
  `Write` ops. It then refuses unless this tool proposed a `Write` to
  `payload.root.join("translate.lua")`. The refusal is at `Location::file(entry)`, with `<root>`
  filled in: `claude/adapter.lua: lua hooks need .claude/agentstacks/translate.lua (write it)`.

**Proof:** written first, seen failing, then passing.
- `every_item_covered_passes` — a payload whose items are each copied, written or skipped gives no
  diagnostic.
- `uncovered_items_are_refused_one_each_in_id_order` — leaving `core/skill/status/references/x.md`
  and `core/agent/coder` uncovered gives exactly two lines:
  - first `claude/adapter.lua: core/agent/coder got no operation (copy it, write it, or skip it
    with a reason)`;
  - then `claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write
    it, or skip it with a reason)`.

  This covers §3.2 row 1 and the §13 row 3 example.
- `skill_group_skip_covers_its_files` — skipping `core/skill/status` covers `SKILL.md` and
  `references/x.md`.
- `skipped_by_the_core_needs_no_cover` — a payload built without a core-skipped item gives no
  refusal for that item.
- `another_tools_ops_do_not_cover` — opencode's copy of an id does not cover claude's item.
- `lua_hooks_written_without_translate_are_refused` — with runtime.lua recorded for claude, a
  write covering `core/hook/0` (lua) and no write to `.claude/agentstacks/translate.lua` gives
  exactly `claude/adapter.lua: lua hooks need .claude/agentstacks/translate.lua (write it)`.
- `native_lua_hook_written_without_translate_is_refused` — the same with only
  `core/native-hook/0` (lua) covered by a write (§16 R11).
- `lua_hooks_with_translate_pass` — adding a write to `./.claude/agentstacks/translate.lua`
  (normalised) passes.
- `skipped_lua_hooks_need_no_translate` — skipping `core/hook/0` passes without translate.lua.
- `message_hooks_need_no_translate` — a write covering only `core/hook/1` (message) passes.
- `no_runtime_no_translate_rule` — the same lua-hook write with no runtime.lua recorded passes
  (§16 R11, "applies when `runtime.lua` is written").

**Done when:** `cargo test -p agentstacks types::op` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): coverage and translate.lua rules`.

### Task 10 — Build the core writes under the root

**Files:**
- Create `crates/agentstacks/src/types/op/root.rs` — `root_file_write`, `runtime_write`, `runtime_needed`.
- Modify `crates/agentstacks/src/types/op/mod.rs`.

**Change:** These are the §3.1 `write` (core) row and D7, as pure builders. Plan 13 supplies the
bytes and records the writes with `Proposals::record_core` just before the tool's `plan` dispatch
(§16 R6):
- `root_file_write(root: &OpPath, pack: &PackName, rel: &TreePath, bytes: Vec<u8>) -> Proposal`
  makes a `Write` to `<root>/<pack>/<rel>`, with no `covers` and origin `Core { pack }`. So `owner`
  gives the pack (§3.1 `by` second bullet), and an extension clash names `core (root file of
  <pack>)`.
- `runtime_write(root: &OpPath, tool: &ToolName, bytes: Vec<u8>) -> Proposal` makes a `Write` to
  `<root>/runtime.lua` with origin `Tool { tool }`. It is owned by the tool (§3.1 `by` third
  bullet), and an extension clash names `core (runtime.lua of <tool>)` (§16 R6). The bytes are plan
  08's `RUNTIME_LUA`, passed in so `types/op` does not depend on `types/protocol`.
- `runtime_needed(packs: &[&Pack], tool: &ToolName) -> bool` is true when some pack has a lua hook:
  a `[[hooks]]` entry (`PackManifest::hooks`) whose `HookAction` is `Lua`, or a hook in that
  pack's `native/<tool>/hooks.toml` (`NativeDir::hooks`) whose `HookAction` is `Lua`. This holds
  "whether or not the extension later skips it" (§16 R11), so it reads the packs, not the proposals
  or the payload. A message hook, neutral or native, does not count.

**Proof:** written first, seen failing, then passing. Packs come from `FakeFileTree` under
`#[cfg(test)]`.
- `root_file_goes_under_root_and_pack` — root `.claude/agentstacks`, pack `core`, rel
  `hooks/enforce.lua` gives a write to `.claude/agentstacks/core/hooks/enforce.lua`. Its origin is
  `Core{core}`, and `owner` is `core`.
- `runtime_goes_to_root_runtime_lua` — gives a write to `.claude/agentstacks/runtime.lua` with the
  given bytes. Its origin is `Tool{claude}`, and `owner` is `claude`.
- `runtime_needed_for_a_neutral_lua_hook` — a pack with a `lua` `[[hooks]]` entry → true.
- `runtime_needed_for_a_native_lua_hook_of_this_tool` — a pack whose `native/claude/hooks.toml`
  holds a lua hook → true for `claude`, and false for `opencode` when that is its only hook.
- `runtime_not_needed_for_message_hooks_only` — packs with only message hooks (neutral and native),
  or with no hooks → false (§16 R11 "any lua hook").

**Done when:** `cargo test -p agentstacks types::op::root` passes and `cargo make dod-crate agentstacks` is green. Commit `feat(agentstacks): core writes under the extension root`.

### Task 11 — Collect binary requirements and read the airsl version

**Files:**
- Create `crates/agentstacks/src/types/op/binaries.rs` — `BinaryRequirement`, `collect_binaries`.
- Create `crates/agentstacks/src/types/op/airsl.rs` — `parse_airsl_version`, `airsl_not_found`, `airsl_unparseable`.
- Modify `crates/agentstacks/src/types/op/mod.rs` — final docs: the full responsibilities list and
  the entry points (`OpPath::proposed`, `Proposals::propose`, `check_coverage`).

**Change:** These are the "what to check" half of §10. Plan 13 asks `Binary` and calls the
refusal builders here (§6.1 step 7).
- **`BinaryRequirement`** is `Pack { pack: PackName, name: BinaryName } | Extension { origin:
  Origin, name: BinaryName }`.
- **`collect_binaries(packs: &[&Pack], proposals: &Proposals, tools: &[ToolName]) ->
  Vec<BinaryRequirement>`** yields:
  - first each pack's `requires.binaries` (built `Requires::binaries`), packs in name order and
    names in written order (§3.1 row 6, §10 row 1);
  - then, for each tool in name order, `proposals.binaries(tool)`, each name once per tool with its
    first origin (§3.3 "checked once").

  A name required by a pack and by an extension appears once for each (§10 "reported once for
  each").
- **`BinaryRequirement::not_found(&self, project_file: &Path) -> Diagnostic`** gives the §10
  refusals:
  - a pack → at `Location::set_level(project_file, Some("packs.<pack>"))`:
    `agentstacks.toml: packs.core: binary "rg" not found on PATH`;
  - an extension → at the origin's location: `claude/adapter.lua:7: require_binary: "node" not
    found on PATH`.
- **`parse_airsl_version(text: &str) -> Option<PackVersion>`** (§10 row 3, §16 R2) is the one
  copy of this rule. Plan 13 (plan) and plan 06 (doctor's airsl answer) both import it. It first
  trims surrounding whitespace, then accepts exactly `airsl <semver>`: the prefix `airsl `, then a
  version the built `PackVersion::new` accepts. `airsl 0.1.4` is the probed output
  (`probes/hookreq-results.md`). Lossy UTF-8 decoding of the binary's bytes, and treating a
  non-zero exit as unparseable, belong to the `Binary` adapter and the flow (§16 R2, plans 11 and
  13).
- **`airsl_not_found(project_file, range: &AirslRange) -> Diagnostic`** is at
  `Location::set_level(project_file, None)`: `agentstacks.toml: airsl not found on PATH (needed by
  core, sdlc)`. The packs come from the built `AirslRange::needs()`, in pack name order.
- **`airsl_unparseable(project_file, text) -> Diagnostic`** gives `agentstacks.toml: airsl
  --version printed "<text>", expected "airsl <version>"`, with `<text>` trimmed (§16 R2).
- The out-of-range row is the built `AirslRange::installed_refusal` (`agentstacks.toml: airsl
  0.1.3 installed, packs need >=0.1.4`), reused, not re-worded.
- Whether airsl is required at all is `ValidSet::airsl()` being `Some` ("when any locked pack
  declares `requires.airsl`", §10 row 3, §3.1 row 6). The flow asks it directly; no wrapper is
  added.

**Proof:** written first, seen failing, then passing.
- `pack_binaries_come_first_in_pack_order` — packs `sdlc` (needs `rg`) and `core` (needs `git`,
  `rg`) give `core/git`, `core/rg`, `sdlc/rg`, then the extension requirements.
- `extension_binaries_once_per_tool` — claude requiring `node` twice and opencode once gives one
  claude `node` with its first origin and one opencode `node`.
- `pack_and_extension_requirement_both_listed` — `rg` from pack `core` and from claude gives two
  requirements (§10 "once for each").
- `pack_binary_not_found_text` — gives exactly `agentstacks.toml: packs.core: binary "rg" not found
  on PATH` (§10 row 1).
- `extension_binary_not_found_text` — origin `Extension{claude, adapter.lua, 7}` gives exactly
  `claude/adapter.lua:7: require_binary: "node" not found on PATH` (§10 row 2).
- `airsl_version_parses_the_probed_output` — `airsl 0.1.4` and `"airsl 0.1.4\n"` give `0.1.4`
  (§16 R2).
- `airsl_version_refuses_other_text` — `airsl-dev`, `0.1.4`, `airsl` and `airsl x.y` give `None`.
- `airsl_not_found_names_the_needing_packs` — a range needed by `core` and `sdlc` gives exactly
  `agentstacks.toml: airsl not found on PATH (needed by core, sdlc)` (§10 row 3).
- `airsl_unparseable_quotes_the_trimmed_text` — `"  airsl-dev\n"` gives exactly `agentstacks.toml:
  airsl --version printed "airsl-dev", expected "airsl <version>"` (§10 row 3, §16 R2).
- `airsl_out_of_range_reuses_the_built_refusal` — `0.1.3` against `>=0.1.4` gives exactly
  `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4` through
  `AirslRange::installed_refusal`.

**Done when:** `cargo test -p agentstacks types::op` passes and `cargo make dod` is green. Commit `feat(agentstacks): binary requirements and the airsl version rule`.

---

## Risks

- **Riskiest task: Task 8 (`Proposals`).** The duplicate wording depends on call order across core
  writes and tools (§16 R6). The tests fix the wording inside `Proposals`, and plan 13 fixes the
  order in the flow. The host must call `check_path` at its place in the §3.3 table, so that
  "first failing check in table order" (§16 R5) holds across the host's and this module's checks.
- **Cross-plan seams:**
  - `OpPath::new` returns plan 02's `PathShape`. `OpPath::proposed` wraps it as
    `OpError::OpPath { function, field, shape }`, and the values reader wraps it as
    `OpError::ValuesPath { key, shape }` (§16 R4). Every other text quoted here (§16 R6, R12, R13)
    is a plan 02 `OpError` variant.
  - Plan 04 alone owns `parse_airsl_version` (`types/op/airsl.rs`). Plan 06 imports it for doctor
    and keeps no copy (modularity Rule 2).
  - The host's `json_word` is passed into `ValuesResult::read`, so the word rule is not written
    twice.
  - `PlanSkill::meta` / `PlanAgent::meta` are the built `SkillMeta` / `AgentMeta` of the rendered
    file. Plan 13 produces them, for example by parsing the rendered frontmatter with
    `SkillMeta::parse`.
  - Plan 06's `Plan` holds `CoreSkip` values beside the extension `Skip` proposals, because a
    root-file skip has no `ItemId`.
  - Plan 09 is the source of truth for port signatures. `Adapter::values` returns
    `Result<ValuesResult, Diagnostics>`, and `Adapter::plan` takes `&mut Proposals`. This plan
    names the insertion `Proposals::propose`. Plan 09's fake calls "`Proposals`' own insertion",
    which is this method.
- **Settled readings (§16 R28):**
  - "id order" (§3.2 row 1) is the byte order of the id text (R28(h));
  - "already covered at" names the first covering op recorded (R28(h));
  - `require_binary` is checked once per tool (R28(h));
  - `skills`, `agents` and `vars` may be absent from the values result (R28(h));
  - a native `match` is the list the built `NativeHook` reader holds, so `PlanNativeHook` holds
    the list (R28(d)).
- **Not done here:**
  - argument reading, UTF-8, the phase rule, `read`, the nearest-frame location (§16 R5) and the
    §16 R3 shapes, which are plan 12's;
  - rendering, and the order the flow feeds `Proposals` in, which are plan 13's;
  - the removed-tool filter, which is plan 06's.

## Proof

- `cargo make dod` green at the end of Task 11.
- Spec rows covered:
  - **§3.1 Kinds table:**
    - `write`, `copy`, `skip`, `require_binary`, `delete` → Task 3
      (`operation_lists_what_it_covers`, `operation_path_of_each_kind`);
    - core `write` → Task 10;
    - core `require_binary` → Task 11 (pack binaries; the airsl decision is `ValidSet::airsl`).
  - **§3.1 `by` bullets 1–3 and "never passed"** → Task 4, all seven tests. Shared has no `by` →
    plan 06.
  - **§3.2 id table**, all seven formats → Task 2 (`item_id_formats_follow_the_spec`). Root files
    are not items → Tasks 5 and 10. Core-skipped items need no cover → Task 9
    (`skipped_by_the_core_needs_no_cover`).
  - **§3.2 case table:**
    - row 1 → Task 9 (`uncovered_items_are_refused_one_each_in_id_order`);
    - row 2 → Task 8 (`skip_after_copy_is_refused_at_the_skip`,
      `write_after_skip_is_refused_at_the_write`, `copy_after_group_skip_is_refused`);
    - row 3 → Task 8 (`skipping_twice_keeps_the_first_reason`);
    - row 4 → Task 8 (`copying_one_file_to_two_paths_is_accepted`,
      `two_writes_covering_one_item_are_accepted`);
    - row 5 → Task 9 (`lua_hooks_written_without_translate_are_refused`,
      `native_lua_hook_written_without_translate_is_refused` and the passing cases).
  - **§3.3 rule table:**
    - rows 1–6 (argument, required, type, list, unknown key, UTF-8) → plan 12;
    - path shape → Task 1 (the `op_path_refuses_*` tests, `op_path_first_failing_cell_wins`);
    - own files → Task 1 (`op_path_proposed_refuses_agentstacks_own_files`);
    - not proposed twice → Task 8 (the duplicate tests);
    - ids in payload → Task 5 (`plan_input_resolves_every_payload_id`,
      `plan_input_refuses_an_id_not_in_the_payload`);
    - `reason` empty → Task 3 (`skip_reason_refuses_empty_and_blank`);
    - binary name → Task 3 (`binary_name_refusal_maps_to_the_op_refusal`);
    - `read` in reads and the phase rows → plan 12.
  - **§3.3 Normalisation** → Task 1 (`op_path_normalises_dot_and_empty_segments`,
    `op_path_equal_after_normalising`) and Task 6 (`values_paths_are_normalised`). Normalising
    `reads` entries → plans 03 and 12.
  - **§3.3 Accepted:**
    - empty content → plan 12;
    - empty `covers` → Task 3 (`operation_lists_what_it_covers`);
    - `require_binary` repeated → Task 8 (`require_binary_twice_is_checked_once`).
  - **§3.3 Type words** → plan 02 (`LuaType`), used through the word function in Task 6.
    Strictness and refusals sticking → plan 12.
  - **§4.4:**
    - `values` payload → Task 5 (`values_input_*`);
    - token table → Task 7 (the lookup tests);
    - Missing values (skill group, no install location, other items, root file) → Task 7;
    - left out of the payload → Task 5 / plan 13;
    - type words → Task 6, via the host's word function; empty Lua table as an object → Task 6
      (`values_root_is_required` on `{}`);
    - `values` table rows 1–2 → plan 12; row 3 → Task 6 (`values_root_is_required`,
      `values_root_must_be_a_valid_path`, `values_root_must_be_a_string`); row 4 → Task 6
      (`values_tables_refuse_a_list`, `values_tables_may_be_absent`); row 5 → Task 6 (skill, agent
      and var tests); row 6 → Task 6 (`values_skill_entry_needs_invoke_and_dir`,
      `values_skill_fields_must_be_strings`, `values_skill_dir_must_be_a_valid_path`,
      `values_agent_and_var_values_must_be_strings`); row 7 → Task 6
      (`values_refuses_unknown_keys_at_any_level`);
    - `plan` payload: the model, `text` absent for binary, ids to copy back, `command` carried →
      Task 5. JSON shape and no plan handler → plan 12;
    - "Tools run in name order" → the order `Proposals` is fed in (plan 13), and its wording
      effect → Task 8 (`other_tool_duplicate_names_the_tool`).
  - **§10:**
    - row 1 → Task 11 (`pack_binary_not_found_text`);
    - row 2 → Task 11 (`extension_binary_not_found_text`);
    - row 3 → Task 11 (`airsl_not_found_names_the_needing_packs`,
      `airsl_unparseable_quotes_the_trimmed_text`, `airsl_out_of_range_reuses_the_built_refusal`,
      `airsl_version_parses_the_probed_output`);
    - "once for each" → Task 11 (`pack_and_extension_requirement_both_listed`);
    - the lookups run in plan 13; the `[airsl] version` pin is plan 06's.
  - **§16 rows:**
    - R2 → Task 11;
    - R4 → Tasks 1 and 6;
    - R5 → Tasks 1, 5 and 8 (one refusal per call, in table order);
    - R6 → Task 8 (`runtime_duplicate_names_the_tool_runtime`,
      `core_write_meeting_an_earlier_tool_is_refused_at_the_tool_entry`) and Task 10;
    - R8 → Task 5 (`values_input_lists_used_vars_sorted_by_name`) and Task 7
      (`first_missing_is_the_first_in_text_order`, `skill_without_entry_has_no_install_location`);
    - R11 → Task 5 (native hook shape, `plan_input_lists_neutral_and_native_lua_hooks`), Task 9
      (`native_lua_hook_written_without_translate_is_refused`, `no_runtime_no_translate_rule`) and
      Task 10 (the `runtime_needed_*` tests);
    - R12 → Tasks 5 and 6;
    - R13 → Task 5;
    - R18 → plan 13 (noted in Task 8);
    - R25 → Task 1 (`op_path_refuses_empty`).

## Review findings

Reviewer: one round, batch with plans 05, 07 and 08. Blocking: none. The batch `cargo make dod` exited 0 (1034 unit, 275 doctests, runtime 8/8); the reviewer's `cargo test -p agentstacks --all-targets` exited 0.

- open 🔵 `BinaryRequirement::Extension` accepts any `Origin`.
- open 🔵 `propose` accepts non-extension origins, and its scans are O(n²).
- open 🔵 `id.rs:214`: probe/spec vocabulary in test code.
- open 🔵 `payloads_clone_equal` checks only `PlanInput`; Task 5 also names `ValuesInput` and `vars`.

## Deviations

- 2026-11-22: `SkipReason::new` and `require_binary` refusals carry plan 02's function prefix (`skip: reason: empty`, `require_binary: name: "Git" is not a valid name`), as pinned by `op/error.rs` tests. The plan's text omitted the prefix.
- 2026-11-22: `Proposals::check_path` takes an `ApiFunction` argument, because the refusal must start with `write:` or `copy:` and nothing else supplies it.
- 2026-11-22: `values.root` / `dir` path refusals are located at `Location::file(entry)`. At the key path the key would print twice and would not match the plan's expected text.
- 2026-11-22: `ValuesResult.vars` is a `HashMap<Var, String>`, because `Var` derives `Hash` but not `Ord`.
- 2026-11-22: `Origin::site()` returns `Option<Location>` (shown as `adapter.lua:9`), because `OpError::ProposedAt`, `SkipAfterCover` and `CoverAfterSkip` hold a `Location`.
- 2026-11-22: The T9 uncovered-files test adds `references/x.md` to the shared fixture so it can use the plan's example id.
- 2026-11-22: `Proposals` derives skip/cover state from its ordered records; the observable behaviour is the same.
- 2026-11-22: Added a `#[cfg(test)]` helper `op/testing.rs`, the same pattern as `set/testing.rs`.
- 2026-11-22: `PlanSkill`, `PlanAgent`, `PlanPack` and `PlanInput` derive `Debug + Clone` only, because `SkillMeta`/`AgentMeta` have no `PartialEq`.
- 2026-11-22: Iteration ran in a scratch copy of the crate while plan 08 kept the tree red; the real `cargo make dod` ran once at the end and was green.
