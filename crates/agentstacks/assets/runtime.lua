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
local out = translate.result(result)
if out ~= nil then
  airsstack.stdio.write(out)
end
