$ airsl --version
airsl 0.1.4
$ echo '{"x":1}' | airsl run --policy confined root/runtime.lua core.hooks.enforce
RESULT {"context":"hello tool.before"}
payload {"x":1}
exit=0
$ echo '{"x":1}' | airsl run --policy confined root/runtime.lua team-lint.hooks.my-file
RESULT {}
payload {"x":1}
exit=0
$ echo '{"x":1}' | airsl run --policy confined root/runtime.lua core/hooks/enforce
REQUIRE-FAIL invalid require target `core/hooks/enforce`: must contain only ASCII letters, digits, underscores, hyphens and dots
stack traceback:
	[C]: in function 'require'
	[C]: in function 'pcall'
	root/runtime.lua:2: in main chunk

exit=0
$ echo '{"x":1}' | airsl run --policy confined root/runtime.lua core.hooks.missing
REQUIRE-FAIL module `core.hooks.missing` not found under `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-hookreq/root`
stack traceback:
	[C]: in function 'require'
	[C]: in function 'pcall'
	root/runtime.lua:2: in main chunk

exit=0
$ echo '{"x":1}' | airsl run --policy pure root/runtime.lua core.hooks.enforce
REQUIRE-FAIL attempt to call a nil value
exit=0
$ echo '{"x":1}' | airsl run --policy pure root/runtime.lua team-lint.hooks.my-file
REQUIRE-FAIL attempt to call a nil value
exit=0
$ echo '{"x":1}' | airsl run --policy pure root/runtime.lua core/hooks/enforce
REQUIRE-FAIL attempt to call a nil value
exit=0
$ echo '{"x":1}' | airsl run --policy pure root/runtime.lua core.hooks.missing
REQUIRE-FAIL attempt to call a nil value
exit=0
