---
status: done
created: 2026-11-25
built-on: d54f8422f1ff
---

# Runtime Event Lists Implementation Plan

**Goal:** The shared hook runtime runs a hook once per event when a translator returns a list of events.

**Architecture:** One asset change (`crates/agentstacks/assets/runtime.lua`) replaced by the spec's
text byte for byte; the Rust constant that embeds it (`types/protocol/runtime.rs`) follows with its
pinned hash, length and doc. The API documents that describe the runtime and the translator change
in the same plan, as `CLAUDE.md` requires for ch06-00/01/02.

**Tech Stack:** Lua (airsl 0.1.5), Rust (`agentstacks` crate tests), mdBook.

**Content authority:** spec §3.2, §3.3, §8.2 (runtime cases), §8.5 (the `agentstacks` changelog's runtime line).

---

## Files that change

```
crates/agentstacks/assets/runtime.lua                                  — modify: the spec §3.2 text
crates/agentstacks/src/types/protocol/runtime.rs                       — modify: pinned hash, length, RUNTIME_LUA doc
crates/agentstacks/tests/runtime.rs                                    — modify: four list cases
crates/agentstacks/tests/fixtures/runtime/proj/.tool/agentstacks/translate.lua   — modify: a list branch
crates/agentstacks/tests/fixtures/runtime/proj/.tool/agentstacks/core/hooks/each.lua — create: per-event fixture hook
book/src/ch06-02-hooks.md                                              — modify: §4 runtime text and rules, §6 contract, §9 shim wording
book/src/ch06-01-extensions.md                                         — modify: §14 shim bullet
book/src/ch06-00-authoring.md                                          — modify: step 5 point 4
crates/agentstacks/CHANGELOG.md                                        — modify: Unreleased entry for the runtime
```

## Order of work

### Task 1 — List-aware runtime

**Files:**
- Modify `crates/agentstacks/tests/fixtures/runtime/proj/.tool/agentstacks/translate.lua` — when the payload holds a `paths` array, `M.event` returns a list with one neutral `tool.before` event per path (`tool = "edit"`, `path` = the element), and `M.result` prints `nil` plus a newline for a `nil` result, else `airsstack.json.encode(r)` plus a newline (both fields, as the spec's probe translator did, `probes/runtime-results.md:78-83`). Every payload without `paths` keeps today's `M.event` and `M.result` output, so `runtime_passes_block_when_may_block` and the other existing cases are unchanged.
- Create `crates/agentstacks/tests/fixtures/runtime/proj/.tool/agentstacks/core/hooks/each.lua` — returns `{ context = "each " .. event.path }`, plus `block = "stop " .. event.path` when the path is `stop`, and `nil` when the path is `quiet`.
- Modify `crates/agentstacks/tests/runtime.rs` — the four cases below; the module doc's "five hook modules" (line 4) becomes six.
- Modify `crates/agentstacks/assets/runtime.lua` — replaced by the spec §3.2 block, byte for byte.
- Modify `crates/agentstacks/src/types/protocol/runtime.rs` — `runtime_lua_hash_is_pinned` expects `sha256:1d9e23f6d26d49ae14e28d0425d5d31fd7be5dae7e6c8fa33a3153ba8acde459` and length `1715`; the `RUNTIME_LUA` doc paragraph becomes the spec §3.2 text.

**Change:** A translator may return a list; the runtime runs the hook per element in order, joins
contexts with `\n`, stops at the first block of a `may_block` hook (keeping contexts so far), drops
blocks of other hooks with the existing stderr line, and gives `M.result` `nil` when nothing came
back. Single events and native hooks behave exactly as before.

**Proof:**
- `runtime_list_joins_contexts` — payload `{"paths":["a","b"]}`, hook `each`, not `may_block`: stdout is exactly `{"context":"each a\neach b"}` and a newline.
- `runtime_list_stops_at_the_first_block` — `{"paths":["a","stop","c"]}` with `may_block` 1: stdout is exactly `{"block":"stop stop","context":"each a\neach stop"}` and a newline, so `c` never ran.
- `runtime_list_drops_a_block_when_not_may_block` — same payload with `may_block` 0: stderr holds `agentstacks: hook core.hooks.each tried to block but is not declared may_block; reason: stop stop`; stdout is exactly `{"context":"each a\neach stop\neach c"}` and a newline.
- `runtime_empty_list_gives_nil` — `{"paths":[]}` and `{"paths":["quiet","quiet"]}`: stdout is exactly `nil` and a newline for each, so `M.result` got `nil`, not an empty table.
- `runtime_lua_hash_is_pinned` (existing, updated) — the new hash and length.
- Every existing case in `tests/runtime.rs` and `tests/claude.rs` still passes unchanged.

**Done when:** `cargo test -p agentstacks --test runtime` and `cargo test -p agentstacks --test claude` pass, then `cargo make dod` is green. Commit `feat(agentstacks): let a hook translator return a list of events`.

### Task 2 — Document the list contract

**Files:**
- Modify `book/src/ch06-02-hooks.md` — §4: the new runtime text (byte for byte, as in the asset) and the spec §3.2 rules table; §6: "`M.event(payload)` returns the neutral event, a list of neutral events, or `nil`"; §9: OpenCode hooks are JavaScript plugins (`.js`), and the shim is the extension's ordinary `write`.
- Modify `book/src/ch06-01-extensions.md` — §14 last bullet: "OpenCode plugins are JavaScript or TypeScript; the opencode extension writes a JavaScript shim".
- Modify `book/src/ch06-00-authoring.md` — step 5 point 4: a translator may return a list of events.
- Modify `crates/agentstacks/CHANGELOG.md` — an `## [Unreleased]` section, `### Changed`: the runtime accepts a list of events from a translator; existing projects with a lua hook show `update <root>/runtime.lua` once after upgrading.

**Change:** The three API chapters describe the runtime as built.

**Proof:**
- `cargo make book-build` builds with no `[ERROR]` line.
- The §4 block in `ch06-02-hooks.md` equals `crates/agentstacks/assets/runtime.lua` byte for byte (checked with `diff` against the fenced block).

**Done when:** `cargo make book-build` passes and `cargo make dod` is green. Commit `docs(agentstacks): document translator event lists`.

---

## Risks

- Every existing project's `runtime.lua` changes: the first `plan` after upgrading shows an update.
  That is intended and goes in the changelog; the claude extension's behaviour does not change.
- The asset text must be the spec block byte for byte, or the pinned hash test fails; the hash is the
  check.
- Riskiest step: Task 1's fixture translator change must keep every existing runtime case passing.

## Proof

- `cargo make dod` (fmt-check, clippy, rustdoc, tests, doctests, lua) green.
- `cargo test -p agentstacks --test runtime` runs the list cases on the real airsl 0.1.5.

## Review findings

Batch review, 2026-11-25 (code: `claudestacks:reviewer`, 0🔴 for this plan; docs: `claudestacks:reviewer`, 0🔴). Workspace gate: `cargo make dod` → exit 0, 24 `test result: ok`, 0 FAILED/warning, lua 122 + 651 passed.

- fixed 🟡 `crates/agentstacks/tests/runtime.rs:217` — no test asserts a native hook receives `args`, drops a native non-`may_block` block, or checks per-event args in a list.
- fixed 🟡 `book/src/ch06-02-hooks.md:75-135` — nothing ties the §4 block to the asset; only the asset's hash is pinned (suggest `{{#include}}` or a diff test).
- fixed 🔵 `crates/agentstacks/src/types/protocol/runtime.rs:15` — `RUNTIME_LUA` doc ("stays silent when the translator has none", "stops at the first block") is looser than the code; the text is the spec's, verbatim.
- no change (spec text, hash-pinned) 🔵 `crates/agentstacks/assets/runtime.lua:31` — a single event without an `event` key counts as an empty list; both official translators set `event`. Spec text, hash-pinned.
- fixed 🔵 `crates/agentstacks/tests/fixtures/runtime/proj/.tool/agentstacks/translate.lua:1` — header comment omits the `{"paths":[...]}` payload.
- fixed 🔵 `book/src/ch06-02-hooks.md:155,157,159` — rules table: the single-event row omits the dropped block; the block row implies `context` is always present; the dropped-block-only list case is not stated.
- no change (both statements true) 🔵 `book/src/ch06-02-hooks.md:222` vs `book/src/ch06-01-extensions.md:532` — ".js" vs "JavaScript or TypeScript" (each is its plan's wording); `CLAUDE.md:66` still says OpenCode plugins are TypeScript (plan 07 Task 2 owns it).

### Fix round, 2026-11-25

All open findings above were fixed on the author's instruction ("fix all of findings"), by four
parallel coders plus inline edits; one `claudestacks:reviewer` pass over the fixes (1🔴 F1: the
first `--tool ../agentstacks/extensions/opencode` workaround is refused, `must stay inside the
project`; fixed to copy the extension in and pass `--tool ext/opencode`, the form
`init --tool ext/opencode --pack packs/check --yes` already proved, `applied: 14 written`). The
reviewer's 🟡/🔵 on the fixes (CLAUDE.md claim and wraps, RUNTIME_LUA native-event wording, Lua
lookup comments, `covers` doc, check.mjs header, per-directory PATH links, explicit mock env, stale
`target/opencode-check/2.4.1/opencode`) were fixed inline. Gate after the round:
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` → exit 0, 25 `test result: ok`, lua 122 + 651 passed
0 failed; `cargo make opencode-check` → `opencode v2.4.1`, `ok S1` … `ok S7`; `cargo make
book-build` → 0 `[ERROR]`; `crates/agentstacks/assets/runtime.lua` unchanged.

## Deviations

- 2026-11-25 — Task 2 ran together with plan 02 Task 2 in one coder, because both edit `crates/agentstacks/CHANGELOG.md` (one `## [Unreleased]` holding `### Added` and `### Changed`).
- 2026-11-25 — Not committed: the execute instruction did not ask for commits.
