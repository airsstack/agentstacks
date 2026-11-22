---
status: done
created: 2026-11-22
depends-on: [01]
built-on: cefc42fbbc4c
---

# Testing feature Implementation Plan

**Goal:** Let dependent crates reuse the library's port fakes through a `testing` feature.

**Architecture:**
- The fakes stay where they are (`src/port/fake.rs`, `src/port/{adapter,binary,git,workspace}/fake.rs`).
- Only their gate changes, from `#[cfg(test)]` to `#[cfg(any(test, feature = "testing"))]`.
- A dev-dependency feature stays off in a normal or release build of a dependent binary
  (`probes/redesign-results.md`, G2).
- The CLI turns it on in plan 06.

**Tech Stack:** Rust 2024, Cargo features (resolver 3).

**Content authority:** spec D20; §2 (the `port/**/fake` line, the `testing` feature row); §12 (the
`testing` feature bullet); §13 (the `testing` row).

---

## Files that change

```
crates/agentstacks/Cargo.toml                       — modify: [features] testing = []
crates/agentstacks/src/port/mod.rs                  — modify: fake module and re-exports gated cfg(any(test, feature = "testing"))
crates/agentstacks/src/port/adapter/mod.rs          — modify: same gate change
crates/agentstacks/src/port/binary/mod.rs           — modify: same gate change
crates/agentstacks/src/port/git/mod.rs              — modify: same gate change
crates/agentstacks/src/port/workspace/mod.rs        — modify: same gate change
```

## Order of work

### Task 1 — Gate the fakes on `testing`

**Files:**
- Modify `crates/agentstacks/Cargo.toml` — `[features] testing = []`.
- Modify `crates/agentstacks/src/port/mod.rs` (`:29-81`) and the four `port/*/mod.rs` files:
  - every `#[cfg(test)]` that gates `mod fake`, `pub mod fake` or a re-export of `Fake*`, `*Call`,
    `ScriptStep` or `ToolScript` becomes `#[cfg(any(test, feature = "testing"))]`;
  - the fakes' own `#[cfg(test)] mod tests` blocks stay unchanged;
  - module docs say the fakes are available under the `testing` feature.

**Change:** Dependent crates can enable `testing` and use `FakeWorkspace`, `FakeGit`, `FakeBinary`,
`FakeAdapter` and `FakeFileTree`. On these exact files, clippy `-D warnings` and rustdoc
`-D warnings` exit 0 under `--features testing`, and rustdoc lists the five fakes
(`probes/redesign-results.md`, G2b).

**Proof:**
- `cargo clippy -p agentstacks --lib --features testing -- -D warnings` exits 0.
- The gate checks the fakes outside `cfg(test)`: `cargo make dod` runs clippy and rustdoc with
  `--all-features`, which turns on `testing`.
- `cargo test -p agentstacks` stays green, so the fakes still serve the library's own tests.

**Done when:** `cargo clippy -p agentstacks --lib --features testing -- -D warnings` passes and
`cargo make dod` is green. Commit `feat(agentstacks): expose the port fakes behind a testing feature`.

---

## Risks

- **Rule-of-the-gate comment.** The rust guideline notes that no crate declares `[features]` today.
  This plan adds the first feature, which is the case `--all-features` in the gate was kept for.
- **Isolation test.** The fakes become part of the library build when `testing` is on.
  `tests/isolation.rs` must stay green: the fakes import only `std` and crate items (spec §2).

## Proof

- `cargo make dod` green.
- `cargo clippy -p agentstacks --lib --features testing -- -D warnings` green.

## Review findings

- 🟡 five `fake.rs` module docs still said "It is compiled for tests only." (blocking). Fixed inline: "It is compiled for tests and under the `testing` feature." in `port/fake.rs`, `port/{adapter,binary,git,workspace}/fake.rs`. `cargo make dod` → exit 0.
- 🔵 `Makefile.toml:33` still said no crate declares features. Fixed in the same pass.

## Deviations

- 2026-11-22: `Makefile.toml` comment and the five `fake.rs` docs changed outside the file list (the plan's Risks named the Makefile comment).
- 2026-11-22 (process): coders read their task sections straight from this plan file instead of a task-briefer brief; the ledger came from `grep '^### Task'`. Earlier batches were staged in the git index (not committed) so each review saw only its own batch.
