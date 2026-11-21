---
status: draft
created: 2026-11-22
depends-on: [09, 10]
built-on: ea104af7451e
---

# Process Implementation Plan

**Goal:** drive git and binaries for the flows.

**Architecture:** `src/process/` is the only `std::process` caller (spec §2). It holds two adapters:

- `GitCli` implements plan 09's `Git` on the `git` command line, with the exact §9.2 commands. Every
  disk step of the cache goes through plan 10's `Cache` (so `std::fs` stays in `fs`); parsing
  `ls-remote` lines and `ls-tree` records, mapping a mode to `EntryKind`, and the cache and temp
  names are plan 07's pure functions; failures are plan 02's `FetchError`. What stays here is
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
rows `process::GitCli` and `process::Binary`, D11.

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

- spawn fails with `io::ErrorKind::NotFound` → plan 02's §9.2 row 1 outcome ("git not on `PATH`";
  the io error does not name the program, G8), with no keys: the flow names them once (§9.2 row 1
  "once");
- exit status non-zero → plan 02's §9.2 row 2 variant carrying the subcommand word (`ls-remote`,
  `init`, `fetch`, `rev-parse`, `ls-tree`, `cat-file`) and git's stderr trimmed of surrounding
  whitespace, which displays after its location as `git <subcommand> failed: <git's stderr,
  trimmed>` (§9.2 row 2; D3 "tool errors are passed through");
- success → stdout bytes.

Any other spawn error has no message in the spec (gap GP1). Stderr bytes are decoded as UTF-8 with
replacement (gap GP2).

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
  the row 2 variant with subcommand `ls-remote` and stderr starting `fatal: '/nope' does not appear
  to be a git repository` with no trailing newline (G7).
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
`core.autocrlf=true`"). An answer `<oid> missing` (G5) or a malformed frame has no message in the
spec (gap GP3).

**Proof:**
- `parse_batch_reads_crlf_bytes_verbatim` — the G5 frame for `c30dea8…` gives `a\r\nb\r\n`.
- `parse_batch_reads_several_frames_in_order` — two frames (CRLF and LF blobs) give both contents in
  order, each without the trailing frame `LF`.
- `parse_batch_keeps_a_newline_inside_the_content` — a blob whose last content byte is `\n` keeps it.
- `read_blobs_returns_crlf_bytes_from_git` — on the test repository, with `core.autocrlf=true` set
  in that repository's config, the CRLF blob still comes back as `a\r\nb\r\n` (G5).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): read git blobs in one batch`.

### Task 3 — Serve a rev's files from memory as a `FileTree`

**Files:**
- Create `crates/agentstacks/src/process/git/snapshot.rs` — `Snapshot`.
- Modify `crates/agentstacks/src/process/git/mod.rs`, `crates/agentstacks/src/process/mod.rs` (`pub use`).

**Change:** `Snapshot` is a rev's tree read once and kept in memory (§9.2 "The source's tree is read
into memory once and served through the `FileTree` port"). It is built from plan 07's parsed
`ls-tree -r -z --full-tree` records (`<mode> SP <type> SP <oid> TAB <path> NUL`, byte-ordered, G4,
G9) and the blob bytes of Task 2. Building it first applies the two `subdir` rows of §9.2's failure
table (§13 lets `process` raise source refusals), using plan 02's variants:

- `subdir` given and the listing is empty (G4 `packs/nope`) → the variant displayed after its
  location as `"packs/nope" not found at <tag or rev>`, where `<tag or rev>` is the source's pin
  (`at`, plan 09);
- `subdir` given and the listing is exactly one blob record whose path equals `subdir` (G4
  `packs/core/pack.toml`) → the variant displayed as `"packs/core/pack.toml" is a file, not a
  directory`.

Which spelling of `subdir` the message shows (as written or normalised) is gap GP4. Each record's
path loses the `<normalised subdir>/` prefix (the source's root is the subdir), so `kind`,
`read_dir` and `read_file` take paths relative to it. Its `EntryKind` comes from plan 07's mode
mapping: `100644` / `100755` → `File`, `120000` → `Symlink` (so formats' "symlinks are not allowed"
refusal applies), `160000` → `Other` (§9.2 tree bullets). Every ancestor of a record is a `Dir`;
the empty path is the root. `read_dir` lists direct children; `read_file` returns a file's bytes,
refuses a symlink with `InvalidInput` displayed `symlinks are not followed` (the wording
`StdFileTree` uses), and answers other kinds as `FakeFileTree` does. A path not in the tree is
`NotFound`. On Unix a path's bytes become an `OsString` unchanged, so a non-UTF-8 name reaches
formats' own refusal. `root()` is the display root for messages; what it shows for a git source is
not specified (gap GP5), and the plan uses `<cache>/git/<hex>/<rev>.git[/<subdir>]` until the author
rules.

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
cache, so listing tags and path-only projects never need `HOME`; when it is missing such a call
returns plan 10's §9.3 refusal (displayed `HOME is not set, so the cache (~/.airsstack/cache) has no
home`). Temp names are plan 07's `tmp-<pid>-<n>`, built from `std::process::id()` and a process-wide
atomic counter (§9.3 "`<n>` counts up within the process"). Every command goes through Task 1.

- `list_tags(url, prefix)`: `git ls-remote --tags --refs -- <url> refs/tags/<prefix>*` (§9.2 row 1);
  stdout is parsed with plan 07's ls-remote line parser into tags, in git's order (G1). Empty output
  with exit 0 is an empty list.
- `fetch_tag(url, tag)`: `Cache::temp`; `git init --bare -q <tmp>`; `git -C <tmp> fetch --no-tags
  --depth 1 -- <url> refs/tags/<tag>:refs/tags/<tag>`; `git -C <tmp> rev-parse --verify
  --end-of-options refs/tags/<tag>^{commit}` (annotated tags need `^{commit}`, G2); the trimmed
  stdout is the `CommitId`; `Cache::install(tmp, url, rev)` (§9.3 step 2, a race keeps the existing
  directory). On any failure after the temp exists, `Cache::discard` removes it, and the git failure
  is returned (only a killed process leaves temps, §9.3).
- `tree(url, rev, subdir, at)`: `Cache::repo(url, rev)`; when absent, `Cache::temp`, `git init
  --bare -q <tmp>`, `git -C <tmp> fetch --no-tags --depth 1 -- <url> <rev>` (§9.2 row 3, G3) and
  `Cache::install`. Then `git -C <repo> ls-tree -r -z --full-tree <rev> -- <normalised subdir>` (§9.2
  row 4; `SubPath::normalized`, §9.2 "`subdir` is normalised (§3.3) before use"); without a `subdir`
  the pathspec is left out (gap GP6: §9.2 gives only the `-- <subdir>` form; G9 lists a whole tree
  without one). The blob records' ids go to Task 2, and Task 3 builds the `Snapshot`.

A cache I/O error (plan 10 `Cache`) has no message in the spec (gap GW1 of plan 10) and is handed on
as the platform's error in the form plan 02 gives it.

**Proof (contract tests on the Task 1 repository via `file://`, a `tempfile` home; never the network):**
- `list_tags_lists_matching_tags_in_git_order` — prefix `packs-v` gives `packs-v0.1.0`,
  `packs-v0.10.0`, `packs-v0.2.0`, and not `other-v9.9.9` (G1).
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
- `a_cached_rev_wins_a_rename_race` — a second `fetch_tag` of the same tag into a cache already
  holding `<rev>.git` returns the same rev, keeps the existing directory and leaves no `tmp-*`
  (§9.3 step 2, S1).
- `a_dash_host_never_reaches_ssh` — URL `ssh://-oProxyCommand=false/x` gives the row 2 variant with
  stderr containing `strange hostname` (G6): git refuses it before connecting.
- `git_not_found_is_reported_for_every_call` — `GitCli::new("/nonexistent/dir/git", home)` gives
  the §9.2 row 1 outcome from `list_tags`, `fetch_tag` and `tree`.
- `no_home_is_refused_when_the_cache_is_needed` — `fetch_tag` without a home gives the §9.3
  refusal.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): git sources through the git command line`.

### Task 5 — Implement the `Binary` port on `PATH`

**Files:**
- Create `crates/agentstacks/src/process/binary.rs` — `SystemBinary`.
- Modify `crates/agentstacks/src/process/mod.rs` — `pub mod binary;` plus `pub use`.

**Change:** `SystemBinary::new(path: Option<OsString>)` and `SystemBinary::from_env()`
(`std::env::var_os("PATH")`), so tests never change the process environment.

- `on_path(name)` looks for `name` in each `PATH` directory (`split_paths`) (§10 rows 1-2). What
  counts as found is gap GP7; the plan uses: a regular file (following links) with an execute bit,
  empty entries skipped, no `PATH` → not found.
- `airsl_version()` finds `airsl` the same way; not found → `AirslAnswer::NotFound`; found → runs
  that path with `--version` and returns `AirslAnswer::Printed(stdout)` (§10 row 3, `airsl 0.1.4`
  in `probes/hookreq-results.md`). Whether the line break is trimmed, how non-UTF-8 output and a
  non-zero exit are treated, and what a spawn failure of a found binary means are gap GP8 (shared
  with plan 06's G3); the plan trims trailing whitespace, decodes with replacement, ignores the
  exit status (the parse refuses anything that is not `airsl <semver>`), and answers `NotFound` when
  the found file cannot be run.

**Proof (temp `PATH` directories; files are created but never executed, so no test races a write):**
- `a_file_with_an_execute_bit_is_on_path` — `rg` with mode 755 in the second of two directories.
- `a_file_without_an_execute_bit_is_not_on_path` — mode 644.
- `a_directory_named_like_the_binary_is_not_on_path`.
- `no_path_finds_nothing` — `SystemBinary::new(None)`.
- `airsl_absent_is_not_found` — a `PATH` without `airsl` gives `AirslAnswer::NotFound`.
- `real_airsl_prints_a_parsable_version` — `SystemBinary::from_env()`; when `airsl` is not on
  `PATH` the test prints `skipped: airsl is not on PATH` and returns (spec §14 `process::Binary`);
  otherwise the answer is `Printed` and plan 06's `parse_airsl_version` accepts it.

**Done when:** `cargo make dod` passes (end of plan) and the Definition of Done is green.
Commit `feat(agentstacks): binary lookup on PATH and the airsl version`.

---

## Risks

- **Gaps held for the author (no answer invented above; each task names its gap):**
  - GP1: a spawn error other than `NotFound` (for example `PermissionDenied` on a non-executable
    `git`) has no message.
  - GP2: git's stderr decoding (§9.2 row 2 says only "trimmed").
  - GP3: `cat-file --batch` answering `<oid> missing`, a malformed frame, `rev-parse` output that is
    not a commit id, or an unparseable `ls-tree` record: no message.
  - GP4: the `subdir` spelling in the §9.2 rows 4-5 messages (written vs normalised); the spec's
    examples are equal either way.
  - GP5: the display root (`FileTree::root`) of a git source, which prefixes pack and extension
    load messages; neither this spec nor the formats spec defines it.
  - GP6: the `ls-tree` command without a `subdir` (§9.2 shows only `-- <subdir>`).
  - GP7: what "on `PATH`" means (execute bit, empty entries, unset `PATH`).
  - GP8: the `airsl --version` text: trimming, non-UTF-8, non-zero exit, a found file that cannot
    run.
- **`process` uses `fs`.** The plan-set map has `GitCli` call plan 10's `Cache`, an adapter-to-adapter
  edge that spec §2's arrow (`fs, process, host → port, types`) does not list. It keeps `std::fs` in
  `fs` and `std::process` in `process`, as §2's module rows require. Flagged for the orchestrator.
- **Plan 07 and plan 02 names.** The ls-remote line parser, the ls-tree record parser, the mode
  mapping, the cache and temp name rules (plan 07) and the `FetchError` variants (plan 02) are used
  by role; the coder takes their names from those plans. The git-missing variant must carry no keys
  (the flow adds them).
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
- Spec rows covered: §9.2 rules (`GIT_TERMINAL_PROMPT=0`, `-c core.autocrlf=false`, URL after `--`,
  `subdir` normalised: Tasks 1, 4); §9.2 step table rows 1-5 (Tasks 2, 4); tree bullets (Task 3);
  failure table rows 1, 2, 3 (Tasks 1, 4), 4, 5 (Task 3, 4), 6 (the flow's, plan 13, using these
  outcomes); §9.3 layout use, filling steps 1-2, race, HOME, temps removed after a live failure
  (Task 4, with plan 10's `Cache`); §10 `PATH` lookup rows 1-2 and the airsl answer row 3 (Task 5);
  §8 rows 6-8 input (Task 5); §14 `process::GitCli` (Task 4 contract tests, local bare repository)
  and `process::Binary` (Task 5, skipped with a printed reason when absent).
