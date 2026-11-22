local hooks = require("hooks")

local VARS = {
  tmpdir = "$TMPDIR",
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

local function ok(hook)
  local r, reason = hooks.render(hook, VARS)
  assert(r, "expected a rendered hook, got skip: " .. tostring(reason))
  return r
end

local function skipped(hook)
  local r, reason = hooks.render(hook, VARS)
  assert(r == nil, "expected a skip")
  return reason
end

local function eq(got, want)
  assert(got == want, "expected " .. tostring(want) .. ", got " .. tostring(got))
end

local AGENT_STOP_REASON = "claude treats hook output at agent stop as feedback that keeps the agent running"

return {
  each_neutral_event_maps_to_its_claude_event = function()
    local cases = {
      { "session.start", "SessionStart" },
      { "session.end", "SessionEnd" },
      { "prompt.submit", "UserPromptSubmit" },
      { "tool.before", "PreToolUse" },
      { "tool.after", "PostToolUse" },
      { "agent.stop", "SubagentStop" },
    }
    for _, c in ipairs(cases) do
      eq(ok(lua_hook(c[1])).event, c[2])
    end
  end,

  tool_match_maps_and_joins = function()
    local r = ok(lua_hook("tool.before", { match = { "read", "edit" } }))
    eq(r.group.matcher, "Read|Edit")
    r = ok(lua_hook("tool.after", { match = { "write" } }))
    eq(r.group.matcher, "Write")
  end,

  tool_match_absent_defaults_to_read_edit_write = function()
    eq(ok(lua_hook("tool.before")).group.matcher, "Read|Edit|Write")
    eq(ok(lua_hook("tool.after")).group.matcher, "Read|Edit|Write")
  end,

  agent_stop_matcher_joins_agents = function()
    eq(ok(lua_hook("agent.stop", { agents = { "coder", "reviewer" } })).group.matcher, "coder|reviewer")
    eq(ok(lua_hook("agent.stop")).group.matcher, nil)
  end,

  prompt_submit_has_no_matcher = function()
    eq(ok(lua_hook("prompt.submit", { match = { "x" } })).group.matcher, nil)
    eq(ok(lua_hook("session.end")).group.matcher, nil)
  end,

  session_start_matcher_joins_match = function()
    eq(ok(lua_hook("session.start", { match = { "startup", "resume" } })).group.matcher, "startup|resume")
    eq(ok(lua_hook("session.start")).group.matcher, nil)
  end,

  native_hook_keeps_event_and_match = function()
    local r = ok(lua_hook("SubagentStop", { match = { "a", "b" } }))
    eq(r.event, "SubagentStop")
    eq(r.group.matcher, "a|b")
    r = ok(lua_hook("Notification"))
    eq(r.event, "Notification")
    eq(r.group.matcher, nil)
  end,

  message_command_is_printf_of_additional_context_json = function()
    local r = ok(message_hook("session.start", "hello"))
    local json = airsstack.json.encode({
      hookSpecificOutput = { hookEventName = "SessionStart", additionalContext = "hello" },
    })
    eq(r.group.hooks[1].type, "command")
    eq(r.group.hooks[1].command, "printf '%s\\n' '" .. json .. "' # agentstacks")
  end,

  single_quote_in_message_is_shell_escaped = function()
    local r = ok(message_hook("prompt.submit", "it's"))
    local json = airsstack.json.encode({
      hookSpecificOutput = { hookEventName = "UserPromptSubmit", additionalContext = "it's" },
    })
    local escaped = json:gsub("'", "'\"'\"'")
    assert(escaped ~= json)
    eq(r.group.hooks[1].command, "printf '%s\\n' '" .. escaped .. "' # agentstacks")
  end,

  lua_command_matches_the_spec_example = function()
    local h = lua_hook("tool.before", {
      command = {
        "airsl", "run", "--fail-open", "--policy", "confined",
        "--allow-read", ".claude/agentstacks/core",
        "--allow-write", "$TMPDIR",
        "--allow-env", "HOME",
        ".claude/agentstacks/runtime.lua", "core.hooks.enforce", "0", "neutral",
        "--x", 'a "q" \\b `c`',
      },
    })
    local r = ok(h)
    eq(
      r.group.hooks[1].command,
      'cd "$CLAUDE_PROJECT_DIR" && "airsl" "run" "--fail-open" "--policy" "confined" "--allow-read" '
        .. '".claude/agentstacks/core" "--allow-write" "$TMPDIR" "--allow-env" "HOME" '
        .. '".claude/agentstacks/runtime.lua" "core.hooks.enforce" "0" "neutral" "--x" '
        .. '"a \\"q\\" \\\\b \\`c\\`" # agentstacks'
    )
  end,

  dollar_is_left_unescaped = function()
    local r = ok(lua_hook("tool.before", { command = { "airsl", "$TMPDIR" } }))
    eq(r.group.hooks[1].command, 'cd "$CLAUDE_PROJECT_DIR" && "airsl" "$TMPDIR" # agentstacks')
  end,

  session_id_var_in_message_is_skipped = function()
    eq(
      skipped(message_hook("session.start", "id ${CLAUDE_SESSION_ID}")),
      "claude gives hooks the session id in the event (event.session_id), not as {{var session_id}}"
    )
  end,

  session_id_var_in_argv_is_skipped = function()
    eq(
      skipped(lua_hook("tool.before", { command = { "airsl", "${CLAUDE_SESSION_ID}" } })),
      "claude gives hooks the session id in the event (event.session_id), not as {{var session_id}}"
    )
  end,

  message_on_session_end_is_skipped = function()
    eq(skipped(message_hook("session.end", "bye")), "claude shows no hook output to the model at session end")
  end,

  message_on_agent_stop_is_skipped = function()
    eq(skipped(message_hook("agent.stop", "done")), AGENT_STOP_REASON)
  end,

  native_message_on_stop_and_subagent_stop_is_skipped = function()
    eq(skipped(message_hook("Stop", "x")), AGENT_STOP_REASON)
    eq(skipped(message_hook("SubagentStop", "x")), AGENT_STOP_REASON)
  end,

  native_message_on_notification_is_skipped = function()
    eq(skipped(message_hook("Notification", "x")), "claude passes no hook message to the model on Notification")
  end,

  native_message_on_post_tool_use_failure_renders = function()
    local r = ok(message_hook("PostToolUseFailure", "x", { match = { "Bash" } }))
    eq(r.event, "PostToolUseFailure")
    eq(r.group.matcher, "Bash")
    assert(r.group.hooks[1].command:find('"hookEventName":"PostToolUseFailure"', 1, true))
  end,

  literal_dollar_in_argv_is_skipped = function()
    eq(
      skipped(lua_hook("tool.before", { command = { "airsl", "^a$" } })),
      'claude runs hook commands through sh, which would expand the literal $ in argument "^a$"'
    )
  end,

  var_forms_in_argv_render = function()
    local r = ok(lua_hook("tool.before", {
      command = { "airsl", "$TMPDIR/x", "${AIRSSTACK_HOME:-$HOME/.airsstack}/y" },
    }))
    eq(
      r.group.hooks[1].command,
      'cd "$CLAUDE_PROJECT_DIR" && "airsl" "$TMPDIR/x" "${AIRSSTACK_HOME:-$HOME/.airsstack}/y" # agentstacks'
    )
  end,

  every_command_ends_with_the_marker = function()
    eq(hooks.MARKER, " # agentstacks")
    local all = {
      ok(lua_hook("tool.before")),
      ok(message_hook("session.start", "hi")),
      ok(lua_hook("Notification")),
    }
    for _, r in ipairs(all) do
      local c = r.group.hooks[1].command
      eq(c:sub(-#hooks.MARKER), hooks.MARKER)
    end
  end,
}
