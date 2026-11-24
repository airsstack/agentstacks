local values = require("values")

local function payload(vars)
  return {
    api = 0,
    tool = "claude",
    packs = { { name = "core", version = "0.3.0", skills = { "status" }, agents = { "coder" } } },
    vars = vars or {},
  }
end

return {
  root_is_dot_claude_agentstacks = function()
    assert(values.result(payload()).root == ".claude/agentstacks")
  end,

  skill_maps_to_pack_prefixed_name_and_dir = function()
    local p = payload()
    p.packs[1].name = "sdlc"
    local skill = values.result(p).skills["sdlc/status"]
    assert(skill.invoke == "/sdlc-status")
    assert(skill.dir == ".claude/skills/sdlc-status")
  end,

  agent_maps_to_pack_prefixed_name = function()
    assert(values.result(payload()).agents["core/coder"] == "core-coder")
  end,

  hyphenated_pack_is_prefixed_whole = function()
    local p = payload()
    p.packs[1].name = "guideline-python"
    p.packs[1].skills = { "guidelines" }
    local skill = values.result(p).skills["guideline-python/guidelines"]
    assert(skill.invoke == "/guideline-python-guidelines")
    assert(skill.dir == ".claude/skills/guideline-python-guidelines")
  end,

  only_requested_vars_are_returned = function()
    local vars = values.result(payload({ "tmpdir" })).vars
    assert(vars.tmpdir == "${TMPDIR:-/tmp}")
    local count = 0
    for _ in pairs(vars) do
      count = count + 1
    end
    assert(count == 1, "expected exactly one var, got " .. count)
  end,

  every_var_has_its_value = function()
    local expected = {
      session_id = "${CLAUDE_SESSION_ID}",
      arguments = "$ARGUMENTS",
      project_dir = "${CLAUDE_PROJECT_DIR}",
      tmpdir = "${TMPDIR:-/tmp}",
      home = "$HOME",
      airsstack_home = "${AIRSSTACK_HOME:-$HOME/.airsstack}",
    }
    for name, value in pairs(expected) do
      local vars = values.result(payload({ name })).vars
      assert(vars[name] == value, name .. ": got " .. tostring(vars[name]))
    end
  end,
  hook_agents_renders_each_name_in_order = function()
    local p = payload()
    p.packs[1].agents = { "coder", "reviewer" }
    local result = values.result(p)
    local names = values.hook_agents(result, "core", { agents = { "reviewer", "coder" } })
    assert(#names == 2, "expected two names, got " .. #names)
    assert(names[1] == "core-reviewer", names[1])
    assert(names[2] == "core-coder", names[2])
  end,

  hook_agents_prefixes_a_hyphenated_pack_whole = function()
    local p = payload()
    p.packs[1].name = "guideline-rust"
    local result = values.result(p)
    local names = values.hook_agents(result, "guideline-rust", { agents = { "coder" } })
    assert(names[1] == "guideline-rust-coder", names[1])
  end,

  hook_agents_is_nil_for_a_hook_without_agents = function()
    local result = values.result(payload())
    assert(values.hook_agents(result, "core", {}) == nil)
  end,

  hook_agents_is_empty_for_an_empty_list = function()
    local result = values.result(payload())
    local names = values.hook_agents(result, "core", { agents = {} })
    assert(names ~= nil and #names == 0, "expected an empty list")
  end,
}
