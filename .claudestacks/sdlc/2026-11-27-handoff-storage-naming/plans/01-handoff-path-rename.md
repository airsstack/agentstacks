---
status: done
created: 2026-11-27
built-on: d9b30845813d
---

# Tool-Neutral Handoff Path Implementation Plan

**Goal:** Move Context Handoff minting to `.airsstack/handoff`, heal legacy trees at `init`, and keep pre-rename reports valid — spec sections 2–4 (D1–D4, R1–R7, E1–E8).

**Architecture:** No new files. `lib/handoff.lua` owns paths and healing; `lib/handoff_report.lua` owns recognition; `protocol.md` mirrors both; one Rust test fixture follows the canonical path.

**Tech Stack:** Lua under airsl 0.1.5 (core pack), Rust integration test in `agentstacks-cli`, gates `cargo make lua` and `cargo make dod`.

**Content authority:** spec.md sections 2–7 of chain `2026-11-27-handoff-storage-naming`.

---

## Files that change

```
packs/core/scripts/lib/handoff.lua                  — modify: canonical + legacy constants, legacy prune (keep 0) and empty-parent cleanup in init
packs/core/scripts/lib/handoff_report.lua           — modify: both segments recognised in under_handoff_root and session_tier
packs/core/scripts/handoff_test.lua                 — modify: six new healing tests (spec section 5)
packs/core/scripts/handoff_report_test.lua          — modify: canonical literals move; two legacy-segment tests added
packs/core/skills/context-handoff/references/protocol.md — modify: canonical path (2 mentions) plus one legacy-recognition sentence
crates/agentstacks-cli/tests/packs.rs               — modify: write_reports fixture writes under the canonical path
```

## Order of work

### Task 1 — Canonical path and legacy healing in the session owner

**Files:**
- Modify `packs/core/scripts/lib/handoff.lua` — `M.HANDOFF_REL` becomes `.airsstack/handoff`; new `M.LEGACY_HANDOFF_REL` names the legacy path; `init` additionally prunes the legacy base and climbs-remove empty legacy parents.
- Test `packs/core/scripts/handoff_test.lua` — the six tests named in spec section 5.

**Change:** After the canonical prune inside `init`, run `prune` on the legacy base with keep 0 and the caller's grace (R2/D2), then for the legacy base and each parent up to but not including `.airsstack`: stop at the first level that is missing, not a directory, unreadable, or non-empty; otherwise `pcall(fs.remove_dir, level)` and continue upward (R3). Every healing step is guarded so no legacy failure can fail `init` (R4). Session id, lease, keep/grace defaults and the `.gitignore` line do not change (R1). `beat`/`close` in `lib/handoff.lua` and the `beat`/`end` subcommands of `scripts/handoff.lua` are not touched, so a legacy-minted session keeps heartbeating and closing under its old path (R7). Healing runs under the same resolved root as canonical minting, so a linked worktree heals its own legacy tree and still writes no `.gitignore` (E6).

**Proof:** written first, seen failing, then passing, each building its fixture from `handoff.LEGACY_HANDOFF_REL`:
- `init_prunes_every_stale_legacy_session` — more stale legacy sessions than the canonical keep count are all removed by one `init` (R2/D2).
- `a_live_legacy_lease_survives_init` — a legacy session holding a fresh lease, and its parent chain, survive `init` (E3).
- `init_removes_the_empty_legacy_parents` — with the legacy sessions gone, `handoff/`, `claudestacks/`, `plugins/` and `cc/` are removed and `.airsstack` remains (R3).
- `init_leaves_a_nonempty_legacy_parent` — a file in `plugins/` keeps `plugins/`, `cc/` and the file, while the emptied levels below are removed (R3/E5).
- `a_legacy_base_that_is_a_file_is_left_untouched` — `init` returns its canonical directory and the file is untouched (E2/R4).
- `init_with_no_legacy_tree_mints_as_before` — canonical minting, output and `.gitignore` are unchanged when no legacy tree exists (E1).
Unmodified existing tests keep pinning the untouched behaviour: `a_heartbeat_refreshes_a_lease_and_reports_a_missing_session` and `closing_drops_the_lease_and_leaves_the_session` (R7), and `a_linked_worktree_keeps_its_gitignore_untouched` with `a_linked_worktree_still_mints_its_session` (E6).

**Done when:** `airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" packs/core/scripts` shows the six new tests passing beside the untouched suite, and `cargo make lua` is green. Commit `feat(packs): mint handoffs under .airsstack/handoff and heal legacy trees`.

### Task 2 — Validator recognises both session segments

**Files:**
- Modify `packs/core/scripts/lib/handoff_report.lua` — `under_handoff_root` and `session_tier` test both segment constants from `lib/handoff.lua`.
- Test `packs/core/scripts/handoff_report_test.lua` — canonical literals updated; legacy tests added.

**Change:** Both functions gain the `handoff.LEGACY_HANDOFF_REL` plain substring test beside the existing `handoff.HANDOFF_REL` one — the constant lives on the `lib/handoff` module (required here as `handoff`), not on this module (R5). `under_handoff_root` keeps its session-before-fallback order; the `/handoff/` and temp-root tests are untouched (R6), and `resolve_against` is untouched, so a relative report path still resolves against the hook event's `cwd` and a relative path with no `cwd` still does not qualify (E7). A hand-built path containing both segments qualifies under the dual tests — the safe direction, since it is checked rather than ignored (E8; the constants themselves cannot mint such a path, probe F5).

**Proof:** written first, seen failing, then passing:
- `a_legacy_session_tree_path_is_under_the_root` — an absolute legacy session path qualifies (R5).
- `a_legacy_session_tier_report_needs_session_and_seq` — a report under the legacy tree without `session:`/`seq:` earns both violations (R5/E3).
Every hardcoded legacy literal in `handoff_report_test.lua` moves to the canonical segment — the `under_handoff_root`, `path_in`, `path_in_root` and `report_path` groups alike — except the two new legacy tests above, which keep the old segment deliberately; unmodified relative-path cases (`a_relative_session_path_resolves_against_cwd_and_qualifies`, `a_relative_path_with_no_cwd_does_not_qualify`, and the `path_in_root`/`report_path` cwd cases) keep pinning E7.

**Done when:** the same `airsl test` invocation as Task 1 and `cargo make lua` are green. Commit `feat(packs): keep legacy handoff reports session-tier`.

### Task 3 — Prose mirror, Rust fixture, and the full gate

**Files:**
- Modify `packs/core/skills/context-handoff/references/protocol.md` — path layout and tier-table rows name `.airsstack/handoff`; one added sentence states the legacy segment remains recognised as session-tier (spec section 1).
- Modify `crates/agentstacks-cli/tests/packs.rs` — `write_reports` writes under `.airsstack/handoff/s`.

**Change:** Prose and code agree on the canonical path and on legacy recognition; the Rust integration fixture exercises the hooks over the shipped canonical layout, as it does over the legacy layout today.

**Proof:** `cargo test -p agentstacks-cli --test packs core_report_checks` passes (hooks still block the bad report and pass the good one, now over the canonical path), a grep for `cc/plugins/claudestacks` over `packs/core` and `crates/agentstacks-cli/tests` returns hits only at the `LEGACY_HANDOFF_REL` constant, the two new legacy tests that name it, and the legacy sentence in `protocol.md`, and `cargo make dod` is green end to end.

**Done when:** all three proofs above hold in one run. Commit `docs(packs): mirror the tool-neutral handoff path in protocol and fixtures`.

---

## Risks

- The accepted check-then-remove window (spec D3/E4) is the riskiest behaviour: it is bounded and documented, but Task 1's tests cannot reproduce a microsecond race; they pin the sequential behaviour only.
- A missed hardcoded legacy string would leave a test minting the old tree; Task 3's closing grep is the guard.
- The pack text gates (`Makefile.toml` `[tasks.lua]`, run by `cargo make lua`) reject `claudestacks(-[a-z]+)*:[a-z]` style names in pack prose. Task 1's `cargo make lua` surfaces a violation in the legacy constant's comment; the `protocol.md` legacy sentence lands in Task 3, whose `cargo make dod` (which includes the lua step) is the first gate that can surface it there.

## Proof

- `cargo make lua` green after each task (Lua check + all pack suites + text gates).
- `cargo test -p agentstacks-cli --test packs core_report_checks` green in Task 3.
- `cargo make dod` green once, at the end (fmt, clippy, rustdoc, workspace tests, doctests, lua).

---

## Review findings

- Reviewer (handoff `05-reviewer-diff.md`, one round over the whole diff): Tasks 1–2 behaviour and Task 3 prose/fixture match the plan — fixed: its one blocking finding, that the closing grep proof failed on extra hardcoded legacy literals (`lib/handoff.lua` climb literal and four `handoff_test.lua` literals). A fresh coder derived the climb levels from `M.LEGACY_HANDOFF_REL` and the test parent paths from the same constant (handoff `06-coder-fix1.md`). Re-verified by the orchestrator: `airsl test` over `packs/core/scripts` 253 passed / 0 failed, `cargo make lua` green, and `grep -rn "cc/plugins/claudestacks" packs/core crates/agentstacks-cli/tests` returns exactly the constant, the deliberate legacy test literal, and the protocol sentence.
- Design-stage review (spec) and plan-stage review each ran their single `artifact-reviewer` round; every 🔴/🟡 finding was applied (see the design and plan records of this chain).

## Deviations

- 2026-11-27 — Execute reviewed the whole diff in one reviewer round instead of one review per task batch. The three tasks are sequentially dependent (Task 2 consumes Task 1's constant, Task 3 mirrors both), so no two could batch; one review over the combined diff judged the same code the three batch reviews would have.
- 2026-11-27 — The reviewer's blocking fix improved on the plan's Task 1 text: the legacy parent climb is derived from `M.LEGACY_HANDOFF_REL` rather than spelling parent paths, leaving the legacy string in exactly one source location.
- 2026-11-27 — Task 2's test-first record is narrower than the plan's phrasing: the moved canonical cases did not fail before the validator change (Task 1's constant already carried them, and both segments contain `/handoff/`); the one falsifying case was the legacy session-tier test, which failed then passed as required. Recorded in handoff `03-coder-task2.md`.
- 2026-11-27 — `cargo make dod` in the shell sandbox fails one unrelated test (`fs::disk::tests::a_unix_socket_is_other`, sandbox denies `UnixListener` bind); the same test and the full `cargo make dod` pass unsandboxed (Task 3 coder, handoff `04-coder-task3.md`, 53.01s). The orchestrator re-ran `cargo make lua`, the pack Lua suite, the grep proof and `cargo test -p agentstacks-cli --test packs core_report_checks` green in-sandbox.
- 2026-11-27 — No commits were made: the execute instruction did not ask for them. The plan's three commit subjects remain the suggested split for the author's commits.
