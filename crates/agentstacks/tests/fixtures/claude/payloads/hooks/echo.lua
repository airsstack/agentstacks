-- Writes the neutral event's fields to stderr as `key=value`, `nil` for an absent one, and
-- returns nothing, so the mapping shows for every event whatever it prints on stdout.
local FIELDS = { "event", "source", "prompt", "tool", "path", "last_message", "agent", "session_id", "cwd" }

return function(ev)
  local parts = {}
  for _, key in ipairs(FIELDS) do
    parts[#parts + 1] = key .. "=" .. tostring(ev[key])
  end
  airsstack.stdio.error(table.concat(parts, " ") .. "\n")
  return nil
end
