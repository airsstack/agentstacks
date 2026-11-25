# Probe results: plan-set review follow-ups (OpenCode v2.4.1, airsl 0.1.5, agentstacks 0.1.0 at f19693c)

`<scratchpad>` = `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad`. `show.js` is `shim-bin/show.js` (in `shim-results.md`).

## R1: a hook shim that tells the model once when airsl is not on PATH (ms-01, ms-02)

Both projects are copies of shim-01 (`shim-results.md`: session.start message and lua hooks, a
prompt.submit lua hook, a `tool.before` lua hook on write, a `tool.after` message on read). The shim
is shim-01's header and table, then the spec §4.5 fixed body with this change (`ms-shim.js`):

```diff
--- spec §4.5 fixed body
+++ ms-shim.js body
@@ -1,6 +1,11 @@
 const ROOT = fileURLToPath(new URL("../../", import.meta.url)).replace(/\/+$/, "");
 const TIMEOUT_MS = 60000;
+const AIRSL_MISSING =
+  "agentstacks: airsl is not on PATH, so the agentstacks Lua hooks are not running. Install airsl, then run agentstacks doctor.";
 
+// Set when sh could not find a hook command's program (exit 127): every Lua hook runs airsl.
+let airslMissing = false;
+
 // Runs one hook command under sh from the project root, with `payload` as JSON on stdin.
 // Resolves to its stdout, or null when it cannot run or runs past the timeout.
 function run(command, payload) {
@@ -35,8 +40,9 @@
     child.stdout.on("data", (chunk) => {
       out += chunk;
     });
-    child.on("close", () => {
+    child.on("close", (code) => {
       clearTimeout(timer);
+      if (code === 127) airslMissing = true;
       finish(out);
     });
     child.stdin.on("error", () => {});
@@ -81,6 +87,7 @@
   setup: async (ctx) => {
     const parents = new Map();
     const started = new Set();
+    let told = false;
     const held = new Map();
 
     async function parentOf(sessionID) {
@@ -146,6 +153,10 @@
         }
         const r = await dispatch(["prompt.submit", "session.prompt"], null, { ...payload, prompt: e.prompt.text });
         if (r.context) parts.push(r.context);
+        if (airslMissing && !told) {
+          told = true;
+          parts.push(AIRSL_MISSING);
+        }
         if (parts.length > 0) e.prompt.text = e.prompt.text + "\n\n" + parts.join("\n");
       } catch {}
     });
```

Runner: `ms-run.sh` (ms-01 runs `probes/harness/run.sh` with `PATH` minus `~/.local/bin` and
`~/.cargo/bin`, the two directories holding airsl; ms-02 with the full `PATH`). Scenario for both:

```ms-01/scenario.json
[
  {"tool":"write","args":{"path":"ok.txt","content":"yes\n"}},
  {"tool":"read","args":{"path":"a.txt"}},
  {"text":"done-ms"}
]
```

```text
$ sh ms-run.sh
$ PATH=<PATH without ~/.local/bin and ~/.cargo/bin> command -v airsl; echo "found=$?"
found=1
$ PATH=<same> /bin/sh -c '"airsl" "run" "--fail-open" "x.lua"' </dev/null; echo "exit=$?"
/bin/sh: airsl: command not found
exit=127
$ (airsl on PATH) airsl run --fail-open --policy confined missing.lua </dev/null; echo "exit=$?"
exit=0

== ms-01 (no airsl on PATH)
exit=0
{"role":"user","content":"hello\n\nWELCOME-MSG\nagentstacks: airsl is not on PATH, so the agentstacks Lua hooks are not running. Install airsl, then run agentstacks doctor."}
{"role":"assistant","calls":["write {\"path\":\"ok.txt\",\"content\":\"yes\\n\"}"]}
{"role":"tool","tool_call_id":"call_1","content":"Created file successfully: ok.txt"}
{"role":"assistant","calls":["read {\"path\":\"a.txt\"}"]}
{"role":"tool","tool_call_id":"call_2","content":"Read file a.txt, lines 1-1\n1: alpha\nAFTER-MSG"}
file a.txt: "alpha\n"
file ok.txt: "yes\n"

== ms-02 (airsl on PATH)
exit=0
{"role":"user","content":"hello\n\nWELCOME-MSG\nNOTE-CTX session.start source=startup prompt=nil\nNOTE-CTX prompt.submit source=nil prompt=hello"}
{"role":"assistant","calls":["write {\"path\":\"ok.txt\",\"content\":\"yes\\n\"}"]}
{"role":"tool","tool_call_id":"call_1","content":"Created file successfully: ok.txt\nGUARD-CTX tool.before write ok.txt agent=nil arg=A1 /var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/"}
{"role":"assistant","calls":["read {\"path\":\"a.txt\"}"]}
{"role":"tool","tool_call_id":"call_2","content":"Read file a.txt, lines 1-1\n1: alpha\nAFTER-MSG"}
file a.txt: "alpha\n"
file ok.txt: "yes\n"
```

**Answer**: with airsl missing, `sh` exits 127 (`command not found`), every Lua hook gives no
output, and the model gets the notice once, at the end of the first prompt, after the message hook
context; the session goes on (the write and read ran, the read's message hook still delivered).
Two Lua hooks failed in that prompt and one notice was added. With airsl present (ms-02), nothing
changes from shim-01. airsl itself exits 0 for a missing script under `--fail-open`.

## R2: how `airsstack.json.encode` writes permissions and empty tables, and whether OpenCode loads it (pe-01)

```pe/enc.lua
-- How airsstack.json.encode writes the values the opencode frontmatter and shim table need.
local function rule(action, effect)
  return { action = action, resource = "*", effect = effect }
end
print("ro:    " .. airsstack.json.encode({ rule("*", "deny"), rule("read", "allow"), rule("grep", "allow") }))
print("deny:  " .. airsstack.json.encode({ rule("*", "deny") }))
print("empty: " .. airsstack.json.encode({}))
print("entry: " .. airsstack.json.encode({ on = "tool.execute.before", tools = {}, command = "x" }))
```

```text
$ airsl --version; airsl run --policy confined enc.lua; echo "exit=$?"
airsl 0.1.5
ro:    [{"action":"*","effect":"deny","resource":"*"},{"action":"read","effect":"allow","resource":"*"},{"action":"grep","effect":"allow","resource":"*"}]
deny:  [{"action":"*","effect":"deny","resource":"*"}]
empty: {}
entry: {"command":"x","on":"tool.execute.before","tools":{}}
exit=0
```

The agent file below holds the `ro:` line above byte for byte (`pe-run.sh`):

```pe-01/proj/.opencode/agents/ro.md
---
description: "Read-only: read and grep only"
mode: "subagent"
permissions: [{"action":"*","effect":"deny","resource":"*"},{"action":"read","effect":"allow","resource":"*"},{"action":"grep","effect":"allow","resource":"*"}]
---
You are RO-SYSTEM-PROMPT.
```
```pe-01/scenario.json
[{"tool":"subagent","args":{"agent":"ro","description":"probe read only","prompt":"Write a file."}},{"text":"parent waits"},{"tool":"write","args":{"path":"x.txt","content":"hi"}},{"text":"child done"}]
```

```text
$ sh probes/harness/run.sh <scratchpad>/pe-01 18873 "hello"; tail -1 pe-01/out.txt; <per request: system prompt head, tools offered, last tool result>
exit=0
parent tools=edit,execute,generate_image,generate_video,glob,grep,question,read,shell,skill,subagent,webfetch,websearch,write last="hello"
parent tools=edit,execute,generate_image,generate_video,glob,grep,question,read,shell,skill,subagent,webfetch,websearch,write last="The subagent is working in the background (sessionID: ses_df14f4185ffe3r2635tUpDK3rD). You will receive a <background-result kind=\"subagent\"> messa
ro     tools=grep,read,skill last="You are a subagent spawned by another session.\nWrite a file."
ro     tools=grep,read,skill last="{\"error\":{\"type\":\"tool.execution\",\"message\":\"No tool named \\\"write\\\" is currently available. Please use a tool from the available tool l
parent tools=edit,execute,generate_image,generate_video,glob,grep,question,read,shell,skill,subagent,webfetch,websearch,write last="<background-result kind=\"subagent\" sessionID=\"ses_df14f4185ffe3r2635tUpDK3rD\" state=\"completed\" description=\"probe read only\">\nchild done\n<
```

**Answer**: `encode` sorts object keys and writes no spaces; OpenCode reads that `permissions`
list the same as the spaced form of `agents-results.md` P3 (the subagent is offered `grep,read,skill`
and its `write` call fails). An empty Lua table encodes as `{}`, never `[]`.

## R3: a native hook with `match = []` (nm-01)

A scratch project with `extensions/claude` copied to `ext/claude` and the CLI demo pack copied to
`packs/demo`, plus a native hook file (`nm-run.sh`):

```packs/demo/native/claude/hooks.toml
[[hooks]]
kind = "message"
event = "PostToolUse"
match = []
text = "EMPTY-MATCH"
```
```text
$ agentstacks --version
agentstacks 0.1.0
$ agentstacks init --tool ext/claude --pack packs/demo; echo "exit=$?"
agentstacks plan

  create   .claude/agents/demo-helper.md             demo
  create   .claude/agentstacks/demo/hooks/guard.lua  demo
  create   .claude/agentstacks/runtime.lua           claude
  create   .claude/agentstacks/translate.lua         claude
  create   .claude/settings.json                     shared
  create   .claude/skills/demo-greet/SKILL.md        demo

  airsl    0.1.5 (new pin)

6 to write, 0 to delete, 0 unchanged, 0 skipped
stdin is not a terminal; pass --yes to apply
agentstacks.toml written; nothing applied (run apply)
exit=1
$ agentstacks plan; echo "exit=$?"
agentstacks plan

  create   .claude/agents/demo-helper.md             demo
  create   .claude/agentstacks/demo/hooks/guard.lua  demo
  create   .claude/agentstacks/runtime.lua           claude
  create   .claude/agentstacks/translate.lua         claude
  create   .claude/settings.json                     shared
  create   .claude/skills/demo-greet/SKILL.md        demo

  airsl    0.1.5 (new pin)

6 to write, 0 to delete, 0 unchanged, 0 skipped
exit=0
$ agentstacks apply --yes; echo "exit=$?"

6 to write, 0 to delete, 0 unchanged, 0 skipped
applied: 6 written, 0 deleted
exit=0
$ grep -n -B2 -A6 EMPTY-MATCH .claude/settings.json
5-        "hooks": [
6-          {
7:            "command": "printf '%s\\n' '{\"hookSpecificOutput\":{\"additionalContext\":\"EMPTY-MATCH\",\"hookEventName\":\"PostToolUse\"}}' # agentstacks",
8-            "type": "command"
9-          }
10-        ],
11-        "matcher": ""
12-      }
13-    ],
```

**Answer**: the core accepts an empty native `match` (plan and apply succeed, nothing skipped) and
passes it to the extension; the claude extension renders it as `"matcher": ""`, which Claude Code
reads as every tool. Also seen: `init` without a terminal exits 1 after writing `agentstacks.toml`
(`pass --yes to apply`).

## R4: the airsl notice across prompts and sessions of one OpenCode process (ms-03, ms-04)

One `opencode serve --port <p>` process (with `OPENCODE_PASSWORD` set: without it a client fails
`Server at http://127.0.0.1:<p> requires a password; set OPENCODE_PASSWORD`), started in the
project with airsl off its `PATH`, and several `opencode run --server http://127.0.0.1:<p> --auto
--format json` clients. Each project is a copy of ms-01 with `ms-shim.js` (R1). The mock, config
and isolation are those of `probes/harness/run.sh`. The `user:` lines list every distinct user
message the mock received, in order. Runner: `ms-srv.sh`.

```text
$ sh ms-srv.sh
== ms-03: shim-01 table; prompt 1 and 2 in one session, prompt 3 in a new session; one server process
```ms-03/scenario.json
[
  {"tool":"write","args":{"path":"blocked.txt","content":"no\n"}},
  {"text":"done-1"},
  {"text":"done-2"},
  {"text":"done-3"}
]
```
c1: exit=0 session=ses_df146bdd6ffeLCC9NI0xLfHI9H
c2: exit=0 session=ses_df146bdd6ffeLCC9NI0xLfHI9H
c3: exit=0 session=ses_df146bb6bffeJyG4sPNQI0GhHa
server pids running after stop: 0
user: "first\n\nWELCOME-MSG\nagentstacks: airsl is not on PATH, so the agentstacks Lua hooks are not running. Install airsl, then run agentstacks doctor."
user: "second"
user: "third\n\nWELCOME-MSG"
file blocked.txt: no

== ms-04: only a lua tool.before hook on write; prompt 1 writes, prompt 2 in the same session
$ sed -n "/^const HOOKS/,/^];/p" ms-04/proj/.opencode/plugins/agentstacks.js
const HOOKS = [
  {"on":"tool.before","tools":["write","patch"],"match":["write"],"command":"\"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \".opencode/agentstacks/runtime.lua\" \"t.hooks.guard\" \"1\" \"neutral\" \"A1 $TMPDIR\""}
];
```ms-04/scenario.json
[
  {"tool":"write","args":{"path":"ok.txt","content":"yes\n"}},
  {"text":"done-1"},
  {"text":"done-2"}
]
```
c1: exit=0 session=ses_df146aa44ffe9HC6qQMv7Cer5V
c2: exit=0 session=ses_df146aa44ffe9HC6qQMv7Cer5V
server pids running after stop: 0
user: "first"
user: "second\n\nagentstacks: airsl is not on PATH, so the agentstacks Lua hooks are not running. Install airsl, then run agentstacks doctor."
file ok.txt: yes
```

**Answer**: ms-03: prompt 1 got the notice; prompt 2 in the same session and prompt 3 in a new
session of the same process did not (prompt 3 still got its session.start message, so its
session was seen as new). The `may_block` guard on `blocked.txt` did not run, so the file was
written: with airsl missing every block is off. ms-04: with only a lua tool hook, prompt 1 got no
notice, the write ran the hook (exit 127), and prompt 2 got the notice.

## R5: an airsl on PATH without an execute bit (ms-126)

```text
$ sh ms-126.sh
$ ls -l <dir>/bin/airsl | cut -c1-10
-rw-r--r--
$ PATH=<dir>/bin:<PATH without airsl> /bin/sh -c '"airsl" "run" "--fail-open" "x.lua"' </dev/null; echo "exit=$?"
/bin/sh: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/ms-126/bin/airsl: Permission denied
exit=126
```

**Answer**: sh exits 126, not 127, so the shim gives no notice for it.

## Runner scripts (as run)

```ms-build.sh
#!/bin/sh
# Builds ms-shim.js: shim-01's header and HOOKS table, then the spec §4.5 fixed body.
S=$(cd "$(dirname "$0")" && pwd)
SPEC=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/spec.md
which -a airsl
{
  sed -n '1,/^];$/p' "$S/shim-01/proj/.opencode/plugins/agentstacks.js"
  echo
  sed -n '/^const ROOT = fileURL/,/^```$/p' "$SPEC" | sed '$d'
} > "$S/ms-shim.js"
tail -3 "$S/ms-shim.js"
wc -l "$S/ms-shim.js"
```

```ms-run.sh
#!/bin/sh
# ms-01: airsl not on PATH; ms-02: control with airsl on PATH. Same project, same probe shim.
S=$(cd "$(dirname "$0")" && pwd)
H=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh
NOAIRSL=$(echo "$PATH" | tr ':' '\n' | grep -v -e '/.local/bin$' -e '/.cargo/bin$' | paste -sd: -)
for d in ms-01 ms-02; do
  cp "$S/ms-shim.js" "$S/$d/proj/.opencode/plugins/agentstacks.js"
  printf 'alpha\n' > "$S/$d/proj/a.txt"
  rm -f "$S/$d/proj/ok.txt"
  cat > "$S/$d/scenario.json" <<'EOF'
[
  {"tool":"write","args":{"path":"ok.txt","content":"yes\n"}},
  {"tool":"read","args":{"path":"a.txt"}},
  {"text":"done-ms"}
]
EOF
done
echo '$ PATH=<PATH without ~/.local/bin and ~/.cargo/bin> command -v airsl; echo "found=$?"'
PATH="$NOAIRSL" command -v airsl; echo "found=$?"
echo '$ PATH=<same> /bin/sh -c '"'"'"airsl" "run" "--fail-open" "x.lua"'"'"' </dev/null; echo "exit=$?"'
PATH="$NOAIRSL" /bin/sh -c '"airsl" "run" "--fail-open" "x.lua"' </dev/null; echo "exit=$?"
echo '$ (airsl on PATH) airsl run --fail-open --policy confined missing.lua </dev/null; echo "exit=$?"'
(cd "$S/ms-02/proj" && airsl run --fail-open --policy confined missing.lua </dev/null; echo "exit=$?")
echo
echo "== ms-01 (no airsl on PATH)"
PATH="$NOAIRSL" sh "$H" "$S/ms-01" 18871 "hello"
tail -1 "$S/ms-01/out.txt"; node "$S/shim-bin/show.js" "$S/ms-01"
echo
echo "== ms-02 (airsl on PATH)"
sh "$H" "$S/ms-02" 18872 "hello"
tail -1 "$S/ms-02/out.txt"; node "$S/shim-bin/show.js" "$S/ms-02"
```

```ms-srv.sh
#!/bin/sh
# Several prompts in ONE OpenCode process: `opencode serve --port` plus `opencode run --server`.
# airsl is not on the server's PATH. ms-03: the shim-01 table (lua hooks on session.start and
# prompt.submit). ms-04: only a lua tool.before hook on write (no lua prompt or session.start hook).
S=$(cd "$(dirname "$0")" && pwd)
OC=/Users/hiraq/.opencode/bin/opencode
H=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness
NOAIRSL=$(echo "$PATH" | tr ':' '\n' | grep -v -e '/.local/bin$' -e '/.cargo/bin$' | paste -sd: -)

setup() { # setup <dir> : fresh copy of ms-01's project with the probe shim
  D=$S/$1
  rm -rf "$D"; mkdir -p "$D/proj"
  cp -R "$S/ms-01/proj/.opencode" "$D/proj/"
  cp "$S/ms-shim.js" "$D/proj/.opencode/plugins/agentstacks.js"
  printf 'alpha\n' > "$D/proj/a.txt"
}

serve() { # serve <dir> <mock port> <server port> ; then the client runs given as remaining lines
  D=$1; MP=$2; SP=$3
  mkdir -p "$D/home/.config/opencode"
  : > "$D/requests.jsonl"
  node "$H/mock.mjs" "$MP" "$D/scenario.json" "$D/requests.jsonl" &
  MOCK=$!
  export HOME="$D/home" XDG_CONFIG_HOME="$D/home/.config" XDG_DATA_HOME="$D/home/.local/share" \
    XDG_CACHE_HOME="$D/home/.cache" XDG_STATE_HOME="$D/home/.local/state" \
    OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_AUTOUPDATE=1 OPENCODE_PASSWORD=probe-pass
  cat > "$HOME/.config/opencode/opencode.json" <<EOF
{ "\$schema": "https://opencode.ai/config.json",
  "model": "mock/m",
  "providers": { "mock": { "name": "Mock", "package": "@opencode/ai/providers/openai-compatible",
    "settings": { "baseURL": "http://127.0.0.1:$MP/v1", "apiKey": "x" },
    "models": { "m": { "name": "M" } } } } }
EOF
  (cd "$D/proj" && PATH="$NOAIRSL" exec "$OC" serve --port "$SP" > "$D/serve.txt" 2>&1) &
  SERVER=$!
  sleep 4
}

client() { # client <dir> <server port> <label> <prompt> [flags]
  D=$1; SP=$2; L=$3; P=$4; shift 4
  (cd "$D/proj" && "$OC" run --server "http://127.0.0.1:$SP" --auto --format json "$@" "$P" > "$D/$L.txt" 2>&1; echo "exit=$?" >> "$D/$L.txt")
  echo "$L: $(tail -1 "$D/$L.txt") session=$(grep -o '"sessionID":"[^"]*"' "$D/$L.txt" | head -1 | cut -d'"' -f4)"
}

stop() {
  kill "$SERVER" "$MOCK" 2>/dev/null
  wait "$SERVER" 2>/dev/null
  echo "server pids running after stop: $(pgrep -f "serve --port $1" | wc -l | tr -d ' ')"
}

users() { # every distinct user message the mock received, in first-seen order
  node -e '
const seen = new Set();
for (const l of require("fs").readFileSync(process.argv[1], "utf8").trim().split("\n")) {
  const b = JSON.parse(l).body; if (!b || !b.messages || !b.tools || !b.tools.length) continue;
  for (const m of b.messages) if (m.role === "user") {
    const t = typeof m.content === "string" ? m.content : JSON.stringify(m.content);
    if (!seen.has(t)) { seen.add(t); console.log("user: " + JSON.stringify(t)); }
  }
}' "$1/requests.jsonl"
}

echo '== ms-03: shim-01 table; prompt 1 and 2 in one session, prompt 3 in a new session; one server process'
setup ms-03
cat > "$S/ms-03/scenario.json" <<'EOF'
[
  {"tool":"write","args":{"path":"blocked.txt","content":"no\n"}},
  {"text":"done-1"},
  {"text":"done-2"},
  {"text":"done-3"}
]
EOF
echo '```ms-03/scenario.json'; cat "$S/ms-03/scenario.json"; echo '```'
serve "$S/ms-03" 18881 18891
client "$S/ms-03" 18891 c1 "first"
SID=$(grep -o '"sessionID":"[^"]*"' "$S/ms-03/c1.txt" | head -1 | cut -d'"' -f4)
client "$S/ms-03" 18891 c2 "second" --session "$SID"
client "$S/ms-03" 18891 c3 "third"
stop 18891
users "$S/ms-03"
echo "file blocked.txt: $(cat "$S/ms-03/proj/blocked.txt" 2>&1)"
echo

echo '== ms-04: only a lua tool.before hook on write; prompt 1 writes, prompt 2 in the same session'
setup ms-04
node -e '
const f = process.argv[1], fs = require("fs");
const t = fs.readFileSync(f, "utf8");
const lines = t.split("\n");
const keep = lines.filter((l) => !(l.startsWith("  {\"on\":") && !l.includes("\"on\":\"tool.before\"")));
fs.writeFileSync(f, keep.join("\n"));' "$S/ms-04/proj/.opencode/plugins/agentstacks.js"
sed -i '' 's/^\(  {"on":"tool.before".*}\),$/\1/' "$S/ms-04/proj/.opencode/plugins/agentstacks.js"
echo '$ sed -n "/^const HOOKS/,/^];/p" ms-04/proj/.opencode/plugins/agentstacks.js'
sed -n '/^const HOOKS/,/^];/p' "$S/ms-04/proj/.opencode/plugins/agentstacks.js"
cat > "$S/ms-04/scenario.json" <<'EOF'
[
  {"tool":"write","args":{"path":"ok.txt","content":"yes\n"}},
  {"text":"done-1"},
  {"text":"done-2"}
]
EOF
echo '```ms-04/scenario.json'; cat "$S/ms-04/scenario.json"; echo '```'
serve "$S/ms-04" 18882 18892
client "$S/ms-04" 18892 c1 "first"
SID=$(grep -o '"sessionID":"[^"]*"' "$S/ms-04/c1.txt" | head -1 | cut -d'"' -f4)
client "$S/ms-04" 18892 c2 "second" --session "$SID"
stop 18892
users "$S/ms-04"
echo "file ok.txt: $(cat "$S/ms-04/proj/ok.txt" 2>&1)"
```

```ms-126.sh
#!/bin/sh
# What sh exits with when the airsl on PATH is a file without an execute bit.
S=$(cd "$(dirname "$0")" && pwd)
D=$S/ms-126
rm -rf "$D"; mkdir -p "$D/bin"
printf '#!/bin/sh\necho hi\n' > "$D/bin/airsl"
chmod 644 "$D/bin/airsl"
NOAIRSL=$(echo "$PATH" | tr ':' '\n' | grep -v -e '/.local/bin$' -e '/.cargo/bin$' | paste -sd: -)
echo '$ ls -l <dir>/bin/airsl | cut -c1-10'
ls -l "$D/bin/airsl" | cut -c1-10
echo '$ PATH=<dir>/bin:<PATH without airsl> /bin/sh -c '"'"'"airsl" "run" "--fail-open" "x.lua"'"'"' </dev/null; echo "exit=$?"'
PATH="$D/bin:$NOAIRSL" /bin/sh -c '"airsl" "run" "--fail-open" "x.lua"' </dev/null 2>&1; echo "exit=$?"
```

```pe-run.sh
#!/bin/sh
# pe-01: an agent file with permissions written as airsstack.json.encode writes them (sorted keys,
# no spaces), called as a subagent. Same scenario as agents-05 (probes/agents-results.md P3).
S=$(cd "$(dirname "$0")" && pwd)
H=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh
D=$S/pe-01
rm -rf "$D"; mkdir -p "$D/proj/.opencode/agents"
{
  echo '---'
  echo 'description: "Read-only: read and grep only"'
  echo 'mode: "subagent"'
  printf 'permissions: '
  (cd "$S/pe" && airsl run --policy confined enc.lua | sed -n 's/^ro:    //p')
  echo '---'
  echo 'You are RO-SYSTEM-PROMPT.'
} > "$D/proj/.opencode/agents/ro.md"
cp "$S/agents-05/scenario.json" "$D/scenario.json"
echo '```pe-01/proj/.opencode/agents/ro.md'; cat "$D/proj/.opencode/agents/ro.md"; echo '```'
echo '```pe-01/scenario.json'; cat "$D/scenario.json"; echo '```'
echo
echo '```text'
echo '$ sh probes/harness/run.sh <scratchpad>/pe-01 18873 "hello"; tail -1 pe-01/out.txt; <per request: system prompt head, tools offered, last tool result>'
sh "$H" "$D" 18873 "hello"
tail -1 "$D/out.txt"
node -e '
const fs = require("fs");
for (const l of fs.readFileSync(process.argv[1], "utf8").trim().split("\n")) {
  const b = JSON.parse(l).body; if (!b || !b.tools || !b.tools.length) continue;
  const sys = b.messages.filter((m) => m.role === "system").map((m) => String(m.content)).join(" ");
  const last = b.messages[b.messages.length - 1];
  console.log((sys.includes("RO-SYSTEM-PROMPT") ? "ro    " : "parent") + " tools=" + b.tools.map((t) => t.function.name).sort().join(",") +
    " last=" + JSON.stringify(String(last.content)).slice(0, 150));
}' "$D/requests.jsonl"
echo '```'
```

```nm-run.sh
#!/bin/sh
# nm-01: does the core accept a native hook with `match = []`, and what reaches the extension?
S=$(cd "$(dirname "$0")" && pwd)
R=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode
A=$R/target/debug/agentstacks
D=$S/nm-01
rm -rf "$D"; mkdir -p "$D/ext" "$D/packs"
cp -R "$R/extensions/claude" "$D/ext/claude"
cp -R "$R/crates/agentstacks-cli/tests/fixtures/project/packs/demo" "$D/packs/demo"
mkdir -p "$D/packs/demo/native/claude"
cat > "$D/packs/demo/native/claude/hooks.toml" <<'EOF'
[[hooks]]
kind = "message"
event = "PostToolUse"
match = []
text = "EMPTY-MATCH"
EOF
cd "$D" || exit 1
git init -q
echo '```packs/demo/native/claude/hooks.toml'; cat packs/demo/native/claude/hooks.toml; echo '```'
echo '```text'
echo '$ agentstacks --version'; "$A" --version
echo '$ agentstacks init --tool ext/claude --pack packs/demo; echo "exit=$?"'
"$A" init --tool ext/claude --pack packs/demo 2>&1; echo "exit=$?"
echo '$ agentstacks plan; echo "exit=$?"'
"$A" plan 2>&1; echo "exit=$?"
echo '$ agentstacks apply --yes; echo "exit=$?"'
"$A" apply --yes 2>&1 | tail -3; echo "exit=$?"
echo '$ grep -n -B2 -A6 EMPTY-MATCH .claude/settings.json'
grep -n -B2 -A6 EMPTY-MATCH .claude/settings.json
echo '```'
```
