# Probe results: SKILLS (OpenCode v2.4.1)

Paths: `$S` = `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad` (scratchpad). `<chain>` = `/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension`.

All runs: `sh <chain>/probes/harness/run.sh <probe-dir> <port> "<prompt>" [flags]` (OpenCode v2.4.1, isolated HOME/XDG, `--standalone --auto --format json`, mock model). Every run below printed `exit=0` as the last line of its `out.txt`.

Extraction helper (`$S/skills-x.js`, written for these probes; prints the skill block of the system prompt, then every non-system message of the last tools-bearing request in `requests.jsonl`; with `schema`, the `skill` tool definition):

```$S/skills-x.js
// usage: node skills-x.js <probe-dir> [schema]
// Prints, from the last tools-bearing request: the skill guidance block of the system prompt,
// the user message, and every tool result; with "schema", the skill tool definition.
const fs = require("fs");
const dir = process.argv[2];
const L = fs.readFileSync(dir + "/requests.jsonl", "utf8").trim().split("\n").map(JSON.parse);
const reqs = L.filter((r) => (r.body.tools || []).length > 0);
const last = reqs[reqs.length - 1].body;
const text = (c) => (typeof c === "string" ? c : JSON.stringify(c));
if (process.argv[3] === "schema") {
  console.log(JSON.stringify(last.tools.find((t) => t.function.name === "skill"), null, 1));
  process.exit(0);
}
const sys = last.messages.filter((m) => m.role === "system").map((m) => text(m.content)).join("\n");
const a = sys.indexOf("Skills provide");
const e1 = sys.indexOf("</available_skills>", a);
const e2 = sys.indexOf("No skills are currently available.", a);
const end = e1 >= 0 ? e1 + "</available_skills>".length : e2 >= 0 ? e2 + 34 : a + 600;
console.log("--- system prompt, skill block (index " + a + ")");
console.log(a >= 0 ? sys.slice(a, end) : "(no 'Skills provide' in system prompt)");
for (const m of last.messages.filter((m) => m.role !== "system")) {
  console.log("--- " + m.role + (m.tool_calls ? " tool_calls=" + JSON.stringify(m.tool_calls.map((c) => c.function)) : ""));
  if (m.content) console.log(text(m.content));
}
```

Setup script that created every probe project below (`sh $S/skills-setup.sh`):

```$S/skills-setup.sh
#!/bin/sh
# Creates the skills-NN probe projects.
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad
# P1/P7
D=$S/skills-01/proj/.opencode/skills/core-status; mkdir -p $D/references $D/scripts
cat > $D/SKILL.md <<'EOF'
---
name: Core Status Display
description: Show the project status board.
---

# Status

Run with: $ARGUMENTS

Read `references/x.md` first, then run `scripts/y.lua`.
EOF
echo "reference body X" > $D/references/x.md
echo 'print("y")' > $D/scripts/y.lua
cat > $S/skills-01/scenario.json <<EOF
[{"tool":"skill","args":{"id":"core-status"}},
 {"tool":"skill","args":{"id":"Core Status Display"}},
 {"tool":"read","args":{"path":"$S/skills-01/proj/.opencode/skills/core-status/references/x.md"}},
 {"text":"ok"}]
EOF
# P2
for n in hidden shown; do mkdir -p $S/skills-02/proj/.opencode/skills/$n; done
cat > $S/skills-02/proj/.opencode/skills/hidden/SKILL.md <<'EOF'
---
description: Hidden skill, user only.
disable-model-invocation: true
---

Hidden body.
EOF
cat > $S/skills-02/proj/.opencode/skills/shown/SKILL.md <<'EOF'
---
description: Shown skill.
---

Shown body.
EOF
echo '[{"tool":"skill","args":{"id":"hidden"}},{"text":"ok"}]' > $S/skills-02/scenario.json
# P3
B=$S/skills-03/proj/.opencode/skills
for n in unknown-keys no-desc bad-yaml quoted flow-extra flow-desc; do mkdir -p $B/$n; done
cat > $B/unknown-keys/SKILL.md <<'EOF'
---
description: Unknown keys skill.
argument-hint: "<x>"
foo: 1
---

Unknown keys body.
EOF
cat > $B/no-desc/SKILL.md <<'EOF'
---
argument-hint: "<x>"
---

No desc body.
EOF
cat > $B/bad-yaml/SKILL.md <<'EOF'
---
description: [unclosed
foo: : :
---

Bad yaml body.
EOF
cat > $B/quoted/SKILL.md <<'EOF'
---
description: "a \"b\" c"
---

Quoted body.
EOF
cat > $B/flow-extra/SKILL.md <<'EOF'
---
description: Flow extra skill.
allowed-tools: ["read", "grep"]
---

Flow extra body.
EOF
cat > $B/flow-desc/SKILL.md <<'EOF'
---
description: ["a", "b"]
---

Flow desc body.
EOF
echo '[{"tool":"skill","args":{"id":"unknown-keys"}},{"tool":"skill","args":{"id":"no-desc"}},{"tool":"skill","args":{"id":"bad-yaml"}},{"tool":"skill","args":{"id":"quoted"}},{"tool":"skill","args":{"id":"flow-extra"}},{"tool":"skill","args":{"id":"flow-desc"}},{"text":"ok"}]' > $S/skills-03/scenario.json
# P4
B=$S/skills-04/proj/.opencode/skills
for n in core-status Upper-Case snake_case; do
  mkdir -p $B/$n
  printf -- "---\ndescription: Skill %s.\n---\n\nBody of %s.\n" $n $n > $B/$n/SKILL.md
done
echo '[{"tool":"skill","args":{"id":"Upper-Case"}},{"tool":"skill","args":{"id":"upper-case"}},{"tool":"skill","args":{"id":"snake_case"}},{"text":"ok"}]' > $S/skills-04/scenario.json
# P5 (default) / P5b (OPENCODE_DISABLE_CLAUDE_CODE=1)
for p in skills-05 skills-06; do
  mkdir -p $S/$p/proj/.opencode/skills/dup $S/$p/proj/.claude/skills/dup $S/$p/proj/.claude/skills/claude-only
  printf -- "---\ndescription: dup from opencode.\n---\n\nOpenCode dup body.\n" > $S/$p/proj/.opencode/skills/dup/SKILL.md
  printf -- "---\ndescription: dup from claude.\n---\n\nClaude dup body.\n" > $S/$p/proj/.claude/skills/dup/SKILL.md
  printf -- "---\ndescription: claude only.\n---\n\nClaude only body.\n" > $S/$p/proj/.claude/skills/claude-only/SKILL.md
  echo '[{"tool":"skill","args":{"id":"dup"}},{"tool":"skill","args":{"id":"claude-only"}},{"text":"ok"}]' > $S/$p/scenario.json
done
# P6: "/x args" (07) and "@x args" (08)
for p in skills-07 skills-08; do
  mkdir -p $S/$p/proj/.opencode/skills/x
  printf -- "---\ndescription: Skill x.\ndisable-model-invocation: true\n---\n\nX body. Args: \$ARGUMENTS\n" > $S/$p/proj/.opencode/skills/x/SKILL.md
  echo '[{"text":"ok"}]' > $S/$p/scenario.json
done
```

Runner (`sh $S/skills-run.sh`, each probe run once, concurrently, one port each):

```$S/skills-run.sh
#!/bin/sh
# Runs the skills-NN probes once each, concurrently, one port each.
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad
R=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh
sh $R $S/skills-01 18710 "load the status skill" &
sh $R $S/skills-02 18711 "load the hidden skill" &
OPENCODE_LOG_LEVEL=DEBUG sh $R $S/skills-03 18712 "load the skills" --print-logs &
sh $R $S/skills-04 18713 "load the skills" &
sh $R $S/skills-05 18714 "load dup" &
OPENCODE_DISABLE_CLAUDE_CODE=1 sh $R $S/skills-06 18715 "load dup" &
sh $R $S/skills-07 18716 "/x some args" &
sh $R $S/skills-08 18717 "@x some args" &
wait
```

## P1: Is `.opencode/skills/<id>/SKILL.md` discovered, what does the model see, what does a `skill` call return, is the id the dir name?

### Setup

```$S/skills-01/proj/.opencode/skills/core-status/references/x.md
reference body X
```

```$S/skills-01/proj/.opencode/skills/core-status/scripts/y.lua
print("y")
```

```$S/skills-01/proj/.opencode/skills/core-status/SKILL.md
---
name: Core Status Display
description: Show the project status board.
---

# Status

Run with: $ARGUMENTS

Read `references/x.md` first, then run `scripts/y.lua`.
```

```$S/skills-01/scenario.json
[{"tool":"skill","args":{"id":"core-status"}},
 {"tool":"skill","args":{"id":"Core Status Display"}},
 {"tool":"read","args":{"path":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-01/proj/.opencode/skills/core-status/references/x.md"}},
 {"text":"ok"}]
```

### Command

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh $S/skills-01 18710 "load the status skill"
```

### Output

```sh
$ node skills-x.js skills-01 schema
```

```text
{
 "type": "function",
 "function": {
  "name": "skill",
  "description": "Load a specialized skill's instructions and resources into the current conversation when the task at hand matches its description.\n\nThe skill ID must match an available skill or a skill explicitly referenced by the user.",
  "parameters": {
   "type": "object",
   "properties": {
    "id": {
     "type": "string",
     "description": "The ID of an available skill or a skill explicitly referenced by the user"
    }
   },
   "required": [
    "id"
   ],
   "additionalProperties": false
  },
  "strict": false
 }
}
```

```sh
$ node skills-x.js skills-01
```

```text
--- system prompt, skill block (index 10189)
Skills provide specialized instructions and workflows for specific tasks.
Use the skill tool to load a skill when a task matches its description.
The user may also invoke a skill directly. When that happens, its instructions appear in the conversation as a <skill_content> block, the same shape the skill tool returns. A skill that is already present this way does not need to be invoked again.
<available_skills>
  <skill>
    <id>core-status</id>
    <name>Core Status Display</name>
    <description>Show the project status board.</description>
  </skill>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
</available_skills>
--- user
load the status skill
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"core-status\"}"}]
--- tool
<skill_content name="Core Status Display">
# Skill: Core Status Display

# Status

Run with: $ARGUMENTS

Read `references/x.md` first, then run `scripts/y.lua`.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-01/proj/.opencode/skills/core-status
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
<file>/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-01/proj/.opencode/skills/core-status/references/x.md</file>
<file>/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-01/proj/.opencode/skills/core-status/scripts/y.lua</file>
</skill_files>
</skill_content>
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"Core Status Display\"}"}]
--- tool
{"error":{"type":"tool.execution","message":"Unable to load skill Core Status Display"},"content":[]}
--- assistant tool_calls=[{"name":"read","arguments":"{\"path\":\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-01/proj/.opencode/skills/core-status/references/x.md\"}"}]
--- tool
Read file /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-01/proj/.opencode/skills/core-status/references/x.md, lines 1-1
1: reference body X
```

Source: the listing text is `probes/src/packages/core/src/skill/instructions.ts:25-33` (system prompt `<available_skills>`, id/name/description only); the tool result is built by `probes/src/packages/core/src/skill.ts:40-57`, with the file sample at `probes/src/packages/core/src/skill.ts:76-89` (recursive scan of the skill dir, dotfiles included, SKILL.md excluded, sorted, first 10). The id is `path.basename(path.dirname(filepath))` and `name` defaults to the id: `probes/src/packages/core/src/config/plugin/skill-file.ts:44-55`. The skill tool looks up by id only: `probes/src/packages/core/src/tool/plugin/skill.ts:47-48`.

### Answer

Discovered. The model sees it in the system prompt as `<skill><id>core-status</id><name>Core Status Display</name><description>…</description></skill>` (the `skill` tool's own description lists nothing). A `skill` call returns `<skill_content name="<name>">`, `# Skill: <name>`, the body with frontmatter stripped and `$ARGUMENTS` left verbatim, `Base directory for this skill: <abs dir>`, and `<skill_files>` with absolute paths of up to 10 sibling files. The id is the directory name; frontmatter `name` is only the display label, and calling the skill by its `name` fails with `Unable to load skill Core Status Display`.

## P2: `disable-model-invocation: true`: hidden from the listing? Is a model `skill` call refused?

### Setup

```$S/skills-02/proj/.opencode/skills/hidden/SKILL.md
---
description: Hidden skill, user only.
disable-model-invocation: true
---

Hidden body.
```

```$S/skills-02/proj/.opencode/skills/shown/SKILL.md
---
description: Shown skill.
---

Shown body.
```

```$S/skills-02/scenario.json
[{"tool":"skill","args":{"id":"hidden"}},{"text":"ok"}]
```

### Command

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh $S/skills-02 18711 "load the hidden skill"
```

### Output

```sh
$ node skills-x.js skills-02
```

```text
--- system prompt, skill block (index 10189)
Skills provide specialized instructions and workflows for specific tasks.
Use the skill tool to load a skill when a task matches its description.
The user may also invoke a skill directly. When that happens, its instructions appear in the conversation as a <skill_content> block, the same shape the skill tool returns. A skill that is already present this way does not need to be invoked again.
<available_skills>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
  <skill>
    <id>shown</id>
    <name>shown</name>
    <description>Shown skill.</description>
  </skill>
</available_skills>
--- user
load the hidden skill
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"hidden\"}"}]
--- tool
<skill_content name="hidden">
# Skill: hidden

Hidden body.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-02/proj/.opencode/skills/hidden
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
```

Source: hidden from the listing when `autoinvoke === false` or `description` is missing, `probes/src/packages/core/src/skill/instructions.ts:62-66`; `disable-model-invocation` maps to `autoinvoke: false` unless `metadata.opencode/autoinvoke` is set, `probes/src/packages/core/src/config/plugin/skill-file.ts:48-50`; the tool checks only the `skill` permission, not `autoinvoke`, `probes/src/packages/core/src/tool/plugin/skill.ts:47-57`. Docs: `probes/docs/skills.mdx:145` ("only hides the skill from the model's available list … can still be loaded explicitly by ID").

### Answer

Hidden from `<available_skills>` (only `shown` and the built-ins are listed), but **not refused**: a model `skill` call with `id: hidden` returns the full `<skill_content>` body. `disable-model-invocation` is a listing filter, not an enforcement; the only enforcement is a `permissions` rule with `action: skill` and `effect: deny` (`probes/docs/skills.mdx:208-226`, not probed here).

## P3: Frontmatter edge cases: unknown keys, missing description, invalid YAML, quoted JSON-style scalar, JSON flow list

### Setup

```$S/skills-03/proj/.opencode/skills/bad-yaml/SKILL.md
---
description: [unclosed
foo: : :
---

Bad yaml body.
```

```$S/skills-03/proj/.opencode/skills/flow-desc/SKILL.md
---
description: ["a", "b"]
---

Flow desc body.
```

```$S/skills-03/proj/.opencode/skills/flow-extra/SKILL.md
---
description: Flow extra skill.
allowed-tools: ["read", "grep"]
---

Flow extra body.
```

```$S/skills-03/proj/.opencode/skills/no-desc/SKILL.md
---
argument-hint: "<x>"
---

No desc body.
```

```$S/skills-03/proj/.opencode/skills/quoted/SKILL.md
---
description: "a \"b\" c"
---

Quoted body.
```

```$S/skills-03/proj/.opencode/skills/unknown-keys/SKILL.md
---
description: Unknown keys skill.
argument-hint: "<x>"
foo: 1
---

Unknown keys body.
```

```$S/skills-03/scenario.json
[{"tool":"skill","args":{"id":"unknown-keys"}},{"tool":"skill","args":{"id":"no-desc"}},{"tool":"skill","args":{"id":"bad-yaml"}},{"tool":"skill","args":{"id":"quoted"}},{"tool":"skill","args":{"id":"flow-extra"}},{"tool":"skill","args":{"id":"flow-desc"}},{"text":"ok"}]
```

### Command

```sh
OPENCODE_LOG_LEVEL=DEBUG sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh $S/skills-03 18712 "load the skills" --print-logs
```

### Output

```sh
$ node skills-x.js skills-03
```

```text
--- system prompt, skill block (index 10189)
Skills provide specialized instructions and workflows for specific tasks.
Use the skill tool to load a skill when a task matches its description.
The user may also invoke a skill directly. When that happens, its instructions appear in the conversation as a <skill_content> block, the same shape the skill tool returns. A skill that is already present this way does not need to be invoked again.
<available_skills>
  <skill>
    <id>flow-extra</id>
    <name>flow-extra</name>
    <description>Flow extra skill.</description>
  </skill>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>quoted</id>
    <name>quoted</name>
    <description>a "b" c</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
  <skill>
    <id>unknown-keys</id>
    <name>unknown-keys</name>
    <description>Unknown keys skill.</description>
  </skill>
</available_skills>
--- user
load the skills
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"unknown-keys\"}"}]
--- tool
<skill_content name="unknown-keys">
# Skill: unknown-keys

Unknown keys body.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/proj/.opencode/skills/unknown-keys
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"no-desc\"}"}]
--- tool
<skill_content name="no-desc">
# Skill: no-desc

No desc body.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/proj/.opencode/skills/no-desc
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"bad-yaml\"}"}]
--- tool
{"error":{"type":"tool.execution","message":"Unable to load skill bad-yaml"},"content":[]}
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"quoted\"}"}]
--- tool
<skill_content name="quoted">
# Skill: quoted

Quoted body.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/proj/.opencode/skills/quoted
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"flow-extra\"}"}]
--- tool
<skill_content name="flow-extra">
# Skill: flow-extra

Flow extra body.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/proj/.opencode/skills/flow-extra
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"flow-desc\"}"}]
--- tool
{"error":{"type":"tool.execution","message":"Unable to load skill flow-desc"},"content":[]}
```

```sh
$ grep -n 'skill file skipped\|skill source loaded' skills-03/err.txt
```

```text
42:timestamp=2026-11-24T16:31:42.390Z level=DEBUG run=3a684e4f message="skill source loaded" source=directory:/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/home/.config/opencode/skill type=directory directories="[\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/home/.config/opencode/skill\"]" skills=[]
45:timestamp=2026-11-24T16:31:42.391Z level=DEBUG run=3a684e4f message="skill source loaded" source=directory:/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/home/.config/opencode/skills type=directory directories="[\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/home/.config/opencode/skills\"]" skills=[]
48:timestamp=2026-11-24T16:31:42.394Z level=DEBUG run=3a684e4f message="skill source loaded" source=directory:/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/proj/.opencode/skill type=directory directories="[\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/proj/.opencode/skill\"]" skills=[]
51:timestamp=2026-11-24T16:31:42.406Z level=DEBUG run=3a684e4f message="skill file skipped" filepath=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/proj/.opencode/skills/bad-yaml/SKILL.md reason=markdown
52:timestamp=2026-11-24T16:31:42.412Z level=DEBUG run=3a684e4f message="skill file skipped" filepath=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/proj/.opencode/skills/flow-desc/SKILL.md reason=frontmatter issue="Pc({\"~effect/SchemaIssue/Issue\":\"~effect/SchemaIssue/Issue\",\"_tag\":\"Composite\",\"ast\":<Objects>,\"issues\":[gp({\"~effect/SchemaIssue/Issue\":\"~effect/SchemaIssue/Issue\",\"_tag\":\"Pointer\",\"path\":[\"description\"],\"issue\":Ee({\"~effect/SchemaIssue/Issue\":\"~effect/SchemaIssue/Issue\",\"_tag\":\"AnyOf\",\"ast\":<Union>,\"issues\":[]})})]})"
53:timestamp=2026-11-24T16:31:42.418Z level=DEBUG run=3a684e4f message="skill source loaded" source=directory:/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/proj/.opencode/skills type=directory directories="[\"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-03/proj/.opencode/skills\"]" skills="[\"flow-extra\",\"no-desc\",\"quoted\",\"unknown-keys\"]"
```

```sh
$ grep -c 'level=WARN\|level=ERROR' skills-03/err.txt; for i in 01 02 04 05 06 07 08; do printf '%s ' skills-$i; wc -c < skills-$i/err.txt; done
```

```text
0
skills-01        0
skills-02        0
skills-04        0
skills-05        0
skills-06        0
skills-07        0
skills-08        0
```

Source: the frontmatter schema has only `name`, `description` (both `Schema.String`, optional), `metadata`, `disable-model-invocation`, and `Schema.Struct` ignores other keys, `probes/src/packages/core/src/config/plugin/skill-file.ts:10-15`; unparsable markdown is `Skipped reason=markdown`, a schema failure is `Skipped reason=frontmatter`, `probes/src/packages/core/src/config/plugin/skill-file.ts:39-42`; both are logged only at `logDebug` and dropped, `probes/src/packages/core/src/config/plugin/skill.ts:129-136`.

### Answer

Unknown keys (`argument-hint: "<x>"`, `foo: 1`, `allowed-tools: ["read", "grep"]`) are ignored and the skill loads. A missing `description` loads the skill (the model can call it by id) but it is not listed. A quoted JSON-style scalar `"a \"b\" c"` parses to `a "b" c`. Invalid YAML (`reason=markdown`) and a non-string `description` (flow list, `reason=frontmatter`) silently drop the skill: no warning or error at the default level (the err.txt of every run without `--print-logs` is 0 bytes); the reason appears only as a DEBUG log line, and the model's call gets `Unable to load skill <id>`.

## P4: Skill dir ids with a dash, uppercase, underscore

### Setup

```$S/skills-04/proj/.opencode/skills/core-status/SKILL.md
---
description: Skill core-status.
---

Body of core-status.
```

```$S/skills-04/proj/.opencode/skills/snake_case/SKILL.md
---
description: Skill snake_case.
---

Body of snake_case.
```

```$S/skills-04/proj/.opencode/skills/Upper-Case/SKILL.md
---
description: Skill Upper-Case.
---

Body of Upper-Case.
```

```$S/skills-04/scenario.json
[{"tool":"skill","args":{"id":"Upper-Case"}},{"tool":"skill","args":{"id":"upper-case"}},{"tool":"skill","args":{"id":"snake_case"}},{"text":"ok"}]
```

### Command

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh $S/skills-04 18713 "load the skills"
```

### Output

```sh
$ node skills-x.js skills-04
```

```text
--- system prompt, skill block (index 10189)
Skills provide specialized instructions and workflows for specific tasks.
Use the skill tool to load a skill when a task matches its description.
The user may also invoke a skill directly. When that happens, its instructions appear in the conversation as a <skill_content> block, the same shape the skill tool returns. A skill that is already present this way does not need to be invoked again.
<available_skills>
  <skill>
    <id>core-status</id>
    <name>core-status</name>
    <description>Skill core-status.</description>
  </skill>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
  <skill>
    <id>snake_case</id>
    <name>snake_case</name>
    <description>Skill snake_case.</description>
  </skill>
  <skill>
    <id>Upper-Case</id>
    <name>Upper-Case</name>
    <description>Skill Upper-Case.</description>
  </skill>
</available_skills>
--- user
load the skills
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"Upper-Case\"}"}]
--- tool
<skill_content name="Upper-Case">
# Skill: Upper-Case

Body of Upper-Case.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-04/proj/.opencode/skills/Upper-Case
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"upper-case\"}"}]
--- tool
{"error":{"type":"tool.execution","message":"Unable to load skill upper-case"},"content":[]}
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"snake_case\"}"}]
--- tool
<skill_content name="snake_case">
# Skill: snake_case

Body of snake_case.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-04/proj/.opencode/skills/snake_case
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
```

Source: no id validation in the parser, `probes/src/packages/core/src/config/plugin/skill-file.ts:44-47`; docs: `probes/docs/skills.mdx:157-163` ("IDs are exact and case-sensitive … V2 currently does not enforce this pattern").

### Answer

All accepted: `core-status`, `Upper-Case` and `snake_case` are listed and load. Ids are case-sensitive: `upper-case` fails with `Unable to load skill upper-case`.

## P5: Duplicate id in `.opencode/skills/` and `.claude/skills/`, with and without `OPENCODE_DISABLE_CLAUDE_CODE=1`

### Setup

```$S/skills-05/proj/.claude/skills/claude-only/SKILL.md
---
description: claude only.
---

Claude only body.
```

```$S/skills-05/proj/.claude/skills/dup/SKILL.md
---
description: dup from claude.
---

Claude dup body.
```

```$S/skills-05/proj/.opencode/skills/dup/SKILL.md
---
description: dup from opencode.
---

OpenCode dup body.
```

```$S/skills-05/scenario.json
[{"tool":"skill","args":{"id":"dup"}},{"tool":"skill","args":{"id":"claude-only"}},{"text":"ok"}]
```

`skills-06` is identical (same files under `$S/skills-06/`, created by the same loop in `skills-setup.sh`).

### Command

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh $S/skills-05 18714 "load dup"
OPENCODE_DISABLE_CLAUDE_CODE=1 sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh $S/skills-06 18715 "load dup"
```

### Output

```sh
$ node skills-x.js skills-05
```

```text
--- system prompt, skill block (index 10189)
Skills provide specialized instructions and workflows for specific tasks.
Use the skill tool to load a skill when a task matches its description.
The user may also invoke a skill directly. When that happens, its instructions appear in the conversation as a <skill_content> block, the same shape the skill tool returns. A skill that is already present this way does not need to be invoked again.
<available_skills>
  <skill>
    <id>claude-only</id>
    <name>claude-only</name>
    <description>claude only.</description>
  </skill>
  <skill>
    <id>dup</id>
    <name>dup</name>
    <description>dup from opencode.</description>
  </skill>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
</available_skills>
--- user
load dup
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"dup\"}"}]
--- tool
<skill_content name="dup">
# Skill: dup

OpenCode dup body.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-05/proj/.opencode/skills/dup
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"claude-only\"}"}]
--- tool
<skill_content name="claude-only">
# Skill: claude-only

Claude only body.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-05/proj/.claude/skills/claude-only
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
```

```sh
$ node skills-x.js skills-06
```

```text
--- system prompt, skill block (index 10189)
Skills provide specialized instructions and workflows for specific tasks.
Use the skill tool to load a skill when a task matches its description.
The user may also invoke a skill directly. When that happens, its instructions appear in the conversation as a <skill_content> block, the same shape the skill tool returns. A skill that is already present this way does not need to be invoked again.
<available_skills>
  <skill>
    <id>dup</id>
    <name>dup</name>
    <description>dup from opencode.</description>
  </skill>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
</available_skills>
--- user
load dup
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"dup\"}"}]
--- tool
<skill_content name="dup">
# Skill: dup

OpenCode dup body.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-06/proj/.opencode/skills/dup
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
--- assistant tool_calls=[{"name":"skill","arguments":"{\"id\":\"claude-only\"}"}]
--- tool
{"error":{"type":"tool.execution","message":"Unable to load skill claude-only"},"content":[]}
```

Source: `.claude/skills` and `.agents/skills` are loaded by `ConfigCompatibilityPlugin` (`probes/src/packages/core/src/config/plugin/compatibility.ts:45,84`), `.opencode/skill` and `.opencode/skills` by `ConfigSkillPlugin` (`probes/src/packages/core/src/config/plugin/skill.ts:88-91`); the plugin list registers compatibility first, then config skills (`probes/src/packages/core/src/plugin/internal.ts:326-327`), and the skill editor is a `Map` keyed by id where a later `add` replaces the earlier one (`probes/src/packages/core/src/skill.ts:128-129`). `OPENCODE_DISABLE_CLAUDE_CODE` (or `OPENCODE_DISABLE_CLAUDE_CODE_SKILLS`) turns off `.claude` skill reading: `probes/src/packages/cli/src/server-process.ts:127-131`. Docs: `probes/docs/skills.mdx:165-183`.

### Answer

The model sees `dup` once, and the `.opencode/skills/dup` copy wins (description and body from opencode). Skills only in `.claude/skills/` are also listed and loadable. With `OPENCODE_DISABLE_CLAUDE_CODE=1`, `.claude/skills/` is not read at all: `claude-only` disappears (`Unable to load skill claude-only`) and `dup` is the opencode copy.

## P6: Can the user invoke a skill directly (`/x args`), and is `$ARGUMENTS` substituted?

### Setup

```$S/skills-07/proj/.opencode/skills/x/SKILL.md
---
description: Skill x.
disable-model-invocation: true
---

X body. Args: $ARGUMENTS
```

```$S/skills-07/scenario.json
[{"text":"ok"}]
```

```$S/skills-08/proj/.opencode/skills/x/SKILL.md
---
description: Skill x.
disable-model-invocation: true
---

X body. Args: $ARGUMENTS
```

```$S/skills-08/scenario.json
[{"text":"ok"}]
```

### Command

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh $S/skills-07 18716 "/x some args"
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh $S/skills-08 18717 "@x some args"
```

### Output

```sh
$ node skills-x.js skills-07; grep -c 'X body' skills-07/requests.jsonl
```

```text
--- system prompt, skill block (index 10189)
Skills provide specialized instructions and workflows for specific tasks.
Use the skill tool to load a skill when a task matches its description.
The user may also invoke a skill directly. When that happens, its instructions appear in the conversation as a <skill_content> block, the same shape the skill tool returns. A skill that is already present this way does not need to be invoked again.
<available_skills>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
</available_skills>
--- user
/x some args
0
```

```sh
$ node skills-x.js skills-08; grep -c 'X body' skills-08/requests.jsonl
```

```text
--- system prompt, skill block (index 10189)
Skills provide specialized instructions and workflows for specific tasks.
Use the skill tool to load a skill when a task matches its description.
The user may also invoke a skill directly. When that happens, its instructions appear in the conversation as a <skill_content> block, the same shape the skill tool returns. A skill that is already present this way does not need to be invoked again.
<available_skills>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
</available_skills>
--- user
@x some args
0
```

`opencode run --help` (isolated: HOME/XDG under `$S/skills-07/home`, cwd `$S/skills-07/proj`), saved to `$S/skills-07/run-help.txt`:

```$S/skills-07/run-help.txt
DESCRIPTION
  Run OpenCode with a message

USAGE
  opencode run [flags] [<message...>]

ARGUMENTS
  message... string    Message to send (optional)

FLAGS
  --continue, -c            Continue the last session
  --session, -s string      Session ID to continue, or to create if it does not exist
  --fork                    Fork the session before continuing
  --worktree, -w [=name]    Start a new session in a new git worktree, optionally with this name
  --model, -m string        Model to use in the format provider/model#variant
  --agent string            Agent to use
  --format choice           Output format (choices: default, json)
  --file, -f string         File to attach to the message
  --title string            Session title
  --archived                Create the session archived so it stays out of session lists (useful for scripted or CI runs)
  --thinking                Show thinking blocks
  --auto                    Auto-approve permissions that are not explicitly denied

GLOBAL FLAGS
  --help, -h                                                          Show help information
  --version, -v                                                       Show version information
  --wizard                                                            Start wizard mode for a command
  --completions <bash|zsh|fish|powershell|sh|pwsh>                    Print shell completion script (choices: bash, zsh, fish, powershell, sh, pwsh)
  --log-level <all|trace|debug|info|warn|warning|error|fatal|none>    Sets the minimum log level (choices: all, trace, debug, info, warn, warning, error, fatal, none)
  --print-logs                                                        Print logs to stderr (server logs require --standalone)
  --standalone                                                        Run with a private server instead of the background service
  --server string                                                     Connect to a server URL instead of the background service
exit=0
```

**P6b: the user-attached skill path (HTTP API `session.prompt` with `skills`, what the TUI's `@id` pick sends).** `opencode serve` under the same isolation, mock on 18718, serve on 18719. First run (no auth) failed; that script is kept as `$S/skills-09-first.sh`, its outputs in `$S/skills-09-first/`:

```sh
$ cat skills-09-first/create.json; echo; cat skills-09-first/prompt.json; echo; cat skills-09-first/serve.out.txt
```

```text
{"_tag":"UnauthorizedError","message":"Authentication required"}
{"_tag":"UnauthorizedError","message":"Authentication required"}
server listening on http://127.0.0.1:18719
server password 73Aq2dzBgtdIMVYp9P2I_gw3K9JP1QvTSf4Jt8tWhq0
```

Second run with HTTP Basic auth (user `opencode`, `probes/src/packages/server/src/auth.ts:18-21`; password from the serve stdout). Script:

```$S/skills-09.sh
#!/bin/sh
# P6b: a user-attached skill through the HTTP API (session.prompt with `skills`), the path the
# TUI's @id autocomplete uses. Basic auth: user opencode, password printed by serve. Same isolation as the harness; mock on 18718, serve on 18719.
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad
H=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness
DIR=$S/skills-09
mkdir -p "$DIR/proj/.opencode/skills/x" "$DIR/home/.config/opencode"
printf -- "---\ndescription: Skill x.\ndisable-model-invocation: true\n---\n\nX body. Args: \$ARGUMENTS\n" > "$DIR/proj/.opencode/skills/x/SKILL.md"
echo '[{"text":"ok"}]' > "$DIR/scenario.json"
: > "$DIR/requests.jsonl"
node "$H/mock.mjs" 18718 "$DIR/scenario.json" "$DIR/requests.jsonl" &
MOCK=$!
export HOME="$DIR/home" XDG_CONFIG_HOME="$DIR/home/.config" XDG_DATA_HOME="$DIR/home/.local/share" \
  XDG_CACHE_HOME="$DIR/home/.cache" XDG_STATE_HOME="$DIR/home/.local/state" \
  OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_AUTOUPDATE=1
cat > "$HOME/.config/opencode/opencode.json" <<EOF
{ "\$schema": "https://opencode.ai/config.json",
  "model": "mock/m",
  "providers": { "mock": { "name": "Mock", "package": "@opencode/ai/providers/openai-compatible",
    "settings": { "baseURL": "http://127.0.0.1:18718/v1", "apiKey": "x" },
    "models": { "m": { "name": "M" } } } } }
EOF
cd "$DIR/proj" || exit 1
/Users/hiraq/.opencode/bin/opencode serve --hostname 127.0.0.1 --port 18719 > "$DIR/serve.out.txt" 2> "$DIR/serve.err.txt" &
SERVE=$!
sleep 4
PW=$(sed -n "s/^server password //p" "$DIR/serve.out.txt")
curl -s -u "opencode:$PW" -X POST http://127.0.0.1:18719/api/session -H 'content-type: application/json' \
  --data "{\"location\":{\"directory\":\"$DIR/proj\"}}" > "$DIR/create.json"
echo "create: $(cat "$DIR/create.json")"
SID=$(node -e 'const j=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8"));console.log(j.data.id)' "$DIR/create.json")
curl -s -u "opencode:$PW" -X POST "http://127.0.0.1:18719/api/session/$SID/prompt" -H 'content-type: application/json' \
  --data '{"text":"some args","skills":[{"id":"x"}]}' > "$DIR/prompt.json"
echo "prompt: $(cat "$DIR/prompt.json")"
sleep 6
kill $SERVE $MOCK 2>/dev/null
```

```sh
$ sh $S/skills-09.sh
```

Saved responses and the model request:

```sh
$ cat skills-09/create.json; echo; cat skills-09/prompt.json
```

```text
{"data":{"id":"ses_df1981a70fferbeNQV54LOk8FJ","projectID":"4f636fd2f832c96a921bea7e54a406b5c1fff274","cost":0,"tokens":{"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}},"time":{"created":1795538019732,"updated":1795538019732},"location":{"directory":"/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-09/proj"}}}
{"data":{"id":"msg_20e67e5ee001BBytBwaH7CYHL4","sessionID":"ses_df1981a70fferbeNQV54LOk8FJ","time":{"created":1795538020127},"type":"user","payload":{"text":"some args","skills":[{"id":"x","name":"x","text":"<skill_content name=\"x\">\n# Skill: x\n\nX body. Args: $ARGUMENTS\n\nBase directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-09/proj/.opencode/skills/x\nRelative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.\nNote: file list is sampled.\n\n<skill_files>\n</skill_files>\n</skill_content>"}]},"delivery":"steer"}}
```

```sh
$ node skills-x.js skills-09
```

```text
--- system prompt, skill block (index 10189)
Skills provide specialized instructions and workflows for specific tasks.
Use the skill tool to load a skill when a task matches its description.
The user may also invoke a skill directly. When that happens, its instructions appear in the conversation as a <skill_content> block, the same shape the skill tool returns. A skill that is already present this way does not need to be invoked again.
<available_skills>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
</available_skills>
--- user
<skill_content name="x">
# Skill: x

X body. Args: $ARGUMENTS

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-09/proj/.opencode/skills/x
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>some args
```

```sh
$ node -e 'const L=require("fs").readFileSync("skills-09/requests.jsonl","utf8").trim().split("\n").map(JSON.parse);const b=L.filter(r=>(r.body.tools||[]).length)[0].body;const u=b.messages.find(m=>m.role==="user");console.log(typeof u.content)'
```

```text
string
```

Search plus control for `$ARGUMENTS` in the v2.4.1 core (control: the command code substitutes it; no skill file mentions it):

```sh
$ cd oc-src/anomalyco-opencode-6f05140/packages/core/src && grep -rn 'ARGUMENTS' . | grep -v test
```

```text
./config/plugin/command.ts:181:      if (match === "$ARGUMENTS") return input
./config/plugin/command.ts:189:      placeholders.length === 0 && !template.includes("$ARGUMENTS") && input.trim()
./config/plugin/command.ts:227:const argumentRegex = /\$(?:ARGUMENTS|([1-9]\d*))/g
./plugin/command/review.txt:5:Input: $ARGUMENTS
./plugin/command/review.txt:19:   - Run: `git show $ARGUMENTS`
./plugin/command/review.txt:22:   - Run: `git diff $ARGUMENTS...HEAD`
./plugin/command/review.txt:25:   - Run: `gh pr view $ARGUMENTS` to get PR context
./plugin/command/review.txt:26:   - Run: `gh pr diff $ARGUMENTS` to get the diff
./plugin/command/automation.txt:3:Request: $ARGUMENTS
./plugin/command/initialize.txt:6:$ARGUMENTS
./plugin/command.ts:88:  if (template.includes("$ARGUMENTS")) return template.replaceAll("$ARGUMENTS", () => value)
./tool/plugin/automation.ts:169:              text: PROMPT_AUTOMATION.replaceAll("$ARGUMENTS", () => input.prompt.text.trim()),
```

Source: `opencode run` sends only `text` and `files` to `session.prompt`, never `skills`, and has no command flag: `probes/src/packages/cli/src/run/noninteractive.ts:217-223`. The prompt API takes `skills: [{id}]` (`probes/src/packages/schema/src/prompt-input.ts:19-30`) and prepends `Skill.prepare(...)` output, with no argument handling: `probes/src/packages/core/src/session/prompt.ts:58-75`. In the TUI, `@` offers skills (`probes/src/packages/tui/src/component/prompt/autocomplete.tsx:436-449`) while `/` lists only server commands (`probes/src/packages/tui/src/component/prompt/autocomplete.tsx:495-502`). `$ARGUMENTS` is substituted only for commands: `probes/src/packages/core/src/config/plugin/command.ts:181`, `probes/src/packages/core/src/plugin/command.ts:88`. Docs: `probes/docs/skills.mdx:204` ("To load a skill yourself, mention it in your prompt as `@skill-id`").

### Answer

No slash-command form: `opencode run "/x some args"` and `"@x some args"` reach the model as literal text and the skill is never loaded; `run` has no `--command` flag. The only user path is attaching the skill (TUI `@x` autocomplete, or API `skills: [{id}]`), which works even with `disable-model-invocation: true` and prepends the `<skill_content>` block to the user text. `$ARGUMENTS` is never substituted in a skill (it stays literal; the user's text follows the block); only OpenCode commands substitute it.

## P7: Are files beside SKILL.md usable; does the model learn the skill directory?

### Setup

Same run as P1 (`$S/skills-01`): `references/x.md` and `scripts/y.lua` beside `SKILL.md`; the third scenario step reads `references/x.md` by the absolute path from the skill result.

### Command

```sh
sh /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh $S/skills-01 18710 "load the status skill"
```

### Output

From the P1 extraction above (same command, same output): the skill result carries

```sh
$ node skills-x.js skills-01 | sed -n '/^Base directory/,/^<\/skill_content>/p'
```

```text
Base directory for this skill: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-01/proj/.opencode/skills/core-status
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
<file>/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-01/proj/.opencode/skills/core-status/references/x.md</file>
<file>/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-01/proj/.opencode/skills/core-status/scripts/y.lua</file>
</skill_files>
</skill_content>
```

```sh
$ node skills-x.js skills-01 | tail -3
```

```text
--- tool
Read file /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/skills-01/proj/.opencode/skills/core-status/references/x.md, lines 1-1
1: reference body X
```

Source: `probes/src/packages/core/src/skill.ts:40-57` and `:76-89` (up to 10 files, sorted, recursive, dotfiles included, `FSUtil.ignoreGlobs` skipped). The scan stops at a directory holding `SKILL.md`, so `references/*.md` is not taken for a skill: `probes/src/packages/core/src/config/plugin/skill-scan.ts:29-33`. Docs: `probes/docs/skills.mdx:202,206`.

### Answer

Yes. The result gives `Base directory for this skill: <absolute dir>`, says relative paths in the skill resolve against it, and lists sibling files as absolute paths (sampled: first 10 sorted). The model then read `references/x.md` with `read` and got its content. File contents are not loaded automatically.
