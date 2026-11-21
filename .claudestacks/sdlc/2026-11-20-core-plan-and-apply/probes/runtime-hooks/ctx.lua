local fskeys = {}
for k in pairs(airsstack.fs) do fskeys[#fskeys + 1] = k end
table.sort(fskeys)
return function(ev)
  local ok, data = pcall(airsstack.fs.read, ".tool/agentstacks/core/data.txt")
  return { context = "event=" .. tostring(ev.event) .. " session=" .. tostring(ev.session_id)
    .. " args=" .. table.concat(ev.args, ",") .. " read=" .. tostring(ok) .. ":" .. tostring(data)
    .. " fs=" .. table.concat(fskeys, " ") }
end
