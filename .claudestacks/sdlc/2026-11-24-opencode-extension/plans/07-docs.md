---
status: done
created: 2026-11-25
depends-on: [02, 05, 06]
built-on: d54f8422f1ff
---

# OpenCode Documentation Implementation Plan

**Goal:** Every repository doc describes the opencode extension as built.

**Architecture:** One new book page, `book/src/ch06-03-opencode.md`, holds the OpenCode reference;
the README, `CLAUDE.md` and the packs changelog name the extension and the check. The API chapters
(plan 01) and the CLI chapters (plan 02) are already done.

**Tech Stack:** mdBook, Markdown.

**Content authority:** spec §6 (rows marked "documented"), §7, §8.5, §9.

---

## Files that change

```
book/src/ch06-03-opencode.md   — create: what the opencode extension writes, hook mapping, limits
book/src/SUMMARY.md            — modify: ch06-03 after ch06-02
book/src/introduction.md       — modify: line 18, OpenCode supported
book/src/ch05-00-plan-and-apply.md — modify: lines 372-373, the missing-airsl notice under OpenCode
README.md                      — modify: status lines 7 and 106, layout line 163, line 149 wording
CLAUDE.md                      — modify: Status, layout lines and 113, Commands, line 66 wording
packs/CHANGELOG.md             — modify: Unreleased Added: the opencode extension
```

## Order of work

### Task 1 — The OpenCode book page

**Files:**
- Create `book/src/ch06-03-opencode.md`.
- Modify `book/src/SUMMARY.md` — `  - [OpenCode](ch06-03-opencode.md)` after the Hooks entry.

**Change:** The page covers: the files the extension writes (spec §4.3 table); vars (§4.1); the hook
mapping and delivery (§4.4, §4.5 tables, without the shim source); every skip reason (§4.6); what is
not carried over from Claude (§7); the limits (§9); and the documented edge rows of §6 — resumed
sessions get `session.start` as `startup`, a `disable-model-invocation` skill is hidden but loadable,
nested agentstacks projects are not supported, the before hook runs before OpenCode's permission
check, and with both tools Claude-only skills appear unless `OPENCODE_DISABLE_CLAUDE_CODE=1` is set.
It names the pinned OpenCode version (v2.4.1) the extension is checked against, and says what a
missing airsl does (spec §4.5 **Missing airsl**, §9): every Lua hook is off, guards included; the
model is told once per OpenCode process, which with the background service means once per service
start, with the exact notice text; an airsl without an execute bit is not told; `doctor` checks the pin.

**Proof:**
- `cargo make book-build` builds with no `[ERROR]` line; the page is in the built book.
- Every skip reason on the page matches `extensions/opencode/lib/hooks.lua` and `frontmatter.lua` text: `grep -F` for each fixed reason, and for the three templated ones (`"<event>"`, `"<arg>"`, `"<key>"`) for the fixed text on each side of the placeholder.
- The notice text on the page equals `AIRSL_MISSING` in `extensions/opencode/lib/plugin.lua` (`grep -F`).

**Done when:** `cargo make book-build` passes and `cargo make dod` is green. Commit `docs(repo): add the OpenCode extension chapter`.

### Task 2 — Project docs

**Files:**
- Modify `README.md` — line 7 "OpenCode is not here yet" and line 106 `| OpenCode | Planned |` name it as built; line 149 "OpenCode plugins are TypeScript" becomes "OpenCode hooks are JavaScript plugins"; line 163 "opencode/ planned" becomes "opencode/ built".
- Modify `CLAUDE.md` — Status heading and layout list `extensions/opencode/`; line 113 "claude is built" names both as built; line 66 wording as in README; Commands lists `cargo make opencode-check` (real OpenCode with a mock model; not part of dod); the CI line lists the `opencode-check` job.
- Modify `book/src/introduction.md` — line 18 "Claude Code is the one supported tool" names Claude Code and OpenCode.
- Modify `book/src/ch05-00-plan-and-apply.md` — lines 372-373: a missing airsl after install is caught by the tool's own error for a missing command (under OpenCode, the shim's one-time notice to the model, linking ch06-03) and by `doctor`.
- Modify `packs/CHANGELOG.md` — `## [Unreleased]` `### Added`: the official `opencode` extension for OpenCode V2.

**Change:** No doc claims OpenCode is planned, unsupported, or that its hooks are TypeScript.

**Proof:**
- `grep -rn "TypeScript" README.md CLAUDE.md book/src` finds no claim that OpenCode plugins are TypeScript.
- `grep -n -e "opencode/ planned" -e "OpenCode is not here yet" -e "| OpenCode | Planned |" README.md` finds nothing.
- `grep -n "one supported tool" book/src/introduction.md` and `grep -n "claude is built" CLAUDE.md` find nothing.

**Done when:** both greps are clean, `cargo make book-build` passes and `cargo make dod` is green. Commit `docs(repo): mark the opencode extension built`.

---

## Risks

- The org docs (`.github/profile/README.md`, workspace `README.md`) live outside this repository and
  change at the `foundation/02` roll-up, not here (spec §11).
- Riskiest step: keeping the page's tables in step with the code; Task 1's grep check covers the skip
  reasons, the rest is reviewed.

## Proof

- `cargo make book-build` and `cargo make dod` green.

## Review findings

Reviews, 2026-11-25 (`claudestacks:reviewer`): Task 1 in batch 1 (0🔴, 2 blocking 🟡), Task 2 in the final batch (0🔴, 1 blocking 🟡). Blocking lines fixed inline by the orchestrator. `cargo make book-build` → "Build Done in 0.96 seconds.", 0 `[ERROR]`; the Task 2 greps find nothing (`grep -n "one supported tool" book/src/introduction.md` → exit 1).

- fixed 🟡 `book/src/ch06-03-opencode.md:39` — wrong cause for a path clash; now: paths join pack and item names with `-`, so only equal joined names or a native file on a generated path clash.
- fixed 🟡 `book/src/ch06-03-opencode.md:110` (and `:50`) — rule 3 now says "non-empty `match`"; no `match` renders.
- fixed 🟡 `book/src/introduction.md:18` — said OpenCode is supported though no `packs-v*` tag holds it; now uses the README's "built and ships with the next `packs-v*` release".
- fixed 🔵 `book/src/ch06-03-opencode.md:63` — "each Lua entry" runs through `/bin/sh -c`.
- fixed 🔵 `book/src/ch06-03-opencode.md:116` — names the allowed native-key characters.
- fixed 🔵 `CLAUDE.md:8,17` — lines past the file's ~100-column wrap.
- fixed 🔵 `CLAUDE.md:101` — "To build a new tool extension (OpenCode, or any other tool)" is stale.
- fixed 🔵 `README.md:134` — "unstable until both official tools run on it": both now do; the freeze is M3.
- fixed 🔵 `packs/CHANGELOG.md:12` — the entry omits the version and api the 0.1.0 entry names.

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

- 2026-11-25 — Task 1 ran in batch 1, before plan 05 Tasks 2-3 and plan 06 (`depends-on: [02, 05, 06]`), because it is docs-only from spec text; its hook and shim sections described Task 2 behaviour ahead of the code until plan 05 Task 2 landed in the same tree.
- 2026-11-25 — Task 1 Proof: a whole-string `grep -F` returns 0 for two fixed reasons, because `extensions/opencode/lib/hooks.lua` builds them by `..` over two lines; the joined text equals the page (reviewer read both halves). The other reasons and `AIRSL_MISSING` pass `grep -F`.
- 2026-11-25 — The blocking review findings were fixed inline by the orchestrator rather than by a fresh coder (three doc lines).
- 2026-11-25 — The coder report for Task 2 (`10-coder-07t2.md`) was returned inline, not written to the handoff session dir.
- 2026-11-25 — Not committed: the execute instruction did not ask for commits.
