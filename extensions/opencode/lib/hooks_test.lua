local hooks = require("hooks")

local VARS = {
  tmpdir = "${TMPDIR:-/tmp}",
  home = "$HOME",
  airsstack_home = "${AIRSSTACK_HOME:-$HOME/.airsstack}",
}

local function lua_hook(event, extra)
  local h = { id = "p/hook/0", kind = "lua", event = event, may_block = false, command = { "airsl", "run" } }
  for k, v in pairs(extra or {}) do
    h[k] = v
  end
  return h
end

local function message_hook(event, text, extra)
  local h = { id = "p/hook/1", kind = "message", event = event, text = text }
  for k, v in pairs(extra or {}) do
    h[k] = v
  end
  return h
end

local function ok(hook, native)
  local r, reason = hooks.entry(hook, VARS, native or false)
  assert(r, "expected an entry, got skip: " .. tostring(reason))
  return r
end

local function skipped(hook, native)
  local r, reason = hooks.entry(hook, VARS, native or false)
  assert(r == nil, "expected a skip")
  return reason
end

local function eq(got, want)
  assert(got == want, "expected " .. tostring(want) .. ", got " .. tostring(got))
end

local function list_eq(got, want)
  assert(type(got) == "table", "expected a list, got " .. tostring(got))
  eq(#got, #want)
  for i, v in ipairs(want) do
    eq(got[i], v)
  end
end

local function no_tools_no_match(entry)
  eq(entry.tools, nil)
  eq(entry.match, nil)
end

local SESSION_END_REASON = "opencode has no hook when a session ends"
local AGENT_STOP_REASON = "opencode has no hook when a subagent stops"
local SESSION_START_REASON = "opencode runs session.start hooks only for a new session "
  .. '(source "startup"), which this hook does not match'
local NO_BLOCK_REASON = "opencode cannot block a prompt"
local NO_MATCHER_REASON = "opencode has no matcher for session.prompt"
local SESSION_ID_REASON = "opencode gives hooks the session id in the event (event.session_id), "
  .. "not as {{var session_id}}"

local function native_event_reason(event)
  return 'opencode has no native hook event "'
    .. event
    .. '" (use tool.execute.before, tool.execute.after or session.prompt)'
end

return {
  tool_before_write_maps_to_write_and_patch = function()
    local e = ok(lua_hook("tool.before", { match = { "write" } }))
    eq(e.on, "tool.before")
    list_eq(e.tools, { "write", "patch" })
    list_eq(e.match, { "write" })
  end,

  tool_after_without_match_runs_for_all_file_tools = function()
    local e = ok(lua_hook("tool.after"))
    eq(e.on, "tool.after")
    list_eq(e.tools, { "read", "edit", "write", "patch" })
    eq(e.match, nil)
  end,

  edit_and_write_share_patch_once = function()
    local e = ok(lua_hook("tool.before", { match = { "edit", "write" } }))
    list_eq(e.tools, { "edit", "patch", "write" })
    list_eq(e.match, { "edit", "write" })
  end,

  prompt_and_session_start_have_no_tools = function()
    local p = ok(lua_hook("prompt.submit"))
    eq(p.on, "prompt.submit")
    no_tools_no_match(p)
    local s = ok(lua_hook("session.start", { match = { "startup", "resume" } }))
    eq(s.on, "session.start")
    no_tools_no_match(s)
  end,

  native_tool_hook_keeps_its_match_as_tools = function()
    local e = ok(lua_hook("tool.execute.before", { match = { "shell" } }), true)
    eq(e.on, "tool.execute.before")
    list_eq(e.tools, { "shell" })
    eq(e.match, nil)
  end,

  native_tool_hook_without_match_has_no_tools = function()
    local a = ok(lua_hook("tool.execute.after"), true)
    eq(a.on, "tool.execute.after")
    no_tools_no_match(a)
    local b = ok(lua_hook("tool.execute.before", { match = {} }), true)
    eq(b.on, "tool.execute.before")
    no_tools_no_match(b)
  end,

  native_prompt_message_renders = function()
    local e = ok(message_hook("session.prompt", "hello"), true)
    eq(e.on, "session.prompt")
    eq(e.text, "hello")
    eq(e.command, nil)
    no_tools_no_match(e)
  end,

  native_prompt_lua_renders = function()
    local e = ok(lua_hook("session.prompt"), true)
    eq(e.on, "session.prompt")
    eq(e.command, '"airsl" "run"')
    no_tools_no_match(e)
    local empty = ok(lua_hook("session.prompt", { match = {} }), true)
    eq(empty.on, "session.prompt")
    no_tools_no_match(empty)
  end,

  native_flag_decides_not_the_event_name = function()
    eq(skipped(message_hook("prompt.submit", "hi"), true), native_event_reason("prompt.submit"))
    eq(ok(message_hook("prompt.submit", "hi"), false).on, "prompt.submit")
  end,

  native_hooks_on_neutral_names_get_the_native_reason = function()
    eq(skipped(message_hook("session.end", "bye"), true), native_event_reason("session.end"))
    eq(skipped(lua_hook("agent.stop"), true), native_event_reason("agent.stop"))
    eq(skipped(lua_hook("session.start"), true), native_event_reason("session.start"))
    eq(skipped(lua_hook("prompt.submit", { may_block = true }), true), native_event_reason("prompt.submit"))
  end,

  session_start_without_match_renders = function()
    local e = ok(lua_hook("session.start"))
    eq(e.on, "session.start")
    no_tools_no_match(e)
  end,

  message_hook_gives_text = function()
    local e = ok(message_hook("prompt.submit", "remember the style guide"))
    eq(e.text, "remember the style guide")
    eq(e.command, nil)
  end,

  lua_hook_command_is_quoted = function()
    local e = ok(lua_hook("prompt.submit", { command = { "airsl", 'a"b', "c\\d", "e`f" } }))
    eq(e.command, '"airsl" "a\\"b" "c\\\\d" "e\\`f"')
    eq(e.text, nil)
  end,

  skips_session_end = function()
    eq(skipped(message_hook("session.end", "bye")), SESSION_END_REASON)
    eq(skipped(lua_hook("session.end")), SESSION_END_REASON)
  end,

  skips_agent_stop = function()
    eq(skipped(message_hook("agent.stop", "done")), AGENT_STOP_REASON)
    eq(skipped(lua_hook("agent.stop")), AGENT_STOP_REASON)
  end,

  skips_session_start_without_startup = function()
    eq(skipped(lua_hook("session.start", { match = { "compact" } })), SESSION_START_REASON)
    eq(skipped(message_hook("session.start", "hi", { match = { "resume", "clear" } })), SESSION_START_REASON)
  end,

  skips_may_block_prompt = function()
    eq(skipped(lua_hook("prompt.submit", { may_block = true })), NO_BLOCK_REASON)
    eq(skipped(lua_hook("session.prompt", { may_block = true }), true), NO_BLOCK_REASON)
    -- A message hook on either renders.
    eq(ok(message_hook("prompt.submit", "hi", { may_block = true })).on, "prompt.submit")
  end,

  skips_native_prompt_with_match = function()
    eq(skipped(message_hook("session.prompt", "hi", { match = { "x" } }), true), NO_MATCHER_REASON)
    -- Rule 4 is checked before rule 5.
    eq(skipped(lua_hook("session.prompt", { may_block = true, match = { "x" } }), true), NO_BLOCK_REASON)
  end,

  skips_other_native_event = function()
    eq(skipped(lua_hook("file.edited"), true), native_event_reason("file.edited"))
    eq(skipped(message_hook("session.idle", "hi"), true), native_event_reason("session.idle"))
  end,

  skips_session_id_in_hook = function()
    eq(skipped(message_hook("prompt.submit", "id ${OPENCODE_SESSION_ID}")), SESSION_ID_REASON)
    eq(skipped(lua_hook("prompt.submit", { command = { "airsl", "${OPENCODE_SESSION_ID}" } })), SESSION_ID_REASON)
    eq(
      skipped(lua_hook("tool.execute.after", { command = { "x", "${OPENCODE_SESSION_ID}" } }), true),
      SESSION_ID_REASON
    )
  end,

  skips_follow_table_order = function()
    -- Rule 1 beats rule 7, and rule 7 beats rule 8, when a hook matches both.
    eq(skipped(message_hook("session.end", "${OPENCODE_SESSION_ID}")), SESSION_END_REASON)
    eq(skipped(lua_hook("prompt.submit", { command = { "a$", "${OPENCODE_SESSION_ID}" } })), SESSION_ID_REASON)
  end,

  skips_literal_dollar = function()
    local reason = skipped(lua_hook("prompt.submit", { command = { "airsl", "^a$" } }))
    eq(reason, 'opencode runs hook commands through sh, which would expand the literal $ in argument "^a$"')
  end,

  renders_var_form_with_dollar = function()
    local e = ok(lua_hook("prompt.submit", { command = { "airsl", "${TMPDIR:-/tmp}/x" } }))
    eq(e.command, '"airsl" "${TMPDIR:-/tmp}/x"')
  end,

  session_start_with_startup_in_match_renders = function()
    local e = ok(message_hook("session.start", "hi", { match = { "startup", "resume", "clear" } }))
    eq(e.on, "session.start")
    eq(e.text, "hi")
    no_tools_no_match(e)
  end,
}
