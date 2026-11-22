---
status: done
created: 2026-11-22
depends-on: [02]
built-on: f39b2a814319
---

# Resolve Implementation Plan

**Goal:** reconcile the lock with agentstacks.toml.

**Architecture:** pure rules in `types/resolve` (spec §2 table row `types/resolve`, D13). Each rule takes
values the flows and adapters already hold and returns an answer; nothing here runs git, reads the
disk or reads the environment. Reconciliation (§9.1, §16 R9) classifies every tool and pack entry
from the `ProjectFile` and the optional `Lockfile`. The tree check refuses a git tree that differs
from the lock, removed tools included (§16 R18). The git output parsers (§9.2) turn `ls-remote` lines
and `ls-tree -z` records into values, refuse output they cannot read (§16 R2) and map tree modes to
`EntryKind` (§16 R10); the subdir checks decide "not found" and "is a file" from those records, and a
helper gives a git tree's root label (§16 R22). The cache layout (§9.3, §16 R10) is a set of path
functions over a home directory that the caller supplies. The tag rules (§9.4, §16 R8) split a tag
into prefix and version, choose the latest tag, and build one `UpdateEntry` per git source.
Refusals are `FetchError` variants (plan 02), wrapped in a `Diagnostic` at `agentstacks.toml` with
the entry's key path (§13 row "`agentstacks.toml` with key") or with no key (HOME, §13 row
"`agentstacks.toml`, no key"). Plan notes (§9.4) are `UpdateNote` values that display their exact
text. The §9.1 "source changed" note text is `Note::SourceChanged` in `types/plan` (plan 06), because
`Plan` holds it; this plan supplies the `Stale` classification it is made from. From `crate::port`
this module imports `EntryKind` only, which §16 X2 allows (`tests/isolation.rs` permits exactly
`FileTree` and `EntryKind` inside `src/types`).

**Tech Stack:** Rust 2024, `crates/agentstacks` only; existing types `ProjectFile`, `PackEntry`,
`Source`, `Pin`, `GitUrl`, `GitTag`, `CommitId`, `SubPath`, `LocalPath` (`src/types/project/*`),
`Lockfile`, `LockedEntry`, `LockedSource` (`src/types/lock/*`), `ContentHash`, `PackVersion`
(semver precedence, `src/types/manifest/pack/version.rs`), `EntryKind` (`crate::port`, after plan
01's move), `Diagnostic`, `Location`; no new dependency.

**Content authority:** spec §9.1, §9.2 (parsing and mapping rows, subdir failure rows), §9.3
(layout, temp name, HOME), §9.4, §16 X2, R2, R8, R9, R10, R18, R22, R26; probes
`probes/git-results.md` G1, G4 and G9.

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6): `mod-rs-export-only.md`
(`types/resolve/mod.rs` holds only docs, `mod` and `pub use`); `modularity.md` Rule 1 (one job per
file: reconcile, tree check, each parser, cache layout, tag rule, update entry) and Rule 2 (the
source key and update notes are defined once, here); `strong-types.md` (keys are `SourceKey`, revs
`CommitId`, tags `GitTag`, versions `PackVersion`, the git subcommand a `GitStep` enum, no `bool`
decision parameter); `unit-test-mandate.md` (colocated tests in every logic file);
`doc-comment-discipline.md` (doctests build plain values and never use a `#[cfg(test)]` fake, §16 R24
plan-level note); `static-dispatch.md` (no trait objects, no port).

---

## Files that change

```
crates/agentstacks/src/types/resolve/mod.rs        — modify: add mod / pub use lines (docs stay)
crates/agentstacks/src/types/resolve/key.rs        — create: SourceKey
crates/agentstacks/src/types/resolve/reconcile.rs  — create: Reconciled, same-source rule, reconcile (§9.1, R9)
crates/agentstacks/src/types/resolve/check.rs      — create: the git tree check (§9.1 row 2, R18)
crates/agentstacks/src/types/resolve/unread.rs     — create: GitStep, the unreadable-output refusal, unlocated and located (R2)
crates/agentstacks/src/types/resolve/remote.rs     — create: RemoteTag, the ls-remote parser (§9.2 G1)
crates/agentstacks/src/types/resolve/listing.rs    — create: TreeRecord, ls-tree parser, mode mapping, subdir checks, root label (§9.2 G4, R10, R22, R26)
crates/agentstacks/src/types/resolve/cache.rs      — create: cache layout, temp name, HOME (§9.3, R10)
crates/agentstacks/src/types/resolve/tag.rs        — create: TagVersion split, latest_tag (§9.4, R8)
crates/agentstacks/src/types/resolve/update.rs     — create: UpdateQuery, UpdateEntry, UpdateNote (§9.4)
```

`src/types/resolve/error.rs` (`FetchError`) belongs to plan 02 and is not touched here.

## Order of work

### Task 1 — Name a source entry by its key

**Files:**
- Create `crates/agentstacks/src/types/resolve/key.rs` — `SourceKey`.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod key;` plus `pub use`.

**Change:** `SourceKey` is `Pack(PackName)` or `Tool(ToolName)`, displayed `packs.<name>` or
`tools.<name>` (the key paths §9.2 shows: `packs.core`, `tools.claude`). `SourceKey::location()` is
`Location::at_key("agentstacks.toml", <key>)`; `SourceKey::subdir_location()` is the same file at
`<key>.subdir` (§9.2 subdir rows: `packs.core.subdir`). It orders packs before tools and by name
within each, so a map keyed by it follows the order §16 R8 sets for update entries (`packs.*`, then
`tools.*`).

**Proof:**
- `source_key_displays_its_key_path` — `packs.core`, `tools.claude`.
- `source_key_locations` — `location()` displays as the prefix `agentstacks.toml: packs.core`;
  `subdir_location()` as `agentstacks.toml: packs.core.subdir`.
- `source_key_orders_packs_before_tools` — `tools.a` sorts after `packs.z`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): source entry key`.

### Task 2 — Reconcile every entry with the lock

**Files:**
- Create `crates/agentstacks/src/types/resolve/reconcile.rs` — `Reconciled`, the same-source rule
  and `reconcile`.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod reconcile;` plus `pub use`.

**Change:** `same_source(source: &Source, locked: &LockedSource) -> bool` compares the source fields
§9.1 names (`git`, `tag`, `rev`, `subdir` / `path`) as normalised values (§16 R9: `./packs/core` =
`packs/core`): `subdir` through `SubPath::normalized`, `path` with the same rule (empty and `.`
segments removed, formats §8; `..` segments are kept as written), `url`, `tag` and `rev` as their
typed values. Git against git compares `url`, normalised `subdir` (none equals none), and the pin:
`Pin::Tag(t)` matches a lock whose `tag` is `Some(t)` (its `rev` is then the lock's, used as is);
`Pin::Rev(r)` matches a lock whose `rev` is `r` and whose `tag` is `None` (formats §8: an extra `tag`
makes the lock stale). Path against path compares the normalised `LocalPath`; git against path is
different. Messages elsewhere keep quoting values as written (§16 R9).

`Reconciled` is (§9.1 table):

| Variant | Row |
|---|---|
| `New` | entry in toml, none in the lock: resolve (git: tag → rev; path: read) |
| `Locked { rev: Option<CommitId>, tree: ContentHash }` | same source fields: use the lock's `rev` (`None` for a path source) and check the tree |
| `Stale` | different source fields: resolve as new; the flow adds the plan note |
| `RemovedPack` | lock pack entry, none in toml: its files get delete steps (§6.2) |
| `RemovedTool { source: LockedSource, tree: ContentHash }` | lock tool entry, none in toml: §6.4; its locked source is fetched and its tree checked like any locked entry (§16 R18) |

`reconcile(project: &ProjectFile, lock: Option<&Lockfile>) -> BTreeMap<SourceKey, Reconciled>`
classifies every toml tool and pack and every lock tool and pack once. A missing lock gives `New` for
every toml entry. A moved remote tag cannot change the answer: a tag-pinned entry whose tag equals
the lock's gets `Locked` with the lock's `rev`, and nothing here touches the network (§9.1 last
paragraph).

**Proof:**
- `same_source_tag_pin_matches_the_locked_tag` — toml `tag = "packs-v0.1.0"`, lock tag
  `packs-v0.1.0` with any rev → `true`; lock tag `packs-v0.2.0` → `false`; lock without tag →
  `false`.
- `same_source_rev_pin_needs_no_locked_tag` — toml `rev = r`, lock `rev = r`, no tag → `true`;
  lock with a tag → `false`; lock `rev ≠ r` → `false`.
- `same_source_compares_normalised_subdir_and_path` — `subdir = "./packs/core"` against a lock
  `packs/core` → `true`; `path = "packs//team-lint/."` against `packs/team-lint` → `true`.
- `same_source_differs_on_url_subdir_path_or_kind` — a different url, a different normalised subdir,
  subdir against none, a different path, or git against path → `false`.
- `reconcile_new_entry` — toml entry, no lock entry → `New`; a missing lock → every entry `New`.
- `reconcile_locked_entry_uses_the_lock_rev` — same fields → `Locked` with the lock's rev and tree.
- `reconcile_path_entry_locked_without_rev` — same path → `Locked { rev: None, .. }`.
- `reconcile_changed_source_is_stale` — different fields → `Stale`.
- `reconcile_removed_pack` — lock pack, no toml pack → `RemovedPack`.
- `reconcile_removed_tool_keeps_its_locked_source` — lock tool, no toml tool → `RemovedTool` with
  the lock's source and tree.
- `moved_tag_keeps_the_lock_rev` — the toml tag equals the lock's tag while the remote tag now names
  another commit (only the lock is input) → `Locked` with the lock's rev.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): reconcile the lock with the project file`.

### Task 3 — Refuse a git tree that differs from the lock

**Files:**
- Create `crates/agentstacks/src/types/resolve/check.rs` — the tree check.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod check;` plus `pub use`.

**Change:** `check_tree(key: &SourceKey, locked: &LockedSource, fetched: &ContentHash, lock_tree:
&ContentHash) -> Result<(), Diagnostic>`: for a git source, `fetched ≠ lock_tree` is refused at
`key.location()` with the `FetchError` tree-mismatch variant (plan 02), which reads
`agentstacks.toml: packs.core: tree sha256:<a> at rev <rev> differs from the lock's sha256:<b>`
(§9.1 row 2; `<a>` the fetched tree, `<b>` the lock's, `<rev>` the locked commit). It applies to a
`Locked` git entry and to a `RemovedTool` git entry alike (§16 R18). For a path source it always
passes: "a changed path tree is a normal change, shown in the plan" (§9.1 row 2; formats §8.2).

**Proof:**
- `git_tree_mismatch_is_refused` — the whole diagnostic equals the §9.1 text with the test's hashes
  and rev filled in.
- `removed_tool_tree_mismatch_is_refused` — key `tools.claude` → the same text at
  `agentstacks.toml: tools.claude: `.
- `git_tree_equal_passes` — equal hashes → `Ok`.
- `path_tree_change_is_not_refused` — a path source with different hashes → `Ok`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): refuse a git tree that differs from the lock`.

### Task 4 — Refuse git output that cannot be read

**Files:**
- Create `crates/agentstacks/src/types/resolve/unread.rs` — `GitStep` and the refusal builder.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod unread;` plus `pub use`.

**Change:** `GitStep` names the git subcommand whose output is read: `LsRemote`, `LsTree`,
`RevParse`, `CatFile`, displayed `ls-remote`, `ls-tree`, `rev-parse`, `cat-file` (§16 R2 lists these
outputs). `first_line(output: &[u8]) -> String` decodes the output as lossy UTF-8, trims surrounding
whitespace and keeps the text before the first `\n` (§16 R2). `unreadable(step: GitStep, output:
&[u8]) -> FetchError` is the **unlocated** `FetchError::GitUnreadable` (plan 02, §16 R2), displayed
after its location as `git <subcommand> printed output agentstacks cannot read: "<first line>"`.
`unreadable_at(key: &SourceKey, step, output) -> Diagnostic` is the located form at `key.location()`:
`agentstacks.toml: packs.core: git <subcommand> printed output agentstacks cannot read: "<first line>"`.
Plan 11 calls the unlocated `unreadable` for `rev-parse` and `cat-file` and owns no copy of its own;
the flows (plans 13, 16) locate what comes back through the `Git` port with `SourceKey::location()`.
Tasks 5 and 6 use it for the two parsers.

**Proof:**
- `unreadable_quotes_the_first_line` — `LsRemote`, output `b"  junk\nmore\n"`: `unreadable` displays
  `git ls-remote printed output agentstacks cannot read: "junk"`, and `unreadable_at` with key
  `packs.core` displays
  `agentstacks.toml: packs.core: git ls-remote printed output agentstacks cannot read: "junk"`.
- `unreadable_decodes_lossy_utf8` — output `b"a\xffb"` quotes `a` U+FFFD `b`.
- `git_step_words` — each variant displays its subcommand.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): refuse unreadable git output`.

### Task 5 — Parse `ls-remote` tag lines

**Files:**
- Create `crates/agentstacks/src/types/resolve/remote.rs` — `RemoteTag` and its parser.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod remote;` plus `pub use`.

**Change:** `RemoteTag { rev: CommitId, tag: GitTag }`, one parsed `ls-remote` line. It is the item
type `Git::list_tags` returns (plan 09), so the port, `latest_tag` and `UpdateQuery::finish` share
one type. `parse_ls_remote(stdout: &[u8]) -> Result<Vec<RemoteTag>, FetchError>` decodes the output as lossy UTF-8 and trims it (§16
R2), splits it on `\n`, and reads each line as `<hex>\t<ref>` (§9.2 "list tags", G1): the hex part
through `CommitId::new`, the ref must start with `refs/tags/` and the rest goes through `GitTag::new`.
Empty output (exit 0, nothing matched, G1 `packs-v9*`) is an empty list. Any line of another shape
refuses the whole output with the unlocated `unreadable(GitStep::LsRemote, stdout)` (§16 R2).
`parse_ls_remote_at(key: &SourceKey, stdout) -> Result<Vec<RemoteTag>, Diagnostic>` is the located
form at `key.location()`. Plan 11 calls the unlocated form.

**Proof:** the inputs are G1's stdout bytes copied byte for byte.
- `ls_remote_lines_parse` — the four G1 lines (`aec92b45…\trefs/tags/packs-v0.1.0`,
  `90e2a535…\trefs/tags/packs-v0.10.0`, `…packs-v0.2.0`, `…packs-v0.9.0`) give four `RemoteTag`s in
  output order with those revs and tags.
- `ls_remote_empty_output_is_empty` — `b""` → no entries.
- `ls_remote_non_tag_ref_is_unreadable` — `90e2a535…\trefs/heads/packs-v1` → the unlocated
  `GitUnreadable` error, and through `parse_ls_remote_at` with key `packs.core`
  `agentstacks.toml: packs.core: git ls-remote printed output agentstacks cannot read: "90e2a535…\trefs/heads/packs-v1"`
  (the full 40-hex line, as the test builds it).
- `ls_remote_malformed_line_is_unreadable` — a line without a tab, and one with a 39-hex id, each
  give the R2 refusal quoting the output's first line.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse ls-remote tag lines`.

### Task 6 — Parse `ls-tree -z` records and check the subdir

**Files:**
- Create `crates/agentstacks/src/types/resolve/listing.rs` — `TreeRecord`, the parser, the mode
  mapping, the subdir checks and the root label.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod listing;` plus `pub use`.

**Change:** `TreeRecord { mode: String, object: String, oid: String, path: Vec<u8> }` with
`TreeRecord::kind() -> EntryKind` (§16 R10): `100644` and `100755` → `EntryKind::File`, `120000` →
`EntryKind::Symlink` (formats' "symlinks are not allowed" then applies, §9.2), any other mode
(`160000` gitlink and `040000` included) → `EntryKind::Other`.

`parse_ls_tree(stdout: &[u8]) -> Result<Vec<TreeRecord>, FetchError>` splits the
raw bytes on NUL and reads each record as `<mode> SP <type> SP <oid> TAB <path>` (§9.2 "list files",
G4). The path stays raw bytes, because `-z` prints it unquoted (G4 follow-up: `ü` arrives as its
UTF-8 bytes) and the UTF-8 refusal belongs to the formats walk; lossy decoding is used only for the
quoted text of a refusal. Records keep git's byte order (G9). Empty output is an empty list. A
record of another shape refuses with the unlocated `unreadable(GitStep::LsTree, stdout)` (§16 R2).
`parse_ls_tree_at(key: &SourceKey, stdout) -> Result<Vec<TreeRecord>, Diagnostic>` is the located form
at `key.location()`. Plan 11 calls the unlocated form.

`TreeRecord::relative(&self, subdir: Option<&SubPath>) -> Option<Vec<u8>>` strips
`<normalised subdir>/` from the path (§9.2: "`subdir` is normalised (§3.3) before use"). Without a
subdir the listing is of the whole tree, because `ls-tree` then runs with no pathspec (§16 R26; plan
11 builds the command), and the whole path is returned. `root_label(subdir: Option<&SubPath>) ->
PathBuf` is the `subdir` as written, or `.` without one: the label a git source's `FileTree::root()`
reports and its load messages start with (§16 R22).

`check_subdir(records: &[TreeRecord], subdir: &SubPath, at: &Pin) -> Result<(), FetchError>` (§9.2
failure rows) returns the **unlocated** `FetchError`, because `Git::tree` (plan 09) knows no entry
key; `<tag or rev>` is the pin's `GitTag` or `CommitId` display (`Pin` itself has no `Display`), and
the subdir is quoted as written (§16 R9):
- no records → the `FetchError` not-found variant, displayed after its location as
  `"packs/nope" not found at <tag or rev>`;
- exactly one blob record whose path equals the normalised subdir → the is-a-file variant,
  `"packs/core/pack.toml" is a file, not a directory`;
- otherwise `Ok`. It is called only for a source with a `subdir`.

`check_subdir_at(key: &SourceKey, records, subdir, at) -> Result<(), Diagnostic>` is the located form:
the same error wrapped at `key.subdir_location()`, so it reads
`agentstacks.toml: packs.core.subdir: "packs/nope" not found at <tag or rev>` and
`agentstacks.toml: packs.core.subdir: "packs/core/pack.toml" is a file, not a directory`. Plans 13 and
16 locate an error that came back through `Git::tree` the same way, with `SourceKey::subdir_location()`.

**Proof:** the inputs are G4's records with `\0` separators, copied byte for byte.
- `ls_tree_records_parse` — the G4 listing of `packs/core` parses into fifteen records with the
  modes, types, oids and paths shown, in order.
- `ls_tree_mode_mapping` — `100644 blob` and `100755 blob` → `File`; `120000 blob …\tpacks/core/link`
  → `Symlink`; `160000 commit 0123…\tpacks/core/sub` → `Other`; `040000 tree 3519f5a9…\tpacks/core`
  (G4's non-recursive line) → `Other`.
- `ls_tree_path_keeps_raw_bytes` — the `packs/core/ü.md` record's path is `packs/core/` followed by
  `0xC3 0xBC` and `.md`; `packs/core/sp ace.md` keeps its space.
- `ls_tree_malformed_record_is_unreadable` — a record without a tab → the unlocated
  `GitUnreadable` error, and through `parse_ls_tree_at` with key `packs.core`
  `agentstacks.toml: packs.core: git ls-tree printed output agentstacks cannot read: "<first line>"`
  with the record's text.
- `ls_tree_empty_output_is_empty` — `b""` → an empty list.
- `record_relative_strips_the_subdir` — `packs/core/nested/deeper/file.md` with subdir
  `./packs//core` → `nested/deeper/file.md`; no subdir → the whole path.
- `root_label_is_the_subdir_as_written_or_dot` — `./packs//core` → `./packs//core`; none → `.`.
- `subdir_listing_nothing_is_refused` — no records, subdir `packs/nope`, pin tag `packs-v0.1.0`:
  `check_subdir` gives the not-found `FetchError` displaying `"packs/nope" not found at packs-v0.1.0`,
  and `check_subdir_at` with key `packs.core` displays
  `agentstacks.toml: packs.core.subdir: "packs/nope" not found at packs-v0.1.0`.
- `subdir_naming_a_file_is_refused` — one blob record `packs/core/pack.toml`, subdir
  `packs/core/pack.toml`: the unlocated error displays
  `"packs/core/pack.toml" is a file, not a directory`, the located one
  `agentstacks.toml: packs.core.subdir: "packs/core/pack.toml" is a file, not a directory`.
- `subdir_directory_passes` — the G4 listing with subdir `packs/core` → `Ok` in both forms.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse ls-tree records`.

### Task 7 — Lay out the git cache

**Files:**
- Create `crates/agentstacks/src/types/resolve/cache.rs` — the cache path rules and HOME.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod cache;` plus `pub use`.

**Change:** (§9.3) `cache_home(home: Option<&OsStr>) -> Result<PathBuf, FetchError>` returns the
given `$HOME` value, or, when it is unset or empty (§16 R10), the **unlocated**
`FetchError::HomeUnset` (plan 02). The caller locates it at `Location::file("agentstacks.toml")`
(§13 row "`agentstacks.toml`, no key"), so it reads
`agentstacks.toml: HOME is not set, so the cache (~/.airsstack/cache) has no home`. The value is passed
in by the caller (plan 10), which reports it once (§16 R10); reading the environment is not done here.
`url_dir(home: &Path, url: &GitUrl) -> PathBuf` is `<home>/.airsstack/cache/git/<hex sha256 of url>`,
the hex from `ContentHash::of_bytes(url.as_str().as_bytes()).hex()`. `rev_dir(url_dir: &Path, rev:
&CommitId) -> PathBuf` is `<url dir>/<rev>.git`. `temp_name(pid: u32, n: u64) -> String` is
`tmp-<pid>-<n>`; the caller supplies its process id and counter, so this module never calls
`std::process`.

**Proof:**
- `cache_home_refuses_a_missing_home` — `None` → `FetchError::HomeUnset`; wrapped in a `Diagnostic`
  at `Location::file("agentstacks.toml")` it displays
  `agentstacks.toml: HOME is not set, so the cache (~/.airsstack/cache) has no home`.
- `cache_home_refuses_an_empty_home` — `Some("")` → `FetchError::HomeUnset`, the same text.
- `cache_home_takes_the_given_home` — `Some("/home/u")` → `/home/u`.
- `url_dir_is_the_sha256_of_the_url` — for `https://github.com/airsstack/agentstacks` the last
  segment equals `ContentHash::of_bytes` of the URL's bytes as hex, under
  `/home/u/.airsstack/cache/git/`.
- `rev_dir_appends_rev_dot_git` — `<url dir>/<40 hex>.git`.
- `temp_name_counts_up` — `temp_name(42, 0)` is `tmp-42-0`, `temp_name(42, 1)` is `tmp-42-1`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): git cache layout`.

### Task 8 — Split a tag and choose the latest

**Files:**
- Create `crates/agentstacks/src/types/resolve/tag.rs` — `TagVersion`, `latest_tag`.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod tag;` plus `pub use`.

**Change:** `TagVersion { prefix: String, version: PackVersion }`. `TagVersion::split(tag: &GitTag)
-> Option<TagVersion>`: the version starts at the leftmost ASCII digit whose remainder parses as
semver (`PackVersion::new`); the prefix is everything before it (§9.4). `latest_tag(tags:
&[RemoteTag], prefix: &str) -> Option<(GitTag, PackVersion)>`: keep the tags that start with
`prefix` and whose suffix after it parses as semver and has no pre-release; return the highest by
semver precedence, not the lexical order `ls-remote` gives (§9.4 steps 2-3, G1); `None` when none
remains. Precedence ignores build metadata; among tags of equal precedence the greater tag in byte
order wins (§16 R8). The comparison is written as (precedence, tag bytes), so it does not depend on
how `PackVersion`'s derived ordering treats build metadata.

**Proof:**
- `split_packs_v` — `packs-v0.1.0` → prefix `packs-v`, version `0.1.0`.
- `split_takes_the_leftmost_digit_that_parses` — `x-1.2.3-4.5.6` → prefix `x-`, version
  `1.2.3-4.5.6`.
- `split_without_a_version_is_none` — `main-latest` → `None`.
- `latest_is_by_semver_not_lexical` — the four G1 tags with prefix `packs-v` → `packs-v0.10.0`.
- `latest_skips_pre_releases` — `packs-v0.3.0-rc.1` beside `packs-v0.2.0` → `packs-v0.2.0`.
- `latest_skips_other_prefixes_and_non_semver` — `other-v9.9.9` and `packs-vnext` are ignored.
- `latest_breaks_build_metadata_ties_by_tag_bytes` — `packs-v1.0.0+a` and `packs-v1.0.0+b` →
  `packs-v1.0.0+b`, whichever order they are listed in.
- `latest_of_nothing_is_none` — empty list, or only pre-releases → `None`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): tag split and latest tag`.

### Task 9 — Build one update entry per git source

**Files:**
- Create `crates/agentstacks/src/types/resolve/update.rs` — `UpdateQuery`, `UpdateEntry`,
  `UpdateOutcome`, `UpdateNote`.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod update;` plus `pub use`.

**Change:** `UpdateNote` displays the §9.4 note texts verbatim, with the entry's key:

| Variant | Display (§9.4) |
|---|---|
| `NoVersion { key, tag }` | `agentstacks.toml: packs.core: tag "main-latest" has no version to update from` |
| `NoNewer { key, tag }` | `agentstacks.toml: packs.core: no newer tag than packs-v0.1.0` |
| `PinnedRev { key, rev }` | `agentstacks.toml: packs.core: pinned to rev <rev>, not updated` |

`UpdateOutcome` is `NewTag(GitTag)` or `Note(UpdateNote)`. `UpdateEntry { key: SourceKey, current:
Option<GitTag>, outcome: UpdateOutcome }` is what `app/update` returns per git source (§9.4 last
paragraph); entries follow `SourceKey` order (§16 R8). `UpdateQuery::for_source(key, source:
&Source) -> Option<Result<UpdateQuery, UpdateEntry>>`:
- a `path` source → `None` (nothing, §9.4 last row);
- a git source pinned by `rev` → `Err(entry)` with `PinnedRev` and `current: None`;
- a git tag that has no split → `Err(entry)` with `NoVersion`;
- otherwise `Ok(UpdateQuery { key, url, prefix, current tag, current version })`, for the flow to
  list `refs/tags/<prefix>*` (plan 16).

`UpdateQuery::finish(self, tags: &[RemoteTag]) -> UpdateEntry`: `latest_tag` with the prefix; a
result newer (by semver precedence) than the current version → `NewTag`; `None` or nothing newer →
`NoNewer` with the current tag. Writing the new tag into `agentstacks.toml`, shorthand expansion and
its refusal are `cli-commands`' (§9.4, §17), not built here.

**Proof:**
- `update_note_texts_match_the_spec` — each variant at key `packs.core` displays its §9.4 text
  (`main-latest`, `packs-v0.1.0`, a 40-hex rev).
- `update_skips_a_path_source` — `None`.
- `update_notes_a_rev_pin` — `PinnedRev`, `current` `None`.
- `update_notes_a_tag_without_version` — `main-latest` → `NoVersion`.
- `update_finds_a_newer_tag` — current `packs-v0.1.0`, G1 tags → `NewTag(packs-v0.10.0)` with
  current `packs-v0.1.0`.
- `update_notes_nothing_newer` — current `packs-v0.10.0`, G1 tags → `NoNewer`; no tags at all →
  `NoNewer`.
- `update_entry_for_a_tool_uses_its_key` — a tool source's note starts with
  `agentstacks.toml: tools.claude: `.

**Done when:** `cargo make dod` passes (end of plan) and the Definition of Done is green. Commit
`feat(agentstacks): update entries for git sources`.

---

## Risks

- `EntryKind` lives in `crate::port` after plan 01; `types/resolve` importing it is one of the two
  names §16 X2 allows. Importing anything else from `crate::port` here would fail
  `tests/isolation.rs`.
- §16 R2 says output "is decoded as lossy UTF-8 and trimmed"; the author confirmed it covers the quoted
  text and `ls-remote` lines only. This plan applies that to `ls-remote`
  text and to every quoted `<first line>`, but parses `ls-tree -z` paths from the raw bytes: decoding
  a path lossily would turn a non-UTF-8 name into U+FFFD and hide the formats walk's
  `path is not valid UTF-8` refusal, and trimming could cut a trailing space from the last path.
- The subdir checks, `TreeRecord::relative` and `root_label` are pure decisions over parsed records,
  so they are built here (D13: rules in `types`); plan 11 calls them.
- The §9.1 stale note wording is `Note::SourceChanged` (plan 06), since `Plan` holds it and plan 06
  does not depend on this plan.

## Proof

- `cargo make dod` is green at the end of the plan.
- Spec rows covered, each by a named test above: §9.1 rows 1-5, the moved-tag paragraph and §16 R9
  (Task 2); §9.1 row 2 tree refusal, path-tree change and §16 R18 (Task 3); §16 R2 unreadable git
  output (Tasks 4, 5, 6); §9.2 "list tags" output shape and empty output (Task 5); §9.2 "list files"
  record shape, byte order, raw paths, §16 R10 mode mapping, §16 R22 root label, §16 R26 whole-tree
  paths, and the two subdir failure rows (Task 6); §9.3 layout, temp name, HOME refusal and §16 R10
  empty HOME (Task 7); §9.4 steps 1-3, the split rule, both examples and §16 R8 tie-break (Task 8);
  §9.4 case table rows 1-4, the `app/update` output shape and §16 R8 entry order (Task 9). The §9.2
  commands, the git-missing and git-failure refusals, and the cache rename race are plan 11's and
  plan 10's.
- `grep -rn "spec\|chain" crates/agentstacks/src/types/resolve` finds nothing in doc comments.
- `grep -rn "Fake" crates/agentstacks/src/types/resolve` finds nothing (no doctest uses a fake).

## Review findings

Reviewer: one round, batch with plans 04, 05 and 08. Blocking: none. The batch's `cargo make dod` exited 0.

- open 🟡 `reconcile.rs:69`: the private path normaliser copies `SubPath::normalized` line for line. Move it to `LocalPath::normalized`.
- open ❓ `remote.rs:81`, `listing.rs:134`: when a later `ls-remote` line or `ls-tree` record is bad, the refusal still quotes the first line, which is valid. This follows the plan's "<first line>" wording but points at the wrong line.
- open 🔵 `unread.rs:9`: the doc is stale.
- open 🔵 `listing.rs:272`, `update.rs:276`: probe/spec vocabulary in test code.
- open 🔵 `TreeRecord` has plain `String` fields.

## Deviations

- 2026-11-22: `first_line` cuts at the first `\n` or NUL, where the plan says only `\n`. A malformed `ls-tree -z` record is then quoted without a trailing NUL. Output without NULs is unchanged.
- 2026-11-22: `UpdateQuery::for_source` takes the key by value. For a tag with no version, `UpdateEntry.current` is `Some(tag)`; the plan specifies `None` only for rev pins.
- 2026-11-22: `same_source` normalises `path` with a private helper, because `LocalPath` has no `normalized()`. A subdir of `.` normalises to empty and is not treated as "none".
- 2026-11-22: Four tests were added beyond the plan: `classes_follow_packs_then_tools_order`, `first_line_stops_at_the_first_record_of_nul_separated_output`, `ls_remote_one_bad_line_refuses_the_whole_output`, `ls_tree_empty_output_is_empty`.
- 2026-11-22: The red phase was seen in a scratch copy, because others' in-progress files kept the real crate red. Task 1 was written test-and-code together. The real crate was green at the wave gate.
