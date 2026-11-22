-- Merge rules for the hooks section of `.claude/settings.json`: which events the extension
-- rewrites, what each one becomes, and which are left alone because a rewrite would lose data.
-- Pure: decoded values in, plain tables out. The caller applies the edits.
local M = {}

local function is_blank(text)
  return text == nil or text:match("^%s*$") ~= nil
end

-- A decoded empty JSON array is an empty table carrying a metatable; an empty object has none.
local function is_empty_list(v)
  return type(v) == "table" and next(v) == nil and getmetatable(v) ~= nil
end

-- A list is a table whose keys are exactly 1..n, or a decoded empty JSON array.
local function is_list(v)
  if type(v) ~= "table" then
    return false
  end
  if next(v) == nil then
    return is_empty_list(v)
  end
  local count = 0
  for _ in pairs(v) do
    count = count + 1
  end
  return count == #v
end

-- True when a JSON null (decoded to userdata) or an empty list appears anywhere inside `v`.
-- Neither survives the edit call: a null cannot be written back and an empty list becomes `{}`.
local function holds_lossy_value(v)
  if type(v) == "userdata" or is_empty_list(v) then
    return true
  end
  if type(v) == "table" then
    for _, child in pairs(v) do
      if holds_lossy_value(child) then
        return true
      end
    end
  end
  return false
end

local function all_tables(list)
  for _, item in ipairs(list) do
    if type(item) ~= "table" then
      return false
    end
  end
  return true
end

-- The array can be rewritten whole: a list of groups, each holding a list of handlers with a
-- string command, and nothing in it that the edit call would drop or change.
local function is_rewritable(groups)
  if not is_list(groups) or not all_tables(groups) or holds_lossy_value(groups) then
    return false
  end
  for _, group in ipairs(groups) do
    if not is_list(group.hooks) or not all_tables(group.hooks) then
      return false
    end
    for _, handler in ipairs(group.hooks) do
      if type(handler.command) ~= "string" then
        return false
      end
    end
  end
  return true
end

local function ends_with(text, suffix)
  return #text >= #suffix and text:sub(-#suffix) == suffix
end

local function is_marked(handler, marker)
  return type(handler) == "table"
    and type(handler.command) == "string"
    and ends_with(handler.command, marker)
end

-- Reads a possibly malformed array without raising: true when some handler in it is marked.
local function holds_marked_handler(groups, marker)
  if type(groups) ~= "table" then
    return false
  end
  for _, group in ipairs(groups) do
    if type(group) == "table" and type(group.hooks) == "table" then
      for _, handler in ipairs(group.hooks) do
        if is_marked(handler, marker) then
          return true
        end
      end
    end
  end
  return false
end

-- The old groups minus marked handlers, with groups left empty dropped, then `ours` appended.
local function merged(groups, ours, marker)
  local out = {}
  for _, group in ipairs(groups or {}) do
    local kept = {}
    for _, handler in ipairs(group.hooks) do
      if not is_marked(handler, marker) then
        kept[#kept + 1] = handler
      end
    end
    if #kept > 0 then
      local copy = {}
      for k, v in pairs(group) do
        copy[k] = v
      end
      copy.hooks = kept
      out[#out + 1] = copy
    end
  end
  for _, group in ipairs(ours or {}) do
    out[#out + 1] = group
  end
  if #out == 0 then
    return nil
  end
  return out
end

-- Decodes the old settings text. `nil`, empty and whitespace-only text are an empty table, because
-- decoding empty text fails.
function M.decode(old)
  if is_blank(old) then
    return {}
  end
  return airsstack.json.decode(old)
end

-- `decoded` is the decoded old settings, `groups_by_event` maps an event to our groups for it, and
-- `marker` is the suffix that tags our handlers.
-- Returns `{ edits = { { event, value_or_nil }, ... }, skipped = { [event] = true } }`.
-- `edits` is sorted by event name; a `nil` value removes the event. A touched event that cannot be
-- rewritten is in `skipped` and has no edit.
function M.edits(decoded, groups_by_event, marker)
  local old_hooks = type(decoded) == "table" and decoded.hooks or nil
  if type(old_hooks) ~= "table" then
    old_hooks = {}
  end

  local touched = {}
  for event, groups in pairs(old_hooks) do
    if type(event) == "string" and holds_marked_handler(groups, marker) then
      touched[event] = true
    end
  end
  for event in pairs(groups_by_event) do
    touched[event] = true
  end

  local names = {}
  for event in pairs(touched) do
    names[#names + 1] = event
  end
  table.sort(names)

  local result = { edits = {}, skipped = {} }
  for _, event in ipairs(names) do
    local old_groups = old_hooks[event]
    if old_groups ~= nil and not is_rewritable(old_groups) then
      result.skipped[event] = true
    else
      result.edits[#result.edits + 1] = { event, merged(old_groups, groups_by_event[event], marker) }
    end
  end
  return result
end

-- The settings file is written when it already exists or when there is at least one edit.
function M.should_write(old, edits)
  return old ~= nil or #edits > 0
end

-- The skip reason for a hook whose event cannot be rewritten.
function M.skip_reason(event)
  return ".claude/settings.json: hooks."
    .. event
    .. " holds entries the claude extension cannot rewrite"
    .. " (a null, an empty list, or a value that is not a list of hook groups)"
end

return M
