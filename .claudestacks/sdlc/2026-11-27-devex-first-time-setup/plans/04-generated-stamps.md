---
status: done
created: 2026-11-27
built-on: 543ad491d3e5
---

# Generated Stamps Implementation Plan

**Goal:** Stamp extension-owned Markdown and Lua writes with a generated-by header

**Architecture:** Per-extension `lib/stamp.lua` helper called by each adapter on Write content; suffix decides

**Tech Stack:** Lua under airsl 0.1.5

## Files that change

```
extensions/claude/lib/stamp.lua       — create stamp helper
extensions/claude/lib/stamp_test.lua  — create tests
extensions/claude/adapter.lua         — modify stamp Writes
extensions/musecode/lib/stamp.lua     — create (same helper)
extensions/musecode/lib/stamp_test.lua — create
extensions/musecode/adapter.lua       — modify
extensions/opencode/lib/stamp.lua     — create (same helper)
extensions/opencode/lib/stamp_test.lua — create
extensions/opencode/adapter.lua       — modify
```

## Order of work

### Task 1 — Stamp helper and tests (all three extensions)

**Files:**
- Create `extensions/{claude,musecode,opencode}/lib/stamp.lua` — `stamp(path, content)`
- Create `extensions/{claude,musecode,opencode}/lib/stamp_test.lua` — behaviour tests

**Change:** `.md`/`.lua` only, case-sensitive; `.md` after frontmatter when present else first line; `.lua` first line; already-stamped at insertion point (CRLF-aware) left single; empty content gets stamp plus newline; other suffixes untouched.

**Proof:**
- `stamps_markdown_after_frontmatter` — stamp lands after closing `---`
- `stamps_lua_first_line_and_skips_json` — suffix rule
- `does_not_double_stamp` — idempotence incl. CRLF

**Done when:** `cargo make lua` passes for the extensions and the Definition of Done is green. Commit `feat(repo): generated-file stamps in extensions`.

### Task 2 — Adapter wiring

**Files:**
- Modify `extensions/{claude,musecode,opencode}/adapter.lua` — apply `stamp` to every Write proposal

**Change:** Stamp Write content before proposing; Copy untouched. Conformance/ext-test expectations updated where they assert written bytes.

**Proof:**
- extension adapter tests and `agentstacks ext test` conformance pass under `cargo make lua`

**Done when:** `cargo make lua` passes and the Definition of Done is green. Commit `feat(repo): adapters stamp owned writes`.

---

## Risks

- Existing golden tests asserting exact written file bytes (CLI packs/claude/musecode tests) will see stamped bytes; update expectations to stamped content.

## Proof

- `cargo make lua` and `cargo make dod-crate agentstacks-cli` green.

---

## Review findings

- Self-review (inline): stamp.lua in all three extensions, adapter writes stamped, fixtures updated. Verified: cargo make lua (651 passed) and extension integration tests (23+28+20 green). Fixed on review: fmt, clippy `map_or_else`/`String::len`, test `panic!` removal.

## Deviations

- Executed inline in this session instead of parallel coder/reviewer subagent spawns; verification is the plan's own tests plus `cargo make dod` sub-gates.
- `cargo make dod` test stage failed in the sandbox only on `fs::disk::tests::a_unix_socket_is_other`: the sandbox blocks `AF_UNIX bind` everywhere (proven with a Python probe in three directories). Re-run outside the sandbox on 2026-11-27 at the author's direction: the test passes and the full core suite is 1864 passed, 0 failed — environmental, no code fix needed. fmt, clippy -D warnings, rustdoc and doctests (312+1) passed.
- Plans 05–07 of the spec (activation hint, init --dry-run, book) were not drafted or executed in this run; the chain stays open.
