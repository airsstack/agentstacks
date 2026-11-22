---
status: approved
created: 2026-11-22
depends-on: [01]
built-on: cefc42fbbc4c
---

# Init and retag flows Implementation Plan

**Goal:** Give the library the flows that write `agentstacks.toml`, for `init` and for `update`.

**Architecture:**
- The `Workspace` port gains `write_project`. The real adapter writes through temp + rename, and the
  fake records the text.
- `app/init` is a flow generic over `W: Workspace, G: Git` (static dispatch). It runs check →
  classify → resolve → render → write and calls the plan 01 rules.
- `app/update::retag` re-reads the file, applies the `NewTag` entries through
  `types/project/retag` and writes once.
- No rule lives in the flows.

**Tech Stack:** Rust 2024; the existing ports (`src/port/workspace/seam.rs:41`,
`src/port/git/seam.rs`) and fakes (`src/port/workspace/fake.rs:125`, `src/port/git/fake.rs:94`).

**Content authority:** spec D19, D22, D26; §4.1, §4.2 (duplicate rows), §4.3 (the `app/init`
and `fs` rows, git missing), §7 step 4 and its refusal table, §8.1, §8.2.

---

## Files that change

```
crates/agentstacks/src/port/workspace/seam.rs  — modify: Workspace::write_project(&self, text) -> io::Result<()>
crates/agentstacks/src/port/workspace/fake.rs  — modify: FakeWorkspace records the written project text, and can fail the write
crates/agentstacks/src/fs/workspace/swap.rs    — modify: write_project helper, temp + rename with the old mode kept
crates/agentstacks/src/fs/workspace/project.rs — modify: StdWorkspace::write_project calls it
crates/agentstacks/src/app/mod.rs              — modify: pub mod init
crates/agentstacks/src/app/init.rs             — create: the init flow
crates/agentstacks/src/app/update.rs           — modify: retag()
```

## Order of work

### Task 1 — Add `Workspace::write_project`

**Files:**
- Modify `crates/agentstacks/src/port/workspace/seam.rs` — the new trait method, documented with its
  `# Errors`.
- Modify `crates/agentstacks/src/port/workspace/fake.rs`:
  - `FakeWorkspace::project_written() -> Option<String>`;
  - a `WorkspaceCall::WriteProject` variant, so `fail(WorkspaceCall::WriteProject)` follows the
    existing failure pattern (`src/port/workspace/fake.rs:44-63`, `:214`).
- Modify `crates/agentstacks/src/fs/workspace/swap.rs`:
  - `write_project(project, text)` writes `.agentstacks.toml.agentstacks-tmp` beside the file, then
    renames it over `agentstacks.toml`;
  - when the old file exists, its mode is kept, the same way `write_lock` keeps it
    (`src/fs/workspace/swap.rs:50`).
- Modify `crates/agentstacks/src/fs/workspace/project.rs` — `StdWorkspace::write_project`.

**Change:** The port can now write the project file. The flow wraps the I/O error in its
`agentstacks.toml: <io::Error Display>` refusal.

**Proof:**
- `write_project_replaces_the_file_and_leaves_no_temp` (fs contract, tempfile): the text is written,
  and `.agentstacks.toml.agentstacks-tmp` is absent afterwards.
- `write_project_keeps_the_old_mode` (fs contract): a `0o600` file stays `0o600` after the write.
- `write_project_creates_the_file_when_absent` (fs contract).
- `fake_records_the_written_project_text` (fake unit).

**Done when:** `cargo test -p agentstacks write_project` passes and `cargo make dod` is green. Commit
`feat(agentstacks): write agentstacks.toml through the workspace port`.

### Task 2 — The `init` flow

**Files:**
- Modify `crates/agentstacks/src/app/mod.rs` — declare `init` and list it under Responsibilities.
- Create `crates/agentstacks/src/app/init.rs` — the flow and its tests on fakes.

**Change:** Add `init(workspace, git, tools: &[String], packs: &[String]) -> Result<(), Diagnostics>`.
Steps, in order:
1. `Workspace::project_text`:
   - `Ok` refuses with `agentstacks.toml: already exists (edit it and run apply)`, and nothing is
     written;
   - an `Err` other than `NotFound` refuses with `agentstacks.toml: <io::Error Display>`.
2. Collapse repeated identical arguments per flag, keeping first-seen order. Classify each with
   `types::shorthand::classify`.
3. Resolve each argument.
   - **Official:**
     - `Git::list_tags` on the official URL, then `types::resolve::latest_tag(tags, "packs-v")`
       (`src/types/resolve/tag.rs:90`).
     - No match refuses with
       `--pack "core": no packs-v release tag in https://github.com/airsstack/agentstacks`.
     - `FetchError::GitNotFound` is collected, not refused per argument (spec D19).
     - Any other `FetchError` refuses with `--pack "core": ` + its Display.
     - The key is the argument's name.
   - **Path:**
     - `Workspace::source_tree(&path)`. `Ok(None)` refuses with `--pack "./x": resolves outside the
       project`, and an `Err` with `<path as written>: <io::Error Display>`.
     - The tree is loaded with `Pack::load` (`src/types/pack/load.rs:136`) or `Extension::load`
       (`src/types/extension/loaded.rs:101`). Their refusals pass through unchanged.
     - The key is the loaded manifest's name.
4. Two arguments of one flag with the same key refuse at the later one:
   `--pack "./packs/core": pack "core" is already given by --pack "core"`, with the tool form
   `--tool "./ext": tool "claude" is already given by --tool "claude"`.
5. If any argument met `GitNotFound`, add one refusal at `Location::file("init")` with
   `FetchError::GitNotFound { needed_by }`. `needed_by` holds each such argument's flag text
   (`--tool "claude"`), tools first, then packs, in the order given, after step 2's collapse. It
   displays as `init: git not found on PATH (needed by --tool "claude", --pack "core")`.
6. Collect every refusal of steps 2-5. If any, return them all, and nothing is written.
7. `render_project`, then `Workspace::write_project`. A write failure refuses with
   `agentstacks.toml: <io::Error Display>`.

Every refusal is a `Diagnostic` located as spec §10 shows: the flag text, the path as written, or
`agentstacks.toml`.

**Proof:** written first on `FakeWorkspace` and `FakeGit`.
- `init_refuses_an_existing_project_file`: exact message, and `project_written()` is `None`.
- `init_reports_a_read_error_other_than_not_found`: `agentstacks.toml: <Display>`.
- `init_writes_the_official_set`: with tags `packs-v0.1.0` and `packs-v0.2.0` listed, the written
  text equals `render_project` of the three entries at `packs-v0.2.0`.
- `init_collapses_a_repeated_argument`: `--pack core --pack core` writes one entry.
- `init_refuses_two_arguments_with_the_same_key`: official `core` plus path `./packs/core` (whose
  `pack.toml` names `core`) gives the exact duplicate message.
- `init_refuses_the_tool_duplicate_with_the_tool_wording`.
- `init_refuses_when_no_release_tag_exists`: exact message.
- `init_passes_a_fetch_error_through_at_the_flag`: a `GitFailed` gives `--pack "core": ` + its Display.
- `init_merges_git_missing_into_one_refusal`: `FakeGit::git_missing_for` on the official URL, with
  `--tool claude --pack core --pack core --pack ./packs/team`, gives exactly one refusal,
  `init: git not found on PATH (needed by --tool "claude", --pack "core")`. The path argument
  resolves, and nothing is written.
- `init_refuses_a_path_that_resolves_outside`: `FakeWorkspace::source_outside` gives the exact
  message.
- `init_passes_a_loader_refusal_through`: a source tree without `pack.toml` refuses with the
  loader's `not a pack (no pack.toml)`.
- `init_collects_every_refusal_before_writing`: an unknown name plus a bad path gives two refusals,
  and nothing is written.
- `init_reports_a_failed_write`: `fail(WorkspaceCall::WriteProject)` gives `agentstacks.toml: <Display>`.
- `init_reports_a_path_io_error_as_written`: `FakeWorkspace::fail(WorkspaceCall::SourceTree(..))` (`src/port/workspace/fake.rs:214`) gives `./packs/x: <Display>`.
- `init_writes_a_path_source_as_given`: `--pack ./packs/team` writes `path = "./packs/team"` under
  the manifest's name.

**Done when:** `cargo test -p agentstacks app::init` passes and `cargo make dod` is green. Commit
`feat(agentstacks): init flow writes agentstacks.toml from arguments`.

### Task 3 — `update::retag`

**Files:**
- Modify `crates/agentstacks/src/app/update.rs`:
  - add `retag`;
  - update the module docs, whose Non-responsibilities say "It never writes the file"
    (`src/app/update.rs:3-4`);
  - add tests.

**Change:** Add `retag(workspace, entries: &[UpdateEntry]) -> Result<bool, Diagnostics>`:
- It keeps only the entries whose outcome is a newer tag (`UpdateOutcome`,
  `src/types/resolve/update.rs:97`).
- With none, it returns `Ok(false)` and writes nothing.
- Otherwise it re-reads `Workspace::project_text` and checks the text with `ProjectFile::parse`,
  whose refusals pass through. It then applies `retag_project` once with every
  `(key, current tag, new tag)`, calls `write_project` once, and returns `Ok(true)`.
- Read and write failures are `agentstacks.toml: <io::Error Display>`.
- A retag refusal passes through.

**Proof:**
- `retag_writes_every_newer_tag_once`: two newer entries mean one write holding both tags.
- `retag_with_no_newer_tag_writes_nothing`: `Ok(false)`, and `project_written()` is `None`.
- `retag_refuses_text_that_no_longer_parses`: re-read text `[packs.core\n` gives
  `agentstacks.toml:1: unclosed table, expected `]``, and nothing is written.
- `retag_refuses_a_tag_changed_at_re_read`: the exact D26 message, and nothing is written.
- `retag_reports_a_failed_write`: `fail(WorkspaceCall::WriteProject)` gives `agentstacks.toml: <Display>`.
- `retag_refuses_a_key_missing_at_re_read`: the project text lacks the pack, giving the exact
  `no tag to update (run update again)` message, and nothing is written.

**Done when:** `cargo test -p agentstacks app::update` passes and `cargo make dod` is green. Commit
`feat(agentstacks): write newer tags into agentstacks.toml`.

---

## Risks

- **Port change breaks `isolation.rs`.** `write_project` is the only port change. The fake and the
  adapter must both compile in one task (Task 1), or other crates' builds break.
- **Path arguments load the whole pack.** `Pack::load` and `Extension::load` are reused to get the
  manifest name, so a broken pack refuses at `init`. This is intended (spec §4.3 "existing
  loaders").

## Proof

- `cargo make dod` green.
- `cargo test -p agentstacks app::init app::update write_project` all pass.
