---
status: done
created: 2026-11-27
---

# Intent: Devex improvements from first-time setup

## Problem

First-time setup of agentstacks 1.0.0 (airsl 0.1.5, macOS) succeeds and `doctor` reports
healthy, but six communication frictions bury the signal and leave avoidable half-done or
silent-failure states, as reported in GitHub issue #17:

1. `plan` prints one line per file (77 lines for one pack on two tools); new pins, skips and
   shared-config diffs are buried.
2. The git cache hardcodes `~/.airsstack/cache` under `HOME`; sandboxed agent environments
   cannot write there, the failure gives no discoverable workaround, and `AIRSSTACK_HOME` —
   already honoured by hooks and packs — is ignored by the CLI cache.
3. `doctor` checks drift only; a missing or wrong `airsl` runtime fails hooks silently at
   session start instead of being reported, with a remedy, at doctor time.
4. Files written by `apply` carry no generated-file stamp; a maintainer learns the file is
   managed only after `doctor` flags their edit as drift.
5. A successful `apply` prints no activation hint; in the already-running session a correct
   setup appears to do nothing, because hooks, skills and agents load at session start.
6. `init` writes `agentstacks.toml` before planning; when the plan stage then fails, the user
   is left with a half-done state to clean up or complete manually.

## Affected systems

- `crates/agentstacks-cli`: `plan`, `apply`, `init`, `doctor` presentation and flags.
- `crates/agentstacks`: plan rendering inputs, cache-home resolution (`types`, `fs`,
  `process`), the doctor flow, and the init/plan flows.
- `extensions/claude`, `extensions/musecode`, `extensions/opencode` and `packs/` where
  generated-file stamping or hook runtime behaviour is concerned.
- `book/`: CLI, plan-and-apply and first-project chapters that document the changed output.

## Desired outcome

All six improvements from issue #17 are delivered in the reporter's priority order
(1 plan summary, 2 cache home override, 3 doctor runtime checks, 4 generated-file stamps,
5 activation hint, 6 `init --dry-run`), each with its user-visible behaviour documented in
the book and covered by the project's own tests. The issue reporter could repeat the
first-time setup and hit none of the six frictions.

## Constraints

- The extension API stays `api = 1`; any API change is additive only.
- The core stays tool-agnostic: no tool's file shapes in `types/`, `app/` or `host/`.
- Extensions still only propose; `apply` remains the only writer, and every change still
  appears in the plan.
- `doctor`'s documented offline character (no network, no git, no extension) is a settled
  contract; changing it is a design decision, not an assumption.
- Work follows this pack's chain (intent → design/spec → plan → execute) and the Rust
  guideline pack's Definition of Done (`cargo make dod`).

## Non-goals

- No new tools, packs or extensions.
- No change to the pack, extension, project or lock formats beyond what a single item's
  approved spec requires.
- Pre-committing solution choices in this intent: whether the plan summary is the default
  or opt-in, the exact stamp scope per file format, the doctor runtime scope, and the
  dry-run plumbing are settled in design, with probes, not here.
