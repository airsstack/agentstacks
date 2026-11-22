---
status: approved
created: 2026-11-22
built-on: 7b4a3a8f175f
---

# Temp file symlinks Implementation Plan

**Goal:** Make every temp file the core writes in the project discard a planted symlink instead of following it.

**Architecture:**
- `stage_bytes` (`crates/agentstacks/src/fs/workspace/stage.rs`) becomes the one function that writes a temp: it creates the parent directories, removes whatever sits at the temp (`NotFound` is done), then opens it with `OpenOptions::new().write(true).create_new(true)` and writes the bytes.
- `write_project` and `write_lock` (`crates/agentstacks/src/fs/workspace/swap.rs`) write their temp through `stage_bytes`, then `swap_onto` as today.
- No new refusal. A failure is the platform's `io::Error`, located by the callers exactly as today (spec §8.5 Refusals).

**Tech Stack:** Rust 2024, `std::fs::OpenOptions`, `tempfile` for the contract tests.

**Content authority:** spec D28; §2 (the `fs/workspace/stage` line and its why-row); §8.5 (Change, Behaviour, Refusals); §11 (the two temp-path rows); §12 (the `fs` temp files row).

---

## Files that change

```
crates/agentstacks/src/fs/workspace/stage.rs   — modify: stage_bytes removes then create_new; unix contract tests
crates/agentstacks/src/fs/workspace/swap.rs    — modify: write_project and write_lock stage through stage_bytes; unix contract tests
```

## Order of work

### Task 1 — `stage_bytes` discards what sits at the temp

**Files:**
- Modify `crates/agentstacks/src/fs/workspace/stage.rs`:
  - `stage_bytes` runs the three steps of spec §8.5 Change, in that order;
  - its doc comment and the module's Responsibilities bullet say a symlink at the temp is removed, never followed;
  - a new test module gated `#[cfg(all(test, unix))]` holds the contract tests below; the existing `#[cfg(test)] mod tests` stays, and `stage_overwrites_a_leftover_temp` stays green unchanged.

**Change:** A symlink, live or dangling, or any other non-directory entry at the temp is removed, then a regular file is created. A directory at the temp is an `Err`.

**Proof:** written first, seen failing, then passing:
- `stage_discards_a_symlink_to_an_outside_file`: a symlink at the temp points to a file outside the tempdir project. After `stage_bytes`, the outside file's bytes are unchanged, and the temp is a regular file (`symlink_metadata`, not a symlink) holding the new bytes.
- `stage_discards_a_dangling_symlink`: a symlink at the temp points to a missing path. After `stage_bytes`, nothing exists at that path, and the temp is a regular file holding the new bytes.
- `stage_refuses_a_directory_at_the_temp`: a directory sits at the temp. `stage_bytes` returns `Err`, and the directory is still there.

**Done when:** `cargo test -p agentstacks fs::workspace::stage` passes and `cargo make dod` is green. Commit `fix(agentstacks): stage temp files without following a planted symlink`.

### Task 2 — Project and lock temps go through `stage_bytes`

**Files:**
- Modify `crates/agentstacks/src/fs/workspace/swap.rs`:
  - `write_project` and `write_lock` call `stage_bytes(&temp, text.as_bytes())` in place of `std::fs::write(&temp, text)`, then `swap_onto` as today;
  - their doc comments say a symlink at the temp is removed, never followed;
  - the contract tests below join the existing `#[cfg(all(test, unix))] mod tests`.

**Change:** `agentstacks.toml` and `agentstacks.lock` are written through the same temp rule as every staged file. Mode keeping and the rename are unchanged.

**Proof:** written first, seen failing, then passing:
- `write_project_discards_a_symlink_at_its_temp`: a symlink at `.agentstacks.toml.agentstacks-tmp` points to a file outside the tempdir project. After `write_project`, the outside file's bytes are unchanged, `agentstacks.toml` is a regular file holding the new text, and no temp is left.
- `write_project_discards_a_dangling_symlink_at_its_temp`: the link points to a missing path. After `write_project`, nothing exists at that path, and `agentstacks.toml` holds the new text.
- `write_project_refuses_a_directory_at_its_temp`: a directory sits at the temp. `write_project` returns `Err`, the directory is still there, and an existing `agentstacks.toml` keeps its old text.
- `write_lock_discards_a_symlink_at_its_temp`, `write_lock_discards_a_dangling_symlink_at_its_temp`, `write_lock_refuses_a_directory_at_its_temp`: the same three cases for `write_lock` and `.agentstacks.lock.agentstacks-tmp`, each asserting the same outcomes on `agentstacks.lock`.

**Done when:** `cargo test -p agentstacks fs::workspace::swap` passes and `cargo make dod` is green. Commit `fix(agentstacks): write the project and lock temps through stage_bytes`.

---

## Risks

- **Platform text.** The tests assert `Err` and the file system state, never the error text, because the text differs by OS (spec §8.5). CI on ubuntu-latest runs them for Linux.
- **Two lines for a directory at a staged temp.** Apply prints the same line twice (stage, then cleanup). Spec §8.5 keeps this as today; no task changes `src/app/apply/stage.rs`.
- **Riskiest step:** Task 1, because every staged file in apply goes through it. The existing `stage.rs`, `swap.rs` and apply tests must stay green unchanged.

## Proof

- `cargo make dod` green.
- `cargo deny check` passes.
