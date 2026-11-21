---
status: done
created: 2026-11-20
depends-on: [01]
built-on: 3162a314af79
---

# Names and Tokens Implementation Plan

**Goal:** One token module turns any pack text into located tokens built on validated names.

**Architecture:** Names are newtypes that parse on construction (parse, don't validate), one per
file: `PackName`, `ToolName`, `ItemName` share one crate-private kebab rule (`kebab_name.rs`) and
one error, `InvalidName`; `OptionName` has its own snake rule and error, `InvalidOptionName`. The
rules are checked by hand, so no regex dependency is added. Spec §11 sketches `name.rs` and
`token.rs` as multi-item files; §11 also requires one item per file, so this plan splits them.

The token vocabulary is one item per file: `TokenKind` (the six keywords), `Var` (the fixed var
list), `ItemRef` (`[pack/]item`), `TokenTarget` (one variant per kind, so a kind and its target
cannot disagree; `TokenTarget::kind` gives the kind) and `Token` (target, byte span in the scanned
text, and `reaches_native`). `TokenError` is this module's `thiserror` enum; it wraps
`InvalidName` and `InvalidOptionName` transparently and joins plan 01's `FormatError` as a `Token`
variant.

Scanning follows spec §3.5 "each text is scanned exactly once": every caller (frontmatter values,
Markdown bodies, `render` files, hook fields) calls the same function with a `TextOrigin` saying
how to locate what it finds (spec §10, first bullet):

- `TextOrigin::Lines { file, first_line }` — Markdown bodies and `render` files: located
  `<file>:<line>` through plan 01's `Location::at_line`, where `line` = `first_line` + the token's
  0-based line in the text.
- `TextOrigin::Field { file, key_path }` — TOML fields and frontmatter values: located
  `<file>: <key.path>` through plan 01's `Location::at_key`, with no line.

`scan(text, &origin) → (Vec<LocatedToken>, Vec<Diagnostic>)` never stops at the first error.
`LocatedToken { token, location }` (the contract item this plan owns) is what every later plan
stores; it has `token()` and `location()`, and the set-level refusals of §10 ("keep that token's
own location") take `location()` as is.

Spec §2.2: "The scanner decides 'followed by `/native/`' while it has the text and records it on
the token, so the rule runs over every token, wherever it came from." `scan` sets
`Token::reaches_native()` to true exactly when the token's kind is `root` and the scanned text
right after its closing `}}` starts with `/native/`. Plan 06 applies the §2.2 refusal to every
stored token through `located.token().reaches_native()`, whether the token came from a body, a
`render` file, a frontmatter value or a hook field; no caller needs the text again.

`scan` applies every §3.2 row that needs no pack: the grammar rows and the option-name rows
(`{{option a-b}}`, `{{option core/archive_keep}}`), refused for every origin, so an `Option`
target always holds a valid `OptionName` with no pack part. The pack-local rules of §3.5 that need
the pack or the file (§3.2 `{{var arguments}}` placement, §2.2 no `native/` from neutral content,
own-pack targets) are plan 06's; the set-level rules (§3.3 cross-pack targets, §9 item 3
undeclared options) are plan 10's. Both read the `LocatedToken`s this plan produces.

Rendering takes the spans `scan` recorded and never re-scans tokens:
`render(text: &str, values: &TokenValues) -> String`. `TokenValues` holds one replacement string
per token, keyed by the token's span; `render` splices each value in verbatim and turns every
`\{{` outside a token into `{{`. This is the only `render` signature; every task and test below
uses it.

**Tech Stack:** Rust 2024, `thiserror` 2 (added by plan 01). No new dependency.

**Content authority:** spec §2.2 (the scanner records "followed by `/native/`" on the token), §3.1
(grammar, kinds, var list, escape, accepted limitation), §3.2 (checked examples that need no pack
knowledge), §3.3 (values are never re-scanned), §3.5 (scanned once, every token kept with its
location), §4.1 (names), §10 (location forms, `Diagnostic` `Display`).

---

## Files that change

```
crates/agentstacks/src/model/mod.rs                 — modify  declare and re-export the new items
crates/agentstacks/src/model/kebab_name.rs          — create  crate-private kebab rule ^[a-z][a-z0-9-]{0,63}$
crates/agentstacks/src/model/invalid_name.rs        — create  InvalidName: "<value>" is not a valid name
crates/agentstacks/src/model/invalid_option_name.rs — create  InvalidOptionName: "<value>" is not a valid option name
crates/agentstacks/src/model/pack_name.rs           — create  PackName (kebab rule)
crates/agentstacks/src/model/tool_name.rs           — create  ToolName (kebab rule)
crates/agentstacks/src/model/item_name.rs           — create  ItemName (kebab rule)
crates/agentstacks/src/model/option_name.rs         — create  OptionName (snake rule ^[a-z][a-z0-9_]{0,63}$)
crates/agentstacks/src/model/token_error.rs         — create  TokenError (thiserror), the §3.2 messages scan owns
crates/agentstacks/src/model/token_kind.rs          — create  TokenKind: skill, agent, skill_dir, root, var, option
crates/agentstacks/src/model/var.rs                 — create  Var: the six fixed var names
crates/agentstacks/src/model/item_ref.rs            — create  ItemRef { pack: Option<PackName>, item: ItemName }
crates/agentstacks/src/model/token_target.rs        — create  TokenTarget: one variant per kind
crates/agentstacks/src/model/token.rs               — create  Token { target, span, reaches_native }
crates/agentstacks/src/model/text_origin.rs         — create  TextOrigin::{Lines, Field}: how scanned text is located
crates/agentstacks/src/model/located_token.rs       — create  LocatedToken { token, location }
crates/agentstacks/src/model/scan.rs                — create  scan(text, &origin) → (tokens, diagnostics)
crates/agentstacks/src/model/token_values.rs        — create  TokenValues: replacement per token span
crates/agentstacks/src/model/render.rs              — create  render(text, values): splice values, unescape \{{
crates/agentstacks/src/model/format_error.rs        — modify  FormatError gains Token(#[from] TokenError)
```

## Order of work

### Task 1 — Add the name newtypes

**Files:**
- Create `crates/agentstacks/src/model/invalid_name.rs` — `InvalidName` (`thiserror` struct
  holding the refused value), displayed `"<value>" is not a valid name`.
- Create `crates/agentstacks/src/model/invalid_option_name.rs` — `InvalidOptionName`, displayed
  `"<value>" is not a valid option name`.
- Create `crates/agentstacks/src/model/kebab_name.rs` — crate-private `check_kebab_name(value)`
  returning `InvalidName` unless the value is 1 to 64 bytes, starts with `a`–`z`, and continues
  with `a`–`z`, `0`–`9` or `-` only.
- Create `crates/agentstacks/src/model/pack_name.rs`, `crates/agentstacks/src/model/tool_name.rs`,
  `crates/agentstacks/src/model/item_name.rs` — `PackName`, `ToolName`, `ItemName`: each a private
  tuple struct over `String` with a fallible `new` (calls `check_kebab_name`), `FromStr` and
  `TryFrom<&str>` delegating to `new`, `as_str`, `Display`, `Debug`, `Clone`, `Eq`, `Ord`, `Hash`.
- Create `crates/agentstacks/src/model/option_name.rs` — `OptionName`, same shape, with the rule
  1 to 64 bytes, first byte `a`–`z`, then `a`–`z`, `0`–`9` or `_`; refuses with
  `InvalidOptionName`.
- Modify `crates/agentstacks/src/model/mod.rs` — declare the files; re-export the public items
  (`kebab_name` stays crate-private).

**Change:** spec §4.1. Callers whose spec wording differs (§5 `native: "Claude" is not a valid tool
name`, §7 `packs.Core: not a valid name`) map `InvalidName` into their own message; that mapping
belongs to plans 04 and 08.

**Proof:**
- `kebab_rule_accepts_spec_examples` (kebab_name.rs) — `core`, `guideline-rust` and a 64-byte
  `a…a` pass.
- `kebab_rule_refuses_spec_examples` (kebab_name.rs) — `Core`, `9lives`, `-x`, a 65-byte `a…a`,
  `archive_keep` and the empty string are each refused with an `InvalidName` holding that value.
- `invalid_name_display_quotes_the_value` (invalid_name.rs) — displays `"Core" is not a valid
  name`.
- `invalid_option_name_display_quotes_the_value` (invalid_option_name.rs) — displays `"a-b" is
  not a valid option name`.
- `pack_name_parses_spec_examples` (pack_name.rs) — `core` and `guideline-rust` parse through
  `new`, `FromStr` and `TryFrom`; `as_str` and `Display` return the input.
- `pack_name_refuses_option_style_name` (pack_name.rs) — `archive_keep` is refused with the
  message `"archive_keep" is not a valid name` (spec §4.1: ✓ as option, ✗ as pack name).
- `tool_name_parses_and_refuses` (tool_name.rs) — `claude` parses; `Claude` is refused with
  `"Claude" is not a valid name`.
- `item_name_parses_and_refuses` (item_name.rs) — `discuss` parses; `My_Skill` is refused with
  `"My_Skill" is not a valid name`.
- `option_name_accepts_snake_case` (option_name.rs) — `archive_keep`, `style_reinject` and a
  64-byte `a…a` parse.
- `option_name_refuses_spec_examples` (option_name.rs) — `a-b` is refused with `"a-b" is not a
  valid option name`; `Archive`, `9x`, `_x`, a 65-byte `a…a` and the empty string are refused
  with that message shape and their own value.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add pack, tool, item and option names`.

### Task 2 — Add the token vocabulary

**Files:**
- Create `crates/agentstacks/src/model/token_error.rs` — `TokenError` (`thiserror`), variants
  and messages copied from spec §3.2:
  `UnknownKind` → `unknown token kind "<word>"`; `Unterminated` → `unterminated token`; `Empty`
  → `empty token`; `NeedsTarget` → `token "<kind>" needs a target`; `TakesOneTarget` → `token
  "<kind>" takes one target`; `InvalidTarget` → `"<target>" is not a valid target (expected
  [pack/]name)`; `Name(#[from] InvalidName)` (transparent); `RootTakesPackName` → `token "root"
  takes a pack name, not "<target>"`; `UnknownVar` → `unknown var "<name>" (known: session_id,
  arguments, tmpdir, home, airsstack_home, project_dir)`; `OptionName(#[from]
  InvalidOptionName)` (transparent); `OptionNotThisPack` → `option tokens name this pack's
  options only`. The §3.2 rows that need the pack or the file (`var "arguments" is only valid in
  SKILL.md`, `pack "core" has no option "archive_kep"`, `pack "core" has no skill "coder"`) are
  not variants here: plans 06 and 10 own those messages.
- Create `crates/agentstacks/src/model/token_kind.rs` — `TokenKind`: `Skill`, `Agent`,
  `SkillDir`, `Root`, `Var`, `Option`; `parse(word)` (refuses with `UnknownKind`), `Display` as
  the keyword, `needs_target()` (true for all but `Root`).
- Create `crates/agentstacks/src/model/var.rs` — `Var`: `SessionId`, `Arguments`, `Tmpdir`,
  `Home`, `AirsstackHome`, `ProjectDir`; `parse(word)` (refuses with `UnknownVar`, whose message
  lists the six names in spec order from one `Var::ALL` list), `Display` as the var name.
- Create `crates/agentstacks/src/model/item_ref.rs` — `ItemRef { pack: Option<PackName>, item:
  ItemName }`; `parse(target)`: more than one `/`, or one `/` with an empty side, is
  `InvalidTarget`; otherwise the pack side (if any) then the item side are parsed as names, a bad
  one refused with `Name`. `Display` gives `pack/item` or `item`.
- Create `crates/agentstacks/src/model/token_target.rs` — `TokenTarget::{Skill(ItemRef),
  Agent(ItemRef), SkillDir(ItemRef), Root(Option<PackName>), Var(Var), Option(OptionName)}`;
  `parse(kind, words)` where `words` are the whitespace-separated words after the kind:
  none for a kind that needs a target → `NeedsTarget`; two or more, for any kind →
  `TakesOneTarget` with that kind's name; `root` with a `/` in its word → `RootTakesPackName`,
  else the word is a `PackName` (a bad one refused with `Name`); `var` → `Var::parse` on the whole
  word; `option` with a `/` → `OptionNotThisPack`, else `OptionName`; the others →
  `ItemRef::parse`. `kind()` returns the `TokenKind`; `Display` gives the target text (empty for
  `Root(None)`).
- Create `crates/agentstacks/src/model/token.rs` — `Token { target, span, reaches_native }`:
  `target()`, `kind()` (from the target), `span()` (byte range of the whole `{{…}}` in the scanned
  text), `reaches_native()` (spec §2.2: a `root` token followed by `/native/` in the scanned text;
  always false for other kinds). Constructed crate-internally by `scan`. `Display` gives the
  canonical form `{{<kind> <target>}}`, or `{{root}}` with no target, which plan 10 quotes in the
  §3.3 message.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** the grammar of spec §3.1 as types: which kinds exist, which need a target, what a
target may be per kind. The target-count rule is the §3.2 row "`{{root core extra}}` (any kind
given two or more target words) → `token "root" takes one target` (with that kind's name)", and
the `root` name rule is the row "`{{root Core}}` → `"Core" is not a valid name`".

**Proof:**
- `token_error_messages_match_spec` (token_error.rs) — table-driven: each variant's `Display`
  equals the §3.2 message without location (`unknown token kind "skil"`, `unterminated token`,
  `empty token`, `token "skill" needs a target`, `token "root" takes one target`, `"a/b/c" is not
  a valid target (expected [pack/]name)`, `"Core" is not a valid name`, `token "root" takes a pack
  name, not "core/x"`, `unknown var "session" (known: session_id, arguments, tmpdir, home,
  airsstack_home, project_dir)`, `"a-b" is not a valid option name`, `option tokens name this
  pack's options only`).
- `token_kind_parses_each_keyword` (token_kind.rs) — the six keywords parse and display back
  unchanged.
- `token_kind_refuses_unknown_word` (token_kind.rs) — `skil` is refused with `unknown token kind
  "skil"`.
- `token_kind_says_which_kinds_need_a_target` (token_kind.rs) — `skill`, `agent`, `skill_dir`,
  `var`, `option` need one; `root` does not.
- `var_parses_each_known_name` (var.rs) — the six names parse and display back unchanged.
- `var_refuses_unknown_with_known_list` (var.rs) — `session` and `core/tmpdir` are refused with
  `unknown var "session" (known: session_id, arguments, tmpdir, home, airsstack_home,
  project_dir)` and `unknown var "core/tmpdir" (known: session_id, arguments, tmpdir, home,
  airsstack_home, project_dir)`.
- `item_ref_parses_own_and_cross_pack` (item_ref.rs) — `discuss` gives no pack and item
  `discuss`; `sdlc/status` gives pack `sdlc` and item `status`; both display back unchanged.
- `item_ref_refuses_bad_shapes` (item_ref.rs) — `a/b/c`, `/x` and `core/` are refused with
  `"<target>" is not a valid target (expected [pack/]name)` naming that target; `Core/x` is
  refused with `"Core" is not a valid name`.
- `token_target_parses_per_kind` (token_target.rs) — `skill sdlc/status`, `agent coder`,
  `skill_dir discuss`, `root`, `root core`, `var tmpdir`, `option archive_keep` parse to the
  matching variant, and `kind()` returns the kind they were parsed with.
- `token_target_refuses_missing_or_extra_targets` (token_target.rs) — `skill`, `agent`,
  `skill_dir`, `var` and `option` with no word are refused with `token "<kind>" needs a target`;
  `root core extra` is refused with `token "root" takes one target`, and `skill a b`, `agent a b`,
  `skill_dir a b`, `var tmpdir home` and `option a b` with `token "<kind>" takes one target`
  naming their own kind (§3.2 "any kind given two or more target words").
- `token_target_refuses_root_var_and_option_shapes` (token_target.rs) — `root core/x` →
  `token "root" takes a pack name, not "core/x"`; `root Core` → `"Core" is not a valid name`;
  `var core/tmpdir` → `unknown var "core/tmpdir" (known: session_id, arguments, tmpdir, home,
  airsstack_home, project_dir)`; `option a-b` → `"a-b" is not a valid option name`;
  `option core/archive_keep` → `option tokens name this pack's options only`.
- `token_displays_canonical_form` (token.rs) — tokens built from `skill sdlc/status`, `root`,
  `root core` and `var tmpdir` display as `{{skill sdlc/status}}`, `{{root}}`, `{{root core}}`
  and `{{var tmpdir}}`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): add the token vocabulary`.

### Task 3 — Scan text into located tokens

**Files:**
- Create `crates/agentstacks/src/model/text_origin.rs` — `TextOrigin::{Lines { file: PathBuf,
  first_line: NonZeroUsize }, Field { file: PathBuf, key_path: String }}`, typed as plan 01's
  `Location` types its parts; `locate(line_in_text)` builds the `Location`: `Lines` gives
  `Location::at_line(file, first_line + line_in_text)` (0-based line in text, 1-based result);
  `Field` gives `Location::at_key(file, key_path)` whatever the line.
- Create `crates/agentstacks/src/model/located_token.rs` — `LocatedToken { token, location }`
  with `token()` and `location()`; constructed crate-internally by `scan`. It carries no rule: the
  rules that read it (`reaches_native`, the `var arguments` placement, targets) belong to plans 06
  and 10.
- Create `crates/agentstacks/src/model/scan.rs` — `scan(text, &origin)`.
- Modify `crates/agentstacks/src/model/format_error.rs` — `FormatError` gains
  `Token(#[from] TokenError)`, transparent.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** `scan` walks the text line by line (`\n` ends a line). On each line it looks for
`{{`: preceded by `\` it is the escape and is skipped; otherwise it opens a token, which must close
with the first `}}` on the same line, else `Unterminated` is reported for that line and the rest
of the line is not scanned. Between the braces, space and tab are the only whitespace; the trimmed
inside empty is `Empty`; the first word goes to `TokenKind::parse`, the remaining words to
`TokenTarget::parse`. A `}}` without an opener is text. A good token becomes a `LocatedToken`
whose location comes from `origin.locate(line)`; for a `root` token, `scan` records whether the
text right after the closing `}}` starts with `/native/` (§2.2), across the rest of the whole
text, not only the line. A refusal becomes `Diagnostic::new(origin.locate(line), error)` (plan
01). Scanning resumes after the closing `}}`, so later tokens on the same line and later lines are
still read. Results keep text order. `scan` applies no rule that needs the pack or the file name.

**Proof:**
- `lines_origin_offsets_the_line` (text_origin.rs) — `Lines` for `skills/x/SKILL.md` with
  `first_line` 6 locates text line 2 (0-based) at `skills/x/SKILL.md:8`.
- `field_origin_locates_by_key_path` (text_origin.rs) — `Field` for `pack.toml` with key path
  `hooks[0].text` locates any line at `pack.toml: hooks[0].text`, with no line.
- `located_token_exposes_token_and_location` (located_token.rs) — the one token `scan` returns for
  `{{skill status}}` with origin `Lines` for `notes.md`, `first_line` 3, has kind `skill` and a
  location displayed `notes.md:3`.
- `scan_accepts_the_spec_examples` (scan.rs) — `run {{skill sdlc/status}} now` gives one `skill`
  token for pack `sdlc`, item `status`; `{{ root }}/scripts/x.lua` gives `root` with no pack;
  `{{skill_dir discuss}}/references/protocol.md` gives `skill_dir` for this pack's `discuss`; no
  diagnostics.
- `scan_treats_escape_and_lone_closer_as_text` (scan.rs) — `$\{{ secrets.X }}` and `a }} b` give
  no tokens and no diagnostics.
- `scan_refuses_each_spec_example` (scan.rs) — table-driven with origin `Lines` for `notes.md`,
  `first_line` 1; each input yields exactly one diagnostic whose `Display` is: `{{skil core/x}}` →
  `notes.md:1: unknown token kind "skil"`; `{{skill core/x` → `notes.md:1: unterminated token`;
  `{{skill core/x` + newline + `}}` → `notes.md:1: unterminated token` and nothing for line 2;
  `{{}}` and `{{ }}` → `notes.md:1: empty token`; `{{skill}}`, `{{agent}}`, `{{skill_dir}}`,
  `{{var}}`, `{{option}}` → `notes.md:1: token "<kind>" needs a target`; `{{root core extra}}` →
  `notes.md:1: token "root" takes one target`; `{{skill a b}}` → `notes.md:1: token "skill" takes
  one target`; `{{root Core}}` → `notes.md:1: "Core" is not a valid name`; `{{skill a/b/c}}`,
  `{{skill /x}}`, `{{skill core/}}` → `notes.md:1: "<target>" is not a valid target (expected
  [pack/]name)`; `{{skill Core/x}}` → `notes.md:1: "Core" is not a valid name`; `{{root core/x}}`
  → `notes.md:1: token "root" takes a pack name, not "core/x"`; `{{var core/tmpdir}}`,
  `{{var session}}` → `notes.md:1: unknown var "<name>" (known: session_id, arguments, tmpdir,
  home, airsstack_home, project_dir)`; `{{option a-b}}` → `notes.md:1: "a-b" is not a valid option
  name`; `{{option core/archive_keep}}` → `notes.md:1: option tokens name this pack's options
  only`.
- `scan_leaves_pack_rules_to_later_plans` (scan.rs) — origin `Lines` for `notes.md`:
  `{{var arguments}}`, `{{option archive_kep}}` and `{{skill_dir core/coder}}` each give one token
  and no diagnostic (the §3.2 rows for these need the file name or the pack: plans 06 and 10).
- `scan_reports_every_error_in_one_pass` (scan.rs) — the three lines `{{skil x}}`, `{{root}}`,
  `{{}}` give the `root` token at `notes.md:2` and the diagnostics `notes.md:1: unknown token kind
  "skil"` and `notes.md:3: empty token`, in that order.
- `scan_continues_after_an_error_on_the_same_line` (scan.rs) — `{{skil x}} then {{root}}` gives
  one diagnostic and the `root` token.
- `scan_finds_several_tokens_and_an_escape_on_one_line` (scan.rs) — `see {{skill status}},
  \{{literal}} and {{root}}/x` gives two tokens, `skill` then `root`, whose spans cover exactly the
  bytes of `{{skill status}}` and `{{root}}`, and no diagnostics.
- `scan_offsets_lines_from_first_line` (scan.rs) — origin `Lines` for `SKILL.md` with
  `first_line` 6 and `{{skil x}}` on the second text line gives `SKILL.md:7: unknown token kind
  "skil"`.
- `scan_locates_field_tokens_by_key_path` (scan.rs) — origin `Field` for `pack.toml`, key path
  `hooks[1].args[1]`, a two-line text with `{{root}}` on line 1 and `{{skil x}}` on line 2 gives a
  `root` token located at `pack.toml: hooks[1].args[1]` and the diagnostic `pack.toml:
  hooks[1].args[1]: unknown token kind "skil"`.
- `scan_applies_option_rows_to_field_text` (scan.rs) — origin `Field` for `pack.toml`, key path
  `hooks[0].text`: `{{option core/archive_keep}}` gives `pack.toml: hooks[0].text: option tokens
  name this pack's options only`; `{{option a-b}}` gives `pack.toml: hooks[0].text: "a-b" is not a
  valid option name`.
- `scan_marks_root_followed_by_native` (scan.rs) — origin `Lines`: `{{root}}/native/claude/x` and
  `{{root core}}/native/x` give tokens with `reaches_native()` true; `{{root}}/scripts/x`,
  `{{root}}/natives/x`, `{{root}}/hooks/native/x`, `{{root}}` at the end of the text and
  `{{skill_dir a}}/native/x` give false.
- `scan_marks_native_reach_in_field_text` (scan.rs) — origin `Field` for `pack.toml`, key path
  `hooks[0].args[0]`, text `{{root}}/native/claude/x` gives a token with `reaches_native()` true
  located at `pack.toml: hooks[0].args[0]`, so plan 06 can refuse it there without the text.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): scan pack text into located tokens`.

### Task 4 — Render tokens

**Files:**
- Create `crates/agentstacks/src/model/token_values.rs` — `TokenValues`: `new()`,
  `insert(&Token, value: String)` keyed by the token's span (a second insert for the same token
  replaces the first), and iteration in text order (by span start).
- Create `crates/agentstacks/src/model/render.rs` — `render(text: &str, values: &TokenValues) ->
  String`: copies the text, replacing each valued span with its value verbatim (values are never
  scanned) and turning every `\{{` in the copied text outside those spans into `{{`. Its doc states
  the precondition: `text` is the exact text that was scanned (spans index it) and `values` holds
  one value per token.
- Modify `crates/agentstacks/src/model/mod.rs` — declare and re-export.

**Change:** the splice half of spec §3.5, the escape rule of §3.1, §3.3 "values are inserted as-is
and never re-scanned", and the accepted limitation: `\` immediately before `{{` is always the
escape.

**Proof:**
- `token_values_iterate_in_text_order` (token_values.rs) — values inserted for the second token
  first and the first token second iterate first-token-first; a second insert for one token keeps
  only the later value.
- `render_replaces_tokens_and_keeps_surrounding_text` (render.rs) — `run {{skill sdlc/status}}
  now`, scanned, with value `/status` renders `run /status now`.
- `render_unescapes_literal_braces` (render.rs) — `$\{{ secrets.X }}`, scanned (no tokens),
  renders `${{ secrets.X }}`.
- `render_does_not_rescan_values` (render.rs) — a value `{{skill x}}` appears literally in the
  output.
- `render_backslash_before_token_is_the_escape` (render.rs) — `C:\{{root}}`, scanned (no tokens),
  renders `C:{{root}}`.
- `render_handles_several_tokens_and_an_escape_on_one_line` (render.rs) — `see {{skill status}},
  \{{literal}} and {{root}}/x`, scanned, with values `/status` and `.agentstacks/core` renders
  `see /status, {{literal}} and .agentstacks/core/x`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): render tokens with supplied values`.

---

## Risks

- Spans index the exact text passed to `scan`. A caller that renders a different string (the
  whole `SKILL.md` when only the body after `---` was scanned) would misplace values; `render`'s doc
  states the precondition, and each caller renders the text it scanned.
- `render` copies a span that has no value as written. Callers decide per §3.5 that an item whose
  value the extension cannot supply becomes a skip before anything is rendered; no caller renders
  in this chain (§14).
- `reaches_native` is decided on the scanned text only. A frontmatter value is scanned after YAML
  parsing (§3.4), so the flag reflects the parsed string, which is what renders; plan 04 scans that
  string, so no caller has to re-derive it.
- Option not taken: placement (`{{var arguments}}`) as a method on `LocatedToken`. Plan 06 owns
  that message (`PackError`) and applies it with the other pack-local rules over all tokens (§3.5),
  so the message is defined once.
- Option not taken: a kind field next to a free-form target (the earlier draft), which allowed a
  kind and target to disagree; the per-kind `TokenTarget` rules that out.
- Option not taken: a regex or template crate; the rules are a few byte checks, so no dependency
  is added and `cargo make deny` is not needed for this plan.

## Proof

- `cargo make dod-crate agentstacks` green after each task; `cargo make dod` green after Task 4.
- Coverage of spec rows in scope:
  - §4.1 checked examples → `kebab_rule_accepts_spec_examples`, `kebab_rule_refuses_spec_examples`,
    `pack_name_refuses_option_style_name`, `option_name_accepts_snake_case`,
    `option_name_refuses_spec_examples`.
  - §3.2 accepted rows (1–3) → `scan_accepts_the_spec_examples`; escape and lone `}}` rows →
    `scan_treats_escape_and_lone_closer_as_text`, `render_unescapes_literal_braces`.
  - §3.2 refusal rows that need no pack (unknown kind, unterminated, empty, needs a target, any
    kind given two or more target words, `{{root Core}}`, not a valid target, `{{skill Core/x}}`,
    root takes a pack name, unknown var, option name, option this-pack) →
    `scan_refuses_each_spec_example`, each asserting the `Diagnostic` `Display`;
    `token_target_refuses_missing_or_extra_targets`,
    `token_target_refuses_root_var_and_option_shapes`; for field origins →
    `scan_locates_field_tokens_by_key_path`, `scan_applies_option_rows_to_field_text`.
  - §2.2 the scanner records "root token followed by `/native/`" on the token →
    `scan_marks_root_followed_by_native`, `scan_marks_native_reach_in_field_text`; the refusal
    itself → plan 06.
  - §3.1 accepted limitation → `render_backslash_before_token_is_the_escape`; §3.3 values not
    re-scanned → `render_does_not_rescan_values`; several tokens plus an escape on one line →
    `scan_finds_several_tokens_and_an_escape_on_one_line`,
    `render_handles_several_tokens_and_an_escape_on_one_line`.
  - §10 location forms → `lines_origin_offsets_the_line`, `field_origin_locates_by_key_path`,
    `located_token_exposes_token_and_location`, `scan_offsets_lines_from_first_line`.
  - Rows owned elsewhere (`scan_leaves_pack_rules_to_later_plans` proves `scan` does not refuse
    them): §3.2 `{{var arguments}}` outside a `SKILL.md` → plan 06; §3.2
    `{{skill_dir core/coder}}` → plan 06 (own pack) and plan 10 (other pack, §9 item 2); §3.2
    `{{option archive_kep}}` → plan 10 (§9 item 3).

## Review findings

Reviewer (one round): blocking none; code 0🔴 3🟡 7🔵, plan 2🔵. Fixer applied all; `cargo test -p agentstacks model::` → `test result: ok. 162 passed; 0 failed`.

- 🟡 render.rs:28 broken span precondition silently drops values — fixed: `debug_assert!`s for order, bounds and char boundaries.
- 🟡 scan.rs:52 no multi-line span test — fixed: `scan_gives_a_token_on_a_later_line_its_text_span_and_line` (span 6..14, `reaches_native`, `n.md:2`, rendered output).
- 🟡 scan.rs:232 workflow vocabulary in test name — fixed (renamed, see Deviations).
- 🔵 test names containing "spec" — fixed (renamed, see Deviations).
- 🔵 render.rs:21 untested no-value copy — fixed: `render_copies_a_token_with_no_value_as_written`.
- 🔵 hard-coded counts in token_kind/var/token_error docs — fixed.
- 🔵 token_target.rs:51 `[]` assumed root — fixed (matches on kind).
- 🔵 token_values.rs:66 test pattern — fixed (slice pattern).
- 🔵 item_ref.rs:37 Vec allocation — fixed (`split_once`); option_name.rs:30 duplicated rule — fixed (`is_name_with_separator` in kebab_name.rs, test `separator_rule_takes_the_separator_as_a_parameter`).
- 🔵 plan additions `InvalidName::value()`, `InvalidOptionName::value()`, `TokenKind::as_str`, `Var::as_str` — accepted.

## Deviations

- 2026-11-20 — Run shape: one coder ran Tasks 1–4 in order, in parallel with plan 03 in the same tree (shared file `model/mod.rs`, append-only lines).
- 2026-11-20 — `kebab_name` is a private module with `pub(super)` functions (clippy `redundant_pub_crate` rejects `pub(crate)` inside it).
- 2026-11-20 — `Token::span()` returns an owned `Range<usize>`; `TokenValues` keys by `(start, end)`.
- 2026-11-20 — `InvalidName` / `InvalidOptionName` gained public `value()`; `TokenKind::as_str`, `Var::as_str` added.
- 2026-11-20 — `TokenError` variants carry their message fields (`UnknownKind{word}`, `NeedsTarget{kind}`, `TakesOneTarget{kind}`, `InvalidTarget{target}`, `RootTakesPackName{target}`, `UnknownVar{name}`); derives `Clone, PartialEq, Eq`.
- 2026-11-20 — mod.rs doc links written `scan()` / `render()` (rustdoc: module and function share the name).
- 2026-11-20 — Test renames after review (the guideline forbids workflow vocabulary in source): `kebab_rule_accepts_spec_examples` → `kebab_rule_accepts_lowercase_digits_and_hyphens_up_to_64_bytes`; `kebab_rule_refuses_spec_examples` → `kebab_rule_refuses_bad_start_bad_character_overlong_and_empty`; `pack_name_parses_spec_examples` → `pack_name_parses_through_new_from_str_and_try_from`; `option_name_refuses_spec_examples` → `option_name_refuses_bad_start_bad_character_overlong_and_empty`; `token_error_messages_match_spec` → `token_error_displays_each_refusal_message`; `scan_accepts_the_spec_examples` → `scan_accepts_skill_root_and_skill_dir_tokens`; `scan_refuses_each_spec_example` → `scan_refuses_each_malformed_token`; `scan_leaves_pack_rules_to_later_plans` → `scan_accepts_tokens_whose_validity_depends_on_the_pack`.

- 2026-11-20 — Final gate after fixes: `cargo make dod` → `test result: ok. 191 passed; 0 failed`, `[cargo-make] INFO - Build Done in 8.89 seconds.`
- 2026-11-21 — **Redesign after build:** the code was moved into `crates/agentstacks/src/types/<group>/` with the `types::port::FileTree` port and `fs::StdFileTree`, files renamed so no name repeats its folder word, and refined by crate-wide reviews. Paths and names above are superseded by the built tree; behaviour follows the redesigned `spec.md`.
