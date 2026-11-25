// Prints, per model request, who asked and the tool results it carries (each clipped).
// usage: node results.mjs <requests.jsonl>
import fs from "node:fs";
const lines = fs.readFileSync(process.argv[2], "utf8").trim().split("\n").map((l) => JSON.parse(l));
lines.forEach((r, i) => {
  if (r.who === "aux") return;
  const msgs = r.body?.messages ?? [];
  const calls = {};
  msgs.forEach((m) => (m.tool_calls ?? []).forEach((c) => (calls[c.id] = c.function.name)));
  const results = msgs.filter((m) => m.role === "tool");
  console.log(`request ${i} (${r.who}) tool results: ${results.length}`);
  results.forEach((m) => {
    const text = typeof m.content === "string" ? m.content : JSON.stringify(m.content);
    console.log(`  ${calls[m.tool_call_id] ?? "?"}: ${text.replace(/\s+/g, " ").slice(0, 260)}`);
  });
  const last = msgs[msgs.length - 1];
  if (last && last.role === "user") {
    const t = typeof last.content === "string" ? last.content : JSON.stringify(last.content);
    console.log(`  last user message: ${t.replace(/\s+/g, " ").slice(0, 200)}`);
  }
});
