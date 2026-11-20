---
status: draft
created: 2026-11-20
depends-on: [01, 02]
built-on: e665589a43a9
---

# Frontmatter Implementation Plan

**Goal:** Skill and agent frontmatter parses into strict, tool-neutral metadata.

**Architecture:** `frontmatter.rs` splits a Markdown file into its YAML block (between `---`
lines at the very top) and body, and parses the block with `serde-saphyr` into a struct with
`deny_unknown_fields`. YAML is parsed **before** token scanning; afterwards each string value
(`description`, `argument_hint`) is scanned with `scan`, and its token errors are located by
`<file>: <key>`. The neutral vocabulary is one enum per concept: `Invocation`, `ModelTier`,
`Effort`, `AgentTool`. `native` is a map from `ToolName` to `OpaqueValue`, a small enum (null,
bool, integer, float, string, list, map) with its own serde visitor, so no extra YAML value
dependency is needed. `SkillMeta::parse(file, text, expected_name)` and `AgentMeta::parse(…)`
take the directory or file-stem name so the name rule is checked here. `FrontmatterError` joins
`FormatError`.

**Tech Stack:** Rust 2024, `serde` 1 (derive), `serde-saphyr` 2
(`default-features = false, features = ["deserialize"]`, the same entry claudestacks uses).

**Content authority:** spec §5 (all rows), §3.4 (parse order, quoting hint, literal `name`), §1
(the YAML probe behind the quoting hint).

---

## Files that change

```
Cargo.toml                                    — modify  add serde and serde-saphyr to [workspace.dependencies]
crates/agentstacks/Cargo.toml                 — modify  depend on serde, serde-saphyr
crates/agentstacks/src/model/mod.rs           — modify  declare and re-export
crates/agentstacks/src/model/frontmatter.rs   — create  split ---block/body; YAML parse; quoting hint
crates/agentstacks/src/model/frontmatter_error.rs — create FrontmatterError (thiserror)
crates/agentstacks/src/model/opaque_value.rs  — create  OpaqueValue + its serde visitor
crates/agentstacks/src/model/invocation.rs    — create  Invocation: user | any (default any)
crates/agentstacks/src/model/model_tier.rs    — create  ModelTier: fast | balanced | deep
crates/agentstacks/src/model/effort.rs        — create  Effort: low | medium | high
crates/agentstacks/src/model/agent_tool.rs    — create  AgentTool: read write edit grep glob shell skill
crates/agentstacks/src/model/skill_meta.rs    — create  SkillMeta::parse
crates/agentstacks/src/model/agent_meta.rs    — create  AgentMeta::parse
crates/agentstacks/src/model/diagnostic.rs    — modify  FormatError gains a Frontmatter variant
```

## Order of work

### Task 1 — Split and parse the frontmatter block

**Files:**
- Create `frontmatter.rs`, `frontmatter_error.rs`, `opaque_value.rs`; modify `mod.rs`,
  `diagnostic.rs`, both `Cargo.toml` files.

**Change:** `split(text)` returns the YAML block and the body, or `missing frontmatter` when the
file does not start with a `---` line or the block is not closed. `parse_block<T>` runs
`serde-saphyr`, converting its error into `<file>:<line>: <parser message>`; when the offending
line holds `: {{`, the message gains ` (a value starting with {{ must be quoted)`. A YAML error
stops this file only. `OpaqueValue` accepts any YAML value.

**Proof:**
- `split_refuses_missing_or_unterminated_frontmatter` — a body with no `---`, a `---` on line 2,
  and an unclosed block each give `<file>: missing frontmatter`.
- `yaml_error_carries_line_and_quoting_hint` — `description: {{skill status}} runs it` gives an
  error on that line ending in `(a value starting with {{ must be quoted)`.
- `quoted_token_value_parses` — `description: "{{skill status}} runs it"` parses to the string
  with the token intact.
- `opaque_value_accepts_nested_yaml` — `native: { claude: { model: claude-opus-5-5, n: [1, null] } }`
  parses into nested `OpaqueValue`s.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green;
`cargo make deny` passes (new dependencies). Commit `feat(agentstacks): parse YAML frontmatter
blocks`.

### Task 2 — Parse skill metadata

**Files:**
- Create `invocation.rs`, `skill_meta.rs`; modify `frontmatter_error.rs`, `mod.rs`.

**Change:** `SkillMeta { name, description, invocation, argument_hint, native }`.
`deny_unknown_fields` gives `<file>: unknown key "<key>"`. `name` must be a literal (any `{{`
gives `<file>: name must be a literal`) and equal the expected directory name (`<file>: name "x"
does not match "y"`). `description` required and not empty or whitespace-only. `invocation` ∈
{user, any}, default `any`. `argument_hint` must be a string. `native` keys must be valid
`ToolName`s (`<file>: native: "Claude" is not a valid tool name`). After parsing, `description`
and `argument_hint` are scanned; token errors are located by key.

**Proof:**
- `skill_meta_parses_neutral_fields` — the spec §5 skill example parses with `invocation = user`.
- `skill_meta_defaults_invocation_to_any`.
- `skill_meta_refuses_each_rule` — table-driven over the §5 rows that apply to skills: unknown
  `disable-model-invocation`, empty `description`, token in `name`, name mismatch,
  `invocation: manual` (message lists `user, any`), `argument_hint: [a]` (`expected a string`),
  `native: { Claude: … }`; each asserts the exact message.
- `skill_meta_reports_token_errors_by_key` — `description: "run {{skil x}}"` gives
  `<file>: description: unknown token kind "skil"`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse skill frontmatter`.

### Task 3 — Parse agent metadata

**Files:**
- Create `model_tier.rs`, `effort.rs`, `agent_tool.rs`, `agent_meta.rs`; modify
  `frontmatter_error.rs`, `mod.rs`.

**Change:** `AgentMeta { name, description, model, effort, tools, native }` with the same name,
description, unknown-key, native and token rules as skills. `model` ∈ {fast, balanced, deep},
`effort` ∈ {low, medium, high}, `tools` ⊆ {read, write, edit, grep, glob, shell, skill} — all
optional; a duplicate tool is refused. Messages name the value and the allowed set.

**Proof:**
- `agent_meta_parses_the_spec_example` — the spec §5 agent example parses, including the
  `native.claude` override as an `OpaqueValue`.
- `agent_meta_allows_missing_optional_fields` — only `name` and `description` present parses.
- `agent_meta_refuses_each_rule` — table-driven: `model: sonnet` (`(expected fast, balanced,
  deep)`), `effort: max` (`(expected low, medium, high)`), `tools: [bash]` (`tools: unknown
  "bash"`), `tools: [read, read]` (`tools: "read" listed twice`), unknown key, empty description,
  name mismatch; each asserts the exact message.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): parse agent frontmatter`.

---

## Risks

- `serde-saphyr` error positions are relied on for `<file>:<line>`; if a parser error carries no
  line, the diagnostic falls back to `<file>` and the test asserts the line only where the parser
  provides one. Record any such case under the plan's deviations.
- `deny_unknown_fields` with a renamed field (`argument_hint`) must still refuse the Claude
  spelling `argument-hint`; the unknown-key test covers it.

## Proof

- `cargo make dod-crate agentstacks` green; `cargo make deny` green; every §5 row has a case.
