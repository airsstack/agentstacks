// Mock model endpoint for the musecode-check run: the endpoint Muse Code reads from its settings.
//   GET  <base>/muse-code/models  -> one model, `fake-model`
//   POST <base>/responses         -> Responses SSE
// usage: node mock.mjs <port> <script.json> <logDir>
//   <port> 0 picks any free port. Once bound the mock prints `listening <port>` on stdout.
// script: an array of steps, each served once, in order of the first match:
//   { "when": ["substr", ...], "unless": ["substr", ...], "name": "<tool>", "args": { ... } }
// A step without a `name` is refused: the mock prints the reason on stderr and exits 1 before it
// listens. A request body matches a step when it holds every `when` substring and none of the
// `unless` substrings. A request no step matches is answered the text `MOCKDONE`. A step answers
// one function call, whose `args` object is sent as the JSON-encoded call arguments. The tool names
// and argument shapes Muse Code accepts are:
//   read_skill      { name }
//   subagent_spawn  { command_id, role, objective, subagent_type }
//   bash            { command, description }
//   write_file      { path, content }
//   read_file       { path, offset, limit }
// Every request body is written to <logDir>/req-NNN.json, numbered from 001.
import { createServer } from "node:http";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

const [port, scriptPath, logDir] = process.argv.slice(2);
const script = JSON.parse(readFileSync(scriptPath, "utf8"));
script.forEach((s, i) => {
  if (typeof s.name !== "string" || s.name === "") {
    console.error(`mock: step ${i} has no name`);
    process.exit(1);
  }
});
mkdirSync(logDir, { recursive: true });
let n = 0;
const sse = (v) => `data: ${JSON.stringify(v)}\n\n`;
const frame = (id, status, extra = {}) => ({ id, object: "response", model: "fake-model", status, output: [], ...extra });

const server = createServer((req, res) => {
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
    const step = script.find((s) => !s.done && (s.when || []).every((w) => body.includes(w)) && !(s.unless || []).some((w) => body.includes(w)));
    res.writeHead(200, { "content-type": "text/event-stream" });
    const created = sse({ type: "response.created", sequence_number: 1, response: frame("r" + n, "in_progress") });
    let head;
    if (step) {
      step.done = true;
      const cid = `call_${n}`;
      head = created + sse({ type: "response.function_call_arguments.done", sequence_number: 2, output_index: 0, item_id: "fc_" + cid, name: step.name, call_id: cid, arguments: JSON.stringify(step.args) });
    } else {
      head = created + sse({ type: "response.output_text.delta", sequence_number: 2, output_index: 0, item_id: "m" + n, content_index: 0, delta: "MOCKDONE" });
    }
    res.write(head);
    res.end(sse({ type: "response.completed", sequence_number: 3, response: frame("r" + n, "completed", { usage: { input_tokens: 1, output_tokens: 1, total_tokens: 2 } }) }));
  });
});
server.listen(Number(port), "127.0.0.1", () => {
  console.log(`listening ${server.address().port}`);
});
