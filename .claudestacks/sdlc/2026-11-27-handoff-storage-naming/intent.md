---
status: done
created: 2026-11-27
---

# Intent: Handoff storage path still uses legacy cc/plugins/claudestacks naming

## Problem

Context Handoff session reports are stored under
`<worktree-root>/.airsstack/cc/plugins/claudestacks/handoff/<session-id>/<NN>-<agent>-<slug>.md`,
as reported in GitHub issue #18 of airsstack/agentstacks. Both path segments are
legacy: `cc` names Claude Code and `claudestacks` names the archived predecessor
project that agentstacks superseded. The same protocol now runs under Claude
Code, Muse Code and OpenCode sessions — it was observed under the `musecode`
extension on agentstacks 1.0.0 — so a Muse Code session mints its handoff tree
under a Claude-Code/claudestacks-named path. The path implies the feature is
Claude Code-only, and new users meeting it have no way to connect it to
agentstacks documentation. Per-project `.gitignore` entries and docs end up
enshrining the legacy name.

## Affected systems

- The `core` pack's Context Handoff protocol: the session-tree path owned by
  `packs/core/scripts/lib/handoff.lua`, its prose mirror
  `packs/core/skills/context-handoff/references/protocol.md`, and the report
  validator `packs/core/scripts/lib/handoff_report.lua` that recognises report
  paths under the session tree.
- Tests that pin the path: the core pack Lua suites and the `agentstacks-cli`
  pack integration tests.
- Any consuming project that already has a handoff tree under the legacy path,
  including its sessions in flight when the rename ships.
- The `sdlc` and `journal` packs only indirectly: they reuse the core pack's
  handoff check hook and follow the same protocol.

## Desired outcome

Handoff session reports are minted under a tool-neutral storage path that names
neither a single tool nor the archived predecessor, under every supported tool
extension. Existing trees under the legacy path heal without manual cleanup by
the project owner, and reports written before the rename — including any from a
session still in flight across the upgrade — remain recognised by the report
validator.

## Constraints

- The extension API is frozen as `api = 1` with additive-only 1.x changes, so
  the fix must ship inside the `core` pack without breaking that promise.
- The legacy tree is ephemeral, git-ignored session scratch (the handoff
  `.gitignore` line covers `.airsstack/`), never committed history; healing it
  must respect the existing session liveness lease so a live session is never
  deleted under its orchestrator.
- The Rust core and the three tool extensions do not own this path today; the
  rename must not move ownership out of the `core` pack.
- Code and prose for the protocol must stay in agreement, as both sides of the
  protocol state today.

## Non-goals

- Renaming the committed `.claudestacks/sdlc/` SDLC chain root or the
  `claudestacks-sdlc` label; that is tracked separately in GitHub issue #19.
- Renaming any other legacy-named state, such as the enforce hook's
  `cc/plugins/sdd` root or its sentinel prefix.
- Changing the handoff file schema, the session-id or report-file naming inside
  a session, the lease, keep-count or grace-window behaviour, or the three
  protocol tiers beyond where their paths point.
- Changing historical references to the archived claudestacks project in the
  book, changelogs or git history.
