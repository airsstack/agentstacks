# Muse Code hook probes — muse 1.4.25 (1.4.25-R9361.1), macOS arm64, 2026-11-25

Binary `scratchpad/muse/bin/muse-bin-1.4.25-R9361.1`. Every run: fresh `HOME`, `TMPDIR` and a `git init` project
(`proj/`, containing `a.txt` = `hello`) under `W/<run>/`, created by `harness/run.sh`:
`env -u XDG_CONFIG_HOME HOME=W/<run>/home TMPDIR=W/<run>/tmp [ENVX] muse exec --trust-workspace --yolo [--provider echo] <args> "<prompt>"`,
launched from `W/<run>/proj` (or `proj/$SUB`). Mock runs use `harness/mock2.mjs` on port 18781 (mock.mjs plus `text`/`repeat`
steps). `harness/gen.py` writes matcher-test hook files. The project hook file is `W/<run>/proj/.muse/hooks.json` =
the template shown with `@OUT@`/`@H@` substituted.

Transformations applied to pasted output, nothing else: `W` replaces
`/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/hookprobe`
(sed); stderr lines containing `bundled` removed; where a line says "cut", long lines are truncated with `cut -c`.
`muse exec` prints no hook failure on stderr; hook outcomes are read from the session log
`home/.local/share/muse/sessions/<date>/<id>/session.jsonl` (`hook_run_terminal` records).

## P1. Which shell runs a hook command; quoting and ${VAR:-default}

Commands (echo provider): `ENVX="SHELL=<s>" ./run.sh p1-<name> echo hi` for s in /bin/sh, /bin/zsh, /usr/bin/false, /nonexistent/shell; `UNSET=SHELL ./run.sh p1-unset echo hi` (SHELL removed with `env -u SHELL`).

`p1.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{
  "hooks": {
    "SessionStart": [
      { "hooks": [
        { "type": "command", "command": "ps -o comm= -p $$ > @OUT@/shell.txt; echo \"$0\" >> @OUT@/shell.txt; echo \"SHELL=$SHELL\" >> @OUT@/shell.txt" },
        { "type": "command", "command": "\"printf\" \"%s\\n\" \"${TMPDIR:-/tmp}/x\" \"${NOPE_UNSET:-/fallback}/y\" > @OUT@/quote.txt" }
      ] }
    ]
  }
}
```

### p1-sh
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p1-sh/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`shell.txt`:
```text
/bin/sh
/bin/sh
SHELL=/bin/sh
```
`quote.txt`:
```text
W/p1-sh/tmp/x
/fallback/y
```
### p1-zsh
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p1-zsh/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`shell.txt`:
```text
/bin/zsh
/bin/zsh
SHELL=/bin/zsh
```
`quote.txt`:
```text
W/p1-zsh/tmp/x
/fallback/y
```
### p1-false
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p1-false/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`shell.txt`:
```text
cat: p1-false/out/shell.txt: No such file or directory
```
`quote.txt`:
```text
cat: p1-false/out/quote.txt: No such file or directory
```
### p1-shell
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p1-shell/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`shell.txt`:
```text
cat: p1-shell/out/shell.txt: No such file or directory
```
`quote.txt`:
```text
cat: p1-shell/out/quote.txt: No such file or directory
```
### p1-unset
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p1-unset/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`shell.txt`:
```text
/bin/sh
/bin/sh
SHELL=/bin/zsh
```
`quote.txt`:
```text
W/p1-unset/tmp/x
/fallback/y
```
### p1-false hook status
session.jsonl hook_run_terminal events (python: event, status, exit_code, stdout, stderr, error, effects):
```text
{"event": "SessionStart", "status": "failed", "exit_code": 1, "stdout": "", "stderr": "", "error": null, "effects": null}
{"event": "SessionStart", "status": "failed", "exit_code": 1, "stdout": "", "stderr": "", "error": null, "effects": null}
```
### p1-shell hook status
session.jsonl hook_run_terminal events (python: event, status, exit_code, stdout, stderr, error, effects):
```text
{"event": "SessionStart", "status": "failed", "exit_code": null, "stdout": "", "stderr": "", "error": "No such file or directory (os error 2)", "effects": null}
{"event": "SessionStart", "status": "failed", "exit_code": null, "stdout": "", "stderr": "", "error": "No such file or directory (os error 2)", "effects": null}
```

Answer: hooks run as `$SHELL -c '<command>'` with $SHELL from the muse process env (/bin/sh → /bin/sh, /bin/zsh → /bin/zsh); SHELL unset → /bin/sh (the hook env still shows SHELL=/bin/zsh; its source was not determined). SHELL=/usr/bin/false → every hook fails (exit 1); SHELL=nonexistent → every hook fails ("No such file or directory (os error 2)"); both silent on stderr, exec exit 0. `"printf" "%s\n" "${TMPDIR:-/tmp}/x"` style quoting and `${VAR:-default}` expansion work under sh and zsh.

## P2. SessionStart source values reachable headless; does the matcher filter on source

Commands: run 1 `./run.sh p2 echo first`; run 2 (same home) `KEEP=1 RUNTAG=2 ./run.sh p2 echo second --session-id <run-1 session_id>`; `./run.sh p2c echo /clear`; `./run.sh p2k echo /compact`; `muse --provider echo --trust-workspace resume --last </dev/null` (p2 home); compaction: `./run.sh p2h mock "probe compact" --context-compaction-soft-threshold 0.0240 --context-compaction-hard-threshold 0.0260` (script: bash `seq 1 2000`) and `./run.sh p2m mock "probe compact" --context-compaction-soft-threshold 0.026 --context-compaction-hard-threshold 0.034` (script: bash `seq 1 8000` + a 9-heading summary text answer to "Summarize the preceding session", repeat; killed by a 90 s alarm).

`p2.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{
  "hooks": {
    "SessionStart": [
      { "hooks": [ { "type": "command", "command": "cat > /dev/null; echo \"$$ any\" >> @OUT@/matched.txt; cat >/dev/null" } ] },
      { "hooks": [ { "type": "command", "command": "tee -a @OUT@/payloads.jsonl >/dev/null; echo >> @OUT@/payloads.jsonl" } ] },
      { "matcher": "startup", "hooks": [ { "type": "command", "command": "cat >/dev/null; echo startup >> @OUT@/matched.txt" } ] },
      { "matcher": "resume", "hooks": [ { "type": "command", "command": "cat >/dev/null; echo resume >> @OUT@/matched.txt" } ] },
      { "matcher": "clear", "hooks": [ { "type": "command", "command": "cat >/dev/null; echo clear >> @OUT@/matched.txt" } ] },
      { "matcher": "compact", "hooks": [ { "type": "command", "command": "cat >/dev/null; echo compact >> @OUT@/matched.txt" } ] }
    ],
    "SessionEnd": [
      { "hooks": [ { "type": "command", "command": "tee -a @OUT@/payloads.jsonl >/dev/null; echo >> @OUT@/payloads.jsonl" } ] }
    ]
  }
}
```
p2h/p2m add PreCompact and PostCompact groups with the same tee command.

### p2 run 1 (startup)
```text
exit: 0
--- stdout
echo: first
--- stderr (bundled lines removed)
muse: workspace root: W/p2/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
### p2 run 2 (--session-id of run 1)
```text
exit: 0
--- stdout
echo: second
--- stderr (bundled lines removed)
muse: warning: session 01a2120b-732d-72a3-bdcc-bf4bf198eebd does not record a usable model for provider echo; using the provider default model
muse: workspace root: W/p2/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`matched.txt`:
```text
42482 any
startup
resume
42658 any
```
`payloads.jsonl`:
```text
{"hook_event_name":"SessionStart","source":"startup","session_id":"01a2120b-732d-72a3-bdcc-bf4bf198eebd","cwd":"W/p2/proj","transcript_path":null,"model":"unknown","permission_mode":"bypassPermissions"}
{"hook_event_name":"SessionEnd","reason":"other","session_id":"01a2120b-732d-72a3-bdcc-bf4bf198eebd","cwd":"W/p2/proj","transcript_path":null,"model":"unknown","permission_mode":"bypassPermissions"}
{"hook_event_name":"SessionStart","source":"resume","session_id":"01a2120b-732d-72a3-bdcc-bf4bf198eebd","cwd":"W/p2/proj","transcript_path":null,"model":"unknown","permission_mode":"bypassPermissions"}
{"hook_event_name":"SessionEnd","reason":"other","session_id":"01a2120b-732d-72a3-bdcc-bf4bf198eebd","cwd":"W/p2/proj","transcript_path":null,"model":"unknown","permission_mode":"bypassPermissions"}
```
### /clear as an exec prompt
```text
exit: 2
--- stdout
--- stderr (bundled lines removed)
/clear is only available in the interactive TUI
usage: muse exec [OPTIONS] [PROMPT] (run `muse exec --help` for options)
```
### /compact as an exec prompt
```text
exit: 0
--- stdout
echo: /compact
--- stderr (bundled lines removed)
muse: workspace root: W/p2k/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`matched.txt`:
```text
startup
42809 any
```
### muse resume --last without a TTY
```text
Failed to initialize input reader
exit=1
```
### compaction, hard threshold (p2h)
```text
exit: 0
--- stdout
MOCKDONE
--- stderr (bundled lines removed)
muse: workspace root: W/p2h/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
payloads.jsonl (cut -c1-140):
```text
{"hook_event_name":"SessionStart","source":"startup","session_id":"01a21215-205b-74b3-88f3-7534a079f31c","cwd":"W/p2h/proj","transcript_path
{"hook_event_name":"PreCompact","trigger":"hard","session_id":"01a21215-205b-74b3-88f3-7534a079f31c","turn_id":"01a21215-2756-71e1-ad0d-b539
{"hook_event_name":"SessionEnd","reason":"other","session_id":"01a21215-205b-74b3-88f3-7534a079f31c","cwd":"W/p2h/proj","transcript_path":nu
```
context_compaction records from session.jsonl:
```text
"kind":"context_compaction_candidate","candidate_id":"candidate-8","trigger":"hard_threshold_blocking","timing":"pre_turn","status":"rejected","strategy":{"strategy_id":"prefix-
"diagnostics":[{"reason":"unfit_install_floor","message":"the install needs 26237 tokens before any summary (1 kept user messages), which already reaches the hard threshold of 26000 tokens; no summary was requested"}]
"kind":"context_compaction_fallback","fallback_id":"fallback-9","reason":"hard_threshold_failed","severity":"warning","can_continue":true,"trigger":"hard_threshold_blocking","t
```
### compaction, soft threshold (p2m)
```text
exit: 142
--- stdout
--- stderr (bundled lines removed)
muse: workspace root: W/p2m/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
payloads.jsonl (cut -c1-140):
```text
{"hook_event_name":"SessionStart","source":"startup","session_id":"01a21210-405d-7660-a71d-775464d07235","cwd":"W/p2m/proj","transcript_path
{"hook_event_name":"PreCompact","trigger":"soft","session_id":"01a21210-405d-7660-a71d-775464d07235","turn_id":"01a21210-48d0-75b3-960d-6fa9
```
tracing log lines (cut -c1-260):
```text
2026-11-25T09:36:27.075400Z INFO tbh.local.context L50 event="context.compaction.selected" run_id=01a21210-48d0-75b3-960d-6fa97b9311bd candidate_id="candidate-7" install_id="none" trigger="soft_threshold_async" timing="pre_turn" status=running strategy_id="pre
2026-11-25T09:36:27.136581Z INFO tbh.local.provider L22 event="provider_request.lifecycle" provider="meta" phase="retry" outcome="scheduled" reason="decode_error" attempt=1 max_attempts=10 next_attempt=2 retry_delay_ms=1000 has_http_status=false http_status=0
2026-11-25T09:36:28.141966Z INFO tbh.local.provider L22 event="provider_request.lifecycle" provider="meta" phase="retry" outcome="scheduled" reason="decode_error" attempt=2 max_attempts=10 next_attempt=3 retry_delay_ms=2000 has_http_status=false http_status=0
```
model requests logged: 12; requests whose last input contains "Summarize the preceding session": 7

Answer: headless sources: `startup` (every exec) and `resume` (`exec --session-id <existing id>` with the session log on). `clear` is unreachable headless ("/clear is only available in the interactive TUI", exit 2); `/compact` in exec is just prompt text; `muse resume` needs a TTY ("Failed to initialize input reader"). Compaction thresholds fire PreCompact (trigger hard/soft), but with the mock no compaction completed (hard: unfit_install_floor; soft: summary call retried on decode_error), so `compact` source was not observed. SessionStart matcher filters on source: `startup` group ran only on run 1, `resume` only on run 2, `clear`/`compact` never.

## P3. A matcher on UserPromptSubmit, Stop, SessionEnd, SessionStart

Commands: `python3 -I gen.py <name> <Event>:<matcher> ...` then `./run.sh <name> echo hi` (`-` = no matcher).
### p3-UserPromptSubmit
`p3-UserPromptSubmit.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{
 "hooks": {
  "UserPromptSubmit": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'UserPromptSubmit zzz' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "zzz"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'UserPromptSubmit *' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "*"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'UserPromptSubmit -' >> @OUT@/fired.txt"
     }
    ]
   }
  ],
  "SessionStart": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SessionStart -' >> @OUT@/fired.txt"
     }
    ]
   }
  ]
 }
}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p3-UserPromptSubmit/proj (cwd default)
muse: workspace trust: trusted source=run-flag
muse: Hooks: 3 runnable · 1 warning
muse:   hooks.json: InvalidMatcher: D11: `user_prompt_submit` is unmatchable; drop this group's matcher
```
fired.txt (sort):
```text
SessionStart -
UserPromptSubmit -
UserPromptSubmit *
```
### p3-Stop
`p3-Stop.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{
 "hooks": {
  "Stop": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'Stop zzz' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "zzz"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'Stop *' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "*"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'Stop -' >> @OUT@/fired.txt"
     }
    ]
   }
  ],
  "SessionStart": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SessionStart -' >> @OUT@/fired.txt"
     }
    ]
   }
  ]
 }
}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p3-Stop/proj (cwd default)
muse: workspace trust: trusted source=run-flag
muse: Hooks: 3 runnable · 1 warning
muse:   hooks.json: InvalidMatcher: D11: `stop` is unmatchable; drop this group's matcher
```
fired.txt (sort):
```text
SessionStart -
Stop -
Stop *
```
### p3-SessionEnd
`p3-SessionEnd.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{
 "hooks": {
  "SessionEnd": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SessionEnd zzz' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "zzz"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SessionEnd other' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "other"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SessionEnd -' >> @OUT@/fired.txt"
     }
    ]
   }
  ],
  "SessionStart": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SessionStart -' >> @OUT@/fired.txt"
     }
    ]
   }
  ]
 }
}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p3-SessionEnd/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
fired.txt (sort):
```text
SessionEnd -
SessionEnd other
SessionStart -
```
### p3-SessionStart
`p3-SessionStart.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{
 "hooks": {
  "SessionStart": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SessionStart zzz' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "zzz"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SessionStart startup' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "startup"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SessionStart -' >> @OUT@/fired.txt"
     }
    ]
   }
  ],
  "Stop": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'Stop -' >> @OUT@/fired.txt"
     }
    ]
   }
  ]
 }
}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p3-SessionStart/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
fired.txt (sort):
```text
SessionStart -
SessionStart startup
Stop -
```

Answer: UserPromptSubmit and Stop: matcher `zzz` → ``muse:   hooks.json: InvalidMatcher: D11: `user_prompt_submit` is unmatchable; drop this group's matcher`` (Stop: `` `stop` is unmatchable ``), plus `muse: Hooks: 3 runnable · 1 warning`; only that group is skipped, `*` and no-matcher groups and other events still fire. SessionEnd matcher matches `reason` (`other` ran, `zzz` did not), SessionStart matcher matches `source`; no warning for those.

## P4. SubagentStart/SubagentStop matcher on subagent_id

Commands: `./run.sh p4 echo hi`; payload dump `./run.sh p4b echo hi`.
`p4.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{
 "hooks": {
  "SubagentStart": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SubagentStart goal-reminder' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "goal-reminder"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SubagentStart nomatch' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "nomatch"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SubagentStart -' >> @OUT@/fired.txt"
     }
    ]
   }
  ],
  "SubagentStop": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SubagentStop goal-reminder' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "goal-reminder"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SubagentStop nomatch' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "nomatch"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SubagentStop -' >> @OUT@/fired.txt"
     }
    ]
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SubagentStop goal' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "goal"
   },
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SubagentStop ^goal' >> @OUT@/fired.txt"
     }
    ],
    "matcher": "^goal"
   }
  ]
 }
}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p4/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
fired.txt (sort):
```text
SubagentStart -
SubagentStart goal-reminder
SubagentStop -
SubagentStop -
SubagentStop -
SubagentStop -
SubagentStop ^goal
SubagentStop ^goal
SubagentStop ^goal
SubagentStop ^goal
SubagentStop goal-reminder
SubagentStop goal-reminder
SubagentStop goal-reminder
SubagentStop goal-reminder
```
`p4b.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{
 "SubagentStart":[{"hooks":[{"type":"command","command":"tee -a @OUT@/payloads.jsonl >/dev/null; echo >> @OUT@/payloads.jsonl"}]}],
 "SubagentStop":[{"hooks":[{"type":"command","command":"tee -a @OUT@/payloads.jsonl >/dev/null; echo >> @OUT@/payloads.jsonl"}]}]
}}
```
p4b payloads.jsonl (cut -c1-330; 5 lines: 1 SubagentStart, 4 SubagentStop):
```text
{"hook_event_name":"SubagentStart","subagent_id":"goal-reminder","child_session_id":"01a2120f-1174-72e0-88f1-95b0e473b6af","session_id":"01a2120f-1174-72e0-88f1-95b0e473b6af","turn_id":"01a2120f-1174-72e0-88f1-95b0e473b6af","cwd":"W/p4b/proj","transcript_path":null,"model":"unknown","permission_mode":"bypassPermissions"}
{"hook_event_name":"SubagentStop","subagent_id":"goal-reminder","child_session_id":"01a2120f-1174-72e0-88f1-95b0e473b6af","stop_hook_active":false,"last_assistant_message":"echo: You are a reminder observer for the main agent.\nDo not answer the user. Do ALL reasoning privately and record exactly one decision this round by calli
{"hook_event_name":"SubagentStop","subagent_id":"goal-reminder","child_session_id":"01a2120f-1174-72e0-88f1-95b0e473b6af","stop_hook_active":false,"last_assistant_message":"echo: You are a reminder observer for the main agent.\nDo not answer the user. Do ALL reasoning privately and record exactly one decision this round by calli
{"hook_event_name":"SubagentStop","subagent_id":"goal-reminder","child_session_id":"01a2120f-1174-72e0-88f1-95b0e473b6af","stop_hook_active":false,"last_assistant_message":"echo: You are a reminder observer for the main agent.\nDo not answer the user. Do ALL reasoning privately and record exactly one decision this round by calli
{"hook_event_name":"SubagentStop","subagent_id":"goal-reminder","child_session_id":"01a2120f-1174-72e0-88f1-95b0e473b6af","stop_hook_active":false,"last_assistant_message":"echo: You are a reminder observer for the main agent.\nDo not answer the user. Do ALL reasoning privately and record exactly one decision this round by calli
```

Answer: yes, the matcher matches the child agent id (`subagent_id`, here `goal-reminder`): `goal-reminder` and regex `^goal` match, `nomatch` and the plain word `goal` (exact-alternative form) do not. With the echo provider the goal-reminder child starts once and stops 4 times (it retries its decision call), so SubagentStop fired 4× per group.

## P5. UserPromptSubmit block: JSON decision vs exit 2

Commands: `./run.sh p5a mock "probe prompt block"`, `./run.sh p5b mock "probe prompt block"` (script `[]`), each again with `--json` (RUNTAG=2).
### p5a
`p5a.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"cat >/dev/null; echo '{\"decision\":\"block\",\"reason\":\"RBLOCK\"}'"}]}],
 "Stop":[{"hooks":[{"type":"command","command":"cat >/dev/null; echo Stop >> @OUT@/fired.txt"}]}]}}
```
```text
exit: 1
--- stdout
--- stderr (bundled lines removed)
muse: workspace root: W/p5a/proj (cwd default)
muse: workspace trust: trusted source=run-flag
run ended with Cancelled
```
model requests logged: 0 ; Stop fired.txt: absent

--json run_terminal record + stderr:
```text
{"schema_version":1,"id":"01a21212-5050-72d0-b8bd-0ca71d315481","stream":{"kind":"session","id":"01a21212-4a47-70e2-b761-82cd0a4ecb6d"},"sequence":6,"recorded_at":1795599519824878,"record_type":"event","durability":"durable","causation_id":"01a21212-5021-7ac3-bdac-19a84b2b29d4","payload_type":"run.terminal.cancelled","payload_schema_version":1,"payload":{"kind":"run_terminal","command_id":"01a21212-5021-7ac3-bdac-19a84b2b29d4","run_stream":{"kind":"run","id":"01a21212-5021-7ac3-bdac-19a84b2b29d4"},"terminal":"cancelled","text":"","reason":"RBLOCK"}}
muse: workspace root: W/p5a/proj (cwd default)
muse: workspace trust: trusted source=run-flag
run ended with Cancelled
exit: 1
```
### p5b
`p5b.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{"UserPromptSubmit":[{"hooks":[{"type":"command","command":"cat >/dev/null; echo RBLOCK2 >&2; exit 2"}]}],
 "Stop":[{"hooks":[{"type":"command","command":"cat >/dev/null; echo Stop >> @OUT@/fired.txt"}]}]}}
```
```text
exit: 1
--- stdout
--- stderr (bundled lines removed)
muse: workspace root: W/p5b/proj (cwd default)
muse: workspace trust: trusted source=run-flag
run ended with Cancelled
```
model requests logged: 0 ; Stop fired.txt: absent

--json run_terminal record + stderr:
```text
{"schema_version":1,"id":"01a21212-70bc-7823-9339-2f91e95b9658","stream":{"kind":"session","id":"01a21212-6aa8-7fc3-8a48-352202542c9a"},"sequence":6,"recorded_at":1795599528124831,"record_type":"event","durability":"durable","causation_id":"01a21212-708a-7022-8962-d87b81b6246b","payload_type":"run.terminal.cancelled","payload_schema_version":1,"payload":{"kind":"run_terminal","command_id":"01a21212-708a-7022-8962-d87b81b6246b","run_stream":{"kind":"run","id":"01a21212-708a-7022-8962-d87b81b6246b"},"terminal":"cancelled","text":"","reason":"RBLOCK2"}}
muse: workspace root: W/p5b/proj (cwd default)
muse: workspace trust: trusted source=run-flag
run ended with Cancelled
exit: 1
```

Answer: both forms block identically: exec exits 1, stderr `run ended with Cancelled`, the prompt never reaches the model (0 requests), Stop does not fire; the reason (RBLOCK / RBLOCK2) appears only in the --json `run_terminal` record (`"terminal":"cancelled","reason":"RBLOCK"`), not on stderr.

## P6. PostToolUse block + additionalContext together

Command: `./run.sh p6 mock "probe post block"`.
`p6.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{"PostToolUse":[{"matcher":"Read","hooks":[{"type":"command","command":"cat >/dev/null; echo '{\"decision\":\"block\",\"reason\":\"PBLOCK\",\"hookSpecificOutput\":{\"hookEventName\":\"PostToolUse\",\"additionalContext\":\"PCTX\"}}'"}]}]}}
```
script:
```text
[{"when":["probe post block"],"unless":["You are a goal reminder","call_"],"name":"read_file","args":{"path":"a.txt","offset":null,"limit":null}}]
```
```text
exit: 0
--- stdout
MOCKDONE
--- stderr (bundled lines removed)
muse: workspace root: W/p6/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
Main-agent request after the tool call (req-002.json, last 3 input items):
```text
{"type": "function_call", "id": "fc_call_1", "call_id": "call_1", "name": "muse.read_file", "arguments": "{\"path\":\"a.txt\",\"offset\":null,\"limit\":null}"}
{"type": "function_call_output", "call_id": "call_1", "output": "PBLOCK"}
{"type": "message", "role": "developer", "content": "PCTX"}
```

Answer: the tool output the model sees is replaced by the reason (`function_call_output` = "PBLOCK"; a.txt's "hello" is withheld), followed by a developer message "PCTX"; the turn continues. SessionStart additionalContext: skipped (identical to initial-results.md item 6).

## P7. Command not found (exit 127)

Commands: `./run.sh p7a echo hi`; `./run.sh p7b mock "probe pre 127"` (script: write_file b.txt).
### p7a
`p7a.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{
 "SessionStart":[{"hooks":[{"type":"command","command":"nonexistent-cmd-xyz arg"}]}],
 "UserPromptSubmit":[{"hooks":[{"type":"command","command":"nonexistent-cmd-xyz arg"}]}],
 "Stop":[{"hooks":[{"type":"command","command":"nonexistent-cmd-xyz arg"}]},{"hooks":[{"type":"command","command":"cat >/dev/null; echo Stop-ok >> @OUT@/fired.txt"}]}]
}}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p7a/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`fired.txt`:
```text
Stop-ok
```
session.jsonl hook_run_terminal events (python: event, status, exit_code, stdout, stderr, error, effects):
```text
{"event": "SessionStart", "status": "failed", "exit_code": 127, "stdout": "", "stderr": "zsh:1: command not found: nonexistent-cmd-xyz\n", "error": null, "effects": null}
{"event": "UserPromptSubmit", "status": "failed", "exit_code": 127, "stdout": "", "stderr": "zsh:1: command not found: nonexistent-cmd-xyz\n", "error": null, "effects": null}
{"event": "Stop", "status": "failed", "exit_code": 127, "stdout": "", "stderr": "zsh:1: command not found: nonexistent-cmd-xyz\n", "error": null, "effects": null}
```
### p7b
`p7b.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{"PreToolUse":[{"matcher":"Write","hooks":[{"type":"command","command":"nonexistent-cmd-xyz arg"}]}]}}
```
script:
```text
[{"when":["probe pre 127"],"unless":["You are a goal reminder","call_"],"name":"write_file","args":{"path":"b.txt","content":"WRITTEN\n"}}]
```
```text
exit: 0
--- stdout
MOCKDONE
--- stderr (bundled lines removed)
muse: workspace root: W/p7b/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`b.txt`:
```text
WRITTEN
```
req-002.json last 2 input items (cut -c1-300):
```text
{"type": "function_call", "id": "fc_call_1", "call_id": "call_1", "name": "muse.write_file", "arguments": "{\"path\":\"b.txt\",\"content\":\"WRITTEN\\n\"}"}
{"type": "function_call_output", "call_id": "call_1", "output": "wrote 8 bytes to W/p7b/proj/b.txt; note: near-duplicate sibling a.txt exists in this dire
```
session.jsonl hook_run_terminal events (python: event, status, exit_code, stdout, stderr, error, effects):
```text
{"event": "PreToolUse", "status": "failed", "exit_code": 127, "stdout": "", "stderr": "zsh:1: command not found: nonexistent-cmd-xyz\n", "error": null, "effects": null}
```

Answer: nothing is printed to the user (stderr has only the two startup lines; exec exit 0); the hook is recorded `status:"failed","exit_code":127` with stderr `zsh:1: command not found: nonexistent-cmd-xyz` in session.jsonl only; the run continues and other hooks on the event still run. A PreToolUse hook exiting 127 fails open: the write_file call ran and b.txt was written.

## P8. Invalid hooks.json

Commands: `./run.sh <name> echo hi` for each file.
### p8a
`p8a.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"cat >/dev/null; echo good-SessionStart >> @OUT@/fired.txt"}]}],
 "UserPromptSubmit":[{"hooks":[{"type":"command","command":"cat >/dev/null; echo bad-foo >> @OUT@/fired.txt","foo":1},{"type":"command","command":"cat >/dev/null; echo good-UserPromptSubmit >> @OUT@/fired.txt"}]}]}}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p8a/proj (cwd default)
muse: workspace trust: trusted source=run-flag
muse: Hooks: 2 runnable · 1 warning
muse:   hooks.json: UnsupportedHandler: D66: unknown handler field `foo` may narrow execution, so this handler is skipped
```
`fired.txt`:
```text
good-SessionStart
good-UserPromptSubmit
```
### p8b
`p8b.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"cat >/dev/null; echo good-SessionStart >> @OUT@/fired.txt"}]}],
 "UserPromptSubmit":[{"matcher":5,"hooks":[{"type":"command","command":"cat >/dev/null; echo bad-matcher >> @OUT@/fired.txt"}]}]}}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p8b/proj (cwd default)
muse: workspace trust: trusted source=run-flag
muse: Hooks: 0 runnable · 1 warning
muse:   hooks.json: MalformedConfig: hook matcher must be a string, found a number
```
`fired.txt`:
```text
cat: p8b/out/fired.txt: No such file or directory
```
### p8c1
`p8c1.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"extra":1,"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"cat >/dev/null; echo good-SessionStart >> @OUT@/fired.txt"}]}]}}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p8c1/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`fired.txt`:
```text
good-SessionStart
```
### p8c2
`p8c2.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"SessionStart":[{"hooks":[{"type":"command","command":"cat >/dev/null; echo good-SessionStart >> @OUT@/fired.txt"}]}]}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p8c2/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`fired.txt`:
```text
cat: p8c2/out/fired.txt: No such file or directory
```
### p8d
`p8d.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p8d/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`fired.txt`:
```text
cat: p8d/out/fired.txt: No such file or directory
```
### p8e
`p8e.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"cat >/dev/null; echo good-SessionStart >> @OUT@/fired.txt"}]}]},}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p8e/proj (cwd default)
muse: workspace trust: trusted source=run-flag
muse: Hooks: 0 runnable · 1 warning
muse:   hooks.json: MalformedConfig: trailing comma at line 1 column 280
```
`fired.txt`:
```text
cat: p8e/out/fired.txt: No such file or directory
```

Answer: (a) unknown handler key → ``UnsupportedHandler: D66: unknown handler field `foo` may narrow execution, so this handler is skipped``, only that handler is dropped; (b) numeric matcher → `MalformedConfig: hook matcher must be a string, found a number`, whole file dropped (0 runnable); (c) extra top-level key next to `hooks` → silently ignored, hooks fire; a file with events at top level (no `hooks`) → silently nothing; (d) `{}` → silently nothing; (e) invalid JSON → `MalformedConfig: trailing comma at line 1 column 280`, whole file dropped. Warnings appear as `muse: Hooks: N runnable · K warning` + `muse:   hooks.json: <Class>: [Dnn: ]<msg>`.

## P9. Which events deliver additionalContext to the model

Command: `./run.sh p9 mock "probe ctx events"` (script: read_file missing.txt).
`p9.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{
 "PostToolUseFailure":[{"matcher":"Read","hooks":[{"type":"command","command":"@H@/ctx.sh PostToolUseFailure"}]},{"hooks":[{"type":"command","command":"tee -a @OUT@/payloads.jsonl >/dev/null; echo >> @OUT@/payloads.jsonl"}]}],
 "SubagentStart":[{"hooks":[{"type":"command","command":"@H@/ctx.sh SubagentStart"}]}],
 "Stop":[{"hooks":[{"type":"command","command":"@H@/ctx.sh Stop"}]}]
}}
```
script:
```text
[{"when":["probe ctx events"],"unless":["You are a goal reminder","call_"],"name":"read_file","args":{"path":"missing.txt","offset":null,"limit":null}}]
```
```text
exit: 0
--- stdout
MOCKDONE
--- stderr (bundled lines removed)
muse: workspace root: W/p9/proj (cwd default)
muse: workspace trust: trusted source=run-flag
```
`payloads.jsonl`:
```text
{"hook_event_name":"PostToolUseFailure","tool_name":"read_file","tool_input":{"path":"missing.txt","offset":null,"limit":null},"tool_use_id":"call_1","error":"tool failed: No such file or directory (os error 2); requested relative path \"missing.txt\"; resolved candidate \"W/p9/proj/missing.txt\"; Active Workspace Root \"W/p9/proj\". Relative paths resolve from the Active Workspace Root. Shell `cd`/`workdir` affects only that shell call and does not change this root. Absolute paths may be used only when the current filesystem policy allows them.","is_interrupt":false,"duration_ms":11,"session_id":"01a21213-cee7-7de3-8236-8cfddfbda29c","turn_id":"01a21213-d92d-7ea1-9e9b-7c4facd7fde9","cwd":"W/p9/proj","transcript_path":null,"model":"fake-model","permission_mode":"bypassPermissions","model_provider":"meta"}
```
CTX_ markers per logged request:
```text
p9/mlog1/req-001.json main []
p9/mlog1/req-002.json main ['CTX_PostToolUseFailure']
p9/mlog1/req-003.json goal-reminder ['CTX_PostToolUseFailure', 'CTX_SubagentStart']
p9/mlog1/req-004.json goal-reminder ['CTX_PostToolUseFailure', 'CTX_SubagentStart']
p9/mlog1/req-005.json goal-reminder ['CTX_PostToolUseFailure', 'CTX_SubagentStart']
p9/mlog1/req-006.json goal-reminder ['CTX_PostToolUseFailure', 'CTX_SubagentStart']
```
req-002.json last input item; req-003.json first input item:
```text
{"type": "message", "role": "developer", "content": "CTX_PostToolUseFailure"}
{"type": "message", "role": "developer", "content": "CTX_SubagentStart"}
```
session.jsonl hook_run_terminal events (python: event, status, exit_code, stdout, stderr, error, effects):
```text
{"event": "PostToolUseFailure", "status": "completed", "exit_code": 0, "stdout": "", "stderr": "", "error": null, "effects": ["context"]}
{"event": "Stop", "status": "failed", "exit_code": 0, "stdout": "{\"hookSpecificOutput\":{\"hookEventName\":\"Stop\",\"additionalContext\":\"CTX_Stop\"}}\n", "stderr": "", "error": "unsupported `additionalContext` in hookSpecificOutput of Stop hook output", "effects": null}
{"event": "SubagentStart", "status": "completed", "exit_code": 0, "stdout": "", "stderr": "", "error": null, "effects": ["context"]}
```
Docs:
```text
probes/docs/guides_plugins_reference_hook-events_.txt:18: takes effect. “Context” means hookSpecificOutput.additionalContext takes
probes/docs/guides_plugins_reference_hook-events_.txt:19: effect. “Stops” means continue: false ends the turn.
probes/docs/guides_plugins_reference_hook-events_.txt:80: SubagentStart | 
probes/docs/guides_plugins_reference_hook-events_.txt:81: A child agent session starts, inside a turn. | 
probes/docs/guides_plugins_reference_hook-events_.txt:82: No | 
probes/docs/guides_plugins_reference_hook-events_.txt:83: Yes, JSON and plain stdout | 
probes/docs/guides_plugins_reference_hook-events_.txt:85: SubagentStop | 
probes/docs/guides_plugins_reference_hook-events_.txt:86: A child agent wants to stop. | 
probes/docs/guides_plugins_reference_hook-events_.txt:87: Yes, the child keeps working | 
probes/docs/guides_plugins_reference_hook-events_.txt:88: No | 
probes/docs/guides_plugins_reference_hook-events_.txt:90: Stop | 
probes/docs/guides_plugins_reference_hook-events_.txt:91: The model wants to end the turn. | 
probes/docs/guides_plugins_reference_hook-events_.txt:92: Yes, the model keeps working | 
probes/docs/guides_plugins_reference_hook-events_.txt:93: No | 
probes/docs/guides_plugins_reference_hook-events_.txt:240: hookSpecificOutput.additionalContext | 
probes/docs/guides_plugins_reference_hook-events_.txt:241: string | 
probes/docs/guides_plugins_reference_hook-events_.txt:242: SessionStart, SubagentStart, UserPromptSubmit, PreToolUse, PostToolUse, PostToolUseFailure, PostToolBatch, PreLLMCall, PostLLMCall | 
probes/docs/guides_plugins_reference_hook-events_.txt:243: Appended to the model’s context. Blank strings are ignored. | 
```

Answer: PostToolUseFailure → developer message after the failed tool output (main agent); SubagentStart → developer message first in the child (goal-reminder) request, not in the main agent; Stop → rejected: hook `failed` with ``unsupported `additionalContext` in hookSpecificOutput of Stop hook output``, nothing reaches the model. Matches the doc list at guides_plugins_reference_hook-events_.txt:242 (SessionStart, SubagentStart, UserPromptSubmit, PreToolUse, PostToolUse, PostToolUseFailure, PostToolBatch, PreLLMCall, PostLLMCall).

## P10. --workspace <root> launched from a subdirectory

Commands: `SUB=sub ./run.sh p10a echo hi` (control); `SUB=sub ./run.sh p10b echo hi --workspace W/p10b/proj`; `SUB=sub ./run.sh p10c echo hi --workspace W/p10c/proj` (payload + pwd).
`p10a.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{
 "hooks": {
  "SessionStart": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'SessionStart -' >> @OUT@/fired.txt"
     }
    ]
   }
  ],
  "UserPromptSubmit": [
   {
    "hooks": [
     {
      "type": "command",
      "command": "cat >/dev/null; echo 'UserPromptSubmit -' >> @OUT@/fired.txt"
     }
    ]
   }
  ]
 }
}
```
### p10a (no --workspace)
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p10a/proj/sub (cwd default)
muse: workspace trust: trusted source=run-flag
```
`fired.txt`:
```text
cat: p10a/out/fired.txt: No such file or directory
```
### p10b (--workspace)
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p10b/proj (explicit)
muse: workspace trust: trusted source=run-flag
```
`fired.txt`:
```text
SessionStart -
UserPromptSubmit -
```
### p10c
`p10c.hooks.json` (@OUT@ = W/<run>/out, @H@ = probes/harness):
```json
{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"tee @OUT@/payload.json >/dev/null; pwd > @OUT@/pwd.txt"}]}]}}
```
```text
exit: 0
--- stdout
echo: hi
--- stderr (bundled lines removed)
muse: workspace root: W/p10c/proj (explicit)
muse: workspace trust: trusted source=run-flag
```
`payload.json`:
```text
{"hook_event_name":"SessionStart","source":"startup","session_id":"01a21214-a8db-7493-bb8e-c16cd11c163e","cwd":"W/p10c/proj/sub","transcript_path":null,"model":"unknown","permission_mode":"bypassPermissions"}
```
`pwd.txt`:
```text
W/p10c/proj/sub
```

Answer: yes, `--workspace <root>` makes `<root>/.muse/hooks.json` load ("workspace root: … (explicit)"), but hooks still run with cwd = the launch directory (payload `cwd` and `pwd` = proj/sub), not the workspace root.
