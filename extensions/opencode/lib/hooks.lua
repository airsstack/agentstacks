-- Renders one payload hook as an entry of the generated OpenCode plugin.
--
-- Pure: it takes the hook, the rendered var forms and whether the hook came from the pack's native
-- hooks, and returns the plugin entry, or nil and the reason the hook cannot be expressed. It calls
-- no `agentstacks.*` and no `require`.

local M = {}

local SESSION_END_REASON = "opencode has no hook when a session ends"
local AGENT_STOP_REASON = "opencode has no hook when a subagent stops"
local SESSION_START_REASON = "opencode runs session.start hooks only for a new session "
  .. '(source "startup"), which this hook does not match'
local NO_BLOCK_REASON = "opencode cannot block a prompt"
local NO_MATCHER_REASON = "opencode has no matcher for session.prompt"
local SESSION_ID_REASON = "opencode gives hooks the session id in the event (event.session_id), "
  .. "not as {{var session_id}}"

-- The OpenCode tools a neutral `match` word runs for.
local TOOLS = {
  read = { "read" },
  edit = { "edit", "patch" },
  write = { "write", "patch" },
}
local ALL_FILE_TOOLS = { "read", "edit", "write", "patch" }

local NATIVE_TOOL_EVENTS = { ["tool.execute.before"] = true, ["tool.execute.after"] = true }

local function copy(list)
  local out = {}
  for i, v in ipairs(list) do
    out[i] = v
  end
  return out
end

local function non_empty(list)
  return list ~= nil and #list > 0
end

local function contains(text, needle)
  return text:find(needle, 1, true) ~= nil
end

local function mentions_session_id(hook)
  local needle = "${OPENCODE_SESSION_ID}"
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

local function command_text(argv)
  local quoted = {}
  for i, arg in ipairs(argv) do
    quoted[i] = quote_argument(arg)
  end
  return table.concat(quoted, " ")
end

-- The OpenCode tools for a neutral `match`: each word's tools, duplicates dropped, first seen first.
-- A word that is not a neutral tool name stands for itself. No `match` means every file tool.
local function neutral_tools(match)
  if not non_empty(match) then
    return copy(ALL_FILE_TOOLS)
  end
  local tools, seen = {}, {}
  for _, word in ipairs(match) do
    for _, tool in ipairs(TOOLS[word] or { word }) do
      if not seen[tool] then
        seen[tool] = true
        tools[#tools + 1] = tool
      end
    end
  end
  return tools
end

local function holds(list, word)
  for _, v in ipairs(list or {}) do
    if v == word then
      return true
    end
  end
  return false
end

-- Returns the reason `hook` is skipped, or nil. The checks follow the skip table's order, each
-- one only for the hooks the table says it applies to. A `session.start` hook with no
-- `match` renders; only one whose non-empty `match` lacks `startup` is skipped.
local function skip_reason(hook, vars, native)
  local event = hook.event
  local lua = hook.kind == "lua"
  if not native then
    if event == "session.end" then
      return SESSION_END_REASON
    elseif event == "agent.stop" then
      return AGENT_STOP_REASON
    elseif event == "session.start" and non_empty(hook.match) and not holds(hook.match, "startup") then
      return SESSION_START_REASON
    end
  end
  local prompt_event = native and "session.prompt" or "prompt.submit"
  if lua and hook.may_block and event == prompt_event then
    return NO_BLOCK_REASON
  end
  if native then
    if event == "session.prompt" and non_empty(hook.match) then
      return NO_MATCHER_REASON
    elseif event ~= "session.prompt" and not NATIVE_TOOL_EVENTS[event] then
      return 'opencode has no native hook event "'
        .. event
        .. '" (use tool.execute.before, tool.execute.after or session.prompt)'
    end
  end
  if mentions_session_id(hook) then
    return SESSION_ID_REASON
  end
  if lua then
    for _, arg in ipairs(hook.command) do
      if has_literal_dollar(arg, vars) then
        return 'opencode runs hook commands through sh, which would expand the literal $ in argument "' .. arg .. '"'
      end
    end
  end
  return nil
end

-- Returns the plugin entry for `hook`, or nil and the reason OpenCode cannot express it. `vars` is
-- the `values` var map (name to rendered form). `native` is true when the hook came from the pack's
-- native hooks; the event name never decides it. An entry holds `on`, `tools` (tool events only,
-- never empty), `match` (neutral tool events only, never empty) and `command` or `text`.
function M.entry(hook, vars, native)
  local reason = skip_reason(hook, vars, native)
  if reason then
    return nil, reason
  end

  local entry = { on = hook.event }
  if hook.kind == "message" then
    entry.text = hook.text
  else
    entry.command = command_text(hook.command)
  end

  if not native and (hook.event == "tool.before" or hook.event == "tool.after") then
    entry.tools = neutral_tools(hook.match)
    if non_empty(hook.match) then
      entry.match = copy(hook.match)
    end
  elseif native and NATIVE_TOOL_EVENTS[hook.event] and non_empty(hook.match) then
    entry.tools = copy(hook.match)
  end
  return entry
end

return M
