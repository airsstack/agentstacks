-- Builds the YAML frontmatter of a Claude Code skill or agent file.
--
-- Every value is written with `airsstack.json.encode`: a JSON string is a valid YAML
-- double-quoted scalar and a JSON list is a valid YAML flow list. Generated keys come first
-- in a fixed order; `meta.native` keys replace a generated key of the same name, and the
-- ones that were not generated follow, sorted by name.
local M = {}

local TOOLS = {
  read = "Read",
  write = "Write",
  edit = "Edit",
  grep = "Grep",
  glob = "Glob",
  shell = "Bash",
  skill = "Skill",
}

local MODELS = { fast = "haiku", balanced = "sonnet", deep = "opus" }

-- Returns `list` with each word replaced through `map`. A word the map does not know is kept,
-- since the host has already validated the words.
local function mapped(list, map)
  local out = {}
  for i, word in ipairs(list) do
    out[i] = map[word] or word
  end
  return out
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
    lines[#lines + 1] = key .. ": " .. airsstack.json.encode(value) .. "\n"
  end
  local extra = {}
  for key in pairs(native_set) do
    if not seen[key] then
      extra[#extra + 1] = key
    end
  end
  table.sort(extra)
  for _, key in ipairs(extra) do
    lines[#lines + 1] = key .. ": " .. airsstack.json.encode(native_set[key]) .. "\n"
  end
  lines[#lines + 1] = "---\n"
  lines[#lines + 1] = text
  return table.concat(lines)
end

local function check_native(meta)
  if meta.native then
    local key = bad_native_key(meta.native)
    if key then
      return 'native.claude key "' .. tostring(key) .. '" cannot be written as frontmatter'
    end
  end
  return nil
end

-- Returns the `SKILL.md` content for a skill, or `nil, reason` when a native key cannot be
-- written as frontmatter.
function M.skill(name, meta, text)
  local reason = check_native(meta)
  if reason then
    return nil, reason
  end
  local generated = { { "name", name }, { "description", meta.description } }
  if meta.invocation == "user" then
    generated[#generated + 1] = { "disable-model-invocation", true }
  end
  if meta.argument_hint ~= nil then
    generated[#generated + 1] = { "argument-hint", meta.argument_hint }
  end
  return render(generated, meta.native, text)
end

-- Returns the agent file content for an agent, or `nil, reason` when a native key cannot be
-- written as frontmatter.
function M.agent(name, meta, text)
  local reason = check_native(meta)
  if reason then
    return nil, reason
  end
  local generated = { { "name", name }, { "description", meta.description } }
  if meta.tools ~= nil then
    generated[#generated + 1] = { "tools", mapped(meta.tools, TOOLS) }
  end
  if meta.model ~= nil then
    generated[#generated + 1] = { "model", MODELS[meta.model] or meta.model }
  end
  if meta.effort ~= nil then
    generated[#generated + 1] = { "effort", meta.effort }
  end
  return render(generated, meta.native, text)
end

return M
