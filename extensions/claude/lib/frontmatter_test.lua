local frontmatter = require("frontmatter")

local function eq(got, want)
  assert(got == want, "got:\n" .. tostring(got) .. "\nwant:\n" .. tostring(want))
end

return {
  skill_frontmatter_has_name_and_description_in_order = function()
    local out = frontmatter.skill("status", { description = "Show the board" }, "\nbody\n")
    eq(out, '---\nname: "status"\ndescription: "Show the board"\n---\n\nbody\n')
  end,

  user_invocation_writes_disable_model_invocation = function()
    local out = frontmatter.skill("s", { description = "d", invocation = "user" }, "")
    eq(out, '---\nname: "s"\ndescription: "d"\ndisable-model-invocation: true\n---\n')
  end,

  any_invocation_writes_nothing = function()
    local out = frontmatter.skill("s", { description = "d", invocation = "any" }, "")
    eq(out, '---\nname: "s"\ndescription: "d"\n---\n')
  end,

  argument_hint_is_renamed = function()
    local out = frontmatter.skill("s", { description = "d", argument_hint = "[chain]" }, "")
    eq(out, '---\nname: "s"\ndescription: "d"\nargument-hint: "[chain]"\n---\n')
  end,

  agent_tools_map_in_written_order = function()
    local out = frontmatter.agent("a", { description = "d", tools = { "shell", "read" } }, "")
    eq(out, '---\nname: "a"\ndescription: "d"\ntools: ["Bash","Read"]\n---\n')
  end,

  every_model_word_maps = function()
    local want = { fast = "haiku", balanced = "sonnet", deep = "opus" }
    for word, claude in pairs(want) do
      local out = frontmatter.agent("a", { description = "d", model = word }, "")
      eq(out, '---\nname: "a"\ndescription: "d"\nmodel: "' .. claude .. '"\n---\n')
    end
  end,

  every_tool_word_maps = function()
    local out = frontmatter.agent("a", {
      description = "d",
      tools = { "read", "write", "edit", "grep", "glob", "shell", "skill" },
    }, "")
    eq(
      out,
      '---\nname: "a"\ndescription: "d"\ntools: ["Read","Write","Edit","Grep","Glob","Bash","Skill"]\n---\n'
    )
  end,

  effort_is_kept = function()
    local out = frontmatter.agent("a", { description = "d", effort = "high" }, "")
    eq(out, '---\nname: "a"\ndescription: "d"\neffort: "high"\n---\n')
  end,

  absent_tools_and_model_write_nothing = function()
    local out = frontmatter.agent("a", { description = "d" }, "")
    eq(out, '---\nname: "a"\ndescription: "d"\n---\n')
  end,

  native_key_overrides_a_generated_key = function()
    local out = frontmatter.agent("a", { description = "d", native = { description = "over" } }, "")
    eq(out, '---\nname: "a"\ndescription: "over"\n---\n')
  end,

  extra_native_keys_follow_sorted = function()
    local out = frontmatter.agent(
      "a",
      { description = "d", effort = "low", native = { zeta = "z", color = "blue", ["a-b_c"] = 1 } },
      ""
    )
    eq(out, '---\nname: "a"\ndescription: "d"\neffort: "low"\na-b_c: 1\ncolor: "blue"\nzeta: "z"\n---\n')
  end,

  native_key_with_a_space_is_a_skip_reason = function()
    local out, reason = frontmatter.skill("s", { description = "d", native = { ["bad key"] = "x" } }, "")
    eq(out, nil)
    eq(reason, 'native.claude key "bad key" cannot be written as frontmatter')
  end,

  native_empty_object_and_list_encode_back = function()
    local native = airsstack.json.decode('{"empty":{},"list":[]}')
    local out = frontmatter.skill("s", { description = "d", native = native }, "")
    eq(out, '---\nname: "s"\ndescription: "d"\nempty: {}\nlist: []\n---\n')
  end,

  string_with_quote_is_json_escaped = function()
    local out = frontmatter.skill("s", { description = 'a "q"' }, "")
    eq(out, '---\nname: "s"\ndescription: "a \\"q\\""\n---\n')
  end,

  body_text_follows_the_closing_fence_unchanged = function()
    local text = "\n# Title\n\n---\nkeep: 1\n\ttabs  \n"
    local out = frontmatter.agent("a", { description = "d" }, text)
    eq(out, '---\nname: "a"\ndescription: "d"\n---\n' .. text)
  end,
}
