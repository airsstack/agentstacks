-- A block whose reason names the tool path when the event has one, so the path the translator
-- made absolute shows in the output.
return function(ev)
  if ev.path ~= nil then
    return { block = "blk " .. ev.path }
  end
  return { block = "blk" }
end
