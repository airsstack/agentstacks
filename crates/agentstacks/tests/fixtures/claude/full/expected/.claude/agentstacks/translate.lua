-- agentstacks translator for claude, api 0.
local M = {}

local EVENTS = {
  SessionStart = "session.start",
  SessionEnd = "session.end",
  UserPromptSubmit = "prompt.submit",
  PreToolUse = "tool.before",
  PostToolUse = "tool.after",
  SubagentStop = "agent.stop",
}
local TOOLS = { Read = "read", Edit = "edit", Write = "write" }
local SOURCES = { startup = true, resume = true, clear = true, compact = true }

local function str(v)
  if type(v) == "string" then
    return v
  end
  return nil
end

function M.event(p)
  local name = EVENTS[p.hook_event_name]
  if name == nil then
    return nil
  end
  local e = { event = name, session_id = p.session_id, cwd = p.cwd }
  if type(p.agent_type) == "string" and p.agent_type ~= "" then
    e.agent = p.agent_type
  end
  if type(p.agent_id) == "string" and p.agent_id ~= "" then
    e.agent_id = p.agent_id
  end
  if name == "session.start" then
    if SOURCES[p.source] then
      e.source = p.source
    end
  elseif name == "prompt.submit" then
    e.prompt = str(p.prompt)
  elseif name == "tool.before" or name == "tool.after" then
    local tool = TOOLS[p.tool_name]
    if tool == nil then
      return nil
    end
    e.tool = tool
    if type(p.tool_input) == "table" then
      e.path = str(p.tool_input.file_path)
    end
  elseif name == "agent.stop" then
    e.last_message = str(p.last_assistant_message)
  end
  return e
end

local function encode(out)
  return airsstack.json.encode(out) .. "\n"
end

function M.result(r, p)
  if r == nil then
    return nil
  end
  local event = p.hook_event_name
  local ctx, blk = r.context, r.block
  if ctx == nil and blk == nil then
    return nil
  end
  if event == "PreToolUse" then
    local o = { hookEventName = event }
    if blk ~= nil then
      o.permissionDecision = "deny"
      o.permissionDecisionReason = blk
    end
    if ctx ~= nil then
      o.additionalContext = ctx
    end
    return encode({ hookSpecificOutput = o })
  elseif event == "UserPromptSubmit" or event == "PostToolUse" then
    local out = {}
    if blk ~= nil then
      out.decision = "block"
      out.reason = blk
    end
    if ctx ~= nil then
      out.hookSpecificOutput = { hookEventName = event, additionalContext = ctx }
    end
    return encode(out)
  elseif event == "SubagentStop" then
    if blk == nil then
      return nil
    end
    return encode({ decision = "block", reason = blk })
  elseif event == "SessionStart" then
    local parts = {}
    if ctx ~= nil then
      parts[#parts + 1] = ctx
    end
    if blk ~= nil then
      parts[#parts + 1] = blk
    end
    return encode({ hookSpecificOutput = { hookEventName = event, additionalContext = table.concat(parts, "\n") } })
  end
  return nil
end

return M
