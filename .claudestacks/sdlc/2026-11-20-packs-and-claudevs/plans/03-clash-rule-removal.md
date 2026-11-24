---
status: done
created: 2026-11-23
built-on: 0f89fdb3ebb3
---

# Clash rule removal Implementation Plan

**Goal:** Let two packs in one project ship a skill or an agent of the same name.

**Architecture:**
- The cross-pack name clash rule lives in `types/set/clash.rs` and is called once from
  `types/set/check.rs:129`. Removing the module, its call, its error variant and its docs removes
  the rule. No other module changes (spec §5, §10).
- A tool whose extension gives two items the same path is still refused, by the existing
  "already proposed" rule (`types/op/proposals.rs:405-424`; spec §1, §4).

**Tech Stack:** Rust (`crates/agentstacks`).

**Content authority:** spec §1 (clash rows), §5 first bullet, §8 "Removed", §10, §11 (core row).

---

## Files that change

```
crates/agentstacks/src/types/set/clash.rs        — delete: the rule and its unit tests
crates/agentstacks/src/types/set/mod.rs          — modify: drop `mod clash;` and its doc line (:15)
crates/agentstacks/src/types/set/check.rs        — modify: drop the import (:35), the call (:129), the doc line (:13), clash lines in unit tests
crates/agentstacks/src/types/set/error.rs        — modify: drop `SetError::NameClash`, its display helper, its test cases
crates/agentstacks/src/types/pack/load.rs        — modify: drop "clashing names" from the doc line (:44)
crates/agentstacks/tests/set.rs                  — modify: remove three refusal cases; new test for shared names; failed-pack expectation
crates/agentstacks/tests/fixtures/sets/skill-clash/  — rename to shared-skill/
crates/agentstacks/tests/fixtures/sets/agent-clash/  — rename to shared-agent/
crates/agentstacks/tests/fixtures/sets/skill-clash-three/ — delete
```

## Order of work

### Task 1 — Remove the rule and its error

**Files:**
- Delete `crates/agentstacks/src/types/set/clash.rs`.
- Modify `types/set/mod.rs`, `types/set/check.rs`, `types/set/error.rs`, `types/pack/load.rs`.

**Change:**
- `set/mod.rs`: `mod clash;` goes; the `//!` line at `:15` no longer lists `clash`.
- `set/check.rs`:
  - the `use crate::types::set::clash::check_name_clashes;` line and the
    `diagnostics.extend(check_name_clashes(…))` call go;
  - the `//!` line at `:13` no longer lists "name clashes";
  - `check_runs_every_check_and_sorts_the_result` (`:197-234`) drops the line
    `"agentstacks.toml: skill \"status\" is shipped by both sdlc and team-lint"` from its expected
    list, and `check_still_reports_unrelated_errors_beside_a_failed_load` (`:250-264`) drops the
    same line. No other unit test in `check.rs` names the rule.
- `set/error.rs`: the `NameClash` variant, the `Shippers` display helper and any import used only by
  them (`ItemKind`, if unused after) go, along with the `NameClash` cases of
  `set_error_displays_the_airsl_and_clash_messages`. That test keeps its airsl cases and is renamed
  `set_error_displays_the_airsl_messages`.
- `set/error.rs:14-15`: the `//!` list of refusals drops "a skill or agent that two packs ship".
- `pack/load.rs:44`: the `//!` sentence lists required packs, targets in other packs and airsl
  ranges, without "clashing names".

**Proof:**
- `check_runs_every_check_and_sorts_the_result` passes with the clash line gone from its expected
  list.
- `check_still_reports_unrelated_errors_beside_a_failed_load` passes with two lines.
- `set_error_displays_the_airsl_messages` passes.
- `grep -rn 'NameClash\|check_name_clashes\|is shipped by' crates/agentstacks/src` prints nothing.

**Done when:** `cargo make dod-crate agentstacks` is green. Commit
`refactor(agentstacks): drop the cross-pack name clash rule`.

### Task 2 — Turn the clash fixtures into shared-name fixtures

**Files:**
- `git mv` `tests/fixtures/sets/skill-clash` → `shared-skill`, `agent-clash` → `shared-agent`.
- Delete `tests/fixtures/sets/skill-clash-three/`.
- Modify `crates/agentstacks/tests/set.rs`.

**Change:**
- The refusal cases `skill-clash`, `skill-clash-three` and `agent-clash` (`tests/set.rs:173-184`)
  leave the case table.
- A new test, `packs_may_share_skill_and_agent_names`, runs the set check on `shared-skill` (sdlc and
  team-lint both ship skill `status`) and on `shared-agent` (core and team-lint both ship agent
  `coder`), and asserts each passes with no diagnostic.
- `check_skips_follow_on_errors_from_failed_packs` (`:195-213`) expects exactly one line, the
  `core: api 1 not supported (this agentstacks speaks 0)` line. Its comment about sort order is
  removed with the second line; the test still shows that no follow-on error from the failed pack
  is reported.

**Proof:**
- `packs_may_share_skill_and_agent_names` — both fixtures load with an empty diagnostic list.
- `check_skips_follow_on_errors_from_failed_packs` — the refusal list equals the one api line.
- The case-table test still passes for every remaining fixture.

**Done when:** `cargo test -p agentstacks --test set` passes and `cargo make dod` is green. Commit
`test(agentstacks): packs may share skill and agent names`.

---

## Risks

- `check_runs_every_check_and_sorts_the_result` exercises many rules at once; dropping one line must not reorder the
  rest. Only the clash line leaves the list.
- `docs/` may describe the rule. `grep -rn 'shipped by' docs/ README.md` runs in Task 1; any hit is
  removed in the same commit.

## Proof

- `cargo make dod` green.
- `grep -rn 'clash\|two packs ship' crates/agentstacks/src/types/set` prints nothing.

## Review findings

- 🔵 fixed (follow-up) — `types/set/check.rs`: `failing_core`'s `.skill("status", "body")` and the `team_lint` PackSpec in `check_still_reports_unrelated_errors_beside_a_failed_load` existed only to cause the removed clash line.
- 🔵 fixed (follow-up) — `tests/fixtures/sets/failed-pack/packs/team-lint/` and its `agentstacks.toml` entry now add nothing to `check_skips_follow_on_errors_from_failed_packs`.
- Verification: `cargo make dod-crate agentstacks` exit 0 (lib 1690 passed, `tests/set.rs` 4 passed, 292 doctests); greps `NameClash|check_name_clashes|is shipped by` (src), `clash|two packs ship` (types/set), `shipped by` (docs, README) print nothing. Workspace `cargo make dod` exit 0.

## Deviations

- 2026-11-23 — Tasks 1 and 2 ran in one coder: Task 1's gate needs Task 2's `tests/set.rs` changes.
- 2026-11-23 — Test-first was partial: the first red run failed to compile on plan 04's in-flight `shorthand.rs` change (`[&str; 6]` vs `[&str; 5]`), so the red caused by the still-present rule was not seen; `packs_may_share_skill_and_agent_names` was written after the rule was gone.
- 2026-11-23 — `git status` pairs the renames by content (`skill-clash-three → shared-skill` R, `skill-clash` D); the resulting tree is as planned. No per-task commits (execute does not commit unless told).

## Follow-up fixes

- 2026-11-23 — Every open finding above was fixed in one follow-up pass (three parallel coders, disjoint files). Workspace `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` exit 0: 23 `test result: ok` blocks, 2474 Rust tests passed, lua `71 passed, 0 failed (5 files)`, `Build Done in 55.65 seconds`.
- `check.rs` leftover clash setup removed (`check::` 12 passed); `failed-pack/packs/team-lint` removed with `git rm -r` plus its `agentstacks.toml` entry (`--test set` 4 passed).
