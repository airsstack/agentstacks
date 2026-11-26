local frontmatter = require("frontmatter")

local function eq(got, want)
  assert(got == want, "got:\n" .. tostring(got) .. "\nwant:\n" .. tostring(want))
end

-- Asserts that `out, reason` is a skip with exactly `want`.
local function skipped(out, reason, want)
  eq(out, nil)
  eq(reason, want)
end

return {
  skill_any_writes_name_and_description = function()
    local out = frontmatter.skill("core-status", { description = "Show the board", invocation = "any" }, "\nbody\n")
    eq(out, '---\nname: "core-status"\ndescription: "Show the board"\n---\n\nbody\n')
  end,

  skill_user_writes_disable_model_invocation = function()
    local out = frontmatter.skill("s", { description = "d", invocation = "user" }, "")
    eq(out, '---\nname: "s"\ndescription: "d"\ndisable-model-invocation: true\n---\n')
  end,

  skill_writes_argument_hint = function()
    local out = frontmatter.skill("s", { description = "d", argument_hint = "[x]" }, "")
    eq(out, '---\nname: "s"\ndescription: "d"\nargument-hint: "[x]"\n---\n')
  end,

  agent_core_coder_exact = function()
    local out = frontmatter.agent("core-coder", {
      description = "<description>",
      tools = { "read", "edit", "write", "grep", "glob", "shell", "skill" },
      effort = "high",
    }, "<body>\n")
    eq(
      out,
      '---\nname: "core-coder"\n'
        .. 'description: "<description>"\n'
        .. 'tools: ["read_file","edit_file","write_file","search","bash","read_skill"]\n'
        .. 'effort: "high"\n---\n<body>\n'
    )
  end,

  agent_without_tools_has_no_tools_key = function()
    local out = frontmatter.agent("a", { description = "d" }, "")
    eq(out, '---\nname: "a"\ndescription: "d"\n---\n')
  end,

  agent_with_empty_tools_writes_literal_list = function()
    local out = frontmatter.agent("a", { description = "d", tools = {} }, "")
    eq(out, '---\nname: "a"\ndescription: "d"\ntools: []\n---\n')
  end,

  agent_grep_and_glob_give_one_search = function()
    local out = frontmatter.agent("a", { description = "d", tools = { "glob", "shell", "grep" } }, "")
    eq(out, '---\nname: "a"\ndescription: "d"\ntools: ["search","bash"]\n---\n')
  end,

  agent_never_writes_model = function()
    local out = frontmatter.agent("a", { description = "d", model = "deep" }, "")
    eq(out, '---\nname: "a"\ndescription: "d"\n---\n')
  end,

  native_key_replaces_generated = function()
    local out = frontmatter.agent("a", { description = "d", native = { description = "over" } }, "")
    eq(out, '---\nname: "a"\ndescription: "over"\n---\n')
  end,

  extra_native_keys_follow_sorted = function()
    local out = frontmatter.agent("a", { description = "d", native = { zeta = "z", color = "blue" } }, "")
    eq(out, '---\nname: "a"\ndescription: "d"\ncolor: "blue"\nzeta: "z"\n---\n')
  end,

  bad_native_key_skips = function()
    local out, reason = frontmatter.skill("s", { description = "d", native = { ["bad key"] = "x" } }, "")
    skipped(out, reason, 'native.musecode key "bad key" cannot be written as frontmatter')
    out, reason = frontmatter.agent("a", { description = "d", native = { ["b.c"] = "x" } }, "")
    skipped(out, reason, 'native.musecode key "b.c" cannot be written as frontmatter')
  end,

  skill_name_128_written_129_skipped = function()
    local ok = string.rep("a", 128)
    eq(frontmatter.skill(ok, { description = "d" }, "") ~= nil, true)
    local out, reason = frontmatter.skill(ok .. "a", { description = "d" }, "")
    skipped(out, reason, "musecode skill names are at most 128 characters (this one has 129)")
  end,

  skill_name_limit_applies_to_a_native_name = function()
    local long = string.rep("a", 129)
    local out, reason = frontmatter.skill("s", { description = "d", native = { name = long } }, "")
    skipped(out, reason, "musecode skill names are at most 128 characters (this one has 129)")
    eq(frontmatter.skill(long, { description = "d", native = { name = "s" } }, "") ~= nil, true)
  end,

  skill_name_counts_bytes_when_not_utf8 = function()
    local bad = string.rep("\xff", 129)
    local out, reason = frontmatter.skill(bad, { description = "d" }, "")
    skipped(out, reason, "musecode skill names are at most 128 characters (this one has 129)")
  end,

  native_empty_list_from_json_writes_brackets = function()
    local native = airsstack.json.decode('{"tools":[]}')
    local out = frontmatter.agent("a", { description = "d", native = native }, "")
    eq(out, '---\nname: "a"\ndescription: "d"\ntools: []\n---\n')
  end,

  skill_name_counts_characters_not_bytes = function()
    local wide = string.rep("\u{e9}", 128)
    eq(frontmatter.skill(wide, { description = "d" }, "") ~= nil, true)
    local out, reason = frontmatter.skill(wide .. "\u{e9}", { description = "d" }, "")
    skipped(out, reason, "musecode skill names are at most 128 characters (this one has 129)")
  end,

  agent_name_rejects_digit_underscore_capital_and_empty_words = function()
    for _, name in ipairs({ "core-coder2", "core_x", "Core-x", "", "-a", "a-", "a--b" }) do
      local out, reason = frontmatter.agent(name, { description = "d" }, "")
      skipped(out, reason, 'musecode agent names are lowercase letter words joined by - (got "' .. name .. '")')
    end
  end,

  agent_name_accepts_words_joined_by_dashes = function()
    eq(frontmatter.agent("core-coder", { description = "d" }, "") ~= nil, true)
    eq(frontmatter.agent("coder", { description = "d" }, "") ~= nil, true)
  end,

  agent_name_and_description_rules_apply_to_native_values = function()
    local out, reason = frontmatter.agent("core-coder", { description = "d", native = { name = "Bad_1" } }, "")
    skipped(out, reason, 'musecode agent names are lowercase letter words joined by - (got "Bad_1")')
    local long = string.rep("x", 4097)
    out, reason = frontmatter.agent("core-coder", { description = "d", native = { description = long } }, "")
    skipped(out, reason, "musecode drops an agent whose description is over 4096 bytes (this one has 4097)")
    -- a native name or description that is not a string is written as given and not checked
    out = frontmatter.agent("core-coder", { description = "d", native = { name = 7, description = 8 } }, "")
    eq(out, "---\nname: 7\ndescription: 8\n---\n")
  end,

  agent_description_4096_written_4097_skipped = function()
    eq(frontmatter.agent("a", { description = string.rep("x", 4096) }, "") ~= nil, true)
    local out, reason = frontmatter.agent("a", { description = string.rep("x", 4097) }, "")
    skipped(out, reason, "musecode drops an agent whose description is over 4096 bytes (this one has 4097)")
  end,

  skill_file_262144_written_262145_skipped = function()
    local meta = { description = "d" }
    local head = frontmatter.skill("s", meta, "")
    local fill = 262144 - #head
    local out = frontmatter.skill("s", meta, string.rep("x", fill))
    eq(#out, 262144)
    local over, reason = frontmatter.skill("s", meta, string.rep("x", fill + 1))
    skipped(over, reason, "musecode refuses a SKILL.md over 262144 bytes (this one has 262145)")
  end,

  bad_native_key_wins_over_later_rules = function()
    local out, reason = frontmatter.agent("Bad", { description = "d", native = { ["x y"] = 1 } }, "")
    skipped(out, reason, 'native.musecode key "x y" cannot be written as frontmatter')
  end,

  body_text_follows_the_closing_fence_unchanged = function()
    local text = "\n# Title\n\n---\nkeep: 1\n\ttabs  \n"
    local out = frontmatter.agent("a", { description = "d" }, text)
    eq(out, '---\nname: "a"\ndescription: "d"\n---\n' .. text)
  end,
}
