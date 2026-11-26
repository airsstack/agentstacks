// spawnseq.mjs <trigger> <agent-id>...: print a mock script that spawns each agent in turn from the lead (one spawn per lead turn).
// Lead turn i is recognised by containing command id sp<i-1>x and not sp<i>x; children (bodies contain "BODY of") are excluded.
const [trigger, ...ids] = process.argv.slice(2);
const pad = (i) => `sp${String(i).padStart(2, "0")}x`;
console.log(JSON.stringify(ids.map((id, k) => ({
  when: k === 0 ? [trigger] : [trigger, pad(k - 1)],
  unless: ["You are a goal reminder", "BODY of", pad(k), ...(k === 0 ? ['"call_id":"call_'] : [])],
  name: "subagent_spawn",
  args: { command_id: pad(k), role: "probe", objective: `probe ${id}`, subagent_type: id },
})), null, 1));
