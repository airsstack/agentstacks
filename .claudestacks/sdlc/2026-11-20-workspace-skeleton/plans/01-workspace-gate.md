---
status: approved
created: 2026-11-20
built-on: 5c6b28af8487
---

# Workspace Gate Implementation Plan

**Goal:** A clean checkout passes `cargo make dod` on a Cargo workspace whose first member is the
`agentstacks` library holding its six empty module seams.

**Architecture:** A virtual workspace at the repo root (`members = ["crates/*"]`, per the
`rust-guidelines` workspace reference, which wins over claudestacks' explicit member list) that
copies claudestacks' toolchain pin, formatting, lint policy and release profile. Its one member,
`crates/agentstacks`, is an export-only `lib.rs` declaring six doc-only seam modules. The gate
(`Makefile.toml`) and supply-chain policy (`deny.toml`) are copied from claudestacks, keeping only
the tasks and rules agentstacks uses. `publish = false` workspace-wide, because the intent says
nothing is published; the 0.x release chain flips it. No external dependency is added in this
plan; `airsl` is deliberately not added (the intent's "if it is added at all"; it arrives with the
extension host).

**Tech Stack:** Rust 1.94.1 (edition 2024), Cargo workspace (resolver 3), cargo-make, cargo-deny.

**Content authority:** `intent.md` (spec skipped) — Desired outcome bullets 1–4, Constraints.

---

## Files that change

```
Cargo.toml                                — create  workspace manifest: members, package metadata, lints, release profile
Cargo.lock                                — create  generated lockfile, committed (the workspace will ship a binary)
rust-toolchain.toml                       — create  exact toolchain pin 1.94.1
rustfmt.toml                              — create  edition 2024, max_width 100
crates/agentstacks/Cargo.toml             — create  library package manifest inheriting workspace metadata and lints
crates/agentstacks/src/lib.rs             — create  export-only crate root declaring the six seams
crates/agentstacks/src/model.rs           — create  doc-only seam
crates/agentstacks/src/validation.rs      — create  doc-only seam
crates/agentstacks/src/plan.rs            — create  doc-only seam
crates/agentstacks/src/executor.rs        — create  doc-only seam
crates/agentstacks/src/lock.rs            — create  doc-only seam
crates/agentstacks/src/extension_host.rs  — create  doc-only seam
Makefile.toml                             — create  cargo-make gate: dod, dod-crate, fmt, clippy-fix, deny
deny.toml                                 — create  cargo-deny policy
```

## Order of work

### Task 1 — Create the workspace root and the `agentstacks` library seams

**Files:**
- Create `Cargo.toml` — `[workspace]` with `resolver = "3"` and `members = ["crates/*"]`.
  `[workspace.package]`: `edition = "2024"`, `rust-version = "1.94"`, `license = "Apache-2.0"`
  (matches `LICENSE`, Apache 2.0), `repository = "https://github.com/airsstack/agentstacks"`, the
  same `authors` value as `../claudestacks/Cargo.toml`, and `publish = false`.
  `[workspace.lints.rust]` and `[workspace.lints.clippy]` copied verbatim from
  `../claudestacks/Cargo.toml:72-87` (`unsafe_code = "forbid"` at :73, `unwrap_used = "deny"` at
  :81, pedantic + nursery at :79-80). `[profile.release]` copied from
  `../claudestacks/Cargo.toml:89-92`; `[profile.dist]` (:94-97) is not copied (no dist).
  No `[workspace.dependencies]` entries yet.
- Create `rust-toolchain.toml` — `channel = "1.94.1"`, `components = ["rustfmt", "clippy"]`,
  `profile = "minimal"` (as `../claudestacks/rust-toolchain.toml:12-14`). Its comment states the
  pin's reason for this repo: it matches `rust-version` so the MSRV claim is exercised by every
  build and matches the org's other workspaces. The claudestacks comment about `trybuild`
  fixtures is not copied (no such fixtures here).
- Create `rustfmt.toml` — `edition = "2024"`, `max_width = 100` (as `../claudestacks/rustfmt.toml`).
- Create `crates/agentstacks/Cargo.toml` — `name = "agentstacks"`, `version = "0.0.0"`
  (unreleased; the 0.x release chain sets the first real version), a one-line `description`,
  `edition`/`rust-version`/`license`/`repository`/`authors`/`publish` all `.workspace = true`,
  `[lints] workspace = true`, no dependencies.
- Create `crates/agentstacks/src/lib.rs` — export-only per the `mod-rs-export-only` reference: a
  `//!` block (what the crate is: the agentstacks core library; why it is separate from the CLI;
  responsibilities: the six seams; non-responsibilities: argument parsing and terminal output
  belong to `agentstacks-cli`) followed by `pub mod model; pub mod validation; pub mod plan;
  pub mod executor; pub mod lock; pub mod extension_host;`. Nothing else.
- Create the six seam files `model.rs`, `validation.rs`, `plan.rs`, `executor.rs`, `lock.rs`,
  `extension_host.rs` under `crates/agentstacks/src/` — each contains only a `//!` block, no
  items. Each block names, in the present tense, what the module is the home of, taken from the
  existing docs and not extended: model — the pack, tool extension, operation and lock terms
  (`CLAUDE.md` "What agentstacks is" table); validation — validating every proposed operation
  (`CLAUDE.md` Invariants, first bullet); plan — showing every operation before apply
  (`README.md:25`, `CLAUDE.md` diagram); executor — the only writer to disk (`README.md:25`,
  `README.md:90`); lock — `agentstacks.lock`, pinned versions plus file hashes (`README.md:35`);
  extension_host — running Lua tool extensions under airsl with zero grants (`CLAUDE.md`
  diagram and Invariants). Wording satisfies both guideline rules that touch these blocks:
  it describes what exists now (the module holds no items) rather than narrating future work
  (`doc-comment-discipline.md`), and it states the test exemption in plain words — "export-only
  module with no executable logic, so it carries no inline tests" — without a rule name or
  number (`unit-test-mandate.md` requires the exemption to be visible; `doc-comment-discipline.md`
  rejects rule-numbered references). `lib.rs` follows the same wording rules.
- Create `Cargo.lock` — generated by the first build; committed.

**Change:** The repository becomes a buildable Cargo workspace with one library crate whose public
surface is six empty, documented modules.

**Proof:** No unit tests: every source file is export-only or doc-only and ships under
unit-test-mandate exemption #1, cited in its own `//!` block. The proof is the gate itself, run as
raw commands because `Makefile.toml` arrives in Task 2:
- `cargo metadata --format-version 1 > /dev/null` succeeds — the workspace graph resolves with
  the `crates/*` glob.
- The five Definition-of-Done commands from the `rust-guidelines` skill exit 0 with zero warnings;
  `RUSTDOCFLAGS="-D warnings" cargo doc` is what proves every seam carries its module docs
  (`missing_docs` is warn at workspace level, promoted by the flag).
- `rustup show active-toolchain` from the repo root names `1.94.1`.

**Done when:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features
-- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`,
`cargo test --workspace --all-targets --all-features` and `cargo test --workspace --all-features
--doc` all pass — the stack's Definition of Done is green. Commit
`build(repo): add cargo workspace and agentstacks library seams`.

### Task 2 — Add the cargo-make gate

**Files:**
- Create `Makefile.toml` — copied from `../claudestacks/Makefile.toml` keeping only: the header
  comment (claudestacks-specific lines such as `book` removed), `[config]` (lines 19-28:
  `default_to_workspace = false`, `skip_core_tasks = true`, `min_version`), `[env]`
  (`CARGO_TERM_COLOR`), the `--all-features` comment, the gate tasks `default`, `dod`,
  `fmt-check`, `clippy`, `doc` + `doc.env`, `test`, `test-doc` (lines 41-97), `deny` (lines
  99-108, kept outside `dod`), and the development tasks `fmt`, `clippy-fix`, `dod-crate` (lines
  590-627). The `dod-crate` description's example crate name becomes `agentstacks`. Every
  claudestacks-only task (`plugins*`, `claudevs-check`, `corpus-*`, `install-*`, `book*`) is
  left out.

**Change:** `cargo make dod`, `cargo make dod-crate <crate>`, `cargo make fmt` and
`cargo make deny` exist. The five gate steps are the `rust-guidelines` skill's Definition of Done
one for one (the skill's block, `SKILL.md` lines 17-23, matches `../claudestacks/Makefile.toml`
lines 49-97); where they would differ, the skill wins.

**Proof:** No Rust tests (configuration only). Checks, each run before and after the file exists:
- `cargo make dod` — fails before (no `Makefile.toml`), exits 0 after.
- `cargo make dod-crate agentstacks` exits 0.
- `cargo make fmt` leaves `git status --porcelain` unchanged on the formatted tree.
- `cargo make --list-all-steps` shows each kept task (`dod`, `fmt-check`, `clippy`, `doc`,
  `test`, `test-doc`, `deny`, `fmt`, `clippy-fix`, `dod-crate`), and
  `cargo make --list-all-steps | grep -E 'plugins|claudevs|corpus|install|book'` prints nothing.
  The listing is not compared exhaustively: cargo-make also prints its own built-ins (`init`,
  `end`, `empty`) and shows `default` only as an alias on the `dod` line.

**Done when:** `cargo make dod` passes and the stack's Definition of Done is green. Commit
`build(repo): add cargo-make definition-of-done gate`.

### Task 3 — Add the cargo-deny policy

**Files:**
- Create `deny.toml` — copied from `../claudestacks/deny.toml` sections `[graph]`, `[output]`,
  `[advisories]`, `[licenses]` (allow list and `confidence-threshold` as-is), `[licenses.private]`,
  `[bans]` (`multiple-versions = "deny"`, `wildcards = "deny"`), `[sources]` (crates.io only, no
  git). Every claudestacks-specific exception is dropped: the `[advisories] ignore` entry for
  `RUSTSEC-2024-0436` (`paste` via `ratatui`) and every `[bans] skip` entry (`syn@3.0.3`,
  `toml@0.8.23` and its three companions via `airsl`), because none of those crates is in this
  workspace's graph. Explanatory comments that name claudestacks crates are dropped with them.
  Comments that would be false here are rewritten for this repo, not copied:
  `../claudestacks/deny.toml:2` ("policy for the claudestacks workspace" → agentstacks);
  `:39` ("Every license actually present in the graph today" → the allow list is the org's
  permissive set, kept so the policy matches claudestacks); `:62` ("Workspace members are
  published to crates.io" → members are not published yet, and their Apache-2.0 licence is still
  checked); `:72-75` (the `skip` list "is the set that exists today" → the list is empty, so any
  duplicate fails until acknowledged).

**Change:** `cargo make deny` checks advisories, licences, duplicate versions and sources against a
policy identical to claudestacks' minus its exceptions. It stays outside `dod`.

**Proof:** No Rust tests (configuration only). Checks:
- `cargo make deny` exits 0 with `deny.toml` in place.
- `grep -nE 'RUSTSEC-2024-0436|syn@|toml@|toml_datetime@|serde_spanned@|winnow@' deny.toml`
  prints nothing — no copied exception survives.

**Done when:** `cargo make deny` passes and `cargo make dod` (the stack's Definition of Done) is
green. Commit `build(repo): add cargo-deny supply-chain policy`.

---

## Risks

- **Riskiest step: Task 1's empty seams under pedantic + nursery clippy and `missing_docs`.** A
  doc-only module is unusual; if a lint objects, the fix is in the doc block, never an `#[allow]`
  (an `#[expect]` with a reason only if genuinely unavoidable, per strict-quality).
- `members = ["crates/*"]` departs from claudestacks' explicit list; it picks up every directory
  under `crates/`, so a stray directory there breaks `cargo metadata`.
- `publish = false` and `version = "0.0.0"` must be revisited by the 0.x release chain; they are
  placeholders by intent ("nothing is published"), not decisions about release.
- Options not taken: copying `[profile.dist]` (no dist config by intent); adding `airsl` now (no
  Lua yet); keeping claudestacks' deny exceptions "just in case" (they would warn as unmatched).

## Proof

- From a fresh clone of the branch: `cargo make dod`, `cargo make dod-crate agentstacks` and
  `cargo make deny` all exit 0.
- `cargo tree -d` reports no duplicate versions (workspace reference's DoD for
  workspace-touching changes).
