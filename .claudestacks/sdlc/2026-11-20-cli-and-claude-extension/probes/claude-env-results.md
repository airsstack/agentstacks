# Env names a Claude Code hook sees (Claude Code 2.1.330, 2026-11-22)

Project: scratchpad/claude-probe (settings restored afterwards).

`.claude/settings.json`:
```json
{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"cd \"$CLAUDE_PROJECT_DIR\" && env | grep -E '^(CLAUDE_|AIRSSTACK_HOME|TMPDIR|HOME)' | sed 's/=.*//' | sort > .claude/env.log # agentstacks"}]}]}}
```

Command: `claude -p --model haiku "Reply ok." < /dev/null`

Reply: `ok` (the trailing ` # agentstacks` shell comment did not break the command)

`.claude/env.log`:
```
CLAUDE_CODE_CHILD_NOTES_DIR
CLAUDE_CODE_CHILD_SESSION
CLAUDE_CODE_ENTRYPOINT
CLAUDE_CODE_EXECPATH
CLAUDE_CODE_MESSAGING_SOCKET
CLAUDE_CODE_MESSAGING_TOKEN
CLAUDE_CODE_SESSION_ATTENDED
CLAUDE_CODE_SESSION_ID
CLAUDE_EFFORT
CLAUDE_ENV_FILE
CLAUDE_PID
CLAUDE_PROJECT_DIR
HOME
TMPDIR
```

Findings: hooks get `CLAUDE_CODE_SESSION_ID`, not `CLAUDE_SESSION_ID`; `AIRSSTACK_HOME` is not set here.

Plugin default for AIRSSTACK_HOME (`../claudestacks/plugins/claudestacks/scripts/discuss.lua:57`):
`local root = read_env("AIRSSTACK_HOME") or path.join(read_env("HOME") or "", ".airsstack")`
