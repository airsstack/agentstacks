// Runs the opencode extension inside a real OpenCode against a local mock model.
// usage: node check.mjs <opencode> <agentstacks>
//
// Builds a scratch project (the extension and the check pack copied in, `agentstacks init`
// applied), then runs scenarios S1-S7. Every OpenCode run is isolated: HOME and the XDG
// directories live in the scratch directory, the model is the mock from mock.mjs, and the child
// environment is built explicitly rather than inherited. Prints `ok <S>` or `FAIL <S>: <reason>`
// per scenario and exits 1 when any failed. Every child it starts (mock, server, OpenCode run)
// runs in its own process group; a run's group is killed when the run ends, and every mock and
// server group at the end.
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(HERE, "../../../..");
const MOCK = path.join(HERE, "mock.mjs");
const PACK = path.join(HERE, "pack");
const EXTENSION = path.join(REPO, "extensions", "opencode");

const NOTICE =
  "agentstacks: airsl is not on PATH, so the agentstacks Lua hooks are not running. " +
  "Install airsl, then run agentstacks doctor.";

const [opencodeArg, agentstacksArg] = process.argv.slice(2);
if (!opencodeArg || !agentstacksArg) {
  console.error("usage: node check.mjs <opencode> <agentstacks>");
  process.exit(2);
}
const OPENCODE = path.resolve(opencodeArg);
const AGENTSTACKS = path.resolve(agentstacksArg);

const SCRATCH = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), "opencode-check-")));
const live = new Set();

// Children are spawned detached, so each is the leader of its own process group; killing the
// group (negative pid) takes any grandchild with it.
function killTree(child) {
  try { process.kill(-child.pid, "SIGKILL"); } catch { /* group already gone */ }
  try { child.kill("SIGKILL"); } catch { /* already gone */ }
}

function killAll() {
  for (const child of live) killTree(child);
  live.clear();
}

function cleanup() {
  killAll();
  fs.rmSync(SCRATCH, { recursive: true, force: true });
}

process.on("SIGINT", () => { cleanup(); process.exit(130); });
process.on("SIGTERM", () => { cleanup(); process.exit(143); });

// ---- helpers --------------------------------------------------------------------------------

class Failure extends Error {}

function assert(cond, reason) {
  if (!cond) throw new Failure(reason);
}

function includes(text, needle, what) {
  assert(text.includes(needle), `${what} lacks ${JSON.stringify(needle)}: ${clip(text)}`);
}

function clip(text, n = 400) {
  const s = typeof text === "string" ? text : JSON.stringify(text);
  return s.length > n ? s.slice(0, n) + "..." : s;
}

function count(text, needle) {
  return text.split(needle).length - 1;
}

async function freePort() {
  return new Promise((resolve, reject) => {
    const srv = net.createServer();
    srv.once("error", reject);
    srv.listen(0, "127.0.0.1", () => {
      const { port } = srv.address();
      srv.close(() => resolve(port));
    });
  });
}

async function waitFor(probe, what, timeoutMs = 15000) {
  const end = Date.now() + timeoutMs;
  while (Date.now() < end) {
    if (await probe()) return;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Failure(`${what} did not come up`);
}

function portOpen(port) {
  return new Promise((resolve) => {
    const sock = net.connect(port, "127.0.0.1");
    sock.once("connect", () => { sock.destroy(); resolve(true); });
    sock.once("error", () => resolve(false));
  });
}

// Runs a command to completion with an explicit environment; resolves { code, stdout, stderr }.
function run(cmd, args, { cwd, env, timeoutMs = 120000 }) {
  return new Promise((resolve) => {
    const child = spawn(cmd, args, { cwd, env, stdio: ["ignore", "pipe", "pipe"], detached: true });
    live.add(child);
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (c) => (stdout += c));
    child.stderr.on("data", (c) => (stderr += c));
    const timer = setTimeout(() => killTree(child), timeoutMs);
    child.on("close", (code) => {
      clearTimeout(timer);
      killTree(child);
      live.delete(child);
      resolve({ code, stdout, stderr });
    });
    child.on("error", (err) => {
      clearTimeout(timer);
      killTree(child);
      live.delete(child);
      resolve({ code: null, stdout, stderr: stderr + String(err) });
    });
  });
}

function stop(child) {
  killTree(child);
  live.delete(child);
}

// The environment of an isolated OpenCode (or agentstacks) process: only what it needs.
function isolatedEnv(home, port, extra = {}) {
  return {
    PATH: process.env.PATH ?? "",
    HOME: home,
    XDG_CONFIG_HOME: path.join(home, ".config"),
    XDG_DATA_HOME: path.join(home, ".local", "share"),
    XDG_CACHE_HOME: path.join(home, ".cache"),
    XDG_STATE_HOME: path.join(home, ".local", "state"),
    TMPDIR: path.join(home, "tmp"),
    OPENCODE_DISABLE_MODELS_FETCH: "1",
    OPENCODE_DISABLE_AUTOUPDATE: "1",
    MOCK_PORT: String(port),
    ...extra,
  };
}

// Creates an isolated home with the global opencode.json registering provider `mock`.
function makeHome(name, port) {
  const home = path.join(SCRATCH, "home-" + name);
  fs.mkdirSync(path.join(home, ".config", "opencode"), { recursive: true });
  fs.mkdirSync(path.join(home, "tmp"), { recursive: true });
  const config = {
    $schema: "https://opencode.ai/config.json",
    model: "mock/m",
    providers: {
      mock: {
        name: "Mock",
        package: "@opencode/ai/providers/openai-compatible",
        settings: { baseURL: `http://127.0.0.1:${port}/v1`, apiKey: "x" },
        models: { m: { name: "M" } },
      },
    },
  };
  fs.writeFileSync(path.join(home, ".config", "opencode", "opencode.json"), JSON.stringify(config, null, 2));
  return home;
}

// Starts the mock model on a free port with the given scripted steps.
async function startMock(name, steps) {
  const port = await freePort();
  const scenario = path.join(SCRATCH, `${name}.scenario.json`);
  const log = path.join(SCRATCH, `${name}.requests.jsonl`);
  fs.writeFileSync(scenario, JSON.stringify(steps));
  fs.writeFileSync(log, "");
  const child = spawn(process.execPath, [MOCK, String(port), scenario, log], {
    stdio: "ignore",
    detached: true,
    env: { PATH: process.env.PATH ?? "" },
  });
  live.add(child);
  await waitFor(() => portOpen(port), "the mock model");
  return { port, log, stop: () => stop(child) };
}

// The chat requests that offered tools (OpenCode's title requests offer none), in order.
function chatRequests(log) {
  return fs
    .readFileSync(log, "utf8")
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line))
    .filter((r) => r.url.includes("chat/completions") && r.body?.tools?.length > 0)
    .map((r) => r.body);
}

function text(message) {
  const c = message?.content;
  if (typeof c === "string") return c;
  if (Array.isArray(c)) return c.map((p) => p?.text ?? "").join("");
  return "";
}

const userMessages = (req) => req.messages.filter((m) => m.role === "user").map(text);
const toolResults = (req) => req.messages.filter((m) => m.role === "tool").map(text);
const systemText = (req) => req.messages.filter((m) => m.role === "system").map(text).join("\n");
const toolNames = (req) => req.tools.map((t) => t.function.name).sort();

// The session id in the `--format json` output of `opencode run`.
function sessionId(stdout) {
  for (const line of stdout.split("\n")) {
    const m = line.match(/"sessionID"\s*:\s*"([^"]+)"/);
    if (m) return m[1];
  }
  throw new Failure(`no sessionID in the run output: ${clip(stdout)}`);
}

// A fresh copy of the applied project, so scenarios do not see each other's files.
function copyProject(from, name, extraFiles = {}) {
  const to = path.join(SCRATCH, "proj-" + name);
  fs.cpSync(from, to, { recursive: true });
  fs.writeFileSync(path.join(to, "a.txt"), "alpha\n");
  for (const [file, body] of Object.entries(extraFiles)) fs.writeFileSync(path.join(to, file), body);
  return to;
}

// One isolated `opencode run` against a mock with the given steps; returns the run and the
// requests the mock received. `before` may run more OpenCode commands against the same mock.
async function withOpencode(name, steps, project, body, { homeExtra = {} } = {}) {
  const mock = await startMock(name, steps);
  try {
    const home = makeHome(name, mock.port);
    const env = isolatedEnv(home, mock.port, homeExtra);
    const oc = (args) => run(OPENCODE, args, { cwd: project, env });
    await body({ oc, env, mock });
    return chatRequests(mock.log);
  } finally {
    mock.stop();
  }
}

const runArgs = (prompt, ...more) => ["run", "--standalone", "--auto", "--format", "json", ...more, prompt];

// ---- scratch project ------------------------------------------------------------------------

const projectEnv = () => {
  const home = path.join(SCRATCH, "home-agentstacks");
  fs.mkdirSync(path.join(home, "tmp"), { recursive: true });
  return {
    PATH: process.env.PATH ?? "",
    HOME: home,
    TMPDIR: path.join(home, "tmp"),
    GIT_CONFIG_GLOBAL: path.join(home, "gitconfig"),
    GIT_CONFIG_NOSYSTEM: "1",
  };
};

function buildProject() {
  const proj = path.join(SCRATCH, "proj");
  fs.mkdirSync(proj, { recursive: true });
  fs.cpSync(EXTENSION, path.join(proj, "ext", "opencode"), { recursive: true });
  fs.cpSync(PACK, path.join(proj, "packs", "check"), { recursive: true });
  return proj;
}

// PATH with airsl made unresolvable and nothing else taken away: each PATH directory that holds
// airsl is replaced, in its own place, by a scratch directory of symlinks to that directory's other
// executables, so name resolution order is unchanged. Throws when airsl would still resolve.
function pathWithoutAirsl(binDir) {
  const dirs = (process.env.PATH ?? "").split(path.delimiter).filter(Boolean);
  const hasAirsl = (dir) => fs.existsSync(path.join(dir, "airsl"));
  const out = [];
  for (const [index, dir] of dirs.entries()) {
    if (!hasAirsl(dir)) {
      out.push(dir);
      continue;
    }
    const linkDir = path.join(binDir, String(index));
    fs.mkdirSync(linkDir, { recursive: true });
    out.push(linkDir);
    for (const name of fs.readdirSync(dir)) {
      if (name === "airsl") continue;
      const target = path.join(dir, name);
      const link = path.join(linkDir, name);
      try {
        if (!fs.statSync(target).isFile()) continue;
        fs.accessSync(target, fs.constants.X_OK);
        if (!fs.existsSync(link)) fs.symlinkSync(target, link);
      } catch { /* not an executable file, or unreadable */ }
    }
  }
  if (out.some(hasAirsl)) throw new Failure("airsl is still resolvable on the S7 PATH");
  return out.join(path.delimiter);
}

// ---- scenarios ------------------------------------------------------------------------------

async function s1(proj) {
  const env = projectEnv();
  const ag = (args) => run(AGENTSTACKS, args, { cwd: proj, env });
  const git = spawnSync("git", ["init", "-q"], { cwd: proj, env });
  assert(git.status === 0, "git init failed");

  const init = await ag(["init", "--tool", "ext/opencode", "--pack", "packs/check", "--yes"]);
  assert(init.code === 0, `init exited ${init.code}: ${clip(init.stdout + init.stderr)}`);
  includes(init.stdout, "applied:", "init output");

  const plan = await ag(["plan"]);
  assert(plan.code === 0, `plan exited ${plan.code}: ${clip(plan.stdout + plan.stderr)}`);
  includes(plan.stdout, "0 to write", "plan output");
  includes(plan.stdout, "opencode has no hook when a session ends", "plan output");
  includes(plan.stdout, "opencode has no hook when a subagent stops", "plan output");

  const doctor = await ag(["doctor"]);
  assert(doctor.code === 0, `doctor exited ${doctor.code}: ${clip(doctor.stdout + doctor.stderr)}`);
  includes(doctor.stdout, "healthy", "doctor output");

  const update = await ag(["update"]);
  assert(update.code === 0, `update exited ${update.code}: ${clip(update.stdout + update.stderr)}`);
  includes(update.stdout, "nothing to update", "update output");
}

async function s2(proj) {
  const project = copyProject(proj, "S2");
  const reqs = await withOpencode(
    "S2",
    [{ tool: "skill", args: { id: "check-hello" } }, { text: "done" }],
    project,
    async ({ oc }) => {
      const r = await oc(runArgs("hello"));
      assert(r.code === 0, `opencode run exited ${r.code}: ${clip(r.stderr)}`);
    },
  );
  assert(reqs.length >= 2, `expected 2 chat requests, got ${reqs.length}`);
  const sys = systemText(reqs[0]);
  includes(sys, "check-hello", "the system prompt");
  assert(!sys.includes("check-ask"), "the system prompt lists the user-only skill check-ask");
  const result = toolResults(reqs[1])[0] ?? "";
  includes(result, "HELLO-BODY", "the skill result");
  includes(result, "Base directory for this skill:", "the skill result");
  // The body's own sentence, not the skill's file list, which names the same file.
  includes(
    result,
    "The note for this skill is at .opencode/skills/check-hello/references/note.md.",
    "the skill result",
  );
  assert(!result.includes("{{skill_dir"), `the skill result holds an unexpanded token: ${clip(result)}`);
}

async function s3(proj) {
  const project = copyProject(proj, "S3");
  const reqs = await withOpencode(
    "S3",
    [
      { tool: "subagent", args: { agent: "check-helper", description: "check helper", prompt: "Say hi." } },
      { text: "child done" },
      { text: "parent done" },
    ],
    project,
    async ({ oc }) => {
      const r = await oc(runArgs("delegate"));
      assert(r.code === 0, `opencode run exited ${r.code}: ${clip(r.stderr)}`);
    },
  );
  const child = reqs.filter((r) => systemText(r).includes("HELPER-BODY"));
  assert(child.length > 0, "no request carried HELPER-BODY");
  const names = toolNames(child[0]).join(",");
  assert(names === "grep,read,skill", `the helper was offered ${names}, not grep,read,skill`);
}

async function s4(proj) {
  const project = copyProject(proj, "S4");
  const reqs = await withOpencode(
    "S4",
    [
      { tool: "write", args: { path: "blocked.txt", content: "no\n" } },
      { tool: "write", args: { path: "ok.txt", content: "yes\n" } },
      { tool: "read", args: { path: "a.txt" } },
      { tool: "shell", args: { command: 'echo "PD=$AGENTSTACKS_PROJECT_DIR"' } },
      { text: "done" },
    ],
    project,
    async ({ oc }) => {
      const r = await oc(runArgs("go"));
      assert(r.code === 0, `opencode run exited ${r.code}: ${clip(r.stderr)}`);
    },
  );
  assert(reqs.length >= 5, `expected 5 chat requests, got ${reqs.length}`);
  const first = userMessages(reqs[0])[0] ?? "";
  includes(first, "CHECK-WELCOME", "the first user message");
  includes(first, "CHECK-START startup", "the first user message");
  includes(first, "CHECK-PROMPT", "the first user message");
  const results = toolResults(reqs[reqs.length - 1]);
  assert(results.length === 4, `expected 4 tool results, got ${results.length}: ${clip(results)}`);
  includes(results[0], "CHECK-BLOCK blocked.txt", "the blocked write result");
  assert(!fs.existsSync(path.join(project, "blocked.txt")), "blocked.txt was written");
  includes(results[1], "CHECK-BEFORE ok.txt", "the ok.txt write result");
  assert(fs.existsSync(path.join(project, "ok.txt")), "ok.txt was not written");
  includes(results[2], "CHECK-AFTER", "the read result");
  const pd = "PD=" + fs.realpathSync(project);
  assert(
    results[3].split("\n").some((line) => line.trim() === pd),
    `the shell result has no line ${JSON.stringify(pd)}: ${clip(results[3])}`,
  );
}

async function s5(proj) {
  const patch = (file, to) =>
    `*** Begin Patch\n*** Add File: ${file}\n+yes\n*** Update File: a.txt\n@@\n-alpha\n+${to}\n*** End Patch`;
  const project = copyProject(proj, "S5", {
    "opencode.json": JSON.stringify({
      $schema: "https://opencode.ai/config.json",
      model: "mock/gpt-5",
      providers: { mock: { models: { "gpt-5": { name: "G5" } } } },
    }),
  });
  const reqs = await withOpencode(
    "S5",
    [
      { tool: "patch", args: { patchText: patch("blocked.txt", "beta") } },
      { tool: "read", args: { path: "a.txt" } },
      { tool: "patch", args: { patchText: patch("ok2.txt", "gamma") } },
      { text: "done" },
    ],
    project,
    async ({ oc }) => {
      const r = await oc(runArgs("patch it"));
      assert(r.code === 0, `opencode run exited ${r.code}: ${clip(r.stderr)}`);
    },
  );
  assert(reqs.length >= 4, `expected 4 chat requests, got ${reqs.length}`);
  const results = toolResults(reqs[reqs.length - 1]);
  assert(results.length === 3, `expected 3 tool results, got ${results.length}: ${clip(results)}`);
  includes(results[0], "CHECK-BLOCK blocked.txt", "the first patch result");
  assert(!fs.existsSync(path.join(project, "blocked.txt")), "blocked.txt was written");
  // The read between the patches sees a.txt as it was before the blocked patch.
  includes(results[1], "alpha", "the read after the blocked patch");
  assert(!results[1].includes("beta"), `the blocked patch changed a.txt: ${clip(results[1])}`);
  includes(results[2], "CHECK-BEFORE ok2.txt", "the second patch result");
  assert(fs.existsSync(path.join(project, "ok2.txt")), "ok2.txt was not written");
  const a = fs.readFileSync(path.join(project, "a.txt"), "utf8");
  assert(a === "gamma\n", `a.txt is ${JSON.stringify(a)} after the second patch, not "gamma\\n"`);
}

async function s6(proj) {
  const project = copyProject(proj, "S6");
  let session = "";
  const reqs = await withOpencode(
    "S6",
    [{ text: "ok" }, { text: "ok again" }],
    project,
    async ({ oc }) => {
      const first = await oc(runArgs("hi"));
      assert(first.code === 0, `opencode run exited ${first.code}: ${clip(first.stderr)}`);
      session = sessionId(first.stdout);
      const api = await oc([
        "api", "--standalone", "session.command",
        "--param", `sessionID=${session}`,
        "-d", JSON.stringify({ name: "check-ask", text: "a b" }),
      ]);
      assert(api.code === 0, `opencode api exited ${api.code}: ${clip(api.stdout + api.stderr)}`);
      const second = await oc(runArgs("next", "--session", session));
      assert(second.code === 0, `opencode run --session exited ${second.code}: ${clip(second.stderr)}`);
    },
  );
  const last = reqs[reqs.length - 1];
  const expanded = userMessages(last).find((m) => m.includes("Load the skill `check-ask`"));
  assert(expanded, `no user message holds the command body: ${clip(userMessages(last))}`);
  includes(expanded, "a b", "the expanded command");
}

async function s7(proj) {
  const project = copyProject(proj, "S7");
  const steps = [
    { tool: "write", args: { path: "blocked.txt", content: "no\n" } },
    { text: "done-1" },
    { text: "done-2" },
  ];
  const mock = await startMock("S7", steps);
  let server;
  try {
    const home = makeHome("S7", mock.port);
    const noAirsl = pathWithoutAirsl(path.join(SCRATCH, "bin-S7"));
    const port = await freePort();
    const password = "check-" + port;
    const serverEnv = isolatedEnv(home, mock.port, { OPENCODE_PASSWORD: password, PATH: noAirsl });
    server = spawn(OPENCODE, ["serve", "--port", String(port)], {
      cwd: project,
      env: serverEnv,
      stdio: "ignore",
      detached: true,
    });
    live.add(server);
    await waitFor(() => portOpen(port), "opencode serve");

    const clientEnv = isolatedEnv(home, mock.port, { OPENCODE_PASSWORD: password });
    const client = (prompt, ...more) =>
      run(OPENCODE, ["run", "--server", `http://127.0.0.1:${port}`, "--auto", "--format", "json", ...more, prompt], {
        cwd: project,
        env: clientEnv,
      });
    const one = await client("hello");
    assert(one.code === 0, `client 1 exited ${one.code}: ${clip(one.stderr)}`);
    const session = sessionId(one.stdout);
    const two = await client("again", "--session", session);
    assert(two.code === 0, `client 2 exited ${two.code}: ${clip(two.stderr)}`);

    const reqs = chatRequests(mock.log);
    const users = userMessages(reqs[reqs.length - 1]);
    assert(users.length === 2, `expected 2 user messages, got ${users.length}: ${clip(users)}`);
    const [first, second] = users;
    includes(first, "CHECK-WELCOME", "prompt 1's user message");
    assert(first.trimEnd().endsWith(NOTICE), `prompt 1's user message does not end with the notice: ${clip(first)}`);
    assert(count(users.join("\n"), NOTICE) === 1, "the notice does not appear exactly once");
    assert(!first.includes("CHECK-START"), "prompt 1 holds CHECK-START");
    assert(!first.includes("CHECK-PROMPT"), "prompt 1 holds CHECK-PROMPT");
    assert(!second.includes("airsl is not on PATH"), `prompt 2 holds the notice: ${clip(second)}`);
    assert(fs.existsSync(path.join(project, "blocked.txt")), "blocked.txt was not written (the guard should be off)");
  } finally {
    if (server) stop(server);
    mock.stop();
  }
}

// ---- main -----------------------------------------------------------------------------------

async function main() {
  const version = spawnSync(OPENCODE, ["--version"], { env: { PATH: process.env.PATH ?? "", HOME: SCRATCH } });
  console.error(`opencode ${String(version.stdout).trim()}`);

  const proj = buildProject();
  let failed = 0;
  const scenarios = [["S1", s1], ["S2", s2], ["S3", s3], ["S4", s4], ["S5", s5], ["S6", s6], ["S7", s7]];
  for (const [name, fn] of scenarios) {
    try {
      await fn(proj);
      console.log(`ok ${name}`);
    } catch (err) {
      failed += 1;
      console.log(`FAIL ${name}: ${err instanceof Failure ? err.message : (err?.stack ?? err)}`);
      // Later scenarios copy the applied project, so a failed S1 leaves nothing to run on.
      if (name === "S1") break;
    }
  }
  return failed;
}

let failed = 1;
try {
  failed = await main();
} finally {
  cleanup();
}
process.exit(failed === 0 ? 0 : 1);
