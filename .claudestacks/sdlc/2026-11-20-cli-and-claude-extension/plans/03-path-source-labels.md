---
status: approved
created: 2026-11-22
depends-on: [02]
built-on: cefc42fbbc4c
---

# Path source labels Implementation Plan

**Goal:** Label refusals from a `path`-source tree with the path as written in `agentstacks.toml`.

**Architecture:**
- `StdFileTree` gains a label, the path shown in messages, kept apart from the directory it reads.
  `FileTree::root` is documented as "the root directory, as it should appear in messages"
  (`src/port/tree.rs:57-58`), so it returns the label.
- `StdWorkspace::source_tree` builds the tree with the canonical directory for reads and the
  `LocalPath` as the label.
- This mirrors the git snapshot's `root_label` (`src/process/git/snapshot.rs:88`).

**Tech Stack:** Rust 2024; `crates/agentstacks/src/fs/`.

**Content authority:** spec D15, §8.3.

---

## Files that change

```
crates/agentstacks/src/fs/disk.rs              — modify: StdFileTree::labelled(dir, label); root() returns the label
crates/agentstacks/src/fs/workspace/project.rs — modify: source_tree labels the tree with the LocalPath as written
```

## Order of work

### Task 1 — A labelled `StdFileTree`

**Files:**
- Modify `crates/agentstacks/src/fs/disk.rs`:
  - add a `label: PathBuf` field;
  - `StdFileTree::new(root)` keeps label = root, so existing callers and doctests are unchanged;
  - a new `StdFileTree::labelled(dir, label)` reads from `dir`, and `root()` returns `label`;
  - every read keeps resolving under `dir`.

**Change:** A tree can now read one directory and name another in messages.

**Proof:**
- `labelled_tree_reads_the_directory_but_names_the_label`: a tempfile dir holding `pack.toml`, with
  label `packs/core`. `read_text("pack.toml")` returns the file, and `root()` is `packs/core`.
- `new_tree_label_is_its_root`: `StdFileTree::new(p).root() == p`.

**Done when:** `cargo test -p agentstacks fs::disk` passes and `cargo make dod` is green. Commit
`feat(agentstacks): label a disk tree apart from the directory it reads`.

### Task 2 — Label path-source trees as written

**Files:**
- Modify `crates/agentstacks/src/fs/workspace/project.rs`:
  - `source_tree` (`:135-143`) returns `StdFileTree::labelled(<canonical target>, dir.as_str())`
    for the inside and dangling cases;
  - update the existing test `source_tree_inside_reads_the_directory`, whose assertion `tree.root() ==
    ws.root().join("packs/core")` (`:375`) now expects `packs/core`.

**Change:** A refusal from a path-source pack reads
`packs/core/native/probe/hooks.toml: hooks[0].match: expected array, got string` instead of the
absolute `/private/tmp/…` form (spec §8.3).

**Proof:**
- `source_tree_inside_reads_the_directory` (updated): `root()` is `packs/core`, and reads still
  return the file.
- `path_source_refusal_names_the_path_as_written`: in a tempfile project, `packs/core/pack.toml`
  holds an invalid `[pack]` table. `Pack::load(&ws.source_tree(&LocalPath::new("packs/core")?)?…)`
  returns a refusal whose Display starts with `packs/core/pack.toml:`, and none contains the
  tempdir path.

**Done when:** `cargo test -p agentstacks fs::workspace` passes and `cargo make dod` is green.
Commit `fix(agentstacks): name path-source refusals by the path as written`.

---

## Risks

- **Other callers of `root()`.** Callers already receive a label, never a directory, from git
  trees: the snapshot's `root_label` gives `packs/core` or `.` (`src/process/git/snapshot.rs:88`,
  `:392-394`). The callers that hold a `Source` either way are
  `src/app/plan/sources.rs:47-48`, `src/types/pack/load.rs:321`, `src/types/extension/loaded.rs:159`,
  `src/types/tree/walk/run.rs:75`, `:110` and `src/app/plan/render.rs:212`. They already work with a
  relative label. Only the one fs test (`project.rs:375`) asserts an absolute root.

## Proof

- `cargo make dod` green.
