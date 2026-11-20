---
status: approved
created: 2026-11-20
depends-on: [06, 07, 08, 09]
built-on: 3162a314af79
---

# Pack Set Check Implementation Plan

**Goal:** `pack_set::check` validates a whole project, returning a `ValidSet` as the only input
later stages accept.

**Architecture:** `model/pack_set.rs` holds one item, `check(project, packs, extensions)`.
`project` is the parsed `ProjectFile` (plan 08). `packs` maps each `[packs.*]` key (`PackName`)
to what `Pack::load` (plan 06) returned for it: the `Pack`, or the diagnostics that refused it.
`extensions` maps each `[tools.*]` key (`ToolName`) to what `Extension::load` (plan 07) returned.
A project key with no entry in its map counts as a failed load. `check` returns `Ok(ValidSet)`
only when every entry loaded and no set check refused. Otherwise it returns the `Err` of plan
01's `Diagnostics::into_result`: the load diagnostics plus every set diagnostic, sorted per §10.
A failed load never yields a `ValidSet`, and it causes no follow-on diagnostic (§9). Every check
addresses a pack by its `[packs.*]` key; a message that names a pack uses that key, which equals
`pack.name` whenever the fetched-name check passes.

Each §9 check lives in its own file, one item per file, under `crates/agentstacks/src/model/`:

- `fetched_names.rs`: key equals the fetched name (§7).
- `requires_graph.rs`: requires are listed, with no cycle (§7, §9 item 1).
- `token_targets.rs`: other-pack `skill`/`agent`/`skill_dir`/`root` targets and `option` tokens
  (§3.3, §9 items 2 and 3).
- `hook_runs.rs`: other-pack `run` (§6.2, §9 item 4).
- `option_value.rs` and `option_values.rs`: the typed value with its rendering form, then
  resolution (§4.4, §9 item 3).
- `version_interval.rs` and `airsl_range.rs`: one comparator as an interval, then the set's range
  with the installed-version check (§4.3, §9 item 5).
- `name_clash.rs`: name clashes (§9 item 6).
- `valid_set.rs`: `ValidSet`, which holds the project, the loaded packs and extensions by key, the
  resolved options per pack, and the combined `AirslRange`. The range is absent when no pack
  declares `requires.airsl`.

`SetError` (`set_error.rs`, thiserror) carries every set message and joins `FormatError`.

Plan 10 reads `Pack` and adds nothing to it. It reads the manifest (`requires.packs`,
`requires.airsl` as plan 05's `AirslReq`, the `options` declarations as plan 05's `OptionDecl`,
the hooks with their `HookRun`), skill names, agent names, the installed root files, each native
directory's `NativeHooks`, and the `LocatedToken`s (plan 02) that plan 06 keeps from frontmatter,
bodies, `render` files and hook fields (§3.5).

Own-pack token targets, own-pack `run` and hook `agents` are pack-local and belong to plan 06
(§3.5, §6.2, §9 items 2 and 4); this plan never re-checks them. Unknown `requires.airsl`
operators are refused when `pack.toml` is parsed (plan 05, §4.3).

**Locations (§10).** Refusals about one token or one hook field keep that token's or field's own
location. A token refusal is built with plan 02's `LocatedToken::refuse`, so it is
`<file>:<line>` for a token in Markdown or a `render` file and `<file>: <key.path>` for a token in
frontmatter or a hook field (`<pack>/pack.toml: hooks[0].args[1]`). An other-pack `run` refusal is
at `<pack>/pack.toml: hooks[<i>].run`, or at `<pack>/native/<tool>/hooks.toml: hooks[<i>].run`
for a native hook, `<pack>` being the directory passed to `Pack::load`. Every other refusal is
built with plan 01's `Location::set_level`, so it is at `agentstacks.toml`:

| Refusal | Location |
|---|---|
| fetched name | `agentstacks.toml: packs.<key>` / `agentstacks.toml: tools.<key>` |
| unlisted requirement | `agentstacks.toml: packs.<requiring key>` |
| option value of the wrong type, or naming no declared option | `agentstacks.toml: packs.<key>.options.<name>` |
| required option not set | `agentstacks.toml: packs.<key>` |
| cycle, disjoint airsl ranges, installed airsl, name clash | `agentstacks.toml` alone |

Unit tests build packs and extensions in `tempfile` directories through `Pack::load` and
`Extension::load`, so no test constructs a `Pack` by hand. `<dir>` in a unit test's expected text
is the pack directory passed to `Pack::load`. The end-to-end test drives `ProjectFile::parse`,
then `Pack::load` / `Extension::load`, then `pack_set::check`, over fixtures under
`crates/agentstacks/tests/fixtures/sets/`; its expected texts prefix token and `run` locations with
the fixture path, as plan 06's fixture test does.

**Tech Stack:** Rust 2024; `semver` 1 and `toml` 1 (added by plan 05); `tempfile` 3 (dev, added by
plan 03). No new dependency.

**Content authority:** spec §9 (checks 1–6; failed loads skipped; own-pack targets not re-checked;
multi-pack message shapes), §7 (key equals fetched name, requires listed, no cycle), §3.2 (the
undeclared-option row), §3.3 (other-pack target rules), §3.5 (tokens kept on `Pack` with their
location), §4.3 (airsl range), §4.4 (option resolution and rendering), §6.2 (other-pack `run`),
§10 (locations, including set-level locations; all errors per run; sorting), §12 (testing and the
coverage rule), §1.1 (semver facts: `Op` is `#[non_exhaustive]` with 8 variants,
`S/lib.rs:246-257`; `*` parses to no comparators; `0.1` is Caret).

---

## Files that change

```
crates/agentstacks/src/lib.rs                     — modify  crate docs name pack_set::check and ValidSet as the entry points
crates/agentstacks/src/model/mod.rs               — modify  declare pack_set and the new files; re-export ValidSet, SetError, OptionValue, AirslRange
crates/agentstacks/src/model/format_error.rs      — modify  FormatError gains a Set variant
crates/agentstacks/src/model/set_error.rs         — create  SetError (thiserror): every set-level message
crates/agentstacks/src/model/fetched_names.rs     — create  check_fetched_names: [packs.*]/[tools.*] key = fetched name
crates/agentstacks/src/model/requires_graph.rs    — create  check_requires: requires listed, no cycle
crates/agentstacks/src/model/token_targets.rs     — create  check_token_targets: other-pack targets, option tokens
crates/agentstacks/src/model/hook_runs.rs         — create  check_hook_runs: other-pack {{root <pack>}}/… runs
crates/agentstacks/src/model/option_value.rs      — create  OptionValue: typed resolved value and its rendering form
crates/agentstacks/src/model/option_values.rs     — create  resolve_options: §4.4 per pack
crates/agentstacks/src/model/version_interval.rs  — create  VersionInterval: release versions one comparator matches
crates/agentstacks/src/model/airsl_range.rs       — create  AirslRange over AirslReq: intersect, admits(installed)
crates/agentstacks/src/model/name_clash.rs        — create  check_name_clashes: same skill/agent name across packs
crates/agentstacks/src/model/valid_set.rs         — create  ValidSet
crates/agentstacks/src/model/pack_set.rs          — create  check: runs every set check, returns ValidSet or diagnostics
crates/agentstacks/tests/pack_set.rs              — create  end-to-end fixture test
crates/agentstacks/tests/fixtures/sets/**         — create  one valid project, one invalid project per set refusal
```

## Order of work

### Task 1 — Check fetched names and the requires graph

**Files:**
- Create `crates/agentstacks/src/model/set_error.rs`, `crates/agentstacks/src/model/fetched_names.rs`,
  `crates/agentstacks/src/model/requires_graph.rs`.
- Modify `crates/agentstacks/src/model/format_error.rs`, `crates/agentstacks/src/model/mod.rs`.

**Change:** `SetError` starts with the variants for this task. `FormatError` gains
`Set(#[from] SetError)`, transparent.

`check_fetched_names` refuses any loaded pack whose `pack.name` differs from its `[packs.<key>]`
key, at `agentstacks.toml: packs.<key>`, with the message `source contains pack "<name>"`. It
refuses any loaded extension whose manifest `name` differs from its `[tools.<key>]` key, at
`agentstacks.toml: tools.<key>`, with the message `source contains extension "<name>"` (§7).
Failed loads are not checked.

`check_requires` works over the loaded packs. Each `requires.packs` entry that is not a
`[packs.*]` key is refused at `agentstacks.toml: packs.<requiring key>` with the message
`<pack> requires pack "<req>", which agentstacks.toml does not list (add it: --pack <req>)` (§7,
§10).

Cycles are detected over the edges between listed packs. A failed pack contributes no edges,
because its requires are unknown. Each strongly connected component of two or more packs is
reported once, at `agentstacks.toml` alone, as `requires cycle: <p1> → … → <p1>`. The printed
cycle is the shortest path from the component's smallest name back to itself, with ties broken by
name order.

**Proof:**
- `set_error_displays_each_spec_message` (`set_error.rs`): table-driven. Each variant's `Display`
  equals its spec message. This task adds rows for `source contains pack "kore"`, `source contains
  extension "claud"`, `sdlc requires pack "core", which agentstacks.toml does not list (add it:
  --pack core)` and `requires cycle: a → b → a`.
- `pack_key_must_equal_fetched_name` (`fetched_names.rs`): key `core` holding pack `kore` gives
  exactly one diagnostic whose `Display` is `agentstacks.toml: packs.core: source contains pack
  "kore"`. A matching key gives none.
- `tool_key_must_equal_fetched_name` (`fetched_names.rs`): key `claude` holding extension `claud`
  gives `agentstacks.toml: tools.claude: source contains extension "claud"`.
- `failed_loads_are_not_name_checked` (`fetched_names.rs`): a failed pack and a failed extension
  produce no diagnostic.
- `unlisted_requirement_is_refused_with_the_add_hint` (`requires_graph.rs`): `sdlc` requiring an
  unlisted `core` gives `agentstacks.toml: packs.sdlc: sdlc requires pack "core", which
  agentstacks.toml does not list (add it: --pack core)`.
- `each_cycle_is_reported_once_from_its_smallest_name` (`requires_graph.rs`): `b` requiring `a`
  and `a` requiring `b` gives exactly one diagnostic, `agentstacks.toml: requires cycle: a → b →
  a`. A second, separate cycle `c ↔ d` in the same set gives a second diagnostic,
  `agentstacks.toml: requires cycle: c → d → c`.
- `cycle_prints_the_shortest_path_back_to_the_smallest_name` (`requires_graph.rs`): `a → b → c →
  a` alone gives `agentstacks.toml: requires cycle: a → b → c → a`. With an extra edge `b → a`,
  the same component gives only `agentstacks.toml: requires cycle: a → b → a`.
- `diamond_is_not_a_cycle` (`requires_graph.rs`): `a → b`, `a → c`, `b → d`, `c → d` gives no
  diagnostic.
- `requirements_of_failed_packs_are_unknown` (`requires_graph.rs`): `a` requires `b`, `b` failed
  to load, and the failed `b`'s manifest would have required `a`. This gives no cycle diagnostic
  and no unlisted diagnostic.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): check fetched names and pack requirements`.

### Task 2 — Resolve other-pack token targets, option tokens and other-pack hook runs

**Files:**
- Create `crates/agentstacks/src/model/token_targets.rs`, `crates/agentstacks/src/model/hook_runs.rs`.
- Modify `crates/agentstacks/src/model/set_error.rs`, `crates/agentstacks/src/model/mod.rs`.

**Change:** `check_token_targets` runs over every `LocatedToken` of every loaded pack (§3.5),
wherever it was found, and refuses through `LocatedToken::refuse`, so each refusal keeps the
token's location (§10). It applies these rules:

- **Own-pack targets are skipped.** A `skill`, `agent` or `skill_dir` target whose `ItemRef` has
  no pack or names this pack, and a `root` target with no pack or this pack, were checked by plan
  06 and are not re-checked (§9 item 2).
- **Target in another pack, not required.** The named pack must be in this pack's
  `requires.packs` (§3.3). Otherwise the message is `{{<kind> <target>}} targets pack "<p>",
  which <this> does not require. Add it to requires.packs or remove the ref.` The token is quoted
  through plan 02's `Token` `Display`, its canonical form.
- **Target in a required pack that loaded.** The named item must exist there (§3.3). A `skill` or
  `skill_dir` target must be one of that pack's skills, else `pack "<p>" has no skill "<item>"`;
  an `agent` target one of its agents, else `pack "<p>" has no agent "<item>"`. `root` names no
  item.
- **Target in a required pack that failed to load or is not listed.** No item check runs: Task 1
  already refused the unlisted pack, and a failed load gets no follow-on (§9).
- **Option tokens.** An `option` token must name an option this pack declares (§9 item 3), else
  the message is `pack "<this>" has no option "<name>"` (§3.2).

`check_hook_runs` applies to every lua hook of a loaded pack, in `pack.toml` and in each
`native/<tool>/hooks.toml` (§2.2: the `run` rules apply unchanged), whose `HookRun` names another
pack `<p>` (`{{root <p>}}/…`, §6.2). When `<p>` is required and loaded, the `HookRun` path must
be one of `<p>`'s installed root files (never excluded, never under `skills/`, `agents/` or
`native/`, §2). Otherwise the hook is refused at its `run` field (see Locations) with the message
`"<run as written>" does not exist in <p>`, the run text being `HookRun`'s `written`.

A `run` into an unrequired pack gets only the `check_token_targets` refusal of its `root` token. A
`run` into a failed or unlisted pack is skipped.

**Proof:**
- `set_error_displays_each_spec_message`: adds rows for the §3.3 example message, `pack "core" has
  no agent "codr"`, `pack "core" has no skill "coder"`, `pack "core" has no option "archive_kep"`
  and `"{{root core}}/hooks/enforc.lua" does not exist in core`.
- `targets_outside_requires_are_refused_with_the_spec_message` (`token_targets.rs`): pack `sdlc`,
  which requires only `core`, has `{{skill guideline-rust/rust-guidelines}}` on line 68 of
  `skills/plan/SKILL.md`. This gives `<dir>/skills/plan/SKILL.md:68: {{skill
  guideline-rust/rust-guidelines}} targets pack "guideline-rust", which sdlc does not require. Add
  it to requires.packs or remove the ref.` The same token written `{{ skill
  guideline-rust/rust-guidelines }}` gives the same text.
- `targets_in_required_packs_resolve` (`token_targets.rs`): `{{skill core/discuss}}`,
  `{{agent core/coder}}`, `{{skill_dir core/discuss}}` and `{{root core}}` in `sdlc`, which
  requires `core`, give no diagnostic.
- `missing_items_are_refused_per_kind` (`token_targets.rs`): in `sdlc`, which requires `core`, on
  line 6 of `skills/a/SKILL.md`: `{{agent core/codr}}` gives `<dir>/skills/a/SKILL.md:6: pack
  "core" has no agent "codr"`; `{{skill_dir core/coder}}`, where `coder` is only an agent, gives
  `<dir>/skills/a/SKILL.md:6: pack "core" has no skill "coder"`; `{{skill core/statuss}}` gives
  `<dir>/skills/a/SKILL.md:6: pack "core" has no skill "statuss"`.
- `targets_are_checked_wherever_the_token_was_found` (`token_targets.rs`): `{{skill
  guideline-rust/rust-guidelines}}` in pack `sdlc` (not requiring `guideline-rust`) is refused with
  the §3.3 message at each token's own location: a `SKILL.md` frontmatter `description` →
  `<dir>/skills/a/SKILL.md: description`; line 6 of an agent body → `<dir>/agents/b.md:6`; line 1
  of a `render` file → `<dir>/enforcement.json:1`; line 1 of `native/claude/x.md` →
  `<dir>/native/claude/x.md:1`; a `pack.toml` hook arg → `<dir>/pack.toml: hooks[0].args[1]`; a
  native hook `text` → `<dir>/native/claude/hooks.toml: hooks[0].text`.
- `own_pack_targets_are_left_to_pack_load` (`token_targets.rs`): `check_token_targets` given the
  tokens of a loaded `sdlc` holding `{{skill sdlc/status}}` and `{{skill status}}`, where `status`
  exists, plus a loaded `core` whose tokens name its own existing items, gives no diagnostic; no
  own-pack rule is implemented in this file (plan 06 owns them).
- `option_tokens_must_name_a_declared_option` (`token_targets.rs`): pack `core` declares only
  `archive_keep`. `{{option archive_kep}}` on line 6 of `skills/a/SKILL.md` gives
  `<dir>/skills/a/SKILL.md:6: pack "core" has no option "archive_kep"`; in a hook `text` it gives
  `<dir>/pack.toml: hooks[0].text: pack "core" has no option "archive_kep"`; in a frontmatter
  `description` it gives `<dir>/skills/a/SKILL.md: description: pack "core" has no option
  "archive_kep"`. `{{option archive_keep}}` gives none.
- `targets_into_failed_packs_skip_the_item_check` (`token_targets.rs`): `core` failed to load,
  and `sdlc` requires it. `{{agent core/coder}}` gives no diagnostic.
- `targets_into_unlisted_packs_skip_the_item_check` (`token_targets.rs`): `sdlc` requires an
  unlisted `core`, and `{{agent core/coder}}` gives no diagnostic from this check.
- `other_pack_run_must_exist_in_that_pack` (`hook_runs.rs`): `guideline-rust` requires `core`.
  Its second hook's `run = "{{root core}}/hooks/enforc.lua"` gives `<dir>/pack.toml:
  hooks[1].run: "{{root core}}/hooks/enforc.lua" does not exist in core`, and
  `{{root core}}/hooks/enforce.lua` gives none.
- `other_pack_run_must_not_be_excluded_there` (`hook_runs.rs`): `core` excludes `hooks/old.lua`,
  and `{{root core}}/hooks/old.lua` as the first hook gives `<dir>/pack.toml: hooks[0].run:
  "{{root core}}/hooks/old.lua" does not exist in core`.
- `other_pack_run_must_be_a_root_file_there` (`hook_runs.rs`): `core` ships
  `skills/discuss/x.lua`; `{{root core}}/skills/discuss/x.lua` gives the same "does not exist in
  core" message with that run text.
- `other_pack_run_is_checked_in_native_hooks` (`hook_runs.rs`): the missing target of
  `other_pack_run_must_exist_in_that_pack` in the first hook of `native/claude/hooks.toml` gives
  `<dir>/native/claude/hooks.toml: hooks[0].run: "{{root core}}/hooks/enforc.lua" does not exist
  in core`.
- `other_pack_run_into_an_unrequired_pack_is_not_checked_here` (`hook_runs.rs`): a `run` into
  `core` from a pack not requiring it gives no diagnostic from `check_hook_runs`.
- `own_pack_run_is_left_to_pack_load` (`hook_runs.rs`): `check_hook_runs` over a loaded
  `guideline-rust` whose runs are all own-pack (`hooks/x.lua`, `{{root}}/hooks/x.lua`,
  `{{root guideline-rust}}/hooks/x.lua`) gives no diagnostic and reads no other pack.
- `runs_into_failed_packs_are_skipped` (`hook_runs.rs`): `core` failed to load, and
  `{{root core}}/hooks/enforce.lua` gives no diagnostic.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): resolve cross-pack token targets and hook runs`.

### Task 3 — Resolve option values

**Files:**
- Create `crates/agentstacks/src/model/option_value.rs`, `crates/agentstacks/src/model/option_values.rs`.
- Modify `crates/agentstacks/src/model/set_error.rs`, `crates/agentstacks/src/model/mod.rs`.

**Change:** `OptionValue` is `Boolean`, `Integer` (i64) or `String`. Its `Display` is the §4.4
rendering: boolean as `true`/`false`, integer in decimal, string as-is. The rendered value is
never re-scanned (§3.3).

`resolve_options` takes one loaded pack's key, its `options` declarations (plan 05's
`OptionDecl`s) and the raw `options` table of its `PackEntry` (plan 08). It returns every option's
`OptionValue`, or every refusal at once (§10). §4.4's leading `core:` stands for the location,
which §10 now gives, so each message is the rest of the §4.4 text:

| §4.4 case | Result | Location |
|---|---|---|
| set, right type | the set value | — |
| not set, `default` present | the default | — |
| not set, no `default` | `option "<name>" is required` | `agentstacks.toml: packs.<key>` |
| wrong type | `option <name> expects <type>, got <type>` | `agentstacks.toml: packs.<key>.options.<name>` |
| not declared by the pack | `no option "<name>"` | `agentstacks.toml: packs.<key>.options.<name>` |

In the wrong-type message, `expects` uses `OptionType`'s type word and `got` the TOML type word
of plan 05's `toml_fields` (`boolean`, `integer`, `float`, `string`, `datetime`, `array`,
`table`).

**Proof:**
- `set_error_displays_each_spec_message`: adds rows for `option "x" is required`, `option
  archive_keep expects integer, got string` and `no option "archive_kep"`.
- `option_value_renders_per_spec` (`option_value.rs`): `true` and `false` render as `true` and
  `false`; `20` and `-3` render in decimal. The strings `{{skill x}}` and the empty string render
  unchanged.
- `options_resolve_per_spec_table` (`option_values.rs`): table-driven over every §4.4 case, pack
  key `core`. `archive_keep = 50` resolves to integer 50; an unset `archive_keep` with default 20
  resolves to 20. An unset `x` with no default gives `agentstacks.toml: packs.core: option "x" is
  required`. `archive_keep = "many"` gives `agentstacks.toml: packs.core.options.archive_keep:
  option archive_keep expects integer, got string`. `archive_kep = 1` gives `agentstacks.toml:
  packs.core.options.archive_kep: no option "archive_kep"`.
- `options_type_mismatch_names_the_toml_type` (`option_values.rs`): `archive_keep = 1.5` gives
  `agentstacks.toml: packs.core.options.archive_keep: option archive_keep expects integer, got
  float`; a string option `label = true` gives `agentstacks.toml: packs.core.options.label: option
  label expects string, got boolean`.
- `options_report_every_error_at_once` (`option_values.rs`): a pack entry with one required option
  unset, one wrong type and one undeclared key gives those three diagnostics.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): resolve pack option values`.

### Task 4 — Intersect airsl ranges

**Files:**
- Create `crates/agentstacks/src/model/version_interval.rs`, `crates/agentstacks/src/model/airsl_range.rs`.
- Modify `crates/agentstacks/src/model/set_error.rs`, `crates/agentstacks/src/model/mod.rs`.

**Change:** `VersionInterval` is the half-open interval of **release** versions that one
comparator matches. Its lower bound is inclusive; its upper bound is exclusive or unbounded.
Construction covers the 8 operators of §1.1 (Exact, Greater, GreaterEq, Less, LessEq, Tilde,
Caret, Wildcard), with major-only and major.minor forms. `VersionInterval` also intersects with
another interval and reports emptiness.

`AirslReq` (plan 05) admits only those 8 operators (§4.3: an operator added by a later `semver` is
refused at `pack.toml` parse). The extra match arm that `#[non_exhaustive]` forces (§1.1) is
unreachable under that invariant. It is documented as such per the strong-types rule "validated
parse, then trust". It is not a fallback.

`AirslRange` is built from the `(pack key, AirslReq)` pairs of the loaded packs that declare
`requires.airsl`, kept in pack-name order. Each pair's interval is the intersection of its
comparators (`AirslReq::as_version_req`); `*` has none and is unbounded (§1.1, §4.3). The set's
interval is the intersection of every pair's interval. Both messages below list every pack that
declares `requires.airsl`, in name order, each range as written (§9):

- **Empty intersection.** Refused at `agentstacks.toml` alone, with the message `airsl: <p1>
  needs <r1>, <p2> needs <r2>` (one `<p> needs <r>` per pack, joined by `, `).
- **Installed version.** `AirslRange::admits(installed)` checks an installed `semver::Version`
  against every pack's range with `semver`'s own matching. A version outside the range is refused
  at `agentstacks.toml` alone, with the message `airsl <installed> installed, packs need <r1>,
  <r2>` (the ranges joined by `, `).

Probing the binary belongs to `core-plan-and-apply` (§4.3), so `admits` is not called by `check`.

**Proof:**
- `set_error_displays_each_spec_message`: adds rows for `airsl: core needs >=0.2, sdlc needs <0.2`,
  `airsl 0.1.3 installed, packs need >=0.1.4` and `airsl 0.1.3 installed, packs need >=0.1.4,
  <0.3`.
- `interval_matches_semver_for_every_operator` (`version_interval.rs`): table-driven, one row per
  operator form: `=1.2.3`, `=1.2`, `=1`, `>1.2.3`, `>1.2`, `>=0.1.4`, `>=0.1`, `<0.2`, `<1.2.3`,
  `<=1.2.3`, `<=1.2`, `~0.1.4`, `~1.2`, `~1`, `^1.2.3`, `^0.2.3`, `^0.0.3`, `^0.0`, `^0`, `0.1`,
  `1.2.*`, `1.*`. For every release version in the probe list `0.0.0, 0.0.3, 0.0.4, 0.1.0, 0.1.3,
  0.1.4, 0.1.9, 0.2.0, 0.2.3, 0.3.0, 1.0.0, 1.1.9, 1.2.0, 1.2.3, 1.2.4, 1.3.0, 2.0.0, 3.0.0`,
  membership in the interval equals `semver::VersionReq::matches` for that row's range.
- `interval_intersection_reports_emptiness` (`version_interval.rs`): `>=0.1.4` ∩ `^0.1` is
  non-empty and admits 0.1.4 but not 0.2.0. `>=0.2` ∩ `<0.2` is empty.
- `ranges_intersect_per_cargo_semantics` (`airsl_range.rs`): `>=0.1.4` with `^0.1` is non-empty;
  `~0.1.4` with `>=0.2` is empty; `*` with `>=0.1.4` is non-empty. `*` alone admits 0.0.0 and
  3.0.0. One pack declaring `>=0.1, <0.3` admits 0.2.3 but not 0.3.0.
- `disjoint_ranges_are_refused_naming_each_pack` (`airsl_range.rs`): `core` `>=0.2` with `sdlc`
  `<0.2` gives exactly one diagnostic, `agentstacks.toml: airsl: core needs >=0.2, sdlc needs
  <0.2`. Adding `team-lint` `>=0.1` gives exactly `agentstacks.toml: airsl: core needs >=0.2, sdlc
  needs <0.2, team-lint needs >=0.1`, packs in name order whatever order they were loaded in.
- `installed_version_must_satisfy_every_range` (`airsl_range.rs`): installed 0.1.3 against one
  pack's `>=0.1.4` gives `agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4`; 0.1.4
  passes. Against `core` `>=0.1.4` and `sdlc` `<0.3`, 0.1.3 gives `agentstacks.toml: airsl 0.1.3
  installed, packs need >=0.1.4, <0.3` and 0.3.0 gives `agentstacks.toml: airsl 0.3.0 installed,
  packs need >=0.1.4, <0.3`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): intersect airsl ranges`.

### Task 5 — Detect name clashes

**Files:**
- Create `crates/agentstacks/src/model/name_clash.rs`.
- Modify `crates/agentstacks/src/model/set_error.rs`, `crates/agentstacks/src/model/mod.rs`.

**Change:** `check_name_clashes` works over the loaded packs. Every skill name shipped by two or
more packs gives one diagnostic, at `agentstacks.toml` alone, listing the packs in name order
(§9 item 6): for two packs `skill "<name>" is shipped by both <p1> and <p2>`; for three or more
`skill "<name>" is shipped by <p1>, <p2> and <p3>` (all but the last joined by `, `, then ` and `
before the last). Agents are checked the same way, with `agent` in place of `skill`. A skill and an
agent may share a name.

**Proof:**
- `set_error_displays_each_spec_message`: adds rows for `skill "status" is shipped by both sdlc
  and team-lint`, `agent "coder" is shipped by both core and team-lint` and `skill "status" is
  shipped by sdlc, team-lint and x`.
- `same_skill_in_two_packs_is_refused` (`name_clash.rs`): `sdlc` and `team-lint` both shipping
  skill `status` gives exactly `agentstacks.toml: skill "status" is shipped by both sdlc and
  team-lint`.
- `same_agent_in_two_packs_is_refused` (`name_clash.rs`): `core` and `team-lint` both shipping
  agent `coder` gives exactly `agentstacks.toml: agent "coder" is shipped by both core and
  team-lint`.
- `skill_and_agent_may_share_a_name_across_packs` (`name_clash.rs`): skill `x` in one pack and
  agent `x` in another give no diagnostic.
- `three_or_more_packs_give_one_diagnostic_per_name` (`name_clash.rs`): `x`, `team-lint` and
  `sdlc` all shipping skill `status` give exactly one diagnostic, `agentstacks.toml: skill
  "status" is shipped by sdlc, team-lint and x`. Four packs `a`, `b`, `c`, `d` shipping agent `y`
  give exactly `agentstacks.toml: agent "y" is shipped by a, b, c and d`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): detect skill and agent name clashes`.

### Task 6 — Assemble `pack_set::check` and the end-to-end fixtures

**Files:**
- Create `crates/agentstacks/src/model/valid_set.rs`, `crates/agentstacks/src/model/pack_set.rs`,
  `crates/agentstacks/tests/pack_set.rs`, `crates/agentstacks/tests/fixtures/sets/**`.
- Modify `crates/agentstacks/src/model/mod.rs` (declare `pub mod pack_set`; re-export `ValidSet`),
  `crates/agentstacks/src/lib.rs` (crate docs name `model::pack_set::check` and `ValidSet` as the
  entry points of format validation).

**Change:** `check` collects the load diagnostics of every failed entry into one plan 01
`Diagnostics`. It then runs, in order:

1. `check_fetched_names`.
2. `check_requires`.
3. `check_token_targets`.
4. `check_hook_runs`.
5. `resolve_options` for every loaded pack.
6. The `AirslRange` intersection.
7. `check_name_clashes`.

Every check sees only loaded entries, so a failed load produces no follow-on diagnostic, while
every independent error is still reported. With no diagnostic and no failed entry, `check`
returns `Ok(ValidSet)` holding the project, the packs and extensions by key, the resolved options
per pack and the `AirslRange` (absent when no pack declares one). Otherwise it returns the `Err`
of `Diagnostics::into_result`, sorted per §10.

The fixtures are one directory per case under `crates/agentstacks/tests/fixtures/sets/`. Each
holds an `agentstacks.toml` with `path` sources, plus `packs/` and `extensions/`:

- `valid-realistic`
- `pack-key-mismatch` and `tool-key-mismatch`
- `requires-unlisted` and `requires-cycle`
- `target-not-required`, `target-missing-agent` and `target-missing-skill-dir`
- `option-token-undeclared`
- `hook-run-missing` and `hook-run-excluded`
- `option-required`, `option-wrong-type` and `option-undeclared`
- `airsl-disjoint`
- `skill-clash`, `skill-clash-three` and `agent-clash`
- `failed-pack`

**Proof:**
- `valid_set_answers_option_and_airsl_queries` (`valid_set.rs`): built through `check` from a
  passing set, the set returns `core`'s `archive_keep` as integer 50 and its unset
  `style_reinject` as its default `true`. It has an `AirslRange` when one pack declares
  `requires.airsl`, and none when no pack does.
- `check_runs_every_check_and_sorts_the_result` (`pack_set.rs`): a set breaking one rule of each
  §9 check, plus a fetched-name mismatch, returns every one of those diagnostics, sorted per §10.
- `check_suppresses_follow_on_errors_from_failed_loads` (`pack_set.rs`): `core` failed to load,
  while `sdlc` requires it. `sdlc`'s `{{agent core/coder}}`, its `run = "{{root
  core}}/hooks/x.lua"` and a skill name shared with `core`'s broken directory produce no
  diagnostic. The result holds `core`'s load diagnostics.
- `check_still_reports_unrelated_errors_beside_a_failed_load` (`pack_set.rs`): with `core` failed,
  `sdlc`'s `{{skill guideline-rust/rust-guidelines}}` (not required) and a `sdlc`/`team-lint`
  skill clash are both still reported, with their spec messages at their §10 locations.
- `check_never_returns_a_valid_set_after_a_failed_load` (`pack_set.rs`): a project whose only
  problem is one failed extension load returns `Err` holding exactly that load's diagnostics.
- `check_treats_a_missing_load_result_as_failed` (`pack_set.rs`): a `[packs.*]` key absent from
  the map gives no follow-on diagnostic and no `ValidSet`.
- `check_accepts_a_realistic_project` (`tests/pack_set.rs`): fixture `valid-realistic` contains:
  - a `core`-like pack with both options, lua and message hooks and `enforcement.json` in
    `render`;
  - an `sdlc`-like pack requiring it, with cross-pack `skill`, `agent`, `skill_dir` and `root`
    tokens and an `option` token naming its own declared option;
  - a `guideline`-like pack running `{{root core}}/hooks/enforce.lua` from its own hook and from
    `native/claude/hooks.toml`;
  - a minimal `claude` extension.

  `check` returns a `ValidSet` with the resolved options.
- `check_reports_each_set_refusal` (`tests/pack_set.rs`): table-driven over every invalid fixture
  above. Each yields exactly one diagnostic whose `Display` is the location plus the spec message
  below; `<fx>` is the fixture path, and token and `run` locations start with the pack directory
  passed to `Pack::load`:
  - `pack-key-mismatch` → `agentstacks.toml: packs.core: source contains pack "kore"`;
  - `tool-key-mismatch` → `agentstacks.toml: tools.claude: source contains extension "claud"`;
  - `requires-unlisted` → `agentstacks.toml: packs.sdlc: sdlc requires pack "core", which
    agentstacks.toml does not list (add it: --pack core)`;
  - `requires-cycle` → `agentstacks.toml: requires cycle: a → b → a`;
  - `target-not-required` → `<fx>/packs/sdlc/skills/plan/SKILL.md:68: {{skill
    guideline-rust/rust-guidelines}} targets pack "guideline-rust", which sdlc does not require.
    Add it to requires.packs or remove the ref.` (the §3.3 example after the fixture prefix);
  - `target-missing-agent` → `<fx>/packs/sdlc/skills/a/SKILL.md:6: pack "core" has no agent
    "codr"`;
  - `target-missing-skill-dir` → `<fx>/packs/sdlc/skills/a/SKILL.md:6: pack "core" has no skill
    "coder"`;
  - `option-token-undeclared` → `<fx>/packs/core/skills/a/SKILL.md:6: pack "core" has no option
    "archive_kep"`;
  - `hook-run-missing` → `<fx>/packs/guideline-rust/pack.toml: hooks[1].run: "{{root
    core}}/hooks/enforc.lua" does not exist in core`;
  - `hook-run-excluded` → `<fx>/packs/guideline-rust/pack.toml: hooks[0].run: "{{root
    core}}/hooks/old.lua" does not exist in core`;
  - `option-required` → `agentstacks.toml: packs.core: option "x" is required`;
  - `option-wrong-type` → `agentstacks.toml: packs.core.options.archive_keep: option archive_keep
    expects integer, got string`;
  - `option-undeclared` → `agentstacks.toml: packs.core.options.archive_kep: no option
    "archive_kep"`;
  - `airsl-disjoint` → `agentstacks.toml: airsl: core needs >=0.2, sdlc needs <0.2`;
  - `skill-clash` → `agentstacks.toml: skill "status" is shipped by both sdlc and team-lint`;
  - `skill-clash-three` → `agentstacks.toml: skill "status" is shipped by sdlc, team-lint and x`;
  - `agent-clash` → `agentstacks.toml: agent "coder" is shipped by both core and team-lint`.
- `check_skips_follow_on_errors_from_failed_packs` (`tests/pack_set.rs`): in fixture
  `failed-pack`, `core`'s `pack.toml` has `name = "core"` and `api = 1`. The result holds that
  load diagnostic, `<fx>/packs/core/pack.toml: core: api 1 not supported (this agentstacks speaks
  0)` (plan 05's form), and the unrelated `agentstacks.toml: skill "status" is shipped by both sdlc
  and team-lint`, and nothing about `sdlc`'s `{{agent core/coder}}`.

**Done when:** `cargo make dod` and `cargo make deny` pass. Commit `feat(agentstacks): validate the
whole pack set`.

---

## Risks

- The interval conversion is the riskiest step, because caret and tilde bounds differ below
  `1.0`. `interval_matches_semver_for_every_operator` compares every interval with
  `VersionReq::matches` over a fixed probe list, so the conversion cannot drift from the crate's
  semantics. Intervals cover release versions only. The installed check uses `matches` directly,
  so `semver` alone decides how a pre-release version is treated.
- Suppressing follow-on errors could hide independent errors in healthy packs.
  `check_still_reports_unrelated_errors_beside_a_failed_load` and the `failed-pack` fixture assert
  that they are still reported.
- The airsl messages quote each range as written (§9). Plan 05's `AirslReq` exposes only
  `as_version_req()`, and `semver`'s `Display` normalises some spellings (`>= 0.1.4` prints
  `>=0.1.4`, `0.1` prints `^0.1`). The tests above use ranges whose written and displayed forms
  agree. The written text must come from `AirslReq`; until plan 05 exposes it, a pack writing a
  non-canonical range would be quoted in its normalised form.
- Double reporting of one bad token: plan 06 checks own-pack targets and this plan checks only
  other-pack targets and option existence, so each token gets at most one target refusal.
  `own_pack_targets_are_left_to_pack_load` holds this plan to its side.

## Proof

- `cargo make dod` and `cargo make deny` green.
- Coverage map (spec §12, "every refusal in this spec has a test"). Each spec refusal maps to the
  plan, task number and tests that assert it.

| Spec row (message) | Plan · task (tests) |
|---|---|
| §2.1 `not a pack (no pack.toml)` | 06 · 1 (`classify_refuses_a_directory_without_pack_toml`), 06 · 6 (`load_stops_at_a_missing_pack_toml`), 06 · 7 (fixture `no-pack-toml`) |
| §2.1 `reserved, not supported in api 0` | 06 · 1 (`classify_reports_each_reserved_directory_once`), 06 · 7 (fixtures `reserved-commands`, `reserved-rules`) |
| §2.1 `missing SKILL.md` | 06 · 1 (`classify_refuses_a_skill_directory_without_skill_md`), 06 · 7 (`skill-missing-skill-md`) |
| §2.1 `only skill directories belong in skills/`; skill dir `"My_Skill" is not a valid name` | 06 · 1 (`classify_refuses_files_directly_in_skills`, `classify_refuses_invalid_skill_directory_names`), 06 · 7 (`skills-stray-file`, `skill-bad-dir-name`) |
| §2.1 `agents/ holds only <name>.md files`; agent stem `"Coder" is not a valid name` | 06 · 1 (`classify_refuses_non_md_files_and_subdirectories_in_agents`, `classify_refuses_invalid_agent_stems`), 06 · 7 (`agents-stray-file`, `agents-subdirectory`, `agent-bad-stem`) |
| §2.1 / §5 `name "x" does not match "y"` | 04 · 4 and 04 · 5 (`name_must_match_expected`); 06 · 7 (`skill-name-mismatch`, `agent-name-mismatch`) |
| §2.1 `pack declares nothing to install` | 06 · 4 (`pack_with_only_root_files_has_nothing_to_install`, `excluded_or_empty_native_hooks_do_not_count`), 06 · 7 (`nothing-to-install`) |
| §2.1 `symlinks are not allowed in a pack` / `… in an extension`; `path is not valid UTF-8` | 03 · 2 (`walk_refuses_a_symlinked_file`, `walk_names_the_extension_in_the_symlink_refusal`), 03 · 1 (`walk_error_not_utf8_displays_the_spec_message`); 06 · 6 (`load_passes_walk_refusals_through`); 07 · 3 (`extension_load_refuses_symlinks_with_extension_wording`) |
| §2.1 `only regular files and directories are allowed` | 03 · 2 |
| §2.2 `only tool directories belong in native/`; `"Claude" is not a valid name` | 06 · 1 (`classify_refuses_files_directly_in_native`, `classify_refuses_invalid_native_tool_names`), 06 · 7 (`native-stray-file`, `native-bad-tool`) |
| §2.2 `neutral content must not reference native/` | 06 · 5 (`native_reference_is_refused_after_a_root_token`, `native_reference_in_a_hook_field_keeps_its_key_path`), 06 · 7 (`neutral-references-native`) |
| §2.2 native `hooks.toml` rules (grants, policy, `may_block`, `exec`, `run`; `agents` unknown key) | 05 · 7 (`native_hook_keeps_the_neutral_rules`, `native_hook_refuses_event_and_match_shapes`) |
| §3.2 unknown kind, unterminated, empty, needs a target, takes one target, not a valid target, not a valid name, `root` takes a pack name, unknown var, not a valid option name, option tokens this pack only | 02 · 3 (`scan_refuses_each_spec_example`, `scan_applies_option_rows_to_field_text`) |
| §3.2 `var "arguments" is only valid in SKILL.md` | 02 · 3 (`placement_refuses_arguments_outside_skill_md`); 06 · 5 (`arguments_var_is_refused_outside_skill_md`), 06 · 7 (`arguments-outside-skill`) |
| §3.2 / §3.3 own-pack `pack "core" has no skill "coder"` / `has no agent "codr"` | 06 · 5 (`own_pack_targets_must_exist`), 06 · 7 (`own-skill-dir-is-an-agent`, `own-agent-missing`) |
| §3.2 / §3.3 other-pack `pack "core" has no skill "coder"` / `has no agent "codr"` | 10 · 2 (`missing_items_are_refused_per_kind`), 10 · 6 (`target-missing-agent`, `target-missing-skill-dir`) |
| §3.2 / §9 item 3 `pack "core" has no option "archive_kep"` | 10 · 2 (`option_tokens_must_name_a_declared_option`), 10 · 6 (`option-token-undeclared`) |
| §3.3 `… targets pack "guideline-rust", which sdlc does not require. Add it to requires.packs or remove the ref.` | 10 · 2 (`targets_outside_requires_are_refused_with_the_spec_message`, `targets_are_checked_wherever_the_token_was_found`), 10 · 6 (`target-not-required`) |
| §3.4 quoting hint `(a value starting with {{ must be quoted)` | 04 · 3 (`quoting_hint_on_value_starting_with_token`, `quoting_hint_on_list_item_starting_with_token`, `quoting_hint_on_flow_sequence_item`) |
| §3.4 / §5 `name must be a literal` | 04 · 4 (`name_must_be_a_literal`) |
| §3.4 `render: "x.lua" is code; code is installed verbatim` | 05 · 3 (`pack_manifest_refuses_render_entries`) |
| §3.4 `not valid UTF-8, cannot render` | 06 · 5 (`rendered_text_refuses_invalid_utf8`), 06 · 6 (`load_refuses_a_non_utf8_render_file`), 06 · 7 (`render-not-utf8`) |
| §4.2 unknown key | 05 · 1 (`toml_fields_reports_every_unknown_key`), 05 · 6 (`pack_manifest_refuses_an_unknown_hook_key`) |
| §4.2 `pack.name`, `pack.version`, `pack.api` (both forms), `pack.description` | 05 · 3 (`pack_manifest_refuses_each_pack_field`) |
| §4.2 `requires.packs` twice / self; `requires.airsl` not a range; `requires.binaries` twice | 05 · 3 (`requires_refuses_each_rule`), 05 · 2 (`airsl_req_refuses_a_non_range`) |
| §4.2 `requires.airsl: required (the pack contains .lua files)` | 06 · 4 (`installed_lua_requires_requires_airsl`, `lua_in_a_skill_or_native_dir_counts`), 06 · 7 (`lua-without-airsl`) |
| §4.2 `render` relative / inside the pack | 05 · 2 (`pack_path_refuses_absolute_and_escaping_paths`), 05 · 3 (`pack_manifest_refuses_render_entries`) |
| §4.2 `render` does not exist / also excluded | 06 · 3 (`render_entry_must_exist`, `render_entry_must_not_be_excluded`), 06 · 7 (`render-missing`, `render-excluded`) |
| §4.2 `exclude` glob / relative | 05 · 2 (`exclude_glob_refuses_invalid_and_absolute_globs`), 05 · 3 (`pack_manifest_refuses_exclude_entries`) |
| §4.2 `exclude` excludes a protected file; render-only refusal on overlap | 06 · 3 (`exclude_must_not_match_protected_files`, `exclude_matching_a_rendered_protected_file_reports_only_the_render_refusal`), 06 · 7 (`exclude-protected-*`) |
| §4.2 `options.<n>` name; `.type`; `.default`; `.description` | 05 · 4 (`pack_manifest_refuses_bad_option_names_and_shapes`, `option_type_refuses_others`, `option_decl_refuses_each_rule`) |
| §4.3 `uses an operator this agentstacks does not support` | 05 · 2 (`unsupported_operator_message_names_the_range`) |
| §4.3 / §9 `airsl: core needs >=0.2, sdlc needs <0.2` | 10 · 4 (`disjoint_ranges_are_refused_naming_each_pack`), 10 · 6 (`airsl-disjoint`) |
| §4.3 / §9 `airsl 0.1.3 installed, packs need >=0.1.4` (and joined ranges) | 10 · 4 (`installed_version_must_satisfy_every_range`) |
| §4.4 `option "x" is required`; `option archive_keep expects integer, got string`; `no option "archive_kep"` | 10 · 3 (`options_resolve_per_spec_table`, `options_type_mismatch_names_the_toml_type`, `options_report_every_error_at_once`), 10 · 6 (`option-required`, `option-wrong-type`, `option-undeclared`) |
| §4.4 rendering | 10 · 3 (`option_value_renders_per_spec`) |
| §4.5 `not an extension (no extension.toml)` | 07 · 3 (`extension_load_refuses_a_directory_without_a_manifest`) |
| §4.5 name / version / description; `extension.api`; `entry` outside / not `.lua`; unknown key | 07 · 1 (`entry_path_refuses_paths_leaving_the_extension`, `entry_path_refuses_non_lua_files`), 07 · 2 (`extension_manifest_refuses_an_invalid_name`, `extension_manifest_refuses_a_non_semver_version`, `extension_manifest_refuses_an_empty_description`, `extension_manifest_refuses_api_other_than_zero`, `extension_manifest_refuses_bad_entries`, `extension_manifest_refuses_unknown_keys`) |
| §4.5 `entry` does not exist | 07 · 3 (`extension_load_refuses_a_missing_entry`) |
| §5 `missing frontmatter` | 04 · 2 (`split_refuses_missing_frontmatter`) |
| §5 YAML syntax error with and without a line; duplicate key; null frontmatter | 04 · 3 (`line_is_offset_to_file_line`, `no_location_gives_file_only_diagnostic_without_hint`, `duplicate_key_is_a_syntax_error`, `empty_frontmatter_is_refused_as_null`) |
| §5 `name` / `description` required, empty; unknown key; `invocation`; `argument_hint` | 04 · 4 (`skill_meta_refuses_each_rule`, `unknown_keys_each_reported_in_order`, `invocation_refuses_other_values`) |
| §5 `model`; `effort`; `tools` unknown / twice | 04 · 5 (`model_tier_refuses_sonnet`, `effort_refuses_max`, `agent_tool_refuses_unknown_values`, `agent_meta_refuses_each_rule`) |
| §5 `native: "Claude" is not a valid tool name` | 04 · 4 (`native_keys_must_be_tool_names`) |
| §6.2 `kind`, `event`, `match` ×4, `agents` only on `agent.stop` | 05 · 6 (`hook_action_refuses_a_missing_or_unknown_kind`, `hook_event_refuses_unknown_events`, `hook_refuses_each_selector_rule`) |
| §6.2 message hook `text` / `run` / `args` / `grants` / `may_block` | 05 · 6 (`message_hook_refuses_each_rule`) |
| §6.2 lua `text`; `may_block` type | 05 · 6 (`lua_hook_refuses_each_rule`) |
| §6.2 lua `run` required / `.lua` / form | 05 · 5 (`hook_run_refuses_a_missing_run`, `hook_run_refuses_a_non_lua_file`, `hook_run_refuses_other_forms`) |
| §6.2 `policy`, grant path, `env`, `exec` | 05 · 5 (`grants_refuse_each_rule`), 05 · 2 (`env_name_refuses_other_shapes`, `policy_refuses_other_values`) |
| §6.2 `hooks[3].agents: pack "core" has no agent "task-briefer"` | 06 · 2 (`hook_agents_must_name_own_agents`), 06 · 7 (`hook-agents-unknown`) |
| §6.2 own-pack `run` does not exist | 06 · 2 (`missing_own_run_is_refused`, `native_hook_runs_are_checked_as_own_pack_runs`), 06 · 7 (`hook-run-missing`, `native-hook-run-missing`) |
| §6.2 other-pack `"{{root core}}/hooks/enforc.lua" does not exist in core` | 10 · 2 (`other_pack_run_must_exist_in_that_pack`, `other_pack_run_must_not_be_excluded_there`, `other_pack_run_must_be_a_root_file_there`, `other_pack_run_is_checked_in_native_hooks`), 10 · 6 (`hook-run-missing`, `hook-run-excluded`) |
| §7 `api`; nothing to install; table keys | 08 · 3 (`project_file_refuses_api_rules`, `project_file_refuses_nothing_to_install`, `project_file_refuses_invalid_table_keys`) |
| §7 git/path, tag/rev, `rev` not a commit id, URL `-`, `subdir`, tag/rev/subdir only on git, `path` absolute / inside, empty string | 08 · 1 (`git_url_refuses_a_leading_dash_or_empty`, `commit_id_refuses_other_forms`, `in_repository_refuses_escape_and_leading_slash`, `in_project_refuses_absolute_and_climbing_paths`), 08 · 2 (`source_fields_refuses_git_and_path_together_or_neither`, `source_fields_refuses_git_only_fields_on_a_path_source`, `source_fields_locates_field_errors_at_their_key_path`, `exactly_one_of_tag_rev_refuses_neither_or_both`), 08 · 3 (`project_file_applies_source_rules_to_every_entry`) |
| §7 `options` on tools; unknown key | 08 · 3 (`project_file_refuses_unknown_keys`) |
| §7 `packs.core: source contains pack "kore"`; `tools.claude: source contains extension "claud"` | 10 · 1 (`pack_key_must_equal_fetched_name`, `tool_key_must_equal_fetched_name`), 10 · 6 (`pack-key-mismatch`, `tool-key-mismatch`) |
| §7 / §10 `packs.sdlc: sdlc requires pack "core", … (add it: --pack core)` | 10 · 1 (`unlisted_requirement_is_refused_with_the_add_hint`), 10 · 6 (`requires-unlisted`) |
| §7 `requires cycle: a → b → a` | 10 · 1 (`each_cycle_is_reported_once_from_its_smallest_name`, `cycle_prints_the_shortest_path_back_to_the_smallest_name`), 10 · 6 (`requires-cycle`) |
| §7 shorthand `"x" is not an official pack; pass a git URL or a path` | handed to `cli-commands` (§7, §13); not in this chain |
| §8 `api`; `airsl.version`; syntax error; unknown key | 09 · 4 (`lockfile_refuses_top_level_rules`, `lockfile_reports_a_syntax_error_alone`) |
| §8 entry `rev` / `tree` required, git/path, tag/rev/subdir on path, `name`, `version` | 09 · 2 (`locked_entry_refuses_each_rule`) |
| §8 `file[0].path` relative / inside; `merge[0].key: empty` | 09 · 3 (`file_entry_refuses_each_rule`, `merge_entry_refuses_each_rule`) |
| §8 `file … listed twice`; `merge … listed twice`; `pack "core" listed twice`; `tool "claude" listed twice` | 09 · 4 (`lockfile_refuses_duplicates`) |
| §8 `by` not a locked pack or tool | 09 · 4 (`lockfile_checks_by_against_tools_and_packs`) |
| §8 `[airsl] missing …` / `[airsl] present …` | 09 · 6 (`airsl_presence_refuses_a_missing_section`, `airsl_presence_refuses_an_unneeded_section`) |
| §8 / §8.1 `bad hash (expected sha256:<64 lowercase hex>)`; tree hash example | 03 · 3 (`content_hash_refuses_bad_forms`), 03 · 4 (`tree_hash_matches_the_spec_example`); 09 · 3 (`file_entry_refuses_each_rule`) |
| §9 item 6 `skill "status" is shipped by both sdlc and team-lint`; agent form; three or more packs | 10 · 5 (`same_skill_in_two_packs_is_refused`, `same_agent_in_two_packs_is_refused`, `three_or_more_packs_give_one_diagnostic_per_name`), 10 · 6 (`skill-clash`, `skill-clash-three`, `agent-clash`) |
| §9 failed loads produce no follow-on errors | 10 · 6 (`check_suppresses_follow_on_errors_from_failed_loads`, `check_still_reports_unrelated_errors_beside_a_failed_load`, `failed-pack`) |
| §10 generic shapes; `Diagnostic` `Display`; set-level locations; sorting | 01 · 2 (`set_level_with_key_displays_project_file_then_key`, `set_level_without_key_displays_project_file`), 01 · 3, 01 · 4, 01 · 5 (`into_sorted_mixed_files_and_lines`) |
| §10 TOML syntax line from span; empty TOML refused field by field | 05 · 1 (`parse_document_locates_a_syntax_error_by_line`, `parse_document_reads_empty_text_as_an_empty_table`), 05 · 3 (`pack_manifest_refuses_an_empty_file_field_by_field`) |
| §10 I/O failure `<path>: <io error>` | 01 · 4 (`io_error_at_file`) |
