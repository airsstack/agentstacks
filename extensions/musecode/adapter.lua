-- Muse Code extension: renders packs into native `.agents/` project config and the shared
-- `.muse/hooks.json`, and writes the translator Muse Code's hooks run through.
-- This is the only file that calls `agentstacks.*`; the rules live in `lib/`.

local values = agentstacks.require("lib.values")
local frontmatter = agentstacks.require("lib.frontmatter")
local hooks = agentstacks.require("lib.hooks")
local hookfile = agentstacks.require("lib.hookfile")
local translate = agentstacks.require("lib.translate")

local HOOKS_FILE = ".muse/hooks.json"

-- The `values` result of this run; the `plan` handler needs it for the skill and agent names and
-- the var forms of hook arguments.
-- The host always dispatches `values` before `plan` in one session, so this is set when `plan`
-- runs.
local run_values

agentstacks.on("values", function(payload)
  run_values = values.result(payload)
  return run_values
end)

-- The `SKILL.md` file of a skill, and the other files.
local function split_skill_files(skill)
  local main, others = nil, {}
  for _, file in ipairs(skill.files) do
    if file.rel == "SKILL.md" then
      main = file
    else
      others[#others + 1] = file
    end
  end
  return main, others
end

-- The name Muse Code gives a skill: its `invoke` without the leading `/`.
local function skill_name(pack, skill)
  return run_values.skills[pack.name .. "/" .. skill.name].invoke:sub(2)
end

-- Writes the skill and copies its other files. A skill that cannot be written is skipped whole.
local function propose_skill(pack, skill)
  local main, others = split_skill_files(skill)
  local content, reason = frontmatter.skill(skill_name(pack, skill), skill.meta, main.text)
  if content == nil then
    agentstacks.skip({ item = skill.id, reason = reason })
    return
  end
  agentstacks.write({ path = skill.dir .. "/SKILL.md", content = content, covers = { main.id } })
  for _, file in ipairs(others) do
    agentstacks.copy({ path = skill.dir .. "/" .. file.rel, file = file.id })
  end
end

local function propose_agent(pack, agent)
  local name = run_values.agents[pack.name .. "/" .. agent.name]
  local content, reason = frontmatter.agent(name, agent.meta, agent.text)
  if content == nil then
    agentstacks.skip({ item = agent.id, reason = reason })
    return
  end
  agentstacks.write({
    path = ".agents/agents/" .. name .. ".md",
    content = content,
    covers = { agent.id },
  })
end

-- Every hook of a pack, in payload order: the neutral ones by index, then the native ones.
local function pack_hooks(pack)
  local list = {}
  for _, hook in ipairs(pack.hooks or {}) do
    list[#list + 1] = hook
  end
  for _, hook in ipairs((pack.native and pack.native.hooks) or {}) do
    list[#list + 1] = hook
  end
  return list
end

-- Renders each hook into its Muse Code event group. A hook Muse Code cannot express is skipped.
-- Returns `{ [event] = groups }` and `{ [event] = { { id =, kind = }, ... } }`.
local function render_hooks(packs)
  local groups, members = {}, {}
  for _, pack in ipairs(packs) do
    for _, hook in ipairs(pack_hooks(pack)) do
      local agents = values.hook_agents(run_values, pack.name, hook)
      local rendered, reason = hooks.render(hook, run_values.vars, agents)
      if rendered == nil then
        agentstacks.skip({ item = hook.id, reason = reason })
      else
        local event = rendered.event
        groups[event] = groups[event] or {}
        members[event] = members[event] or {}
        groups[event][#groups[event] + 1] = rendered.group
        members[event][#members[event] + 1] = { id = hook.id, kind = hook.kind }
      end
    end
  end
  return groups, members
end

-- Merges our hook groups into `.muse/hooks.json`. Returns whether a lua hook is among the hooks it
-- wrote.
local function propose_hooks_file(groups, members)
  local old = agentstacks.read(HOOKS_FILE)
  local text = old or "{}\n"
  if old ~= nil then
    -- Refuses a file that is not a valid hooks file: not JSON, not an object, or `hooks` not an
    -- object.
    agentstacks.json.edit(old, { "hooks", "agentstacks-check" }, nil)
  end
  local merged = hookfile.edits(hookfile.decode(old), groups, hooks.MARKER)

  for event in pairs(merged.skipped) do
    for _, member in ipairs(members[event] or {}) do
      agentstacks.skip({ item = member.id, reason = hookfile.skip_reason(event) })
    end
  end
  for _, edit in ipairs(merged.edits) do
    text = agentstacks.json.edit(text, { "hooks", edit[1] }, edit[2])
  end

  local covers, has_lua = {}, false
  for event, list in pairs(members) do
    if not merged.skipped[event] then
      for _, member in ipairs(list) do
        covers[#covers + 1] = member.id
        has_lua = has_lua or member.kind == "lua"
      end
    end
  end
  table.sort(covers)

  if hookfile.should_write(old, merged.edits) then
    agentstacks.write({ path = HOOKS_FILE, content = text, covers = covers })
  end
  return has_lua
end

agentstacks.on("plan", function(payload)
  for _, pack in ipairs(payload.packs) do
    for _, skill in ipairs(pack.skills or {}) do
      propose_skill(pack, skill)
    end
    for _, agent in ipairs(pack.agents or {}) do
      propose_agent(pack, agent)
    end
    for _, file in ipairs((pack.native and pack.native.files) or {}) do
      agentstacks.copy({ path = ".agents/" .. file.rel, file = file.id })
    end
  end

  local groups, members = render_hooks(payload.packs)
  if propose_hooks_file(groups, members) then
    agentstacks.write({ path = payload.root .. "/translate.lua", content = translate.text() })
  end
end)
