local frontmatter = require("frontmatter")

local function eq(got, want)
  assert(got == want, "got:\n" .. tostring(got) .. "\nwant:\n" .. tostring(want))
end

-- Returns the line of `text` that starts with `prefix`.
local function line_of(text, prefix)
  for line in text:gmatch("[^\n]+") do
    if line:sub(1, #prefix) == prefix then
      return line
    end
  end
  return nil
end

local DENY = '{"action":"*","effect":"deny","resource":"*"}'

local function allow(action)
  return '{"action":"' .. action .. '","effect":"allow","resource":"*"}'
end

return {
  skill_any_writes_name_and_description = function()
    local out = frontmatter.skill("core-discuss", { description = "Talk it through", invocation = "any" }, "\nbody\n")
    eq(out, '---\nname: "core-discuss"\ndescription: "Talk it through"\n---\n\nbody\n')
  end,

  skill_user_writes_disable_model_invocation = function()
    local out = frontmatter.skill("s", { description = "d", invocation = "user" }, "")
    eq(out, '---\nname: "s"\ndescription: "d"\ndisable-model-invocation: true\n---\n')
  end,

  skill_drops_argument_hint = function()
    local out = frontmatter.skill("s", { description = "d", argument_hint = "[chain]" }, "")
    eq(out, '---\nname: "s"\ndescription: "d"\n---\n')
    assert(not out:find("argument-hint", 1, true), "argument-hint must not be written")
  end,

  agent_without_tools_has_empty_permissions = function()
    local out = frontmatter.agent({ description = "d" }, "\nbody\n")
    eq(out, '---\ndescription: "d"\nmode: "subagent"\npermissions: []\n---\n\nbody\n')
    eq(line_of(out, "permissions:"), "permissions: []")
  end,

  agent_with_empty_tools_denies_all = function()
    local out = frontmatter.agent({ description = "d", tools = {} }, "")
    eq(line_of(out, "permissions:"), "permissions: [" .. DENY .. "]")
  end,

  agent_tools_map_to_permissions = function()
    local out = frontmatter.agent({ description = "d", tools = { "read", "write", "edit", "grep" } }, "")
    eq(
      line_of(out, "permissions:"),
      'permissions: [{"action":"*","effect":"deny","resource":"*"},{"action":"read","effect":"allow","resource":"*"},{"action":"edit","effect":"allow","resource":"*"},{"action":"grep","effect":"allow","resource":"*"}]'
    )
  end,

  agent_every_tool_word_maps = function()
    local out = frontmatter.agent({
      description = "d",
      tools = { "read", "write", "edit", "grep", "glob", "shell", "skill" },
    }, "")
    local rules = { DENY, allow("read"), allow("edit"), allow("grep"), allow("glob"), allow("shell"), allow("skill") }
    eq(line_of(out, "permissions:"), "permissions: [" .. table.concat(rules, ",") .. "]")
  end,

  agent_writes_no_model_or_effort = function()
    local out = frontmatter.agent({ description = "d", model = "deep", effort = "high" }, "")
    eq(out, '---\ndescription: "d"\nmode: "subagent"\npermissions: []\n---\n')
  end,

  native_key_replaces_generated = function()
    local out = frontmatter.skill("s", { description = "d", native = { description = "over" } }, "")
    eq(out, '---\nname: "s"\ndescription: "over"\n---\n')
    local agent = frontmatter.agent({ description = "d", native = { description = "over", color = "blue" } }, "")
    eq(agent, '---\ndescription: "over"\nmode: "subagent"\npermissions: []\ncolor: "blue"\n---\n')
  end,

  empty_native_table_is_written_as_a_list = function()
    local agent = frontmatter.agent({ description = "d", tools = { "read" }, native = { permissions = {} } }, "")
    eq(line_of(agent, "permissions:"), "permissions: []")
    local skill = frontmatter.skill("s", { description = "d", native = { allowed = {} } }, "")
    eq(skill, '---\nname: "s"\ndescription: "d"\nallowed: []\n---\n')
  end,

  bad_native_key_gives_reason = function()
    local reason = 'native.opencode key "a b" cannot be written as frontmatter'
    local out, got = frontmatter.skill("s", { description = "d", native = { ["a b"] = "x" } }, "")
    eq(out, nil)
    eq(got, reason)
    local agent, agent_got = frontmatter.agent({ description = "d", native = { ["a b"] = "x" } }, "")
    eq(agent, nil)
    eq(agent_got, reason)
  end,

  command_text_for_core_discuss = function()
    local out = frontmatter.command("core-discuss", { description = 'Talk about "it"' })
    eq(
      out,
      '---\ndescription: "Talk about \\"it\\""\n---\n'
        .. "Load the skill `core-discuss` with the skill tool and follow it.\n"
        .. "Wherever it says `<arguments>`, use: $ARGUMENTS\n"
    )
  end,
}
