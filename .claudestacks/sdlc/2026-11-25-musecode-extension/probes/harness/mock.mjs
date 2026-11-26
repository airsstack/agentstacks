// Fake Muse endpoint: GET /muse-code/models, POST /responses (Responses SSE).
// argv: port scriptJson logDir. script = [{when:[substr...], name, args}] each served once, in order.
import { createServer } from "node:http";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
const [port, scriptPath, logDir] = process.argv.slice(2);
const script = JSON.parse(readFileSync(scriptPath, "utf8"));
mkdirSync(logDir, { recursive: true });
let n = 0;
const sse = (v) => `data: ${JSON.stringify(v)}\n\n`;
const frame = (id, status, extra = {}) => ({ id, object: "response", model: "fake-model", status, output: [], ...extra });
createServer((req, res) => {
  const chunks = [];
  req.on("data", (c) => chunks.push(c));
  req.on("end", () => {
    const body = Buffer.concat(chunks).toString("utf8");
    if (req.method === "GET" && req.url.endsWith("/muse-code/models")) {
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ object: "list", data: [{ id: "fake-model", object: "model",
        metadata: { "muse-code": { release_date: "2026-01-01", is_hidden: false, limit: { context: 1000000, output: 1024 } } } }] }));
      return;
    }
    n += 1;
    writeFileSync(`${logDir}/req-${String(n).padStart(3, "0")}.json`, body);
    const step = script.find((s) => !s.done && s.when.every((w) => body.includes(w)) && !(s.unless || []).some((w) => body.includes(w)));
    res.writeHead(200, { "content-type": "text/event-stream" });
    let head;
    if (step) {
      step.done = true;
      const cid = `call_${n}`;
      head = sse({ type: "response.created", sequence_number: 1, response: frame("r" + n, "in_progress") }) +
        sse({ type: "response.function_call_arguments.done", sequence_number: 2, output_index: 0, item_id: "fc_" + cid, name: step.name, call_id: cid, arguments: JSON.stringify(step.args) });
    } else {
      head = sse({ type: "response.created", sequence_number: 1, response: frame("r" + n, "in_progress") }) +
        sse({ type: "response.output_text.delta", sequence_number: 2, output_index: 0, item_id: "m" + n, content_index: 0, delta: "MOCKDONE" });
    }
    res.write(head);
    res.end(sse({ type: "response.completed", sequence_number: 3, response: frame("r" + n, "completed", { usage: { input_tokens: 1, output_tokens: 1, total_tokens: 2 } }) }));
  });
}).listen(Number(port), "127.0.0.1");
