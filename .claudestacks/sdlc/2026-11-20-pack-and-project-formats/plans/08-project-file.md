---
status: draft
created: 2026-11-20
depends-on: [01, 02, 05]
built-on: e665589a43a9
---

# Project File Implementation Plan

**Goal:** `agentstacks.toml` parses into a `ProjectFile` that enforces every rule decidable from
the file alone.

**Architecture:** `source.rs` defines `Source::{Git { url, pin, subdir }, Path(RelPath)}` with
`Pin::{Tag(String), Rev(CommitId)}`; `git_url.rs` (`GitUrl`, refusing a leading `-`),
`commit_id.rs` (40 or 64 lowercase hex) and `rel_path.rs` (relative, `/`-separated; one
constructor for `subdir` that refuses any `..`, one for `path` that refuses absolute paths and any
path whose `..` segments climb above the project directory, decided lexically). `project.rs`
holds `ProjectFile { api, tools, packs }` where each pack entry carries its `Source` and raw
option values (`toml::Value`; typing them against declarations is plan 10). The same `Source`
type is reused by the lock (plan 09). `ProjectError` joins `FormatError`.

**Tech Stack:** Rust 2024, `toml` 1 (from plan 05).

**Content authority:** spec §7 (file-local rows: api, nothing to install, keys, git/path,
tag/rev, `-` URL, subdir, path, options placement, empty strings, unknown keys).

---

## Files that change

```
crates/agentstacks/src/model/mod.rs           — modify  declare and re-export
crates/agentstacks/src/model/git_url.rs       — create  GitUrl (non-empty, no leading "-")
crates/agentstacks/src/model/commit_id.rs     — create  CommitId (40 or 64 lowercase hex)
crates/agentstacks/src/model/rel_path.rs      — create  RelPath with subdir and project-path constructors
crates/agentstacks/src/model/source.rs        — create  Source, Pin
crates/agentstacks/src/model/project.rs       — create  ProjectFile::parse
crates/agentstacks/src/model/project_error.rs — create  ProjectError (thiserror)
crates/agentstacks/src/model/diagnostic.rs    — modify  FormatError gains a Project variant
```

## Order of work

### Task 1 — Add source value types

**Files:**
- Create `git_url.rs`, `commit_id.rs`, `rel_path.rs`, `source.rs`, `project_error.rs`; modify
  `mod.rs`, `diagnostic.rs`.

**Change:** parse-on-construction newtypes for each source field; `Source::from_fields(key,
git, tag, rev, subdir, path)` applies "exactly one of git, path" and "exactly one of tag, rev",
refuses `branch`, and refuses empty or whitespace-only strings.

**Proof:**
- `git_url_refuses_a_leading_dash` — `--upload-pack=x` gives `URL must not start with "-"`;
  `https://github.com/airsstack/agentstacks` parses.
- `commit_id_accepts_40_and_64_hex_only` — 40 and 64 lowercase hex parse; 39 hex, uppercase and
  `main` are refused (`not a commit id`).
- `subdir_refuses_escape_and_leading_slash` — `../x`, `a/../../x`, `/x` refused
  (`must stay inside the repository`); `packs/core` parses.
- `project_path_refuses_absolute_and_climbing_paths` — `/abs` (`absolute paths are not
  portable`) and `../x` (`must stay inside the project`) refused; `packs/team-lint` and
  `a/../b` parse.
- `source_requires_exactly_one_of_each_pair` — neither or both of git/path, neither or both of
  tag/rev, and an empty `tag` are refused with the §7 messages.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add pack and extension source types`.

### Task 2 — Parse `agentstacks.toml`

**Files:**
- Create `project.rs`; modify `project_error.rs`, `mod.rs`.

**Change:** `api` must be 0; `[tools.<ToolName>]` and `[packs.<PackName>]` tables each hold one
`Source`; `options` allowed only under `[packs.*]` (`tools.claude.options: unknown key`); at least
one tool and one pack (`agentstacks.toml: nothing to install`); invalid table keys
(`packs.Core: not a valid name`); unknown keys (`packs.core.branch: unknown key`). All errors
reported together.

**Proof:**
- `project_file_parses_the_spec_example` — the §7 example parses into two git sources (one tag,
  one subdir each) and one path source, with `archive_keep = 50` kept as a raw value.
- `project_file_refuses_each_file_local_rule` — table-driven over the §7 rows decidable from the
  file: `api = 1`, no packs, `[packs.Core]`, `branch = "main"`, `options` under a tool, empty
  `tag`, plus one case each for the source rules of Task 1 reached through a full file; exact
  messages and key paths.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse agentstacks.toml`.

---

## Risks

- `path` escape is decided lexically on the declared string; a symlinked directory inside the
  project is outside this chain (resolution is `core-plan-and-apply`'s).
- Rows that need fetched content (key vs fetched name, extension api, requires listed, cycles,
  option typing) are plan 10.

## Proof

- `cargo make dod-crate agentstacks` green; every file-local §7 row has a case.
