# Build an agentstacks tool extension

This page is written for an AI coding agent to follow, step by step. A human can hand it over with
one line:

> Build an agentstacks extension for `<tool>`. Follow
> https://raw.githubusercontent.com/airsstack/agentstacks/main/docs/authoring.md

Everything here is `api = 0`: the extension API is unstable and may change until it is frozen as
`api = 1`. Pin the agentstacks version you build against.

## Read first

Fetch these before writing code. They are the contract; this page only tells you the order of work.

| Document | What it gives you |
|---|---|
| [extensions.md](https://raw.githubusercontent.com/airsstack/agentstacks/main/docs/extensions.md) | the `api = 0` Lua surface: manifest, events, operations, coverage, validation, every refusal |
| [hooks.md](https://raw.githubusercontent.com/airsstack/agentstacks/main/docs/hooks.md) | the hook command line, the shared runtime, the neutral event and result, the translator contract |
| [adapter.lua](https://raw.githubusercontent.com/airsstack/agentstacks/main/extensions/claude/adapter.lua) | the official Claude Code extension, a complete worked example |
| [demo pack](https://raw.githubusercontent.com/airsstack/agentstacks/main/crates/agentstacks-cli/tests/fixtures/project/packs/demo/pack.toml) | a small pack (one skill, one agent, one lua hook, one message hook) to test with |

## What you build

```text
<tool>/
  extension.toml      manifest: name, version, api = 0, entry, reads
  adapter.lua         the only file that calls agentstacks.*; answers `values` and `plan`
  lib/<rule>.lua      pure rules: no agentstacks.*, no require; plain data in, data out
  lib/<rule>_test.lua one test file per rule module, run with `airsl test`
```

## Rules you must not break

- **Propose only.** The extension has no file, network or process access. It calls
  `agentstacks.write`, `copy`, `skip` and `require_binary`; the core validates, shows and applies.
- **Cover every item.** Each skill file, agent, hook, native file and native hook in the `plan`
  payload gets a `write`, a `copy` or a `skip`. A missing one refuses the whole plan.
- **Unsupported is visible.** What the tool cannot express becomes `agentstacks.skip` with a reason
  a user can act on. Never drop an item silently.
- **No grants.** Ask for nothing. A shared config file the user also edits is declared in
  `extension.toml` `reads`, read with `agentstacks.read`, edited with `agentstacks.json.edit` and
  written whole. There is no merge operation.
- **Lua only, under airsl.** If the tool demands another hook language, write a thin shim that only
  runs the hook command and forwards the payload and result (hooks.md section 9).

## Steps

### 1. Learn the tool

Write down, with a source for each, where the tool keeps project skills, agents (subagents),
commands, rules and hook config; the file format of each; its hook events and payload fields; and
how a hook blocks or adds context. List what it cannot express: those become skips.

### 2. Write `extension.toml`

```toml
[extension]
name        = "mytool"
version     = "0.1.0"
api         = 0
description = "Renders packs into MyTool project config"
entry       = "adapter.lua"
# reads     = [".mytool/settings.json"]   # only for a config file shared with the user
```

### 3. Start from a working adapter

This adapter installs skills and agents, and skips every hook. It was run on the demo pack with
airsl 0.1.4: `airsl test` gave `2 passed, 0 failed`, and `agentstacks init` wrote four files and
showed two skips.

```lua
-- lib/paths.lua: pure rules; no agentstacks.* here.
local M = {}

M.ROOT = ".mytool/agentstacks"

function M.skill_dir(name)
  return ".mytool/skills/" .. name
end

function M.agent_file(name)
  return ".mytool/agents/" .. name .. ".md"
end

return M
```

```lua
-- adapter.lua: the only file that calls agentstacks.*; the rules live in lib/.
local paths = agentstacks.require("lib.paths")

local NO_HOOKS = "mytool has no hooks"

agentstacks.on("values", function(q)
  local out = { root = paths.ROOT, skills = {}, agents = {}, vars = {} }
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills or {}) do
      out.skills[p.name .. "/" .. s] = { invoke = "/" .. s, dir = paths.skill_dir(s) }
    end
    for _, a in ipairs(p.agents or {}) do
      out.agents[p.name .. "/" .. a] = a
    end
  end
  return out
end)

agentstacks.on("plan", function(q)
  for _, p in ipairs(q.packs) do
    for _, skill in ipairs(p.skills or {}) do
      for _, f in ipairs(skill.files) do
        agentstacks.copy({ path = skill.dir .. "/" .. f.rel, file = f.id })
      end
    end
    for _, a in ipairs(p.agents or {}) do
      agentstacks.write({ path = paths.agent_file(a.name), content = a.text, covers = { a.id } })
    end
    for _, h in ipairs(p.hooks or {}) do
      agentstacks.skip({ item = h.id, reason = NO_HOOKS })
    end
    local native = p.native or {}
    for _, f in ipairs(native.files or {}) do
      agentstacks.copy({ path = ".mytool/" .. f.rel, file = f.id })
    end
    for _, h in ipairs(native.hooks or {}) do
      agentstacks.skip({ item = h.id, reason = NO_HOOKS })
    end
  end
end)
```

Then replace each placeholder with the tool's real format:

| Item | Start | Make it real |
|---|---|---|
| skill | `copy` keeps the neutral frontmatter | `write` `SKILL.md` with the tool's frontmatter built from `skill.meta`, `covers = { <SKILL.md id> }`; `copy` the other files |
| agent | body only | prepend the tool's frontmatter built from `a.meta`; skip with a reason when a field cannot map |
| vars | none | return a value for each name in `q.vars` (`{{var}}` tokens), for example `$TMPDIR` |
| hooks | skipped | step 5 |

### 4. Put rules in `lib/` and test them

Keep every decision (frontmatter fields, event mapping, quoting) in a `lib/` module that takes
plain values and returns plain values. The adapter passes data in; a module never calls
`agentstacks.*`.

A test file sits beside its module, requires it by bare name and returns a table of test functions:

```lua
-- lib/paths_test.lua
local paths = require("paths")

return {
  skill_dir_is_under_mytool = function()
    assert(paths.skill_dir("greet") == ".mytool/skills/greet")
  end,
}
```

Run `airsl test <tool>`. It must report `0 failed`.

### 5. Hooks

Skip hooks until skills and agents work. Then, per hooks.md:

1. **Command.** Each lua hook arrives with `command`, an argv. Turn it into the tool's command
   string with the tool's own quoting, run from the project directory. Example: claude extension,
   `cd "$CLAUDE_PROJECT_DIR" && "airsl" "run" …`.
2. **Filter** by the hook's `match` and `agents` in the tool's own config. The runtime does not
   filter.
3. **Message hooks** (`kind = "message"`) never reach the runtime: render them natively, for
   example as a command that prints the tool's context output.
4. **`translate.lua`.** When any lua hook is covered by a write, write `<root>/translate.lua`. It
   returns `M.event(payload)` (the tool's payload to the neutral event, or `nil`) and
   `M.result(result, payload)` (the neutral result to the tool's output string, or `nil`).
5. **Blocking.** A hook result `{ block = "<reason>" }` from a hook declared `may_block` must become
   the tool's own deny output. Map `{ context = "<text>" }` to the tool's context output.
6. **Unsupported events** become skips with a reason.

### 6. Run it for real

In a scratch directory holding a copy of your extension and a pack (the demo pack works):

```sh
agentstacks init --tool ./ext/mytool --pack ./packs/demo --yes   # writes agentstacks.toml, applies
agentstacks plan                                                  # 0 to write
agentstacks doctor                                                # healthy
```

Then fire each generated hook command the way the tool would: its JSON payload on stdin, from the
project directory. Check that context is added and a `may_block` hook really blocks. Finally, run
the tool itself on the scratch project and confirm it loads the skills, agents and hooks.

### 7. Before you call it done

- [ ] `airsl test <tool>` reports `0 failed`.
- [ ] `agentstacks plan` on the demo pack has no refusal; every item is written, copied or skipped.
- [ ] Every skip reason tells the user what the tool cannot do.
- [ ] `agentstacks apply --yes`, then `agentstacks plan` shows `0 to write`; `doctor` says `healthy`.
- [ ] A user's own entries in a shared config file survive `apply`, and removing the tool removes
      only your entries.
- [ ] Each hook fires in the real tool, and a `may_block` hook blocks.

## Common refusals

| You see | Fix |
|---|---|
| `mytool/adapter.lua: demo/hook/0 got no operation (copy it, write it, or skip it with a reason)` | cover that item with `write`, `copy` or `skip` |
| `mytool/adapter.lua: lua hooks need .mytool/agentstacks/translate.lua (write it)` | write `<root>/translate.lua` (step 5) |
| `mytool/adapter.lua:12: write: path ".x" is already proposed at adapter.lua:9` | two operations target one path; merge them into one write |
| `mytool/adapter.lua:9: read: ".env" is not in extension.reads` | add the path to `reads` in `extension.toml` |
| `mytool/adapter.lua:1: require: invalid module name "lib/x" (use dots: lib.x)` | `agentstacks.require("lib.x")` |

The full refusal list is in extensions.md sections 9 to 12.
