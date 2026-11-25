---
status: done
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

## Review findings

- Task 1 (reviewer, handoff 20261125-174220-4854/19-reviewer-batchC.md): 0🔴, blocking none. `cargo make dod` exit 0, `cargo make claudevs-check` exit 0, `cargo make opencode-check` exit 0, `cargo deny check` exit 0 (advisories ok, bans ok, licenses ok, sources ok). The version grep prints nothing; the claudevs diff is empty; Cargo.lock changes only the two workspace crates. The open 🔵 and ❓ below are now fixed (fix pass, below).
- Fix pass. Final gates, each run once after the last edit: `cargo make dod` exit 0 (Rust: 2595 passed, 0 failed across 25 test binaries and doctests; lua 651 passed, fingerprint gate 8 cases, pack text gates 4 passed), `cargo make claudevs-check` exit 0, `cargo make opencode-check` exit 0 (S3 to S8b ok), `cargo deny check` exit 0 (advisories ok, bans ok, licenses ok, sources ok), `cargo make book-build` exit 0 with no `[ERROR]` line.
  - fixed: nothing failed if a pack or extension version went back to 0.x. `every_official_pack_and_extension_is_at_version_one_or_later` in crates/agentstacks-cli/tests/packs.rs reads every `packs/*/pack.toml` and `extensions/*/extension.toml` and asserts major >= 1 (it does not tie them to the crate version, and counts five packs and two extensions). Proof: `cargo test -p agentstacks-cli --test packs every_official` passed (1 passed); with `packs/core/pack.toml` set to `0.9.0` for one run it failed with `.../packs/core/pack.toml is below 1.0.0`, and the file was restored. The crate adds `toml` as a dev-dependency.
  - fixed: book/src/ch06-01-extensions.md showed the claude extension at `version = "0.1.0"`; it now reads `1.0.0`. The same example also said `api = 0`, which api 1 refuses as legacy; it now says `api = 1`. Proof: `cargo make book-build` exit 0, no `[ERROR]` line.
  - kept by design: `VERSION` drops pre-release parts of the package version, because semver has no const way to build them (spec F10). Nothing to change.

## Deviations

- 2026-11-25: The pinned official version was in crates/agentstacks/tests/opencode.rs:88 (`opencode_manifest_loads`), not packs.rs; updated to "1.0.0".
