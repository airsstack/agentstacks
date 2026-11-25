---
status: approved
created: 2026-11-25
depends-on: [01, 02, 03]
built-on: 63f62977694c
---

# Pack Release Record Implementation Plan

**Goal:** The pack manifests and the packs changelog carry the versions this change earns.

**Architecture:** Three `pack.toml` version bumps and one `### Changed` section in
`packs/CHANGELOG.md`. The guideline packs keep 0.1.0 (only their excluded README changed); the
opencode extension keeps 0.1.0 (unreleased). Depends on plans 01-03, whose changes the entries
describe.

**Tech Stack:** TOML, Markdown.

**Content authority:** spec D7, §9 (versions, changelog).

---

## Files that change

```
packs/core/pack.toml      — modify: version 0.1.1
packs/sdlc/pack.toml      — modify: version 0.1.1
packs/journal/pack.toml   — modify: version 0.2.0
packs/CHANGELOG.md        — modify: ### Changed under ## [Unreleased]
```

## Order of work

### Task 1 — Bump the pack versions and record the change

**Files:** Modify `packs/core/pack.toml`, `packs/sdlc/pack.toml`, `packs/journal/pack.toml`,
`packs/CHANGELOG.md`.

**Change:**
- `[pack] version`: `core` `"0.1.0"` → `"0.1.1"`, `sdlc` `"0.1.0"` → `"0.1.1"`, `journal`
  `"0.1.0"` → `"0.2.0"`. Nothing else in the manifests changes; `packs/guideline-*/pack.toml` and
  `extensions/opencode/extension.toml` keep `0.1.0`.
- `packs/CHANGELOG.md`, under `## [Unreleased]`, after the `### Added` list: a `### Changed`
  heading and the three bullets of spec §9, verbatim (the `journal` 0.2.0 bullet, the `core`
  0.1.1 / `sdlc` 0.1.1 / `journal` wording bullet, and the `opencode` extension bullet). No bullet
  backticks a tool's tool name, so the changelog needs no gate exception beyond the one it has.

**Proof:**
- `grep -n '^version' packs/{core,sdlc,journal,guideline-rust,guideline-python}/pack.toml` prints
  `0.1.1`, `0.1.1`, `0.2.0`, `0.1.0`, `0.1.0`.
- `grep -n '^### Changed' packs/CHANGELOG.md` prints one line, after the `### Added` line of
  `## [Unreleased]`.
- `five_packs_install` and `five_packs_install_on_both_tools` in `packs.rs` still pass (the
  versions are read from the manifests, not asserted).

**Done when:** `cargo test -p agentstacks-cli --test packs` and `cargo make lua` pass, `cargo make
book-build` prints no `[ERROR]`, and `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`chore(repo): bump core, sdlc and journal pack versions`.

## Risks

- A test or fixture that pins an official pack's version would fail; none was found
  (`grep -rln '0\.1\.0' crates/agentstacks-cli/tests` lists only the demo fixture and the
  opencode-check pack, neither an official pack).
- Not done here: a release tag or publish (`RELEASING.md`, the author's act).

## Proof

- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` green.
