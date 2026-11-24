# Probe results: plugin loading and tool hooks (OpenCode v2.4.1)

Topic: PLUGIN LOADING AND TOOL HOOKS. Ports 18750-18761. Probe dirs `<scratchpad>/plugtool-NN/`, where
`<scratchpad>` = `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad`.
Every run uses the shared harness (`probes/harness/run.sh`, mock model, isolated HOME/XDG, `--standalone --auto`).
Runtime facts measured by the probes themselves: OpenCode `ctx.app.version` = `2.4.1`, plugin runtime Bun `1.4.4`
(P1), `airsl 0.1.5` (P2). Each `$ command` block below ran as `{ command ; } 2>&1`, so stderr is shown in place.
Helper script (not a probe subject) used to read the model-visible tool results from `requests.jsonl`:

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-scripts/toolresults.js
// usage: node toolresults.js <requests.jsonl>
// Prints every tool-result message (role "tool") of the last request that offered tools, plus the
// assistant tool calls they answer, as the model saw them.
const fs = require("fs");
const lines = fs.readFileSync(process.argv[2], "utf8").trim().split("\n").map((l) => JSON.parse(l));
const withTools = lines.filter((x) => x.body && x.body.tools && x.body.tools.length);
const last = withTools[withTools.length - 1];
for (const m of last.body.messages) {
  if (m.role === "tool") console.log(JSON.stringify({ role: m.role, tool_call_id: m.tool_call_id, content: m.content }));
  if (m.role === "assistant" && m.tool_calls) console.log(JSON.stringify({ role: m.role, tool_calls: m.tool_calls.map((c) => ({ id: c.id, name: c.function.name, arguments: c.function.arguments })) }));
}
```

Source citations point at `probes/src/<path>` (copied from the v2.4.1 tag) and `probes/docs/<file>.mdx`.

## P1: Does a minimal plugin with no imports load, what does an import need, what are the load errors, what is in ctx?

### P1a: plain `.js` and `.ts` with no imports

**Setup**

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj/.opencode/plugins/plainjs.js
export default {
  id: "plainjs",
  setup: async (ctx) => {
    const fs = await import("node:fs");
    fs.appendFileSync("/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/plug.log", JSON.stringify({
      plugin: "plainjs",
      ctxKeys: Object.keys(ctx),
      location: ctx.location,
      options: ctx.options,
      cwd: process.cwd(),
      bun: typeof Bun === "undefined" ? null : Bun.version,
      node: process.versions.node,
      appVersion: ctx.app && ctx.app.version,
    }) + "\n");
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj/.opencode/plugins/plaints.ts
type Ctx = { location: { directory: string } };
export default {
  id: "plaints",
  setup: async (ctx: Ctx) => {
    const fs = await import("node:fs");
    fs.appendFileSync("/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/plug.log", JSON.stringify({ plugin: "plaints", directory: ctx.location.directory }) + "\n");
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/scenario.json
[{"text":"hi"}]
```

**Command**

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01 18750 "say hi"
```

**Output**

```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/plug.log
```
```text
{"plugin":"plainjs","ctxKeys":["app","location","options","agent","aisdk","command","credential","event","experimental","generate","model","provider","integration","mcp","permission","plugin","reference","rpc","skill","storage","tool","vcs","websearch","worktree","session","shell"],"location":{"directory":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj","project":{"id":"9a937c3619cd4b0320add0e2da5739ae569cddb6","directory":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj","canonical":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj"}},"options":{},"cwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj","bun":"1.4.4","node":"26.3.0","appVersion":"2.4.1"}
{"plugin":"plaints","directory":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj"}
```
```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/out.txt; cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/err.txt
```
```text
{"type":"step_start","timestamp":1795537881780,"sessionID":"ses_df19a36ceffefCFrtSV1Mdp91b","part":{"id":"prt_20e65cab40018n1EXRyCl1WJ16","sessionID":"ses_df19a36ceffefCFrtSV1Mdp91b","messageID":"msg_20e65caa40011oah3WcsbA38TC","type":"step-start"}}
{"type":"text","timestamp":1795537881783,"sessionID":"ses_df19a36ceffefCFrtSV1Mdp91b","part":{"id":"prt_20e65cab6001GAVmkJHVvihrKV","sessionID":"ses_df19a36ceffefCFrtSV1Mdp91b","messageID":"msg_20e65caa40011oah3WcsbA38TC","type":"text","text":"hi","time":{"start":1795537881782,"end":1795537881783}}}
{"type":"step_finish","timestamp":1795537881784,"sessionID":"ses_df19a36ceffefCFrtSV1Mdp91b","part":{"id":"prt_20e65cab8002ZvNToC0lQZSAlj","sessionID":"ses_df19a36ceffefCFrtSV1Mdp91b","messageID":"msg_20e65caa40011oah3WcsbA38TC","type":"step-finish","reason":"stop","cost":0,"tokens":{"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}}}}
exit=0
```
```sh
$ grep -n 'loading plugin' /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/home/.local/share/opencode/log/opencode.log
```
```text
32:timestamp=2026-11-24T16:31:21.394Z level=INFO run=607f6b14 msg="loading plugin" id=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj/.opencode/plugins/plainjs.js entrypoint=file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj/.opencode/plugins/plainjs.js http.span=29 role=server
35:timestamp=2026-11-24T16:31:21.404Z level=INFO run=607f6b14 msg="loading plugin" id=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj/.opencode/plugins/plaints.ts entrypoint=file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-01/proj/.opencode/plugins/plaints.ts http.span=39 role=server
```

### P1b: `import { Plugin } from "@opencode/plugin"` with no `node_modules` / `package.json`

**Setup** (no `package.json`, no `node_modules` anywhere in the project)

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-09/proj/.opencode/plugins/withimport.ts
import * as SDK from "@opencode/plugin";
import { Plugin } from "@opencode/plugin";
import fs from "node:fs";

export default Plugin.define({
  id: "withimport",
  async setup(ctx) {
    fs.appendFileSync(
      "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-09/plug.log",
      JSON.stringify({ plugin: "withimport", sdkExports: Object.keys(SDK), ctxKeys: Object.keys(ctx).length }) + "\n",
    );
  },
});
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-09/scenario.json
[{"text":"hi"}]
```

**Command**

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-09 18753 "say hi"
```

**Output**

```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-09/plug.log; tail -1 /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-09/out.txt; cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-09/err.txt; find /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-09/proj -name node_modules -o -name package.json
```
```text
{"plugin":"withimport","sdkExports":["Agent","Artifact","Command","Connection","Credential","Integration","Location","Mcp","Model","PersistentPty","Plugin","Provider","Reference","Rpc","Skill","Vcs","WebSearch","Worktree"],"ctxKeys":26}
exit=0
```

### P1c: importing a name the host module does not export (`Tool`)

**Setup**

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-02/proj/.opencode/plugins/withimport.ts
import { Plugin, Tool } from "@opencode/plugin";
import fs from "node:fs";

export default Plugin.define({
  id: "withimport",
  async setup(ctx) {
    fs.appendFileSync(
      "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-02/plug.log",
      JSON.stringify({ plugin: "withimport", ctxKeys: Object.keys(ctx).length, toolExports: Object.keys(Tool ?? {}) }) + "\n",
    );
  },
});
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-02/scenario.json
[{"text":"hi"}]
```

**Command**

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-02 18751 "say hi"
```

**Output**

```sh
$ ls /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-02/plug.log; tail -1 /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-02/out.txt; cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-02/err.txt
```
```text
ls: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-02/plug.log: No such file or directory
exit=0
```
```sh
$ grep -n 'failed to load plugin' /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-02/home/.local/share/opencode/log/opencode.log
```
```text
35:timestamp=2026-11-24T16:31:55.151Z level=WARN run=917e4064 message="failed to load plugin" target=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-02/proj/.opencode/plugins/withimport.ts ref=err_ba31acca cause="Cause([Die(SyntaxError: Export named 'Tool' not found in module '@opencode/plugin'.)])" http.span=33 role=server
```

### P1d: module throws at load, default export without `id`, `setup` throws, plus one good plugin

**Setup**

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/proj/.opencode/plugins/a-throws.js
throw new Error("agentstacks-probe: module threw at load");
export default { id: "a-throws", setup: async () => {} };
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/proj/.opencode/plugins/b-noid.js
export default { setup: async () => {} };
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/proj/.opencode/plugins/c-setupthrows.js
export default {
  id: "c-setupthrows",
  setup: async () => {
    throw new Error("agentstacks-probe: setup threw");
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/proj/.opencode/plugins/d-ok.js
import fs from "node:fs";
export default {
  id: "d-ok",
  setup: async () => {
    fs.appendFileSync("/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/plug.log", "d-ok loaded\n");
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/scenario.json
[{"text":"hi"}]
```

**Command**

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03 18752 "say hi"
```

**Output**

```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/plug.log; tail -1 /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/out.txt; echo "stderr bytes: $(wc -c < /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/err.txt)"
```
```text
d-ok loaded
exit=0
stderr bytes:        0
```
```sh
$ grep -n 'failed to load plugin' /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/home/.local/share/opencode/log/opencode.log
```
```text
35:timestamp=2026-11-24T16:31:56.927Z level=WARN run=8c18c839 message="failed to load plugin" target=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/proj/.opencode/plugins/a-throws.js ref=err_5ce158cd cause="Cause([Die(Error: agentstacks-probe: module threw at load)])" http.span=31 role=server
39:timestamp=2026-11-24T16:31:56.930Z level=WARN run=8c18c839 message="failed to load plugin" target=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-03/proj/.opencode/plugins/b-noid.js ref=err_6dfcdfab cause="Cause([Fail(PluginModule.LoadError: Plugin must export a default definition with an id and an effect or setup function. (cause: SchemaError(Missing key\n  at [\"default\"][\"id\"]\nMissing key\n  at [\"default\"][\"id\"])))])" http.span=34 role=server
46:timestamp=2026-11-24T16:31:57.181Z level=WARN run=8c18c839 message="failed to load plugin" plugin.id=c-setupthrows cause="Cause([Die(Error: agentstacks-probe: setup threw)])" http.span=285 role=server
```

**Answer**

- A plain `.opencode/plugins/x.js` and `x.ts` with no imports and `export default { id, setup }` both load and
  `setup(ctx)` runs (P1a). The schema accepts `{ id: string, setup: function }` or `{ id, effect }`
  (`probes/src/packages/core/src/plugin/module.ts:81-94`). Discovery takes `.ts`/`.js` files or directories under
  `plugin/` and `plugins/` (`probes/src/packages/core/src/plugin/source-directory.ts:7,23-25`), and the docs say
  `.opencode/plugins/` loads automatically (`probes/docs/build-plugins/index.mdx:59`).
- `import { Plugin } from "@opencode/plugin"` also works with no `node_modules`: the Bun runtime serves
  `@opencode/plugin` and `effect` as host modules (`probes/src/packages/plugin/src/runtime.ts:12-27`,
  `probes/src/packages/plugin/src/runtime-modules.ts:7`). Its root exports no `Tool` (P1b list); importing `Tool` fails
  the load with `SyntaxError: Export named 'Tool' not found in module '@opencode/plugin'.` (P1c).
- A plugin that fails never reaches stdout/stderr or the exit code (exit=0, empty stderr); it is a `level=WARN ...
  message="failed to load plugin"` line in `$XDG_DATA_HOME/opencode/log/opencode.log`, and other plugins still load.
  Exact causes: load-time throw `Cause([Die(Error: <message>)])`; no `id` `PluginModule.LoadError: Plugin must export a
  default definition with an id and an effect or setup function. (cause: SchemaError(Missing key ... ["default"]["id"] ...))`;
  `setup` throws: `plugin.id=<id> cause="Cause([Die(Error: <message>)])"`.
- `Object.keys(ctx)` = `app, location, options, agent, aisdk, command, credential, event, experimental, generate, model,
  provider, integration, mcp, permission, plugin, reference, rpc, skill, storage, tool, vcs, websearch, worktree,
  session, shell`; `ctx.location` = `{ directory, project: { id, directory, canonical } }`; `ctx.options` = `{}`.

## P2: Can a plugin run a child process with stdin and capture stdout / exit code? Which runtime, cwd, env?

### P2a: spawn APIs in `setup` and inside a tool hook

**Setup**

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/proj/.opencode/plugins/spawn.js
import fs from "node:fs";
import { spawnSync, execFileSync, spawn } from "node:child_process";

const LOG = "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/plug.log";
const log = (o) => fs.appendFileSync(LOG, JSON.stringify(o) + "\n");
const tryit = (label, f) => {
  try { log({ label, ...f() }); } catch (e) { log({ label, threw: String(e) }); }
};

export default {
  id: "spawn",
  setup: async (ctx) => {
    const dir = ctx.location.directory;
    log({ label: "ctx", directory: dir, project: ctx.location.project, processCwd: process.cwd(),
      env: { HOME: process.env.HOME, TMPDIR: process.env.TMPDIR, PATH: process.env.PATH } });

    tryit("node spawnSync airsl --version", () => {
      const r = spawnSync("airsl", ["--version"], { encoding: "utf8" });
      return { status: r.status, stdout: r.stdout, stderr: r.stderr, error: r.error && String(r.error) };
    });
    tryit("node spawnSync sh -c cat with stdin", () => {
      const r = spawnSync("/bin/sh", ["-c", "cat"], { input: '{"event":"x"}', encoding: "utf8" });
      return { status: r.status, stdout: r.stdout };
    });
    tryit("node spawnSync exit 3", () => {
      const r = spawnSync("/bin/sh", ["-c", "echo out; echo err >&2; exit 3"], { encoding: "utf8" });
      return { status: r.status, stdout: r.stdout, stderr: r.stderr };
    });
    tryit("node execFileSync sh -c cat with stdin", () => {
      const out = execFileSync("/bin/sh", ["-c", "cat"], { input: "hello-execFileSync", encoding: "utf8" });
      return { stdout: out };
    });
    tryit("node spawnSync default cwd + env", () => {
      const r = spawnSync("/bin/sh", ["-c", 'pwd; echo "HOME=$HOME"; echo "TMPDIR=$TMPDIR"; echo "PATH=$PATH"'], { encoding: "utf8" });
      return { status: r.status, stdout: r.stdout };
    });
    tryit("node spawnSync cwd=ctx.location.directory", () => {
      const r = spawnSync("/bin/sh", ["-c", "pwd"], { cwd: dir, encoding: "utf8" });
      return { status: r.status, stdout: r.stdout };
    });
    tryit("Bun.spawnSync sh -c cat with stdin", () => {
      if (typeof Bun === "undefined") return { bun: "undefined" };
      const r = Bun.spawnSync(["/bin/sh", "-c", "cat; pwd"], { stdin: new TextEncoder().encode('{"bun":"sync"}') });
      return { exitCode: r.exitCode, stdout: r.stdout.toString() };
    });
    tryit("Bun.spawnSync airsl --version", () => {
      const r = Bun.spawnSync(["airsl", "--version"]);
      return { exitCode: r.exitCode, stdout: r.stdout.toString(), stderr: r.stderr.toString() };
    });

    await ctx.tool.hook("execute.before", async (event) => {
      // async spawn inside a hook: node child_process.spawn and Bun.spawn
      const nodeResult = await new Promise((resolve) => {
        const c = spawn("/bin/sh", ["-c", "cat; pwd"], { cwd: dir });
        let out = "";
        c.stdout.on("data", (d) => (out += d));
        c.on("close", (code) => resolve({ code, out }));
        c.stdin.end(JSON.stringify({ tool: event.tool }));
      });
      log({ label: "hook node spawn async", ...nodeResult });
      const p = Bun.spawn(["/bin/sh", "-c", "cat; exit 5"], { stdin: "pipe", stdout: "pipe", cwd: dir });
      p.stdin.write(JSON.stringify({ tool: event.tool, via: "Bun.spawn" }));
      p.stdin.end();
      const out = await new Response(p.stdout).text();
      const code = await p.exited;
      log({ label: "hook Bun.spawn async", code, out });
    });
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/scenario.json
[{"tool":"read","args":{"path":"a.txt"}},{"text":"ok"}]
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/proj/a.txt
alpha
```

**Command**

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04 18754 "read a.txt"
```

**Output**

```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/plug.log
```
```text
{"label":"ctx","directory":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/proj","project":{"id":"60e6de632d51af3b1a80d35daf31daa65b3d1b9d","directory":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/proj","canonical":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/proj"},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/proj","env":{"HOME":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/home","TMPDIR":"/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/","PATH":"/opt/homebrew/bin:/Users/hiraq/.antigravity/antigravity/bin:/Users/hiraq/.bun/bin:/Users/hiraq/.wasmer/bin:/Users/hiraq/.nvm/versions/node/v25.1.0/bin:/opt/homebrew/opt/libpq/bin:/opt/homebrew/opt/postgresql@15/bin:/Users/hiraq/.sdkman/candidates/kotlin/current/bin:/Users/hiraq/.sdkman/candidates/java/current/bin:/Users/hiraq/.sdkman/candidates/gradle/current/bin:/Users/hiraq/bin:/usr/local/bin:/Users/hiraq/.nix-profile/bin:/nix/var/nix/profiles/default/bin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/opt/pmk/env/global/bin:/usr/local/go/bin:/Users/hiraq/.local/bin:/Users/hiraq/.cargo/bin:/Users/hiraq/.orbstack/bin:/opt/homebrew/bin:/Users/hiraq/.orbstack/bin:/opt/homebrew/bin:/Users/hiraq/.nix-profile/bin:/nix/var/nix/profiles/default/bin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/opt/pmk/env/global/bin:/usr/local/go/bin:/Users/hiraq/.local/bin:/Users/hiraq/.cargo/bin:/Users/hiraq/.orbstack/bin:/opt/homebrew/bin:/Users/hiraq/.orbstack/bin:/usr/local/go/bin:/Users/hiraq/go/bin:/Users/hiraq/.cargo/bin"}}
{"label":"node spawnSync airsl --version","status":0,"stdout":"airsl 0.1.5\n","stderr":""}
{"label":"node spawnSync sh -c cat with stdin","status":0,"stdout":"{\"event\":\"x\"}"}
{"label":"node spawnSync exit 3","status":3,"stdout":"out\n","stderr":"err\n"}
{"label":"node execFileSync sh -c cat with stdin","stdout":"hello-execFileSync"}
{"label":"node spawnSync default cwd + env","status":0,"stdout":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/proj\nHOME=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/home\nTMPDIR=/var/folders/8w/4ccj7cw90nx4d32s72tqf5nw0000gn/T/\nPATH=/opt/homebrew/bin:/Users/hiraq/.antigravity/antigravity/bin:/Users/hiraq/.bun/bin:/Users/hiraq/.wasmer/bin:/Users/hiraq/.nvm/versions/node/v25.1.0/bin:/opt/homebrew/opt/libpq/bin:/opt/homebrew/opt/postgresql@15/bin:/Users/hiraq/.sdkman/candidates/kotlin/current/bin:/Users/hiraq/.sdkman/candidates/java/current/bin:/Users/hiraq/.sdkman/candidates/gradle/current/bin:/Users/hiraq/bin:/usr/local/bin:/Users/hiraq/.nix-profile/bin:/nix/var/nix/profiles/default/bin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/opt/pmk/env/global/bin:/usr/local/go/bin:/Users/hiraq/.local/bin:/Users/hiraq/.cargo/bin:/Users/hiraq/.orbstack/bin:/opt/homebrew/bin:/Users/hiraq/.orbstack/bin:/opt/homebrew/bin:/Users/hiraq/.nix-profile/bin:/nix/var/nix/profiles/default/bin:/usr/local/bin:/System/Cryptexes/App/usr/bin:/usr/bin:/bin:/usr/sbin:/sbin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/local/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/bin:/var/run/com.apple.security.cryptexd/codex.system/bootstrap/usr/appleinternal/bin:/opt/pmk/env/global/bin:/usr/local/go/bin:/Users/hiraq/.local/bin:/Users/hiraq/.cargo/bin:/Users/hiraq/.orbstack/bin:/opt/homebrew/bin:/Users/hiraq/.orbstack/bin:/usr/local/go/bin:/Users/hiraq/go/bin:/Users/hiraq/.cargo/bin\n"}
{"label":"node spawnSync cwd=ctx.location.directory","status":0,"stdout":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/proj\n"}
{"label":"Bun.spawnSync sh -c cat with stdin","exitCode":0,"stdout":"{\"bun\":\"sync\"}/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/proj\n"}
{"label":"Bun.spawnSync airsl --version","exitCode":0,"stdout":"airsl 0.1.5\n","stderr":""}
{"label":"hook node spawn async","code":0,"out":"{\"tool\":\"read\"}/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/proj\n"}
{"label":"hook Bun.spawn async","code":5,"out":"{\"tool\":\"read\",\"via\":\"Bun.spawn\"}"}
```
```sh
$ tail -1 /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/out.txt; echo "stderr bytes: $(wc -c < /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-04/err.txt)"
```
```text
exit=0
stderr bytes:        0
```

### P2b: project root differs from the launch directory (git repo one level above `proj/`)

**Setup** (`git init -q /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10`, so `proj/` is a subdirectory of a git repo with no commits)

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/proj/.opencode/plugins/where.js
import fs from "node:fs";
import { spawnSync } from "node:child_process";

export default {
  id: "where",
  setup: async (ctx) => {
    const r = spawnSync("/bin/sh", ["-c", "pwd"], { encoding: "utf8" });
    fs.appendFileSync(
      "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/plug.log",
      JSON.stringify({ location: ctx.location, processCwd: process.cwd(), childDefaultCwd: r.stdout }) + "\n",
    );
    await ctx.tool.hook("execute.before", (event) => {
      fs.appendFileSync(
        "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/plug.log",
        JSON.stringify({ hook: event.tool, input: event.input, processCwd: process.cwd() }) + "\n",
      );
    });
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/scenario.json
[{"tool":"read","args":{"path":"a.txt"}},{"text":"ok"}]
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/proj/a.txt
alpha
```

**Command**

```sh
git init -q /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10
```
```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10 18760 "read a.txt"
```

**Output**

```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/plug.log; tail -1 /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/out.txt
```
```text
{"location":{"directory":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/proj","project":{"id":"global","directory":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10","canonical":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10"}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/proj","childDefaultCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/proj\n"}
{"hook":"read","input":{"path":"a.txt"},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-10/proj"}
exit=0
```

**Answer**

- Project plugins run in Bun 1.4.4 (`typeof Bun` defined; `process.versions.node` reports 26.3.0). `node:child_process`
  `spawnSync`, `execFileSync` and `spawn`, and `Bun.spawnSync` / `Bun.spawn`, all run a child, take stdin, and return
  stdout, stderr and the exit code (3 and 5 observed), both in `setup` and inside an `execute.before` hook.
  `airsl --version` resolves through PATH and prints `airsl 0.1.5`.
- With no `cwd` option the child gets the OpenCode process cwd, which is the directory `opencode run` was launched
  in; `ctx.location.directory` is that same directory, while `ctx.location.project.directory` is the git root (P2b:
  `proj/` vs its parent). Passing `cwd: ctx.location.directory` works. HOME, TMPDIR and PATH are inherited from the
  OpenCode process unchanged (HOME was the harness-isolated one).

## P3: Tool before hook: registration API and the full input object for read, edit, write, main vs subagent

**Setup**

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj/.opencode/plugins/logev.js
import fs from "node:fs";

const LOG = "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/plug.log";
const dump = (hook, event) => {
  const { info, ...rest } = event;
  fs.appendFileSync(LOG, JSON.stringify({ hook, keys: Object.keys(event), infoKeys: info ? Object.keys(info) : null, infoName: info && info.name, event: rest, processCwd: process.cwd() }) + "\n");
};

export default {
  id: "logev",
  setup: async (ctx) => {
    await ctx.tool.hook("execute.before", (event) => dump("execute.before", event));
    await ctx.tool.hook("execute.after", (event) => dump("execute.after", event));
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj/.opencode/agents/sub.md
---
description: Probe subagent that reads a file.
mode: subagent
---

Read the file you are asked to read.
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/scenario.json
[
  {"tool":"read","args":{"path":"a.txt"}},
  {"tool":"edit","args":{"path":"a.txt","oldString":"alpha","newString":"beta"}},
  {"tool":"write","args":{"path":"b.txt","content":"written\n"}},
  {"tool":"subagent","args":{"agent":"sub","description":"read b","prompt":"read b.txt"}},
  {"tool":"read","args":{"path":"b.txt"}},
  {"tool":"read","args":{"path":"b.txt"}},
  {"text":"one"},
  {"text":"two"}
]
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj/a.txt (before the run)
alpha
```

**Command**

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05 18755 "do the steps"
```

**Output**

```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/plug.log
```
```text
{"hook":"execute.before","keys":["tool","info","codemode","sessionID","agent","messageID","id","input"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"read","event":{"tool":"read","codemode":false,"sessionID":"ses_df197923bffeXA1hmqN0hTxaBc","agent":"build","messageID":"msg_20e686f32001syBTEZYlAC9ZeX","id":"call_1","input":{"path":"a.txt"}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.after","keys":["tool","info","codemode","sessionID","agent","messageID","id","input","status","result"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"read","event":{"tool":"read","codemode":false,"sessionID":"ses_df197923bffeXA1hmqN0hTxaBc","agent":"build","messageID":"msg_20e686f32001syBTEZYlAC9ZeX","id":"call_1","input":{"path":"a.txt"},"status":"completed","result":{"output":{"type":"text","offset":1,"lineCount":1,"content":"alpha\n"},"content":[{"type":"text","text":"Read file a.txt, lines 1-1\n1: alpha"}],"metadata":{"truncated":false}}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.before","keys":["tool","info","codemode","sessionID","agent","messageID","id","input"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"edit","event":{"tool":"edit","codemode":false,"sessionID":"ses_df197923bffeXA1hmqN0hTxaBc","agent":"build","messageID":"msg_20e686f4d001BzBT1slP8JZnUA","id":"call_2","input":{"path":"a.txt","oldString":"alpha","newString":"beta"}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.after","keys":["tool","info","codemode","sessionID","agent","messageID","id","input","status","result"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"edit","event":{"tool":"edit","codemode":false,"sessionID":"ses_df197923bffeXA1hmqN0hTxaBc","agent":"build","messageID":"msg_20e686f4d001BzBT1slP8JZnUA","id":"call_2","input":{"path":"a.txt","oldString":"alpha","newString":"beta"},"status":"completed","result":{"output":"Edited a.txt (1 replacement)","content":[{"type":"text","text":"Edited a.txt (1 replacement)"}],"metadata":{"files":[{"file":"a.txt","patch":"Index: a.txt\n===================================================================\n--- a.txt\n+++ a.txt\n@@ -1,1 +1,1 @@\n-alpha\n+beta\n","status":"modified","additions":1,"deletions":1}]}}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.before","keys":["tool","info","codemode","sessionID","agent","messageID","id","input"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"write","event":{"tool":"write","codemode":false,"sessionID":"ses_df197923bffeXA1hmqN0hTxaBc","agent":"build","messageID":"msg_20e686f5d001kiVofprFrHGj5B","id":"call_3","input":{"path":"b.txt","content":"written\n"}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.after","keys":["tool","info","codemode","sessionID","agent","messageID","id","input","status","result"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"write","event":{"tool":"write","codemode":false,"sessionID":"ses_df197923bffeXA1hmqN0hTxaBc","agent":"build","messageID":"msg_20e686f5d001kiVofprFrHGj5B","id":"call_3","input":{"path":"b.txt","content":"written\n"},"status":"completed","result":{"output":"Created file successfully: b.txt","content":[{"type":"text","text":"Created file successfully: b.txt"}]}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.before","keys":["tool","info","codemode","sessionID","agent","messageID","id","input"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"subagent","event":{"tool":"subagent","codemode":false,"sessionID":"ses_df197923bffeXA1hmqN0hTxaBc","agent":"build","messageID":"msg_20e686f6a001KcuonqZF1RBbHB","id":"call_4","input":{"agent":"sub","description":"read b","prompt":"read b.txt"}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.after","keys":["tool","info","codemode","sessionID","agent","messageID","id","input","status","result"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"subagent","event":{"tool":"subagent","codemode":false,"sessionID":"ses_df197923bffeXA1hmqN0hTxaBc","agent":"build","messageID":"msg_20e686f6a001KcuonqZF1RBbHB","id":"call_4","input":{"agent":"sub","description":"read b","prompt":"read b.txt"},"status":"completed","result":{"output":{"sessionID":"ses_df197908effeTQjaxaGtFhErIx","status":"running","output":"The subagent is working in the background (sessionID: ses_df197908effeTQjaxaGtFhErIx). You will receive a <background-result kind=\"subagent\"> message when it goes idle, and again after each later round of work.\nDO NOT sleep, poll, proactively check on its progress, or duplicate this subagent's work.\nWork on non-overlapping tasks, or briefly tell the user what you launched and end your response.\nTo send it follow-ups or change its instructions while it runs, call subagent again with this sessionID."},"content":[{"type":"text","text":"The subagent is working in the background (sessionID: ses_df197908effeTQjaxaGtFhErIx). You will receive a <background-result kind=\"subagent\"> message when it goes idle, and again after each later round of work.\nDO NOT sleep, poll, proactively check on its progress, or duplicate this subagent's work.\nWork on non-overlapping tasks, or briefly tell the user what you launched and end your response.\nTo send it follow-ups or change its instructions while it runs, call subagent again with this sessionID."}],"metadata":{"sessionID":"ses_df197908effeTQjaxaGtFhErIx","status":"running"}}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.before","keys":["tool","info","codemode","sessionID","agent","messageID","id","input"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"read","event":{"tool":"read","codemode":false,"sessionID":"ses_df197923bffeXA1hmqN0hTxaBc","agent":"build","messageID":"msg_20e686f78001fh7qvSAkffsTqv","id":"call_5","input":{"path":"b.txt"}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.before","keys":["tool","info","codemode","sessionID","agent","messageID","id","input"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"read","event":{"tool":"read","codemode":false,"sessionID":"ses_df197908effeTQjaxaGtFhErIx","agent":"sub","messageID":"msg_20e686f7e001C2whzskx4RQg3s","id":"call_6","input":{"path":"b.txt"}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.after","keys":["tool","info","codemode","sessionID","agent","messageID","id","input","status","result"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"read","event":{"tool":"read","codemode":false,"sessionID":"ses_df197923bffeXA1hmqN0hTxaBc","agent":"build","messageID":"msg_20e686f78001fh7qvSAkffsTqv","id":"call_5","input":{"path":"b.txt"},"status":"completed","result":{"output":{"type":"text","offset":1,"lineCount":1,"content":"written\n"},"content":[{"type":"text","text":"Read file b.txt, lines 1-1\n1: written"}],"metadata":{"truncated":false}}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
{"hook":"execute.after","keys":["tool","info","codemode","sessionID","agent","messageID","id","input","status","result"],"infoKeys":["name","options","description","input","output","execute","id"],"infoName":"read","event":{"tool":"read","codemode":false,"sessionID":"ses_df197908effeTQjaxaGtFhErIx","agent":"sub","messageID":"msg_20e686f7e001C2whzskx4RQg3s","id":"call_6","input":{"path":"b.txt"},"status":"completed","result":{"output":{"type":"text","offset":1,"lineCount":1,"content":"written\n"},"content":[{"type":"text","text":"Read file b.txt, lines 1-1\n1: written"}],"metadata":{"truncated":false}}},"processCwd":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj"}
```
```sh
$ tail -1 /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/out.txt; echo "stderr bytes: $(wc -c < /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/err.txt)"; cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj/a.txt /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/proj/b.txt
```
```text
exit=0
stderr bytes:        0
beta
written
```

**Answer**

- Registration: `await ctx.tool.hook("execute.before", (event) => ...)` inside `setup`
  (`probes/docs/build-plugins/server.mdx:1807-1840`, type at `probes/src/packages/plugin/src/promise/tool.ts:51-61`).
- The event has `tool` (name, e.g. `read`), `info` (the tool definition: `name, options, description, input, output,
  execute, id`), `codemode` (false), `sessionID`, `agent`, `messageID`, `id` (the model's tool call id, `call_N`) and
  `input` (the args as the model sent them; the path key is `path` for read, edit and write). There is no cwd field;
  `process.cwd()` in the hook is the launch directory.
- Main agent calls carry `agent: "build"`; the call made inside the subagent carries `agent: "sub"` and the child
  session's `sessionID` (`ses_df197908…`), with no field naming the parent session. The `subagent` call itself
  returns at once ("working in the background"), so main and child calls interleave.

## P4: Blocking from the before hook: what the model sees, does the tool run, can the result carry our reason?

**Setup**

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06/proj/.opencode/plugins/block.js
import fs from "node:fs";

const LOG = "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06/plug.log";
const log = (o) => fs.appendFileSync(LOG, JSON.stringify(o) + "\n");

export default {
  id: "block",
  setup: async (ctx) => {
    await ctx.tool.hook("execute.before", (event) => {
      log({ hook: "before", tool: event.tool, input: event.input });
      if (event.tool === "write") {
        throw new Error("agentstacks: blocked by hook guard-write (reason text from Lua)");
      }
      if (event.tool === "edit") {
        // a non-Error value thrown
        throw "agentstacks: plain string thrown for edit";
      }
    });
    await ctx.tool.hook("execute.after", (event) => {
      log({ hook: "after", tool: event.tool, status: event.status, error: event.status === "error" ? String(event.error && event.error.message) : undefined });
    });
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06/scenario.json
[
  {"tool":"write","args":{"path":"blocked.txt","content":"should not exist\n"}},
  {"tool":"edit","args":{"path":"a.txt","oldString":"alpha","newString":"beta"}},
  {"tool":"read","args":{"path":"a.txt"}},
  {"text":"done-06"}
]
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06/proj/a.txt (before the run)
alpha
```

**Command**

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06 18756 "do the steps"
```

**Output**

```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06/plug.log; tail -1 /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06/out.txt; ls /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06/proj; cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06/proj/a.txt
```
```text
{"hook":"before","tool":"write","input":{"path":"blocked.txt","content":"should not exist\n"}}
{"hook":"before","tool":"edit","input":{"path":"a.txt","oldString":"alpha","newString":"beta"}}
{"hook":"before","tool":"read","input":{"path":"a.txt"}}
{"hook":"after","tool":"read","status":"completed"}
exit=0
a.txt
alpha
```
```sh
$ node /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-scripts/toolresults.js /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06/requests.jsonl
```
```text
{"role":"assistant","tool_calls":[{"id":"call_1","name":"write","arguments":"{\"path\":\"blocked.txt\",\"content\":\"should not exist\\n\"}"}]}
{"role":"tool","tool_call_id":"call_1","content":"{\"error\":{\"type\":\"unknown\",\"message\":\"agentstacks: blocked by hook guard-write (reason text from Lua)\"},\"content\":[]}"}
{"role":"assistant","tool_calls":[{"id":"call_2","name":"edit","arguments":"{\"path\":\"a.txt\",\"oldString\":\"alpha\",\"newString\":\"beta\"}"}]}
{"role":"tool","tool_call_id":"call_2","content":"{\"error\":{\"type\":\"unknown\",\"message\":\"agentstacks: plain string thrown for edit\"},\"content\":[]}"}
{"role":"assistant","tool_calls":[{"id":"call_3","name":"read","arguments":"{\"path\":\"a.txt\"}"}]}
{"role":"tool","tool_call_id":"call_3","content":"Read file a.txt, lines 1-1\n1: alpha"}
```
```sh
$ grep '"type":"tool_use"' /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-06/out.txt | node -e 'const l=require("fs").readFileSync(0,"utf8").trim().split("\n");for(const x of l){const p=JSON.parse(x).part;console.log(JSON.stringify({tool:p.tool,id:p.id,state:p.state}))}'
```
```text
{"tool":"write","id":"call_1","state":{"status":"error","input":{"path":"blocked.txt","content":"should not exist\n"},"error":"agentstacks: blocked by hook guard-write (reason text from Lua)","metadata":{"providerCall":{"executed":false},"providerResult":{"executed":false},"rawInput":"{\"path\":\"blocked.txt\",\"content\":\"should not exist\\n\"}"},"time":{"start":1795538056867,"end":1795538056872}}}
{"tool":"edit","id":"call_2","state":{"status":"error","input":{"path":"a.txt","oldString":"alpha","newString":"beta"},"error":"agentstacks: plain string thrown for edit","metadata":{"providerCall":{"executed":false},"providerResult":{"executed":false},"rawInput":"{\"path\":\"a.txt\",\"oldString\":\"alpha\",\"newString\":\"beta\"}"},"time":{"start":1795538056891,"end":1795538056894}}}
{"tool":"read","id":"call_3","state":{"status":"completed","input":{"path":"a.txt"},"output":"Read file a.txt, lines 1-1\n1: alpha","title":"read","metadata":{"metadata":{"truncated":false},"content":[{"type":"text","text":"Read file a.txt, lines 1-1\n1: alpha"}],"providerCall":{"executed":false},"providerResult":{"executed":false},"rawInput":"{\"path\":\"a.txt\"}"},"time":{"start":1795538056905,"end":1795538056909}}}
```

**Answer**

- Throwing from `execute.before` blocks the call: `write` left no `blocked.txt`, `edit` left `a.txt` as `alpha`, and
  `execute.after` did not fire for either. No import is needed: the Promise adapter wraps any thrown value into a
  `Tool.Error` with the thrown message (`probes/src/packages/plugin/src/promise/adapter.ts:265-282,731-734`), and core
  stops before running the tool (`probes/src/packages/core/src/tool.ts:161-171,227-237`).
- The model sees the tool result `{"error":{"type":"unknown","message":"<our text>"},"content":[]}`, so the reason
  text reaches it verbatim, for an `Error` and for a thrown string alike. The run goes on (exit=0).

## P5: Tool after hook: objects, changing model-visible output, failing after the fact

### P5a: change the result

**Setup**

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-07/proj/.opencode/plugins/after.js
import fs from "node:fs";

const LOG = "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-07/plug.log";
const log = (o) => fs.appendFileSync(LOG, JSON.stringify(o) + "\n");

export default {
  id: "after",
  setup: async (ctx) => {
    await ctx.tool.hook("execute.after", (event) => {
      log({ hook: "after", tool: event.tool, status: event.status, result: event.result, error: event.error && event.error.message });
      if (event.status !== "completed") return;
      if (event.tool === "read") {
        // append a text content item
        event.result = {
          ...event.result,
          content: [...event.result.content, { type: "text", text: "[agentstacks] added context from after hook" }],
        };
      }
      if (event.tool === "write") {
        // replace the output only
        event.result = { ...event.result, output: "[agentstacks] replaced write output" };
      }
      if (event.tool === "edit") {
        // try to turn the call into a failure after the fact
        event.status = "error";
        event.error = new Error("[agentstacks] after hook tried to fail edit");
      }
    });
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-07/scenario.json
[
  {"tool":"read","args":{"path":"a.txt"}},
  {"tool":"write","args":{"path":"w.txt","content":"w\n"}},
  {"tool":"edit","args":{"path":"a.txt","oldString":"alpha","newString":"beta"}},
  {"text":"done-07"}
]
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-07/proj/a.txt (before the run)
alpha
```

**Command**

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-07 18757 "do the steps"
```

**Output**

```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-07/plug.log; tail -1 /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-07/out.txt; ls /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-07/proj; cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-07/proj/a.txt
```
```text
{"hook":"after","tool":"read","status":"completed","result":{"output":{"type":"text","offset":1,"lineCount":1,"content":"alpha\n"},"content":[{"type":"text","text":"Read file a.txt, lines 1-1\n1: alpha"}],"metadata":{"truncated":false}}}
{"hook":"after","tool":"write","status":"completed","result":{"output":"Created file successfully: w.txt","content":[{"type":"text","text":"Created file successfully: w.txt"}]}}
{"hook":"after","tool":"edit","status":"completed","result":{"output":"Edited a.txt (1 replacement)","content":[{"type":"text","text":"Edited a.txt (1 replacement)"}],"metadata":{"files":[{"file":"a.txt","patch":"Index: a.txt\n===================================================================\n--- a.txt\n+++ a.txt\n@@ -1,1 +1,1 @@\n-alpha\n+beta\n","status":"modified","additions":1,"deletions":1}]}}}
exit=0
a.txt
w.txt
beta
```
```sh
$ node /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-scripts/toolresults.js /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-07/requests.jsonl
```
```text
{"role":"assistant","tool_calls":[{"id":"call_1","name":"read","arguments":"{\"path\":\"a.txt\"}"}]}
{"role":"tool","tool_call_id":"call_1","content":"Read file a.txt, lines 1-1\n1: alpha\n[agentstacks] added context from after hook"}
{"role":"assistant","tool_calls":[{"id":"call_2","name":"write","arguments":"{\"path\":\"w.txt\",\"content\":\"w\\n\"}"}]}
{"role":"tool","tool_call_id":"call_2","content":"Created file successfully: w.txt"}
{"role":"assistant","tool_calls":[{"id":"call_3","name":"edit","arguments":"{\"path\":\"a.txt\",\"oldString\":\"alpha\",\"newString\":\"beta\"}"}]}
{"role":"tool","tool_call_id":"call_3","content":"Edited a.txt (1 replacement)"}
```

### P5b: throw from the after hook (`write` in probe plugtool-08, full setup in P7)

See P7 output: `write` ran (`w.txt` exists) and the model saw an error result.

**Answer**

- The after event has the before fields plus `status` and `result` (`{ output, content: [{type:"text",text}], metadata }`)
  when completed, or `error` when failed (P3 output; type at `probes/src/packages/plugin/src/promise/tool.ts:62-81`).
- Replacing `event.result` with extra `content` items changes what the model sees: the read result became
  `Read file a.txt, lines 1-1\n1: alpha\n[agentstacks] added context from after hook`. Changing only `result.output`
  did not change the model text (`Created file successfully: w.txt`). Setting `status`/`error` did nothing: the adapter
  copies back only `result` (completed) or `error` (failed) (`probes/src/packages/plugin/src/promise/adapter.ts:284-294`).
- The typed API gives `execute.after` no failure channel (`probes/src/packages/plugin/src/effect/tool.ts:57-61`), but a
  throw does make the model see `{"error":{"type":"unknown","message":"<text>"},"content":[]}` after the tool has
  already run (P7), so the side effect stays.

## P6: Which tools can change a file, per model, and their path argument names

**Setup** (the harness global config is unchanged; this project config adds a `gpt-5` model to the mock provider)

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/proj/opencode.json
{
  "$schema": "https://opencode.ai/config.json",
  "model": "mock/gpt-5",
  "providers": { "mock": { "models": { "gpt-5": { "name": "G5" } } } }
}
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/proj/.opencode/plugins/logpatch.js
import fs from "node:fs";

const LOG = "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/plug.log";
const log = (o) => fs.appendFileSync(LOG, JSON.stringify(o) + "\n");

export default {
  id: "logpatch",
  setup: async (ctx) => {
    await ctx.tool.hook("execute.before", (event) => {
      log({ hook: "before", tool: event.tool, agent: event.agent, codemode: event.codemode, input: event.input });
    });
    await ctx.tool.hook("execute.after", (event) => {
      log({ hook: "after", tool: event.tool, status: event.status, content: event.status === "completed" ? event.result.content : String(event.error && event.error.message) });
    });
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/scenario.json
[
  {"tool":"patch","args":{"patchText":"*** Begin Patch\n*** Add File: p.txt\n+hello\n*** Update File: a.txt\n@@\n-alpha\n+gamma\n*** End Patch"}},
  {"tool":"shell","args":{"command":"echo shell > s.txt"}},
  {"text":"done-11"}
]
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/proj/a.txt (before the run)
alpha
```

**Command**

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11 18761 "do the steps"
```

**Output**

```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/plug.log; tail -1 /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/out.txt; ls /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/proj; cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/proj/a.txt
```
```text
{"hook":"before","tool":"patch","agent":"build","codemode":false,"input":{"patchText":"*** Begin Patch\n*** Add File: p.txt\n+hello\n*** Update File: a.txt\n@@\n-alpha\n+gamma\n*** End Patch"}}
{"hook":"after","tool":"patch","status":"completed","content":[{"type":"text","text":"Success. Updated the following files:\nA p.txt\nM a.txt"}]}
{"hook":"before","tool":"shell","agent":"build","codemode":false,"input":{"command":"echo shell > s.txt"}}
{"hook":"after","tool":"shell","status":"completed","content":[{"type":"text","text":"(no output)"}]}
exit=0
a.txt
opencode.json
p.txt
s.txt
gamma
```
```sh
$ node -e 'const fs=require("fs");const l=fs.readFileSync("/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/requests.jsonl","utf8").trim().split("\n").map(JSON.parse);const r=l.filter(x=>x.body.tools&&x.body.tools.length);console.log("model:",r[0].body.model);console.log("tools:",r[0].body.tools.map(t=>t.function.name).join(","));for(const n of ["edit","write","patch","shell","generate_image","generate_video"]){const t=r[0].body.tools.find(t=>t.function.name===n);console.log(n, t?JSON.stringify(Object.keys(t.function.parameters.properties)):"(not offered)")}'
```
```text
model: gpt-5
tools: generate_image,generate_video,glob,grep,patch,question,read,shell,skill,subagent,webfetch,websearch,execute
edit (not offered)
write (not offered)
patch ["patchText"]
shell ["command","workdir","timeout","background"]
generate_image ["prompt","images","model","path"]
generate_video ["prompt","start","end","references","duration","aspect","resolution","sound","model","path"]
```
Control, the default mock model `m` (probe plugtool-05):

```sh
$ node -e 'const fs=require("fs");const l=fs.readFileSync("/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-05/requests.jsonl","utf8").trim().split("\n").map(JSON.parse);const r=l.filter(x=>x.body.tools&&x.body.tools.length);console.log("model:",r[0].body.model);console.log("tools:",r[0].body.tools.map(t=>t.function.name).join(","));for(const n of ["edit","write","patch","shell","generate_image","generate_video"]){const t=r[0].body.tools.find(t=>t.function.name===n);console.log(n, t?JSON.stringify(Object.keys(t.function.parameters.properties)):"(not offered)")}'
```
```text
model: m
tools: edit,generate_image,generate_video,glob,grep,question,read,shell,skill,subagent,webfetch,websearch,write,execute
edit ["path","oldString","newString","replaceAll"]
write ["path","content"]
patch (not offered)
shell ["command","workdir","timeout","background"]
generate_image ["prompt","images","model","path"]
generate_video ["prompt","start","end","references","duration","aspect","resolution","sound","model","path"]
```
```sh
$ node /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-scripts/toolresults.js /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-11/requests.jsonl
```
```text
{"role":"assistant","tool_calls":[{"id":"call_1","name":"patch","arguments":"{\"patchText\":\"*** Begin Patch\\n*** Add File: p.txt\\n+hello\\n*** Update File: a.txt\\n@@\\n-alpha\\n+gamma\\n*** End Patch\"}"}]}
{"role":"tool","tool_call_id":"call_1","content":"Success. Updated the following files:\nA p.txt\nM a.txt"}
{"role":"assistant","tool_calls":[{"id":"call_2","name":"shell","arguments":"{\"command\":\"echo shell > s.txt\"}"}]}
{"role":"tool","tool_call_id":"call_2","content":"(no output)"}
```
Search for `multiedit` / `apply_patch` in the v2.4.1 core source, with `"patch"` as the control:

```sh
$ cd /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/oc-src/anomalyco-opencode-6f05140/packages && grep -rn "multiedit\|apply_patch" core/src; echo control:; grep -rln '"patch"' core/src/tool
```
```text
core/src/v1/config/migrate.ts:30:  if (action === "write" || action === "patch" || action === "apply_patch") return "edit"
core/src/database/v1-migration.bun.ts:996:const RENAMED_TOOLS: Readonly<Record<string, string>> = { bash: "shell", task: "subagent", apply_patch: "patch" }
control:
core/src/tool/plugin/patch.ts
core/src/tool/model-policy.ts
```

**Answer**

- Model-dependent: a model whose id contains `gpt-` (not `gpt-4`, not `oss`) gets `patch` and loses `edit` and `write`;
  every other model gets `edit` and `write` and no `patch` (`probes/src/packages/core/src/tool/model-policy.ts:20-37`;
  proven with `mock/gpt-5` vs `mock/m`). V2 has no `multiedit` or `apply_patch`; `apply_patch` appears only as a V1
  rename to `patch` (`probes/src/packages/core/src/database/v1-migration.bun.ts:996`).
- Built-in tools that can change files and their path arguments:
  `edit` → `path` (`probes/src/packages/core/src/tool/plugin/edit.ts:21-24`); `write` → `path`
  (`probes/src/packages/core/src/tool/plugin/write.ts:20-21`); `patch` → `patchText` only, the paths sit inside it on
  `*** Add File: <path>`, `*** Update File: <path>`, `*** Delete File: <path>`, `*** Move to: <path>` lines
  (`probes/src/packages/core/src/tool/plugin/patch.ts:24-27,72`); `shell` → `command` (+ optional `workdir`)
  (`probes/src/packages/core/src/tool/plugin/shell.ts:64-65`); `generate_image` / `generate_video` → optional `path`,
  which overwrites an existing file (`probes/src/packages/core/src/tool/plugin/image-generate.ts:63-66`,
  `probes/src/packages/core/src/tool/plugin/video-generate.ts:80-83`). Hooks fire for `patch` and `shell` like any other
  tool (plugtool-11 log).

## P7: Does an unrelated error thrown in a hook crash the run, or is it logged and the tool continues?

**Setup**

```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/proj/.opencode/plugins/throws.js
import fs from "node:fs";

const LOG = "/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/plug.log";
const log = (o) => fs.appendFileSync(LOG, JSON.stringify(o) + "\n");

export default {
  id: "throws",
  setup: async (ctx) => {
    await ctx.tool.hook("execute.before", (event) => {
      log({ hook: "before", tool: event.tool });
      if (event.tool === "read") {
        // an unrelated bug in the hook (TypeError), not a deliberate block
        const missing = undefined;
        return missing.property;
      }
    });
    await ctx.tool.hook("execute.after", (event) => {
      log({ hook: "after", tool: event.tool, status: event.status });
      if (event.tool === "write") throw new Error("agentstacks-probe: after hook threw");
    });
  },
};
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/scenario.json
[
  {"tool":"read","args":{"path":"a.txt"}},
  {"tool":"write","args":{"path":"w.txt","content":"w\n"}},
  {"tool":"glob","args":{"pattern":"*.txt"}},
  {"text":"done-08"}
]
```
```/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/proj/a.txt (before the run)
alpha
```

**Command**

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08 18758 "do the steps"
```

**Output**

```sh
$ cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/plug.log; tail -1 /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/out.txt; echo "stderr bytes: $(wc -c < /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/err.txt)"; ls /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/proj; cat /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/proj/w.txt
```
```text
{"hook":"before","tool":"read"}
{"hook":"before","tool":"write"}
{"hook":"after","tool":"write","status":"completed"}
{"hook":"before","tool":"glob"}
{"hook":"after","tool":"glob","status":"completed"}
exit=0
stderr bytes:        0
a.txt
w.txt
w
```
```sh
$ node /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-scripts/toolresults.js /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/requests.jsonl
```
```text
{"role":"assistant","tool_calls":[{"id":"call_1","name":"read","arguments":"{\"path\":\"a.txt\"}"}]}
{"role":"tool","tool_call_id":"call_1","content":"{\"error\":{\"type\":\"unknown\",\"message\":\"undefined is not an object (evaluating '(void 0).property')\"},\"content\":[]}"}
{"role":"assistant","tool_calls":[{"id":"call_2","name":"write","arguments":"{\"path\":\"w.txt\",\"content\":\"w\\n\"}"}]}
{"role":"tool","tool_call_id":"call_2","content":"{\"error\":{\"type\":\"unknown\",\"message\":\"agentstacks-probe: after hook threw\"},\"content\":[]}"}
{"role":"assistant","tool_calls":[{"id":"call_3","name":"glob","arguments":"{\"pattern\":\"*.txt\"}"}]}
{"role":"tool","tool_call_id":"call_3","content":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/proj/a.txt\n/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/proj/w.txt"}
```
```sh
$ grep '"type":"tool_use"' /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/out.txt | node -e 'const l=require("fs").readFileSync(0,"utf8").trim().split("\n");for(const x of l){const p=JSON.parse(x).part;console.log(JSON.stringify({tool:p.tool,id:p.id,state:p.state}))}'
```
```text
{"tool":"read","id":"call_1","state":{"status":"error","input":{"path":"a.txt"},"error":"undefined is not an object (evaluating '(void 0).property')","metadata":{"providerCall":{"executed":false},"providerResult":{"executed":false},"rawInput":"{\"path\":\"a.txt\"}"},"time":{"start":1795538060520,"end":1795538060524}}}
{"tool":"write","id":"call_2","state":{"status":"error","input":{"path":"w.txt","content":"w\n"},"error":"agentstacks-probe: after hook threw","metadata":{"providerCall":{"executed":false},"providerResult":{"executed":false},"rawInput":"{\"path\":\"w.txt\",\"content\":\"w\\n\"}"},"time":{"start":1795538060542,"end":1795538060546}}}
{"tool":"glob","id":"call_3","state":{"status":"completed","input":{"pattern":"*.txt"},"output":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/proj/a.txt\n/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/proj/w.txt","title":"glob","metadata":{"metadata":{"count":2,"truncated":false},"content":[{"type":"text","text":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/proj/a.txt\n/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/proj/w.txt"}],"providerCall":{"executed":false},"providerResult":{"executed":false},"rawInput":"{\"pattern\":\"*.txt\"}"},"time":{"start":1795538060561,"end":1795538060615}}}
```
```sh
$ grep -c 'level=WARN\|level=ERROR' /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/plugtool-08/home/.local/share/opencode/log/opencode.log
```
```text
0
[exit 1]
```

**Answer**

- Nothing crashes: the run continues to the end (exit=0, empty stderr, no WARN/ERROR line in the log).
- An accidental `TypeError` in `execute.before` is treated exactly like a deliberate block: `read` did not run and the
  model saw `{"error":{"type":"unknown","message":"undefined is not an object (evaluating '(void 0).property')"},"content":[]}`.
- A throw in `execute.after` leaves the tool's effect in place (`w.txt` written) but the model sees the call as failed:
  `{"error":{"type":"unknown","message":"agentstacks-probe: after hook threw"},"content":[]}`. Later tools (`glob`) run normally.
- A throw in `setup` only disables that plugin (P1d WARN line).

