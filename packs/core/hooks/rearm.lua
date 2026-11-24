-- Enforcement re-arm — session-start hook for compaction.
--
-- Compaction drops the injected context out of the window, but the session id survives it. Without
-- this the enforce hook's one-shot-per-context sentinel would keep the rule suppressed for the
-- rest of the session. Unlinking this session's sentinels lets the pointer re-enter context on
-- the next Read, Edit or Write.
--
-- Returns nothing.

-- The requires start from this module's own name so the module loads under any prefix the
-- caller's runtime gives it.
local base = (...):match("^(.-)[^.]*$")

local enforce = require(base .. "lib.enforce")

return function(event)
  local session_id = type(event.session_id) == "string" and event.session_id or ""
  enforce.clear_session(enforce.sentinel_dir(airsstack.env), session_id)
  return nil
end
