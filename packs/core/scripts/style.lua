-- Reply rules — session-start and prompt-submit hook.
--
-- Returns the `## Reply rules` section of the communication protocol as context. args[1] is
-- the protocol path; args[2] is the `style_reinject` option. `session.start` always returns the
-- rules; `prompt.submit` returns them unless the option is off. Returns nothing when the
-- protocol is missing or has no such section.

local base = (...):match("^(.-)[^.]*$")
local style = require(base .. "lib.style")

return function(event)
  local protocol = event.args and event.args[1]
  if type(protocol) ~= "string" or protocol == "" then
    return nil
  end

  if event.event == "prompt.submit" and not style.reinject_enabled(event.args[2]) then
    return nil
  end

  local ok, text = pcall(airsstack.fs.read, protocol)
  if not ok or type(text) ~= "string" then
    return nil
  end
  local rules = style.reply_rules(text)
  if not rules then
    return nil
  end
  return { context = rules }
end
