---
status: draft
created: 2026-11-20
depends-on: [01]
built-on: e665589a43a9
---

# Tree Hash Implementation Plan

**Goal:** A pack or extension directory has a deterministic tree hash.

**Architecture:** One directory walker, `model/walk.rs`, applies the spec's directory rules once:
skip every entry whose name starts with `.`, refuse symlinks, refuse paths that are not valid
UTF-8, and return regular files as `/`-separated relative paths plus the list of ignored dot
entries. Pack loading and extension loading reuse it in later plans. `lock` becomes a directory
module; `lock/content_hash.rs` holds `ContentHash` (the `sha256:<64 hex>` value, parsed on
construction) and `lock/tree_hash.rs` computes the tree hash from the walker's output with
`std::fs` reads and `sha2`. No new walking dependency: recursion uses `std::fs::read_dir`.

**Tech Stack:** Rust 2024, `sha2` 0.11, `tempfile` 3 (dev-dependency, for directory fixtures).

**Content authority:** spec §2.1 (symlinks, UTF-8 paths, dot entries), §8.1 (tree hash and its
checked example).

---

## Files that change

```
Cargo.toml                                  — modify  add sha2 = "0.11" and tempfile = "3" to [workspace.dependencies]
crates/agentstacks/Cargo.toml               — modify  depend on sha2; dev-depend on tempfile
crates/agentstacks/src/model/mod.rs         — modify  declare and re-export walk
crates/agentstacks/src/model/walk.rs        — create  walk(dir): files + ignored dot entries; refusals
crates/agentstacks/src/model/walk_error.rs  — create  WalkError (thiserror)
crates/agentstacks/src/model/diagnostic.rs  — modify  FormatError gains a Walk variant
crates/agentstacks/src/lock.rs              — delete  replaced by lock/mod.rs
crates/agentstacks/src/lock/mod.rs          — create  export-only table of contents for lock
crates/agentstacks/src/lock/content_hash.rs — create  ContentHash: sha256:<64 lowercase hex>
crates/agentstacks/src/lock/tree_hash.rs    — create  tree_hash(dir) → ContentHash
crates/agentstacks/src/lib.rs               — modify  module docs: lock is no longer empty
```

## Order of work

### Task 1 — Walk a directory under the spec's rules

**Files:**
- Create `crates/agentstacks/src/model/walk_error.rs` — `WalkError`: symlink
  (`symlinks are not allowed in a pack`), non-UTF-8 path (`path is not valid UTF-8`), I/O
  failure (path and message).
- Create `crates/agentstacks/src/model/walk.rs` — `walk(dir) → Walked { files, ignored }` or the
  collected `Diagnostics` (every symlink and bad path is reported, not just the first). `files`
  are relative, `/`-separated, sorted by bytes; `ignored` lists dot entries (a dot directory is
  listed once and not descended). A private `relative_utf8(path)` converts one relative path and
  is unit-tested without touching the disk.
- Modify `model/mod.rs`, `model/diagnostic.rs` (`FormatError::Walk(#[from] WalkError)`),
  `Cargo.toml` and `crates/agentstacks/Cargo.toml` (tempfile dev-dependency).

**Change:** §2.1 rows for symlinks, UTF-8 paths and dot entries.

**Proof:**
- `walk_lists_files_sorted_with_slash_separators` — a temp dir with `b/c.lua` and `a.md` returns
  `a.md`, `b/c.lua`.
- `walk_skips_dot_entries_and_reports_them` — `.DS_Store` and `.git/config` are absent from
  `files`; `ignored` holds `.DS_Store` and `.git`.
- `walk_refuses_symlinks` (unix only) — a symlink in the dir yields
  `<path>: symlinks are not allowed in a pack`.
- `relative_utf8_refuses_invalid_bytes` (unix only) — a path built from bytes `0x66 0xFF` yields
  `path is not valid UTF-8`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): walk pack directories under the format rules`.

### Task 2 — Compute the tree hash

**Files:**
- Delete `crates/agentstacks/src/lock.rs`; create `crates/agentstacks/src/lock/mod.rs`
  (four-question `//!` block, export-only).
- Create `crates/agentstacks/src/lock/content_hash.rs` — `ContentHash`: parses only
  `^sha256:[0-9a-f]{64}$`; `ContentHash::of_bytes` hashes raw bytes; `Display` gives the
  `sha256:` form.
- Create `crates/agentstacks/src/lock/tree_hash.rs` — `tree_hash(dir)`: walks with `walk`, and
  for each file in byte-sorted order appends `path` + `0x00` + lowercase hex SHA-256 of the raw
  bytes + `0x0A`; returns `ContentHash` of the concatenation. Raw bytes, no line-ending
  normalisation, modes ignored.
- Modify `crates/agentstacks/src/lib.rs` (lock docs), `Cargo.toml` and
  `crates/agentstacks/Cargo.toml` (sha2).

**Change:** §8.1.

**Proof:**
- `tree_hash_matches_the_spec_example` — `a.md` = `"a\n"`, `b/c.lua` = `"x\n"` hash to
  `sha256:b753fa890a9101e377ba1a71365c2257312468aa5d939b5e084de025486dc99b`.
- `tree_hash_ignores_creation_order` — the same two files written in the opposite order give the
  same hash.
- `tree_hash_ignores_dot_entries` — adding `.DS_Store` leaves the hash unchanged.
- `tree_hash_changes_with_one_byte` — `"a\r\n"` in place of `"a\n"` changes the hash (no
  normalisation).
- `content_hash_refuses_bad_forms` — `sha256:ABC…` (uppercase), a 63-hex value and `md5:…` are
  refused; `ContentHash::of_bytes(b"a\n")` displays `sha256:87428fc5…25c7` in full.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green;
`cargo make deny` passes (new dependencies). Commit `feat(agentstacks): compute pack tree hashes`.

---

## Risks

- macOS file systems reject non-UTF-8 file names, so the UTF-8 refusal is proven on the path
  conversion helper, not on a real file.
- Symlink tests are unix-only (`#[cfg(unix)]`); the refusal itself is platform-independent.

## Proof

- `cargo make dod-crate agentstacks` green; `cargo make deny` green; the §8.1 example hash is
  asserted verbatim.
