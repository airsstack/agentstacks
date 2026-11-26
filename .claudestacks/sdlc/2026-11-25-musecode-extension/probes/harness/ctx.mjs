// ctx.mjs <file> <needle> [radius=200] [max=5]: print each occurrence of needle in the file's raw text with context.
import { readFileSync } from "node:fs";
const [f, needle, r = "200", max = "5"] = process.argv.slice(2);
const t = readFileSync(f, "utf8"); let i = -1, n = 0;
while ((i = t.indexOf(needle, i + 1)) !== -1 && n < Number(max)) { n++;
  console.log(`--- ${f.split("/").slice(-2).join("/")} @${i}: …${t.slice(Math.max(0, i - Number(r)), i + needle.length + Number(r))}…`); }
if (!n) console.log(`--- ${f.split("/").slice(-2).join("/")}: no occurrence of ${JSON.stringify(needle)}`);
