# Probe results: where a relative tool path resolves (OpenCode v2.4.1)

## R1: OpenCode started in `proj/sub` (no git repo); the model reads `x.txt`, which exists only in `proj/sub`

**Setup** (`<scratchpad>/rel-01/`; harness copy `<scratchpad>/relh/run.sh` = `probes/harness/run.sh` with
`cd "$DIR/proj"` changed to `cd "$DIR/proj/sub"`)

```proj/.opencode/plugins/loc.js
import fs from "node:fs";
export default { id: "loc", setup: async (ctx) => {
  fs.appendFileSync(new URL("../../../loc.log", import.meta.url), JSON.stringify({ dir: ctx.location.directory, proj: ctx.location.project.directory }) + "\n");
  await ctx.tool.hook("execute.before", (e) => {
    fs.appendFileSync(new URL("../../../loc.log", import.meta.url), JSON.stringify({ tool: e.tool, input: e.input }) + "\n");
  });
}};
```

```scenario.json
[{"tool":"read","args":{"path":"x.txt"}},{"text":"ok"}]
```

```proj/sub/x.txt
only-in-sub
```

**Command**

```sh
sh <scratchpad>/relh/run.sh <scratchpad>/rel-01 18830 "read"
cat rel-01/loc.log
node -e '…print the tool messages of the last request offering tools…' rel-01/requests.jsonl
tail -1 rel-01/out.txt
```

**Output**

```text
{"dir":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/rel-01/proj/sub","proj":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/rel-01/proj/sub"}
{"tool":"read","input":{"path":"x.txt"}}
[{"role":"tool","tool_call_id":"call_1","content":"Read file x.txt, lines 1-1\n1: only-in-sub"}]
exit=0
```

**Answer**

A relative `path` in a tool call resolves against the launch directory, `ctx.location.directory`: `x.txt` read
`proj/sub/x.txt`. The hook's `input.path` is the relative text as the model sent it. With no git repo,
`ctx.location.project.directory` equals the launch directory.
