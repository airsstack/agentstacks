#!/bin/sh
# Probe: can a shared runtime.lua `require` a pack hook file under the airsl binary?
D=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-hookreq
rm -rf "$D"
mkdir -p "$D/root/core/hooks" "$D/root/team-lint/hooks"
cd "$D" || exit 1
printf 'return function(ev) return { context = "hello " .. ev.event } end\n' > root/core/hooks/enforce.lua
printf 'return function() return nil end\n' > root/team-lint/hooks/my-file.lua
cat > root/runtime.lua <<'EOF'
local name = arg[1]
local ok, hook = pcall(require, name)
if not ok then print("REQUIRE-FAIL " .. tostring(hook)) return end
local r = hook({ event = "tool.before" })
print("RESULT " .. airsstack.json.encode(r or {}))
print("payload " .. airsstack.json.encode(airsstack.hook.payload()))
EOF
echo '$ airsl --version'
airsl --version
for pol in confined pure; do
  for m in core.hooks.enforce team-lint.hooks.my-file core/hooks/enforce core.hooks.missing; do
    echo "\$ echo '{\"x\":1}' | airsl run --policy $pol root/runtime.lua $m"
    echo '{"x":1}' | airsl run --policy "$pol" root/runtime.lua "$m" 2>&1
    echo "exit=$?"
  done
done
