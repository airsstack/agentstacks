# Probe results: the hook shim, translator and list runtime in real OpenCode (v2.4.1, airsl 0.1.5)

The spec §4.5 shim (`agentstacks.js`), §4.7 translator and §3.2 runtime, hand-written into a project with two
stub hook modules, run through the mock-model harness. `<scratchpad>` =
`/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad`.

## Files (shim-01; shim-02 is a copy plus `opencode.json`)

```proj/.opencode/plugins/agentstacks.js
// agentstacks hook shim for OpenCode, api 0. Written by `agentstacks apply`; edits are drift.
// It runs the hook commands in HOOKS and forwards each event and result. All hook logic is Lua.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const HOOKS = [
  {"on":"session.start","text":"WELCOME-MSG"},
  {"on":"session.start","command":"\"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \".opencode/agentstacks/runtime.lua\" \"t.hooks.note\" \"0\" \"neutral\""},
  {"on":"prompt.submit","command":"\"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \".opencode/agentstacks/runtime.lua\" \"t.hooks.note\" \"0\" \"neutral\""},
  {"on":"tool.before","tools":["write","patch"],"match":["write"],"command":"\"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \".opencode/agentstacks/runtime.lua\" \"t.hooks.guard\" \"1\" \"neutral\" \"A1 $TMPDIR\""},
  {"on":"tool.after","tools":["read"],"text":"AFTER-MSG"}
];

const ROOT = fileURLToPath(new URL("../../", import.meta.url)).replace(/\/+$/, "");
const TIMEOUT_MS = 60000;

// Runs one hook command under sh from the project root, with `payload` as JSON on stdin.
// Resolves to its stdout, or null when it cannot run or runs past the timeout.
function run(command, payload) {
  return new Promise((resolve) => {
    let out = "";
    let settled = false;
    const finish = (value) => {
      if (!settled) {
        settled = true;
        resolve(value);
      }
    };
    let child;
    try {
      child = spawn("/bin/sh", ["-c", command], {
        cwd: ROOT,
        env: { ...process.env, AGENTSTACKS_PROJECT_DIR: ROOT },
        stdio: ["pipe", "pipe", "inherit"],
      });
    } catch {
      finish(null);
      return;
    }
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      finish(null);
    }, TIMEOUT_MS);
    child.on("error", () => {
      clearTimeout(timer);
      finish(null);
    });
    child.stdout.on("data", (chunk) => {
      out += chunk;
    });
    child.on("close", () => {
      clearTimeout(timer);
      finish(out);
    });
    child.stdin.on("error", () => {});
    child.stdin.end(JSON.stringify(payload));
  });
}

function parse(text) {
  try {
    const value = JSON.parse(text);
    return value !== null && typeof value === "object" ? value : null;
  } catch {
    return null;
  }
}

// Runs, in table order, every entry whose `on` is in `events` and whose `tools` holds `tool`.
// Returns the joined contexts and the first block.
async function dispatch(events, tool, payload) {
  const contexts = [];
  for (const entry of HOOKS) {
    if (!events.includes(entry.on)) continue;
    if (entry.tools && !entry.tools.includes(tool)) continue;
    if (typeof entry.text === "string") {
      contexts.push(entry.text);
      continue;
    }
    const result = parse((await run(entry.command, { ...payload, hook: entry.on, match: entry.match ?? null })) ?? "");
    if (result === null) continue;
    if (typeof result.context === "string") contexts.push(result.context);
    if (typeof result.block === "string") return { context: contexts.join("\n"), block: result.block };
  }
  return { context: contexts.join("\n"), block: null };
}

function joined(block, context) {
  return context ? block + "\n" + context : block;
}

export default {
  id: "agentstacks",
  setup: async (ctx) => {
    const parents = new Map();
    const started = new Set();
    const held = new Map();

    async function parentOf(sessionID) {
      if (!parents.has(sessionID)) {
        let parent = null;
        try {
          const info = await ctx.session.get({ sessionID });
          parent = typeof info?.parentID === "string" ? info.parentID : null;
        } catch {}
        parents.set(sessionID, parent);
      }
      return parents.get(sessionID);
    }

    async function base(sessionID) {
      return { session_id: sessionID, parent_id: await parentOf(sessionID), cwd: ctx.location.directory };
    }

    await ctx.shell.hook("create.before", (e) => {
      try {
        e.env.AGENTSTACKS_PROJECT_DIR = ROOT;
      } catch {}
    });

    await ctx.tool.hook("execute.before", async (e) => {
      let block = null;
      try {
        const payload = { ...(await base(e.sessionID)), agent: e.agent, tool: e.tool, input: e.input };
        const r = await dispatch(["tool.before", "tool.execute.before"], e.tool, payload);
        if (r.block !== null) block = joined(r.block, r.context);
        else if (r.context) held.set(e.id, r.context);
      } catch {}
      if (block !== null) throw new Error(block);
    });

    await ctx.tool.hook("execute.after", async (e) => {
      let block = null;
      try {
        const before = held.get(e.id);
        held.delete(e.id);
        if (e.status !== "completed") return;
        const payload = { ...(await base(e.sessionID)), agent: e.agent, tool: e.tool, input: e.input };
        const r = await dispatch(["tool.after", "tool.execute.after"], e.tool, payload);
        const context = [before, r.context].filter(Boolean).join("\n");
        if (r.block !== null) block = joined(r.block, context);
        else if (context) {
          e.result = { ...e.result, content: [...(e.result.content ?? []), { type: "text", text: context }] };
        }
      } catch {}
      if (block !== null) throw new Error(block);
    });

    await ctx.session.hook("prompt", async (e) => {
      try {
        const payload = await base(e.sessionID);
        if (payload.parent_id !== null) return;
        const parts = [];
        if (!started.has(e.sessionID)) {
          started.add(e.sessionID);
          const s = await dispatch(["session.start"], null, { ...payload, source: "startup" });
          if (s.context) parts.push(s.context);
          if (s.block !== null) parts.push(s.block);
        }
        const r = await dispatch(["prompt.submit", "session.prompt"], null, { ...payload, prompt: e.prompt.text });
        if (r.context) parts.push(r.context);
        if (parts.length > 0) e.prompt.text = e.prompt.text + "\n\n" + parts.join("\n");
      } catch {}
    });
  },
};
```
```proj/.opencode/agentstacks/translate.lua
-- agentstacks translator for opencode, api 0.
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
```
```proj/.opencode/agentstacks/runtime.lua
-- agentstacks hook runtime, api 0.
-- usage: airsl run ... runtime.lua <hook-module> <may_block: 0|1> <kind: neutral|native> [args...]
local translate = require("translate")
local name, may_block, kind = arg[1], arg[2] == "1", arg[3]
local args = {}
for i = 4, #arg do
  args[#args + 1] = arg[i]
end
local payload = airsstack.hook.payload()
local hook

-- Runs the hook on one event; a block from a hook that is not may_block is dropped with a line.
local function run(event)
  hook = hook or require(name)
  event.args = args
  local result = hook(event)
  if type(result) == "table" and result.block ~= nil and not may_block then
    airsstack.stdio.error("agentstacks: hook " .. name .. " tried to block but is not declared may_block; reason: "
      .. tostring(result.block) .. "\n")
    result.block = nil
  end
  return result
end

local result
if kind == "native" then
  result = run({ payload = payload })
else
  local event = translate.event(payload)
  if event == nil then
    return
  end
  if event.event ~= nil then
    result = run(event)
  else
    -- A list of events: the hook runs once per event, in order, until one blocks.
    local contexts = {}
    for _, each in ipairs(event) do
      local one = run(each)
      if type(one) == "table" then
        if one.context ~= nil then
          contexts[#contexts + 1] = tostring(one.context)
        end
        if one.block ~= nil then
          result = { block = one.block }
          break
        end
      end
    end
    if #contexts > 0 then
      result = result or {}
      result.context = table.concat(contexts, "\n")
    end
  end
end
local out = translate.result(result, payload)
if out ~= nil then
  airsstack.stdio.write(out)
end
```
```proj/.opencode/agentstacks/t/hooks/guard.lua
return function(e)
  if e.path == "blocked.txt" then
    return { block = "GUARD-BLOCK " .. e.tool .. " " .. e.path }
  end
  return { context = "GUARD-CTX " .. tostring(e.event) .. " " .. tostring(e.tool) .. " " .. tostring(e.path)
    .. " agent=" .. tostring(e.agent) .. " arg=" .. tostring(e.args[1]) }
end
```
```proj/.opencode/agentstacks/t/hooks/note.lua
return function(e)
  return { context = "NOTE-CTX " .. tostring(e.event) .. " source=" .. tostring(e.source) .. " prompt=" .. tostring(e.prompt) }
end
```
```proj/a.txt
alpha
```
```shim-02/proj/opencode.json
{
  "$schema": "https://opencode.ai/config.json",
  "model": "mock/gpt-5",
  "providers": { "mock": { "models": { "gpt-5": { "name": "G5" } } } }
}
```
```shim-01/scenario.json
[
  {"tool":"write","args":{"path":"blocked.txt","content":"no\n"}},
  {"tool":"write","args":{"path":"ok.txt","content":"yes\n"}},
  {"tool":"read","args":{"path":"a.txt"}},
  {"tool":"shell","args":{"command":"echo \"PD=$AGENTSTACKS_PROJECT_DIR\""}},
  {"text":"done-shim-01"}
]
```
```shim-02/scenario.json
[
  {"tool":"patch","args":{"patchText":"*** Begin Patch\n*** Add File: blocked.txt\n+no\n*** Update File: a.txt\n@@\n-alpha\n+beta\n*** End Patch"}},
  {"tool":"patch","args":{"patchText":"*** Begin Patch\n*** Add File: ok2.txt\n+yes\n*** Update File: a.txt\n@@\n-alpha\n+gamma\n*** End Patch"}},
  {"text":"done-shim-02"}
]
```
```shim-bin/show.js
// usage: node show.js <probe-dir>
// Prints the user message and every tool call/result of the last request that offered tools,
// as the model saw them, plus the project files left behind.
const fs = require("fs");
const path = require("path");
const dir = process.argv[2];
const lines = fs.readFileSync(path.join(dir, "requests.jsonl"), "utf8").trim().split("\n").map((l) => JSON.parse(l));
const withTools = lines.filter((x) => x.body && x.body.tools && x.body.tools.length);
const last = withTools[withTools.length - 1];
for (const m of last.body.messages) {
  if (m.role === "user") console.log(JSON.stringify({ role: "user", content: m.content }));
  if (m.role === "tool") console.log(JSON.stringify({ role: "tool", tool_call_id: m.tool_call_id, content: m.content }));
  if (m.role === "assistant" && m.tool_calls)
    console.log(JSON.stringify({ role: "assistant", calls: m.tool_calls.map((c) => c.function.name + " " + c.function.arguments) }));
}
const proj = path.join(dir, "proj");
for (const f of fs.readdirSync(proj).filter((f) => !f.startsWith(".")).sort()) {
  console.log("file " + f + ": " + JSON.stringify(fs.readFileSync(path.join(proj, f), "utf8")));
}
```

## Commands (each run once)

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh <scratchpad>/shim-01 18840 "hello"
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh <scratchpad>/shim-02 18841 "patch it"
```

## Output

```text
$ tail -1 shim-01/out.txt shim-02/out.txt; wc -c shim-01/err.txt shim-02/err.txt
exit=0
exit=0
       0
       0
$ node shim-bin/show.js shim-01
{"role":"user","content":"hello\n\nWELCOME-MSG\nNOTE-CTX session.start source=startup prompt=nil\nNOTE-CTX prompt.submit source=nil prompt=hello"}
{"role":"assistant","calls":["write {\"path\":\"blocked.txt\",\"content\":\"no\\n\"}"]}
{"role":"tool","tool_call_id":"call_1","content":"{\"error\":{\"type\":\"unknown\",\"message\":\"GUARD-BLOCK write blocked.txt\"},\"content\":[]}"}
{"role":"assistant","calls":["write {\"path\":\"ok.txt\",\"content\":\"yes\\n\"}"]}
{"role":"tool","tool_call_id":"call_2","content":"Created file successfully: ok.txt\nGUARD-CTX tool.before write ok.txt agent=nil arg=A1 /var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/"}
{"role":"assistant","calls":["read {\"path\":\"a.txt\"}"]}
{"role":"tool","tool_call_id":"call_3","content":"Read file a.txt, lines 1-1\n1: alpha\nAFTER-MSG"}
{"role":"assistant","calls":["shell {\"command\":\"echo \\\"PD=$AGENTSTACKS_PROJECT_DIR\\\"\"}"]}
{"role":"tool","tool_call_id":"call_4","content":"PD=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/shim-01/proj\n"}
file a.txt: "alpha\n"
file ok.txt: "yes\n"
$ node shim-bin/show.js shim-02
{"role":"user","content":"patch it\n\nWELCOME-MSG\nNOTE-CTX session.start source=startup prompt=nil\nNOTE-CTX prompt.submit source=nil prompt=patch it"}
{"role":"assistant","calls":["patch {\"patchText\":\"*** Begin Patch\\n*** Add File: blocked.txt\\n+no\\n*** Update File: a.txt\\n@@\\n-alpha\\n+beta\\n*** End Patch\"}"]}
{"role":"tool","tool_call_id":"call_1","content":"{\"error\":{\"type\":\"unknown\",\"message\":\"GUARD-BLOCK write blocked.txt\"},\"content\":[]}"}
{"role":"assistant","calls":["patch {\"patchText\":\"*** Begin Patch\\n*** Add File: ok2.txt\\n+yes\\n*** Update File: a.txt\\n@@\\n-alpha\\n+gamma\\n*** End Patch\"}"]}
{"role":"tool","tool_call_id":"call_2","content":"Success. Updated the following files:\nA ok2.txt\nM a.txt\nGUARD-CTX tool.before write ok2.txt agent=nil arg=A1 /var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/"}
file a.txt: "gamma\n"
file ok2.txt: "yes\n"
file opencode.json: "{\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"model\": \"mock/gpt-5\",\n  \"providers\": { \"mock\": { \"models\": { \"gpt-5\": { \"name\": \"G5\" } } } }\n}\n"
```
