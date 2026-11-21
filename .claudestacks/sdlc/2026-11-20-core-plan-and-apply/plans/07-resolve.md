---
status: draft
created: 2026-11-22
depends-on: [02]
built-on: ea104af7451e
---

# Resolve Implementation Plan

**Goal:** reconcile the lock with agentstacks.toml.

**Architecture:** pure rules in `types/resolve` (spec §2 table row `types/resolve`, D13). Each rule takes
values the flows and adapters already hold and returns an answer; nothing here runs git, reads the
disk or reads the environment. Reconciliation (§9.1) classifies every tool and pack entry from the
`ProjectFile` and the optional `Lockfile`. The tree check refuses a git tree that differs from the
lock. The git output parsers (§9.2) turn `ls-remote` lines and `ls-tree -z` records into values and
map tree modes to `EntryKind`; the subdir checks decide "not found" and "is a file" from those records.
The cache layout (§9.3) is a set of path functions over a home directory that the caller supplies.
The tag rules (§9.4) split a tag into prefix and version, choose the latest tag, and build one
`UpdateEntry` per git source. Refusals are `FetchError` variants (plan 02), wrapped in a `Diagnostic`
at `agentstacks.toml` with the entry's key path (§13 row "`agentstacks.toml` with key") or with no key
(HOME, §13 row "`agentstacks.toml`, no key"). Plan notes (§9.4) are `UpdateNote` values that display
their exact text. The §9.1 "source changed" note text is `SourceNote` in `types/plan` (plan 06),
because `Plan` holds it; this plan supplies the `Stale` classification it is made from.

**Tech Stack:** Rust 2024, `crates/agentstacks` only; existing types `ProjectFile`, `PackEntry`,
`Source`, `Pin`, `GitUrl`, `GitTag`, `CommitId`, `SubPath`, `LocalPath` (`src/types/project/*`),
`Lockfile`, `LockedEntry`, `LockedSource` (`src/types/lock/*`), `ContentHash`, `PackVersion`
(semver precedence, `src/types/manifest/pack/version.rs`), `EntryKind` (`crate::port`, after plan
01's move), `Diagnostic`, `Location`; no new dependency.

**Content authority:** spec §9.1, §9.2 (parsing and mapping rows, subdir failure rows), §9.3
(layout, temp name, HOME), §9.4; probes `probes/git-results.md` G1 and G4.

Rust guideline rules applied (claudestacks-guideline-rust 0.1.6): `mod-rs-export-only.md`
(`types/resolve/mod.rs` holds only docs, `mod` and `pub use`); `modularity.md` Rule 1 (one job per
file: reconcile, tree check, each parser, cache layout, tag rule, update entry) and Rule 2 (the
source key and notes are defined once, here); `strong-types.md` (keys are `SourceKey`, revs
`CommitId`, tags `GitTag`, versions `PackVersion`, no `bool` decision parameter);
`unit-test-mandate.md` (colocated tests in every logic file); `doc-comment-discipline.md`;
`static-dispatch.md` (no trait objects, no port).

---

## Files that change

```
crates/agentstacks/src/types/resolve/mod.rs        — modify: add mod / pub use lines (docs stay)
crates/agentstacks/src/types/resolve/key.rs        — create: SourceKey
crates/agentstacks/src/types/resolve/reconcile.rs  — create: Reconciled, same-source rule, reconcile (§9.1)
crates/agentstacks/src/types/resolve/check.rs      — create: the git tree check (§9.1 row 2)
crates/agentstacks/src/types/resolve/remote.rs     — create: RemoteTag, the ls-remote line parser (§9.2 G1)
crates/agentstacks/src/types/resolve/listing.rs    — create: TreeRecord, ls-tree record parser, mode mapping, subdir checks (§9.2 G4)
crates/agentstacks/src/types/resolve/cache.rs      — create: cache layout, temp name, HOME (§9.3)
crates/agentstacks/src/types/resolve/tag.rs        — create: TagVersion split, latest_tag (§9.4)
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
within each, so a map keyed by it is deterministic.

**Proof:**
- `source_key_displays_its_key_path` — `packs.core`, `tools.claude`.
- `source_key_locations` — `location()` displays as the prefix `agentstacks.toml: packs.core`;
  `subdir_location()` as `agentstacks.toml: packs.core.subdir`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): source entry key`.

### Task 2 — Reconcile every entry with the lock

**Files:**
- Create `crates/agentstacks/src/types/resolve/reconcile.rs` — `Reconciled`, the same-source rule
  and `reconcile`.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod reconcile;` plus `pub use`.

**Change:** `same_source(source: &Source, locked: &LockedSource) -> bool` compares the source fields
§9.1 names (`git`, `tag`, `rev`, `subdir` / `path`): git against git compares `url` and `subdir`, and
the pin: `Pin::Tag(t)` matches a lock whose `tag` is `Some(t)` (its `rev` is then the lock's, used
as is); `Pin::Rev(r)` matches a lock whose `rev` is `r` and whose `tag` is `None` (formats §8: an
extra `tag` makes the lock stale); path against path compares the `LocalPath`; git against path is
different. Fields compare as their typed values (gap GAP-6).

`Reconciled` is (§9.1 table):

| Variant | Row |
|---|---|
| `New` | entry in toml, none in the lock: resolve (git: tag → rev; path: read) |
| `Locked { rev: Option<CommitId>, tree: ContentHash }` | same source fields: use the lock's `rev` (`None` for a path source) and check the tree |
| `Stale` | different source fields: resolve as new; the flow adds the plan note |
| `RemovedPack` | lock pack entry, none in toml: its files get delete steps (§6.2) |
| `RemovedTool` | lock tool entry, none in toml: §6.4 |

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
- `same_source_compares_url_subdir_and_path` — a different url, subdir or path → `false`; git
  against path → `false`.
- `reconcile_new_entry` — toml entry, no lock entry → `New`; a missing lock → every entry `New`.
- `reconcile_locked_entry_uses_the_lock_rev` — same fields → `Locked` with the lock's rev and tree.
- `reconcile_path_entry_locked_without_rev` — same path → `Locked { rev: None, .. }`.
- `reconcile_changed_source_is_stale` — different fields → `Stale`.
- `reconcile_removed_pack` — lock pack, no toml pack → `RemovedPack`.
- `reconcile_removed_tool` — lock tool, no toml tool → `RemovedTool`.
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
(§9.1 row 2; `<a>` the fetched tree, `<b>` the lock's, `<rev>` the locked commit). For a path source
it always passes: "a changed path tree is a normal change, shown in the plan" (§9.1 row 2; formats
§8.2).

**Proof:**
- `git_tree_mismatch_is_refused` — the whole diagnostic equals the §9.1 text with the test's hashes
  and rev filled in.
- `git_tree_equal_passes` — equal hashes → `Ok`.
- `path_tree_change_is_not_refused` — a path source with different hashes → `Ok`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): refuse a git tree that differs from the lock`.

### Task 4 — Parse `ls-remote` tag lines

**Files:**
- Create `crates/agentstacks/src/types/resolve/remote.rs` — `RemoteTag` and its parser.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod remote;` plus `pub use`.

**Change:** `RemoteTag { rev: CommitId, tag: GitTag }`. `RemoteTag::parse_line(line: &str) ->
Option<RemoteTag>` reads one `<hex>\t<ref>` line (§9.2 "list tags", G1): the hex part through
`CommitId::new`, the ref must start with `refs/tags/` and the rest through `GitTag::new`. A line that
does not have this shape gives `None`; what the flow does with it is plan 11's, and its wording is a
gap (GAP-7). `parse_ls_remote(stdout: &str) -> Vec<Option<RemoteTag>>` splits on `\n` and drops the
empty final segment, so empty output (exit 0, nothing matched, G1 `packs-v9*`) is an empty list.

**Proof:** the inputs are G1's stdout lines copied byte for byte.
- `ls_remote_lines_parse` — the four G1 lines (`aec92b45…\trefs/tags/packs-v0.1.0`,
  `90e2a535…\trefs/tags/packs-v0.10.0`, `…packs-v0.2.0`, `…packs-v0.9.0`) give four `RemoteTag`s in
  output order with those revs and tags.
- `ls_remote_empty_output_is_empty` — `""` → no entries.
- `ls_remote_non_tag_ref_is_none` — `90e2a535…\trefs/heads/packs-v1` (G1 tail-match follow-up) →
  `None`.
- `ls_remote_malformed_line_is_none` — a line without a tab, and one with a 39-hex id → `None`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse ls-remote tag lines`.

### Task 5 — Parse `ls-tree -z` records and check the subdir

**Files:**
- Create `crates/agentstacks/src/types/resolve/listing.rs` — `TreeRecord`, the parser, the mode
  mapping and the subdir checks.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod listing;` plus `pub use`.

**Change:** `TreeRecord { mode: String, object: String, oid: String, path: Vec<u8> }` with
`TreeRecord::kind() -> EntryKind`. `parse_ls_tree(stdout: &[u8]) -> Option<Vec<TreeRecord>>` splits on
NUL and reads each record as `<mode> SP <type> SP <oid> TAB <path>` (§9.2 "list files", G4); the path
stays raw bytes, because `-z` prints it unquoted (G4 follow-up: `ü` arrives as its UTF-8 bytes) and
the UTF-8 refusal belongs to the formats walk. Records keep git's byte order (G9). A record that does
not match gives `None` for the whole listing (wording: gap GAP-7). Mode mapping (§9.2 bullets):
`120000` → `EntryKind::Symlink` (formats' "symlinks are not allowed" then applies); `160000`
(gitlink) → `EntryKind::Other`; a blob with `100644` or `100755` → `EntryKind::File`; `040000` →
`EntryKind::Dir`. Any other mode is not named by the spec (gap GAP-8); this task maps nothing else until
the author settles it.

`TreeRecord::relative(&self, subdir: Option<&SubPath>) -> Option<Vec<u8>>` strips
`<normalised subdir>/` from the path (§9.2: "`subdir` is normalised (§3.3) before use"), so the
`FileTree` snapshot that plan 11 builds sees pack-relative paths.

`check_subdir(key: &SourceKey, records: &[TreeRecord], subdir: &SubPath, at: &Pin) ->
Result<(), Diagnostic>` (§9.2 failure rows), at `key.subdir_location()`; `<tag or rev>` is the
pin's `GitTag` or `CommitId` display (`Pin` itself has no `Display`):
- no records → `FetchError` not-found variant: `agentstacks.toml: packs.core.subdir: "packs/nope" not found at <tag or rev>`;
- exactly one blob record whose path equals the normalised subdir →
  `agentstacks.toml: packs.core.subdir: "packs/core/pack.toml" is a file, not a directory`;
- otherwise `Ok`.

**Proof:** the inputs are G4's records with `\0` separators, copied byte for byte.
- `ls_tree_records_parse` — the G4 listing of `packs/core` parses into fifteen records with the
  modes, types, oids and paths shown, in order.
- `ls_tree_mode_mapping` — `100644 blob` and `100755 blob` → `File`; `120000 blob …\tpacks/core/link`
  → `Symlink`; `160000 commit 0123…\tpacks/core/sub` → `Other`; `040000 tree` → `Dir`.
- `ls_tree_path_keeps_raw_bytes` — the `packs/core/ü.md` record's path is `packs/core/` followed by
  `0xC3 0xBC` and `.md`; `packs/core/sp ace.md` keeps its space.
- `ls_tree_malformed_record_is_none` — a record without a tab → `None`.
- `ls_tree_empty_output_is_empty` — `b""` → an empty list.
- `record_relative_strips_the_subdir` — `packs/core/nested/deeper/file.md` with subdir
  `./packs//core` → `nested/deeper/file.md`; no subdir → the whole path.
- `subdir_listing_nothing_is_refused` — no records, subdir `packs/nope`, pin tag `packs-v0.1.0` →
  `agentstacks.toml: packs.core.subdir: "packs/nope" not found at packs-v0.1.0`.
- `subdir_naming_a_file_is_refused` — one blob record `packs/core/pack.toml`, subdir
  `packs/core/pack.toml` →
  `agentstacks.toml: packs.core.subdir: "packs/core/pack.toml" is a file, not a directory`.
- `subdir_directory_passes` — the G4 listing with subdir `packs/core` → `Ok`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse ls-tree records`.

### Task 6 — Lay out the git cache

**Files:**
- Create `crates/agentstacks/src/types/resolve/cache.rs` — the cache path rules and HOME.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod cache;` plus `pub use`.

**Change:** (§9.3) `cache_home(home: Option<&OsStr>) -> Result<PathBuf, Diagnostic>` returns the
given `$HOME` value, or refuses at `Location::file("agentstacks.toml")` with the `FetchError` HOME
variant: `agentstacks.toml: HOME is not set, so the cache (~/.airsstack/cache) has no home`. The value
is passed in by the caller (plan 10); reading the environment is not done here. An empty `HOME` is
not settled (gap GAP-9). `url_dir(home: &Path, url: &GitUrl) -> PathBuf` is
`<home>/.airsstack/cache/git/<hex sha256 of url>`, the hex from
`ContentHash::of_bytes(url.as_str().as_bytes()).hex()`. `rev_dir(url_dir: &Path, rev: &CommitId) ->
PathBuf` is `<url dir>/<rev>.git`. `temp_name(pid: u32, n: u64) -> String` is `tmp-<pid>-<n>`; the
caller supplies its process id and counter, so this module never calls `std::process`.

**Proof:**
- `cache_home_refuses_a_missing_home` — `None` →
  `agentstacks.toml: HOME is not set, so the cache (~/.airsstack/cache) has no home`.
- `cache_home_takes_the_given_home` — `Some("/home/u")` → `/home/u`.
- `url_dir_is_the_sha256_of_the_url` — for `https://github.com/airsstack/agentstacks` the last
  segment equals `ContentHash::of_bytes` of the URL's bytes as hex, under
  `/home/u/.airsstack/cache/git/`.
- `rev_dir_appends_rev_dot_git` — `<url dir>/<40 hex>.git`.
- `temp_name_counts_up` — `temp_name(42, 0)` is `tmp-42-0`, `temp_name(42, 1)` is `tmp-42-1`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): git cache layout`.

### Task 7 — Split a tag and choose the latest

**Files:**
- Create `crates/agentstacks/src/types/resolve/tag.rs` — `TagVersion`, `latest_tag`.
- Modify `crates/agentstacks/src/types/resolve/mod.rs` — `mod tag;` plus `pub use`.

**Change:** `TagVersion { prefix: String, version: PackVersion }`. `TagVersion::split(tag: &GitTag)
-> Option<TagVersion>`: the version starts at the leftmost ASCII digit whose remainder parses as
semver (`PackVersion::new`); the prefix is everything before it (§9.4). `latest_tag(tags:
&[RemoteTag], prefix: &str) -> Option<(GitTag, PackVersion)>`: keep the tags that start with
`prefix` and whose suffix after it parses as semver and has no pre-release; return the highest by
semver precedence (`PackVersion` ordering), not the lexical order `ls-remote` gives (§9.4 steps
2-3, G1); `None` when none remains. Two kept tags of equal precedence that differ only in build
metadata are not settled (gap GAP-10); the tests use none.

**Proof:**
- `split_packs_v` — `packs-v0.1.0` → prefix `packs-v`, version `0.1.0`.
- `split_takes_the_leftmost_digit_that_parses` — `x-1.2.3-4.5.6` → prefix `x-`, version
  `1.2.3-4.5.6`.
- `split_without_a_version_is_none` — `main-latest` → `None`.
- `latest_is_by_semver_not_lexical` — the four G1 tags with prefix `packs-v` → `packs-v0.10.0`.
- `latest_skips_pre_releases` — `packs-v0.3.0-rc.1` beside `packs-v0.2.0` → `packs-v0.2.0`.
- `latest_skips_other_prefixes_and_non_semver` — `other-v9.9.9` and `packs-vnext` are ignored.
- `latest_of_nothing_is_none` — empty list, or only pre-releases → `None`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): tag split and latest tag`.

### Task 8 — Build one update entry per git source

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
paragraph). `UpdateQuery::for_source(key, source: &Source) -> Option<Result<UpdateQuery,
UpdateEntry>>`:
- a `path` source → `None` (nothing, §9.4 last row);
- a git source pinned by `rev` → `Err(entry)` with `PinnedRev` and `current: None`;
- a git tag that has no split → `Err(entry)` with `NoVersion`;
- otherwise `Ok(UpdateQuery { key, url, prefix, current tag, current version })`, for the flow to
  list `refs/tags/<prefix>*` (plan 16).

`UpdateQuery::finish(self, tags: &[RemoteTag]) -> UpdateEntry`: `latest_tag` with the prefix; a
result newer (by semver) than the current version → `NewTag`; `None` or nothing newer → `NoNewer`
with the current tag. Writing the new tag into `agentstacks.toml`, shorthand expansion and its
refusal are `cli-commands`' (§9.4, §16), not built here.

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

- **GAP-6 (confirm, author):** "same source fields" (§9.1) is built as equality of the typed values as
  written, so `subdir = "./packs/core"` after `packs/core` is `Stale` (resolved again with a note,
  harmless) rather than equal after normalising.
- **GAP-7 (open, author):** no refusal text exists for git output that does not parse (an `ls-remote`
  line or an `ls-tree` record of another shape). The parsers return `None`; plan 11 needs the
  wording.
- **GAP-8 (open, author):** §9.2 names modes `120000` and `160000`; the plan maps `100644`/`100755`
  blobs to `File` and `040000` to `Dir` as the plain meaning, and holds any other mode.
- **GAP-9 (open, author):** an empty `HOME` is not "not set"; whether it is refused is not stated.
- **GAP-10 (open, author):** two tags of equal semver precedence differing only in build metadata: which
  is "highest" is not stated.
- `EntryKind` lives in `crate::port` after plan 01; `types/resolve` importing it follows the existing
  `types` users of `FileTree`/`EntryKind` that plan 01 re-points. If the author wants `types` free of
  `port` imports, the mapping moves to plan 11.
- The subdir checks and `TreeRecord::relative` are pure decisions over parsed records, so they are
  built here (D13: rules in `types`); plan 11 calls them.
- The §9.1 stale note wording is `SourceNote` (plan 06), since `Plan` holds it and plan 06 does not
  depend on this plan.

## Proof

- `cargo make dod` is green at the end of the plan.
- Spec rows covered, each by a named test above: §9.1 rows 1-5 and the moved-tag paragraph (Task 2);
  §9.1 row 2 tree refusal and path-tree change (Task 3); §9.2 "list tags" output shape and empty
  output (Task 4); §9.2 "list files" record shape, byte order, raw paths, the `120000` and `160000`
  bullets, and the two subdir failure rows (Task 5); §9.3 layout, temp name and HOME refusal (Task
  6); §9.4 steps 1-3, the split rule and both examples (Task 7); §9.4 case table rows 1-4 and the
  `app/update` output shape (Task 8). The §9.2 commands, the git-missing and git-failure refusals,
  and the cache rename race are plan 11's and plan 10's.
- `grep -rn "spec\|chain" crates/agentstacks/src/types/resolve` finds nothing in doc comments.
