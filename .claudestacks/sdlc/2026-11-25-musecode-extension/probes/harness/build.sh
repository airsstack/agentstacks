#!/bin/bash
# Assembles hooks-results.md from the saved run files (no retyped output).
W=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/hookprobe
C=/Users/hiraq/.herdr/worktrees/agentstacks/extension-musecode/.claudestacks/sdlc/2026-11-25-musecode-extension
OUT=$C/probes/hooks-results.md
cd $W
S() { sed "s#$W#W#g"; }
fence() { echo '```text'; S | awk 1; echo '```'; }
show() { local n=$1 r=${2:-1}; { echo "exit: $(cat $n/exit$r)"; echo "--- stdout"; cat $n/stdout$r; echo "--- stderr (bundled lines removed)"; cat $n/stderr$r; } | fence; }
hk() { echo "\`$1.hooks.json\` (@OUT@ = W/<run>/out, @H@ = probes/harness):"; echo '```json'; cat $1.hooks.json; [ -n "$(tail -c1 $1.hooks.json)" ] && echo; echo '```'; }
file() { echo "\`$2\`:"; cat "$1/$2" 2>&1 | fence; }
sl() { find $1/home/.local/share/muse/sessions -name session.jsonl | head -1; }
hookterm() { echo "session.jsonl hook_run_terminal events (python: event, status, exit_code, stdout, stderr, error, effects):"; python3 -I -c "
import json,sys
for l in open(sys.argv[1]):
    e=json.loads(l).get('payload',{}).get('event')
    if isinstance(e,dict) and e.get('kind')=='hook_run_terminal':
        print(json.dumps({k:e.get(k) for k in ('event','status','exit_code','stdout','stderr','error','effects')}))
" "$(sl $1)" | fence; }
{
cat <<'EOF'
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

EOF

echo "## P1. Which shell runs a hook command; quoting and \${VAR:-default}"
echo
echo "Commands (echo provider): \`ENVX=\"SHELL=<s>\" ./run.sh p1-<name> echo hi\` for s in /bin/sh, /bin/zsh, /usr/bin/false, /nonexistent/shell; \`UNSET=SHELL ./run.sh p1-unset echo hi\` (SHELL removed with \`env -u SHELL\`)."
echo; hk p1; echo
for n in p1-sh p1-zsh p1-false p1-shell p1-unset; do echo "### $n"; show $n; file $n/out shell.txt; file $n/out quote.txt; done
for n in p1-false p1-shell; do echo "### $n hook status"; hookterm $n; done
echo
echo "Answer: hooks run as \`\$SHELL -c '<command>'\` with \$SHELL from the muse process env (/bin/sh → /bin/sh, /bin/zsh → /bin/zsh); SHELL unset → /bin/sh (the hook env still shows SHELL=/bin/zsh; its source was not determined). SHELL=/usr/bin/false → every hook fails (exit 1); SHELL=nonexistent → every hook fails (\"No such file or directory (os error 2)\"); both silent on stderr, exec exit 0. \`\"printf\" \"%s\\n\" \"\${TMPDIR:-/tmp}/x\"\` style quoting and \`\${VAR:-default}\` expansion work under sh and zsh."
echo

echo "## P2. SessionStart source values reachable headless; does the matcher filter on source"
echo
echo "Commands: run 1 \`./run.sh p2 echo first\`; run 2 (same home) \`KEEP=1 RUNTAG=2 ./run.sh p2 echo second --session-id <run-1 session_id>\`; \`./run.sh p2c echo /clear\`; \`./run.sh p2k echo /compact\`; \`muse --provider echo --trust-workspace resume --last </dev/null\` (p2 home); compaction: \`./run.sh p2h mock \"probe compact\" --context-compaction-soft-threshold 0.0240 --context-compaction-hard-threshold 0.0260\` (script: bash \`seq 1 2000\`) and \`./run.sh p2m mock \"probe compact\" --context-compaction-soft-threshold 0.026 --context-compaction-hard-threshold 0.034\` (script: bash \`seq 1 8000\` + a 9-heading summary text answer to \"Summarize the preceding session\", repeat; killed by a 90 s alarm)."
echo; hk p2; echo "p2h/p2m add PreCompact and PostCompact groups with the same tee command."; echo
echo "### p2 run 1 (startup)"; show p2 1
echo "### p2 run 2 (--session-id of run 1)"; show p2 2
file p2/out matched.txt; file p2/out payloads.jsonl
echo "### /clear as an exec prompt"; show p2c
echo "### /compact as an exec prompt"; show p2k; file p2k/out matched.txt
echo "### muse resume --last without a TTY"; grep -v bundled p2/resume.out | fence
echo "### compaction, hard threshold (p2h)"; show p2h; echo "payloads.jsonl (cut -c1-140):"; S < p2h/out/payloads.jsonl | cut -c1-140 | fence
echo "context_compaction records from session.jsonl:"; grep -o '"kind":"context_compaction_[a-z]*".\{0,140\}\|"diagnostics":\[[^]]*\]' "$(sl p2h)" | fence
echo "### compaction, soft threshold (p2m)"; show p2m; echo "payloads.jsonl (cut -c1-140):"; S < p2m/out/payloads.jsonl | cut -c1-140 | fence
echo "tracing log lines (cut -c1-260):"; grep -h 'context.compaction.selected\|reason="decode_error"' p2m/home/.local/share/muse/local-tracing/*/*.log | head -3 | cut -c1-260 | fence
echo "model requests logged: $(ls p2m/mlog1 | wc -l | tr -d ' '); requests whose last input contains \"Summarize the preceding session\": $(grep -l 'Summarize the preceding session' p2m/mlog1/*.json | wc -l | tr -d ' ')"
echo
echo "Answer: headless sources: \`startup\` (every exec) and \`resume\` (\`exec --session-id <existing id>\` with the session log on). \`clear\` is unreachable headless (\"/clear is only available in the interactive TUI\", exit 2); \`/compact\` in exec is just prompt text; \`muse resume\` needs a TTY (\"Failed to initialize input reader\"). Compaction thresholds fire PreCompact (trigger hard/soft), but with the mock no compaction completed (hard: unfit_install_floor; soft: summary call retried on decode_error), so \`compact\` source was not observed. SessionStart matcher filters on source: \`startup\` group ran only on run 1, \`resume\` only on run 2, \`clear\`/\`compact\` never."
echo

echo "## P3. A matcher on UserPromptSubmit, Stop, SessionEnd, SessionStart"
echo
echo "Commands: \`python3 -I gen.py <name> <Event>:<matcher> ...\` then \`./run.sh <name> echo hi\` (\`-\` = no matcher)."
for n in p3-UserPromptSubmit p3-Stop p3-SessionEnd p3-SessionStart; do echo "### $n"; hk $n; show $n; echo "fired.txt (sort):"; sort $n/out/fired.txt | fence; done
echo
echo "Answer: UserPromptSubmit and Stop: matcher \`zzz\` → \`\`muse:   hooks.json: InvalidMatcher: D11: \`user_prompt_submit\` is unmatchable; drop this group's matcher\`\` (Stop: \`\` \`stop\` is unmatchable \`\`), plus \`muse: Hooks: 3 runnable · 1 warning\`; only that group is skipped, \`*\` and no-matcher groups and other events still fire. SessionEnd matcher matches \`reason\` (\`other\` ran, \`zzz\` did not), SessionStart matcher matches \`source\`; no warning for those."
echo

echo "## P4. SubagentStart/SubagentStop matcher on subagent_id"
echo
echo "Commands: \`./run.sh p4 echo hi\`; payload dump \`./run.sh p4b echo hi\`."
hk p4; show p4; echo "fired.txt (sort):"; sort p4/out/fired.txt | fence
hk p4b; echo "p4b payloads.jsonl (cut -c1-330; $(grep -c . p4b/out/payloads.jsonl) lines: 1 SubagentStart, $(grep -c SubagentStop p4b/out/payloads.jsonl) SubagentStop):"; S < p4b/out/payloads.jsonl | cut -c1-330 | fence
echo
echo "Answer: yes, the matcher matches the child agent id (\`subagent_id\`, here \`goal-reminder\`): \`goal-reminder\` and regex \`^goal\` match, \`nomatch\` and the plain word \`goal\` (exact-alternative form) do not. With the echo provider the goal-reminder child starts once and stops 4 times (it retries its decision call), so SubagentStop fired 4× per group."
echo

echo "## P5. UserPromptSubmit block: JSON decision vs exit 2"
echo
echo "Commands: \`./run.sh p5a mock \"probe prompt block\"\`, \`./run.sh p5b mock \"probe prompt block\"\` (script \`[]\`), each again with \`--json\` (RUNTAG=2)."
for n in p5a p5b; do echo "### $n"; hk $n; show $n 1; echo "model requests logged: $(ls $n/mlog1 2>/dev/null | wc -l | tr -d ' ') ; Stop fired.txt: $(cat $n/out/fired.txt 2>/dev/null || echo absent)"; echo; echo "--json run_terminal record + stderr:"; { grep run_terminal $n/stdout2; cat $n/stderr2; echo "exit: $(cat $n/exit2)"; } | fence; done
echo
echo "Answer: both forms block identically: exec exits 1, stderr \`run ended with Cancelled\`, the prompt never reaches the model (0 requests), Stop does not fire; the reason (RBLOCK / RBLOCK2) appears only in the --json \`run_terminal\` record (\`\"terminal\":\"cancelled\",\"reason\":\"RBLOCK\"\`), not on stderr."
echo

echo "## P6. PostToolUse block + additionalContext together"
echo
echo "Command: \`./run.sh p6 mock \"probe post block\"\`."
hk p6; echo "script:"; cat p6.script.json | fence; show p6
echo "Main-agent request after the tool call (req-002.json, last 3 input items):"; python3 -I -c "
import json
b=json.load(open('p6/mlog1/req-002.json'))
for it in b['input'][-3:]: print(json.dumps(it))
" | fence
echo
echo "Answer: the tool output the model sees is replaced by the reason (\`function_call_output\` = \"PBLOCK\"; a.txt's \"hello\" is withheld), followed by a developer message \"PCTX\"; the turn continues. SessionStart additionalContext: skipped (identical to initial-results.md item 6)."
echo

echo "## P7. Command not found (exit 127)"
echo
echo "Commands: \`./run.sh p7a echo hi\`; \`./run.sh p7b mock \"probe pre 127\"\` (script: write_file b.txt)."
echo "### p7a"; hk p7a; show p7a; file p7a/out fired.txt; hookterm p7a
echo "### p7b"; hk p7b; echo "script:"; cat p7b.script.json | fence; show p7b; file p7b/proj b.txt
echo "req-002.json last 2 input items (cut -c1-300):"; python3 -I -c "
import json
b=json.load(open('p7b/mlog1/req-002.json'))
for it in b['input'][-2:]: print(json.dumps(it)[:300])
" | fence
hookterm p7b
echo
echo "Answer: nothing is printed to the user (stderr has only the two startup lines; exec exit 0); the hook is recorded \`status:\"failed\",\"exit_code\":127\` with stderr \`zsh:1: command not found: nonexistent-cmd-xyz\` in session.jsonl only; the run continues and other hooks on the event still run. A PreToolUse hook exiting 127 fails open: the write_file call ran and b.txt was written."
echo

echo "## P8. Invalid hooks.json"
echo
echo "Commands: \`./run.sh <name> echo hi\` for each file."
for n in p8a p8b p8c1 p8c2 p8d p8e; do echo "### $n"; hk $n; show $n; file $n/out fired.txt; done
echo
echo "Answer: (a) unknown handler key → \`\`UnsupportedHandler: D66: unknown handler field \`foo\` may narrow execution, so this handler is skipped\`\`, only that handler is dropped; (b) numeric matcher → \`MalformedConfig: hook matcher must be a string, found a number\`, whole file dropped (0 runnable); (c) extra top-level key next to \`hooks\` → silently ignored, hooks fire; a file with events at top level (no \`hooks\`) → silently nothing; (d) \`{}\` → silently nothing; (e) invalid JSON → \`MalformedConfig: trailing comma at line 1 column 280\`, whole file dropped. Warnings appear as \`muse: Hooks: N runnable · K warning\` + \`muse:   hooks.json: <Class>: [Dnn: ]<msg>\`."
echo

echo "## P9. Which events deliver additionalContext to the model"
echo
echo "Command: \`./run.sh p9 mock \"probe ctx events\"\` (script: read_file missing.txt)."
hk p9; echo "script:"; cat p9.script.json | fence; show p9
file p9/out payloads.jsonl
echo "CTX_ markers per logged request:"; for f in p9/mlog1/req-*.json; do python3 -I -c "
import json,re
b=json.load(open('$f')); s=json.dumps(b)
print('$f', ('goal-reminder' if 'You are a goal reminder' in b.get('instructions','') else 'main'), sorted(set(re.findall(r'CTX_[A-Za-z]+', s))))"; done | fence
echo "req-002.json last input item; req-003.json first input item:"; python3 -I -c "
import json
print(json.dumps(json.load(open('p9/mlog1/req-002.json'))['input'][-1]))
print(json.dumps(json.load(open('p9/mlog1/req-003.json'))['input'][0]))" | fence
hookterm p9
echo "Docs:"
echo '```text'
for l in 18 19 80 81 82 83 85 86 87 88 90 91 92 93 240 241 242 243; do echo "probes/docs/guides_plugins_reference_hook-events_.txt:$l: $(sed -n ${l}p $C/probes/docs/guides_plugins_reference_hook-events_.txt)"; done
echo '```'
echo
echo "Answer: PostToolUseFailure → developer message after the failed tool output (main agent); SubagentStart → developer message first in the child (goal-reminder) request, not in the main agent; Stop → rejected: hook \`failed\` with \`\`unsupported \`additionalContext\` in hookSpecificOutput of Stop hook output\`\`, nothing reaches the model. Matches the doc list at guides_plugins_reference_hook-events_.txt:242 (SessionStart, SubagentStart, UserPromptSubmit, PreToolUse, PostToolUse, PostToolUseFailure, PostToolBatch, PreLLMCall, PostLLMCall)."
echo

echo "## P10. --workspace <root> launched from a subdirectory"
echo
echo "Commands: \`SUB=sub ./run.sh p10a echo hi\` (control); \`SUB=sub ./run.sh p10b echo hi --workspace W/p10b/proj\`; \`SUB=sub ./run.sh p10c echo hi --workspace W/p10c/proj\` (payload + pwd)."
hk p10a; echo "### p10a (no --workspace)"; show p10a; file p10a/out fired.txt
echo "### p10b (--workspace)"; show p10b; file p10b/out fired.txt
echo "### p10c"; hk p10c; show p10c; file p10c/out payload.json; file p10c/out pwd.txt
echo
echo "Answer: yes, \`--workspace <root>\` makes \`<root>/.muse/hooks.json\` load (\"workspace root: … (explicit)\"), but hooks still run with cwd = the launch directory (payload \`cwd\` and \`pwd\` = proj/sub), not the workspace root."
} > $OUT
echo written $(wc -l < $OUT)
