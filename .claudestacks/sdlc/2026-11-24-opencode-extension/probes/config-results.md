# Probe results: commands, shell env, project config (OpenCode v2.4.1)

Probe dirs: `<scratchpad>/config-01` … `config-06`, helper scripts in `<scratchpad>/config-tools/`.
`<scratchpad>` = `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad`.
Ports used: 18790–18796.

## Helper scripts (setup shared by several probes)

`harness/run.sh` always starts OpenCode in `<probe-dir>/proj`. These three scripts reuse its
isolation exactly and differ only as their header says.

```sh <scratchpad>/config-tools/oc.sh
#!/bin/sh
# usage: oc.sh <probe-dir> <cwd-relative-to-proj> <opencode args...>
# Runs the opencode CLI with the same isolation as harness/run.sh (HOME/XDG under <probe-dir>/home).
set -u
DIR=$1; SUB=$2; shift 2
export HOME="$DIR/home" XDG_CONFIG_HOME="$DIR/home/.config" XDG_DATA_HOME="$DIR/home/.local/share" \
  XDG_CACHE_HOME="$DIR/home/.cache" XDG_STATE_HOME="$DIR/home/.local/state" \
  OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_AUTOUPDATE=1
cd "$DIR/proj/$SUB" || exit 1
/Users/hiraq/.opencode/bin/opencode "$@"
echo "exit=$?"
```

```sh <scratchpad>/config-tools/run-sub.sh
#!/bin/sh
# usage: run-sub.sh <probe-dir> <port> <subdir-of-proj> "<prompt>" [extra flags]
# Same as harness/run.sh, but starts opencode in <probe-dir>/proj/<subdir> instead of proj/.
set -u
DIR=$1; PORT=$2; SUB=$3; PROMPT=$4; shift 4
H=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness
mkdir -p "$DIR/home"
: > "$DIR/requests.jsonl"
node "$H/mock.mjs" "$PORT" "$DIR/scenario.json" "$DIR/requests.jsonl" &
MOCK=$!
sleep 1
export HOME="$DIR/home" XDG_CONFIG_HOME="$DIR/home/.config" XDG_DATA_HOME="$DIR/home/.local/share" \
  XDG_CACHE_HOME="$DIR/home/.cache" XDG_STATE_HOME="$DIR/home/.local/state" \
  OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_AUTOUPDATE=1 MOCK_PORT="$PORT"
cd "$DIR/proj/$SUB" || exit 1
mkdir -p "$HOME/.config/opencode"
printf '%s\n' '{ "$schema": "https://opencode.ai/config.json",' \
  '  "model": "mock/m",' \
  '  "providers": { "mock": { "name": "Mock", "package": "@opencode/ai/providers/openai-compatible",' \
  "    \"settings\": { \"baseURL\": \"http://127.0.0.1:$PORT/v1\", \"apiKey\": \"x\" }," \
  '    "models": { "m": { "name": "M" } } } } }' > "$HOME/.config/opencode/opencode.json"
/Users/hiraq/.opencode/bin/opencode run --standalone --auto --format json "$@" "$PROMPT" > "$DIR/out.txt" 2> "$DIR/err.txt"
echo "exit=$?" >> "$DIR/out.txt"
kill $MOCK 2>/dev/null
```

```sh <scratchpad>/config-tools/api-mock.sh
#!/bin/sh
# usage: api-mock.sh <probe-dir> <port> <opencode args...>
# Starts the mock model on <port> (global config points at it, same as harness/run.sh), then runs
# the opencode CLI isolated in <probe-dir>/proj, waits 8s for any model request, stops the mock.
# Appends model requests to <probe-dir>/requests-api.jsonl.
set -u
DIR=$1; PORT=$2; shift 2
H=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness
: > "$DIR/requests-api.jsonl"
node "$H/mock.mjs" "$PORT" "$DIR/scenario.json" "$DIR/requests-api.jsonl" &
MOCK=$!
sleep 1
export HOME="$DIR/home" XDG_CONFIG_HOME="$DIR/home/.config" XDG_DATA_HOME="$DIR/home/.local/share" \
  XDG_CACHE_HOME="$DIR/home/.cache" XDG_STATE_HOME="$DIR/home/.local/state" \
  OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_AUTOUPDATE=1
printf '%s\n' '{ "$schema": "https://opencode.ai/config.json",' \
  '  "model": "mock/m",' \
  '  "providers": { "mock": { "name": "Mock", "package": "@opencode/ai/providers/openai-compatible",' \
  "    \"settings\": { \"baseURL\": \"http://127.0.0.1:$PORT/v1\", \"apiKey\": \"x\" }," \
  '    "models": { "m": { "name": "M" } } } } }' > "$HOME/.config/opencode/opencode.json"
cd "$DIR/proj" || exit 1
/Users/hiraq/.opencode/bin/opencode "$@"
echo "exit=$?"
sleep 8
kill $MOCK 2>/dev/null
```

**Warning for every probe agent:** `opencode debug config`, `session list` and `session export`
without `--standalone` start a **persistent background `opencode serve --service`** on 127.0.0.1:49374
that outlives the CLI. My first `debug config` (config-01) did this. The service used config-01's
isolated HOME, and I killed it (PID 47647). Its later `debug config` call failed with:

```text
Error: Managed service port 49374 on 127.0.0.1 is already in use by PID 47647 (/Users/hiraq/.opencode/bin/opencode serve --service). Configure another port with `opencode service set port <port>` and start the service again.
```

```text
$ ps eww -p 47647 | tr ' ' '\n' | grep -E '^(HOME|XDG_CONFIG_HOME)='
HOME=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home
XDG_CONFIG_HOME=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home/.config
```

All later non-`run` commands pass `--standalone`.

---

## P1: Commands: discovery, frontmatter, `$ARGUMENTS`/`$N`, `` !`cmd` ``, non-interactive run, wrapping a skill

### Setup (`config-01`, `git init` in `proj/`)

```md proj/.opencode/commands/echo.md
---
description: Echo probe command
agent: build
subagent: false
unknownkey: whatever
---
ALL=[$ARGUMENTS] ONE=[$1] TWO=[$2]
SHELL=[!`echo injected-$1; pwd`]
Load the skill named demo with the skill tool, then follow it.
```

```md proj/.opencode/commands/badtype.md
---
description: Bad type probe
subagent: notabool
---
BADTYPE body
```

```md proj/.opencode/skills/demo/SKILL.md
---
name: demo
description: Demo skill for the command probe
---
DEMO SKILL BODY sid=${OPENCODE_SESSION_ID} args=$ARGUMENTS
```

```json scenario.json (run 1)
[{"text":"ok"}]
```

### P1a: `opencode run "/echo a b"` sends the text literally

Command:

```sh
sh $C/probes/harness/run.sh $P 18790 "/echo a b"
node -e '
const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").map(JSON.parse);
L.forEach((r,i)=>{const b=r.body; if(!b.tools||!b.tools.length) return; console.log("req",i,"tools:",b.tools.map(t=>t.function.name).join(","));
 b.messages.filter(m=>m.role!=="system").forEach(m=>console.log(m.role, JSON.stringify(m.content).slice(0,400)));})' $P/requests.jsonl
```

Output (`out.txt` ended `exit=0`, `err.txt` empty):

```text
req 1 tools: edit,generate_image,generate_video,glob,grep,question,read,shell,skill,subagent,webfetch,websearch,write,execute
user "/echo a b"
```

`opencode run --help` offers no command flag:

```text
FLAGS
  --continue, -c            Continue the last session
  --session, -s string      Session ID to continue, or to create if it does not exist
  --fork                    Fork the session before continuing
  --worktree, -w [=name]    Start a new session in a new git worktree, optionally with this name
  --model, -m string        Model to use in the format provider/model#variant
  --agent string            Agent to use
  --format choice           Output format (choices: default, json)
  --file, -f string         File to attach to the message
  --title string            Session title
  --archived                Create the session archived so it stays out of session lists (useful for scripted or CI runs)
  --thinking                Show thinking blocks
  --auto                    Auto-approve permissions that are not explicitly denied
```

Source: `run` sends the message as a plain prompt (`probes/src/packages/cli/src/run/noninteractive.ts:220`
`text: input.message`). Only the TUI turns `/name args` into a command call
(`probes/src/packages/tui/src/component/prompt/index.tsx:1324-1329`, `client.api.session.command`),
which is the HTTP endpoint `POST /api/session/:sessionID/command`
(`probes/src/packages/protocol/src/groups/session.ts:364`).

### P1b: invoking the command non-interactively through `opencode api … session.command`

Command (session `ses_df19a8e7fffe97Ru8snWhvlsjn` was created by P1a):

```sh
sh $T/api-mock.sh $P 18791 api --standalone session.command --param sessionID=ses_df19a8e7fffe97Ru8snWhvlsjn -d '{"name":"echo","text":"a \"b c\" d"}'
node -e '
const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").filter(Boolean).map(JSON.parse);
L.forEach((r,i)=>{const b=r.body; if(!b.tools||!b.tools.length) return; console.log("req",i);
 b.messages.filter(m=>m.role!=="system").forEach(m=>console.log(m.role, JSON.stringify(m.content)));})' $P/requests-api.jsonl
```

Output:

```text
exit=0
```

```text
req 0
user "/echo a b"
assistant "ok"
user "ALL=[a \"b c\" d] ONE=[a] TWO=[b c d]\nSHELL=[injected-a\n/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/proj\n]\nLoad the skill named demo with the skill tool, then follow it."
```

The `!`-block ran with cwd = the session's location directory (`pwd` printed `proj`). Source:
`probes/src/packages/core/src/config/plugin/command.ts:202` (`cwd: services.location.directory`).
The argument rules are at lines 189 and 227, the shell regex at line 229, and the docs at
`probes/docs/commands.mdx:100-124,144-152`.

The private server exits when `api` returns, so the model's reply to an api-invoked command is never
stored. The export below shows the two api-invoked prompts (`ALL=[a…` and `ALL=[x y]…`) with no
assistant message after them. A later `opencode run --session <id>` carries them as context.
`api --standalone` is good for proving the expansion, but it is no way to *run* a command.

### P1c: unknown key, wrong type

The `unknownkey: whatever` command loaded and ran (P1b). The `subagent: notabool` command was skipped:

```sh
sh $T/api-mock.sh $P 18791 api --standalone session.command --param sessionID=ses_df19a8e7fffe97Ru8snWhvlsjn -d '{"name":"badtype","text":""}'
grep -rh 'command file skipped' $P/home/.local/share/opencode/log/ | head -2 | cut -c1-700
```

```text
{"_tag":"CommandNotFoundError","command":"badtype","message":"Command not found: badtype"}
HTTP 404 Not Found
exit=1
```

```text
timestamp=2026-11-24T16:30:59.229Z level=WARN run=f113fc32 message="command file skipped" filepath=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/proj/.opencode/commands/badtype.md reason=frontmatter issue="Expected boolean\n  at [\"subagent\"]"
timestamp=2026-11-24T16:31:59.745Z level=WARN run=2a100b4f message="command file skipped" filepath=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/proj/.opencode/commands/badtype.md reason=frontmatter issue="Expected boolean\n  at [\"subagent\"]"
```

The accepted keys are `template` (the body), `description`, `agent`, `model`, `subagent` and
`subtask` (`probes/src/packages/schema/src/config/command.ts:7-14`). The skip warning is at
`probes/src/packages/core/src/config/plugin/command.ts:150`. It goes only to the log file, not to
stderr.

### P1d: a command body that tells the model to load a skill

A second api invocation (`{"name":"echo","text":"x y"}`), then the session was continued with
`opencode run`, and the model (scenario) followed the instruction by calling the `skill` tool:

```json scenario.json (P1d)
[{"tool":"skill","args":{"id":"demo"}},{"text":"loaded"}]
```

```sh
sh $C/probes/harness/run.sh $P 18790 "continue" --session ses_df19a8e7fffe97Ru8snWhvlsjn
node -e '
const fs=require("fs");const L=fs.readFileSync(process.argv[1],"utf8").trim().split("\n").filter(Boolean).map(JSON.parse).filter(r=>r.body.tools&&r.body.tools.length);
console.log("tool requests:",L.length); const r=L[L.length-1];
 r.body.messages.filter(m=>m.role!=="system").forEach(m=>console.log(m.role, JSON.stringify(m.content??m.tool_calls)));' $P/requests.jsonl
```

```text
exit=0
tool requests: 2
user "/echo a b"
assistant "ok"
user "ALL=[a \"b c\" d] ONE=[a] TWO=[b c d]\nSHELL=[injected-a\n/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/proj\n]\nLoad the skill named demo with the skill tool, then follow it."
user "ALL=[x y] ONE=[x] TWO=[y]\nSHELL=[injected-x\n/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/proj\n]\nLoad the skill named demo with the skill tool, then follow it."
user "continue"
assistant [{"id":"call_1","type":"function","function":{"name":"skill","arguments":"{\"id\":\"demo\"}"}}]
tool "<skill_content name=\"demo\">\n# Skill: demo\n\nDEMO SKILL BODY sid=${OPENCODE_SESSION_ID} args=$ARGUMENTS\n\nBase directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/proj/.opencode/skills/demo\nRelative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.\nNote: file list is sampled.\n\n<skill_files>\n</skill_files>\n</skill_content>"
```

Related: the protocol also has `POST …/skill` (`experimental.session.skill`, "Activate a skill for
a session by appending a skill message and resuming execution",
`probes/src/packages/protocol/src/groups/session.ts:396`).

### Answer

`.opencode/commands/<name>.md` is discovered with no config. `description`, `agent`, `model` and
`subagent` are accepted, and an unknown key is ignored. A wrong-typed key skips the whole file, with
a log-only WARN and `Command not found: <name>` when invoked. `$ARGUMENTS` becomes the raw string,
`$N` takes one shell-split word, and the highest `$N` takes the rest. `` !`cmd` `` output is inlined,
run in the session directory. `opencode run "/echo a b"` does **not** run the command: the model
receives the literal `/echo a b`. Commands run only from the TUI or `POST /api/session/:id/command`.
The expanded body is an ordinary user prompt, so a body telling the model to call the `skill` tool
works.

---

## P2: Shell tool environment, cwd and shell

### Setup (`config-02`)

```text
proj/                       (no git)
  .opencode/skills/aboveskill/SKILL.md
  repo/                     (git init + one empty commit; root commit bfa89d887b51ce22f426fcb666c973a392813904)
    .opencode/skills/rootskill/SKILL.md
    .opencode/agents/rootagent.md
    .opencode/commands/rootcmd.md
    .opencode/plugins/marker.js
    pkg/.opencode/skills/pkgskill/SKILL.md
    pkg/sub/                (start dir for run B)
```

```md proj/.opencode/skills/aboveskill/SKILL.md
---
name: aboveskill
description: Skill above the git root
---
ABOVE SKILL BODY
```

```md proj/repo/.opencode/skills/rootskill/SKILL.md
---
name: rootskill
description: Skill at the git root
---
ROOT SKILL BODY
```

```md proj/repo/pkg/.opencode/skills/pkgskill/SKILL.md
---
name: pkgskill
description: Skill in a subdirectory between git root and start dir
---
PKG SKILL BODY
```

```md proj/repo/.opencode/agents/rootagent.md
---
description: Agent at the git root
mode: subagent
---
ROOT AGENT BODY
```

```md proj/repo/.opencode/commands/rootcmd.md
---
description: Command at the git root
---
ROOT COMMAND $ARGUMENTS
```

```js proj/repo/.opencode/plugins/marker.js
import fs from "node:fs"

export default {
  id: "marker",
  setup() {
    fs.appendFileSync(
      "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/marker.log",
      `marker plugin setup cwd=${process.cwd()}\n`,
    )
  },
}
```

```json scenario.json
[
  {"tool": "shell", "args": {"command": "env | sort; echo PWD_IS=$(pwd); echo ARG0=$0; ps -o comm= -p $$; echo ZSH_VERSION=${ZSH_VERSION:-none} BASH_VERSION=${BASH_VERSION:-none}; echo DEFAULT_FORM=${NOT_SET_PROBE_VAR:-fallback}; echo SID=${OPENCODE_SESSION_ID:-unset}; git rev-parse --show-toplevel"}},
  {"text": "done-shell"}
]
```

### Command

```sh
sh $T/run-sub.sh $P 18792 repo "probe-A"           # run A: start at the git root (copied to A-*)
sh $T/run-sub.sh $P 18792 repo/pkg/sub "probe-B"   # run B: start two levels below it (copied to B-*)
grep -o '"sessionID":"ses_[^"]*"' $P/out.txt | sort -u
```

```text
"sessionID":"ses_df19716d0ffe7s4lCZIcdkIw4I"      (run A)
"sessionID":"ses_df196cf37ffeh6sOQhqTmF0hWw"      (run B)
```

Both ended `exit=0` with an empty `err.txt`. The full `env | sort` output holds the host's secrets
(it inherits the parent environment), so it is shown filtered: the command prints only the lines
named in the regex.

```sh
cd $P && node -e '
const fs=require("fs");
for (const f of process.argv.slice(1)) {
 const L=fs.readFileSync(f,"utf8").trim().split("\n").map(JSON.parse).filter(r=>r.body.tools&&r.body.tools.length);
 const r=L[L.length-1]; const tool=r.body.messages.find(m=>m.role==="tool").content;
 console.log("== "+f);
 console.log(tool.split("\n").filter(l=>/^(OPENCODE|PWD|OLDPWD|SHELL=|HOME=|PWD_IS|ARG0|\/bin|ZSH_VERSION|DEFAULT_FORM|SID=|\/private)|PROJECT|WORKSPACE|ROOT/.test(l)).join("\n"));
 const sys=JSON.stringify(r.body.messages.filter(m=>m.role==="system").map(m=>m.content));
 const skillDesc=r.body.tools.find(t=>t.function.name==="skill").function.description;
 for (const s of ["aboveskill","rootskill","pkgskill"]) console.log(s, "in system:", sys.includes(s), "in skill tool desc:", skillDesc.includes(s));
}' A-requests.jsonl B-requests.jsonl
```

### Output

```text
== A-requests.jsonl
HOME=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/home
OLDPWD=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo
OPENCODE_DISABLE_AUTOCOMPACT=true
OPENCODE_DISABLE_AUTOUPDATE=1
OPENCODE_DISABLE_MODELS_FETCH=1
OPENCODE_DISABLE_PRUNE=true
OPENCODE_SESSION_ID=ses_df19716d0ffe7s4lCZIcdkIw4I
OPENCODE_TERMINAL=1
OPENCODE=1
PWD=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo
SHELL=/bin/zsh
PWD_IS=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo
ARG0=/bin/zsh
/bin/zsh
ZSH_VERSION=5.9 BASH_VERSION=none
DEFAULT_FORM=fallback
SID=ses_df19716d0ffe7s4lCZIcdkIw4I
/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo
aboveskill in system: true in skill tool desc: false
rootskill in system: true in skill tool desc: false
pkgskill in system: false in skill tool desc: false
== B-requests.jsonl
HOME=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/home
OLDPWD=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo/pkg/sub
OPENCODE_DISABLE_AUTOCOMPACT=true
OPENCODE_DISABLE_AUTOUPDATE=1
OPENCODE_DISABLE_MODELS_FETCH=1
OPENCODE_DISABLE_PRUNE=true
OPENCODE_SESSION_ID=ses_df196cf37ffeh6sOQhqTmF0hWw
OPENCODE_TERMINAL=1
OPENCODE=1
PWD=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo/pkg/sub
SHELL=/bin/zsh
PWD_IS=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo/pkg/sub
ARG0=/bin/zsh
/bin/zsh
ZSH_VERSION=5.9 BASH_VERSION=none
DEFAULT_FORM=fallback
SID=ses_df196cf37ffeh6sOQhqTmF0hWw
/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo
aboveskill in system: true in skill tool desc: false
rootskill in system: true in skill tool desc: false
pkgskill in system: true in skill tool desc: false
```

Note: the harness sets `OPENCODE_DISABLE_AUTOUPDATE` and `OPENCODE_DISABLE_MODELS_FETCH`. This
probe did not trace where `OPENCODE_DISABLE_AUTOCOMPACT`, `OPENCODE_DISABLE_PRUNE` and
`OPENCODE_TERMINAL` come from. No variable named
`*PROJECT*`, `*WORKSPACE*` or `*ROOT*` exists in the unfiltered dump. The regex would have printed
it.

The model sees the project root only as text, in the system prompt's `<env>` block:

```sh
cd $S && node -e '
const fs=require("fs");
for (const f of process.argv.slice(1)) {
const L=fs.readFileSync(f,"utf8").trim().split("\n").map(JSON.parse).filter(r=>r.body.tools&&r.body.tools.length);
const sys=L[0].body.messages.filter(m=>m.role==="system").map(m=>typeof m.content==="string"?m.content:JSON.stringify(m.content)).join("\n");
const m=sys.match(/<env>[\s\S]*?<\/env>/); console.log("== "+f+"\n"+(m?m[0]:"no env block"));}' config-02/A-requests.jsonl config-02/B-requests.jsonl config-01/requests-run-1.jsonl
```

```text
== config-02/A-requests.jsonl
<env>
  Working directory: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo
  Workspace root folder: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo
  Is directory a git repo: yes
  Platform: darwin
  Prefer /private/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/opencode over generic system temporary directories such as /tmp; it is pre-created and approved for external access.
</env>
== config-02/B-requests.jsonl
<env>
  Working directory: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo/pkg/sub
  Workspace root folder: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo
  Is directory a git repo: yes
  Platform: darwin
  Prefer /private/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/opencode over generic system temporary directories such as /tmp; it is pre-created and approved for external access.
</env>
== config-01/requests-run-1.jsonl
<env>
  Working directory: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/proj
  Workspace root folder: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/proj
  Is directory a git repo: yes
  Platform: darwin
  Prefer /private/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/opencode over generic system temporary directories such as /tmp; it is pre-created and approved for external access.
</env>
```

The shell tool's own description (from `requests.jsonl`) says: `"Execute a shell command and return its output. Commands run on macOS using zsh. … The current session ID is available in the OPENCODE_SESSION_ID environment variable."`

Source:
- `probes/src/packages/core/src/tool/plugin/shell.ts:483` sets `OPENCODE_SESSION_ID = context.sessionID`.
  The same function also sets `AGENT=1`, `OPENCODE=1` and `AI_AGENT ||= "opencode"`.
- `shell.ts:457`: cwd = `input.workdir ?? workingDirectory.absolute` (the session directory).
- `shell.ts:313`: the tool resolves its shell with `priority: "compat"`.
  `probes/src/packages/core/src/shell/select.ts:17,19` mark `fish` and `nu` as `deny`.
  `select.ts:190`: the config `shell` key wins, then `$SHELL` (`:182`). The darwin fallback is
  `/bin/zsh` (`:142`), otherwise bash, otherwise `/bin/sh`. zsh runs as `zsh +o nomatch -c <cmd>`
  (`:176`).

### Answer

`OPENCODE_SESSION_ID` is set in every shell-tool command and equals the session id. No environment
variable holds the project directory. The shell's cwd is the directory OpenCode was started in
(`repo/pkg/sub` in run B), not the git root. The git root reaches the model only as text in the
system prompt (`Workspace root folder: …`). The shell is the config `shell`, else `$SHELL` (fish and
nu are refused), else `/bin/zsh` on macOS, so a POSIX-family shell always runs and
`${VAR:-default}` works (`DEFAULT_FORM=fallback`).

---

## P3: Is `${…}` (or `$ARGUMENTS`) in a skill, agent or command body substituted?

### Setup (`config-03`; skill case reused from `config-01`)

```md proj/.opencode/agents/probe.md
---
description: Substitution probe agent
mode: primary
---
AGENT BODY sid=${OPENCODE_SESSION_ID} home=${HOME} envform={env:HOME} args=$ARGUMENTS one=$1
```

```md proj/.opencode/commands/subst.md
---
description: Substitution probe command
---
CMD sid=${OPENCODE_SESSION_ID} home=${HOME} envform={env:HOME} args=$ARGUMENTS
```

```json scenario.json
[{"text":"agent-ok"}]
```

### Command and output

Agent body (system prompt):

```sh
sh $C/probes/harness/run.sh $P 18793 "probe-agent" --agent probe
cd $P && node -e '
const fs=require("fs");const L=fs.readFileSync("requests.jsonl","utf8").trim().split("\n").map(JSON.parse).filter(r=>r.body.tools&&r.body.tools.length);
const sys=L[0].body.messages.filter(m=>m.role==="system").map(m=>typeof m.content==="string"?m.content:JSON.stringify(m.content)).join("\n");
sys.split("\n").filter(l=>l.includes("AGENT BODY")).forEach(l=>console.log(l));'
```

```text
exit=0
"sessionID":"ses_df1946f2effeW3ddTBfOVJRfju"
AGENT BODY sid=${OPENCODE_SESSION_ID} home=${HOME} envform={env:HOME} args=$ARGUMENTS one=$1
```

Command body:

```sh
sh $T/api-mock.sh $P 18793 api --standalone session.command --param sessionID=ses_df1946f2effeW3ddTBfOVJRfju -d '{"name":"subst","text":"q r"}'
cd $P && node -e '
const fs=require("fs");const L=fs.readFileSync("requests-api.jsonl","utf8").trim().split("\n").filter(Boolean).map(JSON.parse).filter(r=>r.body.tools&&r.body.tools.length);
console.log("requests:",L.length); const ms=L[0].body.messages; console.log(JSON.stringify(ms[ms.length-1]));'
```

```text
exit=0
requests: 1
{"role":"user","content":"CMD sid=${OPENCODE_SESSION_ID} home=${HOME} envform={env:HOME} args=q r"}
```

Skill body (from P1d, the `skill` tool result): `DEMO SKILL BODY sid=${OPENCODE_SESSION_ID} args=$ARGUMENTS`.

### Answer

OpenCode passes `${OPENCODE_SESSION_ID}`, `${HOME}`, `{env:HOME}`, `$ARGUMENTS` and `$1` through
literally in skill bodies and agent bodies. In command bodies only `$ARGUMENTS`, `$N` and
`` !`…` `` are expanded, and `${…}` and `{env:…}` stay literal. `{env:…}` substitution applies to
JSON config files only (`probes/docs/config.mdx:565-587`).

---

## P4: Project config: is `opencode.json` needed? Root vs `.opencode/` precedence; unknown key; invalid JSON

### P4a: no `opencode.json` in the project

`config-02` has no project `opencode.json` at any level (only the isolated global one). Evidence that
skills, agents, commands and plugins still load:

- skills: `rootskill` and `pkgskill` appear in `<available_skills>` (P2 output above).
- plugins: `marker.js` ran in both runs:

```sh
cat $P/marker.log
```

```text
marker plugin setup cwd=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo
marker plugin setup cwd=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo/pkg/sub
```

- agents: `debug agents` (run from `repo/pkg/sub`) lists `rootagent`:

```sh
sh $T/oc.sh $P repo/pkg/sub debug agents --standalone > $P/debug-agents-sub.txt 2>&1
grep -n 'rootagent\|exit=' $P/debug-agents-sub.txt | head
```

```text
313:    "id": "rootagent",
314:    "name": "rootagent",
456:exit=0
```

- commands: `config-01` has no project `opencode.json`, and `/echo` ran (P1b).

### P4b: precedence of `opencode.json` vs `.opencode/opencode.json` (`config-04`)

```json proj/opencode.json
{
  "$schema": "https://opencode.ai/config.json",
  "agents": {
    "probe": {
      "description": "precedence probe",
      "mode": "primary",
      "system": "SYSTEM_FROM_ROOT_OPENCODE_JSON"
    },
    "rootonly": {
      "description": "only in root opencode.json",
      "mode": "subagent",
      "system": "ROOT_ONLY"
    }
  }
}
```

```json proj/.opencode/opencode.json
{
  "$schema": "https://opencode.ai/config.json",
  "agents": {
    "probe": {
      "system": "SYSTEM_FROM_DOT_OPENCODE_JSON"
    }
  }
}
```

```sh
sh $C/probes/harness/run.sh $S/config-04 18794 "probe-precedence" --agent probe
cd $S/config-04 && node -e '
const fs=require("fs");const L=fs.readFileSync("requests.jsonl","utf8").trim().split("\n").map(JSON.parse).filter(r=>r.body.tools&&r.body.tools.length);
const sys=L[0].body.messages.filter(m=>m.role==="system").map(m=>typeof m.content==="string"?m.content:JSON.stringify(m.content)).join("\n");
console.log("ROOT:",sys.includes("SYSTEM_FROM_ROOT_OPENCODE_JSON"),"DOT:",sys.includes("SYSTEM_FROM_DOT_OPENCODE_JSON")); sys.split("\n").filter(l=>l.includes("SYSTEM_FROM")).forEach(l=>console.log(l));'
sh $S/config-tools/oc.sh $S/config-04 . debug config --standalone 2>&1 | grep -n '"path"\|SYSTEM_FROM\|exit='
```

```text
ROOT: false DOT: true
SYSTEM_FROM_DOT_OPENCODE_JSON
4:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-04/home/.config/opencode/opencode.json",
30:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-04/home/.config/opencode"
34:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-04/proj/opencode.json",
39:          "system": "SYSTEM_FROM_ROOT_OPENCODE_JSON",
53:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-04/proj/.opencode/opencode.json",
58:          "system": "SYSTEM_FROM_DOT_OPENCODE_JSON"
65:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-04/proj/.opencode"
```

The run used `--agent probe` with `mode: primary`, which is set only in the root file, and it ran
(`exit=0`). So the two files merge per key, and `.opencode/opencode.json` wins on `system`. The docs
agree: `probes/docs/config.mdx:79-83` ("every discovered `.opencode` config overrides every direct
config"). The source is `probes/src/packages/core/src/config/discovery.ts:154-162` (`direct` vs
`project` lists).

### P4c: unknown top-level key (`config-05`)

```json proj/opencode.json
{
  "$schema": "https://opencode.ai/config.json",
  "agentstacks_probe_unknown": true
}
```

```sh
sh $C/probes/harness/run.sh $S/config-05 18795 "probe-unknown-key"
sh $S/config-tools/oc.sh $S/config-05 . debug config --standalone > $S/config-05/debug-config.txt 2>&1
sed -n '/proj/,$p' $S/config-05/debug-config.txt | head -30
grep -rh 'diagnostic\|agentstacks_probe' $S/config-05/home/.local/share/opencode/log/ | cut -c1-600 | head
```

`out.txt` ended `exit=0` and `err.txt` was empty. The `debug config` output:

```text
    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-05/proj/opencode.json",
    "info": {
      "$schema": "https://opencode.ai/config.json"
    }
  }
]
exit=0
```

The log grep printed nothing (no lines).

### P4d: invalid JSON (`config-06`)

```json proj/opencode.json
{
  "$schema": "https://opencode.ai/config.json",
  "model":
```

```sh
sh $C/probes/harness/run.sh $S/config-06 18796 "probe-invalid-json"
sh $S/config-tools/oc.sh $S/config-06 . debug config --standalone > $S/config-06/debug-config.txt 2>&1
grep -n '"path"\|"type"' $S/config-06/debug-config.txt
grep -rih 'opencode.json\|config' $S/config-06/home/.local/share/opencode/log/ | grep -i 'warn\|error\|invalid\|unknown\|skip' | cut -c1-600 | head -10
```

`out.txt` ended `exit=0`, the model answered `ok`, and `err.txt` was empty.

```text
3:    "type": "document",
4:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-06/home/.config/opencode/opencode.json",
29:    "type": "directory",
30:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-06/home/.config/opencode"
```

```text
timestamp=2026-11-24T16:38:21.010Z level=WARN run=e8747ad3 message="configuration normalization diagnostic" source=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-06/proj/opencode.json path=$ kind=invalid action="rejected malformed JSON or JSONC document" http.span=17 role=server
timestamp=2026-11-24T16:38:21.786Z level=WARN run=e8747ad3 message="configuration normalization diagnostic" source=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-06/proj/opencode.json path=$ kind=invalid action="rejected malformed JSON or JSONC document" http.span=793 role=server
timestamp=2026-11-24T16:38:33.760Z level=WARN run=135cc9ff message="configuration normalization diagnostic" source=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-06/proj/opencode.json path=$ kind=invalid action="rejected malformed JSON or JSONC document" http.span=7 role=server
```

Source: `probes/src/packages/core/src/config.ts:140` (`onExcessProperty: "ignore"`) and
`config.ts:141-151` (a parse error logs the WARN above and returns no document). The parser runs
with `allowTrailingComma: true`.

### Answer

No `opencode.json` is needed: `.opencode/skills`, `agents`, `commands` and `plugins` load without one.
The two files merge per key, and `.opencode/opencode.json` overrides the root `opencode.json`. An
unknown top-level key is silently dropped, with no stderr and no log line. Invalid JSON drops the
whole file silently: the run exits 0, and the only trace is the log-file WARN `configuration
normalization diagnostic … kind=invalid action="rejected malformed JSON or JSONC document"`.

---

## P5: Which directory is the root for `.opencode/` discovery when started in a subdirectory?

### Setup, command

Same `config-02` tree and runs as P2, plus:

```sh
sh $T/oc.sh $P repo/pkg/sub debug config --standalone > $P/debug-config-sub.txt 2>&1
grep -n '"path"\|"type"\|exit=' $P/debug-config-sub.txt
```

### Output

```text
3:    "type": "document",
4:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/home/.config/opencode/opencode.json",
29:    "type": "directory",
30:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/home/.config/opencode"
33:    "type": "directory",
34:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/.opencode"
37:    "type": "directory",
38:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo/.opencode"
41:    "type": "directory",
42:    "path": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo/pkg/.opencode"
45:exit=0
```

In P2, run B (start `repo/pkg/sub`) saw `aboveskill` (above the git root), `rootskill` and
`pkgskill`. Run A (start `repo`) did not see `pkgskill`. The plugin under `repo/.opencode/plugins/`
loaded in both runs, and its `process.cwd()` was the start directory (marker.log above). The session
export records the git root as the project and the start dir as `subpath` (P6).

Source: `probes/src/packages/core/src/config/discovery.ts:92`
`fs.up({ targets: ["."], start: location.directory })` with no `stop`, and
`probes/src/packages/util/src/fs-util.ts:165-167` walks until `parent === current` (the filesystem
root).

### Answer

OpenCode walks from the start directory up to the **filesystem root**, not only to the git root. It
collects every `.opencode/` on the way (above the git root too), and nearer directories win. A
`.opencode/` written at the project root (where `agentstacks.toml` lives) is found from any
subdirectory. The shell cwd and the plugin `process.cwd()` are still the start directory, not the
project root.

---

## P6: Session store and export

### Command

```sh
sh $T/oc.sh $S/config-01 . debug paths
sh $T/oc.sh $P repo session list --standalone
sh $T/oc.sh $P repo session export --standalone ses_df196cf37ffeh6sOQhqTmF0hWw > $P/export-B.json 2>$P/export-B.err
head -32 $P/export-B.json
node -e '
const t=require("fs").readFileSync(process.argv[1],"utf8").replace(/exit=\d+\s*$/,"");const j=JSON.parse(t);
console.log("top keys:",JSON.stringify(Object.keys(j)));
j.messages.forEach(m=>console.log(m.type, JSON.stringify(Object.keys(m)), m.content?JSON.stringify(m.content.map(c=>c.type+(c.name?":"+c.name:""))):""));' $P/export-B.json
sqlite3 $P/home/.local/share/opencode/opencode.db .tables
git -C $P/proj/repo rev-list --max-parents=0 HEAD
```

### Output

```text
home       /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home
data       /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home/.local/share/opencode
cache      /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home/.cache/opencode
config     /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home/.config/opencode
state      /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home/.local/state/opencode
tmp        /private/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/opencode
bin        /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home/.cache/opencode/bin
log        /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home/.local/share/opencode/log
repos      /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home/.local/share/opencode/repos
db         /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/home/.local/share/opencode/opencode.db
exit=0
```

```text
ses_df196cf37ffeh6sOQhqTmF0hWw	title	11/24/2026, 11:35:05 PM
ses_df19716d0ffe7s4lCZIcdkIw4I	title	11/24/2026, 11:34:47 PM
exit=0
```

```text
{
  "info": {
    "id": "ses_df196cf37ffeh6sOQhqTmF0hWw",
    "projectID": "bfa89d887b51ce22f426fcb666c973a392813904",
    "cost": 0,
    "tokens": {
      "input": 0,
      "output": 0,
      "reasoning": 0,
      "cache": {
        "read": 0,
        "write": 0
      }
    },
    "outcome": "succeeded",
    "time": {
      "created": 1795538104522,
      "updated": 1795538105105,
      "busy": 1795538104868,
      "idle": 1795538105107
    },
    "title": "title",
    "subpath": "pkg/sub",
    "location": {
      "directory": "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-02/proj/repo/pkg/sub"
    }
  },
  "messages": [
    {
      "id": "msg_20e6930d2001qDLAkg7F1WyuSR",
      "time": {
        "created": 1795538104876
[… rest cut]
```

```text
top keys: ["info","messages"]
user ["id","time","text","files","type"] 
assistant ["id","time","type","agent","model","content","snapshot","finish","rawFinish","cost","tokens"] ["tool:shell"]
assistant ["id","time","type","agent","model","content","snapshot","finish","rawFinish","cost","tokens"] ["text"]
idle ["id","time","type","outcome"]
```

```text
account            event              machine            session_part     
account_state      event_sequence     migration          session_v2       
automation         instruction_blob   permission         workspace        
blob               instruction_entry  project            worktree         
blob_ref           instruction_state  project_directory
control_account    job                session_inbox    
credential         kv                 session_message
```

```text
bfa89d887b51ce22f426fcb666c973a392813904
```

The `config-01` export (a git repo with **no commits**) has `"projectID": "global"` and no `subpath`.
Its messages, printed with `{id,time}` removed:

```text
{"text":"/echo a b","files":[],"type":"user"}
{"type":"assistant","agent":"build","model":{"id":"m","providerID":"mock"},"content":[{"type":"text","text":"ok"}],"snapshot":{"start":"91fd356bc66bc6527c57186964326b027bc42561","end":"91fd356bc66bc6527c57186964326b027bc42561","files":[]},"finish":"stop","rawFinish":"stop","cost":0,"tokens":{"input"
{"type":"idle","outcome":"succeeded"}
{"text":"ALL=[a \"b c\" d] ONE=[a] TWO=[b c d]\nSHELL=[injected-a\n/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/proj\n]\nLoad the skill named demo with the skill tool, then follow it.","type":"user"}
{"text":"ALL=[x y] ONE=[x] TWO=[y]\nSHELL=[injected-x\n/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/config-01/proj\n]\nLoad the skill named demo with the skill tool, then follow it.","type":"user"}
{"text":"continue","files":[],"type":"user"}
{"type":"assistant","agent":"build","model":{"id":"m","providerID":"mock"},"content":[{"type":"tool","id":"call_1","name":"skill","executed":false,"state":{"status":"completed","input":{"id":"demo"},"content":[{"type":"text","text":"<skill_content name=\"demo\">\n# Skill: demo\n\nDEMO SKILL BODY sid
{"type":"assistant","agent":"build","model":{"id":"m","providerID":"mock"},"content":[{"type":"text","text":"loaded"}],"snapshot":{"start":"91fd356bc66bc6527c57186964326b027bc42561","end":"91fd356bc66bc6527c57186964326b027bc42561","files":[]},"finish":"stop","rawFinish":"stop","cost":0,"tokens":{"in
{"type":"idle","outcome":"succeeded"}
```

### Answer

Sessions live in SQLite at `<XDG_DATA_HOME>/opencode/opencode.db` (tables include `session_v2`,
`session_message` and `session_part`). `opencode session export [--standalone] [--sanitize] <id>`
prints JSON `{info, messages}` to stdout. `info` carries `id`, `projectID` (the repo's root-commit
hash, or `global` when there is no commit), `location.directory` and `subpath`. `messages` is a
flat list typed `user` (`text`, `files`), `assistant` (`content[]` of `text`/`tool` parts, `agent`,
`model`, `tokens`) and `idle`.
