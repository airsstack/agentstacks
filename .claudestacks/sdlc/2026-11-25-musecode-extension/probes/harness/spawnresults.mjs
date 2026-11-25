// spawnresults.mjs <logdir>: from the last lead request, print each subagent_spawn call's subagent_type and its output, raw.
import { readFileSync, readdirSync } from "node:fs";
const dir = process.argv[2];
const reqs = readdirSync(dir).filter((f) => /^req-\d+\.json$/.test(f)).sort();
let best = null;
for (const f of reqs) { const t = readFileSync(`${dir}/${f}`, "utf8"); if (!t || t.includes("BODY of") || t.includes("You are a goal reminder")) continue;
  const j = JSON.parse(t); const n = (j.input || []).filter((i) => i.type === "function_call").length; if (!best || n >= best.n) best = { f, j, n }; }
console.log(`# lead request ${best.f}`);
const outs = Object.fromEntries(best.j.input.filter((i) => i.type === "function_call_output").map((i) => [i.call_id, i.output]));
for (const i of best.j.input.filter((i) => i.type === "function_call")) {
  const a = JSON.parse(i.arguments); console.log(`${a.subagent_type}\t${typeof outs[i.call_id] === "string" ? outs[i.call_id] : JSON.stringify(outs[i.call_id])}`); }
