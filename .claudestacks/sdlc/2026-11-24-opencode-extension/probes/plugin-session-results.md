# Probe results: session hooks and events (OpenCode v2.4.1)

Topic: which OpenCode V2 server-plugin hooks and bus events can carry the agentstacks neutral events
`session.start`, `session.end`, `prompt.submit` and `agent.stop` (`tool.before` / `tool.after` are
probed elsewhere). Binary `/Users/hiraq/.opencode/bin/opencode` v2.4.1, run through
`probes/harness/run.sh` (mock model, isolated HOME, `--standalone --auto --format json`).
Node used by the harness and the extract scripts: v25.1.0.

- `S` = `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad`
- `C` = this chain dir. Probe dirs: `$S/plugsess-01` … `$S/plugsess-07`. Ports 18770–18779.
- Source citations point to copies under `probes/src/` (tag v2.4.1); docs to `probes/docs/`.

## Summary

| Q | Answer | Section |
|---|---|---|
| 1 | Five hook domains: `session` (15 hooks incl. `experimental.idle`, `fork`), `tool` (`execute.before/after`), `permission` (`evaluate`), `shell` (`create.before`), `aisdk` (`sdk`, `language`). Plus `ctx.event.subscribe` for bus events. A plain `{ id, setup }` JS plugin registered 14 of them (10 session, 2 tool, permission, shell) without error. | P1 |
| 2 | `session` `prompt` hook: input `{sessionID, messageID, prompt:{text,files,agents?,skills?}, metadata, delivery}`; no agent field. Editing `prompt.text` changes what the model sees (and what is stored). `ctx.session.instructions.entry.put` from the hook adds a `<context key=…>` block to the system prompt. Throwing blocks the turn, but only as a generic `UnexpectedStatus: 500` error, exit 1, no reason shown. | P2 |
| 3 | No session-start hook. The run's own `session.created` event is never seen by the plugin (session is created before the plugin loads); it is seen only for child sessions. Resume (`-c`, `--session`) emits nothing distinct. The `context` hook (every agent-loop request) can add system text the model sees, but has no source field; the source must be inferred (history length, `<conversation-checkpoint>` message). | P3 |
| 4 | `experimental.idle` session hook (undocumented in docs, in source) fires when a session is about to go idle, and the run waits for an async handler (1.5 s sleep observed). Bus event `session.execution.succeeded` also fires; plugin cleanup runs at exit and is awaited. `session.idle`/`session.status` events are never published. | P4 |
| 5 | Subagents always run in the background. The `subagent` tool's after-hook sees `input.agent`/`prompt` but only a "working in the background" result, never the final text (its result can be changed, and the change reaches the parent model). The child's final text reaches the parent as a synthetic user message `<background-result …>` with metadata `{source:"subagent", childID, agent, state}`. `experimental.idle` fires on the child session with its `sessionID` (`ctx.session.get` gives `parentID`, `agent`); `ctx.session.synthetic` from that hook makes the child continue in the same run. | P5 |
| 6 | `compaction` hook = `context` input (`sessionID, agent, model, system, messages, options, tools`) + optional `result`. Pushing to `system` reaches the summarization request only. Setting `result` skips the model call and the summary is stored. After compaction the next request carries a `<conversation-checkpoint>` user message; the `context` hook can add text there, but has no compaction flag. | P6 |
| 7 | Yes: `prompt`, `context`, `tool.execute.*`, `permission.evaluate`, `experimental.idle` all fire in the child session. Tool, context and permission hooks carry `agent` (`general`); `prompt` and `idle` carry only `sessionID` (`ctx.session.get` gives `agent` and `parentID`). | P7 |

### Shared setup

Probe plugin used by plugsess-02 … 05 and 07 (copied to `<dir>/proj/.opencode/plugins/observe.js`; each of those dirs also has `proj/a.txt` = `hello`):

```plugsess-bin/observe.js
// Probe plugin (no dependencies). Logs every server hook and every bus event to $PS_LOG as JSONL.
// $PS_MODE selects extra behaviour: observe | block | subagent | compact | resume
import { appendFileSync } from "node:fs"

const LOG = process.env.PS_LOG
const MODE = process.env.PS_MODE ?? "observe"
const cut = (v, n = 300) => {
  const s = typeof v === "string" ? v : JSON.stringify(v)
  return s === undefined ? s : s.length > n ? s.slice(0, n) + `…(+${s.length - n})` : s
}
const log = (kind, data) => appendFileSync(LOG, JSON.stringify({ t: Date.now(), kind, ...data }) + "\n")
const msgSummary = (m) => ({
  role: m.role,
  meta: m.metadata,
  text: cut(
    (Array.isArray(m.content) ? m.content : [m.content])
      .map((p) => (typeof p === "string" ? p : p?.text ?? p?.type))
      .join(" | "),
    200,
  ),
})
const reqSummary = (e) => ({
  sessionID: e.sessionID,
  agent: e.agent,
  model: e.model,
  keys: Object.keys(e),
  system: e.system?.map((p) => ({
    type: p.type,
    text: cut(p.text, 80),
    instructionKeys: p.metadata?.experimental?.instructions?.map((i) => i.key),
  })),
  messages: e.messages?.length,
  lastMessages: e.messages?.slice(-3).map(msgSummary),
  tools: e.tools ? Object.keys(e.tools) : undefined,
})

const state = { compacted: false, compactRequested: false, continued: new Set(), parents: new Map() }

export default {
  id: "plugsess-probe",
  async setup(ctx) {
    log("setup", { mode: MODE, location: ctx.location, ctxKeys: Object.keys(ctx), sessionKeys: Object.keys(ctx.session) })

    const controller = new AbortController()
    void (async () => {
      try {
        for await (const ev of ctx.event.subscribe({ signal: controller.signal })) {
          log("event", { type: ev.type, data: cut(ev, 400) })
          if (ev.type === "session.created" && ev.data?.parentID) state.parents.set(ev.data.sessionID, ev.data.parentID)
          if (ev.type === "session.compaction.ended") state.compacted = true
        }
      } catch (err) {
        log("event-error", { err: String(err) })
      }
    })()

    const sessionHooks = [
      "fork", "prompt", "context", "compaction", "generate", "title",
      "experimental.model.resolve", "model.request", "retry", "experimental.idle",
    ]
    for (const name of sessionHooks) {
      try {
        await ctx.session.hook(name, async (e) => {
          if (name === "prompt") {
            log("hook", { name, sessionID: e.sessionID, messageID: e.messageID, keys: Object.keys(e), prompt: e.prompt, metadata: e.metadata, delivery: e.delivery })
            if (MODE === "block") throw new Error("PLUGSESS-BLOCK: prompt refused by hook")
            if (MODE === "observe") {
              e.prompt.text = e.prompt.text + "\n[PROMPT-HOOK-EDIT]"
              e.metadata = { ...e.metadata, plugsess: "prompt-hook" }
              try {
                await ctx.session.instructions.entry.put({ sessionID: e.sessionID, key: "plugsess-prompt", value: "ENTRY-FROM-PROMPT-HOOK" })
                log("entry-put", { from: "prompt", ok: true })
              } catch (err) {
                log("entry-put", { from: "prompt", err: String(err) })
              }
            }
            return
          }
          if (name === "context" || name === "compaction" || name === "generate" || name === "title") {
            log("hook", { name, ...reqSummary(e), parentID: state.parents.get(e.sessionID) })
            if (name === "context") {
              e.system.push({ type: "text", text: `CTX-HOOK-SYSTEM agent=${e.agent}` })
              if (state.compacted) e.system.push({ type: "text", text: "AFTER-COMPACTION-CONTEXT" })
            }
            if (name === "compaction") {
              e.system.push({ type: "text", text: "COMPACTION-HOOK-SYSTEM" })
              if (MODE === "compact-result") e.result = { summary: "PLUGIN-SUPPLIED-SUMMARY" }
            }
            return
          }
          if (name === "experimental.idle") {
            let info
            try {
              info = await ctx.session.get({ sessionID: e.sessionID })
            } catch (err) {
              info = String(err)
            }
            log("hook", { name, sessionID: e.sessionID, background: e.background, keys: Object.keys(e), session: cut(info, 500) })
            const parentID = info?.data?.parentID ?? info?.parentID
            if (parentID) state.parents.set(e.sessionID, parentID)
            // subagent: make the child continue once
            if (MODE === "subagent" && state.parents.has(e.sessionID) && !state.continued.has(e.sessionID)) {
              state.continued.add(e.sessionID)
              const r = await ctx.session.synthetic({ sessionID: e.sessionID, text: "IDLE-HOOK-CONTINUE: keep going" })
              log("synthetic", { sessionID: e.sessionID, r: cut(r) })
            }
            if ((MODE === "compact" || MODE === "compact-result") && !state.compactRequested) {
              state.compactRequested = true
              const r = await ctx.session.compact({ sessionID: e.sessionID })
              log("compact-called", { r: cut(r) })
            }
            return
          }
          log("hook", { name, sessionID: e.sessionID, agent: e.agent, reqKind: e.kind, keys: Object.keys(e) })
        })
        log("registered", { name })
      } catch (err) {
        log("register-error", { name, err: String(err) })
      }
    }

    for (const name of ["execute.before", "execute.after"]) {
      await ctx.tool.hook(name, async (e) => {
        log("hook", { name: `tool.${name}`, tool: e.tool, sessionID: e.sessionID, agent: e.agent, keys: Object.keys(e), input: cut(e.input, 200), status: e.status, result: cut(e.result, 600), parentID: state.parents.get(e.sessionID) })
        if (MODE === "subagent" && name === "execute.after" && e.tool === "subagent" && e.status === "completed") {
          e.result = { ...e.result, content: e.result.content + "\nAFTER-HOOK-APPENDED", output: { ...e.result.output, output: e.result.output?.output + "\nAFTER-HOOK-APPENDED" } }
        }
      })
      log("registered", { name: `tool.${name}` })
    }
    await ctx.permission.hook("evaluate", (e) => log("hook", { name: "permission.evaluate", sessionID: e.sessionID, agent: e.agent, action: e.action }))
    log("registered", { name: "permission.evaluate" })
    await ctx.shell.hook("create.before", (e) => log("hook", { name: "shell.create.before", command: e.command }))
    log("registered", { name: "shell.create.before" })

    return () => {
      log("cleanup", {})
      controller.abort()
    }
  },
}
```

plugsess-01 ran an earlier revision of the same file. Its only differences:

```diff
49,52c49
<           if (ev.type === "session.created") {
<             const info = ev.data?.info ?? ev.properties?.info ?? ev.data
<             if (info?.parentID) state.parents.set(info.id, info.parentID)
<           }
---
>           if (ev.type === "session.created" && ev.data?.parentID) state.parents.set(ev.data.sessionID, ev.data.parentID)
95c92,100
<             log("hook", { name, sessionID: e.sessionID, background: e.background, keys: Object.keys(e), parentID: state.parents.get(e.sessionID) })
---
>             let info
>             try {
>               info = await ctx.session.get({ sessionID: e.sessionID })
>             } catch (err) {
>               info = String(err)
>             }
>             log("hook", { name, sessionID: e.sessionID, background: e.background, keys: Object.keys(e), session: cut(info, 500) })
>             const parentID = info?.data?.parentID ?? info?.parentID
>             if (parentID) state.parents.set(e.sessionID, parentID)
109c114
<           log("hook", { name, sessionID: e.sessionID, agent: e.agent, kind: e.kind, keys: Object.keys(e) })
---
>           log("hook", { name, sessionID: e.sessionID, agent: e.agent, reqKind: e.kind, keys: Object.keys(e) })
```

Log readers (scratchpad only):

```plugsess-bin/hooks.js
// usage: node hooks.js <hooklog.jsonl> [full]  — one line per plugin log entry; noisy bus events filtered unless "full"
const full = process.argv[3] === "full"
const l = require("fs").readFileSync(process.argv[2], "utf8").trim().split("\n").map(JSON.parse)
const noisy = /\.updated$|delta$|first-token$|streamed$|^session\.usage|^session\.tool\.|^session\.step\.|^session\.text\.|^experimental\.job/
for (const x of l) {
  const { t, kind, ...r } = x
  if (kind === "registered") {
    if (full) console.log(t, kind, r.name)
    continue
  }
  if (kind === "event") {
    if (!full && noisy.test(r.type)) continue
    console.log(t, "event", r.type, typeof r.data === "string" ? r.data.replace(/^.*?"data":/, "").slice(0, 260) : JSON.stringify(r.data))
    continue
  }
  if (r.name === "experimental.model.resolve" || r.name === "model.request") {
    if (full) console.log(t, kind, r.name, r.reqKind ?? r.kind ?? "", r.agent, r.sessionID)
    continue
  }
  if (r.name === "context" || r.name === "compaction" || r.name === "title") {
    console.log(t, kind, r.name, JSON.stringify({ sessionID: r.sessionID, agent: r.agent, keys: r.keys, messages: r.messages, lastMessages: r.lastMessages, instructionKeys: (r.system || []).map((s) => s.instructionKeys).filter(Boolean) }).slice(0, 900))
    continue
  }
  if (kind === "setup") {
    console.log(t, kind, JSON.stringify({ mode: r.mode, ctxKeys: r.ctxKeys, sessionKeys: r.sessionKeys }))
    continue
  }
  console.log(t, kind, JSON.stringify(r).slice(0, 700))
}
```

```plugsess-bin/reqs.js
// usage: node reqs.js <requests.jsonl> [marker...] — per model request: tool count, messages (role + first 300 chars), markers found
const [file, ...markers] = process.argv.slice(2)
const l = require("fs").readFileSync(file, "utf8").trim().split("\n").filter(Boolean).map(JSON.parse)
console.log("requests:", l.length)
l.forEach((r, i) => {
  const b = r.body, s = JSON.stringify(b), ms = b.messages || []
  console.log(`req ${i} tools=${(b.tools || []).length} messages=${ms.length}`)
  for (const m of markers) {
    const k = s.indexOf(m)
    console.log(`  marker ${m}: ${k < 0 ? "absent" : JSON.stringify(s.slice(Math.max(0, k - 90), k + m.length + 40))}`)
  }
  for (const m of ms.slice(1)) console.log(`  ${m.role}: ${JSON.stringify(m.content ?? m.tool_calls).slice(0, 300)}`)
})
```

All commands below run from `cd $S` (`S` = the scratchpad, `C` = the chain dir). `PS_MODE` / `PS_LOG` are read by the plugin.

## P1: Every hook and event a V2 server plugin can register, with its input fields

**Source.** The core hook registry has five domains
(`probes/src/packages/core/src/plugin/hooks.ts:13-19`: `aisdk`, `session`, `permission`, `shell`,
`tool`). Only `tool` `execute.before` has a failure channel
(`probes/src/packages/core/src/plugin/hooks.ts:23-30`). Input types for the Promise (plain JS)
plugin API:

| Domain.hook | Input fields (mutable fields in bold) | Source |
|---|---|---|
| session.fork | `sessionID` | `probes/src/packages/plugin/src/promise/session.ts:208` |
| session.prompt | `sessionID`, `messageID`, **`prompt`** (`text`, `files`, `agents`, `skills`), **`metadata`**, **`delivery`** | `…/promise/session.ts:14-20` |
| session.context | `sessionID`, `model`, `agent`, **`system`**, **`messages`**, **`options`**, **`tools`** | `…/promise/session.ts:28-43` |
| session.compaction | as `context` + **`result?`** `{summary, native?, metadata?, tokens?}` | `…/promise/session.ts:45-56` |
| session.generate | as `context` | `…/promise/session.ts:58` |
| session.title | `sessionID`, `model`, **`system`**, **`messages`**, **`options`**, **`result?`** | `…/promise/session.ts:60-63` |
| session.experimental.model.resolve | `sessionID`, `agent`, `kind`, **`model`** | `…/promise/session.ts:77-82` |
| session.model.request | `sessionID`, `agent`, `model`, `kind`, **`baseURL`**, **`headers`** | `…/promise/session.ts:84-91` |
| session.http.request / http.response | `sessionID`, `agent`, `model`, `kind`, **`request`** / `request`, **`response`** | `…/promise/session.ts:93-108` |
| session.experimental.ws.handshake / send / receive | `sessionID`, `agent`, `model`, `kind`, **`url`,`headers`** / **`frame`** | `…/promise/session.ts:114-146` |
| session.retry | `sessionID`, `agent`, `model`, `error`, `attempt`, **`decision`** | `…/promise/session.ts:150-157` |
| session.experimental.idle | `sessionID`, `background[]` (`kind`: codemode, shell, subagent, image, video) | `…/promise/session.ts:163-205` |
| tool.execute.before | **`tool`**, `info`, `codemode`, `sessionID`, `agent`, `messageID`, `id`, **`input`** | `probes/src/packages/plugin/src/promise/tool.ts:52-61` |
| tool.execute.after | `tool`, `info`, `codemode`, `sessionID`, `agent`, `messageID`, `id`, `input`, `status`, **`result`** or **`error`** | `…/promise/tool.ts:62-81` |
| permission.evaluate | `sessionID`, `agent?`, `action`, `resources`, `metadata?`, `source?`, **`effect`**, **`message?`** | `probes/src/packages/plugin/src/promise/permission.ts:8-17` |
| shell.create.before | **`command`, `cwd`, `timeout`, `shell`, `env`** | `probes/src/packages/plugin/src/promise/shell.ts:3-9` |
| aisdk.sdk / aisdk.language | `model`, `package`/`sdk`, `options`, **`sdk?`** / **`language?`** | `probes/src/packages/plugin/src/promise/aisdk.ts:5-18` |

The full `SessionHooks` list is at `probes/src/packages/plugin/src/promise/session.ts:207-223`. The
docs' reference (`probes/docs/build-plugins/server.mdx:1640-1654`) omits `fork` (documented
separately at `server.mdx:1384-1390`) and `experimental.idle` (not documented anywhere in the docs
copy; `grep -n idle probes/docs/build-plugins/*.mdx` finds no hook). Each Promise callback is wrapped
in `Effect.promise` (`probes/src/packages/plugin/src/promise/adapter.ts:675`), so a rejection is a
defect, not a typed failure; only `tool.execute.before` maps a rejection to a tool error
(`adapter.ts:265-297`).

Bus events: `ctx.event.subscribe({ types?, signal? })` returns an async iterable of every public
event (`probes/docs/build-plugins/server.mdx:1338-1365`, `probes/src/packages/plugin/src/promise/event.ts:6-10`).
Session event types are defined in `probes/src/packages/schema/src/session-event.ts` (e.g.
`session.created` :40, `session.execution.started/succeeded/failed/interrupted` :245-259,
`session.synthetic` :285, `session.compaction.started/ended/failed` :600-637). The types actually
observed in a run are listed in P3.

Run: plugsess-01 (observe mode) registers every hook above except `http.*`, `experimental.ws.*` and
`aisdk.*`, and counts which fired in a read → answer turn.

Run (once, earlier):

```sh
PS_MODE=observe PS_LOG=$S/plugsess-01/hooklog.jsonl sh $C/probes/harness/run.sh $S/plugsess-01 18770 "hello probe"
```

Extract:

```sh
node plugsess-bin/hooks.js plugsess-01/hooklog.jsonl full | grep -E " (registered|setup) "
```

Output:

```text
1795537943862 setup {"mode":"observe","ctxKeys":["app","location","options","agent","aisdk","command","credential","event","experimental","generate","model","provider","integration","mcp","permission","plugin","reference","rpc","skill","storage","tool","vcs","websearch","worktree","session","shell"],"sessionKeys":["hook","list","create","get","remove","switchAgent","switchModel","prompt","generate","command","compact","synthetic","interrupt","update","move","wait","context","instructions"]}
1795537943863 registered fork
1795537943863 registered prompt
1795537943863 registered context
1795537943863 registered compaction
1795537943863 registered generate
1795537943863 registered title
1795537943863 registered experimental.model.resolve
1795537943863 registered model.request
1795537943863 registered retry
1795537943863 registered experimental.idle
1795537943863 registered tool.execute.before
1795537943863 registered tool.execute.after
1795537943864 registered permission.evaluate
1795537943864 registered shell.create.before
```

Extract:

```sh
grep "\"kind\":\"hook\"" plugsess-01/hooklog.jsonl | grep -o "\"name\":\"[^\"]*\"" | sort | uniq -c
```

Output:

```text
   2 "name":"context"
   1 "name":"experimental.idle"
   1 "name":"permission.evaluate"
   1 "name":"prompt"
   1 "name":"title"
   1 "name":"tool.execute.after"
   1 "name":"tool.execute.before"
```

**Answer.** The registry exposes the five domains in the table above (15 session hooks, including
the undocumented `experimental.idle`). A no-dependency `export default { id, setup }` plugin
registered 14 of them without error; in a plain turn `prompt` fired once, `context` once per
agent-loop request (2), `title` once, the tool and permission hooks once for the `read` call, and
`experimental.idle` once at the end. `fork`, `compaction`, `generate` and `retry` did not fire in
this turn.

## P2: The session `prompt` hook: input, add context, modify, block

**Source.** Core triggers the hook in `SessionPrompt.prepare` before attachments and skills are
resolved, and uses the returned draft as the stored user input
(`probes/src/packages/core/src/session/prompt.ts:42-54`, `:80-90`). Docs: the draft is mutable,
IDs are read-only, edits become the canonical persisted input, if preparation fails the prompt is
not admitted, and "Prompt hooks transform input and do not expose a typed rejection API"
(`probes/docs/build-plugins/server.mdx:1392-1432`). The hook does not run for synthetic messages
(`server.mdx:1421`). Session instruction entries: `PUT /api/experimental/session/:sessionID/instructions/entries/:key`,
"Attach or replace one durable instruction entry" (`probes/src/packages/protocol/src/groups/session.ts:603-619`);
the key must match `^[a-z0-9][a-z0-9._-]*$` (`probes/src/packages/schema/src/instruction-entry.ts:10`).

Setup: plugsess-01 in `observe` mode appends `\n[PROMPT-HOOK-EDIT]` to `prompt.text`, sets
`metadata`, and calls `ctx.session.instructions.entry.put({ sessionID, key: "plugsess-prompt", value: "ENTRY-FROM-PROMPT-HOOK" })`
from the prompt hook; its `context` hook pushes `CTX-HOOK-SYSTEM agent=<agent>` onto `system`.
plugsess-02 in `block` mode throws `Error("PLUGSESS-BLOCK: prompt refused by hook")` from the prompt hook.

```plugsess-01/run1/scenario.json
[{"tool":"read","args":{"path":"a.txt"}},{"text":"final-01"}]
```

Run (once, earlier):

```sh
PS_MODE=observe PS_LOG=$S/plugsess-01/hooklog.jsonl sh $C/probes/harness/run.sh $S/plugsess-01 18770 "hello probe"   # outputs then copied to plugsess-01/run1/
```

Extract:

```sh
grep "\"kind\":\"hook\",\"name\":\"prompt\"" plugsess-01/hooklog.jsonl; grep entry-put plugsess-01/hooklog.jsonl
```

Output:

```text
{"t":1795537943925,"kind":"hook","name":"prompt","sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","messageID":"msg_20e66bd33001FT97Iiu1C3FLoA","keys":["sessionID","messageID","prompt","metadata","delivery"],"prompt":{"text":"hello probe","files":[]},"delivery":"steer"}
{"t":1795537943926,"kind":"entry-put","from":"prompt","ok":true}
```

Extract:

```sh
node plugsess-bin/reqs.js plugsess-01/run1/requests.jsonl PROMPT-HOOK-EDIT CTX-HOOK-SYSTEM ENTRY-FROM-PROMPT-HOOK
```

Output:

```text
requests: 3
req 0 tools=0 messages=2
  marker PROMPT-HOOK-EDIT: "er\",\"content\":\"Here is the first message of the conversation:\\n\\n<message>\\nhello probe\\n[PROMPT-HOOK-EDIT]\\n</message>\\n\\nReply with only the tit"
  marker CTX-HOOK-SYSTEM: absent
  marker ENTRY-FROM-PROMPT-HOOK: absent
  user: "Here is the first message of the conversation:\n\n<message>\nhello probe\n[PROMPT-HOOK-EDIT]\n</message>\n\nReply with only the title."
req 1 tools=14 messages=2
  marker PROMPT-HOOK-EDIT: "working directory.\\nCTX-HOOK-SYSTEM agent=build\"},{\"role\":\"user\",\"content\":\"hello probe\\n[PROMPT-HOOK-EDIT]\"}],\"tools\":[{\"type\":\"function\",\"functi"
  marker CTX-HOOK-SYSTEM: "all `tools.opencode.session.move` and make the worktree the session's working directory.\\nCTX-HOOK-SYSTEM agent=build\"},{\"role\":\"user\",\"content\":"
  marker ENTRY-FROM-PROMPT-HOOK: "e-created and approved for external access.\\n</env>\\n\\n<context key=\\\"plugsess-prompt\\\">\\nENTRY-FROM-PROMPT-HOOK\\n</context>\\nWhen you create a worktree"
  user: "hello probe\n[PROMPT-HOOK-EDIT]"
req 2 tools=14 messages=4
  marker PROMPT-HOOK-EDIT: "working directory.\\nCTX-HOOK-SYSTEM agent=build\"},{\"role\":\"user\",\"content\":\"hello probe\\n[PROMPT-HOOK-EDIT]\"},{\"role\":\"assistant\",\"content\":null,\""
  marker CTX-HOOK-SYSTEM: "all `tools.opencode.session.move` and make the worktree the session's working directory.\\nCTX-HOOK-SYSTEM agent=build\"},{\"role\":\"user\",\"content\":"
  marker ENTRY-FROM-PROMPT-HOOK: "e-created and approved for external access.\\n</env>\\n\\n<context key=\\\"plugsess-prompt\\\">\\nENTRY-FROM-PROMPT-HOOK\\n</context>\\nWhen you create a worktree"
  user: "hello probe\n[PROMPT-HOOK-EDIT]"
  assistant: [{"id":"call_1","type":"function","function":{"name":"read","arguments":"{\"path\":\"a.txt\"}"}}]
  tool: "Read file a.txt, lines 1-1\n1: hello"
```

```plugsess-02/scenario.json
[{"text":"should-not-be-reached"}]
```

Run (once, earlier):

```sh
PS_MODE=block PS_LOG=$S/plugsess-02/hooklog.jsonl sh $C/probes/harness/run.sh $S/plugsess-02 18771 "please do something"
```

Extract:

```sh
cat plugsess-02/out.txt; echo "--- err.txt bytes: $(wc -c < plugsess-02/err.txt)"; echo "--- requests.jsonl lines: $(wc -l < plugsess-02/requests.jsonl)"; node plugsess-bin/hooks.js plugsess-02/hooklog.jsonl | grep -v " event "
```

Output:

```text
{"type":"error","timestamp":1795538001247,"sessionID":"ses_df19866a3ffes3ZKr3fWemNSpD","error":{"type":"unknown","message":"UnexpectedStatus: 500"}}
exit=1
--- err.txt bytes:        0
--- requests.jsonl lines:        0
1795538001043 setup {"mode":"block","ctxKeys":["app","location","options","agent","aisdk","command","credential","event","experimental","generate","model","provider","integration","mcp","permission","plugin","reference","rpc","skill","storage","tool","vcs","websearch","worktree","session","shell"],"sessionKeys":["hook","list","create","get","remove","switchAgent","switchModel","prompt","generate","command","compact","synthetic","interrupt","update","move","wait","context","instructions"]}
1795538001244 hook {"name":"prompt","sessionID":"ses_df19866a3ffes3ZKr3fWemNSpD","messageID":"msg_20e679c98001H3KGlV2V1eAvjS","keys":["sessionID","messageID","prompt","metadata","delivery"],"prompt":{"text":"please do something","files":[]},"delivery":"steer"}
1795538001287 cleanup {}
```

**Answer.**

- Input is `{sessionID, messageID, prompt: {text, files}, metadata, delivery: "steer"}`. There is no
  agent field (only `prompt.agents`, the `@agent` mentions).
- **Modify / add context:** an edit to `prompt.text` reaches the model as the user message (and the
  title request), because it becomes the stored input. An instruction entry put from the hook shows
  up in the same turn's system prompt as `<context key="plugsess-prompt">…</context>`, and it is
  durable (it is still there on resume, P3).
- **Block:** a throw stops the turn before any model request (`requests.jsonl` is empty), but
  `opencode run` prints only `{"type":"error",…,"message":"UnexpectedStatus: 500"}` and exits 1. The
  hook's message does not reach the user, and the user prompt is not stored.

## P3: Session start: a hook or event on create / resume / compaction, and adding context

**Source.** No session hook is named for start or resume (list in P1). `Session.create` publishes
`session.created` (`probes/src/packages/core/src/session.ts:302-320`; schema with optional
`parentID`, `probes/src/packages/schema/src/session-event.ts:39-57`). The prompt path waits for the
location's plugins to activate (`probes/src/packages/core/src/session/prompt.ts:40`), so plugins load
lazily at the first prompt.

First: every bus event type the plugsess-01 plugin received in a plain run, in order (consecutive
duplicates folded by `uniq`):

Extract:

```sh
node plugsess-bin/hooks.js plugsess-01/hooklog.jsonl full | grep " event " | cut -d" " -f3 | uniq
```

Output:

```text
integration.updated
provider.updated
model.updated
agent.updated
command.updated
skill.updated
artifact.updated
websearch.updated
reference.updated
plugin.updated
session.inbox.enqueued
session.execution.started
session.instructions.updated
session.inbox.delivered
session.usage.updated
session.renamed
session.step.started
session.step.first-token
session.tool.input.started
session.tool.input.ended
session.tool.called
session.step.streamed
session.tool.success
session.step.ended
session.usage.updated
session.step.started
session.step.first-token
session.text.started
session.text.delta
session.text.ended
session.step.streamed
session.step.ended
session.usage.updated
session.execution.succeeded
location.shutdown
```

Run (once, earlier):

```sh
PS_MODE=resume PS_LOG=$S/plugsess-07/hooklog.jsonl sh $C/probes/harness/run.sh $S/plugsess-07 18778 "spawn a child"   # the P7 run
```

Extract:

```sh
node -e 'const l=require("fs").readFileSync("plugsess-07/hooklog.jsonl","utf8").trim().split("\n").map(JSON.parse);
console.log("plugin setup t", l.find(x=>x.kind==="setup").t);
for (const i of l.filter(x=>x.kind==="hook"&&x.name==="experimental.idle")) console.log("session", i.sessionID, "time.created", i.session.match(/"created":(\d+)/)[1], /parentID/.test(i.session)?"(child)":"(top-level)");
console.log("session.created events seen:", l.filter(x=>x.kind==="event"&&x.type==="session.created").map(x=>x.data.match(/"aggregateID":"(\w+)"/)[1]));'
```

Output:

```text
plugin setup t 1795538102343
session ses_df196d900ffenNjsKDabZzgzov time.created 1795538102322 (top-level)
session ses_df196d716ffebFvbQGfN3ruuUG time.created 1795538102507 (child)
session ses_df196d900ffenNjsKDabZzgzov time.created 1795538102322 (top-level)
session.created events seen: [ 'ses_df196d716ffebFvbQGfN3ruuUG' ]
```

Resume with `-c`: second run in plugsess-01 after copying run 1 outputs to `run1/`; `scenario.json` = `[{"text":"resumed-answer"}]`.

Run (once, earlier):

```sh
PS_MODE=resume PS_LOG=$S/plugsess-01/hooklog-resume.jsonl sh $C/probes/harness/run.sh $S/plugsess-01 18776 "resume turn" -c   # outputs then copied to plugsess-01/run2/
```

Extract:

```sh
tail -1 plugsess-01/run2/out.txt; node plugsess-bin/hooks.js plugsess-01/hooklog-resume.jsonl
```

Output:

```text
exit=0
1795538068724 setup {"mode":"resume","ctxKeys":["app","location","options","agent","aisdk","command","credential","event","experimental","generate","model","provider","integration","mcp","permission","plugin","reference","rpc","skill","storage","tool","vcs","websearch","worktree","session","shell"],"sessionKeys":["hook","list","create","get","remove","switchAgent","switchModel","prompt","generate","command","compact","synthetic","interrupt","update","move","wait","context","instructions"]}
1795538069002 hook {"name":"prompt","sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","messageID":"msg_20e68a49c001ipOPBAqdf1BqDr","keys":["sessionID","messageID","prompt","metadata","delivery"],"prompt":{"text":"resume turn","files":[]},"delivery":"steer"}
1795538069007 event session.inbox.enqueued {"sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","inboxID":"m…(+118)
1795538069020 event session.execution.started {"sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv"}}
1795538069022 event session.inbox.delivered {"sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","inboxID":"…(+33)
1795538069027 hook context {"sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":5,"lastMessages":[{"role":"tool","text":"tool-result"},{"role":"assistant","text":"final-01"},{"role":"user","text":"resume turn"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment","api/plugsess-prompt"]]}
1795538069080 hook {"name":"experimental.idle","sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","background":[],"keys":["sessionID","background"]}
1795538069082 event session.execution.succeeded {"sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv"}}
1795538069130 event location.shutdown {}}
1795538069138 cleanup {}
```

Extract:

```sh
node plugsess-bin/reqs.js plugsess-01/run2/requests.jsonl ENTRY-FROM-PROMPT-HOOK CTX-HOOK-SYSTEM
```

Output:

```text
requests: 1
req 0 tools=14 messages=6
  marker ENTRY-FROM-PROMPT-HOOK: "e-created and approved for external access.\\n</env>\\n\\n<context key=\\\"plugsess-prompt\\\">\\nENTRY-FROM-PROMPT-HOOK\\n</context>\\nWhen you create a worktree"
  marker CTX-HOOK-SYSTEM: "all `tools.opencode.session.move` and make the worktree the session's working directory.\\nCTX-HOOK-SYSTEM agent=build\"},{\"role\":\"user\",\"content\":"
  user: "hello probe\n[PROMPT-HOOK-EDIT]"
  assistant: [{"id":"call_1","type":"function","function":{"name":"read","arguments":"{\"path\":\"a.txt\"}"}}]
  tool: "Read file a.txt, lines 1-1\n1: hello"
  assistant: "final-01"
  user: "resume turn"
```

Resume with `--session <id>`: third run in plugsess-01; `scenario.json` = `[{"text":"session-flag-answer"}]`.

Run (once, earlier):

```sh
PS_MODE=resume PS_LOG=$S/plugsess-01/hooklog-session-flag.jsonl sh $C/probes/harness/run.sh $S/plugsess-01 18779 "session flag turn" --session ses_df19943d5ffeTBXAy0MdDBK6wv
```

Extract:

```sh
tail -1 plugsess-01/out.txt; node plugsess-bin/hooks.js plugsess-01/hooklog-session-flag.jsonl
```

Output:

```text
exit=0
1795538102329 setup {"mode":"resume","ctxKeys":["app","location","options","agent","aisdk","command","credential","event","experimental","generate","model","provider","integration","mcp","permission","plugin","reference","rpc","skill","storage","tool","vcs","websearch","worktree","session","shell"],"sessionKeys":["hook","list","create","get","remove","switchAgent","switchModel","prompt","generate","command","compact","synthetic","interrupt","update","move","wait","context","instructions"]}
1795538102418 hook {"name":"prompt","sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","messageID":"msg_20e692703001fIst9Ft7tifStM","keys":["sessionID","messageID","prompt","metadata","delivery"],"prompt":{"text":"session flag turn","files":[]},"delivery":"steer"}
1795538102422 event session.inbox.enqueued {"sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","inboxID":"m…(+124)
1795538102436 event session.execution.started {"sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv"}}
1795538102437 event session.inbox.delivered {"sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","inboxID":"…(+33)
1795538102442 hook context {"sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":7,"lastMessages":[{"role":"user","text":"resume turn"},{"role":"assistant","text":"resumed-answer"},{"role":"user","text":"session flag turn"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment","api/plugsess-prompt"]]}
1795538102466 hook {"name":"experimental.idle","sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv","background":[],"keys":["sessionID","background"]}
1795538102469 event session.execution.succeeded {"sessionID":"ses_df19943d5ffeTBXAy0MdDBK6wv"}}
1795538102493 event location.shutdown {}}
1795538102493 cleanup {}
```

**Answer.**

- **New session:** no hook fires, and the run's own `session.created` is not delivered to the
  plugin: the top-level session is created before the plugin's `setup` runs (created at
  …102322, setup at …102343), so only the child session's `session.created` is seen. The first
  session-scoped thing a plugin sees is the `prompt` hook, then `session.execution.started`.
- **Resume (`-c` and `--session <id>`):** the same sequence as a new turn (prompt hook →
  `session.inbox.enqueued` → `session.execution.started` → context hook). No event or field says
  "resumed"; the only signal is that the `context` hook's `messages` already hold earlier turns
  (5 and 7 messages here) and no `title` request is made.
- **After compaction:** see P6: the next request starts with a `<conversation-checkpoint>` user
  message; nothing else marks it.
- **Adding context the model sees:** yes, two ways, both shown in the `requests.jsonl` extracts:
  the `context` hook's `system.push(...)` (per request, not stored; must be re-added each request)
  and an instruction entry put from the `prompt` hook (stored with the session, shown as
  `<context key=…>` in the system prompt and still present after `-c`). Neither knows a source
  (startup | resume | clear | compact): the `context` input keys are
  `sessionID, model, system, messages, options, agent, tools`.

## P4: Session end / idle, and whether `opencode run` waits for an async handler

**Source.** `experimental.idle` "Runs when a Session finishes model work and has no pending input,
just before it goes idle. It does not run after interruption, failure, or control-only work such as
a manual compaction. Input admitted by the hook, such as a synthetic message, runs in the same
execution instead of the Session going idle." (`probes/src/packages/plugin/src/promise/session.ts:190-205`).
Core triggers it in the drain loop and then re-checks the inbox
(`probes/src/packages/core/src/session/runner/llm.ts:177-189`). The V1-era `session.idle` event is
marked deprecated (`probes/src/packages/schema/src/session-status-event.ts:45-50`) and nothing in
core or server publishes `SessionStatusEvent` (grep below).

Setup: plugsess-06 with only this plugin at `proj/.opencode/plugins/slow.js`. Each handler sleeps
1.5 s before logging that it finished; the shell appends the time `run.sh` returned.

```plugsess-bin/slow.js
// Probe plugin: does `opencode run` wait for slow async handlers at session end?
import { appendFileSync } from "node:fs"

const LOG = process.env.PS_LOG
const log = (kind, data = {}) => appendFileSync(LOG, JSON.stringify({ t: Date.now(), kind, ...data }) + "\n")
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

export default {
  id: "plugsess-slow",
  async setup(ctx) {
    const controller = new AbortController()
    void (async () => {
      for await (const ev of ctx.event.subscribe({ signal: controller.signal, types: ["session.execution.succeeded", "session.idle", "session.status", "location.shutdown"] })) {
        log("event", { type: ev.type, data: ev.data })
        if (ev.type === "session.execution.succeeded") {
          await sleep(1500)
          log("event-slow-done", { type: ev.type })
        }
      }
    })()
    await ctx.session.hook("experimental.idle", async (e) => {
      log("idle-start", { sessionID: e.sessionID })
      await sleep(1500)
      log("idle-slow-done", { sessionID: e.sessionID })
    })
    return async () => {
      log("cleanup-start")
      await sleep(1500)
      log("cleanup-slow-done")
      controller.abort()
    }
  },
}
```

```plugsess-06/scenario.json
[{"text":"final-06"}]
```

Run (once, earlier):

```sh
(PS_LOG=$S/plugsess-06/hooklog.jsonl sh $C/probes/harness/run.sh $S/plugsess-06 18775 "end probe"; echo "run-returned $(node -e 'console.log(Date.now())')" >> $S/plugsess-06/hooklog.jsonl)
```

Extract:

```sh
cat plugsess-06/hooklog.jsonl; tail -1 plugsess-06/out.txt
```

Output:

```text
{"t":1795538069139,"kind":"idle-start","sessionID":"ses_df1975d9dffeAsqTCjAiIqr31S"}
{"t":1795538070640,"kind":"idle-slow-done","sessionID":"ses_df1975d9dffeAsqTCjAiIqr31S"}
{"t":1795538070644,"kind":"event","type":"session.execution.succeeded","data":{"sessionID":"ses_df1975d9dffeAsqTCjAiIqr31S"}}
{"t":1795538070665,"kind":"cleanup-start"}
{"t":1795538072145,"kind":"event-slow-done","type":"session.execution.succeeded"}
{"t":1795538072147,"kind":"event","type":"location.shutdown","data":{}}
{"t":1795538072166,"kind":"cleanup-slow-done"}
run-returned 1795538072281
exit=0
```

Extract:

```sh
grep -rn "SessionStatusEvent\." oc-src/anomalyco-opencode-6f05140/packages/core/src oc-src/anomalyco-opencode-6f05140/packages/server/src; echo "grep exit: $?"
```

Output:

```text
grep exit: 1
```

**Answer.** `experimental.idle` fires at the end of the turn, and the run waits for its async
handler: `idle-slow-done` is logged 1.5 s after `idle-start`, and `session.execution.succeeded` comes
after it. The `session.execution.succeeded` bus event follows. The plugin's async cleanup runs at
shutdown and is awaited (`cleanup-slow-done` before `run-returned`). The slow event handler also
finished before exit, but it ran inside the cleanup's own 1.5 s wait, so this run does not prove
that event handlers are awaited by themselves. No `session.idle` or `session.status` event was
delivered, though both were subscribed.

## P5: Subagent stop: an event with agent id and last message; continue the child; add context to the parent

**Source.** The `subagent` tool always returns at once with
`{sessionID, status: "running", output: "The subagent is working in the background …"}`
(`probes/src/packages/core/src/tool/plugin/subagent.ts:19-28`, `:194-217`). When the child goes idle,
its last assistant text is delivered to the parent as a synthetic message
`<background-result kind="subagent" sessionID=… state=… description=…>` with metadata
`{source: "subagent", childID, agent, state}`
(`probes/src/packages/core/src/session/subagent-completion.ts:25-34`, `:57-86`). `ctx.session.synthetic`
"Durably admit[s] synthetic session input and schedule[s] execution unless resume is false"
(`probes/src/packages/protocol/src/groups/session.ts:403-421`).

Setup: plugsess-03 in `subagent` mode. In `experimental.idle`, the plugin calls `ctx.session.get`;
for a session with a `parentID` it calls
`ctx.session.synthetic({ sessionID, text: "IDLE-HOOK-CONTINUE: keep going" })` once. In
`tool.execute.after` for `subagent` it rewrites `result.content` (an array of parts; the probe's
string concatenation turns it into `"[object Object]\nAFTER-HOOK-APPENDED"`) and `result.output.output`.

```plugsess-03/scenario.json
[{"tool":"subagent","args":{"agent":"general","description":"probe child","prompt":"CHILD-TASK read a.txt"}},{"tool":"read","args":{"path":"a.txt"}},{"text":"T3"},{"text":"T4"},{"text":"T5"},{"text":"T6"},{"text":"T7"}]
```

Run (once, earlier):

```sh
PS_MODE=subagent PS_LOG=$S/plugsess-03/hooklog.jsonl sh $C/probes/harness/run.sh $S/plugsess-03 18772 "spawn a child"
```

Extract:

```sh
tail -1 plugsess-03/out.txt; node plugsess-bin/hooks.js plugsess-03/hooklog.jsonl | cut -c1-900
```

Output:

```text
exit=0
1795538001043 setup {"mode":"subagent","ctxKeys":["app","location","options","agent","aisdk","command","credential","event","experimental","generate","model","provider","integration","mcp","permission","plugin","reference","rpc","skill","storage","tool","vcs","websearch","worktree","session","shell"],"sessionKeys":["hook","list","create","get","remove","switchAgent","switchModel","prompt","generate","command","compact","synthetic","interrupt","update","move","wait","context","instructions"]}
1795538001235 hook {"name":"prompt","sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","messageID":"msg_20e679c98001n5nyy1wt2oqGVs","keys":["sessionID","messageID","prompt","metadata","delivery"],"prompt":{"text":"spawn a child","files":[]},"delivery":"steer"}
1795538001238 event session.inbox.enqueued {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","inboxID":"ms…(+119)
1795538001262 event session.execution.started {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR"}}
1795538001264 event session.inbox.delivered {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","inboxID":"m…(+32)
1795538001267 hook title {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","keys":["sessionID","model","system","messages","options"],"messages":1,"lastMessages":[{"role":"user","text":"Here is the first message of the conversation:\n\n<message>\nspawn a child\n</message>\n\nReply with only the title."}],"instructionKeys":[]}
1795538001280 hook context {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":1,"lastMessages":[{"role":"user","text":"spawn a child"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538001337 event session.renamed {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","title":"title"}}
1795538001349 hook {"name":"tool.execute.before","tool":"subagent","sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","agent":"build","keys":["tool","info","codemode","sessionID","agent","messageID","id","input"],"input":"{\"agent\":\"general\",\"description\":\"probe child\",\"prompt\":\"CHILD-TASK read a.txt\"}"}
1795538001350 hook {"name":"permission.evaluate","sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","agent":"build","action":"subagent"}
1795538001356 hook {"name":"prompt","sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","messageID":"msg_20e679dcb002d7xuXjiKpOQ3f4","keys":["sessionID","messageID","prompt","metadata","delivery"],"prompt":{"text":"You are a subagent spawned by another session.\nCHILD-TASK read a.txt"},"delivery":"steer"}
1795538001362 hook {"name":"tool.execute.after","tool":"subagent","sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","agent":"build","keys":["tool","info","codemode","sessionID","agent","messageID","id","input","status","result"],"input":"{\"agent\":\"general\",\"description\":\"probe child\",\"prompt\":\"CHILD-TASK read a.txt\"}","status":"completed","result":"{\"output\":{\"sessionID\":\"ses_df1986238ffeOJkzR1yh71TQlc\",\"status\":\"running\",\"output\":\"The subagent is working in the background (sessionID: ses_df1986238ffeOJkzR1yh71TQlc). You will receive a <background-result kind=\\\"subagent\\\"> message when it goes idle, and again after each later round of work.\\nDO NOT sleep, poll, proactively check on its
1795538001371 event session.created {"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","projectID":"6b751a0…(+333)
1795538001380 hook context {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":3,"lastMessages":[{"role":"user","text":"spawn a child"},{"role":"assistant","text":"tool-call"},{"role":"tool","text":"tool-result"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538001383 event session.inbox.enqueued {"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","inboxID":"ms…(+164)
1795538001391 hook context {"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","agent":"general","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":1,"lastMessages":[{"role":"user","text":"You are a subagent spawned by another session.\nCHILD-TASK read a.txt"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538001392 event session.execution.started {"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc"}}
1795538001397 event session.inbox.delivered {"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","inboxID":"m…(+32)
1795538001424 hook {"name":"tool.execute.before","tool":"read","sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","agent":"build","keys":["tool","info","codemode","sessionID","agent","messageID","id","input"],"input":"{\"path\":\"a.txt\"}"}
1795538001426 hook {"name":"permission.evaluate","sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","agent":"build","action":"read"}
1795538001435 hook {"name":"experimental.idle","sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","background":[],"keys":["sessionID","background"],"session":"{\"id\":\"ses_df1986238ffeOJkzR1yh71TQlc\",\"parentID\":\"ses_df19866a3ffezyQ4vBrDg1zwYR\",\"projectID\":\"6b751a0476d248b9d04116c07f2cb9045e5ae343\",\"agent\":\"general\",\"cost\":0,\"tokens\":{\"input\":0,\"output\":0,\"reasoning\":0,\"cache\":{\"read\":0,\"write\":0}},\"time\":{\"created\":1795538001353,\"updated\":1795538001433,\"busy\":1795538001384},\"title\":\"probe child\",\"location\":{\"directory\":\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugsess-03/proj\"}}"}
1795538001436 synthetic {"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","r":"{\"id\":\"msg_20e679e1b0011uYfVB59utWsEu\",\"sessionID\":\"ses_df1986238ffeOJkzR1yh71TQlc\",\"time\":{\"created\":1795538001435},\"type\":\"synthetic\",\"payload\":{\"text\":\"IDLE-HOOK-CONTINUE: keep going\"},\"delivery\":\"steer\"}"}
1795538001445 hook context {"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","agent":"general","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":3,"lastMessages":[{"role":"user","text":"You are a subagent spawned by another session.\nCHILD-TASK read a.txt"},{"role":"assistant","text":"T3"},{"role":"user","text":"IDLE-HOOK-CONTINUE: keep going"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538001447 event session.inbox.enqueued {"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","inboxID":"msg_20e679e1b0011uYfVB59utWsEu","item":{"type":"synthetic","payload":{"text":"IDLE-HOOK-CONTINUE: keep going"},"delivery":"steer"}}}
1795538001447 event session.inbox.delivered {"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","inboxID":"…(+33)
1795538001457 hook {"name":"experimental.idle","sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","background":[],"keys":["sessionID","background"],"session":"{\"id\":\"ses_df1986238ffeOJkzR1yh71TQlc\",\"parentID\":\"ses_df19866a3ffezyQ4vBrDg1zwYR\",\"projectID\":\"6b751a0476d248b9d04116c07f2cb9045e5ae343\",\"agent\":\"general\",\"cost\":0,\"tokens\":{\"input\":0,\"output\":0,\"reasoning\":0,\"cache\":{\"read\":0,\"write\":0}},\"time\":{\"created\":1795538001353,\"updated\":1795538001455,\"busy\":1795538001384},\"title\":\"probe child\",\"location\":{\"directory\":\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugsess-03/proj\"}}"}
1795538001461 event session.execution.succeeded {"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc"}}
1795538001461 event session.inbox.enqueued {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","inboxID":"msg_20e679dd0001GJu3MzBh0hE5dK","item":{"type":"synthetic","payload":{"text":"<background-result kind=\"subagent\" sessionID=\"ses_df1986238ffeOJkzR1yh71TQlc\" s…(+239)
1795538001464 hook {"name":"tool.execute.after","tool":"read","sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","agent":"build","keys":["tool","info","codemode","sessionID","agent","messageID","id","input","status","result"],"input":"{\"path\":\"a.txt\"}","status":"completed","result":"{\"output\":{\"type\":\"text\",\"offset\":1,\"lineCount\":1,\"content\":\"hello\\n\"},\"content\":[{\"type\":\"text\",\"text\":\"Read file a.txt, lines 1-1\\n1: hello\"}],\"metadata\":{\"truncated\":false}}"}
1795538001475 hook context {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":6,"lastMessages":[{"role":"assistant","text":"tool-call"},{"role":"tool","text":"tool-result"},{"role":"user","meta":{"source":"subagent","childID":"ses_df1986238ffeOJkzR1yh71TQlc","agent":"General","state":"completed"},"text":"<background-result kind=\"subagent\" sessionID=\"ses_df1986238ffeOJkzR1yh71TQlc\" state=\"completed\" description=\"probe child\">\nT4\n</background-result>"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538001477 event session.inbox.delivered {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","inboxID":"…(+33)
1795538001490 hook {"name":"experimental.idle","sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR","background":[],"keys":["sessionID","background"],"session":"{\"id\":\"ses_df19866a3ffezyQ4vBrDg1zwYR\",\"projectID\":\"6b751a0476d248b9d04116c07f2cb9045e5ae343\",\"cost\":0,\"tokens\":{\"input\":0,\"output\":0,\"reasoning\":0,\"cache\":{\"read\":0,\"write\":0}},\"time\":{\"created\":1795538001036,\"updated\":1795538001486,\"busy\":1795538001239},\"title\":\"title\",\"location\":{\"directory\":\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugsess-03/proj\"}}"}
1795538001494 event session.execution.succeeded {"sessionID":"ses_df19866a3ffezyQ4vBrDg1zwYR"}}
1795538001517 event location.shutdown {}}
1795538001518 cleanup {}
```

Extract:

```sh
node plugsess-bin/reqs.js plugsess-03/requests.jsonl AFTER-HOOK-APPENDED IDLE-HOOK-CONTINUE | cut -c1-400
```

Output:

```text
requests: 6
req 0 tools=0 messages=2
  marker AFTER-HOOK-APPENDED: absent
  marker IDLE-HOOK-CONTINUE: absent
  user: "Here is the first message of the conversation:\n\n<message>\nspawn a child\n</message>\n\nReply with only the title."
req 1 tools=14 messages=2
  marker AFTER-HOOK-APPENDED: absent
  marker IDLE-HOOK-CONTINUE: absent
  user: "spawn a child"
req 2 tools=14 messages=4
  marker AFTER-HOOK-APPENDED: "ASK read a.txt\\\"}\"}}]},{\"role\":\"tool\",\"tool_call_id\":\"call_1\",\"content\":\"[object Object]\\nAFTER-HOOK-APPENDED\"}],\"tools\":[{\"type\":\"function\",\"functio"
  marker IDLE-HOOK-CONTINUE: absent
  user: "spawn a child"
  assistant: [{"id":"call_1","type":"function","function":{"name":"subagent","arguments":"{\"agent\":\"general\",\"description\":\"probe child\",\"prompt\":\"CHILD-TASK read a.txt\"}"}}]
  tool: "[object Object]\nAFTER-HOOK-APPENDED"
req 3 tools=12 messages=2
  marker AFTER-HOOK-APPENDED: absent
  marker IDLE-HOOK-CONTINUE: absent
  user: "You are a subagent spawned by another session.\nCHILD-TASK read a.txt"
req 4 tools=12 messages=4
  marker AFTER-HOOK-APPENDED: absent
  marker IDLE-HOOK-CONTINUE: "n.\\nCHILD-TASK read a.txt\"},{\"role\":\"assistant\",\"content\":\"T3\"},{\"role\":\"user\",\"content\":\"IDLE-HOOK-CONTINUE: keep going\"}],\"tools\":[{\"type\":\"functi"
  user: "You are a subagent spawned by another session.\nCHILD-TASK read a.txt"
  assistant: "T3"
  user: "IDLE-HOOK-CONTINUE: keep going"
req 5 tools=14 messages=7
  marker AFTER-HOOK-APPENDED: "ASK read a.txt\\\"}\"}}]},{\"role\":\"tool\",\"tool_call_id\":\"call_1\",\"content\":\"[object Object]\\nAFTER-HOOK-APPENDED\"},{\"role\":\"assistant\",\"content\":null,\"t"
  marker IDLE-HOOK-CONTINUE: absent
  user: "spawn a child"
  assistant: [{"id":"call_1","type":"function","function":{"name":"subagent","arguments":"{\"agent\":\"general\",\"description\":\"probe child\",\"prompt\":\"CHILD-TASK read a.txt\"}"}}]
  tool: "[object Object]\nAFTER-HOOK-APPENDED"
  assistant: [{"id":"call_2","type":"function","function":{"name":"read","arguments":"{\"path\":\"a.txt\"}"}}]
  tool: "Read file a.txt, lines 1-1\n1: hello"
  user: "<background-result kind=\"subagent\" sessionID=\"ses_df1986238ffeOJkzR1yh71TQlc\" state=\"completed\" description=\"probe child\">\nT4\n</background-result>"
```

Extract:

```sh
node -e 'const l=require("fs").readFileSync("plugsess-03/hooklog.jsonl","utf8").trim().split("\n").map(JSON.parse);
const a=l.find(x=>x.name==="tool.execute.after"&&x.tool==="subagent"); console.log("subagent after-hook input:", a.input); console.log("subagent after-hook result:", a.result);
const c=l.filter(x=>x.name==="context"&&x.agent==="build").pop(); console.log("last parent context hook, last message:", JSON.stringify(c.lastMessages.at(-1)));'
```

Output:

```text
subagent after-hook input: {"agent":"general","description":"probe child","prompt":"CHILD-TASK read a.txt"}
subagent after-hook result: {"output":{"sessionID":"ses_df1986238ffeOJkzR1yh71TQlc","status":"running","output":"The subagent is working in the background (sessionID: ses_df1986238ffeOJkzR1yh71TQlc). You will receive a <background-result kind=\"subagent\"> message when it goes idle, and again after each later round of work.\nDO NOT sleep, poll, proactively check on its progress, or duplicate this subagent's work.\nWork on non-overlapping tasks, or briefly tell the user what you launched and end your response.\nTo send it follow-ups or change its instructions while it runs, call subagent again with this sessionID."},"cont…(+617)
last parent context hook, last message: {"role":"user","meta":{"source":"subagent","childID":"ses_df1986238ffeOJkzR1yh71TQlc","agent":"General","state":"completed"},"text":"<background-result kind=\"subagent\" sessionID=\"ses_df1986238ffeOJkzR1yh71TQlc\" state=\"completed\" description=\"probe child\">\nT4\n</background-result>"}
```

**Answer.**

- **No subagent-stop hook or event.** The closest is `experimental.idle` on the child session
  (`sessionID` of the child; `ctx.session.get` gives `parentID` and `agent: "general"`). Its input has
  no last message; a plugin would read it (e.g. `ctx.session.context`, not probed here). The
  delivery to the parent shows as `session.inbox.enqueued` on the parent with a synthetic
  `<background-result>` item.
- **Make the child continue:** yes. `ctx.session.synthetic` from the child's idle hook admitted
  `IDLE-HOOK-CONTINUE` (req 4); the child ran again, its idle hook fired again, and only then was
  the result (`T4`, the last text) delivered to the parent. `opencode run` waited for all of it
  (exit 0).
- **Tool after-hook on `subagent`:** sees `input.agent`, `description`, `prompt` and a result that is
  only the "working in the background" notice; the final text is not there. Its change does reach
  the parent model (req 2 and 5 tool message).
- **Context to the parent:** the parent's next `context` hook sees the background-result user message
  with metadata `{source:"subagent", childID, agent:"General", state:"completed"}` (agent display
  name, not id), so it can add `system` text at that point.

## P6: The `compaction` hook: input / output, and adding context after compaction

**Source.** `SessionCompaction extends SessionContext { result?: SessionCompactionResult }`
(`probes/src/packages/plugin/src/promise/session.ts:45-56`). Docs: "`compaction` runs for checkpoint
summaries. `messages` is the transcript being summarized; OpenCode appends its summary prompt after
hooks run. Set `result` to record the compaction yourself and skip the model call"; changes affect
only the outgoing model call (`probes/docs/build-plugins/server.mdx:1441-1461`). Compaction is
requested with `POST /api/session/:sessionID/compact` (`probes/src/packages/protocol/src/groups/session.ts:444-460`).

Setup: plugsess-04 (`compact` mode) and plugsess-05 (`compact-result` mode) call
`ctx.session.compact({ sessionID })` from the first `experimental.idle`. The `compaction` hook pushes
`COMPACTION-HOOK-SYSTEM` onto `system`; in plugsess-05 it also sets
`event.result = { summary: "PLUGIN-SUPPLIED-SUMMARY" }`. plugsess-05 is then resumed with `-c`.

```plugsess-04/scenario.json
[{"text":"first-turn-answer"},{"text":"after-compaction"},{"text":"x3"}]
```

Run (once, earlier):

```sh
PS_MODE=compact PS_LOG=$S/plugsess-04/hooklog.jsonl sh $C/probes/harness/run.sh $S/plugsess-04 18773 "first turn"
```

Extract:

```sh
tail -1 plugsess-04/out.txt; node plugsess-bin/hooks.js plugsess-04/hooklog.jsonl | grep -E "compaction|compact-called|idle"
```

Output:

```text
exit=0
1795538001385 hook {"name":"experimental.idle","sessionID":"ses_df19866a2ffeiJEqOzJfjlEaD5","background":[],"keys":["sessionID","background"],"session":"{\"id\":\"ses_df19866a2ffeiJEqOzJfjlEaD5\",\"projectID\":\"4f98c90c26ef87b66be1660fa03cd26a98d9eb2d\",\"cost\":0,\"tokens\":{\"input\":0,\"output\":0,\"reasoning\":0,\"cache\":{\"read\":0,\"write\":0}},\"time\":{\"created\":1795538001058,\"updated\":1795538001383,\"busy\":1795538001301},\"title\":\"title\",\"location\":{\"directory\":\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugsess-04/proj\"}}"}
1795538001386 compact-called {"r":"{\"id\":\"msg_20e679dea001GobzJB6IAwsMUR\",\"sessionID\":\"ses_df19866a2ffeiJEqOzJfjlEaD5\",\"time\":{\"created\":1795538001386},\"type\":\"compaction\",\"payload\":{},\"delivery\":\"steer\"}"}
1795538001394 hook compaction {"sessionID":"ses_df19866a2ffeiJEqOzJfjlEaD5","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":2,"lastMessages":[{"role":"user","text":"first turn"},{"role":"assistant","text":"first-turn-answer"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538001397 event session.inbox.enqueued {"sessionID":"ses_df19866a2ffeiJEqOzJfjlEaD5","inboxID":"msg_20e679dea001GobzJB6IAwsMUR","item":{"type":"compaction","payload":{},"delivery":"steer"}}}
1795538001397 event session.compaction.started {"sessionID":"ses_df19866a2ffeiJEqOzJfjlEaD5","reason"…(+54)
1795538001418 hook {"name":"experimental.idle","sessionID":"ses_df19866a2ffeiJEqOzJfjlEaD5","background":[],"keys":["sessionID","background"],"session":"{\"id\":\"ses_df19866a2ffeiJEqOzJfjlEaD5\",\"projectID\":\"4f98c90c26ef87b66be1660fa03cd26a98d9eb2d\",\"cost\":0,\"tokens\":{\"input\":0,\"output\":0,\"reasoning\":0,\"cache\":{\"read\":0,\"write\":0}},\"time\":{\"created\":1795538001058,\"updated\":1795538001386,\"busy\":1795538001301},\"title\":\"title\",\"location\":{\"directory\":\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugsess-04/proj\"}}"}
1795538001418 event session.compaction.failed {"sessionID":"ses_df19866a2ffeiJEqOzJfjlEaD5","reason":…(+241)
```

Extract:

```sh
node plugsess-bin/reqs.js plugsess-04/requests.jsonl COMPACTION-HOOK-SYSTEM CTX-HOOK-SYSTEM | cut -c1-300
```

Output:

```text
requests: 4
req 0 tools=0 messages=2
  marker COMPACTION-HOOK-SYSTEM: absent
  marker CTX-HOOK-SYSTEM: absent
  user: "Here is the first message of the conversation:\n\n<message>\nfirst turn\n</message>\n\nReply with only the title."
req 1 tools=14 messages=2
  marker COMPACTION-HOOK-SYSTEM: absent
  marker CTX-HOOK-SYSTEM: "all `tools.opencode.session.move` and make the worktree the session's working directory.\\nCTX-HOOK-SYSTEM agent=build\"},{\"role\":\"user\",\"content\":"
  user: "first turn"
req 2 tools=14 messages=4
  marker COMPACTION-HOOK-SYSTEM: "all `tools.opencode.session.move` and make the worktree the session's working directory.\\nCOMPACTION-HOOK-SYSTEM\"},{\"role\":\"user\",\"content\":\"first turn\""
  marker CTX-HOOK-SYSTEM: absent
  user: "first turn"
  assistant: "first-turn-answer"
  user: "You MUST summarize the conversation above into a structured summary that will be given to another agent to resume the work.\n\nSummarize only what the user and the assistant said and did. Leave out instructions and setup the assistant was given rather than told by the user: repository conve
req 3 tools=14 messages=5
  marker COMPACTION-HOOK-SYSTEM: "all `tools.opencode.session.move` and make the worktree the session's working directory.\\nCOMPACTION-HOOK-SYSTEM\"},{\"role\":\"user\",\"content\":\"first turn\""
  marker CTX-HOOK-SYSTEM: absent
  user: "first turn"
  assistant: "first-turn-answer"
  user: "You MUST summarize the conversation above into a structured summary that will be given to another agent to resume the work.\n\nSummarize only what the user and the assistant said and did. Leave out instructions and setup the assistant was given rather than told by the user: repository conve
  user: "The previous response did not fill in the required summary template. Do not call tools. Return the summary as text using the exact section headings from the template."
```

Extract:

```sh
node plugsess-bin/hooks.js plugsess-04/hooklog.jsonl full | grep "session\.compaction\."
```

Output:

```text
1795538001397 event session.compaction.started {"sessionID":"ses_df19866a2ffeiJEqOzJfjlEaD5","reason"…(+54)
1795538001405 event session.compaction.delta {"sessionID":"ses_df19866a2ffeiJEqOzJfjlEaD5","text":"after-compaction"}}
1795538001413 event session.compaction.delta {"sessionID":"ses_df19866a2ffeiJEqOzJfjlEaD5","text":"x3"}}
1795538001418 event session.compaction.failed {"sessionID":"ses_df19866a2ffeiJEqOzJfjlEaD5","reason":…(+241)
```

```plugsess-05/run1/scenario.json
[{"text":"first-turn-answer"},{"text":"after-compaction"},{"text":"x3"}]
```

Run (once, earlier):

```sh
PS_MODE=compact-result PS_LOG=$S/plugsess-05/hooklog.jsonl sh $C/probes/harness/run.sh $S/plugsess-05 18774 "first turn"   # outputs then copied to plugsess-05/run1/
```

Extract:

```sh
tail -1 plugsess-05/run1/out.txt; node plugsess-bin/hooks.js plugsess-05/hooklog.jsonl | grep -E "compaction|compact-called|idle"; node plugsess-bin/reqs.js plugsess-05/run1/requests.jsonl | cut -c1-200
```

Output:

```text
exit=0
1795538001405 hook {"name":"experimental.idle","sessionID":"ses_df1986693ffeu1y9jYb1N1a6u1","background":[],"keys":["sessionID","background"],"session":"{\"id\":\"ses_df1986693ffeu1y9jYb1N1a6u1\",\"projectID\":\"9b850edc7f59eaf3204d31f9cefccecde9b009e5\",\"cost\":0,\"tokens\":{\"input\":0,\"output\":0,\"reasoning\":0,\"cache\":{\"read\":0,\"write\":0}},\"time\":{\"created\":1795538001065,\"updated\":1795538001402,\"busy\":1795538001294},\"title\":\"title\",\"location\":{\"directory\":\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugsess-05/proj\"}}"}
1795538001406 compact-called {"r":"{\"id\":\"msg_20e679dfd001zfkOvP4bNV9GTK\",\"sessionID\":\"ses_df1986693ffeu1y9jYb1N1a6u1\",\"time\":{\"created\":1795538001405},\"type\":\"compaction\",\"payload\":{},\"delivery\":\"steer\"}"}
1795538001415 hook compaction {"sessionID":"ses_df1986693ffeu1y9jYb1N1a6u1","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":2,"lastMessages":[{"role":"user","text":"first turn"},{"role":"assistant","text":"first-turn-answer"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538001419 hook {"name":"experimental.idle","sessionID":"ses_df1986693ffeu1y9jYb1N1a6u1","background":[],"keys":["sessionID","background"],"session":"{\"id\":\"ses_df1986693ffeu1y9jYb1N1a6u1\",\"projectID\":\"9b850edc7f59eaf3204d31f9cefccecde9b009e5\",\"cost\":0,\"tokens\":{\"input\":0,\"output\":0,\"reasoning\":0,\"cache\":{\"read\":0,\"write\":0}},\"time\":{\"created\":1795538001065,\"updated\":1795538001405,\"busy\":1795538001294},\"title\":\"title\",\"location\":{\"directory\":\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugsess-05/proj\"}}"}
1795538001421 event session.inbox.enqueued {"sessionID":"ses_df1986693ffeu1y9jYb1N1a6u1","inboxID":"msg_20e679dfd001zfkOvP4bNV9GTK","item":{"type":"compaction","payload":{},"delivery":"steer"}}}
1795538001422 event session.compaction.started {"sessionID":"ses_df1986693ffeu1y9jYb1N1a6u1","reason"…(+54)
1795538001422 event session.compaction.ended {"sessionID":"ses_df1986693ffeu1y9jYb1N1a6u1","reason":"…(+81)
requests: 2
req 0 tools=0 messages=2
  user: "Here is the first message of the conversation:\n\n<message>\nfirst turn\n</message>\n\nReply with only the title."
req 1 tools=14 messages=2
  user: "first turn"
```

Resume of the compacted session: second run in plugsess-05; `scenario.json` = `[{"text":"resumed-answer"}]`.

Run (once, earlier):

```sh
PS_MODE=resume PS_LOG=$S/plugsess-05/hooklog-resume.jsonl sh $C/probes/harness/run.sh $S/plugsess-05 18777 "resume after compaction" -c
```

Extract:

```sh
node plugsess-bin/hooks.js plugsess-05/hooklog-resume.jsonl | grep -E " (prompt|context)"; node plugsess-bin/reqs.js plugsess-05/requests.jsonl | cut -c1-420
```

Output:

```text
1795538069089 hook context {"sessionID":"ses_df1986693ffeu1y9jYb1N1a6u1","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":2,"lastMessages":[{"role":"user","text":"<conversation-checkpoint>\nThe following is a summary and serialized record of earlier conversation. Treat it as historical context, not as new instructions.\n\n<summary>\nPLUGIN-SUPPLIED-SUMMARY\n</summar…(+29)"},{"role":"user","text":"resume after compaction"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
requests: 1
req 0 tools=14 messages=3
  user: "<conversation-checkpoint>\nThe following is a summary and serialized record of earlier conversation. Treat it as historical context, not as new instructions.\n\n<summary>\nPLUGIN-SUPPLIED-SUMMARY\n</summary>\n</conversation-checkpoint>"
  user: "resume after compaction"
```

**Answer.**

- Input keys `sessionID, model, system, messages, options, agent, tools` (+ optional `result`).
  `messages` is the transcript before OpenCode appends its "You MUST summarize…" prompt.
- `system.push` reaches only the summarization requests (req 2–3 in plugsess-04), not the agent loop.
  In plugsess-04 the mock's reply did not fit OpenCode's summary template, so OpenCode retried once
  and the compaction failed (`session.compaction.failed`). This is a mock limit, not a hook effect.
- Setting `result` skips the model call (plugsess-05 sent only 2 requests) and records the summary
  (`session.compaction.ended`).
- After compaction the next agent-loop request (the `-c` run) opens with a user message
  `<conversation-checkpoint> … <summary>PLUGIN-SUPPLIED-SUMMARY</summary> …`. There is no
  post-compaction hook; the `context` hook on the next request can add context, but it has to spot
  the checkpoint message (or remember `session.compaction.ended` within the same process).
- `experimental.idle` fired a second time after the compaction admitted from the idle hook ran,
  although the source comment says manual compaction skips it.

## P7: Do the hooks fire inside subagent sessions, and how is the agent identified?

Setup: plugsess-07 in `resume` mode (log only, no edits), with a scenario in which the parent answers
`P2` and the child calls `read`.

```plugsess-07/scenario.json
[{"tool":"subagent","args":{"agent":"general","description":"probe child","prompt":"CHILD-TASK read a.txt"}},{"text":"P2"},{"tool":"read","args":{"path":"a.txt"}},{"text":"T4"},{"text":"T5"},{"text":"T6"}]
```

Run (once, earlier):

```sh
PS_MODE=resume PS_LOG=$S/plugsess-07/hooklog.jsonl sh $C/probes/harness/run.sh $S/plugsess-07 18778 "spawn a child"
```

Extract:

```sh
tail -1 plugsess-07/out.txt; node plugsess-bin/hooks.js plugsess-07/hooklog.jsonl | grep -v " event " | cut -c1-560
```

Output:

```text
exit=0
1795538102343 setup {"mode":"resume","ctxKeys":["app","location","options","agent","aisdk","command","credential","event","experimental","generate","model","provider","integration","mcp","permission","plugin","reference","rpc","skill","storage","tool","vcs","websearch","worktree","session","shell"],"sessionKeys":["hook","list","create","get","remove","switchAgent","switchModel","prompt","generate","command","compact","synthetic","interrupt","update","move","wait","context","instructions"]}
1795538102451 hook {"name":"prompt","sessionID":"ses_df196d900ffenNjsKDabZzgzov","messageID":"msg_20e692842001i7hqpka73UI74G","keys":["sessionID","messageID","prompt","metadata","delivery"],"prompt":{"text":"spawn a child","files":[]},"delivery":"steer"}
1795538102474 hook title {"sessionID":"ses_df196d900ffenNjsKDabZzgzov","keys":["sessionID","model","system","messages","options"],"messages":1,"lastMessages":[{"role":"user","text":"Here is the first message of the conversation:\n\n<message>\nspawn a child\n</message>\n\nReply with only the title."}],"instructionKeys":[]}
1795538102478 hook context {"sessionID":"ses_df196d900ffenNjsKDabZzgzov","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":1,"lastMessages":[{"role":"user","text":"spawn a child"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538102504 hook {"name":"tool.execute.before","tool":"subagent","sessionID":"ses_df196d900ffenNjsKDabZzgzov","agent":"build","keys":["tool","info","codemode","sessionID","agent","messageID","id","input"],"input":"{\"agent\":\"general\",\"description\":\"probe child\",\"prompt\":\"CHILD-TASK read a.txt\"}"}
1795538102505 hook {"name":"permission.evaluate","sessionID":"ses_df196d900ffenNjsKDabZzgzov","agent":"build","action":"subagent"}
1795538102509 hook {"name":"prompt","sessionID":"ses_df196d716ffebFvbQGfN3ruuUG","messageID":"msg_20e6928ec0021tiqremFtGTnHn","keys":["sessionID","messageID","prompt","metadata","delivery"],"prompt":{"text":"You are a subagent spawned by another session.\nCHILD-TASK read a.txt"},"delivery":"steer"}
1795538102515 hook {"name":"tool.execute.after","tool":"subagent","sessionID":"ses_df196d900ffenNjsKDabZzgzov","agent":"build","keys":["tool","info","codemode","sessionID","agent","messageID","id","input","status","result"],"input":"{\"agent\":\"general\",\"description\":\"probe child\",\"prompt\":\"CHILD-TASK read a.txt\"}","status":"completed","result":"{\"output\":{\"sessionID\":\"ses_df196d716ffebFvbQGfN3ruuUG\",\"status\":\"running\",\"output\":\"The subagent is working in the background (sessionID: ses_df196d716ffebFvbQGfN3ruuUG). You will receive 
1795538102522 hook context {"sessionID":"ses_df196d900ffenNjsKDabZzgzov","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":3,"lastMessages":[{"role":"user","text":"spawn a child"},{"role":"assistant","text":"tool-call"},{"role":"tool","text":"tool-result"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538102540 hook context {"sessionID":"ses_df196d716ffebFvbQGfN3ruuUG","agent":"general","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":1,"lastMessages":[{"role":"user","text":"You are a subagent spawned by another session.\nCHILD-TASK read a.txt"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538102547 hook {"name":"experimental.idle","sessionID":"ses_df196d900ffenNjsKDabZzgzov","background":[{"kind":"subagent","sessionID":"ses_df196d900ffenNjsKDabZzgzov","childSessionID":"ses_df196d716ffebFvbQGfN3ruuUG","agent":"General","description":"probe child"}],"keys":["sessionID","background"],"session":"{\"id\":\"ses_df196d900ffenNjsKDabZzgzov\",\"projectID\":\"651b66e8642980b4d56a3865bf2f7ed184b5a61b\",\"cost\":0,\"tokens\":{\"input\":0,\"output\":0,\"reasoning\":0,\"cache\":{\"read\":0,\"write\":0}},\"time\":{\"created\":1795538102322,\"updated
1795538102555 hook {"name":"tool.execute.before","tool":"read","sessionID":"ses_df196d716ffebFvbQGfN3ruuUG","agent":"general","keys":["tool","info","codemode","sessionID","agent","messageID","id","input"],"input":"{\"path\":\"a.txt\"}","parentID":"ses_df196d900ffenNjsKDabZzgzov"}
1795538102555 hook {"name":"permission.evaluate","sessionID":"ses_df196d716ffebFvbQGfN3ruuUG","agent":"general","action":"read"}
1795538102557 hook {"name":"tool.execute.after","tool":"read","sessionID":"ses_df196d716ffebFvbQGfN3ruuUG","agent":"general","keys":["tool","info","codemode","sessionID","agent","messageID","id","input","status","result"],"input":"{\"path\":\"a.txt\"}","status":"completed","result":"{\"output\":{\"type\":\"text\",\"offset\":1,\"lineCount\":1,\"content\":\"hello\\n\"},\"content\":[{\"type\":\"text\",\"text\":\"Read file a.txt, lines 1-1\\n1: hello\"}],\"metadata\":{\"truncated\":false}}","parentID":"ses_df196d900ffenNjsKDabZzgzov"}
1795538102562 hook context {"sessionID":"ses_df196d716ffebFvbQGfN3ruuUG","agent":"general","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":3,"lastMessages":[{"role":"user","text":"You are a subagent spawned by another session.\nCHILD-TASK read a.txt"},{"role":"assistant","text":"tool-call"},{"role":"tool","text":"tool-result"}],"instructionKeys":[["core/codemode","core/skill-guidance","core/date","core/environment"]]}
1795538102569 hook {"name":"experimental.idle","sessionID":"ses_df196d716ffebFvbQGfN3ruuUG","background":[],"keys":["sessionID","background"],"session":"{\"id\":\"ses_df196d716ffebFvbQGfN3ruuUG\",\"parentID\":\"ses_df196d900ffenNjsKDabZzgzov\",\"projectID\":\"651b66e8642980b4d56a3865bf2f7ed184b5a61b\",\"agent\":\"general\",\"cost\":0,\"tokens\":{\"input\":0,\"output\":0,\"reasoning\":0,\"cache\":{\"read\":0,\"write\":0}},\"time\":{\"created\":1795538102507,\"updated\":1795538102567,\"busy\":1795538102525},\"title\":\"probe child\",\"location\":{\"directo
1795538102576 hook context {"sessionID":"ses_df196d900ffenNjsKDabZzgzov","agent":"build","keys":["sessionID","model","system","messages","options","agent","tools"],"messages":5,"lastMessages":[{"role":"tool","text":"tool-result"},{"role":"assistant","text":"P2"},{"role":"user","meta":{"source":"subagent","childID":"ses_df196d716ffebFvbQGfN3ruuUG","agent":"General","state":"completed"},"text":"<background-result kind=\"subagent\" sessionID=\"ses_df196d716ffebFvbQGfN3ruuUG\" state=\"completed\" description=\"probe child\">\nT4\n</background-result>"}],"ins
1795538102582 hook {"name":"experimental.idle","sessionID":"ses_df196d900ffenNjsKDabZzgzov","background":[],"keys":["sessionID","background"],"session":"{\"id\":\"ses_df196d900ffenNjsKDabZzgzov\",\"projectID\":\"651b66e8642980b4d56a3865bf2f7ed184b5a61b\",\"cost\":0,\"tokens\":{\"input\":0,\"output\":0,\"reasoning\":0,\"cache\":{\"read\":0,\"write\":0}},\"outcome\":\"succeeded\",\"time\":{\"created\":1795538102322,\"updated\":1795538102580,\"busy\":1795538102571,\"idle\":1795538102547},\"title\":\"title\",\"location\":{\"directory\":\"/private/tmp/claude-
1795538102600 cleanup {}
```

**Answer.** Yes, every hook fires in the child session (`ses_df196d716…`): `prompt` (with the
`You are a subagent spawned by another session.` prefix), `context`, `tool.execute.before/after`,
`permission.evaluate` and `experimental.idle`. Tool, permission and context hooks carry
`agent: "general"` and the child's `sessionID`. `prompt` and `experimental.idle` carry only
`sessionID`; `ctx.session.get` returns `parentID` and `agent` for the child (the top-level session
has no `parentID` and no `agent` field). The parent's idle hook lists the running child in
`background` (`kind: "subagent"`, `childSessionID`, `agent: "General"`).

