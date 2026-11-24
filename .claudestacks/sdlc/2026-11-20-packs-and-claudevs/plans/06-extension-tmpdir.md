---
status: done
created: 2026-11-23
built-on: 0f89fdb3ebb3
---

# Extension tmpdir Implementation Plan

**Goal:** Render the claude extension's `tmpdir` var as `${TMPDIR:-/tmp}`.

**Architecture:** One entry of `VARS` in `extensions/claude/lib/values.lua:14`. `sh` expands
`$TMPDIR` to an empty string when TMPDIR is unset, while `${TMPDIR:-/tmp}` gives `/tmp` (spec §1,
`probes/tmpdir-results.md`). `hooks.lua`'s literal-`$` rule accepts any var form the values hold
(`lib/hooks.lua:89-110`), so the new form passes it unchanged.

**Tech Stack:** Lua under airsl, Rust integration tests, Markdown.

**Content authority:** spec §1 (tmpdir row), §4 (`tmpdir` var row), §12 (`docs/extensions.md:150`).

---

## Files that change

```
extensions/claude/lib/values.lua          — modify: tmpdir = "${TMPDIR:-/tmp}"
extensions/claude/lib/values_test.lua     — modify: every_var_has_its_value
crates/agentstacks/tests/fixtures/claude/*/expected/.claude/settings.json — modify: rendered tmpdir form
docs/extensions.md                        — modify: values example (:150)
```

## Order of work

### Task 1 — Default the tmpdir var to /tmp

**Files:**
- Modify `extensions/claude/lib/values.lua`, `values_test.lua`, `docs/extensions.md`, and each
  expected `settings.json` whose commands hold the rendered `tmpdir` (`full/expected` lines 7 and
  28 hold `$TMPDIR`).

**Change:**
- `VARS.tmpdir = "${TMPDIR:-/tmp}"`.
- In each expected `settings.json`, the argv element that came from `{{var tmpdir}}` reads
  `"${TMPDIR:-/tmp}…"` inside its double quotes. An element that is literal `$TMPDIR` text in a
  fixture pack (not a `{{var}}`) is left as it is.
- `docs/extensions.md:150`: `vars = { tmpdir = "${TMPDIR:-/tmp}" }`.

**Proof:**
- `every_var_has_its_value` asserts `tmpdir = "${TMPDIR:-/tmp}"`; the other five values unchanged.
- New `tmpdir_var_form_passes_the_literal_dollar_rule` in `hooks_test.lua`: a lua hook whose argv
  holds `"${TMPDIR:-/tmp}/x"`, rendered with `vars = { tmpdir = "${TMPDIR:-/tmp}" }`, gives a command
  containing `"${TMPDIR:-/tmp}/x"` and no skip reason.
- `full_pack_renders_every_item_kind` passes against the updated `settings.json`.

**Done when:** `airsl test extensions` passes and `cargo make dod` is green. Commit
`fix(repo): claude tmpdir var falls back to /tmp`.

---

## Risks

- If plan 05 lands first, its expected trees are the base here; if this lands first, plan 05 starts
  from these. Either order works; the two touch different strings.

## Proof

- `cargo make dod` green.
- `grep -rn '"\$TMPDIR' extensions/claude/lib/values.lua` prints nothing.

## Review findings

- No findings on this plan. Verification: red `every_var_has_its_value`, `only_requested_vars_are_returned` → green after `VARS.tmpdir = "${TMPDIR:-/tmp}"`; `grep '"\$TMPDIR' extensions/claude/lib/values.lua` prints nothing; reviewer re-run `cargo make dod` exit 0.

## Deviations

- 2026-11-23 — Ran in the same coder as plan 05, after it (shared files).
- 2026-11-23 — Added `tmpdir_var_form_passes_the_literal_dollar_rule` in `values_test.lua`; green before the change (pins that `hooks.lua` accepts the form).
- 2026-11-23 — `docs/authoring.md:142` and `docs/hooks.md:67` still say `$TMPDIR`: generic var-expression examples, outside this plan's file list.
- 2026-11-23 — Not committed (execute does not commit unless told).
