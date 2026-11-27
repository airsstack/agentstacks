---
status: approved
created: 2026-11-27
source: triage
spec: skipped
---

# Intent: Release the Muse Code extension (packs-v1.1.0, agentstacks 1.1.0)

## Problem

The official Muse Code extension was built and merged in #15 (`06c2e52`), after
`packs-v1.0.0` was tagged (`9ce5239`), so there is no released way to use it with
agentstacks 1.0.0 (GitHub issue #16). The released binary does not know the tool
name, and the released packs tag does not contain the extension. A user following
the book hits a dead end; the only working route is a raw `rev` pin, which
`agentstacks update` deliberately never moves.

Spec skipped: this release makes no new design or behaviour decision. The release
mechanics are fully prescribed by `RELEASING.md`, the version (1.1.0) and the
coupled binary-plus-packs scope are already agreed, and every factual claim
behind them (tag contents, the `OFFICIAL_TOOLS` difference between the tag and
`main`, the working `rev` pin) is already proven in the Evidence below.

## Affected systems

- `crates/agentstacks` and `crates/agentstacks-cli`: version bumps and dated
  changelog entries (the `musecode` entries now under `Unreleased`).
- `packs/CHANGELOG.md`, `README.md`, `CLAUDE.md` and the book pages that say the
  extension "ships with the next `packs-v*` release" (`introduction.md`,
  `ch01-02-first-project.md`, `ch04-00-cli.md`, `ch06-07-musecode.md`).
- `RELEASING.md`: the step-4 verification covers only the claude and opencode
  extensions today; it gains the musecode extension so this gap cannot ship
  unnoticed again.
- Releases, outside this chain's plans (maintainer steps, as in the 1.0 chain):
  `packs-v1.1.0`, `agentstacks-cli-v1.1.0`, crates.io, `agentstacks-v1.1.0`, and
  the closing of issue #16.

## Desired outcome

- `agentstacks init --tool musecode --pack core` and `agentstacks ext add musecode`
  work from the released 1.1.0 binary and resolve the extension from
  `packs-v1.1.0`, ending with a healthy `doctor`.
- `agentstacks ext add https://github.com/airsstack/agentstacks --subdir extensions/musecode --tag packs-v1.1.0`
  works, and tag-pinned projects move to it with `agentstacks update`.
- The README and book describe musecode as released, with no "next release"
  caveat and no dead-end command presented as usable today.
- The closing note on issue #16 tells `rev`-pinned users that `update` will not
  move their pin and how to switch the musecode source to the tag.

## Constraints

- Follow `RELEASING.md` ("Later releases"): the release pull request is green
  first (`cargo make dod`, `cargo make claudevs-check`, `cargo make opencode-check`,
  `cargo make musecode-check`, `cargo deny check`, the `installers` and `book`
  jobs), then `packs-v1.1.0` is pushed alone, then `agentstacks-cli-v1.1.0`
  alone, then verification in a throwaway home, then crates.io.
- Never move or re-push `packs-v1.0.0`: existing projects pin that tag and their
  locks pin its revisions and file hashes.
- Crates.io publishing is irreversible, so it comes only after the GitHub-side
  verification passes, `agentstacks` before `agentstacks-cli`.
- Tagging, publishing and closing the issue are the maintainer's steps outside
  this chain's plans, because they cannot be undone (the 1.0 chain's precedent).
- Work on a branch or worktree, never on `main`. Conventional Commits.

## Non-goals

- No code change: `OFFICIAL_TOOLS` already lists `musecode` on `main`, and the
  adapter, tag resolution and `update` behaviour are unchanged.
- No new tool, pack or feature; the musecode extension itself stays at 1.0.0.
- No `claudevs` release; its crates are untouched.

## Evidence

From GitHub issue #16 (agentstacks 1.0.0, airsl 0.1.5, macOS), verbatim:

- `agentstacks init --tool musecode --pack core` fails:
  `--tool "musecode": not an official tool; pass a path, or add a git source to agentstacks.toml`
- `agentstacks ext add https://github.com/airsstack/agentstacks --subdir extensions/musecode --tag packs-v1.0.0` fails:
  `"extensions/musecode" not found at packs-v1.0.0`
- The workaround
  `agentstacks ext add https://github.com/airsstack/agentstacks --subdir extensions/musecode --rev 06c2e5210b5ceb3fb5f26edec6cd03aa0a96bb64`
  plans and applies correctly (77 files across `.claude/`, `.agents/`, `.muse/`;
  `doctor` healthy). Its downside: a raw `rev` pin does not move with
  `agentstacks update` the way tagged sources do.

Locally verified in this repository: `packs-v1.0.0` (`9ce5239`) contains
`extensions/claude` and `extensions/opencode` but no `extensions/musecode`, and
its `OFFICIAL_TOOLS` is `["claude", "opencode"]`; `main` (`06c2e52`) contains
`extensions/musecode` and `OFFICIAL_TOOLS` is `["claude", "musecode", "opencode"]`.
