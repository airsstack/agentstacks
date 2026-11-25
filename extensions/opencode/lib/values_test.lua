local values = require("values")

local function payload(vars)
  return {
    api = 1,
    tool = "opencode",
    packs = { { name = "core", version = "0.3.0", skills = { "status" }, agents = { "coder" } } },
    vars = vars or {},
  }
end

return {
  root_is_opencode_agentstacks = function()
    assert(values.result(payload()).root == ".opencode/agentstacks")
  end,

  skill_invoke_and_dir = function()
    local skill = values.result(payload()).skills["core/status"]
    assert(skill.invoke == "/core-status", skill.invoke)
    assert(skill.dir == ".opencode/skills/core-status", skill.dir)
  end,

  agent_name_is_pack_dash_agent = function()
    assert(values.result(payload()).agents["core/coder"] == "core-coder")
  end,

  each_var_has_its_form = function()
    local expected = {
      project_dir = "${AGENTSTACKS_PROJECT_DIR}",
      session_id = "${OPENCODE_SESSION_ID}",
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

  var_not_in_payload_is_absent = function()
    local vars = values.result(payload({ "tmpdir" })).vars
    assert(vars.tmpdir == "${TMPDIR:-/tmp}")
    local count = 0
    for _ in pairs(vars) do
      count = count + 1
    end
    assert(count == 1, "expected exactly one var, got " .. count)
  end,
}
