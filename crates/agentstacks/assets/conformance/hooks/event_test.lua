-- Tests for event, the hook every conformance event runs.
--
--   cargo make lua

local hook = require("event")

return {
  event_returns_context = function()
    local result = hook({ event = "session.start" })
    assert(type(result) == "table", "the hook returns a table")
    assert(
      result.context == "conformance session.start",
      "the context names the event, got " .. tostring(result.context)
    )
  end,
}
