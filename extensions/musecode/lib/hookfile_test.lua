local hookfile = require("hookfile")

local MARKER = " # agentstacks"

local function decode(text)
  return airsstack.json.decode(text)
end

local function group(command, matcher)
  return { matcher = matcher, hooks = { { type = "command", command = command } } }
end

local function find(result, event)
  for _, e in ipairs(result.edits) do
    if e[1] == event then
      return e
    end
  end
  return nil
end

local OLD_HOOKFILE = [[
{
  "permissions": {
    "allow": ["Bash(ls:*)"]
  },
  "hooks": {
    "SessionEnd": [
      { "hooks": [ { "type": "command", "command": "echo user-hook", "timeout": 5 } ] },
      { "hooks": [ { "type": "command", "command": "echo old # agentstacks" } ] }
    ]
  }
}
]]

return {
  nil_empty_and_whitespace_text_decode_to_an_empty_table = function()
    for _, text in ipairs({ "", "  \n\t" }) do
      local v = hookfile.decode(text)
      assert(type(v) == "table" and next(v) == nil)
    end
    local v = hookfile.decode(nil)
    assert(type(v) == "table" and next(v) == nil)
  end,

  decode_parses_json_text = function()
    local v = hookfile.decode('{"a": [1]}')
    assert(v.a[1] == 1)
  end,

  marked_handler_is_replaced_and_user_handler_kept = function()
    local ours = { group("printf '%s\\n' 'new'" .. MARKER) }
    local r = hookfile.edits(decode(OLD_HOOKFILE), {
      SessionEnd = ours,
      PreToolUse = { group('echo pre' .. MARKER, "Read|Edit") },
    }, MARKER)
    local e = find(r, "SessionEnd")
    assert(e ~= nil)
    assert(#e[2] == 2)
    assert(e[2][1].hooks[1].command == "echo user-hook")
    assert(e[2][1].hooks[1].timeout == 5)
    assert(e[2][2] == ours[1])
    assert(next(r.skipped) == nil)
  end,

  new_event_is_added = function()
    local r = hookfile.edits(
      decode(OLD_HOOKFILE),
      { PreToolUse = { group("x" .. MARKER, "Read") } },
      MARKER
    )
    local e = find(r, "PreToolUse")
    assert(e ~= nil)
    assert(#e[2] == 1 and e[2][1].matcher == "Read")
    -- SessionEnd holds a marked handler, so it is touched even without our groups.
    assert(find(r, "SessionEnd") ~= nil)
  end,

  group_with_only_marked_handlers_is_dropped = function()
    local old = decode([[{"hooks":{"Stop":[
      {"hooks":[{"type":"command","command":"a # agentstacks"}]},
      {"hooks":[{"type":"command","command":"keep"}]}
    ]}}]])
    local r = hookfile.edits(old, { Stop = { group("b" .. MARKER) } }, MARKER)
    local e = find(r, "Stop")
    assert(#e[2] == 2)
    assert(e[2][1].hooks[1].command == "keep")
    assert(e[2][2].hooks[1].command == "b" .. MARKER)
  end,

  event_left_empty_is_removed = function()
    local old = decode([[{"hooks":{"Stop":[
      {"hooks":[{"type":"command","command":"a # agentstacks"}]}
    ]}}]])
    local r = hookfile.edits(old, {}, MARKER)
    local e = find(r, "Stop")
    assert(e ~= nil)
    assert(e[2] == nil)
    assert(#r.edits == 1)
  end,

  null_inside_an_event_makes_it_unrewritable = function()
    local old = decode([[{"hooks":{"Stop":[
      {"hooks":[{"type":"command","command":"a # agentstacks","timeout":null}]}
    ]}}]])
    local r = hookfile.edits(old, { Stop = { group("b" .. MARKER) } }, MARKER)
    assert(r.skipped.Stop == true)
    assert(find(r, "Stop") == nil)
  end,

  empty_list_inside_an_event_makes_it_unrewritable = function()
    local old = decode([[{"hooks":{"Stop":[
      {"hooks":[{"type":"command","command":"a # agentstacks","tags":[]}]}
    ]}}]])
    local r = hookfile.edits(old, { Stop = { group("b" .. MARKER) } }, MARKER)
    assert(r.skipped.Stop == true)
    assert(find(r, "Stop") == nil)
  end,

  empty_hooks_list_in_a_group_makes_it_unrewritable = function()
    local old = decode([[{"hooks":{"Stop":[
      {"hooks":[]},
      {"hooks":[{"type":"command","command":"a # agentstacks"}]}
    ]}}]])
    local r = hookfile.edits(old, {}, MARKER)
    assert(r.skipped.Stop == true)
    assert(#r.edits == 0)
  end,

  non_string_command_makes_it_unrewritable = function()
    local old = decode([[{"hooks":{"Stop":[
      {"hooks":[{"type":"command","command":"a # agentstacks"},{"type":"command","command":5}]}
    ]}}]])
    local r = hookfile.edits(old, {}, MARKER)
    assert(r.skipped.Stop == true)
    assert(#r.edits == 0)
  end,

  event_that_is_not_a_list_of_groups_is_unrewritable_when_we_add_to_it = function()
    local old = decode('{"hooks":{"Stop":{"hooks":"x"}}}')
    local r = hookfile.edits(old, { Stop = { group("b" .. MARKER) } }, MARKER)
    assert(r.skipped.Stop == true)
    assert(#r.edits == 0)
  end,

  unrewritable_event_with_a_marked_handler_gets_no_edit = function()
    local old = decode([[{"hooks":{
      "Stop":[{"hooks":[{"type":"command","command":"a # agentstacks","x":null}]}],
      "SessionEnd":[{"hooks":[{"type":"command","command":"b # agentstacks"}]}]
    }}]])
    local r = hookfile.edits(old, {}, MARKER)
    assert(r.skipped.Stop == true)
    assert(r.skipped.SessionEnd == nil)
    assert(#r.edits == 1 and r.edits[1][1] == "SessionEnd")
  end,

  untouched_unrewritable_event_is_left_alone = function()
    local old = decode('{"hooks":{"Stop":[{"hooks":[]}]}}')
    local r = hookfile.edits(old, { PreToolUse = { group("x" .. MARKER) } }, MARKER)
    assert(next(r.skipped) == nil)
    assert(#r.edits == 1 and r.edits[1][1] == "PreToolUse")
  end,

  edits_are_sorted_by_event_name = function()
    local r = hookfile.edits({}, {
      SessionStart = { group("a" .. MARKER) },
      PostToolUse = { group("b" .. MARKER) },
      PreToolUse = { group("c" .. MARKER) },
    }, MARKER)
    local names = {}
    for _, e in ipairs(r.edits) do
      names[#names + 1] = e[1]
    end
    assert(table.concat(names, ",") == "PostToolUse,PreToolUse,SessionStart")
  end,

  removed_tool_only_removes_marked_handlers = function()
    local r = hookfile.edits(decode(OLD_HOOKFILE), {}, MARKER)
    assert(#r.edits == 1)
    local e = r.edits[1]
    assert(e[1] == "SessionEnd")
    assert(#e[2] == 1)
    assert(e[2][1].hooks[1].command == "echo user-hook")
  end,

  no_hooks_and_no_groups_makes_no_edits = function()
    local r = hookfile.edits(decode('{"permissions":{}}'), {}, MARKER)
    assert(#r.edits == 0 and next(r.skipped) == nil)
  end,

  should_write_false_only_without_old_and_edits = function()
    assert(hookfile.should_write(nil, {}) == false)
    assert(hookfile.should_write(nil, { { "Stop", nil } }) == true)
    assert(hookfile.should_write("{}", {}) == true)
    assert(hookfile.should_write("", {}) == true)
  end,

  user_handler_ending_with_marker_is_treated_as_ours = function()
    local old = decode([[{"hooks":{"Stop":[
      {"hooks":[{"type":"command","command":"my-own-script # agentstacks"}]}
    ]}}]])
    local r = hookfile.edits(old, {}, MARKER)
    local e = find(r, "Stop")
    assert(e ~= nil and e[2] == nil)
  end,

  skip_reason_names_the_event = function()
    assert(
      hookfile.skip_reason("Stop")
        == ".muse/hooks.json: hooks.Stop holds entries the musecode extension cannot rewrite"
          .. " (a null, an empty list, or a value that is not a list of hook groups)"
    )
  end,

  user_group_kept_before_ours = function()
    local old = decode([[{"hooks":{"SessionStart":[
      {"matcher":"startup","hooks":[{"type":"command","command":"user-start"}]}
    ]}}]])
    local ours = group("ours" .. MARKER)
    local r = hookfile.edits(old, { SessionStart = { ours } }, MARKER)
    local e = find(r, "SessionStart")
    assert(e ~= nil)
    assert(#e[2] == 2)
    assert(e[2][1].matcher == "startup")
    assert(e[2][1].hooks[1].command == "user-start")
    assert(e[2][2] == ours)
    assert(next(r.skipped) == nil)
  end,

  removed_tool_strips_only_marked = function()
    local mixed = decode([[{"hooks":{
      "SessionStart":[
        {"hooks":[
          {"type":"command","command":"keep"},
          {"type":"command","command":"gone # agentstacks"}
        ]}
      ],
      "Stop":[{"hooks":[{"type":"command","command":"only # agentstacks"}]}]
    }}]])
    local r = hookfile.edits(mixed, {}, MARKER)
    assert(#r.edits == 2)
    local start = find(r, "SessionStart")
    assert(start ~= nil and #start[2] == 1)
    assert(#start[2][1].hooks == 1)
    assert(start[2][1].hooks[1].command == "keep")
    local stop = find(r, "Stop")
    assert(stop ~= nil and stop[2] == nil)
    assert(next(r.skipped) == nil)
  end,
}
