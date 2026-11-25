---
status: done
created: 2026-11-25
depends-on: [01, 09]
built-on: c187d693fbea
---

# requires.agentstacks Implementation Plan

**Goal:** A pack or extension can require a range of agentstacks versions.

**Architecture:** Today's `AirslReq` becomes `VersionRange` in `types/manifest/range.rs`, the one
range type both `requires.airsl` and `requires.agentstacks` hold (modularity Rule 2). `types/api.rs`
gains `VERSION`, a `const semver::Version` built from `CARGO_PKG_VERSION_*` (F10). `Pack::load` and
`Extension::load` check an excluding range before anything else and return that one refusal
(§4.2), so the rule is pure and stays in `types/`.

**Tech Stack:** Rust (agentstacks crate), semver 1.0.28, toml 1.2.0.

**Content authority:** spec §4.1, §4.2, §4.3, D3, F1, F4, F8, F10, E6, E7, E8, §12 rows `types/manifest/range.rs`, `types/pack/load.rs`, `types/extension/loaded.rs`, `types/extension/manifest.rs`.

---

## Files that change

```
crates/agentstacks/src/types/manifest/airsl.rs      — rename: becomes range.rs
crates/agentstacks/src/types/manifest/range.rs      — create (from airsl.rs): VersionRange, InvalidRange, same messages
crates/agentstacks/src/types/manifest/mod.rs        — modify: exports VersionRange, InvalidRange
crates/agentstacks/src/types/manifest/error.rs      — modify: InvalidAirslReq → InvalidRange wherever it is held
crates/agentstacks/src/types/manifest/requires.rs   — modify: agentstacks: Option<VersionRange>; airsl holds VersionRange
crates/agentstacks/src/types/manifest/pack/file.rs  — modify: parses requires.agentstacks
crates/agentstacks/src/types/set/airsl.rs, set/interval.rs, set/error.rs — modify: take VersionRange (rename only)
crates/agentstacks/src/types/manifest/invalid.rs, types/op/airsl.rs, types/pack/error.rs, types/pack/install/checks.rs — modify: AirslReq/InvalidAirslReq users renamed
crates/agentstacks/src/types/mod.rs                 — modify: re-exports (VersionRange, VERSION)
crates/agentstacks/src/types/api.rs                 — modify: VERSION
crates/agentstacks/src/types/pack/load.rs           — modify: excluding range checked first
crates/agentstacks/src/types/extension/manifest.rs  — modify: [requires] table with agentstacks only
crates/agentstacks/src/types/extension/loaded.rs    — modify: excluding range checked first
```

## Order of work

### Task 1 — One range type

**Files:**
- Create `crates/agentstacks/src/types/manifest/range.rs` from `airsl.rs` — `VersionRange` and `InvalidRange`; same eight-operator rule, same `Display` as written, same messages; the module doc says both `requires.airsl` and `requires.agentstacks` hold it
- Modify `crates/agentstacks/src/types/manifest/{mod,error,requires,invalid}.rs`, `crates/agentstacks/src/types/set/{airsl,interval,error}.rs`, `crates/agentstacks/src/types/op/airsl.rs`, `crates/agentstacks/src/types/pack/{error,install/checks}.rs`, `crates/agentstacks/src/types/mod.rs` — every user of `AirslReq`/`InvalidAirslReq` (`git grep -ln AirslReq -- crates`) uses the new names

**Change:** A rename with no behaviour change.

**Proof:**
- today's `AirslReq` tests, moved to `range.rs` and renamed (e.g. `version_range_keeps_text_as_written`, `version_range_refuses_nope`), pass unchanged in assertion.

**Done when:** `cargo make dod` passes. Commit `refactor(agentstacks): one VersionRange for every version range`.

### Task 2 — VERSION

**Files:**
- Modify `crates/agentstacks/src/types/api.rs` — `VERSION`, built as F10 (a `const fn` over `u64::from_str_radix` mapping `Err` to 0, then `Version::new`); doc says it is the product version (D8)

**Change:** This agentstacks knows its own version as a semver value, with no `unwrap` or `panic`.

**Proof:**
- `version_is_the_package_version` — `VERSION.to_string()` equals `env!("CARGO_PKG_VERSION")`.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): VERSION, the running agentstacks as semver`.

### Task 3 — requires.agentstacks in pack.toml and extension.toml

**Files:**
- Modify `crates/agentstacks/src/types/manifest/requires.rs` — optional `agentstacks: Option<VersionRange>`, with `agentstacks()` accessor; absent means any (E8)
- Modify `crates/agentstacks/src/types/manifest/pack/file.rs` — reads `requires.agentstacks` (§4.3 rows 3–5 at `requires.agentstacks`)
- Modify `crates/agentstacks/src/types/extension/manifest.rs` — an optional top-level `[requires]` table with only `agentstacks`; unknown key `requires.foo: unknown key`; not a table `requires: expected table, got string` (§4.3 rows 6–7)

**Change:** Both manifests carry the range; a bad one is one refusal among the file's others (§4.2 "Otherwise as today").

**Proof:**
- `requires_agentstacks_absent_accepts` — no key gives `None`.
- `agentstacks_range_messages_match_airsl` — `""`, `"   "`, `"nope"` give `requires.agentstacks: "<text>" is not a version range`; `1` gives `requires.agentstacks: expected string, got integer`.
- `extension_requires_unknown_key` — `ext/mytool/extension.toml: requires.foo: unknown key`.
- `extension_requires_not_a_table` — `ext/mytool/extension.toml: requires: expected table, got string`.
- `spec_examples_parse` — the two §4.1 examples parse as manifests (the `pack.toml` and `extension.toml` parsers, not `Pack::load`) with the range `>=1.0`.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): requires.agentstacks in pack.toml and extension.toml`.

### Task 4 — An excluding range is the only refusal

**Files:**
- Modify `crates/agentstacks/src/types/pack/load.rs` — before the walk refusals, layout, frontmatter and bodies: read `pack.toml` from the tree; when readable, valid TOML, and `requires.agentstacks` parses and does not match `VERSION`, return only `<label>/pack.toml: requires.agentstacks: "<range>" does not match agentstacks <VERSION> (install a newer agentstacks)`; module doc states the order
- Modify `crates/agentstacks/src/types/extension/loaded.rs` — the same, before `walk(tree)`'s refusals, for `extension.toml`

**Change:** §4.2 "Checked first": a too-new pack or extension says only what to do.

**Proof:**
- `excluding_range_is_the_only_refusal` (pack) — a pack with `agentstacks = ">=99"`, an unknown key, `api = 2`, an unknown token kind in `SKILL.md` and a symlink yields exactly one diagnostic, the §4.3 row 1 text with `VERSION`.
- `excluding_range_is_the_only_refusal` (extension) — an extension with `agentstacks = ">=99"`, a walk refusal and an unknown key yields exactly the §4.3 row 2 text.
- `invalid_range_is_one_refusal_among_others` — `agentstacks = "nope"` plus an unknown key yields both refusals.
- `absent_range_loads` — a valid pack and extension with no range load.
- `matching_range_loads` — `">=1.0"` loads at `VERSION` 1.0.0 (plan 09 set it; F1).
- `unreadable_or_invalid_toml_goes_on_as_today` — a `pack.toml` that is not TOML gives today's refusals.

**Done when:** `cargo make dod` passes. Commit `feat(agentstacks): an excluding requires.agentstacks is the only refusal`.

---

## Risks

- The rename touches every holder of `AirslReq`, including `set/` intersection code. Task 1 is behaviour-free and kept apart so a failure is a missed rename, not logic.
- Task 4 reorders the start of both loaders. Today's tests on walk and layout refusals must keep passing for packs without an excluding range. They are the regression net.
- `VERSION` is 1.0.0 because plan 09 runs first (depends-on). Without it, every `>=1.0` in a test would exclude the running version.
- The unsupported-operator row of §4.3 is not tested at `requires.agentstacks`: semver 1.0.28 parses no operator outside the eight (the formats spec §4.3), so no input reaches it. The shared `VersionRange` keeps its existing coverage.

## Proof

- `cargo make dod` is green.
- `git grep -n AirslReq -- crates` prints nothing.

## Review findings

- Task 1 (reviewer, handoff 20261125-174220-4854/22-reviewer-batchD.md): 0🔴, blocking none; batch gate `cargo make dod` exit 0. The rename changes no behaviour; `git grep -nw AirslReq -- crates` prints nothing. Open 🔵 (now fixed, fix pass below): the ManifestError::Range doc says the same thing twice; manifest/mod.rs:12, range.rs:19 and set/interval.rs:22 doc lines run over 100 columns; the doctest `use` lists in set/{airsl,error}.rs are no longer alphabetical.

- Tasks 2 and 3 (reviewer, handoff 20261125-174220-4854/25-reviewer-batchE.md): 0🔴 0🟡, blocking none. `cargo make dod` exit 0 (coder run with both tasks in place, lib 1728 passed); reviewer `cargo test -p agentstacks --lib -- types::api types::manifest types::extension` 129 passed. The `spec_examples_parse` texts carry `agentstacks = ">=1.0"` as spec §4.1 does (checked by grep). Open 🔵 (now fixed, fix pass below): the `component` doc says every `CARGO_PKG_VERSION_*` is a number (PRE is not); VERSION drops pre-release parts, as designed (kept by design, see below); a bad extension `requires.agentstacks` is a ManifestError, not an ExtensionError; `read_range` would sit better beside VersionRange in range.rs; manifest/mod.rs:11 doc line is 115 columns.

- Task 4 (reviewer, handoff 20261125-174220-4854/27-reviewer-batchF.md): 0🔴, blocking none. The refusal text is exact, located at the manifest file; an excluding range is the only refusal in both loaders; an invalid range stays one among others. Final workspace gate: `cargo make dod` exit 0, `cargo make claudevs-check` exit 0, `cargo make opencode-check` exit 0, `cargo deny check` exit 0, `git grep -nw AirslReq -- crates` prints nothing.
- Cross-check over plans 01/09/03/02 (same report): 3🟡, fixed in one pass (handoff 28-coder-fix.md), `cargo make dod` exit 0 ("651 passed, 0 failed", "fingerprint gate: 8 cases passed", "pack text gates: 4 passed"):
  - fixed: the too-new refusal was `InvalidRange::ExcludesVersion` on the public `VersionRange` error; it is now `ManifestError::TooNew { range: Box<VersionRange>, version: semver::Version }`, same text; `excluded_by_agentstacks` renamed `agentstacks_too_new`.
  - fixed: pack.toml was read twice (range check, then parse); now read once in `Run::begin` and both use that text.
  - fixed: `read_api` duplicated in lock/file.rs and project/file.rs; now one `types::api::read` returning `Option<ApiCheck>`.
  - fixed: redundant test import; doc lines over 100 columns in manifest/mod.rs, project/mod.rs, project/upgrade.rs, manifest/range.rs, set/interval.rs.
  - open 🔵 (now fixed, fix pass below): `PROJECT_FILE` is copied in several modules; the legacy `0` is a literal in phrase.rs and show/update.rs; `requires_agentstacks()` (extension) and `Requires::agentstacks()` (pack) differ in shape; the `..._goes_on_as_today` tests have no I/O-failure case.
- Fix pass. Final gates, each run once after the last edit: `cargo make dod` exit 0 (Rust: 2595 passed, 0 failed across 25 test binaries and doctests; lua 651 passed, fingerprint gate 8 cases, pack text gates 4 passed), `cargo make claudevs-check` exit 0, `cargo make opencode-check` exit 0 (S3 to S8b ok), `cargo deny check` exit 0 (advisories ok, bans ok, licenses ok, sources ok), `cargo make book-build` exit 0 with no `[ERROR]` line.
  - fixed: the `ManifestError::Range` doc said the same thing twice; it now says the value is not a version range or its operator is not supported.
  - fixed: the doctest `use` lists in set/airsl.rs and set/error.rs are alphabetical.
  - fixed: the `component` doc names only `CARGO_PKG_VERSION_MAJOR`, `_MINOR` and `_PATCH`; the unit test `component_reads_a_number_and_falls_back_to_zero` covers the `Err => 0` arm.
  - fixed: a bad `requires.agentstacks` in extension.toml is `ExtensionError::Range(InvalidRange)`, transparent, so the message is unchanged. Proof: `extension_requires_bad_range_is_an_extension_error` and the existing `extension_requires_refuses_a_bad_agentstacks_range` pass under `cargo make dod`.
  - fixed: `read_range` is `VersionRange::read::<E>` in manifest/range.rs, generic over the file's error type (`ManifestError` for a pack, `ExtensionError` for an extension).
  - fixed: `spec_examples_parse` is `documented_examples_parse` in extension/manifest.rs and manifest/pack/file.rs. Proof: `git grep -n spec_examples_parse -- crates` prints nothing.
  - fixed: `ExtensionManifest::requires_agentstacks()` is `ExtensionManifest::requires().agentstacks()`, on a new `ExtensionRequires`, the same shape as the pack's `Requires::agentstacks()`.
  - fixed: the extension loader checks `requires.agentstacks` before hashing any file, as the pack loader does, and the excluding range is still the only refusal (`excluding_range_wins_before_an_unreadable_file_is_read` shows an unreadable file is not even reported). The manifest is read once for that check and once more with the hashed files, so its parse text still comes from the hashed read.
  - fixed: the `..._goes_on_as_today` tests are `unreadable_or_invalid_toml_is_refused_by_the_parse_not_the_range_check`, with an I/O-failure case in both loaders and a non-UTF-8 case in the extension one (the pack one already had it).
  - fixed: `PROJECT_FILE` and the legacy `0` duplication: see plans 03 and 01.
  - fixed: doc lines over 100 columns: see plan 03.
  - kept by design: `VERSION` drops pre-release parts of the package version, because semver has no const way to build them (spec F10).

## Deviations

- 2026-11-25: `InvalidRange` stays in `manifest/invalid.rs` with the other `Invalid*` types (it never lived in airsl.rs); range.rs holds `VersionRange` only. `ManifestError::AirslReq` is renamed `ManifestError::Range`.
- 2026-11-25: The proof grep `git grep -n AirslReq -- crates` matches `PackError::AirslRequired` as a substring (pack/error.rs:97,192; pack/install/checks.rs:48). The word-bounded `git grep -nw AirslReq -- crates` prints nothing, and that is the check used.
- 2026-11-25: The extension accessor was first `ExtensionManifest::requires_agentstacks() -> Option<&VersionRange>` (the brief named none); the fix pass made it `ExtensionManifest::requires().agentstacks()`, like the pack side's `Requires::agentstacks()`. The shared helper that reads both `airsl` and `agentstacks` is `VersionRange::read` in manifest/range.rs.
- 2026-11-25: The "too new" refusal is a `ManifestError::TooNew` variant (range boxed to keep `Diagnostic` small under clippy `result_large_err`), checked by `manifest::requires::agentstacks_too_new`; the pack loader reads pack.toml once and checks the range before any other refusal.
