// Runs the musecode extension inside a real Muse Code against a local mock model.
// usage: node check.mjs <muse> <agentstacks>
//
// Builds a scratch project (the extension and the check pack copied in), then runs scenarios
// S1-S6. Every Muse Code run is isolated: HOME and TMPDIR live in the scratch directory,
// XDG_CONFIG_HOME is unset, the credential backend is the file one and the model is the mock from
// mock.mjs. The child environment is built explicitly, never inherited: HOME points at the scratch
// directory. A mock that has not printed `listening <port>` within 10 s stops the check, and Muse
// Code is never started against it. A Muse Code run is never spawned unless its environment passes
// the guard in museEnvProblem. Muse Code output that mentions the macOS Keychain is checked after
// each run returns and stops the check. Prints `ok S<n>` or `FAIL S<n> <reason>` per scenario and
// exits 1 when any failed. Every child (mock, Muse Code run) is the leader of its own process group, killed when the
// run ends; the scratch directory is removed at exit or on a signal.
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { isDeepStrictEqual } from "node:util";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const REPO = path.resolve(HERE, "../../../..");
const MOCK = path.join(HERE, "mock.mjs");
const PACK = path.join(HERE, "pack");
const EXTENSION = path.join(REPO, "extensions", "musecode");

const SKIP_H2 = "musecode shows no hook output to the model at session end";
const SKIP_H3 =
  "musecode gives hooks no agent name at agent stop, so it cannot run this hook for only the listed agents";
const GOAL_REMINDER = "You are a goal reminder";

const [museArg, agentstacksArg] = process.argv.slice(2);
if (!museArg || !agentstacksArg) {
  console.error("usage: node check.mjs <muse> <agentstacks>");
  process.exit(2);
}
const MUSE = path.resolve(museArg);
const AGENTSTACKS = path.resolve(agentstacksArg);

const SCRATCH = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), "musecode-check-")));
const live = new Set();

// Children are spawned detached, so each is the leader of its own process group; killing the
// group (negative pid) takes any grandchild with it.
function killTree(child) {
  try { process.kill(-child.pid, "SIGKILL"); } catch { /* group already gone */ }
  try { child.kill("SIGKILL"); } catch { /* already gone */ }
}

function cleanup() {
  for (const child of live) killTree(child);
  live.clear();
  fs.rmSync(SCRATCH, { recursive: true, force: true });
}

function die(code, message) {
  console.log(message);
  cleanup();
  process.exit(code);
}

process.on("SIGINT", () => { cleanup(); process.exit(130); });
process.on("SIGTERM", () => { cleanup(); process.exit(143); });
process.on("SIGHUP", () => { cleanup(); process.exit(129); });

// ---- helpers --------------------------------------------------------------------------------

class Failure extends Error {}

function assert(cond, reason) {
  if (!cond) throw new Failure(reason);
}

function clip(text, n = 600) {
  const s = typeof text === "string" ? text : JSON.stringify(text);
  return s.length > n ? s.slice(0, n) + "..." : s;
}

function includes(text, needle, what) {
  assert(text.includes(needle), `${what} lacks ${JSON.stringify(needle)}: ${clip(text)}`);
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
    const end = (code, extra = "") => {
      clearTimeout(timer);
      killTree(child);
      live.delete(child);
      resolve({ code, stdout, stderr: stderr + extra });
    };
    child.on("close", (code) => end(code));
    child.on("error", (err) => end(null, String(err)));
  });
}

// ---- the mock model -------------------------------------------------------------------------

// Starts the mock on a free port with the given scripted steps and waits for its `listening` line;
// no line within 10 s ends the check.
async function startMock(name, steps) {
  const scenario = path.join(SCRATCH, `${name}.script.json`);
  const log = path.join(SCRATCH, `${name}.requests`);
  fs.writeFileSync(scenario, JSON.stringify(steps));
  const child = spawn(process.execPath, [MOCK, "0", scenario, log], {
    stdio: ["ignore", "pipe", "pipe"],
    detached: true,
    env: { PATH: process.env.PATH ?? "" },
  });
  live.add(child);
  let stderr = "";
  child.stderr.on("data", (chunk) => (stderr += chunk));
  const port = await new Promise((resolve) => {
    let out = "";
    const timer = setTimeout(() => resolve(null), 10000);
    child.stdout.on("data", (chunk) => {
      out += chunk;
      const m = out.match(/^listening (\d+)$/m);
      if (m) {
        clearTimeout(timer);
        resolve(Number(m[1]));
      }
    });
    child.on("close", () => { clearTimeout(timer); resolve(null); });
  });
  if (port === null) die(1, `FAIL mock did not start: ${clip(stderr.trim() || "no stderr output")}`);
  return { port, log, stop: () => { killTree(child); live.delete(child); } };
}

// The model requests the mock received, parsed, in order.
function requests(log) {
  if (!fs.existsSync(log)) return [];
  return fs
    .readdirSync(log)
    .filter((f) => /^req-\d+\.json$/.test(f))
    .sort((a, b) => Number(a.match(/\d+/)[0]) - Number(b.match(/\d+/)[0]))
    .map((f) => JSON.parse(fs.readFileSync(path.join(log, f), "utf8")));
}

// Every string in a JSON value, joined: the text a request carries, without JSON escaping.
function flat(value) {
  if (typeof value === "string") return value;
  if (Array.isArray(value)) return value.map(flat).join("\n");
  if (value && typeof value === "object") return Object.values(value).map(flat).join("\n");
  return "";
}

// The tool names a request offers; a tool may be listed flat or inside a group.
const toolNames = (req) =>
  (req.tools ?? []).flatMap((t) => (t.tools ? t.tools.map((u) => u.name) : [t.name]));

// Requests of the lead session: not a child's (`HELPER-BODY`), not a goal reminder.
const leadRequests = (reqs) =>
  reqs.filter((r) => !flat(r).includes("HELPER-BODY") && !flat(r).includes(GOAL_REMINDER));

// Each tool call the lead made, with its arguments and the output it got back (from the last
// lead request that carries it).
function toolCalls(reqs) {
  const calls = new Map();
  for (const req of leadRequests(reqs)) {
    for (const item of req.input ?? []) {
      if (item.type === "function_call") {
        const prev = calls.get(item.call_id) ?? {};
        calls.set(item.call_id, {
          ...prev,
          name: String(item.name).replace(/^muse\./, ""),
          args: JSON.parse(item.arguments),
        });
      } else if (item.type === "function_call_output") {
        const prev = calls.get(item.call_id) ?? {};
        calls.set(item.call_id, { ...prev, output: flat(item.output) });
      }
    }
  }
  return [...calls.values()];
}

function callOf(reqs, name, match = () => true) {
  const call = toolCalls(reqs).find((c) => c.name === name && match(c.args));
  assert(call, `the model made no ${name} call: ${clip(toolCalls(reqs))}`);
  assert(call.output !== undefined, `the ${name} call got no result: ${clip(call)}`);
  return call;
}

// ---- scratch projects and Muse Code runs ----------------------------------------------------

// The environment of an agentstacks command: only what it needs.
function agentstacksEnv(home) {
  return {
    PATH: process.env.PATH ?? "",
    HOME: home,
    TMPDIR: path.join(home, "tmp"),
    GIT_CONFIG_GLOBAL: path.join(home, "gitconfig"),
    GIT_CONFIG_NOSYSTEM: "1",
  };
}

// The environment of a Muse Code run: only what it needs, a scratch HOME and TMPDIR, the file
// credential backend and no XDG_CONFIG_HOME.
function museEnv(home, pathValue = process.env.PATH ?? "") {
  return {
    PATH: pathValue,
    HOME: home,
    TMPDIR: path.join(home, "tmp"),
    TBH_CREDENTIAL_BACKEND: "file",
  };
}

// The safety rule for every Muse Code run: the file credential backend, HOME and TMPDIR inside the
// scratch directory, no XDG_CONFIG_HOME. Returns what is wrong with `env`, or null when it is safe.
function museEnvProblem(env) {
  const inScratch = (dir) => typeof dir === "string" && path.resolve(dir).startsWith(SCRATCH + path.sep);
  if (env.TBH_CREDENTIAL_BACKEND !== "file") return "TBH_CREDENTIAL_BACKEND is not file";
  if (!inScratch(env.HOME)) return "HOME is not under the scratch directory";
  if (!inScratch(env.TMPDIR)) return "TMPDIR is not under the scratch directory";
  if (Object.hasOwn(env, "XDG_CONFIG_HOME")) return "XDG_CONFIG_HOME is set";
  return null;
}

// A scratch home whose Muse Code settings point at the mock on `port`.
function makeHome(name, port) {
  const home = path.join(SCRATCH, "home-" + name);
  const config = path.join(home, ".config", "muse");
  fs.mkdirSync(config, { recursive: true });
  fs.mkdirSync(path.join(home, "tmp"), { recursive: true });
  fs.writeFileSync(
    path.join(config, "settings.json"),
    JSON.stringify({ schema_version: 1, endpoint_transport: { base_url: `http://127.0.0.1:${port}`, auth: "bearer" } }),
  );
  fs.writeFileSync(
    path.join(config, "auth.json"),
    JSON.stringify({ schema_version: 1, providers: { meta: { api_key: "dummy" } } }),
  );
  return home;
}

// A git project holding the extension at ext/musecode and the check pack at packs/check.
function buildProject(name) {
  const proj = path.join(SCRATCH, "proj-" + name);
  fs.mkdirSync(proj, { recursive: true });
  fs.cpSync(EXTENSION, path.join(proj, "ext", "musecode"), { recursive: true });
  fs.cpSync(PACK, path.join(proj, "packs", "check"), { recursive: true });
  const home = path.join(SCRATCH, "home-agentstacks-" + name);
  fs.mkdirSync(path.join(home, "tmp"), { recursive: true });
  const git = spawnSync("git", ["init", "-q", "."], { cwd: proj, env: agentstacksEnv(home) });
  assert(git.status === 0, `git init failed: ${git.stderr}`);
  return { proj, home };
}

const agentstacks = (project, args) =>
  run(AGENTSTACKS, args, { cwd: project.proj, env: agentstacksEnv(project.home) });

async function initProject(name, beforeInit = () => {}) {
  const project = buildProject(name);
  beforeInit(project.proj);
  const init = await agentstacks(project, ["init", "--tool", "ext/musecode", "--pack", "packs/check", "--yes"]);
  assert(init.code === 0, `init exited ${init.code}: ${clip(init.stdout + init.stderr)}`);
  return { ...project, init };
}

// One `muse exec` against a mock with the given steps, from the project root; returns the
// requests the mock received and the run.
async function withMuse(name, steps, proj, prompt, { pathValue } = {}) {
  const mock = await startMock(name, steps);
  try {
    const home = makeHome(name, mock.port);
    const env = museEnv(home, pathValue);
    const problem = museEnvProblem(env);
    if (problem) die(1, `FAIL ${name.toUpperCase()} unsafe Muse Code environment, not started: ${problem}`);
    const result = await run(MUSE, ["exec", "--trust-workspace", "--yolo", prompt], { cwd: proj, env });
    const output = result.stdout + result.stderr;
    if (/keychain|security framework/i.test(output)) {
      die(1, `FAIL ${name.toUpperCase()} Keychain: Muse Code touched the Keychain, stopped: ${clip(output, 2000)}`);
    }
    return { reqs: requests(mock.log), result };
  } finally {
    mock.stop();
  }
}

// Fails unless the Muse Code run exited 0; a run killed at the timeout has a null code.
function museExited(result) {
  const how = result.code === null ? "was killed (timeout or signal)" : `exited ${result.code}`;
  assert(result.code === 0, `muse exec ${how}: ${clip(result.stdout + result.stderr)}`);
}

// A scripted step: answered once, to the first request that holds every `when` text and is not a
// goal reminder.
const step = (when, name, args) => ({ when, unless: [GOAL_REMINDER], name, args });

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
  if (out.some(hasAirsl)) throw new Failure("airsl is still resolvable on the S6 PATH");
  return out.join(path.delimiter);
}

// ---- scenarios ------------------------------------------------------------------------------

// S1: init, plan, doctor and update on the check pack.
async function s1() {
  const project = await initProject("s1");
  const plan = await agentstacks(project, ["plan"]);
  assert(plan.code === 0, `plan exited ${plan.code}: ${clip(plan.stdout + plan.stderr)}`);
  includes(plan.stdout, "0 to write", "plan output");
  for (const [out, what] of [[project.init.stdout, "init output"], [plan.stdout, "plan output"]]) {
    includes(out, `skip     check/hook/5: ${SKIP_H2}`, what);
    includes(out, `skip     check/hook/6: ${SKIP_H3}`, what);
  }
  const doctor = await agentstacks(project, ["doctor"]);
  assert(doctor.code === 0, `doctor exited ${doctor.code}: ${clip(doctor.stdout + doctor.stderr)}`);
  includes(doctor.stdout, "healthy", "doctor output");
  const update = await agentstacks(project, ["update"]);
  assert(update.code === 0, `update exited ${update.code}: ${clip(update.stdout + update.stderr)}`);
  includes(update.stdout, "nothing to update", "update output");
}

// S2: skills reach the model: the catalog lists them and read_skill returns a body.
async function s2() {
  const { proj } = await initProject("s2");
  const steps = [step(["RUN-S2"], "read_skill", { name: "check-hello" })];
  const { reqs, result } = await withMuse("s2", steps, proj, "RUN-S2");
  museExited(result);
  assert(reqs.length > 0, "the mock received no request");
  const first = flat(reqs[0]);
  assert(
    /<skill id="check-hello"[^>]*path="\.agents\/skills\/check-hello\/SKILL\.md"[^>]*>/.test(first),
    `the first request does not list check-hello with its path: ${clip(first)}`,
  );
  assert(
    !/<skill id="check-hello"[^>]*model-invocable="false"/.test(first),
    "check-hello is marked model-invocable=false",
  );
  assert(
    /<skill id="check-ask"[^>]*model-invocable="false"/.test(first),
    `the first request does not list check-ask as model-invocable="false": ${clip(first)}`,
  );
  const call = callOf(reqs, "read_skill", (a) => a.name === "check-hello");
  includes(call.output, "HELLO-BODY", "the read_skill result");
  includes(
    call.output,
    "The note for this skill is at .agents/skills/check-hello/references/note.md.",
    "the read_skill result",
  );
  const note = path.join(proj, ".agents", "skills", "check-hello", "references", "note.md");
  assert(fs.existsSync(note), `the skill's reference file was not written: ${note}`);
  includes(fs.readFileSync(note, "utf8"), "HELLO-NOTE", "the skill's reference file");
}

// S3: an agent is in the catalog and its child gets the body and the mapped tools only.
async function s3() {
  const { proj } = await initProject("s3");
  const steps = [
    step(["RUN-S3"], "subagent_spawn", {
      command_id: "sp00x",
      role: "check",
      objective: "RUN-S3-CHILD",
      subagent_type: "check-helper",
    }),
  ];
  const { reqs, result } = await withMuse("s3", steps, proj, "RUN-S3");
  museExited(result);
  assert(reqs.length > 0, "the mock received no request");
  const first = flat(reqs[0]);
  assert(
    /^- check-helper$/m.test(first),
    `the first request does not list check-helper in the agent catalog: ${clip(first)}`,
  );
  const call = callOf(reqs, "subagent_spawn", (a) => a.subagent_type === "check-helper");
  assert(call.output.includes('"status":"accepted"'), `the spawn was not accepted: ${clip(call.output)}`);
  const child = reqs.find((r) => flat(r).includes("HELPER-BODY"));
  assert(child, "no child request holds HELPER-BODY");
  const tools = toolNames(child);
  for (const tool of ["read_file", "search"]) {
    assert(tools.includes(tool), `the child does not offer ${tool}: ${tools.join(",")}`);
  }
  for (const tool of ["write_file", "edit_file", "bash"]) {
    assert(!tools.includes(tool), `the child offers ${tool}: ${tools.join(",")}`);
  }
}

// The steps of S4 and S6: write blocked.txt, then (S4 only) write ok.txt, read it and run pwd. Each
// step waits for the previous call's result by a text that result carries.
const writeBlocked = (token) => step([token], "write_file", { path: "blocked.txt", content: "blocked\n" });

function hookSteps(token) {
  return [
    writeBlocked(token),
    step(["tool blocked by hook"], "write_file", { path: "ok.txt", content: "ok\n" }),
    step(["CHECK-BEFORE"], "read_file", { path: "ok.txt", offset: null, limit: null }),
    step(["CHECK-AFTER"], "bash", { command: "pwd", description: "S4PWD" }),
  ];
}

// S4: hooks reach the model, block a tool and see the project's real path.
async function s4() {
  const { proj } = await initProject("s4");
  const { reqs, result } = await withMuse("s4", hookSteps("RUN-S4"), proj, "RUN-S4");
  museExited(result);
  assert(reqs.length > 0, "the mock received no request");
  const first = flat(reqs[0]);
  for (const text of ["CHECK-WELCOME", "CHECK-START startup", "CHECK-PROMPT"]) {
    includes(first, text, "the first request");
  }
  assert(!fs.existsSync(path.join(proj, "blocked.txt")), "blocked.txt was written");
  const blocked = callOf(reqs, "write_file", (a) => a.path === "blocked.txt");
  includes(blocked.output, `tool blocked by hook: CHECK-BLOCK ${proj}/blocked.txt`, "the blocked write's output");
  assert(fs.existsSync(path.join(proj, "ok.txt")), "ok.txt was not written");
  const written = reqs.findIndex((r) => flat(r).includes(`CHECK-BEFORE ${proj}/ok.txt`));
  assert(written >= 0, `no request holds CHECK-BEFORE ${proj}/ok.txt`);
  assert(
    reqs.slice(written).some((r) => flat(r).includes(`CHECK-AFTER ${proj}/ok.txt`)),
    `no request after the ok.txt write holds CHECK-AFTER ${proj}/ok.txt`,
  );
  const pwd = callOf(reqs, "bash", (a) => a.command === "pwd");
  // The bash result is a JSON record whose `output` field holds what the command printed.
  let printed;
  try {
    printed = JSON.parse(pwd.output).output;
  } catch {
    printed = undefined;
  }
  assert(
    typeof printed === "string" && printed.trim() === proj,
    `the pwd output is not ${proj}: ${clip(pwd.output)}`,
  );
}

// S5: a user's SessionStart group in .muse/hooks.json survives init and both hooks reach the model.
async function s5() {
  const userGroup = {
    hooks: [
      {
        type: "command",
        command:
          "printf '%s\\n' '{\"hookSpecificOutput\":{\"additionalContext\":\"USER-CTX\",\"hookEventName\":\"SessionStart\"}}'",
      },
    ],
  };
  const { proj } = await initProject("s5", (dir) => {
    fs.mkdirSync(path.join(dir, ".muse"), { recursive: true });
    fs.writeFileSync(path.join(dir, ".muse", "hooks.json"), JSON.stringify({ hooks: { SessionStart: [userGroup] } }));
  });
  const file = JSON.parse(fs.readFileSync(path.join(proj, ".muse", "hooks.json"), "utf8"));
  const groups = file.hooks?.SessionStart ?? [];
  assert(groups.length > 1, `SessionStart holds ${groups.length} group(s), expected the user's and ours: ${clip(file)}`);
  assert(
    isDeepStrictEqual(groups[0], userGroup),
    `the first SessionStart group is not the user's: ${clip(groups[0])}`,
  );
  assert(
    groups.slice(1).every((g) => g.hooks.every((h) => h.command.endsWith("# agentstacks"))),
    `a group after the user's is not ours: ${clip(groups.slice(1))}`,
  );
  const { reqs, result } = await withMuse("s5", [], proj, "RUN-S5");
  museExited(result);
  assert(reqs.length > 0, "the mock received no request");
  const first = flat(reqs[0]);
  includes(first, "USER-CTX", "the first request");
  includes(first, "CHECK-START startup", "the first request");
}

// S6: with airsl missing from PATH the hooks are off and Muse Code writes the file the hook would
// have blocked.
async function s6() {
  const { proj } = await initProject("s6");
  const pathValue = pathWithoutAirsl(path.join(SCRATCH, "bin-s6"));
  const { result } = await withMuse("s6", [writeBlocked("RUN-S6")], proj, "RUN-S6", { pathValue });
  museExited(result);
  assert(fs.existsSync(path.join(proj, "blocked.txt")), "blocked.txt was not written");
}

// ---- main -----------------------------------------------------------------------------------

const scenarios = [["S1", s1], ["S2", s2], ["S3", s3], ["S4", s4], ["S5", s5], ["S6", s6]];
let failed = false;
for (const [name, fn] of scenarios) {
  try {
    await fn();
    console.log(`ok ${name}`);
  } catch (err) {
    failed = true;
    console.log(`FAIL ${name} ${err instanceof Failure ? err.message : (err?.stack ?? err)}`);
  }
}
cleanup();
process.exit(failed ? 1 : 0);
