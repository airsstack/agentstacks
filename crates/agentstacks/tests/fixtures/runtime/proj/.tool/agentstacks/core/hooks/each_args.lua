return function(ev)
  return { context = ev.path .. " args=" .. table.concat(ev.args, ",") }
end
