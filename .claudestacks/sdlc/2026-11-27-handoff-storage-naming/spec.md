---
status: approved
created: 2026-11-27
built-on: 39b1f360410e
---

# Spec: Tool-neutral handoff storage path

This spec delivers the fix for GitHub issue #18 of airsstack/agentstacks:
Context Handoff reports move from the legacy
`.airsstack/cc/plugins/claudestacks/handoff` session tree to a tool-neutral
path, existing legacy trees heal without manual cleanup, and reports written
before the rename stay recognised. It implements the approved intent of chain
`2026-11-27-handoff-storage-naming` and nothing else.

## 1. Architecture

Module ownership is unchanged; no file or module is created, moved or split.
The change is two Lua modules in the `core` pack, their prose mirror, and one
Rust test fixture. This follows the one-concept-per-file rule: the session
tree already has exactly one owner and exactly one validator.

- `packs/core/scripts/lib/handoff.lua` — sole owner of the session-tree path,
  the liveness lease and pruning, as its header states today
  (`packs/core/scripts/lib/handoff.lua:1-9`). It gains the canonical path, a
  legacy-path constant named `M.LEGACY_HANDOFF_REL` beside `M.HANDOFF_REL`,
  and the legacy healing that runs inside `init`. The prose mirror gains
  one sentence naming the legacy segment as still recognised (R5), so the
  mirror and the validator do not disagree about the second root.
- `packs/core/scripts/lib/handoff_report.lua` — sole validator of written
  reports (`packs/core/scripts/lib/handoff_report.lua:1-9`). It recognises
  both the canonical and the legacy session segment so pre-rename reports
  keep their tier.
- `packs/core/skills/context-handoff/references/protocol.md` — the prose
  mirror. Its MUST-agree contract with the Lua is stated at
  `packs/core/skills/context-handoff/references/protocol.md:9-11`, so the two
  path mentions at lines 18 and 58 change in the same change.
- `crates/agentstacks-cli/tests/packs.rs` — test fixture only (line 551). No
  Rust `src/` code changes: `grep -rn "cc/plugins/claudestacks" crates/
  --include="*.rs"` returns only that fixture (Section 6 records the
  searches with controls). The three extensions contain no mention of the
  path either (`grep -rn "cc/plugins" extensions/` finds nothing; the
  control `grep -rln "airsstack_home" extensions/` finds the extension
  value tables); they render the pack's hooks generically, so no extension
  work exists to do.

Logic is tested in its own layer, as today: every new rule is exercised by
the Lua suites in `packs/core/scripts/` against tempdir fixtures, and the
existing Rust integration test in `crates/agentstacks-cli/tests/packs.rs`
keeps proving the hooks end to end over the canonical path.

## 2. Settled decisions

- **D1 — Canonical path is `.airsstack/handoff`.** Author decision at design.
  The `.airsstack/` prefix is already the git-ignored scratch namespace:
  `M.IGNORE_LINE = ".airsstack/"` (`packs/core/scripts/lib/handoff.lua:23`),
  appended by `ensure_gitignore` on every `init`
  (`packs/core/scripts/lib/handoff.lua:102-117`). No `.gitignore` behaviour
  changes under either layout, and the new tail names no tool and no project.
- **D2 — Healing is prune-only and lease-respecting, with no legacy
  retention.** Author decision at design, sharpened at design review.
  Nothing is moved between trees: orchestrators have already handed legacy
  write-paths to agents in briefs, and moving a session directory breaks
  those paths. The legacy base is pruned with keep 0 under the same grace
  and lease rule as canonical pruning: retention exists to bound a tree
  that keeps growing, and the legacy tree never grows again, so retaining
  up to `keep` legacy sessions would leave the common small tree — and
  therefore its parents — in place forever. Every stale legacy session is
  removed at the first post-rename `init`; a live-leased one survives
  until its lease goes stale, then a later `init` removes it.
- **D3 — Empty legacy parents are removed when proven empty, accepting a
  bounded check-then-remove window.** Author decision at design, accepted
  explicitly at design review. `fs.remove_dir` is recursive (probe F1),
  and the runtime offers no empty-only directory removal (probe F8: the
  full `airsstack.fs` surface has no `rmdir`; `fs.remove` fails on
  directories and `fs.rename` moves non-empty directories), so a parent
  is removed only after `fs.list` proves it empty, while accepting that
  an entry created in the window between that check and the removal is
  deleted with it. The only population that can create such an entry is a
  pre-rename build minting a legacy session in the same worktree at that
  moment (its report writes then fail and the protocol's inline-receipt
  fallback carries the result, `protocol.md` Error handling); `.airsstack/cc`
  in a worktree root is created by this legacy path and nothing else in
  this repository (Section 6 searches). The first non-empty level stops
  the climb.
- **D4 — Legacy recognition in the validator is kept indefinitely.** Author
  decision at design. It is one extra plain substring test beside the
  canonical one; plain finds for the two segments do not overlap (probe
  F5), so there is no ambiguity to retire. The change is additive inside a
  pack script, within the frozen `api = 1` additive-only promise stated in
  `CLAUDE.md` (Status section).

## 3. Behaviour

- **R1 — Canonical minting.** `M.HANDOFF_REL` becomes `.airsstack/handoff`
  (today `.airsstack/cc/plugins/claudestacks/handoff`,
  `packs/core/scripts/lib/handoff.lua:17`). `init` mints, leases and prunes
  exactly as today (`packs/core/scripts/lib/handoff.lua:182-195`); only the
  base path changes. Session-id shape, report-file naming
  (`<NN>-<agent>-<slug>.md`), keep default (10) and grace default (120
  minutes) are unchanged (`packs/core/scripts/lib/handoff.lua:18-24`).
- **R2 — Legacy pruning.** Inside the same `init` call, after the canonical
  base is pruned, `prune` runs against the legacy base
  (`path.join(root, M.LEGACY_HANDOFF_REL)`) with keep 0 and the caller's
  `grace_minutes`. With keep 0 every session directory is a candidate and
  `prune` skips only those holding a live lease
  (`packs/core/scripts/lib/handoff.lua:160-177`); `leased` treats a lease
  older than the grace window as abandoned
  (`packs/core/scripts/lib/handoff.lua:151-158`). So the first post-rename
  `init` removes every stale legacy session regardless of count (D2), and
  a live one survives to be removed by a later `init`.
- **R3 — Legacy parent cleanup.** After R2, the legacy base and then each
  parent up to but not including `.airsstack` — that is
  `.airsstack/cc/plugins/claudestacks/handoff`,
  `.airsstack/cc/plugins/claudestacks`, `.airsstack/cc/plugins`,
  `.airsstack/cc` — is removed when, at the moment it is checked, it is a
  directory and `fs.list` returns no entries. The climb stops at the first
  level that is missing, not a directory, unreadable, or non-empty. Every
  removal is `pcall`-guarded; a failure leaves that level and stops the
  climb. `.airsstack` itself is never removed.
- **R4 — No failure of healing fails `init`.** Legacy pruning reuses
  `prune`, whose removals are already individually `pcall`-guarded
  (`packs/core/scripts/lib/handoff.lua:171`), and R3 guards each of its own
  steps. A legacy tree that cannot be read or removed therefore changes
  nothing about the canonical session `init` returns: same directory, same
  printed output (`packs/core/scripts/handoff.lua:80`), same
  `.gitignore` line. This mirrors `close`, which already swallows its
  removal error (`packs/core/scripts/lib/handoff.lua:216-218`).
- **R5 — Validator recognises both segments.** `under_handoff_root` returns
  true for a resolved `.md` path containing either segment, keeping its
  existing resolution and extension rules
  (`packs/core/scripts/lib/handoff_report.lua:101-120`), and
  `session_tier` answers true for either segment
  (`packs/core/scripts/lib/handoff_report.lua:197-205`), keyed off the
  constants in `lib/handoff.lua` rather than a copied string, as today.
  Consequence: a legacy session-tier report still requires `session:` and
  `seq:`, and a legacy path cited in a handback still resolves as the
  report path.
- **R6 — Fallback tier is unchanged.** Two separate functions are
  involved. Inside `under_handoff_root`, the session-segment tests run
  before the `/handoff/` fallback test
  (`packs/core/scripts/lib/handoff_report.lua:118` before `:124`), and
  that order is kept. Tier classification in the separate `session_tier`
  function (`packs/core/scripts/lib/handoff_report.lua:203-205`, called
  from `check` at `:261`) is governed by R5 alone. The temp-root tests
  recognising the `init`-refused and single-subagent tiers
  (`packs/core/scripts/lib/handoff_report.lua:122-141`) are not touched.
  Probe F6 (the legacy path also contains a bare `/handoff/` segment)
  therefore changes nothing: a legacy path qualifies as session-tier by
  the session test in both functions, never merely by the fallback.
- **R7 — `beat` and `end` are unchanged.** They take an explicit session
  directory (`packs/core/scripts/handoff.lua:81-92`), so a session minted
  before the rename keeps heartbeating and closing under its legacy path
  for as long as it lives.

## 4. Edge cases

- **E1 — No legacy tree.** `sessions` returns an empty list when the base
  cannot be listed — it `pcall`s `fs.list` today
  (`packs/core/scripts/lib/handoff.lua:129-145`), and probe F3 confirms
  `fs.list` raises on a missing path. R2 is a no-op and R3 stops at its
  first (missing) level. A fresh project sees no new output or files.
- **E2 — Legacy base is a file, not a directory.** `sessions` filters to
  directories and the `pcall` covers the listing, so R2 removes nothing;
  R3's directory check leaves the file untouched. Nothing under a file can
  be a session, so nothing is lost.
- **E3 — Live legacy session across the upgrade.** Its lease is fresh, so
  R2 skips it (R2's lease rule), R7 keeps its heartbeat working, and R5
  keeps its reports valid. It is pruned by a later `init` once its lease
  is absent or older than the grace window.
- **E4 — Concurrent `init` runs.** Two sessions pruning the same legacy
  base race exactly as two sessions pruning one canonical base race today:
  removals are `pcall`-guarded, and probe F7 shows `fs.remove_dir` on a
  just-removed directory fails rather than erroring the run, so the loser
  of the race removes nothing more. R3's check-then-remove is different
  and is stated honestly (D3): because `fs.remove_dir` is recursive
  (probe F1), an entry created between R3's `fs.list` check and its
  removal is deleted with the level, not left. That window is bounded to
  a pre-rename build minting a legacy session in the same worktree during
  the parent climb of the first post-rename `init`, and its consequence —
  that session's report writes fail and its agents fall back to inline
  receipts (`protocol.md` Error handling) — is the accepted cost of
  removing the legacy parents at all (author decision, design review).
  No new lock is introduced; no lock could close the window, since the
  runtime has no empty-only removal (probe F8).
- **E5 — Content the pack does not own under `.airsstack/cc`.** Another
  tool's directory beside `plugins/claudestacks` makes `.airsstack/cc`
  non-empty, so R3 stops below it and removes nothing it does not own.
  The historical `~/.airsstack/cc/concise.json` mentioned in
  `packs/core/README.md:28` is under the home root, not a worktree root,
  and is never under a legacy base this code visits.
- **E6 — Linked worktree.** `ensure_gitignore` already skips linked
  worktrees (`packs/core/scripts/lib/handoff.lua:96-105`); R2 and R3 run
  under the same resolved root as the canonical prune, so a linked
  worktree heals its own legacy tree and writes no `.gitignore`.
- **E7 — Relative report paths.** Session-tree paths are returned relative
  to the worktree root and resolved against the hook event's `cwd`
  (`packs/core/scripts/lib/handoff_report.lua:68-88`). R5 changes only
  which segments qualify, so a relative legacy path resolves the same way
  a relative canonical one does; a relative path with no `cwd` still does
  not qualify.
- **E8 — A report path containing both segments is impossible from these
  constants.** Probe F5: neither segment is a substring of the other. A
  hand-built path containing both still qualifies under R5, which is the
  safe direction (it is checked, never ignored).

## 5. Testing

New cases land in the suite that already owns each rule; names below are
the tests `execute` writes first and sees failing.

- In `packs/core/scripts/handoff_test.lua` (owner of minting, lease and
  pruning tests today, e.g. `init_mints_a_directory_with_a_live_lease`):
  `init_prunes_every_stale_legacy_session` (keep 0: more stale sessions
  than the canonical keep count all go at one `init`, D2/R2),
  `a_live_legacy_lease_survives_init` (E3),
  `init_removes_the_empty_legacy_parents` (R3 happy path: the full climb),
  `init_leaves_a_nonempty_legacy_parent` (R3 stop: a file in `plugins/`
  keeps `plugins/` and `cc/` and everything above the file's level),
  `a_legacy_base_that_is_a_file_is_left_untouched` (E2, and R4's
  failure-isolation for an unreadable base: `init` still returns its
  canonical directory and the file is untouched),
  `init_with_no_legacy_tree_mints_as_before` (E1). The unreadable-parent
  and removal-failure stop reasons of R3 have no tempdir fixture (no
  permission or failure injection exists in this suite's grants); they
  are pinned structurally by R3's `pcall`-guard-every-step rule and F3/F7
  rather than by a named test — stated here so the plan does not invent
  one. Each fixture builds its tree with the legacy constant, as
  `an_explicit_root_places_the_session_tree` does with
  `handoff.HANDOFF_REL` today (`packs/core/scripts/handoff_test.lua:260-262`).
- In `packs/core/scripts/handoff_report_test.lua` (owner of the root and
  tier tests today): the existing canonical literals move to
  `.airsstack/handoff`, and new cases keep the legacy segment pinned:
  `a_legacy_session_tree_path_is_under_the_root`,
  `a_legacy_session_tier_report_needs_session_and_seq`. The suite's own
  comment at `packs/core/scripts/handoff_report_test.lua:253-254` already
  frames these as the `HANDOFF_REL` segment tests.
- In `crates/agentstacks-cli/tests/packs.rs`, `write_reports` (line 551)
  writes under the canonical path, so
  `core_report_checks_run_on_a_write_and_on_a_handback` and
  `core_report_checks_run_at_agent_stop` keep proving the hooks against
  the shipped layout. (The segment-test framing cited for this suite is
  the `under_handoff_root` group comment at
  `packs/core/scripts/handoff_report_test.lua:253-254`; the tier cases it
  extends sit at `:50-59` and `:138-149`.)
- Gate: `cargo make lua` (the repo's Lua gate, `Makefile.toml` `[tasks.lua]`)
  and `cargo make dod` at the end, per `CLAUDE.md` Commands.

## 6. Probes

Recorded in `probes/airsl-fs-results.md` of this chain, run under `airsl
0.1.5` (the exact runtime the pack pins, `packs/core/pack.toml`
`[requires] airsl = ">=0.1.5"`). The probe invocation
(`airsl test --policy confined --allow-read / --allow-write "${TMPDIR:-/tmp}" <dir>`)
matches the shape of the crate's own Lua gate in `Makefile.toml`
`[tasks.lua]` except that it omits that gate's `--allow-exec git` grant
(immaterial to `fs` semantics; the handoff suite passes either way,
`packs/core/scripts/handoff_test.lua:5-8`) and targets a throwaway
directory instead of `packs`. The negative claims in Sections 1 and 7 are
backed by the searches recorded at the end of the same probes file, each
with a positive control:

- F1: `fs.remove_dir` is recursive over a non-empty directory — the reason
  R3 checks emptiness with `fs.list` before every removal.
- F2: `fs.remove` on a directory fails (`Operation not permitted`) — the
  reason R3 removes through `fs.remove_dir`.
- F3: `fs.list` raises on a file and on a missing path — the reason E1/E2
  rely on the existing `pcall` in `sessions`.
- F4: `fs.mkdir` creates intermediate directories — what R1's canonical
  minting relies on, as the legacy minting did.
- F5: plain substring finds for the two segments do not overlap — the
  reason R5 and E8 are unambiguous.
- F6: the legacy path contains a bare `/handoff/` segment — the reason R6
  keeps the session tests ordered before the fallback test.
- F7: `fs.remove_dir` on a missing path fails (os error 2) — what a second
  concurrent pruner observes, and what R3/R4's `pcall` guards swallow.
- F8: the full `airsstack.fs` surface has no empty-only directory removal
  (`fs.remove` fails on directories, `fs.rename` moves non-empty ones) —
  the reason D3/E4 accept a bounded window instead of promising a
  race-free parent removal.

## 7. Non-goals

- Renaming `.claudestacks/sdlc/` or `claudestacks-sdlc` (issue #19).
- Moving or merging legacy sessions into the canonical tree (D2).
- Any change to session-id shape, report naming, lease mechanics, keep or
  grace defaults, or the exception and `init`-refused tiers' paths.
- Any Rust `src/`, CLI or extension change; any book chapter change (the
  book never names the handoff path — a search for `handoff` in
  `book/src/` finds only hook wiring in `book/src/ch06-07-musecode.md:200`).
- Dropping legacy recognition later; D4 keeps it indefinitely, so no
  removal release needs planning here.
