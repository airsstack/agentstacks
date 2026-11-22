# Claude Code behaviour probes

Run 2026-11-22. Scratch project (left in place): `/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe`

Every block below is pasted verbatim from the run with `cat`. Each run's stdout and stderr were captured together with `> runs/<q>.out 2>&1`, and `exit=$?` was appended afterwards.

## Version

~~~~text
$ claude --version
2.1.330 (Claude Code)
~~~~

## Setup

The machine's user-level plugins also fired their own hooks during these runs (a SessionStart "snapshot-load" reminder and a "Reply rules" additionalContext on SessionStart and UserPromptSubmit). They are not part of this project's config. They show up in the Q5 and Q6 replies.

### .claude/settings.json (base; each question swaps one hook command, then restores this)

~~~~text
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && sh .claude/dump.sh SessionStart"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && sh .claude/dump.sh UserPromptSubmit"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Read|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && sh .claude/dump.sh PreToolUse"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Write",
        "hooks": [
          {
            "type": "command",
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && sh .claude/dump.sh PostToolUse"
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "matcher": "probe-agent",
        "hooks": [
          {
            "type": "command",
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && sh .claude/dump.sh SubagentStop"
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && sh .claude/dump.sh SessionEnd"
          }
        ]
      }
    ]
  }
}
~~~~

### .claude/dump.sh

~~~~text
#!/bin/sh
{
  echo "== $1 pwd=$(pwd) =="
  echo "CLAUDE_PROJECT_DIR=$CLAUDE_PROJECT_DIR"
  if [ -n "${TMPDIR+x}" ]; then echo "TMPDIR=set"; else echo "TMPDIR=unset"; fi
  echo "PPID_COMM=$(ps -o comm= -p $PPID)"
  cat
  echo
} >> .claude/payloads.log
~~~~

### .claude/agents/probe-agent.md

~~~~text
---
name: probe-agent
description: "Probe agent: replies PONG"
tools: Read
model: haiku
effort: low
---
Reply with exactly PONG.
~~~~

### .claude/skills/probe-skill/SKILL.md

~~~~text
---
name: "probe-skill"
description: "Probe skill: echoes \"args\" and the session id"
disable-model-invocation: true
argument-hint: "[text]"
---
Reply with exactly: ARGS=$ARGUMENTS SID=${CLAUDE_SESSION_ID}
~~~~

### hello.txt

~~~~text
hello from probe
~~~~

## Q1: hook payloads, SubagentStop, SessionEnd

Command (run in the scratch project):

~~~~sh
claude -p --model haiku --permission-mode bypassPermissions "Read the file hello.txt, then write the text done to out.txt, then use the probe-agent subagent and report its reply."
~~~~

Output:

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 6 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
Done so far:

- Read `hello.txt`: it contains `hello from probe`.
- Wrote `out.txt` with the text `done`.
- Launched `probe-agent`. It's running in the background, so its reply isn't back yet. I'll report it when the completion notification arrives.

probe-agent replied: **PONG**
exit=0
~~~~

`.claude/payloads.log` (full):

~~~~text
== SessionStart pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"85bfb39f-d609-4929-a5de-526e43e8c946","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/85bfb39f-d609-4929-a5de-526e43e8c946.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","hook_event_name":"SessionStart","source":"startup"}

== UserPromptSubmit pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"85bfb39f-d609-4929-a5de-526e43e8c946","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/85bfb39f-d609-4929-a5de-526e43e8c946.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","prompt_id":"7c43fb5a-2be3-455c-98f5-6abf381826e3","permission_mode":"bypassPermissions","hook_event_name":"UserPromptSubmit","prompt":"Read the file hello.txt, then write the text done to out.txt, then use the probe-agent subagent and report its reply."}

== PreToolUse pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"85bfb39f-d609-4929-a5de-526e43e8c946","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/85bfb39f-d609-4929-a5de-526e43e8c946.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","prompt_id":"7c43fb5a-2be3-455c-98f5-6abf381826e3","permission_mode":"bypassPermissions","effort":{"level":"medium"},"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe/hello.txt"},"tool_use_id":"toolu_014nJrv1V22rQeMz5HW74r96"}

== PreToolUse pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"85bfb39f-d609-4929-a5de-526e43e8c946","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/85bfb39f-d609-4929-a5de-526e43e8c946.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","prompt_id":"7c43fb5a-2be3-455c-98f5-6abf381826e3","permission_mode":"bypassPermissions","effort":{"level":"medium"},"hook_event_name":"PreToolUse","tool_name":"Write","tool_input":{"file_path":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe/out.txt","content":"done\n"},"tool_use_id":"toolu_01SwXqmmMiB8mv3Y7c8kNE1b"}

== PostToolUse pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"85bfb39f-d609-4929-a5de-526e43e8c946","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/85bfb39f-d609-4929-a5de-526e43e8c946.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","prompt_id":"7c43fb5a-2be3-455c-98f5-6abf381826e3","permission_mode":"bypassPermissions","effort":{"level":"medium"},"hook_event_name":"PostToolUse","tool_name":"Write","tool_input":{"file_path":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe/out.txt","content":"done\n"},"tool_response":{"type":"create","filePath":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe/out.txt","content":"done\n","structuredPatch":[],"originalFile":null,"userModified":false},"tool_use_id":"toolu_01SwXqmmMiB8mv3Y7c8kNE1b","duration_ms":6}

== SubagentStop pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"85bfb39f-d609-4929-a5de-526e43e8c946","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/85bfb39f-d609-4929-a5de-526e43e8c946.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","prompt_id":"7c43fb5a-2be3-455c-98f5-6abf381826e3","permission_mode":"bypassPermissions","agent_id":"ac19ddd05d66550cf","agent_type":"probe-agent","effort":{"level":"low"},"hook_event_name":"SubagentStop","stop_hook_active":false,"agent_transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/85bfb39f-d609-4929-a5de-526e43e8c946/subagents/agent-ac19ddd05d66550cf.jsonl","last_assistant_message":"PONG","background_tasks":[{"id":"ac19ddd05d66550cf","type":"subagent","status":"running","description":"Run probe-agent check","agent_type":"probe-agent"}],"session_crons":[]}

== UserPromptSubmit pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"85bfb39f-d609-4929-a5de-526e43e8c946","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/85bfb39f-d609-4929-a5de-526e43e8c946.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","prompt_id":"a3fa6ccd-a5c0-4834-8cae-9854b408fe31","permission_mode":"bypassPermissions","hook_event_name":"UserPromptSubmit","prompt":"<task-notification>\n<task-id>ac19ddd05d66550cf</task-id>\n<tool-use-id>toolu_01Nf411N5YvxDQyz6Bb2vWgG</tool-use-id>\n<output-file>/private/tmp/claude-501/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/85bfb39f-d609-4929-a5de-526e43e8c946/tasks/ac19ddd05d66550cf.output</output-file>\n<status>completed</status>\n<summary>Agent \"Run probe-agent check\" finished</summary>\n<note>A task-notification fires each time this agent stops with no live background children of its own. The user can send it another message and resume it, so the same task-id may notify more than once.</note>\n<result>PONG</result>\n<usage><subagent_tokens>3312</subagent_tokens><tool_uses>0</tool_uses><duration_ms>2129</duration_ms></usage>\n</task-notification>"}

== SessionEnd pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"85bfb39f-d609-4929-a5de-526e43e8c946","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/85bfb39f-d609-4929-a5de-526e43e8c946.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","prompt_id":"a3fa6ccd-a5c0-4834-8cae-9854b408fe31","hook_event_name":"SessionEnd","reason":"other"}

~~~~

Answers:

- **Fields per event.** Every event has `session_id`, `transcript_path`, `cwd` and `hook_event_name`.
  - SessionStart adds `source` ("startup").
  - UserPromptSubmit adds `prompt_id`, `permission_mode` and `prompt`.
  - PreToolUse adds `prompt_id`, `permission_mode`, `effort{level}`, `tool_name`, `tool_input` and `tool_use_id`.
  - PostToolUse has the PreToolUse fields plus `tool_response` and `duration_ms`.
  - SubagentStop adds `prompt_id`, `permission_mode`, `agent_id`, `agent_type`, `effort`, `stop_hook_active`, `agent_transcript_path`, `last_assistant_message`, `background_tasks` and `session_crons`.
  - SessionEnd adds `prompt_id` and `reason` ("other").
- **Subagent name.** SubagentStop carries the subagent name in `agent_type`, with the value `"probe-agent"`. Its `agent_id` is a hex id.
- **SubagentStop matcher.** The SubagentStop hook with matcher `probe-agent` fired.
- **Paths.** PreToolUse `tool_input.file_path` is an absolute path, for both Read and Write.
- **Working directory.** The hook's pwd is the project dir. `CLAUDE_PROJECT_DIR` is set to the project dir, `TMPDIR` is set, and the parent process is `/bin/sh`.
- **SessionEnd in -p mode.** SessionEnd fires in -p mode, with `reason: "other"`.
- **Background subagent.** The subagent ran in the background. Its completion came back as a second UserPromptSubmit carrying a `<task-notification>` prompt, so UserPromptSubmit fired twice in one -p run.

## Q2: skill with JSON-quoted frontmatter, $ARGUMENTS, ${CLAUDE_SESSION_ID}

~~~~sh
claude -p --model haiku --permission-mode bypassPermissions "/probe-skill hello world"
~~~~

Output:

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 6 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
ARGS=hello world SID=d6590e86-290b-4a4f-8032-aa89d4d6fae1
exit=0
~~~~

`.claude/payloads.log` for this run:

~~~~text
== SessionStart pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"d6590e86-290b-4a4f-8032-aa89d4d6fae1","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/d6590e86-290b-4a4f-8032-aa89d4d6fae1.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","hook_event_name":"SessionStart","source":"startup"}

== UserPromptSubmit pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"d6590e86-290b-4a4f-8032-aa89d4d6fae1","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/d6590e86-290b-4a4f-8032-aa89d4d6fae1.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","prompt_id":"6ddcff0b-e89a-415a-98a0-fd3d4541aea6","permission_mode":"bypassPermissions","hook_event_name":"UserPromptSubmit","prompt":"/probe-skill hello world"}

== SessionEnd pwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe ==
CLAUDE_PROJECT_DIR=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe
TMPDIR=set
PPID_COMM=/bin/sh
{"session_id":"d6590e86-290b-4a4f-8032-aa89d4d6fae1","transcript_path":"/Users/hiraq/.claude/projects/-private-tmp-claude-501--Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext-d352388e-4150-435e-8b29-db273ed91466-scratchpad-claude-probe/d6590e86-290b-4a4f-8032-aa89d4d6fae1.jsonl","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","prompt_id":"6ddcff0b-e89a-415a-98a0-fd3d4541aea6","hook_event_name":"SessionEnd","reason":"other"}

~~~~

Answer: the skill loads with every frontmatter string JSON-double-quoted, including an escaped `\"` inside the description. `$ARGUMENTS` became `hello world`, and `${CLAUDE_SESSION_ID}` became `d6590e86-290b-4a4f-8032-aa89d4d6fae1`, the same as `session_id` in that run's payload log.

## Q3: PreToolUse blocking

### Q3a: hookSpecificOutput permissionDecision deny

PreToolUse hook command: `cd "$CLAUDE_PROJECT_DIR" && sh .claude/deny.sh`

~~~~text
#!/bin/sh
cat >/dev/null
printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"probe says no"}}'
exit 0
~~~~

~~~~sh
claude -p --model haiku --permission-mode bypassPermissions "Read hello.txt and print its content, or explain why you could not."
~~~~

Output:

~~~~text
Warning: no stdin data received in 3s, proceeding without it. If piping from a slow command, redirect stdin explicitly: < /dev/null to skip, or wait longer.
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 6 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
I couldn't read `hello.txt`. A PreToolUse hook blocked the Read with the error `probe says no`. I didn't try another route like `cat` through Bash, because that would get around the block.

To get the file's contents, the hook's rule needs to allow reading this path, or you can confirm that reading it through Bash is fine and I'll do that.
exit=0
~~~~

### Q3b: older form {"decision":"block"}

PreToolUse hook command: `cd "$CLAUDE_PROJECT_DIR" && sh .claude/block.sh`

~~~~text
#!/bin/sh
cat >/dev/null
printf '%s\n' '{"decision":"block","reason":"probe says no"}'
exit 0
~~~~

~~~~sh
claude -p --model haiku --permission-mode bypassPermissions "Read hello.txt and print its content, or explain why you could not."
~~~~

Output:

~~~~text
Warning: no stdin data received in 3s, proceeding without it. If piping from a slow command, redirect stdin explicitly: < /dev/null to skip, or wait longer.
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 6 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
I couldn't read `hello.txt`. A PreToolUse hook blocked the Read with the error `probe says no`. I haven't tried another way to read the file, because that would route around the block.

The hook is a guard in this environment, so the next step is yours. You can check which hook is returning `probe says no` (it's likely in `.claude/settings*.json` or a hook script), and either adjust it or tell me it's fine to read the file another way.
exit=0
~~~~

The Read `tool_result` in each run's transcript JSONL, pulled with grep. Q3a is session c3d7ad9d-200e-403f-945e-cc509e706cb5 and Q3b is 493a0cf9-a856-4bd1-a216-8f021bcbb37e:

~~~~text
== q3a c3d7ad9d-200e-403f-945e-cc509e706cb5
"content":"PreToolUse:Read hook error: probe says no","is_error":true
== q3b 493a0cf9-a856-4bd1-a216-8f021bcbb37e
"content":"PreToolUse:Read hook error: probe says no","is_error":true
~~~~

Answer: both forms block the Read, even under `--permission-mode bypassPermissions`. The model sees the same result for both, `PreToolUse:Read hook error: probe says no` (is_error true). The CLI printed a "no stdin data received in 3s" warning on these two runs, because stdin was not redirected. Later runs add `< /dev/null`.

## Q4: SessionStart context injection

### Q4a: hookSpecificOutput.additionalContext

SessionStart hook command: `cd "$CLAUDE_PROJECT_DIR" && sh .claude/ctx-json.sh`

~~~~text
#!/bin/sh
cat >/dev/null
printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"The magic word is PLUM."}}'
~~~~

~~~~sh
claude -p --model haiku "What is the magic word? One word." < /dev/null
~~~~

Output:

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 6 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
PLUM
exit=0
~~~~

### Q4b: plain-text stdout

SessionStart hook command: `cd "$CLAUDE_PROJECT_DIR" && sh .claude/ctx-text.sh`

~~~~text
#!/bin/sh
cat >/dev/null
echo 'The magic word is FIG.'
~~~~

~~~~sh
claude -p --model haiku "What is the magic word? One word." < /dev/null
~~~~

Output:

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 6 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
FIG

This came from the session-start hook output, not from anything I could verify independently.
exit=0
~~~~

Answer: SessionStart reaches the model either way, through JSON `additionalContext` (PLUM) or through plain stdout (FIG).

## Q5: UserPromptSubmit plain text

UserPromptSubmit hook command (as stored in settings.json):

~~~~text
echo 'Reminder: it'"'"'s a test'
~~~~

~~~~sh
claude -p --model haiku "Repeat any reminder you were given, verbatim, or say NONE." < /dev/null
~~~~

Output:

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 6 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
**Reminders (verbatim):**

**1. SessionStart hook:**
> Reminder: run /claudestacks:snapshot-load for the current git branch to rehydrate relevant project memory before starting work.

**2. SessionStart hook context:**
> ## Reply rules
>
> 1. Reduce agent verbosity output.
> 2. Do not over-explain everything; only explain what matters.
> 3. Use ASCII visualizations to replace long narratives or text.
> 4. Be concise but precise; only tell what matters and is important.
>
> Applied as:
>
> - The outcome comes first.
> - Anything that would run past a short paragraph becomes an ASCII diagram, table or tree.
> - While a discussion is open, a reply ends with its topic list.

**3. UserPromptSubmit hook:**
> Reminder: it's a test

**4. UserPromptSubmit hook context:** same text as item 2.

**5. Global CLAUDE.md** (`/Users/hiraq/.claude/CLAUDE.md`) and the **SDLC gates / parallel work** rules that accompany it: these are long, so I haven't reproduced them here. I can paste them in full if you want.

**6. Environment, skills list, and agent list:** these are long system context blocks. I haven't reproduced them either.

**7. User email and git status:** the email is redacted here. The git status snapshot shows branch `main` with untracked `.claude/`, `hello.txt`, `out.txt`, and `runs/`.
exit=0
~~~~

Answer: the model receives UserPromptSubmit's plain stdout verbatim ("Reminder: it's a test"). Items 1, 2 and 4 in the reply come from user-level plugins on this machine, not from the probe.

## Q6: PostToolUse(Write) block

PostToolUse hook command: `cd "$CLAUDE_PROJECT_DIR" && sh .claude/post-block.sh`

~~~~text
#!/bin/sh
cat >/dev/null
printf '%s\n' '{"decision":"block","reason":"probe post says fix it"}'
~~~~

### Q6 run 1 (as specified)

~~~~sh
claude -p --model haiku --permission-mode bypassPermissions "Write ok to a.txt, then tell me any feedback you got from hooks verbatim." < /dev/null
~~~~

Output:

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 6 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
Wrote `ok` to `a.txt` (the file contains `ok` plus a newline).

Hook feedback, verbatim:

**1. SessionStart:startup hook success:**
```
Reminder: run /claudestacks:snapshot-load for the current git branch to rehydrate relevant project memory before starting work.
```

**2. SessionStart hook additional context:**
```
## Reply rules

1. Reduce agent verbosity output.
2. Do not over-explain everything; only explain what matters.
3. Use ASCII visualizations to replace long narratives or text.
4. Be concise but precise; only tell what matters and is important.

Applied as:

- The outcome comes first.
- Anything that would run past a short paragraph becomes an ASCII diagram, table or tree.
- While a discussion is open, a reply ends with its topic list.
```

**3. UserPromptSubmit hook additional context:** the same "Reply rules" block as item 2, reproduced word for word.

I have not run `/claudestacks:snapshot-load`, since you didn't ask for it.
exit=0
~~~~

This run was inconclusive. The transcript (session 2ec31bf8-ed18-432a-ae53-2aefa1b1a511) shows the model wrote the file with the **Bash** tool (`printf 'ok
' > a.txt && cat a.txt`), not Write, so the `Write` matcher never fired. I used the one allowed retry with a prompt that names the Write tool.

### Q6 run 2 (retry)

~~~~sh
claude -p --model haiku --permission-mode bypassPermissions "Use the Write tool to write ok to a.txt, then tell me any feedback you got from hooks verbatim." < /dev/null
~~~~

Output:

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 6 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
`a.txt` now contains `ok` (with a trailing newline). The Write tool reported success.

The PostToolUse hook then blocked the call with this feedback, verbatim:

```
PostToolUse:Write hook blocking error from command: "cd "$CLAUDE_PROJECT_DIR" && sh .claude/post-block.sh": probe post says fix it
```

I haven't run `.claude/post-block.sh` or changed anything in response. The message says only "fix it" and doesn't say what is wrong. If you want me to act on it, tell me what to fix or whether to run the script.

Earlier hook feedback, also verbatim:

**SessionStart:startup hook success:**
```
Reminder: run /claudestacks:snapshot-load for the current git branch to rehydrate relevant project memory before starting work.
```

**SessionStart and UserPromptSubmit hook context (both identical):**
```
## Reply rules

1. Reduce agent verbosity output.
2. Do not over-explain everything; only explain what matters.
3. Use ASCII visualizations to replace long narratives or text.
4. Be concise but precise; only tell what matters and is important.

Applied as:

- The outcome comes first.
- Anything that would run past a short paragraph becomes an ASCII diagram, table or tree.
- While a discussion is open, a reply ends with its topic list.
```
exit=0
~~~~

The transcript attachment for the block (session 9b8bf5a7-a51e-461f-90ee-5f21e5de54c1), first grep hit, cut to 1200 characters:

~~~~text
{"parentUuid":"d5f11e2b-1683-4325-b5ad-af2d0166bf35","isSidechain":false,"attachment":{"type":"hook_blocking_error","hookName":"PostToolUse:Write","toolUseID":"toolu_01DFbgCRwmJhjCJ9MaSPpStr","hookEvent":"PostToolUse","blockingError":{"blockingError":"probe post says fix it","command":"cd \"$CLAUDE_PROJECT_DIR\" && sh .claude/post-block.sh"}},"type":"attachment","uuid":"7270409c-3e88-476b-8340-2b1da1e97566","timestamp":"2026-11-22T11:33:56.390Z","rendered":[{"content":"<system-reminder>\nPostToolUse:Write hook blocking error from command: \"cd \"$CLAUDE_PROJECT_DIR\" && sh .claude/post-block.sh\": probe post says fix it\n</system-reminder>"}],"renderedRole":"system","userType":"external","entrypoint":"sdk-cli","cwd":"/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/claude-probe","sessionId":"9b8bf5a7-a51e-461f-90ee-5f21e5de54c1","version":"2.1.330","gitBranch":"main","dataFlowOn":true}
~~~~

Answer: on PostToolUse, `{"decision":"block","reason":...}` does not undo the write (a.txt contains `ok`). The model gets the reason as a system-reminder: `PostToolUse:Write hook blocking error from command: "<command>": probe post says fix it`.

## One-line answers

- Q1: All six events fire in -p mode, SessionEnd included (reason "other"). SubagentStop names the subagent in `agent_type` ("probe-agent"), and the `probe-agent` matcher fires. `tool_input.file_path` is absolute. The hook's pwd and CLAUDE_PROJECT_DIR are both the project dir.
- Q2: The skill loads with JSON-quoted frontmatter. `$ARGUMENTS` became "hello world" and `${CLAUDE_SESSION_ID}` became the real session_id.
- Q3: Both the permissionDecision "deny" form and the older decision "block" form block a Read under bypassPermissions. The model sees "PreToolUse:Read hook error: probe says no" for both.
- Q4: SessionStart context reaches the model through JSON additionalContext (PLUM) and through plain stdout (FIG).
- Q5: The model receives UserPromptSubmit's plain stdout verbatim.
- Q6: PostToolUse block does not undo the write. The model receives the reason as "PostToolUse:Write hook blocking error from command: ...: probe post says fix it". This needed the one retry, because run 1 used Bash instead of Write.
