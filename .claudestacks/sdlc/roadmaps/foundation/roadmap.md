---
status: done
created: 2026-11-20
---

# Roadmap: agentstacks foundation

## Vision

One CLI sets up any agentic coding tool from one tool-neutral copy of its content. Packs say what
the agent gets; Lua tool extensions, sandboxed by airsl with zero authority, say how each tool gets
it. Every change is a plan the human sees before apply writes it, and `agentstacks.lock` pins
versions and file hashes so drift is detected, never silently overwritten.

The claudestacks plugins stop being Claude-only: they become packs that render into Claude Code and
OpenCode alike. The official extensions are built on the same public API as community ones, so
freezing `api = 1` only after both tools run on it proves the API is good enough for real tools
before anyone else depends on it.

## Non-goals

- A plugin marketplace or hosted pack registry. Setup is CLI-only, into each tool's native project
  config.
- Official tools beyond Claude Code and OpenCode. Others come as community extensions, or later when
  real work needs them.
- A scripting language other than Lua. Where a tool demands one, only a thin generated shim that
  forwards events to airsl.
- The `airsstack` front-door CLI hand-off (`airsstack agent …`); that belongs to the airsstack repo.
- Moving airsclaude and openrouter-rs; that is airsdk's work.
