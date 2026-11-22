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

  skill_maps_to_slash_name_and_dir = function()
    local skill = values.result(payload()).skills["core/status"]
    assert(skill.invoke == "/status")
    assert(skill.dir == ".claude/skills/status")
  end,

  agent_maps_to_its_name = function()
    assert(values.result(payload()).agents["core/coder"] == "coder")
  end,

  only_requested_vars_are_returned = function()
    local vars = values.result(payload({ "tmpdir" })).vars
    assert(vars.tmpdir == "$TMPDIR")
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
      tmpdir = "$TMPDIR",
      home = "$HOME",
      airsstack_home = "${AIRSSTACK_HOME:-$HOME/.airsstack}",
    }
    for name, value in pairs(expected) do
      local vars = values.result(payload({ name })).vars
      assert(vars[name] == value, name .. ": got " .. tostring(vars[name]))
    end
  end,
}
