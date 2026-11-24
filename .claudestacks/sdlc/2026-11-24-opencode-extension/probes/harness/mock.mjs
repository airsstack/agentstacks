// Mock OpenAI-compatible chat server for OpenCode probes.
// usage: node mock.mjs <port> <scenario.json> <log.jsonl>
// scenario: array of steps, one consumed per request:
//   { "tool": "<name>", "args": { ... } }  -> one tool call
//   { "text": "..." }                        -> final assistant text
// When steps run out, it answers "done". Every request body is appended to the log.
import http from "node:http";
import fs from "node:fs";

const [port, scenarioPath, logPath] = process.argv.slice(2);
const steps = JSON.parse(fs.readFileSync(scenarioPath, "utf8"));
let i = 0;
let callId = 0;

function chunk(res, delta, finish) {
  const body = {
    id: "c1", object: "chat.completion.chunk", created: 0, model: "mock",
    choices: [{ index: 0, delta, finish_reason: finish ?? null }],
  };
  res.write("data: " + JSON.stringify(body) + "\n\n");
}

http.createServer((req, res) => {
  let data = "";
  req.on("data", (c) => (data += c));
  req.on("end", () => {
    fs.appendFileSync(logPath, JSON.stringify({ url: req.url, body: safe(data) }) + "\n");
    if (!req.url.includes("chat/completions")) {
      res.writeHead(404); res.end(); return;
    }
    const body = safe(data);
    // Title / summary requests from OpenCode itself get a plain text answer.
    const sys = JSON.stringify(body?.messages?.[0] ?? "");
    const isAux = !body?.tools || body.tools.length === 0;
    const step = isAux ? { text: "title" } : (steps[i++] ?? { text: "done" });
    res.writeHead(200, { "content-type": "text/event-stream" });
    if (step.tool) {
      chunk(res, { role: "assistant", tool_calls: [{ index: 0, id: "call_" + ++callId, type: "function",
        function: { name: step.tool, arguments: JSON.stringify(step.args ?? {}) } }] });
      chunk(res, {}, "tool_calls");
    } else {
      chunk(res, { role: "assistant", content: step.text });
      chunk(res, {}, "stop");
    }
    res.write("data: [DONE]\n\n");
    res.end();
  });
}).listen(Number(port), "127.0.0.1");

function safe(t) { try { return JSON.parse(t); } catch { return t; } }
