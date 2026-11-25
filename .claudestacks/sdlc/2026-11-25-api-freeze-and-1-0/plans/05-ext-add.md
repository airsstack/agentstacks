---
status: done
created: 2026-11-25
depends-on: [01, 02, 04]
built-on: c187d693fbea
---

# ext add Implementation Plan

**Goal:** `agentstacks ext add <SOURCE>` installs a tool extension into a project.

**Architecture:** `types/shorthand.rs` gains `Flag::Ext` and `Argument::Git`, so one classifier
serves `init` and `ext add` (modularity Rule 2). `types/project/add.rs` is the pure insert rule
(F2). `app/ext/add.rs` is the flow of §5.3 steps 1–5 over `Workspace` and `Git`, and it reuses the
plan's source resolution and `Extension::load`. The CLI adds `ext` as a nested subcommand (F6) and
runs step 6 with `update`'s confirm and follow-ups.

**Tech Stack:** Rust (agentstacks, agentstacks-cli), toml_edit 0.25.20, clap 4.6.9.

**Content authority:** spec §5.1–§5.4, D5, D11, D16, D19, F2, F6, F8, E9–E15, E26, §12 rows `types/project/add.rs`, `types/shorthand.rs`, `app/ext/add.rs`, CLI `ext add`.

---

## Files that change

```
crates/agentstacks/src/types/shorthand.rs        — modify: Flag::Ext; Argument::Git; URL rule; ext add's not-official text
crates/agentstacks/src/app/init.rs               — modify: the new Flag and Argument arms
crates/agentstacks/src/types/project/add.rs      — create: add_tool(text, name, source) → text with [tools.<name>] after the last tools table
crates/agentstacks/src/types/project/error.rs    — modify: ToolExists, ChangedDuringAdd
crates/agentstacks/src/types/project/mod.rs      — modify: exports add_tool
crates/agentstacks/src/app/ext/mod.rs            — create: exports only
crates/agentstacks/src/app/ext/add.rs            — create: the ext add flow
crates/agentstacks/src/app/mod.rs                — modify: pub mod ext
crates/agentstacks-cli/src/cli.rs                — modify: Ext(Ext) with Add { source, tag, rev, subdir, yes }
crates/agentstacks-cli/src/command/ext/mod.rs    — create: exports only
crates/agentstacks-cli/src/command/ext/add.rs    — create: wiring, confirm, follow-ups, exit codes
crates/agentstacks-cli/src/command/mod.rs        — modify: pub mod ext
crates/agentstacks-cli/src/main.rs               — modify: dispatch Command::Ext
crates/agentstacks-cli/tests/commands.rs         — modify: ext add end to end
```

## Order of work

### Task 1 — Classify an ext add source

**Files:**
- Modify `crates/agentstacks/src/types/shorthand.rs` — `Flag::Ext` (displays `ext add`, noun `tool`, official tools, directory `extensions`); `Argument::Git(GitUrl)` produced only for `Flag::Ext` when the text contains `://` or starts with `git@`; for `Flag::Ext` the not-official text is `not an official tool; pass a path or a git URL`; location `ext add "<SOURCE>"`; module doc names both callers
- Modify `crates/agentstacks/src/app/init.rs` — the exhaustive matches gain arms with no panic: in `resolve`'s `match flag` (`app/init.rs:186-190`), `Flag::Tool | Flag::Ext` load an extension; in `resolve`'s `match classify(…)` (`:172-175`), `Argument::Git(_)` cannot come from `Flag::Pack` or `Flag::Tool`, and its arm returns today's not-official refusal for that flag, so the function stays total

**Change:** §5.2. `init`'s classification is unchanged.

**Proof:**
- `ext_classifies_url_path_and_official` — `https://example.com/x.git` and `git@host:x.git` are git, `./ext/x` and `ext/x` are paths, `claude` is official.
- `ext_not_official_text` — `ext add "x": not an official tool; pass a path or a git URL`.
- `dash_name_is_not_official` — `-x` gives the same refusal (E26).
- `init_still_takes_no_url` — `Flag::Tool` with a URL behaves as today (a path or a not-official refusal), never `Argument::Git`.
- every existing `app/init.rs` test passes.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): classify an ext add source`.

### Task 2 — Insert a tool table

**Files:**
- Create `crates/agentstacks/src/types/project/add.rs` — `add_tool(text, name: &ToolName, source: &Source) -> Result<String, ProjectError>`: keys `git`, `tag` or `rev`, `subdir` (or `path` as written); placed after the last `tools.*` table; inline and dotted `tools` as F2; refuses an existing key
- Modify `crates/agentstacks/src/types/project/error.rs` — `ToolExists { name }` displayed `tools.<name> already exists (edit it there)`; `ChangedDuringAdd` displayed `changed during ext add (run it again)`; both located at `agentstacks.toml`
- Modify `crates/agentstacks/src/types/project/mod.rs` — export

**Change:** The pure rule of §5.3 steps 4 and 5.

**Proof:**
- `inserts_after_last_tool_table` — the F2 standard-tables input gives the F2 output byte for byte.
- `inline_tools` — the F2 inline input gives the F2 inline output.
- `dotted_tools` — the F2 dotted input gives the F2 dotted output.
- `keys_in_order_git_tag_subdir` — a git source with `rev` renders `git`, `rev`, `subdir` in order.
- `path_source_as_written` — `./ext/x` stays `./ext/x`.
- `existing_key_refused` — `tools.claude` present: `agentstacks.toml: tools.claude already exists (edit it there)`.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): insert a tool table keeping the file's layout`.

### Task 3 — The ext add flow

**Files:**
- Create `crates/agentstacks/src/app/ext/mod.rs` — four-question doc; exports `add`
- Create `crates/agentstacks/src/app/ext/add.rs` — `add<W: Workspace, G: Git>(workspace, git, source, tag, rev, subdir)`: §5.3 steps 1–5 (read and keep bytes; legacy refusal; classify; flag checks of §5.4; resolve and `Extension::load`; key check; re-read and compare bytes; insert; write); returns the added name
- Modify `crates/agentstacks/src/app/mod.rs` — `pub mod ext`

**Change:** Every §5.4 refusal that the flow can reach, in its exact text. Resolution follows `app/init.rs` and adds no git path: a path goes through `Workspace::source_tree` (`Ok(None)` is `StepError::Outside`, displayed `resolves outside the project`, as at `app/init.rs:176-179`), then `Extension::load`. An official name goes through `Git::list_tags` and the major rule of plan 04. A tag goes through `Git::fetch_tag`, a rev is used as it is, and then `Git::tree` at `--subdir` gives the tree for `Extension::load`. `app/plan/sources.rs` is not touched (`fetch_sources` is `pub(super)` over `&Inputs`).

**Proof:** (all with `FakeWorkspace`/`FakeGit`)
- `adds_a_path_extension` — a project plus `./ext/mytool`: the written file has `[tools.mytool]` with `path = "./ext/mytool"`.
- `adds_a_git_extension_with_tag` — a URL, `--tag v1.2.0`, `--subdir ext/mytool`: the F2 keys.
- `adds_the_official_extension` — `opencode` resolves to the newest `packs-v1.*` (plan 04) with `subdir = "extensions/opencode"`.
- `no_project_file` — `agentstacks.toml: No such file or directory (os error 2)` (E14).
- `legacy_project_refused` — `api = 0`: the §3.3 project text.
- `git_url_needs_tag_or_rev`, `flag_only_for_git` (one refusal per flag), `rev_not_a_commit`, `subdir_leaves_repository`, `empty_flag_value`, `path_absolute`, `path_climbs`, `path_out_through_symlink` (`FakeWorkspace` source tree `Ok(None)`: `ext add "./x": resolves outside the project`), `write_fails` (`FakeWorkspace` write error: `agentstacks.toml: <io::Error Display>`), `git_missing` (`ext add: git not found on PATH (needed by ext add "<SOURCE>")`), `git_fails` (`ext add: git <subcommand> failed: …`), `official_without_major_tag` — each gives its §5.4 text.
- `legacy_or_too_new_extension_refused` — the load refusal; nothing written (E12).
- `tool_exists` — E9.
- `changed_during_fetch` — the file bytes differ at re-read: `agentstacks.toml: changed during ext add (run it again)`, nothing written (E15).

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): the ext add flow`.

### Task 4 — The ext add command

**Files:**
- Modify `crates/agentstacks-cli/src/cli.rs` — `Command::Ext(Ext)` as a nested `#[command(subcommand)]`; `Ext::Add { source, tag (conflicts_with rev), rev, subdir, yes }`; help texts
- Create `crates/agentstacks-cli/src/command/ext/mod.rs` — exports only
- Create `crates/agentstacks-cli/src/command/ext/add.rs` — runs the flow; prints `agentstacks.toml: added tools.<name>` on stdout; then plan, show, confirm and apply as `command/update.rs` step 5 with the same follow-ups and exit codes (E13)
- Modify `crates/agentstacks-cli/src/command/mod.rs`, `crates/agentstacks-cli/src/main.rs` — dispatch
- Modify `crates/agentstacks-cli/tests/commands.rs` — end to end

**Change:** §5.1 and §5.3 step 6.

**Proof:**
- `ext_add_parses` (in `cli.rs`) — F6's argv list: the `ext add` lines parse as F6 records; `--tag` with `--rev` exits 2; `ext` alone exits 2.
- `ext_add_refused_plan_adds_the_fix_line` (in `command/ext/add.rs`) — a refused plan prints `agentstacks.toml updated; fix the above and run apply` and exits 1.
- `ext_add_no_terminal` — no `--yes` and no terminal: the update follow-ups, exit 1.
- `ext_add_a_path_extension` (in `tests/commands.rs`) — a project with `claude` plus a copied `extensions/opencode` under `ext/opencode`: `agentstacks ext add ./ext/opencode --yes` exits 0, its stdout has the line `agentstacks.toml: added tools.opencode`, the file has `[tools.opencode]`, and `plan` then reads `0 to write`.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks-cli): agentstacks ext add`.

---

## Risks

- Task 3 resolves through the same port calls `init` and the plan use. A git refusal keeps today's `FetchError` wording, located `ext add`.
- The CLI's `Ext` enum is shared with plan 06. This plan owns `Command::Ext` and `Ext::Add`, and plan 06 adds `Ext::Test` beside it.

## Proof

- `cargo make dod` is green.
- `agentstacks ext add --help` lists `--tag`, `--rev`, `--subdir`, `--yes`.

## Review findings

One `claudestacks:reviewer` over tasks 1–4 (`cargo make dod` exit 0; `ext add --help` lists `--tag`,
`--rev`, `--subdir`, `--yes`): 0🔴 6🟡 16🔵 2❓. Every code finding went to one fix coder; `cargo make
dod` after the fix: exit 0 (fmt, clippy, rustdoc, tests, doctests, lua 651 passed).

- fixed 🟡 `app/newest.rs`: colocated tests for the newest tag, no tag of the major, and a fetch error.
- fixed 🟡 `agentstacks-cli/src/command/ext/mod.rs`: plan reference removed; doc names the `added tools.<name>` line.
- fixed 🟡 `app/init.rs`: every argument location is `Location::verbatim`; test `init_shows_every_argument_as_written`.
- fixed 🟡 `app/ext/add.rs`: tests `subdir_not_found_is_located_at_the_source`, `cache_io_is_located_at_its_path`.
- fixed 🟡 E14 literal: CLI test `ext_add_without_project` asserts `agentstacks.toml: No such file or directory (os error 2)`, exit 1.
- fixed 🔵 `app/ext/add.rs`: module doc names every location; `add` takes `GitFlags`; steps described, not numbered; typed `ToolName` from the manifest (dead `InvalidName` branch gone).
- fixed 🔵 `app/init.rs`: the two `NewestError` arms merged through `From<NewestError> for FormatError`.
- fixed 🔵 `types/project/add.rs`: `add_tool` is `pub(crate)`.
- fixed 🔵 `types/project/error.rs`: `GitOption { Tag, Rev, Subdir }` replaces `&'static str` option names.
- fixed 🔵 `types/report/location.rs`: `Place { Path, Text }` replaces `verbatim: bool`; `file_path()` returns `Option<&Path>`.
- fixed 🔵 `types/shorthand.rs`: "blank" dropped; tests `ext_dash_url_refused_verbatim`; `init_still_takes_no_url` asserts the Path result and the refusal.
- resolved ❓ F2 inline output: the test's expected text matches `probes/deps-output-final.txt` "inline tools" byte for byte (` } , mytool = …`).
- resolved ❓ F6 argv list: `cli.rs` `ext_add_parses` covers the `ext add` lines of P6/P6b; the `ext test` lines belong to plan 06.
- recorded 🔵 plan drift items: see Deviations.

## Deviations

- 2026-11-25: Added beyond the Files list, each to keep one home per rule: `app/newest.rs` (newest official release lookup shared by `init` and `ext add`), `types/project` `GitFlags` and `GitOption`, `ProjectError::{GitNeedsPin, PinConflict, FlagNeedsGit, FlagValue}`, `types/report/location.rs` `Place` and `Location::verbatim` (the §5.4 texts quote the SOURCE as typed; `Location::file` rebuilt `https://` as `https:/`), `FakeWorkspace::set_project` (to test E15).
- 2026-11-25: `PinConflict` text `--tag and --rev cannot be used together`: the spec has no text because clap makes it unreachable (F6); the flow still refuses it instead of panicking.
- 2026-11-25: Git refusal locations: git missing and other git failures at `ext add`, a cache I/O failure at its cache path, a missing subdir or a subdir that is a file at `ext add "<SOURCE>"`, a path I/O error at the path (as `init`). The plan's Risks said "located `ext add`" for all.
- 2026-11-25: `init` locations for its arguments now print the argument as typed (`Location::verbatim`), which changes `init` messages only for arguments containing `//` or `/./`.
- 2026-11-25: `book/src/ch04-00-cli.md` gained the `ext add` grammar line, table row and stdout row in this plan (CLAUDE.md: book with the code). Plan 07 writes the full section.
- 2026-11-25: Tasks 3 and 4 ran as separate batches; one review covered the whole plan.
