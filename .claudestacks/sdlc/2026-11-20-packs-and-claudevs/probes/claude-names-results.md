# Probe: pack-prefixed project skill and agent names in Claude Code

Question: does Claude Code accept pack-prefixed project skill and agent names written into a project's `.claude/`?

- Date: 2026-11-23 09:17:07 UTC
- Binary: `/Users/hiraq/.local/bin/claude`
- Probe dir: a fresh `git init` repo in the session scratchpad (`claude-names/`)
- All four runs used `--model haiku --max-turns 3`; no flag was rejected. Each ran once; the four were started concurrently in the background and awaited.
- Noise: every run printed a sign-in expiry warning on stderr (captured below). No user-level plugin hook output appeared in the captured stdout/stderr.

## claude --version

~~~~text
2.1.330 (Claude Code)
~~~~

## Files created

### `.claude/skills/sdlc-status/SKILL.md`

~~~~text
---
name: sdlc-status
description: Probe skill. Use only when invoked by name.
---
Reply with exactly the text PROBE-SDLC-STATUS-OK and nothing else.
~~~~

### `.claude/skills/guideline-python-guidelines/SKILL.md`

~~~~text
---
name: guideline-python-guidelines
description: Probe skill. Use only when invoked by name.
---
Reply with exactly the text PROBE-GUIDELINE-OK and nothing else.
~~~~

### `.claude/agents/core-coder.md`

~~~~text
---
name: core-coder
description: Probe agent. Replies with a fixed marker.
tools: Read
model: haiku
---
Reply with exactly PROBE-CORE-CODER-OK and nothing else.
~~~~

## Q1: skill /sdlc-status

Command (run from the probe dir):

~~~~text
claude -p "/sdlc-status" --model haiku --max-turns 3 > runs/q1.out 2>&1; echo "exit=$?" >> runs/q1.out
~~~~

Output (`runs/q1.out`, stdout+stderr, then `exit=$?`):

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 5 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
PROBE-SDLC-STATUS-OK
exit=0
~~~~

## Q2: skill /guideline-python-guidelines

Command (run from the probe dir):

~~~~text
claude -p "/guideline-python-guidelines" --model haiku --max-turns 3 > runs/q2.out 2>&1; echo "exit=$?" >> runs/q2.out
~~~~

Output (`runs/q2.out`, stdout+stderr, then `exit=$?`):

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 5 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
PROBE-GUIDELINE-OK
exit=0
~~~~

## Q3: agent core-coder

Command (run from the probe dir):

~~~~text
claude -p "Use the core-coder subagent (Task/Agent tool, subagent_type core-coder) and print its reply verbatim." --allowedTools "Task,Agent" --model haiku --max-turns 3 > runs/q3.out 2>&1; echo "exit=$?" >> runs/q3.out
~~~~

Output (`runs/q3.out`, stdout+stderr, then `exit=$?`):

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 5 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
The core-coder probe is still running. I'll print its reply verbatim when the completion notification arrives.

The core-coder subagent replied:

```
PROBE-CORE-CODER-OK
```
exit=0
~~~~

## Q4: list project skills

Command (run from the probe dir):

~~~~text
claude -p "List the names of the project skills available to you (from .claude/skills), one per line, nothing else." --model haiku --max-turns 3 > runs/q4.out 2>&1; echo "exit=$?" >> runs/q4.out
~~~~

Output (`runs/q4.out`, stdout+stderr, then `exit=$?`):

~~~~text
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 5 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
guideline-python-guidelines
sdlc-status
exit=0
~~~~

## Result

| Run | Expected | Seen | Verdict |
|---|---|---|---|
| Q1 | PROBE-SDLC-STATUS-OK | PROBE-SDLC-STATUS-OK, exit 0 | accepted |
| Q2 | PROBE-GUIDELINE-OK | PROBE-GUIDELINE-OK, exit 0 | accepted |
| Q3 | PROBE-CORE-CODER-OK | subagent reply PROBE-CORE-CODER-OK, exit 0 | accepted |
| Q4 | both skill names | guideline-python-guidelines, sdlc-status, exit 0 | both listed |

Claude Code 2.1.330 accepts hyphenated pack-prefixed names (`<pack>-<item>`) for project skills (as slash commands) and project agents (as subagent_type).
