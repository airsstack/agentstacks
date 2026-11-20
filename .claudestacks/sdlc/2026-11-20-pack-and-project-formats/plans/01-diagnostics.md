---
status: draft
created: 2026-11-20
built-on: e665589a43a9
---

# Diagnostics Implementation Plan

**Goal:** Every format refusal is reported as a located diagnostic in one sorted list.

**Architecture:** `model` becomes a directory module (`model/mod.rs`, export-only). A new
`model/location.rs` holds `Location` (file, optional line, optional key path) and its display
forms; `model/diagnostic.rs` holds `Diagnostic { location, error }`, where `error` is
`FormatError`, an enum that wraps each module's own `thiserror` enum through `#[from]`.
`FormatError` starts with no wrapped variants beyond what this plan needs; each later plan adds
the variant for its module's error enum. `model/diagnostics.rs` holds `Diagnostics`, the
collection every validation run returns, which sorts on output. No other module changes
behaviour.

**Tech Stack:** Rust 2024 (workspace `rust-version = "1.94"`), `thiserror` 2.

**Content authority:** spec §10 (Errors), §11 (module layout).

---

## Files that change

```
Cargo.toml                                — modify  add thiserror = "2" to [workspace.dependencies]
crates/agentstacks/Cargo.toml             — modify  depend on thiserror (workspace)
crates/agentstacks/src/lib.rs             — modify  module docs: model is no longer empty
crates/agentstacks/src/model.rs           — delete  replaced by model/mod.rs
crates/agentstacks/src/model/mod.rs       — create  export-only table of contents for model
crates/agentstacks/src/model/location.rs  — create  Location and its two display forms
crates/agentstacks/src/model/diagnostic.rs — create Diagnostic and FormatError
crates/agentstacks/src/model/diagnostics.rs — create Diagnostics collection with sorted output
```

## Order of work

### Task 1 — Turn `model` into a directory module

**Files:**
- Delete `crates/agentstacks/src/model.rs`; create `crates/agentstacks/src/model/mod.rs` with the
  same `//!` block rewritten to the four-question shape of the mod-rs-export-only rule (what,
  why, responsibilities, non-responsibilities) and the exemption line for export-only files.
- Modify `crates/agentstacks/src/lib.rs` — drop "Every module currently holds no items" and say
  that `model` now holds the format vocabulary and its diagnostics.

**Change:** structure only; no item added yet.

**Proof:** no new test; the crate still builds and documents with zero warnings.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `refactor(agentstacks): make model a directory module`.

### Task 2 — Add `Location`

**Files:**
- Modify `Cargo.toml`, `crates/agentstacks/Cargo.toml` — add `thiserror` (workspace dependency,
  version `2`).
- Create `crates/agentstacks/src/model/location.rs` — `Location`: `file` (a path), `line`
  (optional, 1-based), `key` (optional dotted key path such as `hooks[2].grant`). Constructors
  `Location::file`, `Location::line`, `Location::key`. `Display` gives `<file>:<line>` when a line
  is present, `<file>: <key>` when a key path is present, and `<file>` otherwise.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** a located position for every refusal (§10).

**Proof:**
- `location_with_line_displays_file_colon_line` — `packs/sdlc/SKILL.md` line 12 displays as
  `packs/sdlc/SKILL.md:12`.
- `location_with_key_displays_file_then_key` — `pack.toml` with key `hooks[2].grant` displays as
  `pack.toml: hooks[2].grant`.
- `location_with_file_only_displays_file` — displays the path alone.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add Location for format diagnostics`.

### Task 3 — Add `Diagnostic`, `FormatError` and sorted `Diagnostics`

**Files:**
- Create `crates/agentstacks/src/model/diagnostic.rs` — `FormatError` (`thiserror` enum, the
  union of module error enums; this task adds only an `Io` variant carrying the path and the I/O
  error message, used when a file cannot be read) and `Diagnostic { location, error }`.
  `Diagnostic` displays as `<location>: <error>`.
- Create `crates/agentstacks/src/model/diagnostics.rs` — `Diagnostics`: `push`, `extend`,
  `is_empty`, `len`, `into_sorted` (a `Vec<Diagnostic>`) and `into_result(value)` (`Ok(value)`
  when empty, else `Err(self)`). Sort order: by file path; within a file, diagnostics without a
  line first (ordered by key path), then by line ascending; ties keep insertion order.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** "all errors per run, sorted by file; within a file, key-path errors first, then by
line" (§10).

**Proof:**
- `diagnostic_displays_location_then_error` — an `Io` diagnostic for `a.md` displays as
  `a.md: <message>`.
- `diagnostics_sort_by_file_then_keyless_then_line` — pushed in order (`b.md:3`, `a.md:9`,
  `a.md` key `x`, `a.md:2`) come out as `a.md` key `x`, `a.md:2`, `a.md:9`, `b.md:3`.
- `diagnostics_keep_insertion_order_for_ties` — two diagnostics at `a.md:4` keep their order.
- `into_result_is_ok_only_when_empty` — empty gives `Ok`, one diagnostic gives `Err` holding it.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green;
`cargo make deny` passes (new dependency). Commit `feat(agentstacks): add sorted format
diagnostics`.

---

## Risks

- `FormatError` grows a variant per later plan; a plan that forgets its `#[from]` variant cannot
  push its errors. Each later plan names that edit explicitly.
- Doc comments must follow the doc-comment-discipline rule: no chain, plan or task identifiers in
  source.

## Proof

- `cargo make dod-crate agentstacks` green; `cargo make deny` green.
