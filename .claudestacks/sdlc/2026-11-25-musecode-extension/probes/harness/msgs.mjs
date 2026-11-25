// msgs.mjs <req.json> [role] [maxchars=1500]: print input items (type/role) with content as JSON strings, optionally only one role.
import { readFileSync } from "node:fs";
const [f, role, max = "1500"] = process.argv.slice(2);
const j = JSON.parse(readFileSync(f, "utf8"));
console.log(`### ${f.split("/").slice(-2).join("/")} instructions[0..80]=${JSON.stringify((j.instructions || "").slice(0, 80))}`);
for (const [k, i] of (j.input || []).entries()) {
  if (role && i.role !== role && i.type !== role) continue;
  const c = i.content ?? i.output ?? i.arguments ?? "";
  const s = typeof c === "string" ? c : JSON.stringify(c);
  console.log(`[${k}] type=${i.type} role=${i.role ?? ""} name=${i.name ?? ""} len=${s.length}: ${JSON.stringify(s.length > Number(max) ? "…" + s.slice(-Number(max)) : s)}`);
}
