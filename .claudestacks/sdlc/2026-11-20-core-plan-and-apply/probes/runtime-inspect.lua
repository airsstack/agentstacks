local keys = {}
for k, v in pairs(airsstack) do keys[#keys + 1] = k .. ":" .. type(v) end
table.sort(keys)
print("airsstack = " .. table.concat(keys, " "))
print("io=" .. type(io) .. " os=" .. type(os) .. " print=" .. type(print) .. " require=" .. type(require))
local s = {}
for k in pairs(airsstack.stdio or {}) do s[#s + 1] = k end
table.sort(s)
print("airsstack.stdio = " .. table.concat(s, " "))
local h = {}
for k in pairs(airsstack.hook or {}) do h[#h + 1] = k end
table.sort(h)
print("airsstack.hook = " .. table.concat(h, " "))
