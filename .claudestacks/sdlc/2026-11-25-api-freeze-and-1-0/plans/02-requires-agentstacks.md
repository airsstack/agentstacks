---
status: approved
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
