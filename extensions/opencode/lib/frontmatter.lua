-- Builds the YAML frontmatter of an OpenCode skill, agent or command file.
--
-- Every value is written with `airsstack.json.encode`: a JSON string is a valid YAML
-- double-quoted scalar and a JSON list is a valid YAML flow list. A value that is an empty
-- table is written `[]` instead, since `encode` writes it as `{}` and the list form is the one
-- OpenCode reads. Generated keys come first in a fixed order; `meta.native` keys replace a
-- generated key of the same name, and the ones that were not generated follow, sorted by name.
local M = {}

-- The OpenCode permission action each tool word maps to.
local ACTIONS = {
  read = "read",
  write = "edit",
  edit = "edit",
  grep = "grep",
  glob = "glob",
  shell = "shell",
  skill = "skill",
}

local DENY_ALL = { action = "*", effect = "deny", resource = "*" }

-- Returns `value` as the text written after `key: `.
local function written(value)
  if type(value) == "table" and next(value) == nil then
    return "[]"
  end
  return airsstack.json.encode(value)
end

-- Returns the permission rules for `tools`: the deny-all rule, then one allow rule per mapped
-- action in written order, a repeated action dropped. An absent `tools` gives an empty list.
local function permissions(tools)
  if tools == nil then
    return {}
  end
  local rules = { DENY_ALL }
  local seen = {}
  for _, word in ipairs(tools) do
    local action = ACTIONS[word] or word
    if not seen[action] then
      seen[action] = true
      rules[#rules + 1] = { action = action, effect = "allow", resource = "*" }
    end
  end
  return rules
end

-- Returns the first native key that cannot be written as a frontmatter key, in sorted order.
local function bad_native_key(native)
  local names = {}
  for key in pairs(native) do
    names[#names + 1] = key
  end
  table.sort(names)
  for _, key in ipairs(names) do
    if not tostring(key):match("^[A-Za-z0-9_%-]+$") then
      return key
    end
  end
  return nil
end

-- Renders `generated` (an ordered list of `{ key, value }`) and the native keys as one
-- frontmatter block followed by `text`.
local function render(generated, native, text)
  local native_set = native or {}
  local lines = { "---\n" }
  local seen = {}
  for _, pair in ipairs(generated) do
    local key, value = pair[1], pair[2]
    if native_set[key] ~= nil then
      value = native_set[key]
    end
    seen[key] = true
    lines[#lines + 1] = key .. ": " .. written(value) .. "\n"
  end
  local extra = {}
  for key in pairs(native_set) do
    if not seen[key] then
      extra[#extra + 1] = key
    end
  end
  table.sort(extra)
  for _, key in ipairs(extra) do
    lines[#lines + 1] = key .. ": " .. written(native_set[key]) .. "\n"
  end
  lines[#lines + 1] = "---\n"
  lines[#lines + 1] = text
  return table.concat(lines)
end

local function check_native(meta)
  if meta.native then
    local key = bad_native_key(meta.native)
    if key then
      return 'native.opencode key "' .. tostring(key) .. '" cannot be written as frontmatter'
    end
  end
  return nil
end

-- Returns the `SKILL.md` content for a skill, or `nil, reason` when a native key cannot be
-- written as frontmatter. `argument_hint` is not written: OpenCode skills have no such field.
function M.skill(name, meta, text)
  local reason = check_native(meta)
  if reason then
    return nil, reason
  end
  local generated = { { "name", name }, { "description", meta.description } }
  if meta.invocation == "user" then
    generated[#generated + 1] = { "disable-model-invocation", true }
  end
  return render(generated, meta.native, text)
end

-- Returns the agent file content for an agent, or `nil, reason` when a native key cannot be
-- written as frontmatter. The agent's name is its file name, so no `name` is written; `model`
-- and `effort` are not written either, and `permissions` is always written.
function M.agent(meta, text)
  local reason = check_native(meta)
  if reason then
    return nil, reason
  end
  local generated = {
    { "description", meta.description },
    { "mode", "subagent" },
    { "permissions", permissions(meta.tools) },
  }
  return render(generated, meta.native, text)
end

-- Returns the command file content for the skill `id` (`<pack>-<skill>`): a `description`
-- frontmatter, then a body that loads the skill and passes `$ARGUMENTS` for `<arguments>`.
function M.command(id, meta)
  local body = "Load the skill `" .. id .. "` with the skill tool and follow it.\n"
    .. "Wherever it says `<arguments>`, use: $ARGUMENTS\n"
  return render({ { "description", meta.description } }, nil, body)
end

return M
