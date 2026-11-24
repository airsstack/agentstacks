-- Enforcement doctor — explains why the enforce hook does or does not fire for one path.
--
-- A script, not a hook module: `doctor.lua <pack-root> <path>`, where `<pack-root>` is the core
-- pack's own root, as the hook receives it. Prints the explanation to stdout, or the usage text
-- to stderr when an argument is missing. Exits 0 either way.
--
-- The behaviour lives in `lib/enforce.lua`'s `doctor`, so the tests call it directly.

local enforce = require("lib.enforce")

local out, err = enforce.doctor(arg, airsstack.env)
if out then
  airsstack.stdio.write(out)
end
if err then
  airsstack.stdio.error(err)
end
