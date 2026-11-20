---
status: draft
created: 2026-11-20
depends-on: [01, 02, 03]
built-on: e665589a43a9
---

# Extension Manifest Implementation Plan

**Goal:** `Extension::load(dir)` turns an extension directory into a checked `Extension` or every
refusal it contains.

**Architecture:** `extension_manifest.rs` parses `extension.toml` (`[extension]` with `name`,
`version`, `api`, `description`, `entry`) with `deny_unknown_fields`, collecting every refusal.
`extension.rs` walks the directory with `walk` (same symlink, UTF-8 and dot-entry rules as packs)
and checks that `entry` exists among the walked files as a `.lua` file. What the adapter exports
and how it runs is not here. `ExtensionError` joins `FormatError`.

**Tech Stack:** Rust 2024, `toml` 1, `semver` 1 (both from plan 05).

**Content authority:** spec §4.5.

---

## Files that change

```
crates/agentstacks/src/model/mod.rs                — modify  declare and re-export
crates/agentstacks/src/model/extension_manifest.rs — create  ExtensionManifest::parse
crates/agentstacks/src/model/extension.rs          — create  Extension, Extension::load
crates/agentstacks/src/model/extension_error.rs    — create  ExtensionError (thiserror)
crates/agentstacks/src/model/diagnostic.rs         — modify  FormatError gains an Extension variant
crates/agentstacks/tests/extension_load.rs         — create  integration test over the fixtures
crates/agentstacks/tests/fixtures/extensions/**    — create  one valid extension, one invalid per refusal
```

## Order of work

### Task 1 — Parse `extension.toml`

**Files:**
- Create `extension_manifest.rs`, `extension_error.rs`; modify `mod.rs`, `diagnostic.rs`.

**Change:** `name` (`ToolName`), `version` (semver), `api` (must be 0), `description` (not empty),
`entry` (relative, no `..`, ends in `.lua`); unknown keys refused with key path; all errors
reported together. Messages follow §4.2's shapes prefixed `extension.`.

**Proof:**
- `extension_manifest_parses_the_spec_example`.
- `extension_manifest_refuses_each_rule` — table-driven: `name = "Claude"`, `version = "1"`,
  `api = 1` (`api 1 not supported (this agentstacks speaks 0)`), empty `description`,
  `entry = "../a.lua"`, `entry = "adapter.js"`, unknown key `extension.foo`
  (`extension.toml: extension.foo: unknown key`); exact messages.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse extension manifests`.

### Task 2 — Load an extension directory

**Files:**
- Create `extension.rs`, `crates/agentstacks/tests/extension_load.rs`, fixtures; modify
  `extension_error.rs`, `mod.rs`.

**Change:** `Extension::load(dir)`: missing `extension.toml` → `<dir>: not an extension (no
extension.toml)`; `entry` not among the walked files → `extension.entry: "adapter.lua" does not
exist`; walk refusals as for packs; dot entries ignored and kept for `plan`.

**Proof:**
- `extension_load_accepts_a_minimal_extension` — `extension.toml` + `adapter.lua`.
- `extension_load_refuses_each_rule` — fixtures: no manifest, missing entry, a symlink (unix
  only); exact messages.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): load extension directories`.

---

## Risks

- The extension name matching its `[tools.<name>]` key, and the extension's tree hash, are not
  checked here: the key match is plan 10, the hash is `tree_hash` (plan 03).

## Proof

- `cargo make dod-crate agentstacks` green; every §4.5 row has a case.
