---
status: approved
created: 2026-11-28
---

# Intent: Pack and extension versions do not track their artifacts

## Problem

Packs and extensions carry version numbers that do not tell a user what code
they are running, as reported in GitHub issue #26 of airsstack/agentstacks.
The `sdlc` pack has shipped real content changes — most recently a 26-file
chain-root rename — while its manifest still declares `1.0.0`, so a user
reading the manifest concludes nothing changed when it did. The three tool
extensions move in lockstep (`claude`, `musecode` and `opencode` all jumped
`1.0.0` to `1.1.0` in a single release commit), so a user reading one
extension's version cannot tell whether that extension changed or merely rode
along with the bundle. The only number that actually ships code, the `packs-v*`
bundle tag, says nothing about which artifacts inside it moved. A user asking
"what sdlc code am I running, and is there a newer one I should take?" has no
single command that answers both halves today.

## Affected systems

- The five official packs (`core`, `sdlc`, `journal`, `guideline-rust`,
  `guideline-python`): their `pack.toml` manifests and the shared
  `packs/CHANGELOG.md`.
- The three official tool extensions (`claude`, `musecode`, `opencode`):
  their `extension.toml` manifests.
- The `packs-v*` git tag stream that `init` and `update` resolve, and the
  existing projects pinning those tags.
- The `init`, `update`, `ext add` and `doctor` flows and the tag-discovery
  rules behind them.
- The `lua` gate in `Makefile.toml` and the release runbook in `RELEASING.md`.
- The book chapters describing packs, the command line, plan and apply,
  authoring, and the 1.x stability promise, and the tutorials and migration
  chapters that show tags.

## Desired outcome

Every pack and extension version answers what code that artifact runs, and a
newer release is discoverable through the project's own commands rather than
through release-notes archaeology. One documented scheme says which number
means what, where each number is pinned, and how each number moves.

## Constraints

- The pack format and the extension API are frozen as `api = 1` with
  additive-only 1.x changes, and the lock format is frozen for all of 1.x.
- Published `packs-v*` tags stay resolvable: existing pins keep working and no
  existing project is broken by the change.
- Binary publishes to crates.io are irreversible, so release mechanics keep a
  human approval at the publish step.
- Code and book chapters describing the same behaviour are fixed in the same
  change.
- This chain lives on the legacy `.claudestacks/sdlc/` board; migrating the
  board itself is not part of this work.

## Non-goals

- The sdlc chain-root rename and its copy-once migration (shipped separately).
- Binary and library crate versioning, which follows crates.io rules, and
  claudevs releases, which follow their own tags.
- Migrating the SDLC board from `.claudestacks/sdlc/` to `.agentstacks/sdlc/`.
- Rewriting git history or deleting published tags.
