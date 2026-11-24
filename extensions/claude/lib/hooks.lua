-- Renders one payload hook as a Claude Code hook group.
--
-- Pure: it takes the hook, the rendered var forms and the rendered agent names and returns the
-- Claude event with its group, or nil and the reason the hook cannot be expressed. It calls no
-- `agentstacks.*` and no `require`.

local M = {}

-- Ends every command this module writes, so `settings` can tell a group it owns from the user's.
M.MARKER = " # agentstacks"

local SESSION_ID_REASON = "claude gives hooks the session id in the event (event.session_id), "
  .. "not as {{var session_id}}"
local SESSION_END_REASON = "claude shows no hook output to the model at session end"
local AGENT_STOP_REASON = "claude treats hook output at agent stop as feedback that keeps the agent running"

local NEUTRAL_EVENTS = {
  ["session.start"] = "SessionStart",
  ["session.end"] = "SessionEnd",
  ["prompt.submit"] = "UserPromptSubmit",
  ["tool.before"] = "PreToolUse",
  ["tool.after"] = "PostToolUse",
  ["agent.stop"] = "SubagentStop",
}

local TOOLS = { read = "Read", edit = "Edit", write = "Write" }
local DEFAULT_TOOL_MATCHER = "Read|Edit|Write"

-- Claude events that accept `additionalContext`, so a native message hook can render on them.
local CONTEXT_EVENTS = {
  SessionStart = true,
  SubagentStart = true,
  UserPromptSubmit = true,
  UserPromptExpansion = true,
  PreToolUse = true,
  PostToolUse = true,
  PostToolUseFailure = true,
  PostToolBatch = true,
  PostModelSwitch = true,
}

local function join(list)
  if list == nil then
    return nil
  end
  return table.concat(list, "|")
end

local function tool_matcher(match)
  if match == nil then
    return DEFAULT_TOOL_MATCHER
  end
  local names = {}
  for i, word in ipairs(match) do
    names[i] = TOOLS[word] or word
  end
  return table.concat(names, "|")
end

-- The matcher for a hook on `neutral` (the neutral event name, or nil for a native hook).
-- `agents` holds the rendered names of `hook.agents`, or is nil when the hook has none.
local function matcher_for(hook, neutral, agents)
  if neutral == "tool.before" or neutral == "tool.after" then
    return tool_matcher(hook.match)
  elseif neutral == "agent.stop" then
    return join(agents)
  elseif neutral == "session.end" or neutral == "prompt.submit" then
    return nil
  end
  return join(hook.match)
end

local function contains(text, needle)
  return text:find(needle, 1, true) ~= nil
end

local function mentions_session_id(hook)
  local needle = "${CLAUDE_SESSION_ID}"
  if hook.kind == "message" then
    return contains(hook.text, needle)
  end
  for _, arg in ipairs(hook.command) do
    if contains(arg, needle) then
      return true
    end
  end
  return false
end

-- Removes every var form from `arg`, longest first, and reports whether a `$` is left.
local function has_literal_dollar(arg, vars)
  local forms = {}
  for _, form in pairs(vars) do
    forms[#forms + 1] = form
  end
  table.sort(forms, function(a, b)
    if #a ~= #b then
      return #a > #b
    end
    return a < b
  end)
  local rest = arg
  for _, form in ipairs(forms) do
    local out, from = {}, 1
    while true do
      local s, e = rest:find(form, from, true)
      if not s then
        break
      end
      out[#out + 1] = rest:sub(from, s - 1)
      from = e + 1
    end
    out[#out + 1] = rest:sub(from)
    rest = table.concat(out)
  end
  return contains(rest, "$")
end

local function quote_argument(arg)
  return '"' .. arg:gsub("[\\\"`]", "\\%0") .. '"'
end

local function message_command(event, text)
  local json = airsstack.json.encode({
    hookSpecificOutput = { hookEventName = event, additionalContext = text },
  })
  local escaped = json:gsub("'", "'\"'\"'")
  return "printf '%s\\n' '" .. escaped .. "'" .. M.MARKER
end

local function lua_command(argv)
  local quoted = {}
  for i, arg in ipairs(argv) do
    quoted[i] = quote_argument(arg)
  end
  return 'cd "$CLAUDE_PROJECT_DIR" && ' .. table.concat(quoted, " ") .. M.MARKER
end

-- Returns `{ event = <Claude event>, group = { matcher = <m>, hooks = {...} } }` for `hook`, or nil
-- and the reason Claude cannot express it. `vars` is the `values` var map (name to rendered form).
-- `agents` is the rendered name of each of `hook.agents`, in the hook's order, or nil when the hook
-- has none; an agent-stop matcher is built from it.
function M.render(hook, vars, agents)
  local neutral = NEUTRAL_EVENTS[hook.event] and hook.event or nil
  local event = NEUTRAL_EVENTS[hook.event] or hook.event
  local message = hook.kind == "message"

  if mentions_session_id(hook) then
    return nil, SESSION_ID_REASON
  end
  if message and neutral == "session.end" then
    return nil, SESSION_END_REASON
  end
  if message and (neutral == "agent.stop" or event == "Stop" or event == "SubagentStop") then
    return nil, AGENT_STOP_REASON
  end
  if message and not neutral and not CONTEXT_EVENTS[event] then
    return nil, "claude passes no hook message to the model on " .. event
  end

  local command
  if message then
    command = message_command(event, hook.text)
  else
    for _, arg in ipairs(hook.command) do
      if has_literal_dollar(arg, vars) then
        return nil,
          "claude runs hook commands through sh, which would expand the literal $ in argument \"" .. arg .. '"'
      end
    end
    command = lua_command(hook.command)
  end

  return {
    event = event,
    group = {
      matcher = matcher_for(hook, neutral, agents),
      hooks = { { type = "command", command = command } },
    },
  }
end

return M
