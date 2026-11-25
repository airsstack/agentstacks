-- Builds the YAML frontmatter of a Muse Code skill or agent file.
--
-- Every value is written with `airsstack.json.encode`: a JSON string is a valid YAML
-- double-quoted scalar and a JSON list is a valid YAML flow list. Generated keys come first
-- in a fixed order; `meta.native` keys replace a generated key of the same name, and the
-- ones that were not generated follow, sorted by name. Each function returns the file text,
-- or `nil, reason` when a native key cannot be written as frontmatter or Muse Code would drop
-- or refuse the file.
local M = {}

local MAX_SKILL_NAME = 128
local MAX_DESCRIPTION = 4096
local MAX_SKILL_FILE = 262144

-- The Muse Code tool name of each neutral tool word.
local TOOLS = {
  read = "read_file",
  edit = "edit_file",
  write = "write_file",
  grep = "search",
  glob = "search",
  shell = "bash",
  skill = "read_skill",
}

-- Marks an empty tool list, which `airsstack.json.encode` would write as `{}`.
local EMPTY_LIST = {}

-- Returns the Muse Code tool names for `list`, in the list's order, each name once. A word the
-- map does not know is kept, since the host has already validated the words.
local function mapped(list)
  if #list == 0 then
    return EMPTY_LIST
  end
  local out, written = {}, {}
  for _, word in ipairs(list) do
    local tool = TOOLS[word] or word
    if not written[tool] then
      written[tool] = true
      out[#out + 1] = tool
    end
  end
  return out
end

-- Returns whether `name` is lowercase letter words joined by single `-` (`^[a-z]+(-[a-z]+)*$`;
-- a Lua pattern cannot repeat a group).
local function is_word_name(name)
  for word in (name .. "-"):gmatch("(.-)%-") do
    if not word:match("^[a-z]+$") then
      return false
    end
  end
  return true
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

local function write_value(value)
  if value == EMPTY_LIST then
    return "[]"
  end
  return airsstack.json.encode(value)
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
    lines[#lines + 1] = key .. ": " .. write_value(value) .. "\n"
  end
  local extra = {}
  for key in pairs(native_set) do
    if not seen[key] then
      extra[#extra + 1] = key
    end
  end
  table.sort(extra)
  for _, key in ipairs(extra) do
    lines[#lines + 1] = key .. ": " .. write_value(native_set[key]) .. "\n"
  end
  lines[#lines + 1] = "---\n"
  lines[#lines + 1] = text
  return table.concat(lines)
end

local function check_native(meta)
  if meta.native then
    local key = bad_native_key(meta.native)
    if key then
      return 'native.musecode key "' .. tostring(key) .. '" cannot be written as frontmatter'
    end
  end
  return nil
end

-- Returns the value written for `key`: the native one when present, else `generated`.
local function written(meta, key, generated)
  if meta.native and meta.native[key] ~= nil then
    return meta.native[key]
  end
  return generated
end

-- Returns the `SKILL.md` content for a skill, or `nil, reason` when a native key cannot be
-- written as frontmatter, the name is over 128 characters, or the file is over 262144 bytes.
function M.skill(name, meta, text)
  local reason = check_native(meta)
  if reason then
    return nil, reason
  end
  local shown = written(meta, "name", name)
  if type(shown) == "string" then
    local length = utf8.len(shown) or #shown
    if length > MAX_SKILL_NAME then
      return nil, "musecode skill names are at most " .. MAX_SKILL_NAME .. " characters (this one has " .. length .. ")"
    end
  end
  local generated = { { "name", name }, { "description", meta.description } }
  if meta.invocation == "user" then
    generated[#generated + 1] = { "disable-model-invocation", true }
  end
  if meta.argument_hint ~= nil then
    generated[#generated + 1] = { "argument-hint", meta.argument_hint }
  end
  local out = render(generated, meta.native, text)
  if #out > MAX_SKILL_FILE then
    return nil, "musecode refuses a SKILL.md over " .. MAX_SKILL_FILE .. " bytes (this one has " .. #out .. ")"
  end
  return out
end

-- Returns the agent file content for an agent, or `nil, reason` when a native key cannot be
-- written as frontmatter, the name is not lowercase letter words joined by `-`, or the
-- description is over 4096 bytes. `model` is never written: a neutral tier is not a Muse Code
-- model id.
function M.agent(name, meta, text)
  local reason = check_native(meta)
  if reason then
    return nil, reason
  end
  local shown = written(meta, "name", name)
  if type(shown) == "string" and not is_word_name(shown) then
    return nil, 'musecode agent names are lowercase letter words joined by - (got "' .. shown .. '")'
  end
  local description = written(meta, "description", meta.description)
  if type(description) == "string" and #description > MAX_DESCRIPTION then
    return nil,
      "musecode drops an agent whose description is over "
        .. MAX_DESCRIPTION
        .. " bytes (this one has "
        .. #description
        .. ")"
  end
  local generated = { { "name", name }, { "description", meta.description } }
  if meta.tools ~= nil then
    generated[#generated + 1] = { "tools", mapped(meta.tools) }
  end
  if meta.effort ~= nil then
    generated[#generated + 1] = { "effort", meta.effort }
  end
  return render(generated, meta.native, text)
end

return M
