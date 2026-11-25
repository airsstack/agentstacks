# Probe results: review follow-ups (OpenCode v2.4.1, airsl 0.1.5)

Each project starts as a copy of a `shim-results.md` project (same runtime, translator, hook modules and shim);
only the differences are listed. `<scratchpad>` =
`/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad`. `show.js` is `shim-bin/show.js` (in `shim-results.md`).

## F1: without `--auto`, does the before hook run before OpenCode's permission check? (fx-c)

Copy of shim-01. Runner: `fxh/run-noauto.sh` = `probes/harness/run.sh` with `--standalone --auto` changed to `--standalone`.

```fx-c/proj/opencode.json
{
  "$schema": "https://opencode.ai/config.json",
  "permissions": [{ "action": "edit", "resource": "*", "effect": "deny" }]
}
```
```fx-c/scenario.json
[
  {"tool":"write","args":{"path":"blocked.txt","content":"no\n"}},
  {"tool":"write","args":{"path":"ok.txt","content":"yes\n"}},
  {"text":"done-fx-c"}
]
```

```text
$ sh fxh/run-noauto.sh <scratchpad>/fx-c 18850 "hello"; tail -1 fx-c/out.txt; node shim-bin/show.js fx-c
exit=0
{"role":"user","content":"hello\n\nWELCOME-MSG\nNOTE-CTX session.start source=startup prompt=nil\nNOTE-CTX prompt.submit source=nil prompt=hello"}
{"role":"assistant","calls":["write {\"path\":\"blocked.txt\",\"content\":\"no\\n\"}"]}
{"role":"tool","tool_call_id":"call_1","content":"{\"error\":{\"type\":\"unknown\",\"message\":\"GUARD-BLOCK write blocked.txt\"},\"content\":[]}"}
{"role":"assistant","calls":["write {\"path\":\"ok.txt\",\"content\":\"yes\\n\"}"]}
{"role":"tool","tool_call_id":"call_2","content":"{\"error\":{\"type\":\"tool.execution\",\"message\":\"No tool named \\\"write\\\" is currently available. Please use a tool from the available tool list.\"},\"content\":[]}"}
file a.txt: "alpha\n"
file opencode.json: "{\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"permissions\": [{ \"action\": \"edit\", \"resource\": \"*\", \"effect\": \"deny\" }]\n}\n"
```

**Answer**: with `edit` denied by config and no `--auto`, the `blocked.txt` call got the hook's
block text, so `execute.before` ran before OpenCode's permission decision. The `ok.txt` call then
failed with `No tool named "write" is currently available`: the hook ran and its context was
dropped with the failed call.

## F2: patch Move, Delete and an indented header (fx-d)

Copy of shim-02 (`mock/gpt-5`), with the `tool.before` entry changed to run for every file tool with no `match`:

```text
$ grep -n '"on":"tool.before"' fx-d/proj/.opencode/plugins/agentstacks.js
10:  {"on":"tool.before","tools":["read","edit","write","patch"],"command":"\"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \".opencode/agentstacks/runtime.lua\" \"t.hooks.guard\" \"1\" \"neutral\" \"A1 $TMPDIR\""},
$ head -c 200 fx-d/proj/a.txt fx-d/proj/a2.txt fx-d/proj/c.txt  (before the run: alpha, alpha, gone)
```
```fx-d/scenario.json
[
  {"tool":"patch","args":{"patchText":"*** Begin Patch\n*** Update File: a.txt\n*** Move to: blocked.txt\n@@\n-alpha\n+beta\n*** End Patch"}},
  {"tool":"patch","args":{"patchText":"*** Begin Patch\n*** Update File: a2.txt\n*** Move to: moved.txt\n@@\n-alpha\n+beta\n*** End Patch"}},
  {"tool":"patch","args":{"patchText":"*** Begin Patch\n*** Delete File: c.txt\n*** End Patch"}},
  {"tool":"patch","args":{"patchText":"*** Begin Patch\n  *** Add File: d.txt\n+indented\n*** End Patch"}},
  {"text":"done-fx-d"}
]
```

```text
$ sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh <scratchpad>/fx-d 18851 "patch it"; tail -1 fx-d/out.txt; node shim-bin/show.js fx-d
exit=0
{"role":"user","content":"patch it\n\nWELCOME-MSG\nNOTE-CTX session.start source=startup prompt=nil\nNOTE-CTX prompt.submit source=nil prompt=patch it"}
{"role":"assistant","calls":["patch {\"patchText\":\"*** Begin Patch\\n*** Update File: a.txt\\n*** Move to: blocked.txt\\n@@\\n-alpha\\n+beta\\n*** End Patch\"}"]}
{"role":"tool","tool_call_id":"call_1","content":"{\"error\":{\"type\":\"unknown\",\"message\":\"GUARD-BLOCK edit blocked.txt\"},\"content\":[]}"}
{"role":"assistant","calls":["patch {\"patchText\":\"*** Begin Patch\\n*** Update File: a2.txt\\n*** Move to: moved.txt\\n@@\\n-alpha\\n+beta\\n*** End Patch\"}"]}
{"role":"tool","tool_call_id":"call_2","content":"Success. Updated the following files:\nM moved.txt\nGUARD-CTX tool.before edit moved.txt agent=nil arg=A1 /var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/"}
{"role":"assistant","calls":["patch {\"patchText\":\"*** Begin Patch\\n*** Delete File: c.txt\\n*** End Patch\"}"]}
{"role":"tool","tool_call_id":"call_3","content":"Success. Updated the following files:\nD c.txt"}
{"role":"assistant","calls":["patch {\"patchText\":\"*** Begin Patch\\n  *** Add File: d.txt\\n+indented\\n*** End Patch\"}"]}
{"role":"tool","tool_call_id":"call_4","content":"Success. Updated the following files:\nA d.txt"}
file a.txt: "alpha\n"
file d.txt: "indented\n"
file moved.txt: "beta\n"
file opencode.json: "{\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"model\": \"mock/gpt-5\",\n  \"providers\": { \"mock\": { \"models\": { \"gpt-5\": { \"name\": \"G5\" } } } }\n}\n"
```

**Answer**: `Update File: a.txt` + `Move to: blocked.txt` reached the hook as an edit of `blocked.txt`
and was blocked (`a.txt` unchanged). `Move to: moved.txt` gave an edit of `moved.txt`; OpenCode
removed `a2.txt` and wrote `moved.txt`. A delete-only patch ran no hook (no context) and removed
`c.txt`. An indented `  *** Add File: d.txt` was applied by OpenCode (`d.txt` exists) and ran no hook.

## F3: the shim with stderr ignored, in a CommonJS project (fx-e)

Copy of shim-01 with `stdio: ["pipe", "pipe", "inherit"]` changed to `["pipe", "pipe", "ignore"]`, plus:

```fx-e/proj/package.json
{ "name": "cjs-project", "type": "commonjs" }
```
```text
$ grep -n stdio fx-e/proj/.opencode/plugins/agentstacks.js
34:        stdio: ["pipe", "pipe", "ignore"],
$ sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh <scratchpad>/fx-e 18852 "hello"; tail -1 fx-e/out.txt; node shim-bin/show.js fx-e
exit=0
{"role":"user","content":"hello\n\nWELCOME-MSG\nNOTE-CTX session.start source=startup prompt=nil\nNOTE-CTX prompt.submit source=nil prompt=hello"}
{"role":"assistant","calls":["write {\"path\":\"blocked.txt\",\"content\":\"no\\n\"}"]}
{"role":"tool","tool_call_id":"call_1","content":"{\"error\":{\"type\":\"unknown\",\"message\":\"GUARD-BLOCK write blocked.txt\"},\"content\":[]}"}
{"role":"assistant","calls":["write {\"path\":\"ok.txt\",\"content\":\"yes\\n\"}"]}
{"role":"tool","tool_call_id":"call_2","content":"Created file successfully: ok.txt\nGUARD-CTX tool.before write ok.txt agent=nil arg=A1 /var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/"}
{"role":"assistant","calls":["read {\"path\":\"a.txt\"}"]}
{"role":"tool","tool_call_id":"call_3","content":"Read file a.txt, lines 1-1\n1: alpha\nAFTER-MSG"}
{"role":"assistant","calls":["shell {\"command\":\"echo \\\"PD=$AGENTSTACKS_PROJECT_DIR\\\"\"}"]}
{"role":"tool","tool_call_id":"call_4","content":"PD=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/fx-e/proj\n"}
file a.txt: "alpha\n"
file ok.txt: "yes\n"
file package.json: "{ \"name\": \"cjs-project\", \"type\": \"commonjs\" }\n"
```

**Answer**: the shim loads in a project whose `package.json` says `"type": "commonjs"`, and with
stderr ignored every shim-01 result is the same.
