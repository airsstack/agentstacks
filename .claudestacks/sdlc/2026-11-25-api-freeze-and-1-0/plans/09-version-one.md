---
status: approved
created: 2026-11-25
depends-on: [01]
built-on: c187d693fbea
---

# Version 1.0.0 Implementation Plan

**Goal:** Every agentstacks crate, official pack and official extension is version 1.0.0.

**Architecture:** Only version strings change. This runs right after plan 01 and before every plan
that uses `VERSION` (02, then 06), because the conformance pack and the tests carry
`requires.agentstacks = ">=1.0"`, which would exclude a running 0.2.0. claudevs is untouched (D14).

**Tech Stack:** Cargo manifests, TOML.

**Content authority:** spec §8 (Versions), D8, D9, D14, F13.

---

## Files that change

```
crates/agentstacks/Cargo.toml          — modify: version 1.0.0
crates/agentstacks-cli/Cargo.toml      — modify: version 1.0.0
Cargo.toml                             — modify: workspace dependency agentstacks version 1.0.0
Cargo.lock                             — modify: the two versions (by cargo)
packs/{core,sdlc,journal,guideline-rust,guideline-python}/pack.toml — modify: version 1.0.0
extensions/{claude,opencode}/extension.toml — modify: version 1.0.0
```

## Order of work

### Task 1 — Versions 1.0.0

**Files:**
- Modify `crates/agentstacks/Cargo.toml`, `crates/agentstacks-cli/Cargo.toml` — `version = "1.0.0"`
- Modify `Cargo.toml` — `agentstacks = { path = "crates/agentstacks", version = "1.0.0" }`
- Modify `Cargo.lock` — updated by the first cargo run
- Modify `packs/{core,sdlc,journal,guideline-rust,guideline-python}/pack.toml`, `extensions/{claude,opencode}/extension.toml` — `version = "1.0.0"` (D9)
- Modify any test or expected fixture that asserts an official pack's or extension's old version string (found by the suites)

**Change:** Every agentstacks version is 1.0.0. Test fixture packs keep their own versions (D9 moves only the official ones).

**Proof:**
- the existing `crates/agentstacks-cli/tests/version.rs` (`agentstacks {CARGO_PKG_VERSION}`) passes and now prints `agentstacks 1.0.0`.
- `git grep -nE '^version *= *"0\.' -- 'packs/*/pack.toml' 'extensions/*/extension.toml' crates/agentstacks/Cargo.toml crates/agentstacks-cli/Cargo.toml` prints nothing.
- `git diff --stat -- crates/claudevs crates/claudevs-cli` is empty.

**Done when:** `cargo make dod`, `cargo make claudevs-check`, `cargo make opencode-check` and `cargo deny check` pass. Commit `chore(repo): version 1.0.0`.

---

## Risks

- `cargo deny check` sees the new versions of the workspace's own crates. No new crate enters (spec §10), so a red result is a real problem to report, not to retry.
- A test that pins an official pack's version (for example in `packs.rs` or an expected plan text) turns red. Task 1 updates it in the same commit.

## Proof

- `cargo make dod`, `cargo make claudevs-check`, `cargo make opencode-check` and `cargo deny check` are green.
