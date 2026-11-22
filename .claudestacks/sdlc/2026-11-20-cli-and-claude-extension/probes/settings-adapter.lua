local function sorted_keys(t)
  local ks = {}
  for k, _ in pairs(t) do ks[#ks + 1] = tostring(k) end
  table.sort(ks)
  return table.concat(ks, ",")
end

agentstacks.on("values", function(q)
  local out = {
    root = ".claude/agentstacks",
    skills = {},
    agents = {},
    vars = { session_id = "${CLAUDE_SESSION_ID}", arguments = "$ARGUMENTS", tmpdir = "$TMPDIR" },
  }
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills) do
      out.skills[p.name .. "/" .. s] = { invoke = "/" .. s, dir = ".claude/skills/" .. s }
    end
    for _, a in ipairs(p.agents) do
      out.agents[p.name .. "/" .. a] = a
    end
  end
  return out
end)

-- describe one Lua value as JSON-safe strings/booleans
local function describe(x)
  local d = { type = type(x), tostring = tostring(x) }
  if type(x) == "table" then
    d.next_is_nil = (next(x) == nil)
    d.len = #x
  end
  local okm, mt = pcall(getmetatable, x)
  d.has_metatable = okm and (mt ~= nil) or ("getmetatable err: " .. tostring(mt))
  if okm and type(mt) == "table" then d.metatable_keys = sorted_keys(mt) end
  return d
end

local function try_edit(start, path, value)
  local ok, res = pcall(agentstacks.json.edit, start, path, value)
  return { ok = ok, result = tostring(res) }
end

agentstacks.on("plan", function(q)
  local ids = {}
  local agent_meta = nil
  for _, p in ipairs(q.packs) do
    for _, s in ipairs(p.skills or {}) do
      for _, f in ipairs(s.files or {}) do ids[#ids + 1] = f.id end
    end
    for _, a in ipairs(p.agents or {}) do ids[#ids + 1] = a.id; agent_meta = a.meta end
    for _, h in ipairs(p.hooks or {}) do ids[#ids + 1] = h.id end
    local n = p.native or {}
    for _, f in ipairs(n.files or {}) do ids[#ids + 1] = f.id end
    for _, h in ipairs(n.hooks or {}) do ids[#ids + 1] = h.id end
  end

  local obs = {}

  -- S1 + S2
  local cases = {
    { name = "01_null",       text = '{"a":null}' },
    { name = "02_empty_arr",  text = '{"a":[]}' },
    { name = "03_empty_obj",  text = '{"a":{}}' },
    { name = "04_top_arr",    text = '[]' },
    { name = "05_json_empty_string", text = '""' },
    { name = "06_lua_empty",  text = "" },
    { name = "07_whitespace", text = "  \n" },
  }
  local s1, s2 = {}, {}
  for _, c in ipairs(cases) do
    local ok, v = pcall(airsstack.json.decode, c.text)
    local r = { input = c.text, ok = ok }
    if not ok then
      r.err = tostring(v)
    else
      r.top = describe(v)
      if type(v) == "table" then
        local oka, a = pcall(function() return v.a end)
        if oka then r.a = describe(a) else r.a_err = tostring(a) end
        s2[c.name .. "/top"] = try_edit("{}", { "x" }, v)
        if c.text:sub(1, 1) == "{" then
          s2[c.name .. "/a"] = try_edit("{}", { "x" }, v.a)
        end
      else
        s2[c.name .. "/top"] = try_edit("{}", { "x" }, v)
      end
    end
    s1[c.name] = r
  end
  obs.S1 = s1
  obs.S2 = s2

  -- S3
  local s3val = { { hooks = { { type = "command", command = "printf '%s\\n' 'x' # agentstacks" } } } }
  obs.S3 = {
    start_brace_nl = try_edit("{}\n", { "hooks", "SessionEnd" }, s3val),
    start_empty = try_edit("", { "hooks", "SessionEnd" }, s3val),
  }

  -- S4
  local old = [[
{
  "permissions": {
    "allow": ["Bash(ls:*)"]
  },
  "hooks": {
    "SessionEnd": [
      { "hooks": [ { "type": "command", "command": "echo user-hook", "timeout": 5 } ] },
      { "hooks": [ { "type": "command", "command": "echo old # agentstacks" } ] }
    ]
  }
}
]]
  local s4 = { old = old }
  local okd, doc = pcall(airsstack.json.decode, old)
  s4.decode_ok = okd
  if okd then
    local groups = {}
    for _, g in ipairs(doc.hooks.SessionEnd) do
      local kept = {}
      for _, h in ipairs(g.hooks) do
        if not string.find(h.command or "", "# agentstacks", 1, true) then kept[#kept + 1] = h end
      end
      if #kept > 0 then
        local ng = {}
        for k, val in pairs(g) do ng[k] = val end
        ng.hooks = kept
        groups[#groups + 1] = ng
      end
    end
    groups[#groups + 1] = { hooks = { { type = "command", command = "printf '%s\\n' 'new' # agentstacks" } } }
    local pre = { { matcher = "Read|Edit|Write", hooks = { { type = "command", command = "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" # agentstacks" } } } }
    local ok1, t1 = pcall(agentstacks.json.edit, old, { "hooks", "PreToolUse" }, pre)
    s4.step1 = { ok = ok1, result = tostring(t1) }
    if ok1 then
      local ok2, t2 = pcall(agentstacks.json.edit, t1, { "hooks", "SessionEnd" }, groups)
      s4.step2 = { ok = ok2, result = tostring(t2) }
      if ok2 then agentstacks.write { path = "s4.json", content = t2 } end
    end
  else
    s4.decode_err = tostring(doc)
  end
  obs.S4 = s4

  -- S5
  obs.S5 = {
    hooks_5 = try_edit('{"hooks":5}', { "hooks", "agentstacks-check" }, nil),
    hooks_arr = try_edit('{"hooks":[]}', { "hooks", "agentstacks-check" }, nil),
  }

  -- S6
  local n = agent_meta and agent_meta.native or {}
  local function enc(x) local ok, r = pcall(airsstack.json.encode, x); return { ok = ok, result = tostring(r) } end
  obs.S6 = {
    native_keys = sorted_keys(n),
    empty = { encode = enc(n.empty), describe = describe(n.empty) },
    list = { encode = enc(n.list), describe = describe(n.list) },
    s = { encode = enc(n.s), describe = describe(n.s) },
    native_encode = enc(n),
  }

  agentstacks.write { path = q.root .. "/translate.lua", content = "local M = {}\nfunction M.event(p) return nil end\nfunction M.result(r) return nil end\nreturn M\n" }
  agentstacks.write { path = "out.json", content = airsstack.json.encode(obs), covers = ids }
  if obs.S3.start_brace_nl.ok then agentstacks.write { path = "s3a.json", content = obs.S3.start_brace_nl.result } end
  if obs.S3.start_empty.ok then agentstacks.write { path = "s3b.json", content = obs.S3.start_empty.result } end
end)
