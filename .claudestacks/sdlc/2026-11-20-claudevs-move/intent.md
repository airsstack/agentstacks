---
status: draft
created: 2026-11-20
milestone: foundation/01
---

# Intent: Move claudevs

## Problem

`claudevs` and `claudevs-cli`, the deterministic Claude Code setup test engine, still live in
claudestacks (`../claudestacks/crates/claudevs{,-cli}`). The milestone requires them to move here
under the same crate names. Today they check a Claude Code plugin directory (the
`claudevs-check` task runs over a fixture plugin corpus). After the move, what they need to check is
the native `.claude/` config that agentstacks renders from packs.

## Affected systems

- New `crates/claudevs`, `crates/claudevs-cli`. Their dependencies are `airsl`, regex, serde,
  serde_json, serde-saphyr, thiserror, toml, walkdir and tempfile, plus clap for the CLI.
- `Makefile.toml` (a `claudevs-check` equivalent), CI (a corpus job), `deny.toml` (new licences).
- Fixture corpus.

## Desired outcome

- `claudevs` and `claudevs-cli` build, test and pass `cargo make dod` in this workspace under the
  same crate names, with history traceable to claudestacks.
- claudevs can check agentstacks-rendered `.claude/` project config as well as plugin directories.
- CI runs the claudevs corpus check.

## Constraints

- Keep the crate names (crates.io continuity); `airsl` from crates.io only.
- claudestacks keeps working until `release-and-migration` archives it.
- Can start independently of the other chains; checking rendered config needs `claude-extension`.

## Non-goals

- Moving `airsclaude` (airsdk's work, roadmap non-goal).
- New claudevs features beyond what checking rendered config needs.
