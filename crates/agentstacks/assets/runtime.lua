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
