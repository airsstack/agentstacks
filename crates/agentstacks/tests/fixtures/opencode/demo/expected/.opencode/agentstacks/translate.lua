-- agentstacks translator for opencode, api 1.
local M = {}

local TOOLS = { read = "read", edit = "edit", write = "write" }

local function str(v)
  if type(v) == "string" then
    return v
  end
  return nil
end

local function trim(s)
  return (s:gsub("^%s+", ""):gsub("%s+$", ""))
end

-- True when `tool` is in the hook's neutral `match` list, or the hook has none.
local function allowed(match, tool)
  if type(match) ~= "table" then
    return true
  end
  for _, word in ipairs(match) do
    if word == tool then
      return true
    end
  end
  return false
end

-- The files a patch writes, in patch order: an Add File is a write, an Update File an edit (at its
-- Move to path when it has one). A Delete File leaves nothing to check.
local function patch_files(text)
  local files = {}
  for line in (text .. "\n"):gmatch("([^\n]*)\n") do
    line = line:gsub("\r$", "")
    local add = line:match("^%*%*%* Add File: (.+)$")
    local update = line:match("^%*%*%* Update File: (.+)$")
    local move = line:match("^%*%*%* Move to: (.+)$")
    if add then
      files[#files + 1] = { tool = "write", path = trim(add) }
    elseif update then
      files[#files + 1] = { tool = "edit", path = trim(update) }
    elseif move and #files > 0 and files[#files].tool == "edit" then
      files[#files].path = trim(move)
    end
  end
  return files
end

function M.event(p)
  local e = { event = p.hook, session_id = str(p.session_id), cwd = str(p.cwd) }
  if p.hook == "session.start" then
    e.source = "startup"
    return e
  elseif p.hook == "prompt.submit" then
    e.prompt = str(p.prompt)
    return e
  elseif p.hook ~= "tool.before" and p.hook ~= "tool.after" then
    return nil
  end
  if str(p.parent_id) ~= nil then
    e.agent = str(p.agent)
    e.agent_id = e.session_id
  end
  local input = type(p.input) == "table" and p.input or {}
  if p.tool == "patch" then
    local list = {}
    for _, file in ipairs(patch_files(str(input.patchText) or "")) do
      if allowed(p.match, file.tool) then
        local one = {}
        for k, v in pairs(e) do
          one[k] = v
        end
        one.tool, one.path = file.tool, file.path
        list[#list + 1] = one
      end
    end
    if #list == 0 then
      return nil
    end
    return list
  end
  local tool = TOOLS[p.tool]
  if tool == nil or not allowed(p.match, tool) then
    return nil
  end
  e.tool, e.path = tool, str(input.path)
  return e
end

function M.result(r, p)
  if type(r) ~= "table" then
    return nil
  end
  local ctx, blk = r.context, r.block
  if p.hook == "session.start" and blk ~= nil then
    ctx = ctx ~= nil and (tostring(ctx) .. "\n" .. tostring(blk)) or blk
    blk = nil
  end
  if ctx == nil and blk == nil then
    return nil
  end
  local out = {}
  if ctx ~= nil then
    out.context = tostring(ctx)
  end
  if blk ~= nil then
    out.block = tostring(blk)
  end
  return airsstack.json.encode(out) .. "\n"
end

return M
