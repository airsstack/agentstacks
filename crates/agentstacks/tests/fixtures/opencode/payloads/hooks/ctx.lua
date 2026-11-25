-- Returns the event's fields as `key=value`, `nil` for an absent one, as the context, so each
-- test shows what the translator put in the event.
local FIELDS = { "tool", "path", "agent", "agent_id", "prompt" }

return function(ev)
  local parts = {}
  for _, key in ipairs(FIELDS) do
    parts[#parts + 1] = key .. "=" .. tostring(ev[key])
  end
  return { context = table.concat(parts, " ") }
end
