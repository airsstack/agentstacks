-- Where Claude Code puts packs and what each `{{var}}` stands for.
--
-- Pure: reads the `values` payload and returns the result table. It calls no
-- `agentstacks.*` and no `require`; `adapter.lua` hands it in.

local M = {}

-- The text each var renders to. The shell forms are read by the model in skill
-- and agent text and expanded by `sh` in hook commands.
local VARS = {
  session_id = "${CLAUDE_SESSION_ID}",
  arguments = "$ARGUMENTS",
  project_dir = "${CLAUDE_PROJECT_DIR}",
  tmpdir = "$TMPDIR",
  home = "$HOME",
  airsstack_home = "${AIRSSTACK_HOME:-$HOME/.airsstack}",
}

-- The `values` result for `payload`: the pack root, the invocation and
-- directory of every skill, the name of every agent, and the values of the
-- vars the payload names. Names this table has no value for are left out.
function M.result(payload)
  local out = { root = ".claude/agentstacks", skills = {}, agents = {}, vars = {} }
  for _, pack in ipairs(payload.packs or {}) do
    for _, skill in ipairs(pack.skills or {}) do
      out.skills[pack.name .. "/" .. skill] = {
        invoke = "/" .. skill,
        dir = ".claude/skills/" .. skill,
      }
    end
    for _, agent in ipairs(pack.agents or {}) do
      out.agents[pack.name .. "/" .. agent] = agent
    end
  end
  for _, name in ipairs(payload.vars or {}) do
    if VARS[name] ~= nil then
      out.vars[name] = VARS[name]
    end
  end
  return out
end

return M
