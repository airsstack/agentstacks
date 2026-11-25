-- example translator for a made-up tool whose payload is {"hook_event_name": ..., "session_id": ..., "cwd": ...}
local M = {}
local events = { PreToolUse = "tool.before", SessionStart = "session.start" }

function M.event(p)
  if type(p.paths) == "table" then
    local list = {}
    for _, path in ipairs(p.paths) do
      list[#list + 1] = { event = "tool.before", tool = "edit", path = path }
    end
    return list
  end
  local e = events[p.hook_event_name]
  if e == nil then
    return nil
  end
  return { event = e, session_id = p.session_id, cwd = p.cwd }
end

function M.result(r, p)
  if type(p) == "table" and p.paths ~= nil then
    if r == nil then
      return "nil\n"
    end
    return airsstack.json.encode(r) .. "\n"
  end
  if r == nil then return nil end
  local event = type(p) == "table" and p.hook_event_name or nil
  if r.block ~= nil then
    return airsstack.json.encode({ decision = "block", reason = r.block, event = event }) .. "\n"
  end
  if r.context ~= nil then
    return airsstack.json.encode({ context = r.context, event = event }) .. "\n"
  end
  return nil
end

return M
