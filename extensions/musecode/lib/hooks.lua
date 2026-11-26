-- Renders one payload hook as a Muse Code hook group.
--
-- Pure: it takes the hook, the rendered var forms and the rendered agent names and returns the
-- Muse event with its group, or nil and the reason the hook cannot be expressed. It calls no
-- `agentstacks.*` and no `require`.

local M = {}

-- Ends every command this module writes, so `hookfile` can tell a handler it owns from the user's.
M.MARKER = " # agentstacks"

local SESSION_ID_REASON = "musecode gives hooks the session id in the event (event.session_id), "
  .. "not as {{var session_id}}"
local SESSION_END_REASON = "musecode shows no hook output to the model at session end"
local AGENT_NAME_REASON = "musecode gives hooks no agent name at agent stop, "
  .. "so it cannot run this hook for only the listed agents"
local AGENT_STOP_REASON = "musecode passes no hook message to the model at agent stop"

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

-- The events Muse Code runs a native hook on.
local NATIVE_EVENTS = {
  SessionStart = true,
  UserPromptSubmit = true,
  PreToolUse = true,
  PermissionRequest = true,
  PostToolUse = true,
  PostToolUseFailure = true,
  PostToolBatch = true,
  PreLLMCall = true,
  PostLLMCall = true,
  PreCompact = true,
  PostCompact = true,
  SubagentStart = true,
  SubagentStop = true,
  Stop = true,
  StopFailure = true,
  SessionEnd = true,
  Notification = true,
  SessionFork = true,
}

-- The events whose `additionalContext` reaches the model, so a native message hook can render on them.
local CONTEXT_EVENTS = {
  SessionStart = true,
  SubagentStart = true,
  UserPromptSubmit = true,
  PreToolUse = true,
  PostToolUse = true,
  PostToolUseFailure = true,
}

-- The events Muse Code refuses a matcher on.
local UNMATCHABLE_EVENTS = { UserPromptSubmit = true, Stop = true }

local function has_items(list)
  return list ~= nil and #list > 0
end

-- `list` joined with `|`, or nil when it is absent or empty.
local function join(list)
  if not has_items(list) then
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
local function matcher_for(hook, neutral)
  if neutral == "tool.before" or neutral == "tool.after" then
    return tool_matcher(hook.match)
  elseif neutral == "agent.stop" or neutral == "session.end" or neutral == "prompt.submit" then
    return nil
  end
  return join(hook.match)
end

local function contains(text, needle)
  return text:find(needle, 1, true) ~= nil
end

local function mentions_session_id(hook)
  local needle = "${MUSE_SESSION_ID}"
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

-- `/bin/sh -c '<argv>'`: Muse Code runs a hook command as `$SHELL -c`, so the quoted argv is
-- wrapped in single quotes for the user's `SHELL` to see one word, and `/bin/sh` parses the argv.
local function lua_command(argv)
  local quoted = {}
  for i, arg in ipairs(argv) do
    quoted[i] = quote_argument(arg)
  end
  local wrapped = table.concat(quoted, " "):gsub("'", "'\\''")
  return "/bin/sh -c '" .. wrapped .. "'" .. M.MARKER
end

-- The reason `hook` cannot be expressed, or nil. The rules apply in the order they are listed.
local function skip_reason(hook, vars, neutral, event, has_agents)
  local message = hook.kind == "message"

  if mentions_session_id(hook) then
    return SESSION_ID_REASON
  end
  if message and neutral == "session.end" then
    return SESSION_END_REASON
  end
  if neutral == "agent.stop" and has_agents then
    return AGENT_NAME_REASON
  end
  if message and neutral == "agent.stop" then
    return AGENT_STOP_REASON
  end
  if not neutral and not NATIVE_EVENTS[event] then
    return 'musecode does not run hooks on "' .. event .. '"'
  end
  if message and not neutral and not CONTEXT_EVENTS[event] then
    return "musecode passes no hook message to the model on " .. event
  end
  if not neutral and UNMATCHABLE_EVENTS[event] and has_items(hook.match) then
    return "musecode refuses a matcher on " .. event
  end
  if not message then
    for _, arg in ipairs(hook.command) do
      if has_literal_dollar(arg, vars) then
        return "musecode runs hook commands through sh, which would expand the literal $ in argument \""
          .. arg
          .. '"'
      end
    end
  end
  return nil
end

-- Returns `{ event = <Muse event>, group = { matcher = <m>, hooks = {...} } }` for `hook`, or nil
-- and the reason Muse Code cannot express it. `vars` is the `values` var map (name to rendered
-- form). `agents` is the rendered name of each of `hook.agents`, or nil when the hook has none; an
-- `agent.stop` hook with agents is skipped because Muse Code gives the hook no agent name.
function M.render(hook, vars, agents)
  local neutral = NEUTRAL_EVENTS[hook.event] and hook.event or nil
  local event = NEUTRAL_EVENTS[hook.event] or hook.event
  local has_agents = has_items(hook.agents) or has_items(agents)

  local reason = skip_reason(hook, vars, neutral, event, has_agents)
  if reason then
    return nil, reason
  end

  local command
  if hook.kind == "message" then
    command = message_command(event, hook.text)
  else
    command = lua_command(hook.command)
  end

  return {
    event = event,
    group = {
      matcher = matcher_for(hook, neutral),
      hooks = { { type = "command", command = command } },
    },
  }
end

return M
