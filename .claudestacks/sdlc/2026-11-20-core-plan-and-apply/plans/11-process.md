---
status: done
created: 2026-11-22
depends-on: [09, 10]
built-on: f39b2a814319
---

# Process Implementation Plan

**Goal:** drive git and binaries for the flows.

**Architecture:** `src/process/` is the only `std::process` caller (spec §2). It holds two adapters:

- `GitCli` implements plan 09's `Git` on the `git` command line, with the exact §9.2 commands. Every
  disk step of the cache goes through plan 10's `Cache`, so `std::fs` stays in `fs`. This is the one
  allowed edge from one adapter to another (spec §2, X3: `process` may use `fs::cache` and nothing
  else from `fs`). Parsing `ls-remote` lines (`parse_ls_remote`, `RemoteTag`) and `ls-tree` records
  (`parse_ls_tree`, `TreeRecord::kind`, `TreeRecord::relative`), the subdir check and the temp name
  (`temp_name`) are plan 07's pure functions. Failures are plan 02's `FetchError`, unlocated: the
  flows add `agentstacks.toml` and the key (§13). What stays here is
  translation: building argument vectors, running git, reading its output, and serving a rev's files
  from memory as a `FileTree`.
- `SystemBinary` implements plan 09's `Binary`: a `PATH` lookup and the `airsl --version` answer
  (spec §10, §8 rows 6-8).

```text
src/process/
├─ mod.rs          docs + mod / pub use (export-only)
├─ binary.rs       SystemBinary: PATH lookup, airsl --version
└─ git/
   ├─ mod.rs       export-only
   ├─ command.rs   one git command: argv shape, environment, run, failure mapping (§9.2 rules + failure rows 1-2)
   ├─ blobs.rs     `cat-file --batch`: feed object ids, read the framed answers (§9.2 "read blobs")
   ├─ snapshot.rs  Snapshot: a rev's files in memory, served through FileTree (§9.2 tree paragraph, rows 4-5)
   ├─ cli.rs       GitCli: implements Git (list tags, fetch a tag, a rev's tree; §9.2, §9.3)
   └─ testing.rs   #[cfg(test)] a local source repository built with git in a temp dir
```

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6): `mod-rs-export-only.md`
(`process/mod.rs`, `process/git/mod.rs`); `modularity.md` Rule 1 (one job per file above) and Rule
2 (no process copy of `FetchError`, `EntryKind`, `AirslAnswer`, the record parsers or the cache
names); `static-dispatch.md` (concrete adapters; `GitCli::Tree` is `Snapshot`; no trait objects);
`strong-types.md` (`GitUrl`, `GitTag`, `CommitId`, `SubPath`, `BinaryName` at every boundary);
`unit-test-mandate.md` (pure parts tested on byte strings; contract tests against real git on a
local `file://` repository, never the network, spec §14); `doc-comment-discipline.md`.

**Tech Stack:** Rust 2024, crate `agentstacks`; `std::process::{Command, Stdio}`,
`std::thread::scope` (to feed `cat-file` while reading it), `std::env::{var_os, split_paths}`;
`tempfile` (dev-dependency) for test repositories and cache homes; git on the developer and CI
machine (probed: git 2.39.3, `probes/git-results.md`). No new dependency.

**Content authority:** spec §9.2 (all rows and both tables), §9.3 (filling the cache from git's
side, HOME), §10 (the `PATH` lookup and `airsl --version`), §8 rows 6-8 (the airsl answer), §14
rows `process::GitCli` and `process::Binary`, D11, §2 dependency rule with X3, §16 R1, R2, R9, R10,
R21, R22, R26.

---

## Files that change

```
crates/agentstacks/src/process/mod.rs            — modify: module docs (four questions); mod + pub use GitCli, Snapshot, SystemBinary
crates/agentstacks/src/process/binary.rs         — create: SystemBinary
crates/agentstacks/src/process/git/mod.rs        — create: export-only
crates/agentstacks/src/process/git/command.rs    — create: argv shape, environment, run, failure mapping
crates/agentstacks/src/process/git/blobs.rs      — create: cat-file --batch reader
crates/agentstacks/src/process/git/snapshot.rs   — create: Snapshot (FileTree in memory)
crates/agentstacks/src/process/git/cli.rs        — create: GitCli implements Git
crates/agentstacks/src/process/git/testing.rs    — create (#[cfg(test)]): source repository builder
```

The plan-set map lists `process/git.rs`; it becomes the `process/git/` folder because one file
would hold five jobs (modularity Rule 1). Ownership is unchanged: `src/process/*`.

## Order of work

### Task 1 — Run one git command the way §9.2 requires

**Files:**
- Create `crates/agentstacks/src/process/git/command.rs` — argv, environment, run, failure mapping.
- Create `crates/agentstacks/src/process/git/mod.rs` — export-only.
- Create `crates/agentstacks/src/process/git/testing.rs` — the test repository builder.
- Modify `crates/agentstacks/src/process/mod.rs` — `pub mod git;`.

**Change:** `GitRun` (crate-private) describes one command: the program, an optional repository
(`-C <repo>`), the subcommand, its arguments, an optional URL and the arguments after it, and an
optional stdin. `argv()` (pure) builds `-c core.autocrlf=false [-C <repo>] <subcommand> <args…>
[-- <url> <after…>]`: the URL always follows `--` (§9.2 "passes the URL after `--`"; together with
`GitUrl`'s leading-`-` refusal and G6's `strange hostname … blocked`, no URL reaches a flag).
`run()` spawns it with `GIT_TERMINAL_PROMPT=0` set on the command (§9.2, D11) and stdout and stderr
captured; a stdin is written from a scoped thread while the output is read, so a large `cat-file`
answer cannot deadlock the two pipes. Outcomes:

- spawn fails with `io::ErrorKind::NotFound` → plan 02's `GitNotFound` with an empty `needed_by`,
  the per-source git-not-found outcome (§9.2 row 1; the io error does not name the program, G8).
  Plans 13 and 16 merge every source's outcome into the single `agentstacks.toml: git not found on
  PATH (needed by packs.core, tools.claude)` refusal;
- spawn fails any other way → plan 02's R1 spawn variant carrying the `io::Error`, shown as
  `agentstacks.toml: packs.core: git: <io::Error Display>` (R1);
- exit status non-zero → plan 02's `GitFailed` carrying the subcommand word (`ls-remote`, `init`,
  `fetch`, `rev-parse`, `ls-tree`, `cat-file`) and git's stderr decoded as lossy UTF-8 and trimmed of
  surrounding whitespace (R2), shown as `agentstacks.toml: packs.core: git <subcommand> failed:
  <git's stderr, trimmed>` (§9.2 row 2; D3 "tool errors are passed through");
- success → stdout bytes.

Git output that does not parse is R2. Tasks 2-4 build that refusal only through plan 07's
unlocated `unreadable(step, output)` → `FetchError` (the step is plan 07's `GitStep` for the
subcommand), which quotes the first line of the output, lossy UTF-8 and trimmed, shown by the flows
as `agentstacks.toml: packs.core: git <subcommand> printed output agentstacks cannot read: "<first
line>"` (R2). `process` has no copy of that rule.

`testing.rs` builds, in a `tempfile` dir, the G-probe source repository (`probes/git-results.md`
"Source repo"): commits `zero` (untagged), `one` (annotated tag `packs-v0.1.0`) and `two`
(lightweight `packs-v0.2.0`, `packs-v0.9.0`, `packs-v0.10.0`, `other-v9.9.9`), with
`packs/core/pack.toml`, a CRLF blob stored with `hash-object -w --no-filters` + `update-index
--cacheinfo`, a symlink `packs/core/link`, an executable `run.sh`, a nested file and a gitlink
`packs/core/sub` (`update-index --cacheinfo 160000,…`). Its setup commands run with
`GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, fixed author and committer names and dates,
so the test repository does not depend on the developer's config; `GitCli` itself never sets those.
It returns the `file://` URL and the commit ids.

**Proof:** written first, seen failing, then passing.
- `argv_puts_autocrlf_first_and_the_url_after_double_dash` — for a fetch with repo `/r`, URL `u`
  and refspec `x`, `argv()` is `-c core.autocrlf=false -C /r fetch --no-tags --depth 1 -- u x`.
- `run_sets_git_terminal_prompt_off` — the environment of the built `Command` holds
  `GIT_TERMINAL_PROMPT=0`.
- `a_missing_program_is_git_not_found` — program `/nonexistent/dir/git` gives the §9.2 row 1
  outcome (G8 `kind=NotFound`).
- `a_failing_command_carries_subcommand_and_trimmed_stderr` — `ls-remote` of `file:///nope` gives
  `GitFailed` with subcommand `ls-remote` and stderr starting `fatal: '/nope' does not appear to be
  a git repository` with no trailing newline (G7).
- `a_spawn_error_other_than_not_found_is_r1` — the program set to a directory in a `tempfile` dir
  (the spawn fails with an error other than `NotFound`) gives the R1 spawn variant, whose Display
  after its location is `git: <that io::Error Display>`.
- `stdin_is_fed_while_output_is_read` — `cat-file --batch` over 2,000 copies of one blob id of the
  test repository returns all 2,000 answers (more than one pipe buffer each way).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): run git commands without prompts or option-shaped urls`.

### Task 2 — Read blob bytes from `cat-file --batch`

**Files:**
- Create `crates/agentstacks/src/process/git/blobs.rs` — the batch reader.
- Modify `crates/agentstacks/src/process/git/mod.rs`.

**Change:** `read_blobs(repo, ids)` (crate-private) runs `git -C <repo> cat-file --batch` (§9.2
"read blobs") with one object id per stdin line and parses the answer framing proven in G5: a header
`<oid> SP <type> SP <size> LF`, exactly `<size>` content bytes, then `LF`. `parse_batch(bytes)`
(pure) returns the bytes per id in request order, unconverted (§9.2 "bytes unconverted even with
`core.autocrlf=true`"). An answer `<oid> missing` (G5) or a malformed frame is R2: the Task 1
plan 07's `unreadable` for the `cat-file` step, quoting the first line of the output.

**Proof:**
- `parse_batch_reads_crlf_bytes_verbatim` — the G5 frame for `c30dea8…` gives `a\r\nb\r\n`.
- `parse_batch_reads_several_frames_in_order` — two frames (CRLF and LF blobs) give both contents in
  order, each without the trailing frame `LF`.
- `parse_batch_keeps_a_newline_inside_the_content` — a blob whose last content byte is `\n` keeps it.
- `parse_batch_missing_object_is_unreadable` — the G5 answer
  `0123456789abcdef0123456789abcdef01234567 missing` gives the R2 outcome quoting that line.
- `parse_batch_short_frame_is_unreadable` — a header announcing 6 bytes followed by 3 gives the R2
  outcome.
- `read_blobs_returns_crlf_bytes_from_git` — on the test repository, with `core.autocrlf=true` set
  in that repository's config, the CRLF blob still comes back as `a\r\nb\r\n` (G5).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): read git blobs in one batch`.

### Task 3 — Serve a rev's files from memory as a `FileTree`

**Files:**
- Create `crates/agentstacks/src/process/git/snapshot.rs` — `Snapshot`.
- Modify `crates/agentstacks/src/process/git/mod.rs`, `crates/agentstacks/src/process/mod.rs` (`pub use`).

**Change:** `Snapshot` is a rev's tree read once and kept in memory (§9.2 "The source's tree is read
into memory once and served through the `FileTree` port"). It is built from plan 07's
`parse_ls_tree` records of `ls-tree -r -z --full-tree` (`<mode> SP <type> SP <oid> TAB <path> NUL`,
byte-ordered, G4, G9) and the blob bytes of Task 2. `parse_ls_tree` is unlocated and returns
`FetchError` itself for a listing it cannot read (R2), which is passed on unchanged. Building it
first applies plan 07's unlocated `check_subdir` (returning `FetchError`), the two
`subdir` rows of §9.2's failure table (§13 lets `process` raise source refusals), with plan 02's
variants:

- `subdir` given and the listing is empty (G4 `packs/nope`) → `SubdirNotFound`, displayed after its
  location as `"packs/nope" not found at <tag or rev>`, where `<tag or rev>` is the source's pin
  (`at`, plan 09);
- `subdir` given and the listing is exactly one blob record whose path equals the normalised
  `subdir` (G4 `packs/core/pack.toml`) → `SubdirIsFile`, displayed as `"packs/core/pack.toml" is a
  file, not a directory`.

Both quote `subdir` as written in `agentstacks.toml` (R9 "Messages quote a value as written").
Each record's path loses the `<normalised subdir>/` prefix (`TreeRecord::relative`; the source's
root is the subdir), so `kind`, `read_dir` and `read_file` take paths relative to it. Its
`EntryKind` is `TreeRecord::kind`: `100644` and `100755` → `File`, `120000` → `Symlink` (so formats'
"symlinks are not allowed" refusal applies), any other → `Other` (§9.2 tree bullets, R10 "Git
modes"). Every ancestor of a record is a `Dir`;
the empty path is the root. `read_dir` lists direct children; `read_file` returns a file's bytes,
refuses a symlink with `InvalidInput` displayed `symlinks are not followed` (the wording
`StdFileTree` uses), and answers other kinds as `FakeFileTree` does. A path not in the tree is
`NotFound`. On Unix a path's bytes become an `OsString` unchanged, so a non-UTF-8 name reaches
formats' own refusal. `root()`, the label that prefixes the source's load messages, is the
`subdir` as written, or `.` without one (R22).

**Proof:**
- `snapshot_strips_the_subdir_prefix` — records under `packs/core/` are read as `pack.toml`,
  `nested/deeper/file.md`.
- `snapshot_synthesises_directories` — `kind("nested")` is `Dir`; `read_dir("")` lists `nested`,
  `pack.toml`, … once each.
- `snapshot_maps_symlink_and_gitlink_kinds` — `link` is `Symlink`, `sub` is `Other`, `run.sh` is
  `File`.
- `snapshot_refuses_to_read_a_symlink` — `read_file("link")` is `InvalidInput`, `symlinks are not
  followed`.
- `empty_listing_with_subdir_is_not_found` — Display (after the location) `"packs/nope" not found at
  packs-v0.1.0` for a tag pin.
- `single_blob_equal_to_subdir_is_a_file` — Display `"packs/core/pack.toml" is a file, not a
  directory`.
- `subdir_is_quoted_as_written` — subdir `./packs//nope` with an empty listing displays
  `"./packs//nope" not found at packs-v0.1.0` (R9).
- `unreadable_listing_is_r2` — a record without a tab gives the R2 outcome for `ls-tree`.
- `an_unknown_mode_is_other` — a record with mode `100664` is `EntryKind::Other` (R10).
- `root_label_is_the_subdir_as_written_or_dot` — `root()` is `packs/core` for subdir `packs/core`,
  and `.` without a subdir (R22).
- `snapshot_hashes_like_the_same_fake_tree` — `tree_hash` of a snapshot equals `tree_hash` of a
  `FakeFileTree` holding the same files (the snapshot behaves like the fake, `docs/architecture.md`
  "Testing").

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): serve a git rev's files from memory`.

### Task 4 — Implement the `Git` port on the git command line

**Files:**
- Create `crates/agentstacks/src/process/git/cli.rs` — `GitCli`.
- Modify `crates/agentstacks/src/process/git/mod.rs`, `crates/agentstacks/src/process/mod.rs` (`pub use`).

**Change:** `GitCli::new(program, home: Option<OsString>)` and `GitCli::from_env()` (program `git`,
`std::env::var_os("HOME")`). The home is turned into plan 10's `Cache` only by a call that needs the
cache, so listing tags and path-only projects never need `HOME`. When `HOME` is unset or empty (R10),
such a call returns `HomeUnset` (displayed `HOME is not set, so the cache (~/.airsstack/cache) has no
home`); the flows report it once (R10). Temp names are plan 07's `temp_name(pid, n)`
(`tmp-<pid>-<n>`), built from `std::process::id()` and a process-wide atomic counter (§9.3 "`<n>`
counts up within the process"); a leftover temp of the same name is removed and reused by `Cache`
(R21). Every command goes through Task 1.

- `list_tags(url, prefix)`: `git ls-remote --tags --refs -- <url> refs/tags/<prefix>*` (§9.2 row 1);
  stdout is parsed with plan 07's `parse_ls_remote` into `RemoteTag`s, in git's order (G1), which
  are returned as they are (plan 09). Empty output with exit 0 is an empty list. A line that does not
  parse is R2: plan 07's unlocated `parse_ls_remote` returns that `FetchError` itself, and it is
  passed on unchanged.
- `fetch_tag(url, tag)`: `Cache::temp`; `git init --bare -q <tmp>`; `git -C <tmp> fetch --no-tags
  --depth 1 -- <url> refs/tags/<tag>:refs/tags/<tag>`; `git -C <tmp> rev-parse --verify
  --end-of-options refs/tags/<tag>^{commit}` (annotated tags need `^{commit}`, G2); the stdout,
  lossy UTF-8 and trimmed, is the `CommitId` (output `CommitId::new` refuses is R2,
  plan 07's `unreadable` for the `rev-parse` step); `Cache::install(tmp, url, rev)` (§9.3 step 2, a race keeps the existing
  directory). On any failure after the temp exists, `Cache::discard` removes it, and the git failure
  is returned (only a killed process leaves temps, §9.3).
- `tree(url, rev, subdir, at)`: `Cache::repo(url, rev)`; when absent, `Cache::temp`, `git init
  --bare -q <tmp>`, `git -C <tmp> fetch --no-tags --depth 1 -- <url> <rev>` (§9.2 row 3, G3) and
  `Cache::install`. Then `git -C <repo> ls-tree -r -z --full-tree <rev> -- <normalised subdir>` (§9.2
  row 4; `SubPath::normalized`, §9.2 "`subdir` is normalised (§3.3) before use"); without a `subdir`
  the pathspec is omitted and the whole tree is listed (R26). The blob records' ids go to Task 2,
  and Task 3 builds the `Snapshot`.

A cache I/O failure comes back from plan 10's `Cache` as plan 02's R1 cache variant (the path
involved and the `io::Error`, shown as `<path>: <io::Error Display>`) and is returned unchanged.

**Proof (contract tests on the Task 1 repository via `file://`, a `tempfile` home; never the network):**
- `list_tags_lists_matching_tags_in_git_order` — prefix `packs-v` gives the `RemoteTag`s
  `packs-v0.1.0` (rev: the annotated tag object, as `ls-remote --refs` prints it, G1),
  `packs-v0.10.0`, `packs-v0.2.0`, and not `other-v9.9.9`.
- `list_tags_with_no_match_is_empty` — prefix `packs-v9` gives an empty list (G1, exit 0).
- `list_tags_needs_no_home` — a `GitCli` without a home still lists.
- `fetch_tag_peels_an_annotated_tag` — `packs-v0.1.0` gives commit `one`, not the tag object (G2);
  `<rev>.git` exists in the cache and no `tmp-*` entry is left.
- `fetch_tag_of_a_lightweight_tag` — `packs-v0.2.0` gives commit `two`.
- `fetch_tag_missing_passes_git_stderr` — `packs-v9.9.9` gives the §9.2 row 2 variant with
  subcommand `fetch` and stderr containing `refs/tags/packs-v9.9.9` (G7); no `tmp-*` is left.
- `fetch_by_rev_works_for_a_non_tip_commit` — `tree` at commit `one` with an empty cache fetches it
  (G3, protocol v2) and lists `packs/core`.
- `tree_reads_from_the_cache_without_the_remote` — after one `tree` call, the source repository is
  deleted; a second `tree` at the same rev still answers (§9.3 "resolves without network").
- `tree_keeps_crlf_symlink_and_gitlink_as_committed` — the CRLF file reads `a\r\nb\r\n`, `link` is
  `Symlink`, `sub` is `Other`, `run.sh` is `File` (§9.2, G4, G5).
- `tree_refuses_a_missing_subdir` — `packs/nope` gives the §9.2 row 4 variant.
- `tree_refuses_a_subdir_that_is_a_file` — `packs/core/pack.toml` gives the row 5 variant.
- `tree_without_a_subdir_lists_the_whole_tree` — no `subdir`: the snapshot holds `zero.txt` and
  `packs/core/pack.toml`, and its `root()` is `.` (R26, R22).
- `a_cached_rev_wins_a_rename_race` — a second `fetch_tag` of the same tag into a cache already
  holding `<rev>.git` returns the same rev, keeps the existing directory and leaves no `tmp-*`
  (§9.3 step 2, S1).
- `a_dash_host_never_reaches_ssh` — URL `ssh://-oProxyCommand=false/x` gives the row 2 variant with
  stderr containing `strange hostname` (G6): git refuses it before connecting.
- `git_not_found_is_reported_for_every_call` — `GitCli::new("/nonexistent/dir/git", home)` gives
  `GitNotFound` with an empty `needed_by` from `list_tags`, `fetch_tag` and `tree`.
- `no_home_is_refused_when_the_cache_is_needed` — `fetch_tag` with no home, and with an empty home,
  gives `HomeUnset` (§9.3, R10).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): git sources through the git command line`.

### Task 5 — Implement the `Binary` port on `PATH`

**Files:**
- Create `crates/agentstacks/src/process/binary.rs` — `SystemBinary`.
- Modify `crates/agentstacks/src/process/mod.rs` — `pub mod binary;` plus `pub use`.

**Change:** `SystemBinary::new(path: Option<OsString>)` and `SystemBinary::from_env()`
(`std::env::var_os("PATH")`), so tests never change the process environment.

- `on_path(name)` looks for `name` in each `PATH` directory (`split_paths`) (§10 rows 1-2). Found
  means R10: a regular file with an execute bit; empty `PATH` entries are skipped; an unset `PATH`
  finds nothing. A symlink counts when it resolves to an executable regular file (R28(c)), so the
  test uses `std::fs::metadata`, which follows the link.
- `airsl_version()` → `AirslAnswer` (plan 09, plan 06's type). It finds `airsl` the same way:
  - not found → `AirslAnswer::NotFound`;
  - found → runs that path with `--version` (§10 row 3, `airsl 0.1.4` in
    `probes/hookreq-results.md`). Its stdout is decoded as lossy UTF-8 and trimmed (R2). Exit 0 →
    `AirslAnswer::Printed(text)`. A non-zero exit → the `AirslAnswer` that plan 06's rule counts as
    unparseable, carrying the text, so the refusal is `agentstacks.toml: airsl --version printed
    "<text>", expected "airsl <version>"` (R2);
  - a found `airsl` whose spawn fails with any io error other than `NotFound` → plan 06's
    spawn-failure variant carrying the `io::Error`, shown by the flows as `agentstacks.toml: airsl:
    <io::Error Display>` (R1).

**Proof (temp `PATH` directories; only the spawn-failure test spawns a fixture file, and it closes the file first):**
- `a_file_with_an_execute_bit_is_on_path` — `rg` with mode 755 in the second of two directories.
- `a_file_without_an_execute_bit_is_not_on_path` — mode 644.
- `a_directory_named_like_the_binary_is_not_on_path`.
- `no_path_finds_nothing` — `SystemBinary::new(None)`.
- `a_symlink_to_an_executable_is_on_path` — `rg` as a link to a 755 file in another temp dir is
  found; a link to a 644 file is not (R28(c)).
- `empty_path_entries_are_skipped` — `PATH` `:<dir>` finds `rg` in `<dir>` and never looks in the
  current directory (R10).
- `airsl_absent_is_not_found` — a `PATH` without `airsl` gives `AirslAnswer::NotFound`.
- `a_found_airsl_that_will_not_run_is_a_spawn_failure` — a `PATH` directory holding an executable,
  empty file named `airsl` (no interpreter line, so `exec` refuses it) gives the spawn-failure
  variant, never `NotFound` (R1). The file is written and closed before the test spawns it.
- `real_airsl_prints_a_parsable_version` — `SystemBinary::from_env()`; when `airsl` is not on
  `PATH` the test prints `skipped: airsl is not on PATH` and returns (spec §14 `process::Binary`);
  otherwise the answer is `Printed(text)` and plan 06's `parse_airsl_version` accepts `text`.

**Done when:** `cargo make dod` passes (end of plan) and the Definition of Done is green.
Commit `feat(agentstacks): binary lookup on PATH and the airsl version`.

---

## Risks

- **Seams with plans 02, 06 and 07.** Plan 02 needs `FetchError` variants for:
  - the R1 spawn failure (`git: <io::Error Display>`, key location);
  - the R1 cache failure (`<path>: <io::Error Display>`, located at the path);
  - the R2 unreadable output (`git <subcommand> printed output agentstacks cannot read: "<first
    line>"`, key location).

  `GitNotFound { needed_by }` must be buildable with an empty list. Plan 06's `AirslAnswer` needs
  its spawn-failure variant (R1) and a value for a non-zero exit that its parse counts as
  unparseable (R2). Plan 07's `parse_ls_remote`, `parse_ls_tree`, `check_subdir` and `unreadable`
  are used in their unlocated forms, which return `FetchError`; their located `_at` wrappers are the
  flows'.
- **Git version.** Fetching a non-tip commit by id needs protocol v2 on `file://` (G3; protocol v0
  refuses `unadvertised object`); git has defaulted to v2 since 2.26, and the probe ran 2.39.3. The
  contract test `fetch_by_rev_works_for_a_non_tip_commit` fails loudly on an older git.
- **Pathspec matching.** The plan relies on git's pathspec matching whole components (`packs/core`
  never matches `packs/core-x/…`); G4 had no such sibling. `Snapshot` keeps only records under
  `<subdir>/`, so a looser match could not leak files into the tree.
- **Riskiest task: Task 4.** It is the only one that touches the cache and the network-facing
  commands; every contract test runs on `file://` only, and the cache tests use a `tempfile` home.
- Option not taken: `git archive` or a checkout to read files. Reading blobs with `cat-file --batch`
  is the proven path that keeps bytes unconverted (G5); a checkout would apply `.gitattributes`
  conversion (G5 clone comparison).

## Proof

- `cargo make dod` is green at the end of the plan.
- `grep -rn "std::fs" crates/agentstacks/src/process` finds only test code under `#[cfg(test)]`
  (test repositories and `PATH` fixtures), never adapter code.
- No test reaches the network: every URL in `src/process` tests is `file://`, `ssh://-o…` (refused by
  git before connecting, G6) or a nonexistent program.
- Spec rows covered: §2 dependency rule with X3 (only `fs::cache` from `fs`); §16 R1 (spawn and
  cache failures, Tasks 1, 4, 5), R2 (unreadable output and decoding, Tasks 1-5), R9 (subdir quoted
  as written, Task 3), R10 with R28(c) (HOME, "on PATH" incl. symlinks, git modes, Tasks 3-5), R28(b) (cache rename
  reported at `<rev>.git`, through `Cache`), R28(j) (lossy decode for quoted text and `ls-remote`
  lines; `ls-tree -z` paths stay raw bytes, Tasks 1, 3, 4), R21 (leftover cache temp reused,
  Task 4 through `Cache`), R22 (root label, Task 3), R26 (whole tree without a subdir, Task 4); §9.2 rules (`GIT_TERMINAL_PROMPT=0`, `-c core.autocrlf=false`, URL after `--`,
  `subdir` normalised: Tasks 1, 4); §9.2 step table rows 1-5 (Tasks 2, 4); tree bullets (Task 3);
  failure table rows 1, 2, 3 (Tasks 1, 4), 4, 5 (Task 3, 4), 6 (the flow's, plan 13, using these
  outcomes); §9.3 layout use, filling steps 1-2, race, HOME, temps removed after a live failure
  (Task 4, with plan 10's `Cache`); §10 `PATH` lookup rows 1-2 and the airsl answer row 3 (Task 5);
  §8 rows 6-8 input (Task 5); §14 `process::GitCli` (Task 4 contract tests, local bare repository)
  and `process::Binary` (Task 5, skipped with a printed reason when absent).

## Review findings

Reviewer, one round: code 0🔴 5🟡 8🔵 (one ❓), plan 1🟡 3🔵. Reviewer `cargo test -p agentstacks --lib process::`: 56 passed. Blocking: `git/command.rs:99`.

- fixed 🟡 (blocking, security) `src/process/git/command.rs:99`: git commands kept `GIT_DIR` and the other variables git uses to find a repository. Under a git hook, the fetch wrote tags into the user's repository, and the cache would have installed an empty scratch repo.
  - Fix: `command()` calls `env_remove` for each name in `REPOSITORY_VARIABLES` (GIT_DIR, GIT_WORK_TREE, GIT_INDEX_FILE, GIT_OBJECT_DIRECTORY, GIT_ALTERNATE_OBJECT_DIRECTORIES, GIT_COMMON_DIR, GIT_NAMESPACE).
  - Test `run_clears_the_variables_that_point_git_at_a_repository`: red `GIT_DIR is not removed`, then green.
- fixed 🟡 `src/process/binary.rs:115`: an airsl found on PATH whose interpreter is missing (`#!/nonexistent/interp`) was reported as not on PATH.
  - Fix: after `find` succeeds, every spawn error is `SpawnFailed`.
  - Test `a_found_airsl_with_a_missing_interpreter_is_a_spawn_failure_not_absent`: red (panicked on `NotFound`), then green.
- fixed 🟡 `git/command.rs:268`: the stdin test would still pass without the writer thread.
  - Fix: `STDIN_IDS = 20_000`. The reviewer's probe deadlocked at 5000 ids or more with the writer thread removed.
- fixed 🟡 `src/fs/mod.rs:4` and `src/process/mod.rs:10` no longer claim something false about `std::fs`. `process` reads file metadata only for the PATH lookup.
- fixed 🔵 Test names with spec ids were renamed: `..._is_a_git_spawn_refusal` and `an_unreadable_listing_is_a_failure`.
- Fix verification: `cargo test -p agentstacks --lib process::`: 58 passed, 0 failed. `rustfmt --check`: clean.
- open 🔵 Nits:
  - `command.rs:29`: `subcommand: &'static str` repeats words `GitStep` owns.
  - `blobs.rs:154`: the autocrlf test cannot tell its two layers apart.
  - `cli.rs:88`: the closure parameter `name` should be `resolve`.
  - `cli.rs:122`: `init` gets no `--` before the scratch path.
  - `cli.rs:160`: no test asserts `--end-of-options`.
  - `snapshot.rs:293`: the test name says "like the fake", but the fake's error kind differs.
- open 🔵 The plan's Proof line "`std::fs` only in test code" contradicts Task 5, which prescribes `std::fs::metadata`. The code follows Task 5.

## Deviations

- 2026-11-22: The Task 5 spawn-failure fixture is a file whose first line is `#!<temp dir>`, because on macOS (Darwin 25.3.0) an empty executable runs and prints `""`.
- 2026-11-22: `process/binary.rs` calls `std::fs::metadata` for the PATH lookup, as Task 5 prescribes. The module docs were reworded.
- 2026-11-22: `GitRun` models `-- <operands>` generally. The argv matches the plan.
- 2026-11-22: `list_tags_lists_matching_tags_in_git_order` asserts four tags, because the fixture's `packs-v0.9.0` also matches.
- 2026-11-22: Empty PATH entries are tested through the pure helper `search_dirs`. The test repo is built with git plumbing. `Snapshot::from_listing` parses raw ls-tree bytes.
- 2026-11-22: Git commands clear the repository-locating environment variables. The plan did not cover this; it was added from review.
