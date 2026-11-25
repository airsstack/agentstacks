---
status: done
created: 2026-11-20
---

# Milestone: OpenCode extension on the same API

## Goal

The official `opencode` extension renders the same packs into `.opencode/` and `opencode.json`,
using only the public extension API that the `claude` extension uses. A project whose manifest
lists both tools gets both set up from one `apply`. Lua hooks reach OpenCode through a thin
generated TypeScript shim that only forwards events to airsl. Every pack item OpenCode can't express
shows up in `plan` as a skip with a reason. Any API change OpenCode forced has been applied to the
`claude` extension too.

## Non-goals

- Freezing the API.
- Pack content written specifically for OpenCode beyond `native/opencode/` files.
- Tools beyond Claude Code and OpenCode.
