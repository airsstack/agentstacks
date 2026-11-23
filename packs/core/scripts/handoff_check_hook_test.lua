-- Tests for handoff_check_hook — the hook module's three event shapes.
--
--   cargo make lua

local hook = require("handoff_check_hook")
local fs = airsstack.fs
local path = airsstack.path

local BAD = "<summary>\nfine\n</summary>\n"
local GOOD = "---\nagent: coder\ntask: do the thing\n---\n<summary>\nDone.\n</summary>\n"

local REASON = "handoff report does not conform to the protocol:\n"
  .. "frontmatter-missing: the file does not open with a `---` block"

-- Writes `body` to `<tempdir>/handoff/01-coder-x.md`, a path the root test accepts, and returns
-- the tempdir and the file.
local function report(body)
  local dir = fs.tempdir()
  local folder = path.join(dir, "handoff")
  fs.mkdir(folder)
  local file = path.join(folder, "01-coder-x.md")
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

  an_event_with_no_report_path_returns_nil = function()
    assert(hook({ event = "agent.stop", last_message = "no report, receipt inline", args = {} })
      == nil)
  end,
}
