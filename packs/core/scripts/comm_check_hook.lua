-- Communication protocol report conformance hook.
--
-- Rules live in `lib/comm_report.lua`. Prose mirror: `skills/discuss/references/protocol.md`.
-- Receives the same three event shapes as `handoff_check_hook.lua`, beside it; acts only on a
-- report carrying `protocol:`, so every other agent's report passes through untouched.
--
-- Returns `{ block = reason }` for a report that breaks the protocol, `nil` otherwise.

-- The requires start from this module's own name so the module loads under any prefix the
-- caller's runtime gives it.
local base = (...):match("^(.-)[^.]*$")

local report = require(base .. "lib.handoff_report")
local comm = require(base .. "lib.comm_report")

return function(event)
  local file = report.file_from_payload(event)
  if not file then
    return nil
  end

  local reason = comm.hook_output(event, comm.check(file))
  if not reason then
    return nil
  end
  return { block = reason }
end
