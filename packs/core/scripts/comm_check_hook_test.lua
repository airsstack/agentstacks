-- Tests for comm_check_hook — the hook module's three event shapes.
--
--   cargo make lua

local hook = require("comm_check_hook")
local fs = airsstack.fs
local path = airsstack.path

local HEAD = "---\nagent: reviewer\ntask: review x\nprotocol: discuss/1\n---\n<summary>\nOne finding.\n</summary>\n"
local BAD = HEAD .. "<detail>\nbody\n</detail>\n"
local GOOD = HEAD .. "<topics>\n1. the finding\n</topics>\n<detail>\n## 1. the finding\nbody\n</detail>\n"

local REASON = "communication protocol report does not conform:\n"
  .. "topics-missing: <detail> is present but there is no <topics> block"

-- Writes `body` to `<tempdir>/handoff/01-reviewer-x.md`, a path the root test accepts, and
-- returns the tempdir and the file.
local function report(body)
  local dir = fs.tempdir()
  local folder = path.join(dir, "handoff")
  fs.mkdir(folder)
  local file = path.join(folder, "01-reviewer-x.md")
  fs.write(file, body)
  return dir, file
end

return {
  tool_after_bad_report_blocks = function()
    local dir, file = report(BAD)
    local result = hook({ event = "tool.after", tool = "write", path = file, cwd = dir, args = {} })
    assert(result and result.block == REASON, tostring(result and result.block))
  end,

  agent_stop_bad_report_blocks = function()
    local dir, file = report(BAD)
    local result = hook({
      event = "agent.stop",
      last_message = "Done. Report at " .. file,
      cwd = dir,
      args = {},
    })
    assert(result and result.block == REASON, tostring(result and result.block))
  end,

  native_handback_bad_report_blocks = function()
    local dir, file = report(BAD)
    local result = hook({
      payload = { tool_input = { message = "Report at " .. file }, cwd = dir },
      args = {},
    })
    assert(result and result.block == REASON, tostring(result and result.block))
  end,

  good_report_returns_nil = function()
    local dir, file = report(GOOD)
    assert(hook({ event = "tool.after", tool = "write", path = file, cwd = dir, args = {} }) == nil)
    assert(hook({
      event = "agent.stop",
      last_message = "Report at " .. file,
      cwd = dir,
      args = {},
    }) == nil)
    assert(hook({
      payload = { tool_input = { message = "Report at " .. file }, cwd = dir },
      args = {},
    }) == nil)
  end,

  a_report_without_a_protocol_key_returns_nil = function()
    local dir, file = report("---\nagent: coder\ntask: x\n---\n<summary>\nok\n</summary>\n"
      .. "<detail>\nbody\n</detail>\n")
    assert(hook({ event = "tool.after", tool = "write", path = file, cwd = dir, args = {} }) == nil)
  end,
}
