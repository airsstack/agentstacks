// Mock OpenAI-compatible chat server for the opencode-check run.
// usage: node mock.mjs <port> <scenario.json> <log.jsonl>
// scenario: array of steps, one consumed per request that offers tools:
//   { "tool": "<name>", "args": { ... } }  -> one tool call
//   { "text": "..." }                        -> final assistant text
// A request that offers no tools (OpenCode's title request) is answered `title` and consumes no
// step. When the steps run out it answers `done`. Every request body is appended to the log as one
// JSON line `{ "url": ..., "body": ... }`.
import http from "node:http";
import fs from "node:fs";

const [port, scenarioPath, logPath] = process.argv.slice(2);
const steps = JSON.parse(fs.readFileSync(scenarioPath, "utf8"));
let next = 0;
let callId = 0;

function chunk(res, delta, finish) {
  const body = {
    id: "c1", object: "chat.completion.chunk", created: 0, model: "mock",
    choices: [{ index: 0, delta, finish_reason: finish ?? null }],
  };
  res.write("data: " + JSON.stringify(body) + "\n\n");
}

function safe(text) {
  try { return JSON.parse(text); } catch { return text; }
}

http.createServer((req, res) => {
  let data = "";
  req.on("data", (c) => (data += c));
  req.on("end", () => {
    fs.appendFileSync(logPath, JSON.stringify({ url: req.url, body: safe(data) }) + "\n");
    if (!req.url.includes("chat/completions")) {
      res.writeHead(404);
      res.end();
      return;
    }
    const body = safe(data);
    const offersTools = !(!body?.tools || body.tools.length === 0);
    const step = offersTools ? (steps[next++] ?? { text: "done" }) : { text: "title" };
    res.writeHead(200, { "content-type": "text/event-stream" });
    if (step.tool) {
      chunk(res, {
        role: "assistant",
        tool_calls: [{
          index: 0, id: "call_" + ++callId, type: "function",
          function: { name: step.tool, arguments: JSON.stringify(step.args ?? {}) },
        }],
      });
      chunk(res, {}, "tool_calls");
    } else {
      chunk(res, { role: "assistant", content: step.text });
      chunk(res, {}, "stop");
    }
    res.write("data: [DONE]\n\n");
    res.end();
  });
}).listen(Number(port), "127.0.0.1");
