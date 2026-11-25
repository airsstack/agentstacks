# Probe results: the list-aware hook runtime (airsl 0.1.5)

The proposed `runtime.lua` (spec §3.2), a stub translator whose `M.event` returns one event for `{"path":…}` and a list for `{"list":[…]}`, and a stub hook. Run from `<scratchpad>/rt-01`.

```r/runtime.lua
-- agentstacks hook runtime, api 0.
-- usage: airsl run ... runtime.lua <hook-module> <may_block: 0|1> <kind: neutral|native> [args...]
local translate = require("translate")
local name, may_block, kind = arg[1], arg[2] == "1", arg[3]
local args = {}
for i = 4, #arg do
  args[#args + 1] = arg[i]
end
local payload = airsstack.hook.payload()
local hook

-- Runs the hook on one event; a block from a hook that is not may_block is dropped with a line.
local function run(event)
  hook = hook or require(name)
  event.args = args
  local result = hook(event)
  if type(result) == "table" and result.block ~= nil and not may_block then
    airsstack.stdio.error("agentstacks: hook " .. name .. " tried to block but is not declared may_block; reason: "
      .. tostring(result.block) .. "\n")
    result.block = nil
  end
  return result
end

local result
if kind == "native" then
  result = run({ payload = payload })
else
  local event = translate.event(payload)
  if event == nil then
    return
  end
  if event.event ~= nil then
    result = run(event)
  else
    -- A list of events: the hook runs once per event, in order, until one blocks.
    local contexts = {}
    for _, each in ipairs(event) do
      local one = run(each)
      if type(one) == "table" then
        if one.context ~= nil then
          contexts[#contexts + 1] = tostring(one.context)
        end
        if one.block ~= nil then
          result = { block = one.block }
          break
        end
      end
    end
    if #contexts > 0 then
      result = result or {}
      result.context = table.concat(contexts, "\n")
    end
  end
end
local out = translate.result(result, payload)
if out ~= nil then
  airsstack.stdio.write(out)
end
```
```r/translate.lua
local M = {}
function M.event(p)
  if p.list then
    local out = {}
    for _, path in ipairs(p.list) do
      out[#out + 1] = { event = "tool.before", tool = "edit", path = path }
    end
    return out
  end
  return { event = "tool.before", tool = "edit", path = p.path }
end
function M.result(r, p)
  if r == nil then
    return "nil\n"
  end
  return airsstack.json.encode(r) .. "\n"
end
return M
```
```r/h.lua
return function(e)
  if e.path == "stop" then
    return { block = "blocked " .. e.path, context = "ctx " .. e.path }
  end
  if e.path == "quiet" then
    return nil
  end
  return { context = "ctx " .. e.path .. " args=" .. table.concat(e.args, ",") }
end
```

**Command and output**

```text
$ airsl --version; airsl check r
airsl 0.1.5
3 file(s) compiled, 0 failed
$ printf '%s' '{"path":"a"}' | airsl run --fail-open --policy confined --allow-read r r/runtime.lua h 0 neutral X Y
{"context":"ctx a args=X,Y"}
exit=0
$ printf '%s' '{"list":["a","b"]}' | airsl run --fail-open --policy confined --allow-read r r/runtime.lua h 0 neutral X Y
{"context":"ctx a args=X,Y\nctx b args=X,Y"}
exit=0
$ printf '%s' '{"list":["a","stop","c"]}' | airsl run --fail-open --policy confined --allow-read r r/runtime.lua h 1 neutral X Y
{"block":"blocked stop","context":"ctx a args=X,Y\nctx stop"}
exit=0
$ printf '%s' '{"list":["a","stop","c"]}' | airsl run --fail-open --policy confined --allow-read r r/runtime.lua h 0 neutral X Y
agentstacks: hook h tried to block but is not declared may_block; reason: blocked stop
{"context":"ctx a args=X,Y\nctx stop\nctx c args=X,Y"}
exit=0
$ printf '%s' '{"list":["quiet","quiet"]}' | airsl run --fail-open --policy confined --allow-read r r/runtime.lua h 0 neutral X Y
nil
exit=0
$ printf '%s' '{"list":[]}' | airsl run --fail-open --policy confined --allow-read r r/runtime.lua h 0 neutral X Y
nil
exit=0
```

**Answer**

- One event: unchanged shape, `{"context":"ctx a args=X,Y"}`.
- A list: the hook runs per event, contexts joined with `
`, args added to each event.
- `may_block` 1: the first block stops the loop (`c` never runs) and keeps the contexts gathered so far, the blocking one included.
- `may_block` 0: the block line goes to stderr, the loop continues, every context is kept.
- Hooks returning `nil`, and an empty list, give a `nil` result to `M.result`.
