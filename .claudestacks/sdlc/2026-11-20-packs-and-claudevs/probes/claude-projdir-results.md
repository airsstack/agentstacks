# ${CLAUDE_PROJECT_DIR} in a project skill

Run 2026-11-23 in /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/claude-names. Output pasted verbatim.

~~~~text
$ claude --version
2.1.330 (Claude Code)
$ cat .claude/skills/sdlc-projdir/SKILL.md
---
name: sdlc-projdir
description: Probe skill. Use only when invoked by name.
---
Reply with exactly this line and nothing else: DIR=${CLAUDE_PROJECT_DIR}
$ claude -p "/sdlc-projdir" --model haiku --max-turns 6
Warning: this sign-in ends between 2026-11-28 and 2026-12-01 (in 5 days at the earliest). Renewing it does not move those dates. Run claude auth login before then, or use claude setup-token for unattended jobs.
DIR=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/claude-names
exit=0
$ pwd -P
/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/claude-names
~~~~

Note: a first run with `--max-turns 2` ended `Error: Reached max turns (2)` with exit 1; the run above used `--max-turns 6`.
