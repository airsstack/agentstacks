// Mock OpenAI-compatible chat server, with separate step lists for the main agent and subagents.
// usage: node mock2.mjs <port> <scenario.json> <log.jsonl>
// scenario: { "main": [steps], "child": [steps] }. A request whose tools include `subagent` is the
// main agent's; any other request with tools is a subagent's. Step: { "tool", "args" } or { "text" }.
// Requests without tools (titles) get "title". Every request body is appended to the log.
import http from "node:http";
import fs from "node:fs";

const [port, scenarioPath, logPath] = process.argv.slice(2);
const sc = JSON.parse(fs.readFileSync(scenarioPath, "utf8"));
const idx = { main: 0, child: 0 };
let callId = 0;

function chunk(res, delta, finish) {
  res.write("data: " + JSON.stringify({ id: "c1", object: "chat.completion.chunk", created: 0, model: "mock",
    choices: [{ index: 0, delta, finish_reason: finish ?? null }] }) + "\n\n");
}

http.createServer((req, res) => {
  let data = "";
  req.on("data", (c) => (data += c));
  req.on("end", () => {
    const body = safe(data);
    const tools = body?.tools ?? [];
    const who = tools.length === 0 ? "aux" : tools.some((t) => t.function?.name === "subagent") ? "main" : "child";
    fs.appendFileSync(logPath, JSON.stringify({ who, url: req.url, body }) + "\n");
    if (!req.url.includes("chat/completions")) { res.writeHead(404); res.end(); return; }
    const step = who === "aux" ? { text: "title" } : (sc[who][idx[who]++] ?? { text: "done" });
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
