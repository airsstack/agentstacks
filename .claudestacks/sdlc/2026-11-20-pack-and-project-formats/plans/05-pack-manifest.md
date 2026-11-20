---
status: draft
created: 2026-11-20
depends-on: [01, 02, 04]
built-on: e665589a43a9
---

# Pack Manifest Implementation Plan

**Goal:** Pack manifest files (`pack.toml`, `native/<tool>/hooks.toml`) parse into types that
enforce every file-local rule.

**Architecture:** `PackManifest::parse(file, text)` deserialises with `toml` into structs marked
`deny_unknown_fields`, then converts raw fields into validated types, collecting every refusal
as a `Diagnostic` located by key path (`hooks[2].grant`). Rules that need the pack's files
(`render`/`exclude`/`run` existence, own agents) belong to pack loading (plan 06); rules that need
other packs belong to the set check (plan 10). Hooks are an enum `Hook::{Message, Lua}` so fields
that do not apply to a kind cannot be represented; `HookEvent` and its `MatchValue` sets encode
§6.1; `Grants` holds `Policy`, grant paths, env names and exec names. Native hooks reuse the same
types with `event` and `match` as opaque strings (`NativeHook`). `OptionDecl` holds type, default
and description. `ManifestError` joins `FormatError`. Token-bearing hook fields (`text`, `args`,
`run`, grant paths) are scanned here, located by key.

**Tech Stack:** Rust 2024, `toml` 1, `semver` 1, `globset` 0.4, `serde` 1.

**Content authority:** spec §4 (`pack.toml`, §4.1 names, §4.2 field rules), §4.4 (option
declarations), §6 (hooks: §6.1 vocabulary, §6.2 field rules), §2.2 (native hooks schema), §3.4
(which manifest fields carry tokens).

---

## Files that change

```
Cargo.toml                                     — modify  add toml, semver, globset to [workspace.dependencies]
crates/agentstacks/Cargo.toml                  — modify  depend on toml, semver, globset
crates/agentstacks/src/model/mod.rs            — modify  declare and re-export
crates/agentstacks/src/model/manifest_error.rs — create  ManifestError (thiserror)
crates/agentstacks/src/model/pack_manifest.rs  — create  PackManifest::parse: [pack], requires, render, exclude, options, hooks
crates/agentstacks/src/model/requires.rs       — create  Requires { packs, airsl: Option<VersionReq>, binaries }
crates/agentstacks/src/model/option_decl.rs    — create  OptionDecl { type, default, description }, OptionType
crates/agentstacks/src/model/hook.rs           — create  Hook::{Message, Lua}, MessageHook, LuaHook
crates/agentstacks/src/model/hook_event.rs     — create  HookEvent + allowed MatchValue sets
crates/agentstacks/src/model/grants.rs         — create  Grants, Policy, EnvName, grant-path shape check
crates/agentstacks/src/model/native_hooks.rs   — create  NativeHooks::parse for native/<tool>/hooks.toml
crates/agentstacks/src/model/diagnostic.rs     — modify  FormatError gains a Manifest variant
```

## Order of work

### Task 1 — Parse `[pack]`, `[requires]`, `render` and `exclude`

**Files:**
- Create `manifest_error.rs`, `requires.rs`, `pack_manifest.rs`; modify `mod.rs`,
  `diagnostic.rs`, both `Cargo.toml` files.

**Change:** `[pack]`: `name` (`PackName`), `version` (`semver::Version`), `api` (must be 0),
`description` (not empty or whitespace-only). `[requires]`: `packs` (names, no duplicates, not
the pack itself), `airsl` (`semver::VersionReq`), `binaries` (names, no duplicates). `render`:
relative paths without `..`, not ending in `.lua` or `.sh`. `exclude`: globs compiled with
`globset`. Unknown keys anywhere refused with their key path. Every rule reports and the parse
continues, so one run lists all of them.

**Proof:**
- `pack_manifest_parses_the_spec_example` — the spec §4 example (without hooks) parses into the
  expected values.
- `pack_manifest_refuses_each_field_rule` — table-driven over §4.2 rows decidable from the file
  alone: unknown key (`hooks[2].grant: unknown key`), `name = "Core"`, `version = "1.2"`,
  `api = 1` (`sdlc: api 1 not supported (this agentstacks speaks 0)`), empty and whitespace-only
  `description`, `packs = ["core", "core"]`, `packs = ["sdlc"]` in pack `sdlc` (`a pack cannot
  require itself`), `binaries = ["git", "git"]`, `airsl = "nope"`, `render = ["x.lua"]`
  (`render: "x.lua" is code; code is installed verbatim`), `render = ["../x"]`, `exclude = ["["]`
  (`exclude: "[" is not a valid glob`); each asserts the exact message and key path.
- `pack_manifest_reports_all_errors_at_once` — a file with three bad fields yields three
  diagnostics.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green;
`cargo make deny` passes (new dependencies). Commit `feat(agentstacks): parse pack.toml
metadata`.

### Task 2 — Parse option declarations

**Files:**
- Create `option_decl.rs`; modify `pack_manifest.rs`, `manifest_error.rs`, `mod.rs`.

**Change:** `[options.<name>]` with `OptionName` keys; `type` ∈ {boolean, integer, string}
(integer is `i64`); optional `default` must match `type`; `description` required, not empty.

**Proof:**
- `option_decl_parses_each_type` — boolean, integer and string declarations with and without
  defaults.
- `option_decl_refuses_each_rule` — `type = "number"` (`options.keep.type: "number" (expected
  boolean, integer, string)`), `default = "20"` on an integer (`options.archive_keep.default:
  expects integer, got string`), empty `description` (`options.archive_keep.description: empty`),
  key `archive-keep` (not a valid option name).

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse pack option declarations`.

### Task 3 — Parse hooks

**Files:**
- Create `hook_event.rs`, `grants.rs`, `hook.rs`; modify `pack_manifest.rs`, `manifest_error.rs`,
  `mod.rs`.

**Change:** `[[hooks]]` entries per §6.1–§6.2: `kind` ∈ {message, lua}; `event` from the six
neutral events; `match` only where allowed, from that event's set, not empty, no duplicates,
omitted = all; `agents` only on `agent.stop` (names; existence is checked in plan 06); a
`message` hook needs non-empty `text` and refuses `run`, `args`, `grants`, `may_block`; a `lua`
hook needs `run` ending in `.lua`; `grants.policy` ∈ {confined, pure}, default confined,
`trusted` refused; grant paths absolute or starting with a `root`, `skill_dir` or `var` token;
`grants.env` names match `^[A-Z_][A-Z0-9_]*$`; `grants.exec` ⊆ `requires.binaries` ∪ {airsl};
`may_block` a boolean, default false. `text`, `args`, `run` and grant paths are scanned for
tokens, errors located by key path.

**Proof:**
- `hooks_parse_the_spec_examples` — both §6 example hooks parse; the lua hook's `run` and `args`
  carry their tokens.
- `hooks_refuse_each_field_rule` — table-driven over every §6.2 row: `kind = "sh"`,
  `event = "tool.pre"`, `match = ["bash"]` on `tool.before`, `match = []` (`empty list never
  fires`), `match = ["read", "read"]`, `match` on `session.end`, `agents` on `tool.before`, empty
  `text`, `may_block` on a message hook (`message hooks cannot block`), `run` on a message hook,
  lua hook without `run`, `run = "x.sh"`, `policy = "trusted"` (`"trusted" is not allowed for
  pack hooks`), `read = ["data"]` (`must be absolute or start with a token`), `env = ["home"]`,
  `exec = ["git"]` without `git` in `binaries`, `may_block = "yes"` (`expected true or false`);
  each asserts message and key path.
- `hooks_accept_airsl_in_exec_without_declaring_it` — `exec = ["airsl"]` with empty `binaries`
  parses.
- `hooks_report_token_errors_by_key` — `args = ["{{rot}}"]` gives
  `hooks[0].args[0]: unknown token kind "rot"`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse neutral pack hooks`.

### Task 4 — Parse native hook files

**Files:**
- Create `native_hooks.rs`; modify `manifest_error.rs`, `mod.rs`.

**Change:** `NativeHooks::parse(file, text, requires)` reads `[[hooks]]` with the §6.2 rules,
except that `event` is any non-empty string and `match` is any list of non-empty strings; grants,
policy, `may_block`, `exec ⊆ requires.binaries ∪ {airsl}` and token scanning apply unchanged.
Fail-open is not a field and cannot be set.

**Proof:**
- `native_hooks_accept_tool_specific_event_and_match` — `event = "PreToolUse"`,
  `match = ["SubagentHandback"]`, `may_block = true` parses.
- `native_hooks_keep_the_neutral_grant_rules` — `policy = "trusted"` and `exec = ["git"]` without
  the binary are refused with the §6.2 messages.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse native hook declarations`.

---

## Risks

- `toml` errors for unknown keys must be turned into key-path diagnostics; if the crate's
  `deny_unknown_fields` error does not expose the path, unknown keys are detected by a second pass
  over the raw `toml::Table`. Either way the test asserts the spec message.
- Collecting all errors means parsing into raw types first, then validating; a struct-direct
  parse would stop at the first error.

## Proof

- `cargo make dod-crate agentstacks` green; `cargo make deny` green; every file-local row of
  §4.2 and §6.2 has a case.
