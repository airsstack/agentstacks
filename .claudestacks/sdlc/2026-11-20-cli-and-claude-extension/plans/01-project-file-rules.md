---
status: done
created: 2026-11-22
built-on: cefc42fbbc4c
---

# Project file rules Implementation Plan

**Goal:** Add the pure rules that turn `init` arguments and update tags into `agentstacks.toml` text.

**Architecture:** Three new rule files in `types/`, each holding data in and an answer out, with no
port and no I/O. `types/shorthand` classifies an argument. `types/project/render` writes a new file.
`types/project/retag` changes tags in an existing file with its comments kept. The flows that call
them come in plan 02.

**Tech Stack:** Rust 2024, toml_edit 0.25 (new workspace dependency), the existing `types::project`,
`types::name` and `types::report` values.

**Content authority:** spec §4.2, §4.3 (the `types/shorthand` rows), §4.4, §7 (retag rule, D26),
§13 (toml_edit row).

---

## Files that change

```
Cargo.toml                                     — modify: toml_edit = "0.25" in [workspace.dependencies]
crates/agentstacks/Cargo.toml                  — modify: toml_edit = { workspace = true }
crates/agentstacks/src/types/mod.rs            — modify: pub mod shorthand
crates/agentstacks/src/types/shorthand.rs      — create: an init argument → an official git source or a path; the unknown-name refusal
crates/agentstacks/src/types/project/mod.rs    — modify: pub mod render, retag and their re-exports
crates/agentstacks/src/types/project/render.rs — create: tool and pack entries → the text of a new agentstacks.toml
crates/agentstacks/src/types/project/retag.rs  — create: agentstacks.toml text plus (key, new tag) pairs → new text, comments kept
```

## Order of work

### Task 1 — Classify init arguments

**Files:**
- Modify `crates/agentstacks/src/types/mod.rs` — declare `shorthand`.
- Create `crates/agentstacks/src/types/shorthand.rs` — the rule and its colocated tests.

**Change:** Introduce:
- `Flag` enum `{ Pack, Tool }`. It displays as `--pack` / `--tool` and gives its noun, `pack` / `tool`.
- Constants for the official repository `https://github.com/airsstack/agentstacks`, the tag prefix
  `packs-v`, the official pack names (`core`, `sdlc`, `journal`, `guideline-rust`,
  `guideline-python`, `cmux`) and the official tool names (`claude`). This is the single place
  formats §7 asks for.
- `Argument` enum:
  - `Official { url, subdir }`: the `GitUrl`, plus a `SubPath` of `packs/<name>` for a pack or
    `extensions/<name>` for a tool;
  - `Path(LocalPath)`.
- `classify(flag, arg) -> Result<Argument, Diagnostic>`:
  - An argument that contains `/` or starts with `.` is a path, built through `LocalPath::new`.
  - Anything else is an official name. A name `./core` would match is still a name.
- Refusals are located with `Location::file` holding the text `--pack "<arg>"` or `--tool "<arg>"`
  (`src/types/report/location.rs:70`):
  - unknown name: `not an official pack; pass a path, or add a git source to agentstacks.toml`, or
    the `tool` form;
  - a bad path: the `LocalPath` refusal text as it stands (`absolute paths are not portable`,
    `must stay inside the project`).

**Proof:**
- `official_pack_expands_to_the_repository_and_packs_subdir`: `core` gives the URL and `packs/core`.
- `official_tool_expands_to_extensions_subdir`: `claude` gives `extensions/claude`.
- `every_official_pack_name_is_accepted`: all six pack names classify as official.
- `slash_or_leading_dot_makes_a_path`: `./packs/core`, `packs/x` and `.hidden` are paths.
- `bare_name_is_never_a_path`: `core` stays official.
- `unknown_pack_is_refused_with_the_flag_location`: Display is exactly
  `--pack "x": not an official pack; pass a path, or add a git source to agentstacks.toml`.
- `unknown_tool_is_refused_with_the_tool_wording`: Display is exactly
  `--tool "x": not an official tool; pass a path, or add a git source to agentstacks.toml`.
- `absolute_path_is_refused`: `--pack "/x": absolute paths are not portable`.
- `climbing_path_is_refused`: `--pack "../x": must stay inside the project`.

**Done when:** `cargo test -p agentstacks types::shorthand` passes and `cargo make dod` is green.
Commit `feat(agentstacks): classify init arguments as official names or paths`.

### Task 2 — Render a new project file

**Files:**
- Modify `Cargo.toml` — add `toml_edit = "0.25"` to `[workspace.dependencies]`.
- Modify `crates/agentstacks/Cargo.toml` — `toml_edit = { workspace = true }`.
- Modify `crates/agentstacks/src/types/project/mod.rs` — declare `render`, re-export `render_project`.
- Create `crates/agentstacks/src/types/project/render.rs` — the rule and its colocated tests.

**Change:** Add `render_project(tools, packs) -> String`:
- Inputs are `(ToolName, Source)` and `(PackName, Source)` entries.
- The output is built with toml_edit:
  - `api = 0` first;
  - `tools` and `packs` are implicit tables, so no bare `[tools]` or `[packs]` header appears;
  - tools in name order, then packs in name order;
  - a blank line before each table.
- A git source writes `git`, `tag`, `subdir` in that order.
- A path source writes `path = "<as written>"`.

**Proof:**
- `official_set_renders_the_exact_bytes`: for tool `claude` and packs `core` and `sdlc`, all at
  `packs-v0.1.0`, the output equals the spec §4.4 byte string exactly.
- `rendered_text_parses_as_a_project_file`: that text parses through `ProjectFile::parse` with no
  refusal.
- `path_source_renders_one_path_key`: pack `team` at `packs/team` renders `[packs.team]\npath = "packs/team"\n`.
- `no_empty_parent_table_is_written`: the output contains neither `[tools]\n` nor `[packs]\n`.
- `entries_are_sorted_by_name_within_each_section`: passing packs `sdlc` then `core` renders `core` first.

**Done when:** `cargo test -p agentstacks types::project::render` passes and `cargo make dod` is
green. Commit `feat(agentstacks): render a new agentstacks.toml from its entries`.

### Task 3 — Retag a project file with comments kept

**Files:**
- Modify `crates/agentstacks/src/types/project/mod.rs` — declare `retag`, re-export `retag_project`.
- Create `crates/agentstacks/src/types/project/retag.rs` — the rule and its colocated tests.

**Change:** Add `retag_project(text, changes) -> Result<String, Diagnostic>`:
- `changes` is a list of `(SourceKey, current: GitTag, new: GitTag)`.
- For each change, walk `<packs|tools>` → `<name>` → `tag` with `get_mut` at every level, never a
  chained `IndexMut`.
- When the old value's text differs from `current`, refuse with
  `agentstacks.toml: packs.core: tag changed during update (run update again)` (spec D26).
- Otherwise replace the value, then copy the old value's decor onto the new one.
- A level that is missing refuses with
  `agentstacks.toml: packs.core: no tag to update (run update again)`. The same shape is used for
  `tools.*`, built from `SourceKey` Display (`src/types/resolve/key.rs:64`).
- One refusal stops the whole call, and no partial text is returned.

**Proof:**
- `trailing_comment_survives_a_retag`: a `tag = "packs-v0.1.0" # pinned` line becomes
  `tag = "packs-v0.2.0" # pinned`, and every other byte is unchanged.
- `inline_table_tag_is_retagged`: `core = { git = "…", tag = "packs-v0.1.0" } # c` keeps `# c`.
- `dotted_key_tag_is_retagged`: `packs.core.tag = "packs-v0.1.0" # c` keeps `# c`.
- `tool_tag_is_retagged`: a `tools.claude` change edits only that table.
- `missing_pack_is_refused_and_writes_nothing_into_the_document`: the change `packs.nope` refuses
  with the exact message, and the text never gains a `[packs]\nnope = {}` table.
- `several_changes_apply_in_one_call`: two keys are both changed.
- `changed_tag_is_refused`: the file holds `packs-v0.1.5` while the change expects `packs-v0.1.0`,
  giving exactly `agentstacks.toml: packs.core: tag changed during update (run update again)`.

**Done when:** `cargo test -p agentstacks types::project::retag` passes and `cargo make dod` is
green. Commit `feat(agentstacks): retag agentstacks.toml with comments kept`.

---

## Risks

- **Retag is the riskiest step.** toml_edit's decor and `IndexMut` behaviour are the trap, and both
  are proven (spec §7: R2 (a)/(a2), T1 (a)-(d)). The tests pin each probed case.
- **Duplicate crates.** toml_edit 0.25 next to airsl's 0.22 adds duplicate crates. They fall under
  the existing `skip-tree` (spec §13, R5), and `cargo deny check` is run in plan 09's proof.

## Proof

- `cargo make dod` green.
- `cargo test -p agentstacks types::shorthand types::project` lists every test above passing.

## Review findings

- 🟡 `shorthand.rs`, `retag.rs`: refusals were carried as `FormatError::Io(io::Error::other(..))`. Fixed: `ShorthandError::NotOfficial(Flag)` + `FormatError::Shorthand`; `ProjectError::TagChanged` / `ProjectError::NoTagToUpdate`. Display unchanged. `cargo make dod-crate agentstacks` → exit 0.
- 🟡 `render.rs`: `Pin::Rev` branch untested. Fixed: `rev_source_renders_rev_instead_of_tag`. `cargo make dod-crate agentstacks` → exit 0.
- 🔵 open: duplicate names collapse silently in `render_project` (undocumented); `PROJECT_FILE` const repeated (`retag.rs`, `resolve/key.rs`); `(SourceKey, GitTag, GitTag)` is positional; a key given twice in `changes` reports "tag changed"; `Value::from` turns a literal `'packs-v0.1.0'` into a basic string; `types::Flag` is a generic root name; `render_project`/`retag_project` not re-exported at the `types` root.

## Deviations

- 2026-11-22 Task 3: "`get_mut` at every level" — toml_edit 0.25 `Item::get_mut` calls `index_mut`, which inserts. `cargo test -p agentstacks types::project::retag::tests::missing_pack` with an `Item::get_mut` chain left `[packs]\nnope = {}` in the document. Built: `DocumentMut::get_mut` → `as_table_like_mut` → `TableLike::get_mut` → `as_value_mut`. The spec's rule (no inserting lookup) holds; not a disproval.
- 2026-11-22 Task 2: `render_project` writes `rev` for a `Pin::Rev` source (the match must cover it).
- 2026-11-22 Tasks 1, 3: refusal types added outside the file list — `FormatError::Shorthand` in `types/report/format.rs`, two `ProjectError` variants in `types/project/error.rs`.
- 2026-11-22 Task 2: first resolve locked toml_edit 0.25.18; pinned to the probed 0.25.20 with `cargo update -p toml_edit@0.25.18+spec-1.1.0 --precise 0.25.20` (toml_parser 1.1.5→1.1.6, toml_datetime 1.1.2→1.1.3). `cargo tree -i toml_parser -e normal` shows one toml_parser shared by toml and toml_edit.
- 2026-11-22 (process): coders read their task sections straight from this plan file instead of a task-briefer brief; the ledger came from `grep '^### Task'`. Earlier batches were staged in the git index (not committed) so each review saw only its own batch.
