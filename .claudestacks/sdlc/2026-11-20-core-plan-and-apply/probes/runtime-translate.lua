-- example translator for a made-up tool whose payload is {"hook_event_name": ..., "session_id": ..., "cwd": ...}
local M = {}
local events = { PreToolUse = "tool.before", SessionStart = "session.start" }

function M.event(p)
  local e = events[p.hook_event_name]
  if e == nil then
    return nil
  end
  return { event = e, session_id = p.session_id, cwd = p.cwd }
end

function M.result(r)
  if r == nil then return nil end
  if r.block ~= nil then return airsstack.json.encode({ decision = "block", reason = r.block }) .. "\n" end
  if r.context ~= nil then return airsstack.json.encode({ context = r.context }) .. "\n" end
  return nil
end

return M
