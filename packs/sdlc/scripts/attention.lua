-- Session-start hook module for claudestacks-sdlc: returns the board rows that need attention,
-- which Claude Code adds to the session's context. `event.args[1]` names the command that shows
-- the full board.
--
-- The chain root is taken from the event's `cwd` — the session's directory, which in a
-- worktree is the worktree root — and from the process directory when the event carries none.

-- The requires start from this module's own name so the module loads under any prefix the
-- caller's runtime gives it.
local base = (...):match("^(.-)[^.]*$")

local status = require(base .. "lib.status")
local path = airsstack.path

return function(event)
  local cwd = type(event.cwd) == "string" and event.cwd ~= "" and event.cwd or path.absolute(".")
  local text = status.attention(path.join(cwd, status.ROOT), event.args[1])
  if text == "" then
    return nil
  end
  return { context = text }
end
