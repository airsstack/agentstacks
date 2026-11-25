---
status: approved
created: 2026-11-25
milestone: foundation/03
---

# Intent: Freeze api = 1 and release agentstacks 1.0

## Problem

`foundation/01` and `foundation/02` are done: Claude Code and OpenCode both run on the public
extension API, which is still `api = 0` (unstable). That was the condition for freezing it.
`foundation/03` is the last milestone and has no chain yet. This one intent covers all of it
(decided 2026-11-25: one intent for the whole milestone).

Gaps between today (agentstacks 0.2.0, packs-v0.2.0) and the milestone goal:

- **No frozen API.** `API` is `0` in one place (`crates/agentstacks/src/types/api.rs:26`). Four
  formats read it (`pack.toml`, `extension.toml`, `agentstacks.toml`, `agentstacks.lock`), and
  `api = 0` is written in both official extensions, the five official packs and about 235 test
  fixtures. Any `api` other than `0` is refused, so a project or pack written for 0.x stops
  working the moment the number changes, and there is no stated way to move it.
- **No stability promise.** `crates/agentstacks/README.md:13` says the API is unstable during 0.x.
  Nothing says what 1.x will keep working: the extension API, the pack format, the project and
  lock files, or the CLI surface.
- **The API has not been reviewed before the freeze.** Anything wrong in the `api = 0` surface
  (the `agentstacks` Lua table, events, operations, validation, `extension.toml`, the pack layout)
  is permanent once it becomes `api = 1`.
- **No third-party extension tooling.** There is no `agentstacks ext add`: a non-official
  extension or pack from git can be added only by editing `agentstacks.toml` by hand
  (`book/src/ch04-00-cli.md` §3). There is no `agentstacks ext test`, so an author cannot check an
  extension before using it in a project; today only `airsl test` and the repo's own Rust tests
  exist.
- **No pack-authoring docs.** The book explains how to write a tool extension
  (`ch06-00-authoring.md`) but not a pack. A third party cannot write a pack from the book alone.
- **No 1.0 release path.** `RELEASING.md` is the runbook for the first release, with 0.1.0 tag
  names. agentstacks is at 0.2.0, and the five CHANGELOGs and `packs-v*` tags have no 1.0 entry.

## Affected systems

- `crates/agentstacks`: `types/api.rs` and the readers of every format's `api` key, along with
  their refusals; any API shape fixed by the review in `types/`, `host/` and `app/`; the test
  fixtures.
- `crates/agentstacks-cli`: the new `ext add` and `ext test` commands, help, exit codes and tests.
- `extensions/claude/`, `extensions/opencode/`: `extension.toml` `api`, plus any adapter change
  the review forces.
- `packs/*/pack.toml` (five packs) and `packs/CHANGELOG.md`.
- Book: `ch04-00-cli.md` (ext commands), `ch05-00-plan-and-apply.md`, `ch06-00/01/02/03` (api = 1,
  ext add/test), a new pack-authoring chapter, a stability-promise page, migration from 0.x,
  `SUMMARY.md`, getting started.
- `crates/*/README.md`, the workspace `README.md`, `CLAUDE.md` (status, "api = 0" invariant text),
  `RELEASING.md`, the CHANGELOGs, `dist-workspace.toml` and CI if the release needs it.
- Org docs: `.github/profile/README.md` and the org-root workspace `README.md` (the status changes
  to 1.0).

## Desired outcome

- `agentstacks 1.0` and `packs-v1.0.0` are released through the existing dist and crates.io path.
- The extension API and the pack format are frozen as `api = 1`, after a review that fixed what
  should not be frozen as it is. A written 1.x stability promise states what keeps working across
  every 1.x release and what does not.
- Both official extensions and all five official packs run on `api = 1`, using only the public API.
- A project, pack or extension written for `api = 0` gets a clear, actionable message about moving
  to `api = 1`. It is never silently misread.
- A third party can add an extension (and its packs) from a git or path source with
  `agentstacks ext add`, and check an extension with `agentstacks ext test`, before using it in a
  project.
- A third party can write a pack or a tool extension from the book alone. The book covers
  `api = 1`, `ext add` and `ext test`.
- `cargo make dod`, `claudevs-check`, `opencode-check`, `cargo deny` and the book build are green.

## Constraints

- Every invariant in `CLAUDE.md` holds: extensions only propose, nothing is ambient, unsupported
  is visible, Lua only under airsl, official extensions use the public API, and the core stays
  tool-agnostic.
- `ext add` and `ext test` run no third-party code outside airsl, and touch no file outside what
  `plan` shows (`ext add` edits `agentstacks.toml` only).
- `airsl` comes from crates.io. No `aisdk-*` crate names.
- The architecture contract (`book/src/ch07-00-architecture.md`) and the module-layout rules apply
  to every new module.
- Simple design: refuse only what breaks an invariant.
- Every behaviour claim is proven with the real stack (agentstacks, airsl, Claude Code, OpenCode at
  pinned versions).
- Conventional Commits on a branch, never `main`. Release steps that cannot be undone (crates.io
  publish, tags) are taken by the maintainer through `RELEASING.md`.

## Non-goals

- A marketplace, registry or discovery service for community packs and extensions.
- Official support for tools beyond Claude Code and OpenCode.
- Stability guarantees for the Rust crates' library API beyond what the CLI and the extension API
  need.
- claudevs and claudevs-cli releases or API changes (they release on their own).
- An automated `0 → 1` migration tool, beyond what the clear message and the book need.
