// agentstacks hook shim for OpenCode, api 0. Written by `agentstacks apply`; edits are drift.
// It runs the hook commands in HOOKS and forwards each event and result. All hook logic is Lua.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const HOOKS = [
  {"on":"session.start","text":"Welcome"},
  {"command":"\"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \".opencode/agentstacks/runtime.lua\" \"demo.hooks.guard\" \"0\" \"neutral\"","on":"tool.before","tools":["read","edit","write","patch"]}
];

const ROOT = fileURLToPath(new URL("../../", import.meta.url)).replace(/\/+$/, "");
const TIMEOUT_MS = 60000;
const AIRSL_MISSING =
  "agentstacks: airsl is not on PATH, so the agentstacks Lua hooks are not running. Install airsl, then run agentstacks doctor.";

// Set when sh could not find a hook command's program (exit 127): every Lua hook runs airsl.
let airslMissing = false;

// Runs one hook command under sh from the project root, with `payload` as JSON on stdin.
// Resolves to its stdout, or null when it cannot run or runs past the timeout.
function run(command, payload) {
  return new Promise((resolve) => {
    let out = "";
    let settled = false;
    const finish = (value) => {
      if (!settled) {
        settled = true;
        resolve(value);
      }
    };
    let child;
    try {
      child = spawn("/bin/sh", ["-c", command], {
        cwd: ROOT,
        env: { ...process.env, AGENTSTACKS_PROJECT_DIR: ROOT },
        stdio: ["pipe", "pipe", "ignore"],
      });
    } catch {
      finish(null);
      return;
    }
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      finish(null);
    }, TIMEOUT_MS);
    child.on("error", () => {
      clearTimeout(timer);
      finish(null);
    });
    child.stdout.on("data", (chunk) => {
      out += chunk;
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      if (code === 127) airslMissing = true;
      finish(out);
    });
    child.stdin.on("error", () => {});
    child.stdin.end(JSON.stringify(payload));
  });
}

function parse(text) {
  try {
    const value = JSON.parse(text);
    return value !== null && typeof value === "object" ? value : null;
  } catch {
    return null;
  }
}

// Runs, in table order, every entry whose `on` is in `events` and whose `tools` holds `tool`.
// Returns the joined contexts and the first block.
async function dispatch(events, tool, payload) {
  const contexts = [];
  for (const entry of HOOKS) {
    if (!events.includes(entry.on)) continue;
    if (entry.tools && !entry.tools.includes(tool)) continue;
    if (typeof entry.text === "string") {
      contexts.push(entry.text);
      continue;
    }
    const result = parse((await run(entry.command, { ...payload, hook: entry.on, match: entry.match ?? null })) ?? "");
    if (result === null) continue;
    if (typeof result.context === "string") contexts.push(result.context);
    if (typeof result.block === "string") return { context: contexts.join("\n"), block: result.block };
  }
  return { context: contexts.join("\n"), block: null };
}

function joined(block, context) {
  return context ? block + "\n" + context : block;
}

export default {
  id: "agentstacks",
  setup: async (ctx) => {
    const parents = new Map();
    const started = new Set();
    let told = false;
    const held = new Map();

    async function parentOf(sessionID) {
      if (!parents.has(sessionID)) {
        let parent = null;
        try {
          const info = await ctx.session.get({ sessionID });
          parent = typeof info?.parentID === "string" ? info.parentID : null;
        } catch {}
        parents.set(sessionID, parent);
      }
      return parents.get(sessionID);
    }

    async function base(sessionID) {
      return { session_id: sessionID, parent_id: await parentOf(sessionID), cwd: ctx.location.directory };
    }

    await ctx.shell.hook("create.before", (e) => {
      try {
        e.env.AGENTSTACKS_PROJECT_DIR = ROOT;
      } catch {}
    });

    await ctx.tool.hook("execute.before", async (e) => {
      let block = null;
      try {
        const payload = { ...(await base(e.sessionID)), agent: e.agent, tool: e.tool, input: e.input };
        const r = await dispatch(["tool.before", "tool.execute.before"], e.tool, payload);
        if (r.block !== null) block = joined(r.block, r.context);
        else if (r.context) held.set(e.id, r.context);
      } catch {}
      if (block !== null) throw new Error(block);
    });

    await ctx.tool.hook("execute.after", async (e) => {
      let block = null;
      try {
        const before = held.get(e.id);
        held.delete(e.id);
        if (e.status !== "completed") return;
        const payload = { ...(await base(e.sessionID)), agent: e.agent, tool: e.tool, input: e.input };
        const r = await dispatch(["tool.after", "tool.execute.after"], e.tool, payload);
        const context = [before, r.context].filter(Boolean).join("\n");
        if (r.block !== null) block = joined(r.block, context);
        else if (context) {
          e.result = { ...e.result, content: [...(e.result.content ?? []), { type: "text", text: context }] };
        }
      } catch {}
      if (block !== null) throw new Error(block);
    });

    await ctx.session.hook("prompt", async (e) => {
      try {
        const payload = await base(e.sessionID);
        if (payload.parent_id !== null) return;
        const parts = [];
        if (!started.has(e.sessionID)) {
          started.add(e.sessionID);
          const s = await dispatch(["session.start"], null, { ...payload, source: "startup" });
          if (s.context) parts.push(s.context);
          if (s.block !== null) parts.push(s.block);
        }
        const r = await dispatch(["prompt.submit", "session.prompt"], null, { ...payload, prompt: e.prompt.text });
        if (r.context) parts.push(r.context);
        if (airslMissing && !told) {
          told = true;
          parts.push(AIRSL_MISSING);
        }
        if (parts.length > 0) e.prompt.text = e.prompt.text + "\n\n" + parts.join("\n");
      } catch {}
    });
  },
};
