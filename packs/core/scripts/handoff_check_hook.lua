-- Handoff report conformance hook.
--
-- Rules live in `lib/handoff_report.lua`. Prose mirror:
-- `skills/context-handoff/references/protocol.md`.
--
-- Three event shapes arrive here:
--
--   `tool.after` (write)       the file is already written; the block reason goes back beside the
--                              tool result so the agent can rewrite it.
--   `agent.stop`               the agent's final message names the report.
--   native                     a `SubagentHandback` call about to deliver the report
--                              (`event.payload` is a table).
--
-- Returns `{ block = reason }` for a report that breaks the protocol, `nil` otherwise.

-- The requires start from this module's own name so the module loads under any prefix the
-- caller's runtime gives it.
local base = (...):match("^(.-)[^.]*$")

local report = require(base .. "lib.handoff_report")

return function(event)
  -- `report.report_path` is the one place that decides which file, if any, this event
  -- points at: it reads the right field for the event, applies the handoff-root test
  -- (`report.under_handoff_root`) to reject a trailing citation, a documentation URL, or an
  -- ordinary project file that merely sits in the report text, and resolves a relative path
  -- against the event's `cwd`. No qualifying path is not a violation — an agent legitimately
  -- without one (the standalone case the protocol's error handling allows) must never be held,
  -- and an agent that owed a path and omitted it has already broken the return contract where
  -- the orchestrator can see it.
  local file = report.report_path(event)
  if not file then
    return nil
  end

  local found = {}
  for _, violation in ipairs(report.check(file)) do
    found[#found + 1] = violation.id .. ": " .. violation.line
  end
  if #found == 0 then
    return nil
  end
  return { block = "handoff report does not conform to the protocol:\n" .. table.concat(found, "\n") }
end
