-- The reply rules of the core pack's communication protocol, and the option that controls
-- re-sending them each turn.
--
-- Split from the hook module so both decisions can be tested against strings.

local M = {}

M.HEADING = "## Reply rules"

-- The `## Reply rules` section of `text`, heading included, trimmed — or nil when the
-- section is absent or empty. Ends at the next `#` or `##` heading; `###` and deeper stay in.
function M.reply_rules(text)
  local kept, inside = {}, false
  for line in (text .. "\n"):gmatch("(.-)\r?\n") do
    if line == M.HEADING then
      inside = true
    elseif inside and (line:match("^#%s") or line:match("^##%s")) then
      break
    elseif inside then
      kept[#kept + 1] = line
    end
  end
  if not inside then
    return nil
  end
  local body = table.concat(kept, "\n"):match("^%s*(.-)%s*$")
  if body == "" then
    return nil
  end
  return M.HEADING .. "\n\n" .. body
end

-- Whether to re-send the rules on every prompt, from the raw `style_reinject` option (args[2])
-- as the hook receives it. Only an explicit `false` or `0` turns it off; an absent or
-- unrecognised value keeps the default, on.
function M.reinject_enabled(value)
  if value == nil then
    return true
  end
  local normalized = tostring(value):lower():match("^%s*(.-)%s*$")
  return not (normalized == "false" or normalized == "0")
end

return M
