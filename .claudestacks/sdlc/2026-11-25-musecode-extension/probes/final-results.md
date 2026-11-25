# Final probes — muse 1.4.25 (1.4.25-R9361.1), 2026-11-25

Run: `probes/harness/final-run.sh <binary> <scratch>`; W = the scratch dir. Every muse run sets TBH_CREDENTIAL_BACKEND=file (R4).

```text
### R1 — hook command wrapped in /bin/sh -c, under three login shells
$ cat .muse/hooks.json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          {
            "type": "command",
            "command": "/bin/sh -c '\"printf\" \"%s|%s|%s\\n\" \"${TMPDIR:-/tmp}/x\" \"it'\\''s\" \"$(pwd)\" > r1-out.txt' # agentstacks"
          }
        ]
      }
    ]
  }
}
$ SHELL=/bin/zsh TBH_CREDENTIAL_BACKEND=file muse exec --provider echo --trust-workspace r1
muse: workspace root: W/r1/proj (cwd default)
muse: workspace trust: trusted source=run-flag
echo: r1
$ cat r1-out.txt
W/r1/tmp/x|it's|W/r1/proj
$ SHELL=/bin/sh TBH_CREDENTIAL_BACKEND=file muse exec --provider echo --trust-workspace r1
muse: workspace root: W/r1/proj (cwd default)
muse: workspace trust: trusted source=run-flag
echo: r1
$ cat r1-out.txt
W/r1/tmp/x|it's|W/r1/proj
$ SHELL=/bin/bash TBH_CREDENTIAL_BACKEND=file muse exec --provider echo --trust-workspace r1
muse: workspace root: W/r1/proj (cwd default)
muse: workspace trust: trusted source=run-flag
echo: r1
$ cat r1-out.txt
W/r1/tmp/x|it's|W/r1/proj

### R2 — SubagentStop {decision: block} (first stop only)
$ cat stop.sh
#!/bin/sh
n=$(ls "$1" | wc -l | tr -d " "); cat > "$1/stop-$n.json"
if [ "$n" = 0 ]; then printf '{"decision":"block","reason":"KEEPGOING-R2"}\n'; fi
$ TBH_CREDENTIAL_BACKEND=file muse exec --provider echo --trust-workspace r2
muse: workspace root: W/r2/proj (cwd default)
muse: workspace trust: trusted source=run-flag
echo: r2
exit=0
$ per stop payload: file subagent_id stop_hook_active
stop-0.json goal-reminder False
stop-1.json goal-reminder True
stop-2.json goal-reminder True
stop-3.json goal-reminder True
stop-4.json goal-reminder True
```

### R4 — macOS Keychain dialog under an isolated HOME

- Without the variable, every `muse exec` run whose isolated HOME holds `auth.json` with a `meta`
  key showed the macOS dialog "Keychain Not Found — A keychain cannot be found to store “meta.”"
  (Cancel / Reset To Defaults), reported by the author with a screenshot on 2026-11-25.
- The binary names the variable: `strings muse-bin-1.4.25-R9361.1 | grep -o 'TBH_CREDENTIAL_BACKEND'`
  prints `TBH_CREDENTIAL_BACKEND` (next to `META_API_KEY` in the same string table).
- One run at 16:43:03–16:43:06 with `TBH_CREDENTIAL_BACKEND=file`, HOME holding the same `auth.json`
  (`{"schema_version":1,"providers":{"meta":{"api_key":"dummy"}}}`): the author reported "no dialog
  appeared". Every later probe (R1, R2) set it too; no dialog was reported.
- Caution recorded with that run: its mock failed to bind port 18777 (its log directory was never
  created) and a reply came from an unknown listener. The check harness therefore must confirm the
  mock is listening before starting `muse`.

Answer: `TBH_CREDENTIAL_BACKEND=file` stops the dialog; the check sets it for every run.

### R3 — a message hook in the printf form reaches the model

```text
$ cat .muse/hooks.json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          {
            "type": "command",
            "command": "printf '%s\\n' '{\"hookSpecificOutput\":{\"additionalContext\":\"MSG-R3 it'\"'\"'s \\\"q\\\"\",\"hookEventName\":\"SessionStart\"}}' # agentstacks"
          }
        ]
      }
    ]
  }
}
$ SHELL=/bin/zsh TBH_CREDENTIAL_BACKEND=file muse exec --trust-workspace --yolo r3
muse: workspace root: W/r3/proj (cwd default)
muse: workspace trust: trusted source=run-flag
MOCKDONE
$ first request: context around MSG-R3
 unless the user writes in or asks for another language.\n\nMSG-R3 it's \"q\"\n\n<system-reminder so
/Users/hiraq/.herdr/worktrees/agentstacks/extension-musecode/.claudestacks/sdlc/2026-11-25-musecode-extension/probes/harness/msg-run.sh: line 21: 75792 Terminated: 15          node "$HERE/mock.mjs" "$P" $R/script.json $R/log
```

# Final probes, part 2 — R5-R10 (`probes/harness/final2-run.sh <binary> <scratch>`, commands built by `probes/harness/gen.lua`)

```text
### R5 — PreToolUse additionalContext, alone and with deny
$ cat .muse/hooks.json
{"hooks":{"PreToolUse":[{"matcher":"Read","hooks":[{"type":"command","command":"W/r5/ctx.sh"}]},{"matcher":"Write","hooks":[{"type":"command","command":"W/r5/deny.sh"}]}]}}
$ muse exec --trust-workspace --yolo "probe r5"
muse: workspace root: W/r5/proj (cwd default)
muse: workspace trust: trusted source=run-flag
MOCKDONE
exit=0
$ b.txt
ls: W/r5/proj/b.txt: No such file or directory
$ requests holding PRE-CTX
req-002.json 'PRE-CTX'
req-003.json 'PRE-CTX'
req-004.json 'txt`.\n1|hello</tool-result>\n  </message>\n  <message role="developer">\n    <text>PRE-CTX</text>\n  </message>'
req-005.json 'txt`.\n1|hello</tool-result>\n  </message>\n  <message role="developer">\n    <text>PRE-CTX</text>\n  </message>'
req-006.json 'txt`.\n1|hello</tool-result>\n  </message>\n  <message role="developer">\n    <text>PRE-CTX</text>\n  </message>'
req-007.json 'txt`.\n1|hello</tool-result>\n  </message>\n  <message role="developer">\n    <text>PRE-CTX</text>\n  </message>'
hits: 6
$ requests holding PRE-REASON
req-003.json 'tool blocked by hook: PRE-REASON'
req-004.json 'ge>\n  <message role="tool">\n    <tool-result call-id="c2">tool blocked by hook: PRE-REASON</tool-result>\n  </m'
req-005.json 'ge>\n  <message role="tool">\n    <tool-result call-id="c2">tool blocked by hook: PRE-REASON</tool-result>\n  </m'
req-006.json 'ge>\n  <message role="tool">\n    <tool-result call-id="c2">tool blocked by hook: PRE-REASON</tool-result>\n  </m'
req-007.json 'ge>\n  <message role="tool">\n    <tool-result call-id="c2">tool blocked by hook: PRE-REASON</tool-result>\n  </m'
hits: 5
$ requests holding PRE-DENY-CTX
req-003.json 'PRE-DENY-CTX'
req-004.json 'k: PRE-REASON</tool-result>\n  </message>\n  <message role="developer">\n    <text>PRE-DENY-CTX</text>\n  </message>'
req-005.json 'k: PRE-REASON</tool-result>\n  </message>\n  <message role="developer">\n    <text>PRE-DENY-CTX</text>\n  </message>'
req-006.json 'k: PRE-REASON</tool-result>\n  </message>\n  <message role="developer">\n    <text>PRE-DENY-CTX</text>\n  </message>'
req-007.json 'k: PRE-REASON</tool-result>\n  </message>\n  <message role="developer">\n    <text>PRE-DENY-CTX</text>\n  </message>'
hits: 5

### R6 — additionalContext on SubagentStop and SessionEnd reaches no model request (control: SessionStart)
$ cat SubagentStop.sh
#!/bin/sh
cat >/dev/null; printf '{"hookSpecificOutput":{"hookEventName":"SubagentStop","additionalContext":"CTX-SubagentStop"}}\n'
$ muse exec --trust-workspace --yolo r6, then a second turn in the same session
muse: workspace root: W/r6/proj (cwd default)
muse: workspace trust: trusted source=run-flag
MOCKDONE
exit=0
muse: workspace root: W/r6/proj (cwd default)
muse: workspace trust: trusted source=run-flag
MOCKDONE
exit=0
$ requests: 10
$ requests holding CTX-SessionStart
req-001.json 'o not switch languages unless the user writes in or asks for another language.\n\nCTX-SessionStart\n\n<system-reminder s'
req-006.json ' or any other language unless the user writes in or asks for another language.\n\nCTX-SessionStart\n\n<system-reminder s'
hits: 2
$ requests holding CTX-SubagentStop
hits: 0
$ requests holding CTX-SessionEnd
hits: 0

### R7 — the spec's Lua command form (gen.lua) under three shells
$ airsl run gen.lua  # .lua
/bin/sh -c '"touch" "${TMPDIR:-/tmp}/it'\''s \"q\" \`b\` \\z"' # agentstacks
$ SHELL=/bin/zsh muse exec --provider echo --trust-workspace r7
muse: workspace root: W/r7/proj (cwd default)
muse: workspace trust: trusted source=run-flag
echo: r7
exit=0
$ ls $TMPDIR
it's "q" `b` \z
$ SHELL=/bin/sh muse exec --provider echo --trust-workspace r7
muse: workspace root: W/r7/proj (cwd default)
muse: workspace trust: trusted source=run-flag
echo: r7
exit=0
$ ls $TMPDIR
it's "q" `b` \z
$ SHELL=/bin/bash muse exec --provider echo --trust-workspace r7
muse: workspace root: W/r7/proj (cwd default)
muse: workspace trust: trusted source=run-flag
echo: r7
exit=0
$ ls $TMPDIR
it's "q" `b` \z

### R8 — the spec's message command form (gen.lua) under three shells, text with newline, ', " and \
$ airsl run gen.lua  # .msg
printf '%s\n' '{"hookSpecificOutput":{"additionalContext":"MSG-R8 line1\nline2 it'"'"'s \"q\" back\\slash","hookEventName":"SessionStart"}}' # agentstacks
$ SHELL=/bin/zsh muse exec --trust-workspace --yolo r8
muse: workspace root: W/r8/proj (cwd default)
muse: workspace trust: trusted source=run-flag
MOCKDONE
exit=0
$ requests holding the exact text
req-001.json 'o not switch languages unless the user writes in or asks for another language.\n\nMSG-R8 line1\nline2 it\'s "q" back\\slash\n\n<system-reminder s'
hits: 1
$ SHELL=/bin/sh muse exec --trust-workspace --yolo r8
muse: workspace root: W/r8/proj (cwd default)
muse: workspace trust: trusted source=run-flag
MOCKDONE
exit=0
$ requests holding the exact text
req-001.json 'o not switch languages unless the user writes in or asks for another language.\n\nMSG-R8 line1\nline2 it\'s "q" back\\slash\n\n<system-reminder s'
hits: 1
$ SHELL=/bin/bash muse exec --trust-workspace --yolo r8
muse: workspace root: W/r8/proj (cwd default)
muse: workspace trust: trusted source=run-flag
MOCKDONE
exit=0
$ requests holding the exact text
req-001.json 'o not switch languages unless the user writes in or asks for another language.\n\nMSG-R8 line1\nline2 it\'s "q" back\\slash\n\n<system-reminder s'
hits: 1

### R9 — skill name length and SKILL.md size (muse skills validate)
$ muse skills validate name128   # SKILL.md 162 bytes
valid	aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
exit=0
$ muse skills validate name129   # SKILL.md 163 bytes
install path is unsafe: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
exit=1
$ muse skills validate size262144   # SKILL.md 262144 bytes
valid	big
exit=0
$ muse skills validate size262145   # SKILL.md 262145 bytes
SKILL.md exceeds 262144 byte limit
exit=1

### R10 — does Muse load .claude/agents or .claude/settings.json hooks?
$ ls -R .claude .agents
.agents:
agents

.agents/agents:
muse-agent.md

.claude:
agents
settings.json

.claude/agents:
claude-agent.md
$ muse exec --trust-workspace --yolo r10
muse: workspace root: W/r10/proj (cwd default)
muse: workspace trust: trusted source=run-flag
MOCKDONE
exit=0
$ ls CLAUDE-HOOK-RAN
ls: CLAUDE-HOOK-RAN: No such file or directory
$ agent catalog in req-001: 'Agent Definition catalog for this run. Use an exact listed id:\\n- general-purpose\\n- muse-agent\\n- workflow-subagent\\n\\n<system-reminder source=\\"subagent-deleg'
```

### R11 — hook event names Muse 1.4.25 accepts (`probes/harness/events-run.sh`)

```text
$ events in .muse/hooks.json: SessionStart UserPromptSubmit PreToolUse PermissionRequest PostToolUse PostToolUseFailure PostToolBatch PreLLMCall PostLLMCall PreCompact PostCompact SubagentStart SubagentStop Stop StopFailure SessionEnd Notification SessionFork ToolUseStart OnSave
$ TBH_CREDENTIAL_BACKEND=file muse exec --provider echo --trust-workspace r11
muse: workspace root: W/r11/proj (cwd default)
muse: workspace trust: trusted source=run-flag
muse: Hooks: 18 runnable · 2 warnings
muse:   hooks.json: UnsupportedHandler: ToolUseStart hooks must declare `async: true`; synchronous cancellation hooks are not admitted
muse:   hooks.json: UnsupportedEvent: unsupported hook event `OnSave`
echo: r11
exit=0
```
