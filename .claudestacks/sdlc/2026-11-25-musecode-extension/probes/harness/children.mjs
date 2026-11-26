// children.mjs <logdir> [marker-regex="BODY of ([A-Za-z_-]+)"]: for each child request (first per agent), print agent, model, reasoning, tool names.
import { readFileSync, readdirSync } from "node:fs";
const [dir, re = "BODY of ([A-Za-z_-]+)"] = process.argv.slice(2);
const seen = new Set();
for (const f of readdirSync(dir).filter((f) => /^req-\d+\.json$/.test(f)).sort()) {
  const t = readFileSync(`${dir}/${f}`, "utf8"); const m = t.match(new RegExp(re)); if (!m || seen.has(m[1])) continue; seen.add(m[1]);
  const j = JSON.parse(t); const tools = (j.tools || []).flatMap((x) => x.tools ? x.tools.map((y) => y.name) : [x.name || x.type]);
  console.log(`${f}\t${m[1]}\tmodel=${j.model}\treasoning=${JSON.stringify(j.reasoning)}\ttools=${tools.join(",")}`);
}
