---
status: draft
created: 2026-11-20
depends-on: [01]
built-on: e665589a43a9
---

# Token Grammar Implementation Plan

**Goal:** One token module implements the spec's token grammar over pack text, names included.

**Architecture:** Names are newtypes that parse on construction (parse, don't validate):
`PackName`, `ToolName`, `ItemName` share one private kebab rule, and `OptionName` has its own
snake rule. The token module is split by item: `token.rs` (the `Token` value: kind, target, line,
byte span), `token_kind.rs`, `token_target.rs` (the parsed target per kind), `var.rs` (the fixed
var list), `scan.rs` (`scan(text) → (tokens, errors)`; never stops at the first error) and
`render.rs` (`render(text, tokens, values)` splices values and turns every `\{{` into `{{`).
Scanning knows nothing about packs: cross-pack target rules, var placement and option existence
are checked later, where the pack is known. `TokenError` is this module's `thiserror` enum and
joins `FormatError`.

**Tech Stack:** Rust 2024, `thiserror` 2 (already added by plan 01).

**Content authority:** spec §3.1 (grammar, var list, escape, accepted limitation), §3.2 (checked
examples), §4.1 (names).

---

## Files that change

```
crates/agentstacks/src/model/mod.rs          — modify  declare and re-export the new items
crates/agentstacks/src/model/name.rs         — create  PackName, ToolName, ItemName (kebab rule)
crates/agentstacks/src/model/option_name.rs  — create  OptionName (snake rule)
crates/agentstacks/src/model/token_kind.rs   — create  TokenKind: skill, agent, skill_dir, root, var, option
crates/agentstacks/src/model/var.rs          — create  Var: the six fixed var names
crates/agentstacks/src/model/token_target.rs — create  TokenTarget: ItemRef{pack?, item}, Pack(Option), Var, Option
crates/agentstacks/src/model/token.rs        — create  Token { kind, target, line, span }
crates/agentstacks/src/model/token_error.rs  — create  TokenError (thiserror)
crates/agentstacks/src/model/scan.rs         — create  scan(): text → tokens + errors
crates/agentstacks/src/model/render.rs       — create  render(): splice values, unescape \{{
crates/agentstacks/src/model/diagnostic.rs   — modify  FormatError gains a Token variant
```

## Order of work

### Task 1 — Add the name newtypes

**Files:**
- Create `crates/agentstacks/src/model/name.rs` — `PackName`, `ToolName`, `ItemName`, each a
  newtype over `String` built through `FromStr`/`TryFrom<&str>` that applies
  `^[a-z][a-z0-9-]{0,63}$`; a private helper holds the rule once. `Display`, `AsRef<str>`, `Ord`.
- Create `crates/agentstacks/src/model/option_name.rs` — `OptionName` with
  `^[a-z][a-z0-9_]{0,63}$`.
- Create `crates/agentstacks/src/model/token_error.rs` — `TokenError` with an `InvalidName`
  variant (`"<value>" is not a valid name`) and `InvalidOptionName`
  (`"<value>" is not a valid option name`); later tasks add variants.
- Modify `model/mod.rs`, `model/diagnostic.rs` (`FormatError::Token(#[from] TokenError)`).

**Change:** §4.1 names.

**Proof:**
- `names_accept_the_spec_examples` — `core` and `guideline-rust` parse as `PackName`.
- `names_refuse_the_spec_examples` — `Core`, `9lives`, `-x` and a 65-character `a…a` are refused
  with `"<value>" is not a valid name`.
- `option_name_accepts_snake_case_and_refuses_kebab` — `archive_keep` parses as `OptionName`,
  `a-b` is refused with `"a-b" is not a valid option name`; `archive_keep` is refused as
  `PackName`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add pack, tool, item and option names`.

### Task 2 — Scan tokens

**Files:**
- Create `crates/agentstacks/src/model/token_kind.rs`, `var.rs`, `token_target.rs`, `token.rs`,
  `scan.rs`.
- Modify `crates/agentstacks/src/model/token_error.rs` — variants for every §3.2 refusal:
  unknown kind, unterminated, empty, needs a target, takes one target, invalid target
  (`"<t>" is not a valid target (expected [pack/]name)`), root takes a pack name, unknown var
  (with the known list in the message), invalid option name, option tokens name this pack's
  options only.

**Change:** `scan(text)` walks the text line by line. `{{` opens a token unless the two bytes are
preceded by `\`; a token must close with `}}` on the same line; inner `ws` (space or tab) is
trimmed; the first word is the kind; the rest is at most one target. Targets parse per kind:
`skill`/`agent`/`skill_dir` take `[pack/]item` (exactly zero or one `/`, both sides valid names);
`root` takes an optional pack name; `var` takes one of `session_id`, `arguments`, `tmpdir`,
`home`, `airsstack_home`, `project_dir`; `option` takes an option name and refuses any `/`. A
`}}` without an opener is plain text. Every error carries its 1-based line. Scanning continues
after an error.

**Proof:**
- `scan_accepts_the_spec_examples` — `run {{skill sdlc/status}} now` gives one `skill` token for
  `sdlc/status`; `{{ root }}/scripts/x.lua` gives `root` with no pack; `{{skill_dir
  discuss}}/references/protocol.md` gives `skill_dir` for this pack's `discuss`.
- `scan_treats_escape_and_lone_closer_as_text` — `$\{{ secrets.X }}` and `a }} b` give no
  tokens and no errors.
- `scan_refuses_each_spec_example` — table-driven over every refusal row of spec §3.2 that does
  not need pack knowledge (`{{skil core/x}}`, `{{skill core/x`, a newline before `}}`, `{{}}`,
  `{{ }}`, `{{skill}}`, `{{agent}}`, `{{skill_dir}}`, `{{var}}`, `{{option}}`, `{{root core
  extra}}`, `{{skill a/b/c}}`, `{{skill /x}}`, `{{skill core/}}`, `{{skill Core/x}}`, `{{root
  core/x}}`, `{{var core/tmpdir}}`, `{{var session}}`, `{{option a-b}}`, `{{option
  core/archive_keep}}`), asserting the exact message and line of each.
- `scan_reports_every_error_in_one_pass` — a three-line text with two bad tokens and one good one
  returns the good token and both errors with lines 1 and 3.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): scan tokens in pack text`.

### Task 3 — Render tokens

**Files:**
- Create `crates/agentstacks/src/model/render.rs` — `render(text, values)` where `values` maps
  each scanned token (by span) to its replacement string; returns the text with every token
  replaced and every `\{{` turned into `{{`. Values are inserted as-is and never re-scanned.
- Modify `model/mod.rs`.

**Change:** the splice half of §3.5 and the escape rule of §3.1; also the accepted limitation:
`\` immediately before a token always means the escape.

**Proof:**
- `render_replaces_tokens_and_keeps_surrounding_text` — `run {{skill sdlc/status}} now` with
  value `/status` renders `run /status now`.
- `render_unescapes_literal_braces` — `$\{{ secrets.X }}` renders `${{ secrets.X }}`.
- `render_does_not_rescan_values` — a value containing `{{skill x}}` appears literally in the
  output.
- `render_backslash_before_token_is_the_escape` — `C:\{{root}}` renders `C:{{root}}` (the
  documented limitation), not a path.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): render tokens with supplied values`.

---

## Risks

- Byte spans must be taken on the original text so `render` splices correctly when a line holds
  several tokens and escapes; the multi-token test covers it.
- Rules that need pack knowledge (`{{var arguments}}` outside `SKILL.md`, cross-pack targets,
  missing items, undeclared options, `root…/native/`) are not checked here; plans 06 and 10 own
  them.

## Proof

- `cargo make dod-crate agentstacks` green; every row of spec §3.2 that needs no pack knowledge
  has a passing case in `scan_refuses_each_spec_example`.
