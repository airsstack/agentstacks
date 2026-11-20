---
status: draft
created: 2026-11-20
depends-on: [06, 07, 08, 09]
built-on: e665589a43a9
---

# Pack Set Check Implementation Plan

**Goal:** `pack_set::check` turns a project file plus its loaded packs and extensions into a
`ValidSet`, the only input later stages accept.

**Architecture:** `pack_set.rs` takes the parsed `ProjectFile`, the loaded `Pack`s and
`Extension`s keyed by their `[packs.*]` / `[tools.*]` key, and the set of keys whose load failed.
It runs the spec's six whole-set checks plus the §7 rows that need fetched content, collecting
every diagnostic. A diagnostic whose cause is a pack that failed to load is suppressed (no
follow-on errors). Supporting logic sits in focused files: `requires_graph.rs` (listed check and
cycle detection), `token_targets.rs` (cross-pack and own-pack target resolution for `skill`,
`agent`, `skill_dir`, `root`, `option`), `option_values.rs` (§4.4 resolution into typed
`OptionValue`s), `airsl_range.rs` (intersection of `VersionReq`s by converting each comparator to
a version interval, plus the installed-version check), `name_clash.rs` (same skill or agent name
across packs). `ValidSet` holds the project, packs, extensions, resolved option values and the
combined airsl requirement. `SetError` joins `FormatError`. The end-to-end fixture test drives
`ProjectFile::parse` → `Pack::load` / `Extension::load` → `check`.

**Tech Stack:** Rust 2024, `semver` 1.

**Content authority:** spec §9 (checks 1–6, failed packs skipped), §7 (key = fetched name,
extension api, requires listed, cycles), §3.3 (target rules), §4.3 (airsl range), §4.4 (option
resolution), §6.2 (`run` in a required pack), §12 (coverage rule).

---

## Files that change

```
crates/agentstacks/src/model/mod.rs            — modify  declare and re-export
crates/agentstacks/src/model/set_error.rs      — create  SetError (thiserror)
crates/agentstacks/src/model/requires_graph.rs — create  requires listed + cycle detection
crates/agentstacks/src/model/token_targets.rs  — create  resolve token targets across the set
crates/agentstacks/src/model/option_value.rs   — create  OptionValue (typed resolved value) and its rendering form
crates/agentstacks/src/model/option_values.rs  — create  resolve options per pack (§4.4)
crates/agentstacks/src/model/airsl_range.rs    — create  AirslRange: intersect, satisfied_by
crates/agentstacks/src/model/name_clash.rs     — create  same skill/agent name across packs
crates/agentstacks/src/model/pack_set.rs       — create  ValidSet, check()
crates/agentstacks/src/model/diagnostic.rs     — modify  FormatError gains a Set variant
crates/agentstacks/tests/pack_set.rs           — create  end-to-end fixture test
crates/agentstacks/tests/fixtures/sets/**      — create  one valid project, one invalid project per set refusal
```

## Order of work

### Task 1 — Check requires and names against fetched content

**Files:**
- Create `set_error.rs`, `requires_graph.rs`; modify `mod.rs`, `diagnostic.rs`.

**Change:** each `[packs.<key>]` key equals the loaded pack's `pack.name` (`packs.core: source
contains pack "kore"`) and each `[tools.<key>]` key equals `extension.name` (`tools.claude: source
contains extension "claud"`); an extension's api other than 0 is reported as `tools.claude: api 1
not supported (this agentstacks speaks 0)`; every `requires.packs` entry is a listed pack (`sdlc
requires pack "core", which agentstacks.toml does not list (add it: --pack core)`); no cycle
(`requires cycle: a → b → a`, the cycle printed from its smallest name).

**Proof:**
- `set_refuses_key_name_mismatch_for_packs_and_tools`.
- `set_refuses_unlisted_required_pack_with_the_add_hint`.
- `set_reports_a_cycle_once_from_its_smallest_name` — `b → a → b` is reported as
  `requires cycle: a → b → a`.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): check pack requirements across the set`.

### Task 2 — Resolve token targets, hook targets and options

**Files:**
- Create `token_targets.rs`, `option_value.rs`, `option_values.rs`; modify `set_error.rs`,
  `mod.rs`.

**Change:** for every token of every pack: `skill`, `agent`, `skill_dir` and `root` targets name
the pack itself or a pack in its `requires.packs` (the §3.3 message naming the pack and the add
hint), and the named item exists (`pack "core" has no agent "codr"`; for `skill_dir`, `pack "core"
has no skill "coder"`); `option` tokens name a declared option of this pack. Every lua hook `run`
of the form `{{root <pack>}}/…` names a required pack and an existing, non-excluded `.lua` file in
it (`hooks[1].run: "{{root core}}/hooks/enforc.lua" does not exist in core`). Options resolve per
§4.4: set value of the right type, else default, else `core: option "x" is required`; wrong type
`core: option archive_keep expects integer, got string`; undeclared `core: no option
"archive_kep"`. `OptionValue` renders booleans as `true`/`false`, integers in decimal, strings
as-is.

**Proof:**
- `targets_refuse_packs_outside_requires` — the §3.3 example message for
  `{{skill guideline-rust/rust-guidelines}}` in sdlc.
- `targets_refuse_missing_items` — missing agent, missing skill for `skill_dir`, undeclared
  option token; exact messages.
- `hook_run_in_required_pack_must_exist` — `{{root core}}/hooks/enforc.lua` refused;
  `{{root core}}/hooks/enforce.lua` accepted.
- `options_resolve_per_spec` — table-driven over every §4.4 row, including rendering forms.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): resolve tokens and options across the set`.

### Task 3 — Intersect airsl ranges and detect name clashes

**Files:**
- Create `airsl_range.rs`, `name_clash.rs`; modify `set_error.rs`, `mod.rs`.

**Change:** `AirslRange::intersect(&[(pack, VersionReq)])` converts each comparator (exact, `>`,
`>=`, `<`, `<=`, tilde, caret, wildcard, with Cargo's semantics) into a half-open version
interval and intersects them; an empty result is refused naming each pack and range (`airsl: core
needs >=0.2, sdlc needs <0.2`). `AirslRange::satisfied_by(installed)` refuses `airsl 0.1.3
installed, packs need >=0.1.4`. Name clashes: two packs shipping a skill with the same name, or an
agent with the same name, are refused (`skill "status" is shipped by both sdlc and team-lint`); a
skill and an agent sharing a name are allowed.

**Proof:**
- `airsl_ranges_intersect_per_cargo_semantics` — table-driven: `>=0.1.4` ∩ `^0.1` non-empty;
  `>=0.2` ∩ `<0.2` empty with the spec message; `~0.1.4` ∩ `>=0.2` empty; `*` ∩ `>=0.1.4`
  non-empty.
- `airsl_installed_version_must_satisfy` — `0.1.3` against `>=0.1.4` gives the spec message;
  `0.1.4` passes.
- `name_clash_refuses_same_kind_only` — two packs with skill `status` refused; skill `x` in one
  pack and agent `x` in another accepted.

**Done when:** `cargo make dod-crate agentstacks` passes and the Definition of Done is green.
Commit `feat(agentstacks): check airsl ranges and name clashes`.

### Task 4 — Assemble `check` and the end-to-end fixtures

**Files:**
- Create `pack_set.rs`, `crates/agentstacks/tests/pack_set.rs`, fixtures under
  `crates/agentstacks/tests/fixtures/sets/`; modify `mod.rs`, `crates/agentstacks/src/lib.rs`
  (crate docs name `model::check` and `ValidSet` as the entry points).

**Change:** `check(project, packs, extensions, failed)` runs Tasks 1–3 and returns
`Ok(ValidSet)` or every diagnostic sorted. Diagnostics caused by a key in `failed` are
suppressed.

**Proof:**
- `check_accepts_a_realistic_project` — fixture: a `core`-like pack (options, lua and message
  hooks, an `enforcement.json` in `render`), an `sdlc`-like pack requiring it with cross-pack
  `skill`, `agent`, `skill_dir` and `root` tokens, a `guideline`-like pack running core's
  dispatcher on its own data, and a minimal `claude` extension; `check` returns a `ValidSet` with
  resolved options.
- `check_reports_every_set_refusal` — one fixture per §9 check and per §7 fetched-content row,
  each asserting its exact message.
- `check_skips_follow_on_errors_from_failed_packs` — `core` failed to load; `sdlc`'s `{{agent
  core/coder}}` produces no diagnostic.

**Done when:** `cargo make dod` and `cargo make deny` pass. Commit `feat(agentstacks): validate
the whole pack set`.

---

## Risks

- The airsl interval conversion is the riskiest step, because caret and tilde bounds differ for
  `0.x` versions. The table test cross-checks every computed interval against the `semver`
  crate's own `VersionReq::matches` on the interval's boundary versions, so the conversion cannot
  drift from the crate's semantics. Pre-release versions are not specially handled.
- Suppressing follow-on errors must not hide independent errors in healthy packs; the failed-pack
  test asserts that an unrelated error elsewhere is still reported.

## Proof

- `cargo make dod` and `cargo make deny` green.
- Coverage map (spec §12 rule "every refusal has a test"): §2.1/§2.2 → plans 03, 06; §3.2 →
  plan 02 (syntax) and 06/10 (pack-aware rows); §3.3 → plan 10; §3.4 → plans 04, 05, 06; §4.1 →
  plan 02; §4.2 → plans 05, 06; §4.3 → plan 10; §4.4 → plans 05 (declarations), 10 (values);
  §4.5 → plan 07; §5 → plan 04; §6.2 → plans 05, 06, 10; §7 → plans 08, 10; §8 → plans 03, 09;
  §9 → plan 10; §10 → plan 01.
