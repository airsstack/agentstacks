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

local function ok(hook, agents)
  local r, reason = hooks.render(hook, VARS, agents)
  assert(r, "expected a rendered hook, got skip: " .. tostring(reason))
  return r
end

local function skipped(hook, agents)
  local r, reason = hooks.render(hook, VARS, agents)
  assert(r == nil, "expected a skip")
  return reason
end

local function eq(got, want)
  assert(got == want, "expected " .. tostring(want) .. ", got " .. tostring(got))
end

return {
  maps_each_neutral_event = function()
    local cases = {
      { "session.start", nil, "SessionStart", nil },
      { "session.start", { "startup", "resume" }, "SessionStart", "startup|resume" },
      { "session.end", { "x" }, "SessionEnd", nil },
      { "prompt.submit", { "x" }, "UserPromptSubmit", nil },
      { "tool.before", nil, "PreToolUse", "Read|Edit|Write" },
      { "tool.after", nil, "PostToolUse", "Read|Edit|Write" },
      { "tool.before", { "read", "edit" }, "PreToolUse", "Read|Edit" },
      { "tool.after", { "write" }, "PostToolUse", "Write" },
      { "agent.stop", nil, "SubagentStop", nil },
    }
    for _, c in ipairs(cases) do
      local r = ok(lua_hook(c[1], { match = c[2] }))
      eq(r.event, c[3])
      eq(r.group.matcher, c[4])
      eq(r.group.hooks[1].type, "command")
    end
  end,

  session_start_clear_compact_matcher_renders = function()
    local r = ok(lua_hook("session.start", { match = { "clear", "compact" } }))
    eq(r.event, "SessionStart")
    eq(r.group.matcher, "clear|compact")
  end,

  native_match_joined_and_empty_match_has_none = function()
    local r = ok(lua_hook("SubagentStart", { match = { "a", "b" } }))
    eq(r.event, "SubagentStart")
    eq(r.group.matcher, "a|b")
    eq(ok(lua_hook("Notification")).group.matcher, nil)
    eq(ok(lua_hook("Notification", { match = {} })).group.matcher, nil)
  end,

  lua_command_wraps_quoted_argv_in_bin_sh = function()
    local h = lua_hook("session.start", { command = { "touch", "${TMPDIR:-/tmp}/it's \"q\" `b` \\z" } })
    eq(
      ok(h).group.hooks[1].command,
      [==[/bin/sh -c '"touch" "${TMPDIR:-/tmp}/it'\''s \"q\" \`b\` \\z"' # agentstacks]==]
    )
  end,

  lua_command_has_no_cd = function()
    local command = ok(lua_hook("tool.before", { command = { "airsl", "run" } })).group.hooks[1].command
    eq(command, [==[/bin/sh -c '"airsl" "run"' # agentstacks]==])
    assert(not command:find("cd ", 1, true))
  end,

  message_command_prints_additional_context_json = function()
    local r = ok(message_hook("session.start", 'MSG-R8 line1\nline2 it\'s "q" back\\slash'))
    eq(
      r.group.hooks[1].command,
      [==[printf '%s\n' '{"hookSpecificOutput":{"additionalContext":"MSG-R8 line1\nline2 it'"'"'s \"q\" back\\slash","hookEventName":"SessionStart"}}' # agentstacks]==]
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

  skips_session_id_var = function()
    local reason = "musecode gives hooks the session id in the event (event.session_id), "
      .. "not as {{var session_id}}"
    eq(skipped(message_hook("session.start", "id ${MUSE_SESSION_ID}")), reason)
    eq(skipped(lua_hook("tool.before", { command = { "airsl", "${MUSE_SESSION_ID}" } })), reason)
  end,

  skips_message_at_session_end = function()
    eq(skipped(message_hook("session.end", "bye")), "musecode shows no hook output to the model at session end")
  end,

  skips_agent_stop_with_agents = function()
    local hook = lua_hook("agent.stop", { agents = { "task-briefer" } })
    eq(
      skipped(hook, { "sdlc-task-briefer" }),
      "musecode gives hooks no agent name at agent stop, "
        .. "so it cannot run this hook for only the listed agents"
    )
  end,

  agent_stop_with_only_hook_agents_skips = function()
    eq(
      skipped(lua_hook("agent.stop", { agents = { "a" } })),
      "musecode gives hooks no agent name at agent stop, so it cannot run this hook for only the listed agents"
    )
  end,

  agent_stop_with_only_rendered_agents_skips = function()
    eq(
      skipped(lua_hook("agent.stop"), { "p-a" }),
      "musecode gives hooks no agent name at agent stop, so it cannot run this hook for only the listed agents"
    )
  end,

  agents_on_another_event_render = function()
    eq(ok(lua_hook("session.start", { agents = { "a" } }), { "p-a" }).event, "SessionStart")
  end,

  agent_stop_without_agents_renders = function()
    local r = ok(lua_hook("agent.stop"))
    eq(r.event, "SubagentStop")
    eq(r.group.matcher, nil)
  end,

  skips_message_at_agent_stop = function()
    eq(skipped(message_hook("agent.stop", "done")), "musecode passes no hook message to the model at agent stop")
  end,

  skips_unknown_native_event = function()
    eq(skipped(lua_hook("ToolUseStart")), 'musecode does not run hooks on "ToolUseStart"')
    eq(skipped(message_hook("Bogus", "x")), 'musecode does not run hooks on "Bogus"')
  end,

  native_onsave_is_unknown = function()
    eq(skipped(lua_hook("OnSave")), 'musecode does not run hooks on "OnSave"')
  end,

  every_native_event_of_the_eighteen_renders = function()
    local names = {
      "SessionStart", "UserPromptSubmit", "PreToolUse", "PermissionRequest", "PostToolUse",
      "PostToolUseFailure", "PostToolBatch", "PreLLMCall", "PostLLMCall", "PreCompact",
      "PostCompact", "SubagentStart", "SubagentStop", "Stop", "StopFailure", "SessionEnd",
      "Notification", "SessionFork",
    }
    eq(#names, 18)
    for _, name in ipairs(names) do
      eq(ok(lua_hook(name)).event, name)
    end
  end,

  skips_native_message_without_context = function()
    eq(skipped(message_hook("Stop", "x")), "musecode passes no hook message to the model on Stop")
    eq(skipped(message_hook("SubagentStop", "x")), "musecode passes no hook message to the model on SubagentStop")
    eq(skipped(message_hook("SessionEnd", "x")), "musecode passes no hook message to the model on SessionEnd")
  end,

  native_precompact_message_is_skipped = function()
    eq(skipped(message_hook("PreCompact", "x")), "musecode passes no hook message to the model on PreCompact")
  end,

  native_session_start_message_renders = function()
    local r = ok(message_hook("SessionStart", "hello", { match = { "startup" } }))
    eq(r.event, "SessionStart")
    eq(r.group.matcher, "startup")
    assert(r.group.hooks[1].command:find('"hookEventName":"SessionStart"', 1, true))
  end,

  native_message_on_the_six_context_events_renders = function()
    local names = {
      "SessionStart", "SubagentStart", "UserPromptSubmit", "PreToolUse", "PostToolUse", "PostToolUseFailure",
    }
    for _, name in ipairs(names) do
      eq(ok(message_hook(name, "x")).event, name)
    end
  end,

  skips_matcher_on_unmatchable_event = function()
    eq(
      skipped(lua_hook("UserPromptSubmit", { match = { "x" } })),
      "musecode refuses a matcher on UserPromptSubmit"
    )
    eq(skipped(lua_hook("Stop", { match = { "x" } })), "musecode refuses a matcher on Stop")
  end,

  native_user_prompt_submit_empty_match_renders = function()
    local r = ok(lua_hook("UserPromptSubmit", { match = {} }))
    eq(r.event, "UserPromptSubmit")
    eq(r.group.matcher, nil)
    eq(ok(lua_hook("Stop")).group.matcher, nil)
  end,

  neutral_prompt_submit_with_match_renders_without_matcher = function()
    eq(ok(lua_hook("prompt.submit", { match = { "x" } })).group.matcher, nil)
  end,

  skips_literal_dollar_in_argument = function()
    eq(
      skipped(lua_hook("tool.before", { command = { "airsl", "^a$" } })),
      'musecode runs hook commands through sh, which would expand the literal $ in argument "^a$"'
    )
  end,

  var_forms_in_argv_render = function()
    local r = ok(lua_hook("tool.before", {
      command = { "airsl", "$HOME/x", "${AIRSSTACK_HOME:-$HOME/.airsstack}/y", "${TMPDIR:-/tmp}/z" },
    }))
    eq(
      r.group.hooks[1].command,
      [==[/bin/sh -c '"airsl" "$HOME/x" "${AIRSSTACK_HOME:-$HOME/.airsstack}/y" "${TMPDIR:-/tmp}/z"' # agentstacks]==]
    )
  end,

  skips_follow_table_order = function()
    local both = lua_hook("tool.before", { command = { "airsl", "${MUSE_SESSION_ID}", "^a$" } })
    eq(
      skipped(both),
      "musecode gives hooks the session id in the event (event.session_id), not as {{var session_id}}"
    )
    local session_end_message = message_hook("session.end", "x")
    eq(skipped(session_end_message), "musecode shows no hook output to the model at session end")
    local agent_stop_with_agents = lua_hook("agent.stop", { agents = { "a" }, command = { "airsl", "^a$" } })
    eq(
      skipped(agent_stop_with_agents, { "p-a" }),
      "musecode gives hooks no agent name at agent stop, so it cannot run this hook for only the listed agents"
    )
    local unknown_event = lua_hook("OnSave", { command = { "airsl", "^a$" } })
    eq(skipped(unknown_event), 'musecode does not run hooks on "OnSave"')
    local stop_with_matcher = lua_hook("Stop", { match = { "x" }, command = { "airsl", "^a$" } })
    eq(skipped(stop_with_matcher), "musecode refuses a matcher on Stop")
  end,
}
