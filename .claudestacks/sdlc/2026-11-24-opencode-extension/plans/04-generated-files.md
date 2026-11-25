---
status: done
created: 2026-11-25
built-on: d54f8422f1ff
---

# OpenCode Generated Files Implementation Plan

**Goal:** Pure Lua modules produce the exact text of every file the opencode extension generates.

**Architecture:** `extensions/opencode/lib/plugin.lua` holds the spec §4.5 header and fixed body as
long-string constants and renders the entry table between them; `extensions/opencode/lib/translate.lua`
returns the spec §4.7 translator text verbatim, as `extensions/claude/lib/translate.lua` does for
Claude. Neither calls `agentstacks.*` or `require`. Their behaviour in OpenCode is proven by plans 05
and 06; here the texts are pinned.

**Tech Stack:** Lua under airsl 0.1.5; the generated JavaScript (runs in OpenCode's Bun) and Lua.

**Content authority:** spec §4.5 (text, junction, entry encoding), §4.7 (translator text), §8.1.

---

## Files that change

```
extensions/opencode/lib/plugin.lua            — create: agentstacks.js text from a list of entries
extensions/opencode/lib/plugin_test.lua       — create: tests for plugin
extensions/opencode/lib/translate.lua         — create: the translator text
extensions/opencode/lib/translate_test.lua    — create: tests for translate
```

## Order of work

### Task 1 — `plugin`

**Files:**
- Create `extensions/opencode/lib/plugin.lua`, `extensions/opencode/lib/plugin_test.lua`.

**Change:** `M.text(entries)` returns: the spec §4.5 header lines through `const HOOKS = [`, then each
entry as `airsstack.json.encode(entry)` one per line, indented two spaces, joined by `,\n  `, then
`];`; with no entries the table line is `const HOOKS = [];`. Then one empty line, then the spec §4.5
fixed body byte for byte, ending `};\n`. The body uses `stdio: ["pipe", "pipe", "ignore"]` and holds the airsl notice of spec D17 (`AIRSL_MISSING`, the exit-127 check, the `told` flag).

**Proof:**
- `empty_table_is_an_empty_array` — `M.text({})` contains `const HOOKS = [];` and no `{}` table.
- `header_is_exact` — the text up to the table equals the spec §4.5 header.
- `body_is_exact` — the text after the table's `];` and one empty line equals the spec §4.5 fixed body.
- `two_entries_joined` — two entries render on two lines joined by `,\n  ` with keys sorted.
- `text_entry_is_json_escaped` — an entry text holding `"` and a newline is encoded as JSON (`\"`, `\n`).
- `ends_with_newline` — the text ends with `};` and a newline (`};\n`).

**Done when:** `airsl test extensions` reports `0 failed` and `cargo make dod` is green. Commit `feat(repo): opencode hook shim text`.

### Task 2 — `translate`

**Files:**
- Create `extensions/opencode/lib/translate.lua`, `extensions/opencode/lib/translate_test.lua`.

**Change:** `M.text()` returns the spec §4.7 translator text byte for byte.

**Proof:**
- `text_is_a_non_empty_string`, `text_defines_event_and_result` (`function M.event(p)` and `function M.result(r, p)` present), `text_ends_by_returning_the_module` — as `extensions/claude/lib/translate_test.lua` does.
- `text_names_opencode` — the first line is `-- agentstacks translator for opencode, api 0.`.

**Done when:** `airsl test extensions` reports `0 failed` and `cargo make dod` is green. Commit `feat(repo): opencode translator text`.

---

## Risks

- A long-string literal can change bytes (a leading newline after `[==[` is dropped by Lua). The
  exact-text tests are the guard; plan 06's real-OpenCode run is the final one.
- Riskiest step: Task 1's byte-exact body; the test compares against the spec text pasted into the
  test file, which must itself be copied without change.

## Proof

- `airsl test extensions` reports `0 failed`; `cargo make dod` green.

## Review findings

Batch review, 2026-11-25 (`claudestacks:reviewer`, 0🔴). `airsl test extensions` → 122 passed, 0 failed; workspace `cargo make dod` → exit 0. `translate.lua` text compared with `cmp` against the spec §4.7 block: identical, 2937 bytes.

- open 🔵 `extensions/opencode/lib/plugin.lua:1` — the header comment's `<root>` means the project root, unlike `values.lua`'s root.
- open 🔵 `extensions/opencode/lib/plugin_test.lua:3` — "spec" vocabulary in test source (same as the claude extension's precedent).
- open 🔵 `extensions/opencode/lib/translate_test.lua:14` — the tests pin text only; behaviour is left to plans 05 and 06 (the reviewer ran it on airsl 0.1.5 and it behaved as written).

## Deviations

- 2026-11-25 — Task 1, `empty_table_is_an_empty_array`: the "no `{}` table" check covers the text before the fixed body only, because the fixed body itself contains `catch {}`; `body_is_exact` pins the body bytes. The reviewer judged it sound.
- 2026-11-25 — Not committed: the execute instruction did not ask for commits.
