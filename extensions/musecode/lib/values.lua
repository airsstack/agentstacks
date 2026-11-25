-- Where Muse Code puts packs and what each `{{var}}` stands for.
--
-- Pure: reads the `values` payload and returns the result table. It calls no
-- `agentstacks.*` and no `require`; `adapter.lua` hands it in.

local M = {}

-- The text each var renders to. The shell forms are read by the model in skill
-- and agent text and expanded by `sh` in hook commands.
local VARS = {
  project_dir = ".",
  session_id = "${MUSE_SESSION_ID}",
  arguments = "<arguments>",
  tmpdir = "${TMPDIR:-/tmp}",
  home = "$HOME",
  airsstack_home = "${AIRSSTACK_HOME:-$HOME/.airsstack}",
}

-- The `values` result for `payload`: the pack root, the invocation and
-- directory of every skill, the name of every agent (skills and agents are
-- named `<pack>-<item>`), and the values of the vars the payload names. Names
-- this table has no value for are left out.
function M.result(payload)
  local out = { root = ".muse/agentstacks", skills = {}, agents = {}, vars = {} }
  for _, pack in ipairs(payload.packs or {}) do
    for _, skill in ipairs(pack.skills or {}) do
      out.skills[pack.name .. "/" .. skill] = {
        invoke = "/" .. pack.name .. "-" .. skill,
        dir = ".agents/skills/" .. pack.name .. "-" .. skill,
      }
    end
    for _, agent in ipairs(pack.agents or {}) do
      out.agents[pack.name .. "/" .. agent] = pack.name .. "-" .. agent
    end
  end
  for _, name in ipairs(payload.vars or {}) do
    if VARS[name] ~= nil then
      out.vars[name] = VARS[name]
    end
  end
  return out
end

-- The rendered names of the agents `hook` lists, in the hook's order, looked up
-- in `result` (a `result` return) under pack `pack_name`. Nil when the hook
-- lists no agents.
function M.hook_agents(result, pack_name, hook)
  if hook.agents == nil then
    return nil
  end
  local names = {}
  for i, agent in ipairs(hook.agents) do
    names[i] = result.agents[pack_name .. "/" .. agent]
  end
  return names
end

return M
