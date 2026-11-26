# Muse Code item probes B1–B8 — muse 1.4.25 (muse-bin-1.4.25-R9361.1), macOS arm64, 2026-11-25

Scratch prefix `/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/itemprobe`
is printed as `$W` (a `sed` substitution, nothing else changed). Every raw block below was produced by
a script (`harness/msgs.mjs`, `ctx.mjs`, `children.mjs`, `spawnresults.mjs`) from the logged
request bodies or the command output; nothing is retyped. Lines containing `bundled` are filtered
from stderr.

Harness (copied to `harness/`):

- `items-run.sh <proj> <script.json> <logdir> <prompt> [muse args]` — fresh `HOME` + `TMPDIR` per run,
  seeds `settings.json` (`base_url http://127.0.0.1:18782`) and `auth.json`, starts `mock.mjs` on
  18782, runs `muse-bin exec --trust-workspace --yolo` from `<proj>` with `AIRSSTACK_HOME=/probe/airs`,
  kills the mock.
- `tui.sh <proj> <script.json> <logdir> <line>` — same isolation, but runs the **interactive TUI**
  (`muse-bin --trust-workspace --yolo`) in a private tmux server (tmux answers the terminal
  queries the TUI blocks on), types `<line>` + Enter, waits for `MOCKDONE`, captures the screen.
- `palette.sh <proj> <logdir> <text>` — TUI, types text without Enter, captures the `/` palette.
- `spawnseq.mjs` writes a mock script that spawns a list of agents one per lead turn.
- `msgs.mjs`, `ctx.mjs`, `children.mjs`, `spawnresults.mjs` — extractors used for every quote.

## B1 — skill invocation with arguments

Question: what reaches the model for `/core-status argz one` (body `BODY-MARK $ARGUMENTS`); same
for a `disable-model-invocation: true` skill as `/name a b`; and a `user-invocable: false` skill as
`/name`.

Commands:

```text
items-run.sh $W/b1 empty.json $W/logs/b1-status "/core-status argz one"          # headless exec
tui.sh $W/b1 empty.json $W/logs/b1t-status     "/core-status argz one"     # interactive TUI
tui.sh $W/b1 empty.json $W/logs/b1t-statusab   "/core-status a b"
tui.sh $W/b1 empty.json $W/logs/b1t-core-plain "/core-plain a b"
tui.sh $W/b1 empty.json $W/logs/b1t-noargs     "/core-noargs x y"
tui.sh $W/b1 empty.json $W/logs/b1t-hidden     "/core-hidden"
tui.sh $W/b1 empty.json $W/logs/b1t-hiddenargs "/core-hidden a b"
```

Files (`$W/b1`, a `git init` dir):

```text
--- .agents/skills/core-hidden/SKILL.md
---
name: "core-hidden"
description: "Not user invocable"
user-invocable: false
---
HIDDEN-MARK $ARGUMENTS
--- .agents/skills/core-noargs/SKILL.md
---
name: "core-noargs"
description: "Body without the ARGUMENTS placeholder"
---
NOARGS-MARK line
--- .agents/skills/core-plain/SKILL.md
---
name: "core-plain"
description: "Plain skill"
argument-hint: "[x]"
---
PLAIN-MARK $ARGUMENTS
--- .agents/skills/core-status/SKILL.md
---
name: "core-status"
description: "Show status"
disable-model-invocation: true
argument-hint: "[x]"
---
BODY-MARK $ARGUMENTS
```

Raw (user message of the first main-agent request; TUI screen):

```text
== exec: muse exec --trust-workspace --yolo '/core-status argz one'  (req-001 user message)
### b1-status/req-001.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=21: "/core-status argz one"

== TUI b1t-status screen:
  Muse Code 1.4.25
❯ /core-status argz one
◆ Loaded skill core-status · project
◆ MOCKDONE
❯
== TUI b1t-status first model request, user message:
### b1t-status/req-002.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=554: "<system-reminder source=\"skill-body\">\nMuse Code loaded the full instructions for an explicitly invoked skill. Apply these instructions only to the current user turn.\n<skill-body id=\"core-status\" scope=\"project\" path=\".agents/skills/core-status/SKILL.md\" sha256=\"sha256:c75355088240eaad05e5d862530a087891786ae73557a768f9ac6a8926f69c74\" byte-count=\"128\">\n---\nname: &quot;core-status&quot;\ndescription: &quot;Show status&quot;\ndisable-model-invocation: true\nargument-hint: &quot;[x]&quot;\n---\nBODY-MARK $ARGUMENTS\n\n</skill-body>\n</system-reminder>\n\nargz one"

== TUI b1t-statusab screen:
  Muse Code 1.4.25
❯ /core-status a b
◆ Loaded skill core-status · project
◆ MOCKDONE
❯
== TUI b1t-statusab first model request, user message:
### b1t-statusab/req-002.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=549: "<system-reminder source=\"skill-body\">\nMuse Code loaded the full instructions for an explicitly invoked skill. Apply these instructions only to the current user turn.\n<skill-body id=\"core-status\" scope=\"project\" path=\".agents/skills/core-status/SKILL.md\" sha256=\"sha256:c75355088240eaad05e5d862530a087891786ae73557a768f9ac6a8926f69c74\" byte-count=\"128\">\n---\nname: &quot;core-status&quot;\ndescription: &quot;Show status&quot;\ndisable-model-invocation: true\nargument-hint: &quot;[x]&quot;\n---\nBODY-MARK $ARGUMENTS\n\n</skill-body>\n</system-reminder>\n\na b"

== TUI b1t-core-plain screen:
  Muse Code 1.4.25
❯ /core-plain a b
◆ Loaded skill core-plain · project
◆ MOCKDONE
❯
== TUI b1t-core-plain first model request, user message:
### b1t-core-plain/req-002.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=515: "<system-reminder source=\"skill-body\">\nMuse Code loaded the full instructions for an explicitly invoked skill. Apply these instructions only to the current user turn.\n<skill-body id=\"core-plain\" scope=\"project\" path=\".agents/skills/core-plain/SKILL.md\" sha256=\"sha256:c72f8c6161c2618250588b2b3bbb4bc1697e09d21de6feda14c8beb39337d202\" byte-count=\"97\">\n---\nname: &quot;core-plain&quot;\ndescription: &quot;Plain skill&quot;\nargument-hint: &quot;[x]&quot;\n---\nPLAIN-MARK $ARGUMENTS\n\n</skill-body>\n</system-reminder>\n\na b"

== TUI b1t-noargs screen:
  Muse Code 1.4.25
❯ /core-noargs x y
◆ Loaded skill core-noargs · project
◆ MOCKDONE
❯
== TUI b1t-noargs first model request, user message:
### b1t-noargs/req-002.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=509: "<system-reminder source=\"skill-body\">\nMuse Code loaded the full instructions for an explicitly invoked skill. Apply these instructions only to the current user turn.\n<skill-body id=\"core-noargs\" scope=\"project\" path=\".agents/skills/core-noargs/SKILL.md\" sha256=\"sha256:84104ec0b9c18ca7f0a84860b79648438cbdc6ea6ec3a775d551ca231327541e\" byte-count=\"99\">\n---\nname: &quot;core-noargs&quot;\ndescription: &quot;Body without the ARGUMENTS placeholder&quot;\n---\nNOARGS-MARK line\n\n</skill-body>\n</system-reminder>\n\nx y"

== TUI b1t-hidden screen:
  Muse Code 1.4.25
❯ /core-hidden
◆ MOCKDONE
❯
== TUI b1t-hidden first model request, user message:
### b1t-hidden/req-002.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=12: "/core-hidden"

== TUI b1t-hiddenargs screen:
  Muse Code 1.4.25
❯ /core-hidden a b
◆ MOCKDONE
❯
== TUI b1t-hiddenargs first model request, user message:
### b1t-hiddenargs/req-002.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=16: "/core-hidden a b"

== catalog entries (TUI b1t-status first request)
<skill id=\"core-hidden\" scope=\"project\" path=\".agents/skills/core-hidden/SKILL.md\">\n<description>Not user invocable</description>\n</skill>\n<skill id=\"core-noargs\" scope=\"project\" path=\".agents/skills/core-noargs/SKILL.md\">\n<description>Body without the ARGUMENTS placeholder</description>\n</skill>\n<skill id=\"core-plain\" scope=\"project\" path=\".agents/skills/core-plain/SKILL.md\">\n<description>Plain skill</description>\n</skill>\n<skill id=\"core-status\" scope=\"project\" path=\".agents/skills/core-status/SKILL.md\" model-invocable=\"false\">\n<description>Show status</description>\n</skill>\n
```

Answer:

- **Headless `muse exec` does not expand slash commands.** The prompt `/core-status argz one`
  reaches the model verbatim as the user message; no skill body is loaded.
- **Interactive TUI expands it** (screen shows `◆ Loaded skill core-status · project`). The single
  user message is: a `<system-reminder source="skill-body">` wrapper → `<skill-body id=… scope=…
  path=".agents/skills/<id>/SKILL.md" sha256=… byte-count=…>` holding the **whole SKILL.md file,
  front matter included, XML-escaped** (`"` → `&quot;`) → `</skill-body></system-reminder>` →
  blank line → the arguments (`argz one`).
- **`$ARGUMENTS` is NOT substituted** for skills (body keeps the literal `$ARGUMENTS`); the args are
  always appended after a blank line, whether or not the body contains `$ARGUMENTS` (`core-noargs`
  gets `x y` appended identically).
- `disable-model-invocation: true` (`core-status`) changes nothing for the user: `/core-status a b`
  expands exactly like `core-plain`. In the model catalog it carries `model-invocable="false"`.
- `user-invocable: false` (`core-hidden`): `/core-hidden` and `/core-hidden a b` are **not**
  expanded — sent to the model as plain text — and the skill is absent from the `/` palette (B2), but
  it stays in the model catalog with no special attribute.

## B2 — JSON-style front matter, unknown keys, name ≠ directory

Question: do `skills validate` and `skills list --json` accept JSON-encoded values? Where is
`argument-hint` shown? Diagnostics for an unknown key? Which id wins when `name` differs from the
directory?

Commands: from `$W/b2`, `muse skills validate .agents/skills/<dir> [--json]`,
`muse skills list --source project --trust-workspace [--json]`, `muse skills inspect … [--json]`;
`palette.sh $W/b1 … "/core-"`, `"/core-status"`, `"/core-status "` for the palette.

Files (`$W/b2`):

```text
--- .agents/skills/core-dirname/SKILL.md
---
name: "core-othername"
description: "Name differs from directory"
---
DIRNAME-BODY
--- .agents/skills/core-extra/SKILL.md
---
name: "core-extra"
description: "Has an unknown key"
foo: "x"
---
EXTRA-BODY
--- .agents/skills/core-status/SKILL.md
---
name: "core-status"
description: "Show status"
disable-model-invocation: true
argument-hint: "[x]"
---
BODY-MARK $ARGUMENTS
```

Raw:

```text
$ muse skills validate .agents/skills/core-status
valid	core-status
$ muse skills validate .agents/skills/core-status --json
{
  "valid": true,
  "id": "core-status",
  "source_path": ".agents/skills/core-status",
  "files": [
    {
      "relative_path": "SKILL.md",
      "sha256": "sha256:c75355088240eaad05e5d862530a087891786ae73557a768f9ac6a8926f69c74",
      "bytes": 128
    }
  ],
  "diagnostics": [],
  "compatibility": {
    "profile": "agent-skills-common-subset",
    "result": "compatible",
    "known_fields": [
      "argument-hint",
      "description",
      "disable-model-invocation",
      "name"
    ],
    "unknown_fields": [],
    "unsupported_fields": [],
    "allowed_tools": []
  }
}
$ muse skills validate .agents/skills/core-extra
valid	core-extra
$ muse skills validate .agents/skills/core-extra --json
{
  "valid": true,
  "id": "core-extra",
  "source_path": ".agents/skills/core-extra",
  "files": [
    {
      "relative_path": "SKILL.md",
      "sha256": "sha256:bc6938436f119a0e40af2056ff95f62341641e028e3c161471818331f3f7cda7",
      "bytes": 81
    }
  ],
  "diagnostics": [],
  "compatibility": {
    "profile": "agent-skills-common-subset",
    "result": "compatible",
    "known_fields": [
      "description",
      "name"
    ],
    "unknown_fields": [
      "foo"
    ],
    "unsupported_fields": [],
    "allowed_tools": []
  }
}
$ muse skills validate .agents/skills/core-dirname
valid	core-othername
$ muse skills validate .agents/skills/core-dirname --json
{
  "valid": true,
  "id": "core-othername",
  "source_path": ".agents/skills/core-dirname",
  "files": [
    {
      "relative_path": "SKILL.md",
      "sha256": "sha256:f36489451332b5b8820df5a817d5a0cedb35dd9f55357a3bc960958652d93814",
      "bytes": 87
    }
  ],
  "diagnostics": [],
  "compatibility": {
    "profile": "agent-skills-common-subset",
    "result": "compatible",
    "known_fields": [
      "description",
      "name"
    ],
    "unknown_fields": [],
    "unsupported_fields": [],
    "allowed_tools": []
  }
}
```

```text
$ muse skills list --source project --trust-workspace
NAME	SCOPE	ACTIVATION	DESCRIPTION	PATH
core-extra	project	on	Has an unknown key	.agents/skills/core-extra/SKILL.md
core-othername	project	on	Name differs from directory	.agents/skills/core-dirname/SKILL.md
core-status	project	user-invocable-only	Show status	.agents/skills/core-status/SKILL.md
$ muse skills list --source project --trust-workspace --json
{
  "skills": [
    {
      "id": "core-extra",
      "name": "core-extra",
      "display_name": "core-extra",
      "description": "Has an unknown key",
      "short_description": null,
      "scope": "project",
      "source": {
        "type": "local"
      },
      "path": ".agents/skills/core-extra/SKILL.md",
      "activation": "on",
      "diagnostics": [],
      "provenance": null,
      "context_cost": {
        "startup_bytes": 89,
        "startup_estimated_tokens": 23,
        "invoke_bytes": null,
        "invoke_estimated_tokens": null
      }
    },
    {
      "id": "core-othername",
      "name": "core-othername",
      "display_name": "core-othername",
      "description": "Name differs from directory",
      "short_description": null,
      "scope": "project",
      "source": {
        "type": "local"
      },
      "path": ".agents/skills/core-dirname/SKILL.md",
      "activation": "on",
      "diagnostics": [],
      "provenance": null,
      "context_cost": {
        "startup_bytes": 112,
        "startup_estimated_tokens": 28,
        "invoke_bytes": null,
        "invoke_estimated_tokens": null
      }
    },
    {
      "id": "core-status",
      "name": "core-status",
      "display_name": "core-status",
      "description": "Show status",
      "short_description": null,
      "scope": "project",
      "source": {
        "type": "local"
      },
      "path": ".agents/skills/core-status/SKILL.md",
      "activation": "user-invocable-only",
      "diagnostics": [],
      "provenance": null,
      "context_cost": {
        "startup_bytes": 86,
        "startup_estimated_tokens": 22,
        "invoke_bytes": null,
        "invoke_estimated_tokens": null
      }
    }
  ],
  "diagnostics": []
}
```

```text
$ muse skills inspect core-status --source project --trust-workspace
id: core-status
name: core-status
scope: project
activation: user-invocable-only
path: .agents/skills/core-status/SKILL.md
description: Show status
$ muse skills inspect core-status --source project --trust-workspace --json
{
  "skill": {
    "id": "core-status",
    "name": "core-status",
    "display_name": "core-status",
    "description": "Show status",
    "short_description": null,
    "scope": "project",
    "source": {
      "type": "local"
    },
    "path": ".agents/skills/core-status/SKILL.md",
    "activation": "user-invocable-only",
    "diagnostics": [],
    "provenance": null,
    "frontmatter": {
      "known": {},
      "unknown": {}
    }
  }
}
$ muse skills inspect core-dirname --source project --trust-workspace
skill not found: core-dirname
```

TUI palette (`$W/b1`):

```text
== typed: /core-
  Muse Code 1.4.25
❯ /core-
  /core-noargs · project · default skill  Body without the ARGUMENTS placeholder
  /core-plain · project · default skill   Plain skill
  /core-status · project · default skill  Show status
== typed: /core-status
  Muse Code 1.4.25
❯ /core-status [x]
  /core-status · project · default skill  Show status
== typed: /core-status␠
  Muse Code 1.4.25
❯ /core-status [x]
```

Answer:

- JSON-style values (`name: "core-status"`, `argument-hint: "[x]"`) are accepted: `valid`, the
  quotes are stripped (`id: core-status`, `description: Show status`), `known_fields` lists
  `argument-hint`, `disable-model-invocation`, and `activation` becomes `user-invocable-only`.
- `argument-hint` appears nowhere in `list`/`inspect` output; it is shown only in the TUI input line
  as ghost text after the exact command (`❯ /core-status [x]`). The palette rows show
  `/<id> · project · default skill  <description>`.
- Unknown key `foo`: still `valid`, **no diagnostic** (`diagnostics: []`, `list` shows none); only
  `validate --json` reports it under `compatibility.unknown_fields: ["foo"]`. The skill loads (`on`).
- `name` ≠ directory: the **front-matter `name` wins** (`core-othername`, path
  `.agents/skills/core-dirname/SKILL.md`); `skills inspect core-dirname` → `skill not found:
  core-dirname`. No diagnostic.
- Side note: `skills inspect --json` shows `"frontmatter": {"known": {}, "unknown": {}}` (empty) even
  for a skill with keys.

## B3 — agent definitions: which variants list, and which can actually spawn

Question: for JSON-style agent files, which `tools` names, `model`, `effort`, `name` and
`description` variants are listed in the catalog; any warning?

Commands:

```text
items-run.sh $W/b3  empty.json     $W/logs/b3  "probe agents" --json          # catalog (req-001) + JSONL events
items-run.sh $W/b3b empty.json     $W/logs/b3c "probe agents"                 # description 4096/4097/5000 with letter-only names
node spawnseq.mjs "probe agents" <ids…> > b3-spawn.json
items-run.sh $W/b3  b3-spawn.json  $W/logs/b3s "probe agents" --disable-reminders -c agents.execution_capacity=64   # spawn each
items-run.sh $W/b4b b4b-spawn.json $W/logs/b4b "probe agents" -c agents.execution_capacity=64                       # edit_file, write_file, web_fetch, web_search, ["read_file","Read"], []
```

Files: `$W/b3/.agents/agents/*.md`, each `---` + `name` + `description` + listed keys + `---` +
`BODY of <name>` (front matter shown, long descriptions abbreviated by the listing script):

```text
capital.md: name: "Core-Coder" | description: "Probe agent Core-Coder" | 
core_coder.md: name: "core_coder" | description: "Probe agent core_coder" | 
core-coder.md: name: "core-coder" | description: "Probe agent core-coder" | tools: ["Read","Edit"] | effort: "high" | 
core-coder2.md: name: "core-coder2" | description: "Probe agent core-coder2" | 
desc-4096.md: name: "desc-4096" | description: "yyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyyy…(4111 chars) | 
desc-4097.md: name: "desc-4097" | description: "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz…(4112 chars) | 
desc-long.md: name: "desc-long" | description: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx…(5015 chars) | 
effort-absent.md: name: "effort-absent" | description: "Probe agent effort-absent" | 
effort-bogus.md: name: "effort-bogus" | description: "Probe agent effort-bogus" | effort: "bogus" | 
effort-high.md: name: "effort-high" | description: "Probe agent effort-high" | effort: "high" | 
effort-low.md: name: "effort-low" | description: "Probe agent effort-low" | effort: "low" | 
effort-max.md: name: "effort-max" | description: "Probe agent effort-max" | effort: "max" | 
effort-medium.md: name: "effort-medium" | description: "Probe agent effort-medium" | effort: "medium" | 
effort-minimal.md: name: "effort-minimal" | description: "Probe agent effort-minimal" | effort: "minimal" | 
effort-none.md: name: "effort-none" | description: "Probe agent effort-none" | effort: "none" | 
effort-ultra.md: name: "effort-ultra" | description: "Probe agent effort-ultra" | effort: "ultra" | 
effort-xhigh.md: name: "effort-xhigh" | description: "Probe agent effort-xhigh" | effort: "xhigh" | 
file-stem.md: name: "core-namediff" | description: "Probe agent core-namediff" | 
model-absent.md: name: "model-absent" | description: "Probe agent model-absent" | 
model-fake.md: name: "model-fake" | description: "Probe agent model-fake" | model: "fake-model" | 
model-inherit.md: name: "model-inherit" | description: "Probe agent model-inherit" | model: "inherit" | 
model-nonexistent.md: name: "model-nonexistent" | description: "Probe agent model-nonexistent" | model: "nonexistent-model" | 
tool-bash-cap.md: name: "tool-bash-cap" | description: "Probe agent tool-bash-cap" | tools: ["Bash"] | 
tool-bash.md: name: "tool-bash" | description: "Probe agent tool-bash" | tools: ["bash"] | 
tool-edit-cap.md: name: "tool-edit-cap" | description: "Probe agent tool-edit-cap" | tools: ["Edit"] | 
tool-glob-cap.md: name: "tool-glob-cap" | description: "Probe agent tool-glob-cap" | tools: ["Glob"] | 
tool-grep-cap.md: name: "tool-grep-cap" | description: "Probe agent tool-grep-cap" | tools: ["Grep"] | 
tool-nonexistenttool.md: name: "tool-nonexistenttool" | description: "Probe agent tool-nonexistenttool" | tools: ["nonexistent_tool"] | 
tool-read-cap.md: name: "tool-read-cap" | description: "Probe agent tool-read-cap" | tools: ["Read"] | 
tool-readfile.md: name: "tool-readfile" | description: "Probe agent tool-readfile" | tools: ["read_file"] | 
tool-readskill.md: name: "tool-readskill" | description: "Probe agent tool-readskill" | tools: ["read_skill"] | 
tool-search.md: name: "tool-search" | description: "Probe agent tool-search" | tools: ["search"] | 
tool-skill-cap.md: name: "tool-skill-cap" | description: "Probe agent tool-skill-cap" | tools: ["Skill"] | 
tool-webfetch-cap.md: name: "tool-webfetch-cap" | description: "Probe agent tool-webfetch-cap" | tools: ["WebFetch"] | 
tool-write-cap.md: name: "tool-write-cap" | description: "Probe agent tool-write-cap" | tools: ["Write"] | 
```

`$W/b3b/.agents/agents/`: `desc-fourzeroninesix.md`, `desc-fourzeronineseven.md`,
`desc-fivezerozerozero.md` — same shape, description of 4096 / 4097 / 5000 `y` bytes. (Digits in
names are refused, so the first run's `desc-4096` could not isolate the length rule.) `b4b`
extra agent files are in B4.

Raw:

```text
== catalog (b3 req-001)
"Reviewed Agent Definition catalog for this run. Use an exact listed id:\\n- core-coder\\n- core-namediff\\n- effort-absent\\n- effort-high\\n- effort-low\\n- effort-max\\n- effort-medium\\n- effort-minimal\\n- effort-none\\n- effort-ultra\\n- effort-xhigh\\n- general-purpose\\n- model-absent\\n- model-fake\\n- model-inherit\\n- model-nonexistent\\n- tool-bash\\n- tool-bash-cap\\n- tool-edit-cap\\n- tool-glob-cap\\n- tool-grep-cap\\n- tool-nonexistenttool\\n- tool-read-cap\\n- tool-readfile\\n- tool-readskill\\n- tool-search\\n- tool-skill-cap\\n- tool-webfetch-cap\\n- tool-write-cap\\n- workflow-subagent"
== exec stderr (b3)
muse: workspace root: $W/b3 (cwd default)
muse: workspace trust: trusted source=run-flag
== description-length run (b3c) catalog
"Reviewed Agent Definition catalog for this run. Use an exact listed id:\\n- desc-fourzeroninesix\\n- general-purpose\\n- workflow-subagent"
== spawn results (b3s)
# lead request req-051.json
core-coder	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`core-coder` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
core-namediff	{"status":"accepted","subagent_id":"01a21211-cd60-7ce1-a7c3-6bf7ba2e8f48","work_id":"work.v2.native_subagent.sha256.3de8990f0806cd6aa196bbebd26e0f415c0903f961b5044f3a0e959d09c9ef72","agent_path":"main/probe/1","task_ref":"task/01a21211-cd5c-7c90-b5d4-ff2665e0e71c#0"}
tool-read-cap	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`tool-read-cap` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
tool-edit-cap	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`tool-edit-cap` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
tool-write-cap	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`tool-write-cap` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
tool-grep-cap	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`tool-grep-cap` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
tool-glob-cap	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`tool-glob-cap` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
tool-bash-cap	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`tool-bash-cap` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
tool-skill-cap	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`tool-skill-cap` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
tool-readfile	{"status":"accepted","subagent_id":"01a21211-d1c9-7020-adad-77c957a590ee","work_id":"work.v2.native_subagent.sha256.2c44119cce2a58c0117be0d8e6062614cac07a315bc68a3dfb039aa107c881e2","agent_path":"main/probe/2","task_ref":"task/01a21211-d1c8-7b13-ba50-54ec7e0c1850#0"}
tool-search	{"status":"accepted","subagent_id":"01a21211-d34f-73b3-93f7-fd203f380593","work_id":"work.v2.native_subagent.sha256.3073f20cfdb8a564c9ad55dc6c1a5b30b8ebc9906100a525be1258797b65ce75","agent_path":"main/probe/3","task_ref":"task/01a21211-d34f-73b3-93f7-fd007b33f3bc#0"}
tool-bash	{"status":"accepted","subagent_id":"01a21211-d4e2-7693-b455-d24d34d75f97","work_id":"work.v2.native_subagent.sha256.9dcace676a57b7c49f8fd7aca4ccad35178ec2753c7407b93bb9088a9f318c34","agent_path":"main/probe/4","task_ref":"task/01a21211-d4e2-7693-b455-d222695c65ee#0"}
tool-readskill	{"status":"accepted","subagent_id":"01a21211-d675-7873-947d-e8969a86f1d8","work_id":"work.v2.native_subagent.sha256.75c87a6c7adeb966ff721d06f3ecf098054fdb8188a8edceeaf0050e9d328519","agent_path":"main/probe/5","task_ref":"task/01a21211-d675-7873-947d-e871f3840c92#0"}
tool-webfetch-cap	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`tool-webfetch-cap` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
tool-nonexistenttool	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`tool-nonexistenttool` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
model-absent	{"status":"accepted","subagent_id":"01a21211-d88b-7512-9608-48f02e576b11","work_id":"work.v2.native_subagent.sha256.8ad60c30c2638da5babaf36737ac822322a92939ba37eb28ae5f5961afb4d320","agent_path":"main/probe/6","task_ref":"task/01a21211-d88a-7850-9070-116e2087fbd7#0"}
model-inherit	{"status":"accepted","subagent_id":"01a21211-da0c-73a2-85c1-2dde2823c300","work_id":"work.v2.native_subagent.sha256.87dfce142c2d123c53918b4e15e76105d605b16171bc23c8657444249c0e669c","agent_path":"main/probe/7","task_ref":"task/01a21211-da0c-73a2-85c1-2dbc3dfd448e#0"}
model-fake	{"status":"accepted","subagent_id":"01a21211-db8f-7ae2-9cc8-f31d31f3e5fd","work_id":"work.v2.native_subagent.sha256.b341569828635e41ac2c063d0a794ad29bb80900eec9df2b4330e4ef3b36eab0","agent_path":"main/probe/8","task_ref":"task/01a21211-db8f-7ae2-9cc8-f2fa1e246593#0"}
model-nonexistent	{"status":"accepted","subagent_id":"01a21211-dd15-7360-b633-28c0be50236a","work_id":"work.v2.native_subagent.sha256.f469db2d0122501afa6a9a12aa12f97c6d89b2490fbf20bf592208c815f26933","agent_path":"main/probe/9","task_ref":"task/01a21211-dd15-7360-b633-28a93ce2ebe2#0"}
effort-absent	{"status":"accepted","subagent_id":"01a21211-deb3-7131-a1e1-adab9ac146aa","work_id":"work.v2.native_subagent.sha256.d2c67c8118dbc000f67afc50360800da893af977fedf6691e77e67a51acb12db","agent_path":"main/probe/10","task_ref":"task/01a21211-deb3-7131-a1e1-ad8d83f40745#0"}
effort-none	{"status":"accepted","subagent_id":"01a21211-e079-7cf0-bdfe-7e6df2122afe","work_id":"work.v2.native_subagent.sha256.b5ab828e55ff0ff42a6fcbcf155857a28859cbd0a471c5fa5d596f1ce7982102","agent_path":"main/probe/11","task_ref":"task/01a21211-e079-7cf0-bdfe-7e40271f1656#0"}
effort-minimal	{"status":"accepted","subagent_id":"01a21211-e212-7f90-9a98-820268d72659","work_id":"work.v2.native_subagent.sha256.20f248343f363cd2c81cfebd385d8f398f4a71cee41809c7113dd4481ff57971","agent_path":"main/probe/12","task_ref":"task/01a21211-e211-78b2-af11-da6f464b089b#0"}
effort-low	{"status":"accepted","subagent_id":"01a21211-e3e0-72f0-a239-ce25f5bc8184","work_id":"work.v2.native_subagent.sha256.8323e2cb6357c7fae021bd6acd43fbf780c18351572a8015c390ae878f68f2b2","agent_path":"main/probe/13","task_ref":"task/01a21211-e3e0-72f0-a239-ce008832819f#0"}
effort-medium	{"status":"accepted","subagent_id":"01a21211-e589-7920-a3f5-e467afe84f93","work_id":"work.v2.native_subagent.sha256.29841e52bf700b61e8b57d1ebba5c0bee16a12893df28eff575085dc309c11a6","agent_path":"main/probe/14","task_ref":"task/01a21211-e588-7593-a6fd-1eb611de5c47#0"}
effort-high	{"status":"accepted","subagent_id":"01a21211-e713-7d00-9f4d-1814c213bb9b","work_id":"work.v2.native_subagent.sha256.84a7c578f0b7b83dedf3ff2f06b37d1de8e16e321996fea455fe10843832802b","agent_path":"main/probe/15","task_ref":"task/01a21211-e712-79e2-80ca-8821d453f491#0"}
effort-xhigh	{"status":"accepted","subagent_id":"01a21211-e91a-7a82-a455-ced2177c36fd","work_id":"work.v2.native_subagent.sha256.c00cacc94cce6838a0c6a183a3c20a3256ef2758372943d0860a2f7490682ce8","agent_path":"main/probe/16","task_ref":"task/01a21211-e91a-7a82-a455-ceb74601a076#0"}
effort-max	{"status":"accepted","subagent_id":"01a21211-eac2-7170-879b-3d1d39f1e2f8","work_id":"work.v2.native_subagent.sha256.a0dcf5ba3191b653b43f8428e0ed7ad2588679cf72b20e0bfd23f3b5503ac2cd","agent_path":"main/probe/17","task_ref":"task/01a21211-eac1-7783-b9f3-5f2453e6442a#0"}
effort-ultra	{"status":"accepted","subagent_id":"01a21211-ec5b-7821-a2b6-d205c86771ac","work_id":"work.v2.native_subagent.sha256.87e8c757fe7004c4551d2e1a1bccf38d68453d724b372485961fa7d78cb450ad","agent_path":"main/probe/18","task_ref":"task/01a21211-ec5b-7821-a2b6-d1e2ef58cc7a#0"}
effort-bogus	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"Agent Definition `effort-bogus` is invalid in the project source with project depth 0 scope: invalid field (field: effort)."}
core-coder2	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`subagent_type` must be a canonical Agent Definition identifier: use lowercase ASCII letter segments joined by `-`, or `<plugin-id>[/<scope>...]/<name>` for a plugin-scoped definition."}
core_coder	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`subagent_type` must be a canonical Agent Definition identifier: use lowercase ASCII letter segments joined by `-`, or `<plugin-id>[/<scope>...]/<name>` for a plugin-scoped definition."}
Core-Coder	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`subagent_type` must be a canonical Agent Definition identifier: use lowercase ASCII letter segments joined by `-`, or `<plugin-id>[/<scope>...]/<name>` for a plugin-scoped definition."}
== child requests (b3s)
req-003.json	core-namediff	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-012.json	tool-readfile	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_file,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-014.json	tool-search	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=search,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-016.json	tool-bash	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=bash,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-018.json	tool-readskill	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_skill,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-022.json	model-absent	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-024.json	model-inherit	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-026.json	model-fake	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-028.json	model-nonexistent	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-030.json	effort-absent	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-032.json	effort-none	model=fake-model	reasoning={"effort":"minimal","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-034.json	effort-minimal	model=fake-model	reasoning={"effort":"minimal","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-036.json	effort-low	model=fake-model	reasoning={"effort":"low","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-038.json	effort-medium	model=fake-model	reasoning={"effort":"medium","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-040.json	effort-high	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-042.json	effort-xhigh	model=fake-model	reasoning={"effort":"xhigh","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-044.json	effort-max	model=fake-model	reasoning={"effort":"max","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-046.json	effort-ultra	model=fake-model	reasoning={"effort":"max","summary":"auto"}	tools=read_skill,web_search,read_file,search,write_file,edit_file,read_memory,add_memory,edit_memory,web_fetch,bash,bash_input,monitor,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
== lead notice after model-nonexistent spawn (b3s req-051)
--- b3s/req-051.json @39549: …guage.\n\ndefinition_model_notice: Native c…
== extra tool names (b4b)
# lead request req-016.json
core-coder	{"status":"accepted","subagent_id":"01a21213-262b-7382-860e-0ed5de05f98a","work_id":"work.v2.native_subagent.sha256.ead0930e497d33e2e9ab9dcd316b6eb93bb837a10dd9275924184852df37b380","agent_path":"main/probe/1","task_ref":"task/01a21213-2624-7e02-9101-fef5e5407238#0"}
tool-editfile	{"status":"accepted","subagent_id":"01a21213-290c-7d33-ae3f-e918320fb9d0","work_id":"work.v2.native_subagent.sha256.ef69b9bc767147f147345faffcea013f885fdd61bb035eb43e3015f2925258d6","agent_path":"main/probe/2","task_ref":"task/01a21213-290c-7d33-ae3f-e8f4a05cce6f#0"}
tool-writefile	{"status":"accepted","subagent_id":"01a21213-2b44-7cd3-89a6-95cbcaa32fe6","work_id":"work.v2.native_subagent.sha256.486e5cc7153e955c7bf9e60aadba1e4dcc5e3cfc9725532332172ee68acdf070","agent_path":"main/probe/3","task_ref":"task/01a21213-2b40-7961-b1b8-dc7b02602c42#0"}
tool-webfetch	{"status":"accepted","subagent_id":"01a21213-2cc5-7e83-b265-9a8c2ee56d7e","work_id":"work.v2.native_subagent.sha256.0fb6f9ab0c3fa72f55f0f1289e81207e2418547e4b78f01cc14d4d3e71b28ed0","agent_path":"main/probe/4","task_ref":"task/01a21213-2cc4-7963-afc1-42d7d0952c6d#0"}
tool-websearch	{"status":"accepted","subagent_id":"01a21213-2e9d-7130-92e7-b4786415dec2","work_id":"work.v2.native_subagent.sha256.67ce961e48ecce788bd716aba4fb599b51d8725682efde46d25d590052834fcf","agent_path":"main/probe/5","task_ref":"task/01a21213-2e9d-7130-92e7-b4504c0c8371#0"}
tool-multi	{"status":"rejected","reason":"child_run_rejected","cause":"agent_definition_invalid","hint":"`tool-multi` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."}
tool-empty	{"status":"accepted","subagent_id":"01a21213-30ed-7102-8754-b890bfdad10c","work_id":"work.v2.native_subagent.sha256.91879b890da8caf1bc7a23821d9b4de97d23bb8112b144e5032de031f53c4007","agent_path":"main/probe/6","task_ref":"task/01a21213-30ed-7102-8754-b872b29ceb18#0"}
req-002.json	core-coder	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_file,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-004.json	tool-editfile	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=edit_file,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-006.json	tool-writefile	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=write_file,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-008.json	tool-webfetch	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-010.json	tool-websearch	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=web_search,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
req-013.json	tool-empty	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
```

Answer — **listing in the catalog is not the same as valid.** The catalog lists every file whose
name is canonical and whose description is ≤ 4096 bytes and whose `effort` is a known value; the
`tools` names are only checked when the lead calls `subagent_spawn`, which then rejects. Nothing is
printed on stderr and nothing appears in the `--json` event stream for any invalid agent.

| Variant | In catalog | `subagent_spawn` result |
|---|---|---|
| `tools: ["read_file"]`, `["search"]`, `["bash"]`, `["read_skill"]`, `["edit_file"]`, `["write_file"]`, `["web_search"]`, `["web_fetch"]`, `[]` | yes | accepted; child offered only those tools + `subagent_*` + `work_status` (`web_fetch` child got no `web_fetch`; `[]` → only `subagent_*`, `work_status`) |
| `tools:` `Read` `Edit` `Write` `Grep` `Glob` `Bash` `Skill` `WebFetch` `nonexistent_tool`, and `["read_file","Read"]` | yes | rejected: ``"`<id>` tools: unknown_tool. Definition/catalog fixed for this run; child will not start. Do not retry unchanged."`` |
| `model` absent / `"inherit"` / `"fake-model"` | yes | accepted; child runs `fake-model` (no notice for `inherit`) |
| `model: "nonexistent-model"` | yes | accepted; runs on lead model; lead gets `definition_model_notice: … asks for nonexistent-model, which is not available in this session; 1 call ran on fake-model instead. Tell the user which model ran.` |
| `effort` absent / minimal / low / medium / high / xhigh / max | yes | accepted; request `reasoning.effort` = lead's `high` / same value |
| `effort: "none"` | yes | accepted; request sends `minimal` |
| `effort: "ultra"` | yes | accepted; request sends `max` |
| `effort: "bogus"` | **no** | rejected: ``"Agent Definition `effort-bogus` is invalid in the project source with project depth 0 scope: invalid field (field: effort)."`` |
| `name: "core-coder2"` (digit), `"core_coder"`, `"Core-Coder"` | **no** | rejected: ``"`subagent_type` must be a canonical Agent Definition identifier: use lowercase ASCII letter segments joined by `-`, …"`` |
| `name` ≠ file stem (`file-stem.md`, name `core-namediff`) | yes, as `core-namediff` | accepted (the front-matter name is the id) |
| description 4096 bytes | yes | — |
| description 4097 / 5000 bytes | **no** | — |

Valid agent tool names are Muse's native tool names (`read_file`, `edit_file`, `write_file`,
`search`, `bash`, `read_skill`, `web_search`, `web_fetch`, …). One unknown name invalidates the whole
definition. The catalog shows ids only (no description) in the main request.

## B4 — spawning a pack agent; hooks in the child

Question: child request contents (body? tools restricted?), hook payloads in the child
(PreToolUse/SubagentStart/SubagentStop), agent-name fields, `last_assistant_message`.

Commands:

```text
items-run.sh $W/b4  b4-script.json  $W/logs/b4  "probe spawn"
items-run.sh $W/b4b b4b-spawn.json  $W/logs/b4b "probe agents" -c agents.execution_capacity=64   # matcher test
```

(The first run used `tools: ["Read"]`: spawn rejected with `unknown_tool`, see B3; the file below
uses `read_file`.)

Files:

```text
--- .agents/agents/core-coder.md
---
name: "core-coder"
description: "Writes code for one task"
tools: ["read_file"]
effort: "high"
---
CODER-BODY You are the coder.
--- a.txt
A-CONTENT
--- .muse/hooks.json
{"hooks":{
 "PreToolUse":[{"hooks":[{"type":"command","command":"$W/tools/dump.sh PreToolUse $W/hookout/b4"}]}],
 "SubagentStart":[{"hooks":[{"type":"command","command":"$W/tools/dump.sh SubagentStart $W/hookout/b4"}]}],
 "SubagentStop":[{"hooks":[{"type":"command","command":"$W/tools/dump.sh SubagentStop $W/hookout/b4"}]}]
}}
--- mock script
[{"when":["probe spawn"],"unless":["You are a goal reminder","CODER-BODY","\"call_id\":\"call_"],"name":"subagent_spawn","args":{"command_id":"spawn-1","role":"coder","objective":"Read a.txt and report its content","subagent_type":"core-coder"}},
 {"when":["CODER-BODY"],"unless":["You are a goal reminder","\"call_id\":\"call_"],"name":"read_file","args":{"path":"a.txt","offset":null,"limit":null}},
 {"when":["probe spawn","spawn-1"],"unless":["You are a goal reminder","CODER-BODY","subagent-wait-1"],"name":"subagent_wait","args":{"command_id":"subagent-wait-1"}}]
--- b4b extra agents
-- tool-editfile.md
---
name: "tool-editfile"
description: "probe"
tools: ["edit_file"]
---
BODY of tool-editfile
-- tool-empty.md
---
name: "tool-empty"
description: "probe"
tools: []
---
BODY of tool-empty
-- tool-multi.md
---
name: "tool-multi"
description: "probe"
tools: ["read_file","Read"]
---
BODY of tool-multi
-- tool-webfetch.md
---
name: "tool-webfetch"
description: "probe"
tools: ["web_fetch"]
---
BODY of tool-webfetch
-- tool-websearch.md
---
name: "tool-websearch"
description: "probe"
tools: ["web_search"]
---
BODY of tool-websearch
-- tool-writefile.md
---
name: "tool-writefile"
description: "probe"
tools: ["write_file"]
---
BODY of tool-writefile
```

Raw:

```text
== first run (tools: ["Read"]) is in B3: spawn rejected `core-coder` tools: unknown_tool
== child request (b4 req-003): model/effort/tools
req-003.json	CODER-BODY	model=fake-model	reasoning={"effort":"high","summary":"auto"}	tools=read_file,subagent_spawn,subagent_status,subagent_send_message,subagent_wait,subagent_read_result,subagent_cancel,work_status
== child request: where the body sits (developer message [0])
--- b4/req-003.json @3295: … status update: keep working, or end the run with a completely empty final answer (no text at all, not even whitespace), which is discarded and never shown. User opt-outs (\"don't use subagents\", \"single agent\") always win.\n</system-reminder>\n\nCODER-BODY You are the coder.\n\n# Parent task updates\n\nYou are a delegated child agent. Your opening objective comes from your parent and can be revised by that parent; it is not a separate instruction from the human. A subagent_input message delivered by t…
== child request user message
### b4/req-003.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=56: "Role: coder\nObjective: Read a.txt and report its content"
== child second request (b4 req-005): read_file call and its output
### b4/req-005.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=56: "Role: coder\nObjective: Read a.txt and report its content"
[2] type=function_call role= name=muse.read_file len=43: "{\"path\":\"a.txt\",\"offset\":null,\"limit\":null}"
[3] type=function_call_output role= name= len=35: "Read text file `a.txt`.\n1|A-CONTENT"
== lead request after spawn (b4 req-007)
### b4/req-007.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=11: "probe spawn"
[2] type=function_call role= name=muse.subagent_spawn len=116: "{\"command_id\":\"spawn-1\",\"role\":\"coder\",\"objective\":\"Read a.txt and report its content\",\"subagent_type\":\"core-coder\"}"
[3] type=function_call_output role= name= len=267: "{\"status\":\"accepted\",\"subagent_id\":\"01a21212-8ef2-71b2-8a6d-9d479555de52\",\"work_id\":\"work.v2.native_subagent.sha256.917b3a7411de9f4ca703cb47da489437b9f3ff3800e248388ebc370446c8bf8e\",\"agent_path\":\"main/coder/1\",\"task_ref\":\"task/01a21212-8ee7-7892-8daf-1db7b7e85c82#0\"}"
[4] type=function_call role= name=muse.subagent_wait len=32: "{\"command_id\":\"subagent-wait-1\"}"
[5] type=function_call_output role= name= len=254: "{\"status\":\"rejected\",\"reason\":\"target_missing\",\"hint\":\"Pass the spawn-returned subagent_id or exact agent_path, plus a command_id for this operation when required. command_id does not select the child. Any attempt_ref must belong to the selected Agent.\"}"
[6] type=message role=assistant name= len=8: "MOCKDONE"
== hook dumps (b4), stdin + MUSE_* env of the hook process
-- PreToolUse  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-86d5-7232-9339-6c41b0427c8b 
{"hook_event_name":"PreToolUse","tool_name":"subagent_spawn","tool_input":{"command_id":"spawn-1","role":"coder","objective":"Read a.txt and report its content","subagent_type":"core-coder"},"tool_use_id":"call_1","session_id":"01a21212-86d5-7232-9339-6c41b0427c8b","turn_id":"01a21212-8da4-7182-9ce8-39af7af61046","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- PreToolUse  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-8f37-7f12-93fe-033f20c3dfbd 
{"hook_event_name":"PreToolUse","tool_name":"read_file","tool_input":{"path":"a.txt","offset":null,"limit":null},"tool_use_id":"call_3","session_id":"01a21212-8f37-7f12-93fe-033f20c3dfbd","turn_id":"01a21212-8f37-7f12-93fe-033f20c3dfbd","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- PreToolUse  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-86d5-7232-9339-6c41b0427c8b 
{"hook_event_name":"PreToolUse","tool_name":"subagent_wait","tool_input":{"command_id":"subagent-wait-1"},"tool_use_id":"call_2","session_id":"01a21212-86d5-7232-9339-6c41b0427c8b","turn_id":"01a21212-8da4-7182-9ce8-39af7af61046","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- SubagentStart  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-8f37-7f12-93fe-033f20c3dfbd 
{"hook_event_name":"SubagentStart","subagent_id":"01a21212-8ef2-71b2-8a6d-9d479555de52","child_session_id":"01a21212-8f37-7f12-93fe-033f20c3dfbd","session_id":"01a21212-8f37-7f12-93fe-033f20c3dfbd","turn_id":"01a21212-8f37-7f12-93fe-033f20c3dfbd","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- SubagentStart  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-91f8-71c1-b9e0-2aec6085df9a 
{"hook_event_name":"SubagentStart","subagent_id":"goal-reminder","child_session_id":"01a21212-91f8-71c1-b9e0-2aec6085df9a","session_id":"01a21212-91f8-71c1-b9e0-2aec6085df9a","turn_id":"01a21212-91f8-71c1-b9e0-2aec6085df9a","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- SubagentStart  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-9316-7170-b97e-02f580637abd 
{"hook_event_name":"SubagentStart","subagent_id":"goal-reminder","child_session_id":"01a21212-9316-7170-b97e-02f580637abd","session_id":"01a21212-9316-7170-b97e-02f580637abd","turn_id":"01a21212-9316-7170-b97e-02f580637abd","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- SubagentStop  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-8f37-7f12-93fe-033f20c3dfbd 
{"hook_event_name":"SubagentStop","subagent_id":"01a21212-8ef2-71b2-8a6d-9d479555de52","child_session_id":"01a21212-8f37-7f12-93fe-033f20c3dfbd","stop_hook_active":false,"last_assistant_message":"MOCKDONE","session_id":"01a21212-8f37-7f12-93fe-033f20c3dfbd","turn_id":"01a21212-8f37-7f12-93fe-033f20c3dfbd","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- SubagentStop  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-91f8-71c1-b9e0-2aec6085df9a 
{"hook_event_name":"SubagentStop","subagent_id":"goal-reminder","child_session_id":"01a21212-91f8-71c1-b9e0-2aec6085df9a","stop_hook_active":false,"last_assistant_message":"MOCKDONE","session_id":"01a21212-91f8-71c1-b9e0-2aec6085df9a","turn_id":"01a21212-91f8-71c1-b9e0-2aec6085df9a","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- SubagentStop  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-9316-7170-b97e-02f580637abd 
{"hook_event_name":"SubagentStop","subagent_id":"goal-reminder","child_session_id":"01a21212-9316-7170-b97e-02f580637abd","stop_hook_active":false,"last_assistant_message":"MOCKDONE","session_id":"01a21212-9316-7170-b97e-02f580637abd","turn_id":"01a21212-9316-7170-b97e-02f580637abd","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- SubagentStop  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-9316-7170-b97e-02f580637abd 
{"hook_event_name":"SubagentStop","subagent_id":"goal-reminder","child_session_id":"01a21212-9316-7170-b97e-02f580637abd","stop_hook_active":false,"last_assistant_message":"MOCKDONE","session_id":"01a21212-9316-7170-b97e-02f580637abd","turn_id":"01a21212-9316-7170-b97e-02f580637abd","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- SubagentStop  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-9316-7170-b97e-02f580637abd 
{"hook_event_name":"SubagentStop","subagent_id":"goal-reminder","child_session_id":"01a21212-9316-7170-b97e-02f580637abd","stop_hook_active":false,"last_assistant_message":"MOCKDONE","session_id":"01a21212-9316-7170-b97e-02f580637abd","turn_id":"01a21212-9316-7170-b97e-02f580637abd","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
-- SubagentStop  MUSE_SESSION_ID=01a21212-86d5-7232-9339-6c41b0427c8b MUSE_THREAD_ID=01a21212-9316-7170-b97e-02f580637abd 
{"hook_event_name":"SubagentStop","subagent_id":"goal-reminder","child_session_id":"01a21212-9316-7170-b97e-02f580637abd","stop_hook_active":false,"last_assistant_message":"MOCKDONE","session_id":"01a21212-9316-7170-b97e-02f580637abd","turn_id":"01a21212-9316-7170-b97e-02f580637abd","cwd":"$W/b4","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
== matcher run (b4b): hooks.json
{"hooks":{
 "SubagentStart":[{"matcher":"core-coder","hooks":[{"type":"command","command":"$W/tools/dump.sh SubagentStartMatchCoreCoder $W/hookout/b4b"}]}],
 "SubagentStop":[{"matcher":"core-coder","hooks":[{"type":"command","command":"$W/tools/dump.sh SubagentStopMatchCoreCoder $W/hookout/b4b"}]},
                 {"matcher":"goal-reminder","hooks":[{"type":"command","command":"$W/tools/dump.sh SubagentStopMatchGoalReminder $W/hookout/b4b"}]},
                 {"hooks":[{"type":"command","command":"$W/tools/dump.sh SubagentStopAll $W/hookout/b4b"}]}]
}}
== dump files written (b4b)
  10 SubagentStopAll
   4 SubagentStopMatchGoalReminder
== subagent_id values in SubagentStopAll (b4b)
   1 "subagent_id":"01a21213-262b-7382-860e-0ed5de05f98a"
   1 "subagent_id":"01a21213-290c-7d33-ae3f-e918320fb9d0"
   1 "subagent_id":"01a21213-2b44-7cd3-89a6-95cbcaa32fe6"
   1 "subagent_id":"01a21213-2cc5-7e83-b265-9a8c2ee56d7e"
   1 "subagent_id":"01a21213-2e9d-7130-92e7-b4786415dec2"
   1 "subagent_id":"01a21213-30ed-7102-8754-b890bfdad10c"
   4 "subagent_id":"goal-reminder"
```

Answer:

- The child's request holds the agent **body in the first developer message**, right after the
  `subagent-delegation` system-reminder and before `# Parent task updates`; front matter is not
  included. The user message is `Role: <role>\nObjective: <objective>` from the spawn call.
- `tools` **restricts** the child: `["read_file"]` → offered `read_file` + `subagent_spawn`,
  `subagent_status`, `subagent_send_message`, `subagent_wait`, `subagent_read_result`,
  `subagent_cancel`, `work_status` (vs. the full set for a definition without `tools`).
  `effort: "high"` → `reasoning.effort: "high"`.
- Hook payloads in the child: `session_id` = the **child's** thread id (= `child_session_id` in
  SubagentStart/Stop), `turn_id` likewise; the hook process env has `MUSE_SESSION_ID` = the
  **parent** session and `MUSE_THREAD_ID` = the child thread. PreToolUse from the child has
  `tool_name: "read_file"` and **no agent name field**.
- SubagentStart/SubagentStop: `subagent_id` is the spawn-returned **UUID**
  (`01a21212-8ef2-…`), not the agent name; there is **no `agent_type`/name field** anywhere. Only
  the built-in reminder reports a name (`subagent_id: "goal-reminder"`). SubagentStop has
  `stop_hook_active: false` and `last_assistant_message: "MOCKDONE"` (the child's final text).
- Consequence: a `SubagentStart`/`SubagentStop` matcher `"core-coder"` **never fires** (b4b: 0
  `…MatchCoreCoder` dumps for 6 accepted pack-agent children), while matcher `"goal-reminder"` and an
  unmatched hook fire. Hooks cannot tell which pack agent is running.

## B5 — the model's `bash` tool: cwd and environment

Question: cwd of `bash` launched from the root; presence of MUSE_SESSION_ID, MUSE_THREAD_ID,
TMPDIR, HOME, AIRSSTACK_HOME.

Command: `items-run.sh $W/b5 b5-script.json $W/logs/b5 "probe bash"` with script
`[{"when":["probe bash"],"unless":["You are a goal reminder","\"call_id\":\"call_"],"name":"bash","args":{"command":"pwd; env | sort","description":"print cwd and env"}}]`
(`bash` requires `command` and `description`).

Raw: the tool output (next request's `function_call_output`), filtered by script to the asked
variables; the other 50-odd inherited variables of the agent's own shell (tokens included) are listed
by name only, values withheld:

```text
{"chunk_id":"exec-1-1","exit_code":0,"terminal_status":"completed","truncated":false}
output line 1 (pwd): $W/b5
AIRSSTACK_HOME=/probe/airs
HOME=$W/logs/b5.home.6nTw
LANG=en_US.UTF-8
MUSE_SESSION_ID=01a21213-6c45-7313-8738-724b024f1df0
MUSE_THREAD_ID=01a21213-6c45-7313-8738-724b024f1df0
MUSE_TOOL_USE_ID=call_1
PWD=$W/b5
SHELL=/bin/zsh
TERM=xterm-256color
TMPDIR=$W/logs/b5.home.6nTw/tmp
TRACEPARENT=00-01a2121373647a238ae0675c3ba3be7d-cc5c6c79d04d84c1-01
USER=hiraq
total env lines: 71
other names present (values withheld): __CF_USER_TEXT_ENCODING __CFBundleIdentifier _ AI_AGENT BUN_INSTALL CLAUDE_CODE_CHILD_NOTES_DIR CLAUDE_CODE_CHILD_SESSION CLAUDE_CODE_ENTRYPOINT CLAUDE_CODE_EXECPATH CLAUDE_CODE_MESSAGING_SOCKET CLAUDE_CODE_MESSAGING_TOKEN CLAUDE_CODE_SESSION_ATTENDED CLAUDE_CODE_SESSION_ID CLAUDE_CODE_TOOL_USE_ID CLAUDE_EFFORT CLAUDE_PID CLAUDECODE COLORTERM COMMAND_MODE COREPACK_ENABLE_AUTO_PIN CPPFLAGS GIT_EDITOR GRADLE_HOME HC_SCOPE HC_TERMINAL_ID HC_TERMINAL_SOCKET JAVA_HOME KOTLIN_HOME LDFLAGS LESS LOGNAME LS_COLORS LSCOLORS NIX_PROFILES NIX_SSL_CERT_FILE NoDefaultCurrentDirectoryInExePath NVM_BIN NVM_CD_FLAGS NVM_DIR NVM_INC OPENCODE_DISABLE_AUTOCOMPACT OPENCODE_DISABLE_PRUNE OPENROUTER_API_KEY OSLogRateLimit PAGER PATH SDKMAN_BROKER_API SDKMAN_CANDIDATES_API SDKMAN_CANDIDATES_DIR SDKMAN_DIR SDKMAN_PLATFORM SHLVL SSH_AUTH_SOCK TERM_PROGRAM WASMER_CACHE_DIR WASMER_DIR XDG_DATA_DIRS XPC_SERVICE_NAME ZSH
```

Answer: cwd = the workspace root (launch cwd). The `bash` tool **inherits the full launch
environment** (unlike hooks, which get an allowlist): `AIRSSTACK_HOME=/probe/airs`, `HOME`, `TMPDIR`
are the launch values; Muse adds `MUSE_SESSION_ID`, `MUSE_THREAD_ID` (equal in the lead),
`MUSE_TOOL_USE_ID=call_1` and `TRACEPARENT`. Surprise: `CLAUDE_CODE_TOOL_USE_ID=call_1` also equals
Muse's tool call id, so Muse appears to set it too (the outer Claude Code session's own value would
not be `call_1`). Muse printed `OPENROUTER_API_KEY=[REDACTED]` itself.

## B6 — `read_skill` on a skill with `references/`

Command: `items-run.sh $W/b6 b6-script.json $W/logs/b6 "probe readskill" --disable-reminders`.

Files:

```text
--- .agents/skills/core-ref/SKILL.md
---
name: "core-ref"
description: "Skill with a reference file"
---
REF-BODY Read references/note.md for details.
--- .agents/skills/core-ref/references/note.md
NOTE-CONTENT
--- mock script
[{"when":["probe readskill"],"unless":["You are a goal reminder","\"call_id\":\"call_"],"name":"read_skill","args":{"name":"core-ref"}}]
```

Raw:

```text
== read_skill tool output (b6 req-002)
### b6/req-002.json instructions[0..80]="You are Muse Code, an agentic coding CLI (command line interface) that helps use"
[1] type=message role=user name= len=15: "probe readskill"
[2] type=function_call role= name=muse.read_skill len=19: "{\"name\":\"core-ref\"}"
[3] type=function_call_output role= name= len=1143: "<read-skill-result name=\"core-ref\" status=\"ok\">\n<metadata>\npath: $W/b6/.agents/skills/core-ref/SKILL.md\nlocator: file://$W/b6/.agents/skills/core-ref/SKILL.md\nsource: project\nbytes_returned: 114\nbytes_total: 114\ntruncated: false\nnote: a skill body names entry points, not resolved behavior: before describing what named code does or ultimately runs, read that code and any saved record or setting that decides the outcome. Answer with the entry point and the concrete final result for this session in plain words. Do not replay the helpers, templates, or file citations in between, and never leave a placeholder where the final result belongs.\n</metadata>\n\n<skill-body id=\"core-ref\">\n---\nname: &quot;core-ref&quot;\ndescription: &quot;Skill with a reference file&quot;\n---\nREF-BODY Read references/note.md for details.\n\n</skill-body>\n</read-skill-result>"
```

Answer: `read_skill` returns `<read-skill-result name status="ok">` with a `<metadata>` block giving
the **absolute `path:`** of SKILL.md and a `locator: file://…` URL, plus `source`, byte counts and a
fixed note, then `<skill-body id=…>` with the whole file (front matter included, XML-escaped). There
is no explicit base-directory field, but the absolute SKILL.md path lets the model resolve
`references/note.md`. References are not listed or inlined. (The catalog and the slash-invoked
`skill-body` give only the relative `path=".agents/skills/<id>/SKILL.md"`.)

## B7 — can project `.claude/skills/` be turned off?

Docs (`docs/guides_extend_skills_.txt` lines 179–186, quoted by `sed -n`):

```text
    "foreign_personal_skills": false
  }
}
With that setting, muse skills list --source user lists only
$CONFIG_DIR/skills and $HOME/.agents/skills. The project-level
.codex/skills and .claude/skills directories have no switch. If you
would rather own a copy, muse skills import --from claude (or codex)
copies the personal skills into $CONFIG_DIR/skills (see
```

`grep -i foreign docs/*.txt` finds only that setting, `--no-foreign-personal-context` (exec help:
"Exclude foreign personal rules and skills from this run") and unrelated changelog lines.

Files: `$W/b7/.claude/skills/x/SKILL.md` = `name: x`, `description: Claude project skill x`, body
`X-BODY`. Commands (fresh HOME `$W/b7home` for `skills`; `items-run.sh` for exec):

```text
muse skills list --source project --trust-workspace                       # (a) default
# (b) settings.json + "context":{"foreign_personal_skills":false}
muse skills disable x --scope project --trust-workspace                   # (d) activation off
items-run.sh $W/b7 empty.json $W/logs/b7a "probe foreign" --disable-reminders                                  # exec default
items-run.sh $W/b7 empty.json $W/logs/b7c "probe foreign" --disable-reminders --no-foreign-personal-context    # (c)
items-run.sh $W/b7 empty.json $W/logs/b7e "probe foreign" --disable-reminders -c context.foreign_personal_skills=false   # (e)
items-run.sh $W/b7 empty.json $W/logs/b7f "probe foreign" --disable-reminders -c 'skills.activation={"projects":{"$W/b7":{".claude/skills/x/SKILL.md":"off"}}}'   # (f)
```

Raw:

```text
## (a) default
$ muse skills list --source project --trust-workspace
NAME	SCOPE	ACTIVATION	DESCRIPTION	PATH
x	project	on	Claude project skill x	.claude/skills/x/SKILL.md
## (b) settings.json context.foreign_personal_skills=false
{"schema_version":1,"endpoint_transport":{"base_url":"http://127.0.0.1:18782","auth":"bearer"},"context":{"foreign_personal_skills":false}}
$ muse skills list --source project --trust-workspace
NAME	SCOPE	ACTIVATION	DESCRIPTION	PATH
x	project	on	Claude project skill x	.claude/skills/x/SKILL.md
## (d) skills disable x --scope project
$ muse skills disable x --scope project --trust-workspace
x	project	off	$CONFIG_DIR/settings.json
{
  "schema_version": 1,
  "context": {
    "foreign_personal_skills": false
  },
  "skills": {
    "activation": {
      "projects": {
        "$W/b7": {
          ".claude/skills/x/SKILL.md": "off"
        }
      }
    }
  },
  "endpoint_transport": {
    "base_url": "http://127.0.0.1:18782",
    "auth": "bearer"
  }
}
$ muse skills list --source project --trust-workspace
NAME	SCOPE	ACTIVATION	DESCRIPTION	PATH
x	project	off	Claude project skill x	.claude/skills/x/SKILL.md
$ muse skills list --source project --trust-workspace --enabled-only
No skills found.
```

```text
== b7a
--- b7a/req-001.json @31331: …e.js 3D reference hub. Use when working with Three.js: pick the topic below and read its file for patterns and API guidance.</description>\n</skill>\n<skill id=\"x\" scope=\"project\" path=\".claude/skills/x/SKILL.md\" origin-family=\".claude\" written-for=\"Claude Code\">\n<description>Claude project skill x</des…
== b7c
--- b7c/req-001.json @31331: …e.js 3D reference hub. Use when working with Three.js: pick the topic below and read its file for patterns and API guidance.</description>\n</skill>\n<skill id=\"x\" scope=\"project\" path=\".claude/skills/x/SKILL.md\" origin-family=\".claude\" written-for=\"Claude Code\">\n<description>Claude project skill x</des…
== b7e
--- b7e/req-001.json @31331: …e.js 3D reference hub. Use when working with Three.js: pick the topic below and read its file for patterns and API guidance.</description>\n</skill>\n<skill id=\"x\" scope=\"project\" path=\".claude/skills/x/SKILL.md\" origin-family=\".claude\" written-for=\"Claude Code\">\n<description>Claude project skill x</des…
== b7f
--- b7f/req-001.json: no occurrence of "<skill id=\\\"x\\\""
--- b7f/req-001.json @31047: …ejs:threejs\" scope=\"plugin\" path=\"plugin://threejs/skills/threejs/SKILL.md\">\n<description>Three.js 3D reference hub. Use when working with Three.js: pick the topic below and read its file for patterns and API guidance.</description>\n</skill>\n</skill-catalog>\n</system-reminder>\n\n<system-reminder source=\"built-in\">\nDeliver what was asked where it was asked. Ask before changing the user's machine beyond it: putting your build on PATH, replacing or shadowing a command already there (even one that look…
```

Answer: **No project-level switch.** `context.foreign_personal_skills: false` and
`--no-foreign-personal-context` affect only the personal `~/.claude/skills`/Codex roots; project
`x` still lists and still reaches the model catalog, labelled `origin-family=".claude"
written-for="Claude Code"`. The only ways to hide it are per-skill activation `off`:
`muse skills disable x --scope project` (persists in the **user** `settings.json` under
`skills.activation.projects["<absolute project path>"][".claude/skills/x/SKILL.md"]`), or the same
key passed per run with `-c skills.activation=…` (b7f: `x` gone from the catalog). There is no
committed project settings file for this. A same-id `.agents/skills/x` shadows it (initial-results §7).

## B8 — parent directories and nested skill folders

Files: `$W/b8/parent/` is the git root with `.agents/skills/pskill/SKILL.md`;
`$W/b8/parent/proj/` (plain subdirectory) holds `.agents/skills/own/SKILL.md` and
`.agents/skills/a/b/SKILL.md` (name `nested`). Commands: `muse skills list --source project
--trust-workspace [--json]` from `proj/` and from `parent/`; the `--json` run is piped through `grep -i '"id"\|diagnos\|message'`, which also dropped its `$ …` header line.

Raw:

```text
files:
./parent/.agents/skills/pskill/SKILL.md
./parent/proj/.agents/skills/a/b/SKILL.md
./parent/proj/.agents/skills/own/SKILL.md
$ (cwd=$W/b8/parent/proj) muse skills list --source project --trust-workspace
NAME	SCOPE	ACTIVATION	DESCRIPTION	PATH
own	project	on	skill in proj	.agents/skills/own/SKILL.md
      "id": "own",
      "diagnostics": [],
  "diagnostics": []
$ (cwd=$W/b8/parent) muse skills list --source project --trust-workspace
NAME	SCOPE	ACTIVATION	DESCRIPTION	PATH
pskill	project	on	skill in parent dir	.agents/skills/pskill/SKILL.md
```

Answer: **Neither.** From `proj/` only `own` lists: the parent's `.agents/skills` is not scanned,
even though `parent/` is the git root (workspace root = launch cwd), and the nested
`.agents/skills/a/b/SKILL.md` is not found; no diagnostic for either. Skills must sit exactly at
`<launch cwd>/.agents/skills/<id>/SKILL.md`.
