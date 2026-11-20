---
status: approved
created: 2026-11-20
depends-on: [01]
built-on: 3162a314af79
---

# Tree Hash Implementation Plan

**Goal:** A pack or extension directory has a deterministic tree hash.

**Architecture:** One directory walker, `model/walk.rs`, applies the spec §2.1 directory rules once,
so pack loading (plan 06), extension loading (plan 07) and the tree hash all share them. Per entry,
the dot-entry rule runs first: an entry whose name starts with `.` is ignored, even when it is a
symlink, a special file or a non-UTF-8 name. Every other entry is checked in turn: a symlink is
refused, a special file (FIFO, socket, device) is refused, a non-UTF-8 path is refused, an empty
directory contributes nothing. `walk(dir)` returns a `Walked`: the regular files as `TreePath`s
plus the ignored dot entries. Walker refusals are `WalkError` (one `thiserror` enum), joined into
plan 01's `FormatError` as a `Walk` variant. The symlink refusal carries the §2.1 pack wording;
plan 07's `reword_walk_diagnostic` turns it into the §2.1 extension wording
(`symlinks are not allowed in an extension`) for extension directories. An I/O failure is never a
`WalkError`: it is plan 01's single `FormatError::Io` variant, displayed `<path>: <the
std::io::Error Display>` (§10). `lock` becomes a directory module: `lock/content_hash.rs` holds
`ContentHash` (the `sha256:<64 lowercase hex>` value, parsed on construction),
`lock/content_hash_error.rs` its refusal, and `lock/tree_hash.rs` computes `tree_hash(dir)` (§11)
from the walker's output with `std::fs` reads and `sha2`. Recursion uses `std::fs::read_dir`; no
walking dependency is added. Every file holds one item (export-only `mod.rs`); every logic-bearing
file carries a colocated `#[cfg(test)] mod tests`.

**Tech Stack:** Rust 2024 (workspace `rust-version = "1.94"`), `sha2` 0.11 (`0.11.1`), `tempfile` 3
(`3.27.0`, dev-dependency, for directory fixtures). Versions per spec §11.

**Content authority:** spec §2.1 (rows: no symlinks anywhere, with the pack and extension
wordings; only regular files and directories; empty directories ignored; the dot-entry rule
applied first; paths are valid UTF-8; dot entries ignored and listed once), §4.5 (row: no
symlinks, non-UTF-8 paths refused, dot entries ignored, "as §2.1"), §8 (row: hashes
`^sha256:[0-9a-f]{64}$` and the bad-hash message), §8.1 (tree hash and its checked example), §10
(location forms, the I/O form `<path>: <the std::io::Error Display>`, all errors per run, sort
order), §11 (module layout, `tree_hash(dir)`, dependencies), §12 (every checked example of §8.1 is
a test; every refusal has a test).

---

## Files that change

```
Cargo.toml                                        — modify  add sha2 = "0.11" and tempfile = "3" to [workspace.dependencies]
crates/agentstacks/Cargo.toml                     — modify  depend on sha2 (workspace); dev-depend on tempfile (workspace)
crates/agentstacks/src/lib.rs                     — modify  module docs: lock is no longer empty
crates/agentstacks/src/model/mod.rs               — modify  declare and re-export walk_error, tree_path, walked, walk
crates/agentstacks/src/model/walk_error.rs        — create  WalkError (thiserror): Symlink, SpecialFile, NotUtf8
crates/agentstacks/src/model/tree_path.rs         — create  TreePath: relative, "/"-separated, UTF-8 path of a walked file
crates/agentstacks/src/model/walked.rs            — create  Walked { files, ignored }, both sorted
crates/agentstacks/src/model/walk.rs              — create  walk(dir) → Walked or every Diagnostic
crates/agentstacks/src/model/format_error.rs      — modify  FormatError gains Walk(#[from] WalkError)
crates/agentstacks/src/lock.rs                    — delete  replaced by lock/mod.rs
crates/agentstacks/src/lock/mod.rs                — create  export-only table of contents for lock
crates/agentstacks/src/lock/content_hash_error.rs — create  ContentHashError: the §8 bad-hash message
crates/agentstacks/src/lock/content_hash.rs       — create  ContentHash: sha256:<64 lowercase hex>
crates/agentstacks/src/lock/tree_hash.rs          — create  tree_hash(dir) → ContentHash or every Diagnostic
```

## Order of work

### Task 1 — Name the walker's vocabulary

**Files:**
- Create `crates/agentstacks/src/model/walk_error.rs` — `WalkError` (`thiserror`), three variants
  and no I/O variant: `Symlink` displays `symlinks are not allowed in a pack`; `SpecialFile`
  displays `only regular files and directories are allowed`; `NotUtf8` displays `path is not valid
  UTF-8`. The `Diagnostic` location carries the path, so the messages do not repeat it.
- Create `crates/agentstacks/src/model/tree_path.rs` — `TreePath`: a newtype over the
  `/`-separated, valid UTF-8 form of a path relative to the walked directory. Construction is
  `pub(crate)` (`TreePath::from_relative(&Path)`), so only the walker builds one and every value
  is relative by construction; it joins the path's components with `/` and refuses a component
  that is not valid UTF-8 with `WalkError::NotUtf8`. `as_str` returns the text; `Ord` is the byte
  order of that text, which is the "sort by path bytes" order of §8.1; `Display` prints the text.
- Modify `crates/agentstacks/src/model/format_error.rs` — `FormatError` gains
  `Walk(#[from] WalkError)`, `#[error(transparent)]`, displaying the wrapped message unchanged.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export `WalkError`, `TreePath`.

**Change:** the §2.1 symlink, special-file and UTF-8 refusal messages as types. `Symlink` carries
the pack wording; extension directories get `symlinks are not allowed in an extension` from plan
07's `reword_walk_diagnostic`, which rewords this variant. The other two messages are the same for
packs and extensions (§4.5 "as §2.1").

**Proof:**
- `walk_error_symlink_displays_the_pack_wording` (in `walk_error.rs`) — `Symlink` displays
  exactly `symlinks are not allowed in a pack` (§2.1).
- `walk_error_special_file_displays_the_spec_message` (in `walk_error.rs`) — `SpecialFile`
  displays exactly `only regular files and directories are allowed` (§2.1).
- `walk_error_not_utf8_displays_the_spec_message` (in `walk_error.rs`) — `NotUtf8` displays
  exactly `path is not valid UTF-8` (§2.1).
- `tree_path_joins_components_with_slash` (in `tree_path.rs`) — the relative path built from
  components `b` and `c.lua` gives `as_str` = `b/c.lua`; a single component `a.md` gives `a.md`.
- `tree_path_refuses_a_non_utf8_component` (in `tree_path.rs`, `#[cfg(unix)]`) — a relative path
  whose second component is the bytes `0x66 0xFF` is refused with `WalkError::NotUtf8`. The test
  builds the path in memory and touches no disk.
- `tree_path_orders_by_path_bytes` (in `tree_path.rs`) — `b/c.lua`, `a/b`, `a.md`, `B.md`
  sort to `B.md`, `a.md`, `a/b`, `b/c.lua` (`B` 0x42 before `a` 0x61; `.` 0x2E before `/` 0x2F).
- `format_error_walk_displays_the_walk_message` (in `format_error.rs`) — a `FormatError` made
  from each `WalkError` variant displays exactly that variant's message.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add the directory walker vocabulary`.

### Task 2 — Walk a directory under the spec's rules

**Files:**
- Modify `Cargo.toml` — add `tempfile = "3"` to `[workspace.dependencies]`.
- Modify `crates/agentstacks/Cargo.toml` — `[dev-dependencies]` `tempfile` (workspace).
- Create `crates/agentstacks/src/model/walked.rs` — `Walked`: `files` (`Vec<TreePath>`, every
  regular file) and `ignored` (`Vec<PathBuf>`, every dot entry, relative to the walked
  directory; a `PathBuf` because a dot entry may have a non-UTF-8 name). `pub(crate)` constructor
  `Walked::new(files, ignored)` sorts both by path bytes; accessors `files()` and `ignored()`
  return slices.
- Create `crates/agentstacks/src/model/walk.rs` — `walk(dir: &Path) -> Result<Walked,
  Vec<Diagnostic>>`. Two private steps: `is_dot_entry(name: &OsStr) -> bool` decides the dot rule
  on the name's first byte (`OsStr::as_encoded_bytes`), so it never needs UTF-8; and
  `located_tree_path(dir, relative)` turns one relative path into a `TreePath` or into the
  `Diagnostic` for it.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export `Walked`, `walk`.

**Change:** `walk` reads `dir` recursively with `std::fs::read_dir`. For each entry, in this
order (§2.1):
1. **Dot rule first.** A name whose first byte is `.` is a dot entry: its relative path goes to
   `ignored` and it is not inspected further, so a dot entry that is a symlink, a special file or
   a non-UTF-8 name is ignored, never refused. A dot directory is listed once and not descended
   (§2.1: ignored, not hashed, not installed, `plan` lists them once).
2. The entry's own type is read without following links (`DirEntry::file_type`). A symlink, to a
   file or to a directory, is refused with `WalkError::Symlink` and never followed.
3. An entry that is neither a regular file nor a directory (a FIFO, socket or device) is refused
   with `WalkError::SpecialFile`.
4. The relative path becomes a `TreePath` through `located_tree_path`; a non-UTF-8 path is
   refused with `WalkError::NotUtf8`.
5. A directory is descended and is not itself listed: an empty directory therefore contributes
   nothing and is never a refusal (§2.1; §8.1: directories not hashed). A regular file is added
   to `files`.

A refused entry gets exactly one refusal, the first of steps 2–4 that applies, and a refused
directory is not descended. Every refusal is a `Diagnostic` with a file-only `Location` (plan 01's
`Location::file`) whose path is `dir` joined with the entry's relative path, so it displays
`<path>: <message>` (§2.1, §10). A failure to read a directory or an entry's type is a
`Diagnostic` carrying plan 01's `FormatError::Io`, at a file-only location naming that path, so it
displays `<path>: <the std::io::Error Display>` (§10). The walk continues past every refusal and
I/O failure; the diagnostics are collected in plan 01's `Diagnostics` and returned through
`Diagnostics::into_result`, sorted per §10 (all errors per run). `Ok` is returned only when there
are none.

**Proof:**
- `walked_sorts_files_and_ignored_by_path_bytes` (in `walked.rs`) — given files `b/c.lua`,
  `a.md` and ignored `.git`, `.DS_Store` out of order, `files()` is `a.md`, `b/c.lua` and
  `ignored()` is `.DS_Store`, `.git`.
- `is_dot_entry_reads_the_first_byte` (in `walk.rs`, `#[cfg(unix)]`) — names built in memory:
  `.DS_Store` and the bytes `0x2E 0xFF` are dot entries; `a.md`, `a.` and the bytes `0x66 0xFF`
  are not. No file is created.
- `walk_lists_files_sorted_with_slash_separators` (in `walk.rs`) — a temp dir holding `b/c.lua`
  and `a.md` walks to `files` = `a.md`, `b/c.lua` and an empty `ignored`.
- `walk_ignores_empty_directories` (in `walk.rs`) — empty directories `d/`, `e/f/` and
  `skills/a/` beside `a.md` give `Ok` with `files` = `a.md`, an empty `ignored` and no
  diagnostic.
- `walk_ignores_dot_entries_and_lists_each_once` (in `walk.rs`) — a temp dir holding `a.md`,
  `.DS_Store`, `.git/config` and `skills/x/.hidden` walks to `files` = `a.md` and `ignored` =
  `.DS_Store`, `.git`, `skills/x/.hidden`; `.git/config` appears in neither list.
- `walk_refuses_a_symlinked_file` (in `walk.rs`, `#[cfg(unix)]`) — `link` → `a.md` gives exactly
  one `Diagnostic` whose `Display` is `<dir>/link: symlinks are not allowed in a pack`, with
  `<dir>` the temp dir's displayed path.
- `walk_refuses_a_symlinked_directory_without_following_it` (in `walk.rs`, `#[cfg(unix)]`) —
  `sub` → a directory holding `t.md`: exactly one `Diagnostic` displaying `<dir>/sub: symlinks
  are not allowed in a pack`; no diagnostic or file under `sub/` is reported.
- `walk_reports_every_symlink` (in `walk.rs`, `#[cfg(unix)]`) — two symlinks, `l1` and `x/l2`,
  give two diagnostics, `<dir>/l1: symlinks are not allowed in a pack` then `<dir>/x/l2: symlinks
  are not allowed in a pack` (§10 sort by file).
- `walk_refuses_a_special_file` (in `walk.rs`, `#[cfg(unix)]`) — a Unix domain socket `sock`,
  created with `std::os::unix::net::UnixListener::bind` beside `a.md`, gives exactly one
  `Diagnostic` whose `Display` is `<dir>/sock: only regular files and directories are allowed`;
  `sock` is not in `files`.
- `walk_applies_the_dot_rule_first` (in `walk.rs`, `#[cfg(unix)]`) — a symlink `.link` → `a.md`
  and a socket `.sock` beside `a.md` give `Ok`: both are in `ignored`, neither is in `files`, and
  there is no diagnostic. (The non-UTF-8 dot name is covered by
  `is_dot_entry_reads_the_first_byte`.)
- `located_tree_path_refuses_non_utf8_with_location` (in `walk.rs`, `#[cfg(unix)]`) — for `dir`
  = `packs/core` and a relative path built in memory from the bytes `0x66 0xFF`, the
  `Diagnostic`'s `Display` equals the displayed path `packs/core` joined with that relative path,
  then `: path is not valid UTF-8` (§2.1 `<path>: path is not valid UTF-8`). No file is created.
- `walk_reports_a_missing_directory_as_io` (in `walk.rs`) — walking a path `missing` inside a
  temp dir returns exactly one `Diagnostic`, whose error is `FormatError::Io` and whose `Display`
  is `<dir>/missing: ` followed by the `Display` of the `std::io::Error` that
  `std::fs::read_dir` returns for that same path (§10 I/O form).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green;
`cargo make deny` passes (new dev-dependency). Commit `feat(agentstacks): walk pack and extension
directories under the format rules`.

### Task 3 — Add `ContentHash`

**Files:**
- Modify `Cargo.toml` — add `sha2 = "0.11"` to `[workspace.dependencies]`.
- Modify `crates/agentstacks/Cargo.toml` — `[dependencies]` `sha2` (workspace).
- Delete `crates/agentstacks/src/lock.rs`; create `crates/agentstacks/src/lock/mod.rs` — the
  four-question `//!` block (what: `agentstacks.lock` and the hashes it pins; why it exists;
  responsibilities: content hashes and tree hashes now, the lockfile later; non-responsibilities:
  fetching sources, comparing a tree against the lock), declarations and re-exports only, with
  the export-only exemption line.
- Create `crates/agentstacks/src/lock/content_hash_error.rs` — `ContentHashError` (`thiserror`
  struct) displaying `bad hash (expected sha256:<64 lowercase hex>)`, the §8 message without its
  location. Plan 09 places it under the lock's key path (`agentstacks.lock: file[0].hash: …`).
- Create `crates/agentstacks/src/lock/content_hash.rs` — `ContentHash`: `ContentHash::parse(&str)`
  accepts only `sha256:` followed by exactly 64 characters from `0-9a-f`, else
  `ContentHashError`; `FromStr` delegates to `parse`; `ContentHash::of_bytes(&[u8])` hashes raw
  bytes with `sha2::Sha256` and formats the 32 digest bytes as lowercase hex, byte by byte, with no
  hex dependency; `hex()` returns the 64 hex characters without the prefix; `Display` gives the
  `sha256:<hex>` form.
- Modify `crates/agentstacks/src/lib.rs` — the module docs drop "Every module currently holds no
  items" for `lock` and say it now holds the content and tree hashes.

**Change:** the hash form of §8 (`^sha256:[0-9a-f]{64}$`) as one validated type, reused by the
tree hash here and by the lockfile in plan 09.

**Proof:**
- `content_hash_error_displays_the_spec_message` (in `content_hash_error.rs`) — displays exactly
  `bad hash (expected sha256:<64 lowercase hex>)` (§8).
- `content_hash_of_bytes_matches_the_spec_digests` (in `content_hash.rs`) — `of_bytes` of `a\n`
  displays `sha256:87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7`; of `x\n`
  displays `sha256:73cb3858a687a8494ca3323053016282f3dad39d42cf62ca4e79dda2aac7d9ac` (§8.1).
- `content_hash_hex_omits_the_prefix` (in `content_hash.rs`) — `hex()` of the `a\n` hash is
  `87428fc522803d31065e7bce3cf03fe475096631e5e07bbd7a0fde60c4cf25c7`.
- `content_hash_parse_round_trips_display` (in `content_hash.rs`) — parsing the displayed `a\n`
  hash, directly and through `FromStr`, gives a value equal to `of_bytes(b"a\n")`.
- `content_hash_refuses_bad_forms` (in `content_hash.rs`) — each of: the `a\n` hex in uppercase
  after `sha256:`; 63 hex characters; 65 hex characters; `md5:` before 64 hex; 64 hex with no
  prefix; the empty string; 64 characters containing a `g` — is refused, and the error displays
  exactly `bad hash (expected sha256:<64 lowercase hex>)`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green;
`cargo make deny` passes (new dependency). Commit `feat(agentstacks): add validated content
hashes`.

### Task 4 — Compute the tree hash

**Files:**
- Create `crates/agentstacks/src/lock/tree_hash.rs` — `tree_hash(dir: &Path) ->
  Result<ContentHash, Vec<Diagnostic>>` (§11), with a private `combine(entries)` that turns
  `(TreePath, ContentHash)` pairs into the tree hash without touching the disk.
- Modify `crates/agentstacks/src/lock/mod.rs` — declare and re-export `tree_hash`.

**Change:** §8.1. `tree_hash` walks `dir` with `walk(dir)`, so dot entries are skipped, empty
directories contribute nothing, and symlinks, special files and non-UTF-8 paths are refused with
the walker's diagnostics. It takes no exclude list: excluded files are included, as §8.1 and §4.2
require. Each file in `files` is read whole with `std::fs::read`; a read failure is a `Diagnostic`
carrying `FormatError::Io` at a file-only location `<dir>/<path>`, displayed `<dir>/<path>: <the
std::io::Error Display>` (§10), and reading continues so every failure is returned, sorted through
`Diagnostics::into_result`. `combine` sorts the entries by path bytes, forms for each the bytes of
the path, `0x00`, `hex()` of the file's `ContentHash::of_bytes`, and `0x0A`, concatenates them
with no other separators, and returns `ContentHash::of_bytes` of the concatenation. Raw bytes: no
line-ending normalisation; file modes are never read; directories contribute nothing.

**Proof:**
- `combine_sorts_entries_by_path_bytes` (in `tree_hash.rs`) — the entries (`b/c.lua`, digest of
  `x\n`) then (`a.md`, digest of `a\n`), given in that order, combine to
  `sha256:b753fa890a9101e377ba1a71365c2257312468aa5d939b5e084de025486dc99b` (§8.1: a walk visiting
  `b/c.lua` first gives the same result).
- `tree_hash_matches_the_spec_example` (in `tree_hash.rs`) — a temp dir with `a.md` = `a\n` and
  `b/c.lua` = `x\n` hashes to exactly
  `sha256:b753fa890a9101e377ba1a71365c2257312468aa5d939b5e084de025486dc99b` (§8.1).
- `tree_hash_ignores_creation_order` (in `tree_hash.rs`) — the same two files written `b/c.lua`
  first give the same hash.
- `tree_hash_ignores_dot_entries` (in `tree_hash.rs`) — adding `.DS_Store` and `.git/config` to
  the spec example leaves the spec hash unchanged.
- `tree_hash_ignores_empty_directories` (in `tree_hash.rs`) — adding empty directories `d/` and
  `e/f/` to the spec example leaves the spec hash unchanged and gives no diagnostic.
- `tree_hash_ignores_file_modes` (in `tree_hash.rs`, `#[cfg(unix)]`) — setting mode `0o755` on
  `b/c.lua` leaves the spec hash unchanged.
- `tree_hash_keeps_line_endings` (in `tree_hash.rs`) — `a.md` = `a\r\n` in place of `a\n` gives a
  hash different from the spec hash.
- `tree_hash_covers_the_path` (in `tree_hash.rs`) — renaming `a.md` to `A.md` with the same bytes
  gives a hash different from the spec hash.
- `tree_hash_refuses_a_symlink` (in `tree_hash.rs`, `#[cfg(unix)]`) — the spec example plus a
  symlink `link` → `a.md` returns `Err` holding exactly one `Diagnostic` whose `Display` is
  `<dir>/link: symlinks are not allowed in a pack`.
- `tree_hash_refuses_a_special_file` (in `tree_hash.rs`, `#[cfg(unix)]`) — the spec example plus
  a Unix domain socket `sock` returns `Err` holding exactly one `Diagnostic` whose `Display` is
  `<dir>/sock: only regular files and directories are allowed`.
- `tree_hash_reports_a_missing_directory_as_io` (in `tree_hash.rs`) — hashing a path `missing`
  inside a temp dir returns `Err` holding exactly one `Diagnostic` whose `Display` is
  `<dir>/missing: ` followed by the `Display` of the `std::io::Error` that `std::fs::read_dir`
  returns for that same path (§10 I/O form).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done
(`cargo make dod`) is green. Commit `feat(agentstacks): compute pack and extension tree hashes`.

---

## Risks

- The walker words the symlink refusal for a pack. Extension directories get the §2.1 extension
  wording only through plan 07's `reword_walk_diagnostic`; `tree_hash(dir)` called directly on an
  extension directory that holds a symlink returns the pack wording. Plan 07 calls `tree_hash`
  only after a walk with no diagnostic, so no user sees it.
- The spec does not order the symlink, special-file and UTF-8 rules against one another for a
  non-dot entry. This plan gives such an entry one refusal, the first of steps 2–4 that applies,
  and does not descend a refused directory, so a non-UTF-8 directory yields one refusal rather
  than one per file beneath it.
- The UTF-8 refusal is proven on in-memory paths (`tree_path_refuses_a_non_utf8_component`,
  `located_tree_path_refuses_non_utf8_with_location`, `is_dot_entry_reads_the_first_byte`), not
  on a file created on disk: macOS file systems refuse non-UTF-8 names.
- Special files are proven with a Unix domain socket (`UnixListener::bind`, safe `std`) rather
  than a FIFO, which needs `mkfifo` through `libc` or a child process. Socket paths have a length
  limit (about 104 bytes on macOS, 108 on Linux); temp-dir paths stay well under it.
- Symlink, socket, mode and non-UTF-8 tests are `#[cfg(unix)]`. CI runs them: both CI jobs use
  `ubuntu-latest` (`.github/workflows/ci.yml:28`, `:61`).
- I/O tests build their expected text from the `std::io::Error` that `std::fs::read_dir` returns
  for the same path, so they assert the §10 form word for word without hard-coding the operating
  system's wording.
- Option not taken: exposing `combine` for hashing git blobs read with `git cat-file` (§8.1,
  handed to `core-plan-and-apply` in §13). It stays private until that chain needs it.

## Proof

- `cargo make dod-crate agentstacks` green after every task; `cargo make dod` green at the end;
  `cargo make deny` green after Tasks 2 and 3.
- The §8.1 checked example is asserted verbatim: both per-file digests
  (`content_hash_of_bytes_matches_the_spec_digests`), the tree hash
  (`tree_hash_matches_the_spec_example`) and its order independence
  (`combine_sorts_entries_by_path_bytes`, `tree_hash_ignores_creation_order`).
- Coverage map, every rule in this plan's scope → its tests (exact `Display` asserted):

| Spec row | Message | Tests |
|---|---|---|
| §2.1 no symlinks (pack) | `<path>: symlinks are not allowed in a pack` | `walk_error_symlink_displays_the_pack_wording`, `walk_refuses_a_symlinked_file`, `walk_refuses_a_symlinked_directory_without_following_it`, `walk_reports_every_symlink`, `tree_hash_refuses_a_symlink` |
| §2.1 / §4.5 no symlinks (extension) | `<path>: symlinks are not allowed in an extension` | plan 07 (`reword_walk_diagnostic` rewords `WalkError::Symlink`) |
| §2.1 only regular files and directories | `<path>: only regular files and directories are allowed` | `walk_error_special_file_displays_the_spec_message`, `walk_refuses_a_special_file`, `tree_hash_refuses_a_special_file` |
| §2.1 empty directories ignored | — | `walk_ignores_empty_directories`, `tree_hash_ignores_empty_directories` |
| §2.1 dot-entry rule applied first | — | `walk_applies_the_dot_rule_first`, `is_dot_entry_reads_the_first_byte` |
| §2.1 paths are valid UTF-8 | `<path>: path is not valid UTF-8` | `walk_error_not_utf8_displays_the_spec_message`, `tree_path_refuses_a_non_utf8_component`, `located_tree_path_refuses_non_utf8_with_location` |
| §2.1 dot entries ignored, listed once | — | `walk_ignores_dot_entries_and_lists_each_once`, `tree_hash_ignores_dot_entries` |
| §8 hash form | `bad hash (expected sha256:<64 lowercase hex>)` | `content_hash_error_displays_the_spec_message`, `content_hash_refuses_bad_forms` (location asserted in plan 09) |
| §8.1 tree hash | — | `combine_sorts_entries_by_path_bytes`, `tree_hash_matches_the_spec_example`, `tree_hash_ignores_creation_order`, `tree_hash_ignores_file_modes`, `tree_hash_keeps_line_endings`, `tree_hash_covers_the_path` |
| §10 I/O form | `<path>: <the std::io::Error Display>` | `walk_reports_a_missing_directory_as_io`, `tree_hash_reports_a_missing_directory_as_io` |
| §10 all errors per run, sorted | — | `walk_reports_every_symlink` |
