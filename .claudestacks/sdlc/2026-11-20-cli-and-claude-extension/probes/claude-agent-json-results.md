# Agent frontmatter written as JSON values (Claude Code 2.1.330, 2026-11-22)

`.claude/agents/json-agent.md`:
```
---
name: "json-agent"
description: "JSON-frontmatter probe agent: replies with its tool list"
tools: ["Read", "Bash"]
model: "haiku"
effort: "low"
---

List the names of the tools you have, comma-separated, nothing else.
```

Command: `claude -p --model haiku --permission-mode bypassPermissions "Use the json-agent subagent and report its reply verbatim." < /dev/null`

Output (sign-in warning line removed):
```
The json-agent is still running in the background. I'll relay its reply verbatim when the completion notification arrives.

json-agent replied with its tool list, verbatim:

```
Read, Bash
```
```

Finding: the agent loads with JSON-quoted strings and a JSON flow list for `tools`, and the list restricts its tools to Read and Bash.
