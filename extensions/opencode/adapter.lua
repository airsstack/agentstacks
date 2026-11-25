-- OpenCode extension: renders packs into native `.opencode/` project config.
-- This is the only file that calls `agentstacks.*`; the rules live in `lib/`.

local values = agentstacks.require("lib.values")
local frontmatter = agentstacks.require("lib.frontmatter")
local hooks = agentstacks.require("lib.hooks")
local plugin = agentstacks.require("lib.plugin")
local translate = agentstacks.require("lib.translate")

local SHIM = ".opencode/plugins/agentstacks.js"

-- The `values` result of this run; the `plan` handler needs it for the skill and agent names and
-- the var forms of hook arguments.
-- The host always dispatches `values` before `plan` in one session, so this is set when `plan` runs.
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

-- The name OpenCode gives a skill and its command: its `invoke` without the leading `/`.
local function skill_name(pack, skill)
  return run_values.skills[pack.name .. "/" .. skill.name].invoke:sub(2)
end

-- Writes the skill, its command and its other files. A skill that cannot be written is skipped
-- whole, so it gets no command either.
local function propose_skill(pack, skill)
  local main, others = split_skill_files(skill)
  local name = skill_name(pack, skill)
  local content, reason = frontmatter.skill(name, skill.meta, main.text)
  if content == nil then
    agentstacks.skip({ item = skill.id, reason = reason })
    return
  end
  agentstacks.write({ path = skill.dir .. "/SKILL.md", content = content, covers = { main.id } })
  agentstacks.write({
    path = ".opencode/commands/" .. name .. ".md",
    content = frontmatter.command(name, skill.meta),
    covers = { main.id },
  })
  for _, file in ipairs(others) do
    agentstacks.copy({ path = skill.dir .. "/" .. file.rel, file = file.id })
  end
end

local function propose_agent(pack, agent)
  local name = run_values.agents[pack.name .. "/" .. agent.name]
  local content, reason = frontmatter.agent(agent.meta, agent.text)
  if content == nil then
    agentstacks.skip({ item = agent.id, reason = reason })
    return
  end
  agentstacks.write({ path = ".opencode/agents/" .. name .. ".md", content = content, covers = { agent.id } })
end

-- Every hook of a pack, in payload order, as `{ hook =, native = }`: the neutral ones by index, then
-- the native ones.
local function pack_hooks(pack)
  local list = {}
  for _, hook in ipairs(pack.hooks or {}) do
    list[#list + 1] = { hook = hook, native = false }
  end
  for _, hook in ipairs((pack.native and pack.native.hooks) or {}) do
    list[#list + 1] = { hook = hook, native = true }
  end
  return list
end

-- Turns each hook into a plugin entry or a skip. Returns the entries in payload order, the ids of
-- the hooks that became entries, and whether one of them is a lua hook. The core reads `covers` as
-- a set; the ids are sorted only so the proposal reads the same way every time.
local function render_hooks(packs)
  local entries, covers, has_lua = {}, {}, false
  for _, pack in ipairs(packs) do
    for _, item in ipairs(pack_hooks(pack)) do
      local entry, reason = hooks.entry(item.hook, run_values.vars, item.native)
      if entry == nil then
        agentstacks.skip({ item = item.hook.id, reason = reason })
      else
        entries[#entries + 1] = entry
        covers[#covers + 1] = item.hook.id
        has_lua = has_lua or item.hook.kind == "lua"
      end
    end
  end
  table.sort(covers)
  return entries, covers, has_lua
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
      agentstacks.copy({ path = ".opencode/" .. file.rel, file = file.id })
    end
  end

  local entries, covers, has_lua = render_hooks(payload.packs)
  agentstacks.write({ path = SHIM, content = plugin.text(entries), covers = covers })
  if has_lua then
    agentstacks.write({ path = payload.root .. "/translate.lua", content = translate.text() })
  end
end)
