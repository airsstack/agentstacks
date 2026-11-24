# Probe: cross-pack hooks, rendered root files and item tokens (existing agentstacks binary)

Date: 2026-11-23. Design-stage probe for `2026-11-20-packs-and-claudevs`; no agentstacks code changed.

| Item | Value |
|---|---|
| agentstacks | `agentstacks 0.0.0`, built from this worktree (HEAD `80fd75f`, `feat(repo): CLI commands and the claude extension (#5)`) with `CARGO_TARGET_DIR=$SP/as-target cargo build -q -p agentstacks-cli` |
| airsl | `airsl 0.1.4` at `/Users/hiraq/.local/bin/airsl` |
| claude extension | `ext/claude` = unmodified copy of `extensions/claude` (`diff -r` → identical) |
| project | `$SP/as-probe/` (`git init -q`), `$SP` = the session scratchpad |

Each command was run once. Nothing was refused, so no corrected variant was needed.

## Answers

| Q | Answer |
|---|---|
| Q1 | Yes. `gr`'s `[[hooks]]` with `run = "{{root core}}/hooks/enforce.lua"`, `args = ["--manifest", "{{root}}/enforcement.json"]` and `read = ["{{root}}"]` loads, plans and applies with no refusal. In `.claude/settings.json` it becomes a `PreToolUse` entry, matcher `Read\|Edit\|Write`, module `core.hooks.enforce`. `{{root}}` in `args` and in the grant is the **declaring** pack (`gr`): `.claude/agentstacks/gr/enforcement.json` and `--allow-read .claude/agentstacks/gr`. The core module loads from `.claude/agentstacks/core/hooks/enforce.lua` with no read grant on core (require root = the runtime's directory). |
| Q2 | Yes. `render = ["enforcement.json"]` turns `{"skill": "{{skill gr/guidelines}}"}` into `{"skill": "/guidelines"}`: the claude invocation form `/<name>`, with no pack prefix. The file is written at `.claude/agentstacks/gr/enforcement.json`. |
| Q3 | PreToolUse (Read `/tmp/x.rs`): exit 0, stderr empty, stdout `{"hookSpecificOutput":{"additionalContext":"enforce tool=read path=/tmp/x.rs agent=nil args=--manifest,.claude/agentstacks/gr/enforcement.json","hookEventName":"PreToolUse"}}`. SubagentStop: the matcher is `"chain-reader"` (bare agent name), may_block is `"1"` in argv; exit 0, stderr empty, stdout `{"decision":"block","reason":"probe block last=done"}`. |
| Q4 | `{{agent core/coder}}` → `coder`; `{{skill core/orchestrate}}` → `/orchestrate`. Body: `Spawn coder and then re-run /orchestrate.` The frontmatter is re-emitted with quoted values (`name: "orchestrate"`). |

## Created files (inputs)

`packs/core/pack.toml`:

~~~~text
exclude = ["**/*_test.lua"]

[pack]
name        = "core"
version     = "0.1.0"
api         = 0
description = "Probe core pack"

[requires]
airsl = ">=0.1.4"
~~~~

`packs/core/skills/orchestrate/SKILL.md`:

~~~~text
---
name: orchestrate
description: Probe orchestrate skill
---
Spawn {{agent core/coder}} and then re-run {{skill core/orchestrate}}.
~~~~

`packs/core/agents/coder.md`:

~~~~text
---
name: coder
description: Probe coder agent
---
Coder body.
~~~~

`packs/core/hooks/enforce.lua`:

~~~~text
return function(event) return { context = "enforce tool=" .. tostring(event.tool) .. " path=" .. tostring(event.path) .. " agent=" .. tostring(event.agent) .. " args=" .. table.concat(event.args, ",") } end
~~~~

`packs/core/hooks/handoff.lua`:

~~~~text
return function(event) return { block = "probe block last=" .. tostring(event.last_message) } end
~~~~

`packs/gr/pack.toml`:

~~~~text
render = ["enforcement.json"]

[pack]
name        = "gr"
version     = "0.1.0"
api         = 0
description = "Probe guideline pack"

[requires]
packs = ["core"]

[[hooks]]
kind  = "lua"
event = "tool.before"
match = ["read", "edit", "write"]
run   = "{{root core}}/hooks/enforce.lua"
args  = ["--manifest", "{{root}}/enforcement.json"]
[hooks.grants]
policy = "confined"
read   = ["{{root}}"]
~~~~

`packs/gr/enforcement.json`:

~~~~text
{"skill": "{{skill gr/guidelines}}"}
~~~~

`packs/gr/skills/guidelines/SKILL.md`:

~~~~text
---
name: guidelines
description: Probe guidelines skill
---
Guidelines body.
~~~~

`packs/sd/pack.toml`:

~~~~text
[pack]
name        = "sd"
version     = "0.1.0"
api         = 0
description = "Probe sdlc pack"

[requires]
packs = ["core"]

[[hooks]]
kind      = "lua"
event     = "agent.stop"
agents    = ["chain-reader"]
may_block = true
run       = "{{root core}}/hooks/handoff.lua"
[hooks.grants]
policy = "confined"
~~~~

`packs/sd/agents/chain-reader.md`:

~~~~text
---
name: chain-reader
description: Probe chain-reader agent
---
Chain reader body.
~~~~

`ext/claude/` is a copy of the worktree's `extensions/claude/` and is not repeated here:

~~~~text
ext/claude/adapter.lua
ext/claude/extension.toml
ext/claude/lib/frontmatter_test.lua
ext/claude/lib/frontmatter.lua
ext/claude/lib/hooks_test.lua
ext/claude/lib/hooks.lua
ext/claude/lib/settings_test.lua
ext/claude/lib/settings.lua
ext/claude/lib/translate_test.lua
ext/claude/lib/translate.lua
ext/claude/lib/values_test.lua
ext/claude/lib/values.lua
~~~~

## Commands

### init

~~~~text
$ agentstacks init --tool ./ext/claude --pack ./packs/core --pack ./packs/gr --pack ./packs/sd --yes
~~~~

Exit: `0`

stdout:

~~~~text
agentstacks plan

  create   .claude/agents/chain-reader.md              sd
  create   .claude/agents/coder.md                     core
  create   .claude/agentstacks/core/hooks/enforce.lua  core
  create   .claude/agentstacks/core/hooks/handoff.lua  core
  create   .claude/agentstacks/gr/enforcement.json     gr
  create   .claude/agentstacks/runtime.lua             claude
  create   .claude/agentstacks/translate.lua           claude
  create   .claude/settings.json                       shared
  create   .claude/skills/guidelines/SKILL.md          gr
  create   .claude/skills/orchestrate/SKILL.md         core

  airsl    0.1.4 (new pin)

10 to write, 0 to delete, 0 unchanged, 0 skipped
applied: 10 written, 0 deleted
~~~~

stderr:

~~~~text
~~~~

### plan

~~~~text
$ agentstacks plan
~~~~

Exit: `0`

stdout:

~~~~text
agentstacks plan

  airsl    0.1.4 (pinned)

0 to write, 0 to delete, 10 unchanged, 0 skipped
~~~~

stderr:

~~~~text
~~~~

### apply

~~~~text
$ agentstacks apply --yes
~~~~

Exit: `0`

stdout:

~~~~text
agentstacks plan

  airsl    0.1.4 (pinned)

0 to write, 0 to delete, 10 unchanged, 0 skipped
nothing to change
~~~~

stderr:

~~~~text
~~~~

## Written files

~~~~text
$ find .claude -type f | sort
.claude/agents/chain-reader.md
.claude/agents/coder.md
.claude/agentstacks/core/hooks/enforce.lua
.claude/agentstacks/core/hooks/handoff.lua
.claude/agentstacks/gr/enforcement.json
.claude/agentstacks/runtime.lua
.claude/agentstacks/translate.lua
.claude/settings.json
.claude/skills/guidelines/SKILL.md
.claude/skills/orchestrate/SKILL.md
~~~~

`agentstacks.toml`:

~~~~text
api = 0

[tools.claude]
path = "./ext/claude"

[packs.core]
path = "./packs/core"

[packs.gr]
path = "./packs/gr"

[packs.sd]
path = "./packs/sd"
~~~~

`agentstacks.lock`:

~~~~text
api = 0
# Generated by agentstacks. Do not edit.

[airsl]
version = "0.1.4"

[[tool]]
name    = "claude"
version = "0.1.0"
path    = "./ext/claude"
tree    = "sha256:f38905a338205a6add19d0a735c59fada1233b3b296b998593413dcef0541131"

[[pack]]
name    = "core"
version = "0.1.0"
path    = "./packs/core"
tree    = "sha256:1d369e6083c01e478ad29a79c7727a9dddfa99b84ec870785fadded084783f27"

[[pack]]
name    = "gr"
version = "0.1.0"
path    = "./packs/gr"
tree    = "sha256:70e07e6e479663294f6f1951560ca0a1f9513d715528b1576e3ba662013ec96e"

[[pack]]
name    = "sd"
version = "0.1.0"
path    = "./packs/sd"
tree    = "sha256:286c0d7b1988e5db92745c9f801cc205d3d9db0d1cb736631ca72ec96c5c6817"

[[file]]
path = ".claude/agents/chain-reader.md"
hash = "sha256:b93f9b1e8e277a88377a88d6718e92739d67b16db80bb10b0b7804baf3b91902"
by   = "sd"

[[file]]
path = ".claude/agents/coder.md"
hash = "sha256:cbb99eae15d257d59774c2603ee2f0e3f6a0b2ba181f5f84dd5c3d737b477204"
by   = "core"

[[file]]
path = ".claude/agentstacks/core/hooks/enforce.lua"
hash = "sha256:18fede05c794e1b5d94057ede60f0bd175f6be84b8562444f1fa0dec70a6d567"
by   = "core"

[[file]]
path = ".claude/agentstacks/core/hooks/handoff.lua"
hash = "sha256:e4cd0dbbac91d4cb61c67ee04bc2b931d7fc8394b9da2ecbe16c88371cc85f83"
by   = "core"

[[file]]
path = ".claude/agentstacks/gr/enforcement.json"
hash = "sha256:15fa2ad2fd8f7ae96d7ccd6ddda44caf89e060b34d6e91c2886b24e0a8c5fcea"
by   = "gr"

[[file]]
path = ".claude/agentstacks/runtime.lua"
hash = "sha256:05ba59c1af4002b5a212f9eacd1cb74253f69cc79c9bf3a95d4dda312f89a282"
by   = "claude"

[[file]]
path = ".claude/agentstacks/translate.lua"
hash = "sha256:285d8cba0ba7f301fb4163ebbb100dce57318381cd132772ea03dfec909b5cad"
by   = "claude"

[[file]]
path = ".claude/skills/guidelines/SKILL.md"
hash = "sha256:b66d04905f38ac570bdb2e00fe04af21cdc036e1d3ab0cd207bef25016db2424"
by   = "gr"

[[file]]
path = ".claude/skills/orchestrate/SKILL.md"
hash = "sha256:4e2fe423a46577b78aaa2f7427f1491dee9102a660bc392a7ec04c9cf1ee1762"
by   = "core"
~~~~

`.claude/settings.json`:

~~~~text
{
  "hooks": {
    "PreToolUse": [
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \".claude/agentstacks/gr\" \".claude/agentstacks/runtime.lua\" \"core.hooks.enforce\" \"0\" \"neutral\" \"--manifest\" \".claude/agentstacks/gr/enforcement.json\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "Read|Edit|Write"
      }
    ],
    "SubagentStop": [
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \".claude/agentstacks/runtime.lua\" \"core.hooks.handoff\" \"1\" \"neutral\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "chain-reader"
      }
    ]
  }
}
~~~~

`.claude/agentstacks/gr/enforcement.json`:

~~~~text
{"skill": "/guidelines"}
~~~~

`.claude/skills/orchestrate/SKILL.md`:

~~~~text
---
name: "orchestrate"
description: "Probe orchestrate skill"
---
Spawn coder and then re-run /orchestrate.
~~~~

`.claude/skills/guidelines/SKILL.md`:

~~~~text
---
name: "guidelines"
description: "Probe guidelines skill"
---
Guidelines body.
~~~~

`.claude/agents/coder.md`:

~~~~text
---
name: "coder"
description: "Probe coder agent"
---
Coder body.
~~~~

`.claude/agents/chain-reader.md`:

~~~~text
---
name: "chain-reader"
description: "Probe chain-reader agent"
---
Chain reader body.
~~~~

`.claude/agentstacks/core/hooks/enforce.lua`:

~~~~text
return function(event) return { context = "enforce tool=" .. tostring(event.tool) .. " path=" .. tostring(event.path) .. " agent=" .. tostring(event.agent) .. " args=" .. table.concat(event.args, ",") } end
~~~~

`.claude/agentstacks/core/hooks/handoff.lua`:

~~~~text
return function(event) return { block = "probe block last=" .. tostring(event.last_message) } end
~~~~

`.claude/agentstacks/runtime.lua`:

~~~~text
-- agentstacks hook runtime, api 0.
-- usage: airsl run ... runtime.lua <hook-module> <may_block: 0|1> <kind: neutral|native> [args...]
local translate = require("translate")
local name, may_block, kind = arg[1], arg[2] == "1", arg[3]
local args = {}
for i = 4, #arg do
  args[#args + 1] = arg[i]
end
local payload = airsstack.hook.payload()
local event
if kind == "native" then
  event = { payload = payload, args = args }
else
  event = translate.event(payload)
  if event == nil then
    return
  end
  event.args = args
end
local result = require(name)(event)
if type(result) == "table" and result.block ~= nil and not may_block then
  airsstack.stdio.error("agentstacks: hook " .. name .. " tried to block but is not declared may_block; reason: "
    .. tostring(result.block) .. "\n")
  result.block = nil
end
local out = translate.result(result, payload)
if out ~= nil then
  airsstack.stdio.write(out)
end
~~~~

`.claude/agentstacks/translate.lua`:

~~~~text
-- agentstacks translator for claude, api 0.
local M = {}

local EVENTS = {
  SessionStart = "session.start",
  SessionEnd = "session.end",
  UserPromptSubmit = "prompt.submit",
  PreToolUse = "tool.before",
  PostToolUse = "tool.after",
  SubagentStop = "agent.stop",
}
local TOOLS = { Read = "read", Edit = "edit", Write = "write" }
local SOURCES = { startup = true, resume = true, clear = true, compact = true }

local function str(v)
  if type(v) == "string" then
    return v
  end
  return nil
end

function M.event(p)
  local name = EVENTS[p.hook_event_name]
  if name == nil then
    return nil
  end
  local e = { event = name, session_id = p.session_id, cwd = p.cwd }
  if type(p.agent_type) == "string" and p.agent_type ~= "" then
    e.agent = p.agent_type
  end
  if name == "session.start" then
    if SOURCES[p.source] then
      e.source = p.source
    end
  elseif name == "prompt.submit" then
    e.prompt = str(p.prompt)
  elseif name == "tool.before" or name == "tool.after" then
    local tool = TOOLS[p.tool_name]
    if tool == nil then
      return nil
    end
    e.tool = tool
    if type(p.tool_input) == "table" then
      e.path = str(p.tool_input.file_path)
    end
  elseif name == "agent.stop" then
    e.last_message = str(p.last_assistant_message)
  end
  return e
end

local function encode(out)
  return airsstack.json.encode(out) .. "\n"
end

function M.result(r, p)
  if r == nil then
    return nil
  end
  local event = p.hook_event_name
  local ctx, blk = r.context, r.block
  if ctx == nil and blk == nil then
    return nil
  end
  if event == "PreToolUse" then
    local o = { hookEventName = event }
    if blk ~= nil then
      o.permissionDecision = "deny"
      o.permissionDecisionReason = blk
    end
    if ctx ~= nil then
      o.additionalContext = ctx
    end
    return encode({ hookSpecificOutput = o })
  elseif event == "UserPromptSubmit" or event == "PostToolUse" then
    local out = {}
    if blk ~= nil then
      out.decision = "block"
      out.reason = blk
    end
    if ctx ~= nil then
      out.hookSpecificOutput = { hookEventName = event, additionalContext = ctx }
    end
    return encode(out)
  elseif event == "SubagentStop" then
    if blk == nil then
      return nil
    end
    return encode({ decision = "block", reason = blk })
  elseif event == "SessionStart" then
    local parts = {}
    if ctx ~= nil then
      parts[#parts + 1] = ctx
    end
    if blk ~= nil then
      parts[#parts + 1] = blk
    end
    return encode({ hookSpecificOutput = { hookEventName = event, additionalContext = table.concat(parts, "\n") } })
  end
  return nil
end

return M
~~~~

## Hook runs

Command strings taken from `.claude/settings.json` with `jq -r`, run as
`CLAUDE_PROJECT_DIR=$SP/as-probe sh -c "<command>" < <stdin file>`.

### PreToolUse

Command:

~~~~text
cd "$CLAUDE_PROJECT_DIR" && "airsl" "run" "--fail-open" "--policy" "confined" "--allow-read" ".claude/agentstacks/gr" ".claude/agentstacks/runtime.lua" "core.hooks.enforce" "0" "neutral" "--manifest" ".claude/agentstacks/gr/enforcement.json" # agentstacks
~~~~

stdin (no trailing newline):

~~~~text
{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"/tmp/x.rs"},"session_id":"s1","cwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/as-probe"}
~~~~


Exit: `0`

stdout:

~~~~text
{"hookSpecificOutput":{"additionalContext":"enforce tool=read path=/tmp/x.rs agent=nil args=--manifest,.claude/agentstacks/gr/enforcement.json","hookEventName":"PreToolUse"}}
~~~~

stderr:

~~~~text
~~~~

### SubagentStop

Command:

~~~~text
cd "$CLAUDE_PROJECT_DIR" && "airsl" "run" "--fail-open" "--policy" "confined" ".claude/agentstacks/runtime.lua" "core.hooks.handoff" "1" "neutral" # agentstacks
~~~~

stdin (no trailing newline):

~~~~text
{"hook_event_name":"SubagentStop","agent_type":"chain-reader","last_assistant_message":"done","session_id":"s1","cwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/as-probe"}
~~~~


Exit: `0`

stdout:

~~~~text
{"decision":"block","reason":"probe block last=done"}
~~~~

stderr:

~~~~text
~~~~

