local values = require("values")

local function payload(vars)
  return {
    api = 1,
    tool = "musecode",
    packs = { { name = "core", version = "0.3.0", skills = { "status" }, agents = { "coder" } } },
    vars = vars or {},
  }
end

return {
  root_is_muse_agentstacks = function()
    assert(values.result(payload()).root == ".muse/agentstacks")
  end,

  skill_invoke_and_dir = function()
    local skill = values.result(payload()).skills["core/status"]
    assert(skill.invoke == "/core-status", skill.invoke)
    assert(skill.dir == ".agents/skills/core-status", skill.dir)
  end,

  agent_name_is_pack_dash_agent = function()
    assert(values.result(payload()).agents["core/coder"] == "core-coder")
  end,

  each_var_has_its_form = function()
    local expected = {
      project_dir = ".",
      session_id = "${MUSE_SESSION_ID}",
      arguments = "<arguments>",
      tmpdir = "${TMPDIR:-/tmp}",
      home = "$HOME",
      airsstack_home = "${AIRSSTACK_HOME:-$HOME/.airsstack}",
    }
    for name, value in pairs(expected) do
      local vars = values.result(payload({ name })).vars
      assert(vars[name] == value, name .. ": got " .. tostring(vars[name]))
    end
  end,

  only_the_six_known_vars_are_returned = function()
    local asked = { "project_dir", "session_id", "arguments", "tmpdir", "home", "airsstack_home", "unknown" }
    local vars = values.result(payload(asked)).vars
    local count = 0
    for _ in pairs(vars) do
      count = count + 1
    end
    assert(count == 6, "expected six vars, got " .. count)
    assert(vars.unknown == nil)
  end,

  hyphenated_pack_is_prefixed_whole = function()
    local p = payload()
    p.packs[1].name = "guideline-python"
    p.packs[1].skills = { "guidelines" }
    local skill = values.result(p).skills["guideline-python/guidelines"]
    assert(skill.invoke == "/guideline-python-guidelines", skill.invoke)
    assert(skill.dir == ".agents/skills/guideline-python-guidelines", skill.dir)
  end,

  var_not_in_payload_is_absent = function()
    local vars = values.result(payload({ "tmpdir" })).vars
    assert(vars.tmpdir == "${TMPDIR:-/tmp}")
    local count = 0
    for _ in pairs(vars) do
      count = count + 1
    end
    assert(count == 1, "expected exactly one var, got " .. count)
  end,

  hook_agents_renders_names = function()
    local result = values.result(payload())
    local names = values.hook_agents(result, "core", { agents = { "coder" } })
    assert(#names == 1, "expected one name, got " .. #names)
    assert(names[1] == "core-coder", names[1])
    assert(values.hook_agents(result, "core", {}) == nil)
  end,

  hook_agents_renders_each_name_in_order = function()
    local p = payload()
    p.packs[1].agents = { "coder", "reviewer" }
    local names = values.hook_agents(values.result(p), "core", { agents = { "reviewer", "coder" } })
    assert(#names == 2, "expected two names, got " .. #names)
    assert(names[1] == "core-reviewer", names[1])
    assert(names[2] == "core-coder", names[2])
  end,

  hook_agents_prefixes_a_hyphenated_pack_whole = function()
    local p = payload()
    p.packs[1].name = "guideline-rust"
    local names = values.hook_agents(values.result(p), "guideline-rust", { agents = { "coder" } })
    assert(names[1] == "guideline-rust-coder", names[1])
  end,

  hook_agents_is_empty_for_an_empty_list = function()
    local names = values.hook_agents(values.result(payload()), "core", { agents = {} })
    assert(names ~= nil and #names == 0, "expected an empty list")
  end,
}
