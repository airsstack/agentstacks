#!/bin/bash
# R5-R10 probes. usage: final2-run.sh <muse-binary> <scratch-dir>
M=$1; S=$2; HERE=$(cd "$(dirname "$0")" && pwd)
freeport() { python3 -I -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'; }
# newproj <dir> <port|-> : fresh HOME/TMPDIR/proj; with a port, seed the mock endpoint config
newproj() { rm -rf "$1"; mkdir -p "$1/home/.config/muse" "$1/tmp" "$1/proj/.muse"; (cd "$1/proj" && git init -q .)
  if [ "$2" != - ]; then
    printf '{"schema_version":1,"endpoint_transport":{"base_url":"http://127.0.0.1:%s","auth":"bearer"}}' "$2" > "$1/home/.config/muse/settings.json"
    printf '{"schema_version":1,"providers":{"meta":{"api_key":"dummy"}}}' > "$1/home/.config/muse/auth.json"; fi; }
# startmock <dir> <port> <script.json> : start mock, wait until it answers, else exit
startmock() { node "$HERE/mock.mjs" "$2" "$3" "$1/log" >/dev/null 2>&1 & MP=$!
  for i in $(seq 1 20); do curl -s -o /dev/null "http://127.0.0.1:$2/muse-code/models" && [ -d "$1/log" ] && return 0; sleep 0.5; done
  echo "mock not listening on $2"; kill $MP; exit 1; }
# runmuse <dir> <shell> <args...>
runmuse() { local d=$1 sh=$2; shift 2; (cd "$d/proj" && env -u XDG_CONFIG_HOME HOME="$d/home" TMPDIR="$d/tmp" SHELL="$sh" TBH_CREDENTIAL_BACKEND=file "$M" exec --trust-workspace "$@" 2>&1 | grep -v bundled | sed "s#$S#W#g"; echo "exit=${PIPESTATUS[0]}"); }
# hookstatus <dir> : every hook run's event, status and error from the session log
hookstatus() { python3 -I - "$1/home/.local/share/muse/sessions" <<'PY'
import json,sys,glob
for f in sorted(glob.glob(sys.argv[1]+"/**/session.jsonl",recursive=True)):
    for l in open(f):
        if "hook_run_terminal" not in l: continue
        d=json.loads(l); s=json.dumps(d)
        p=d.get("payload",{}); ev=json.dumps(p)[:0]
        import re
        m=re.search(r'"hook_event_name": "([A-Za-z]+)"',s) or re.search(r'"event": "([A-Za-z]+)"',s)
        st=re.search(r'"status": "([a-z_]+)"',s); err=re.search(r'"(error|message|detail)": "([^"]{0,160})',s)
        print((m.group(1) if m else "?"), (st.group(1) if st else "?"), (err.group(2) if err else ""))
PY
}
# findtext <logdir> <text> : the request files holding <text> as a decoded string, with 80 chars before
findtext() { python3 -I - "$1" "$2" <<'PY'
import json,sys,glob
def walk(v):
    if isinstance(v,str): yield v
    elif isinstance(v,dict):
        for x in v.values(): yield from walk(x)
    elif isinstance(v,list):
        for x in v: yield from walk(x)
hits=0
for f in sorted(glob.glob(sys.argv[1]+"/req-*.json")):
    for s in walk(json.load(open(f))):
        i=s.find(sys.argv[2])
        if i>=0: hits+=1; print(f.split("/")[-1], repr(s[max(0,i-80):i+len(sys.argv[2])+20])); break
print("hits:",hits)
PY
}

echo "### R5 — PreToolUse additionalContext, alone and with deny"
R=$S/r5; P=$(freeport); newproj $R $P; echo hello > $R/proj/a.txt
cat > $R/ctx.sh <<'X'
#!/bin/sh
cat >/dev/null; printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"PRE-CTX"}}\n'
X
cat > $R/deny.sh <<'X'
#!/bin/sh
cat >/dev/null; printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"PRE-REASON","additionalContext":"PRE-DENY-CTX"}}\n'
X
chmod +x $R/*.sh
printf '{"hooks":{"PreToolUse":[{"matcher":"Read","hooks":[{"type":"command","command":"%s"}]},{"matcher":"Write","hooks":[{"type":"command","command":"%s"}]}]}}\n' "$R/ctx.sh" "$R/deny.sh" > $R/proj/.muse/hooks.json
cat > $R/script.json <<'X'
[{"when":["probe r5"],"unless":["You are a goal reminder","call_"],"name":"read_file","args":{"path":"a.txt","offset":null,"limit":null}},
 {"when":["probe r5","call_"],"unless":["You are a goal reminder","PRE-REASON"],"name":"write_file","args":{"path":"b.txt","content":"x"}}]
X
echo '$ cat .muse/hooks.json'; sed "s#$S#W#g" $R/proj/.muse/hooks.json
startmock $R $P $R/script.json; echo '$ muse exec --trust-workspace --yolo "probe r5"'; runmuse $R /bin/zsh --yolo "probe r5"; kill $MP
echo '$ b.txt'; ls $R/proj/b.txt 2>&1 | sed "s#$S#W#g"
for t in PRE-CTX PRE-REASON PRE-DENY-CTX; do echo "\$ requests holding $t"; findtext $R/log $t; done

echo; echo "### R6 — additionalContext on SubagentStop and SessionEnd reaches no model request (control: SessionStart)"
R=$S/r6; P=$(freeport); newproj $R $P
for ev in SessionStart SubagentStop SessionEnd; do printf '#!/bin/sh\ncat >/dev/null; printf %s\n' "'{\"hookSpecificOutput\":{\"hookEventName\":\"$ev\",\"additionalContext\":\"CTX-$ev\"}}\\n'" > $R/$ev.sh; done
chmod +x $R/*.sh; echo '$ cat SubagentStop.sh'; cat $R/SubagentStop.sh
printf '{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"%s"}]}],"SubagentStop":[{"hooks":[{"type":"command","command":"%s"}]}],"SessionEnd":[{"hooks":[{"type":"command","command":"%s"}]}]}}\n' "$R/SessionStart.sh" "$R/SubagentStop.sh" "$R/SessionEnd.sh" > $R/proj/.muse/hooks.json
echo '[]' > $R/script.json; startmock $R $P $R/script.json
echo '$ muse exec --trust-workspace --yolo r6, then a second turn in the same session'; runmuse $R /bin/zsh --yolo "r6"
SID=$(ls $R/home/.local/share/muse/sessions/*/*/*/ | head -1); runmuse $R /bin/zsh --yolo --session-id "$SID" "r6 again"; kill $MP
echo "\$ requests: $(ls $R/log | wc -l | tr -d ' ')"
for t in CTX-SessionStart CTX-SubagentStop CTX-SessionEnd; do echo "\$ requests holding $t"; findtext $R/log $t; done

echo; echo "### R7 — the spec's Lua command form (gen.lua) under three shells"
GEN=$(airsl run "$HERE/gen.lua"); LUA=$(printf '%s' "$GEN" | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["lua"])')
echo '$ airsl run gen.lua  # .lua'; printf '%s\n' "$LUA"
for sh in /bin/zsh /bin/sh /bin/bash; do R=$S/r7; newproj $R -
  python3 -I -c 'import json,sys; print(json.dumps({"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":sys.argv[1]}]}]}}))' "$LUA" > $R/proj/.muse/hooks.json
  echo "\$ SHELL=$sh muse exec --provider echo --trust-workspace r7"; runmuse $R $sh --provider echo "r7"
  echo '$ ls $TMPDIR'; ls $R/tmp | grep -v '^muse' ; done

echo; echo "### R8 — the spec's message command form (gen.lua) under three shells, text with newline, ', \" and \\"
MSG=$(printf '%s' "$GEN" | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["msg"])'); TEXT=$(printf '%s' "$GEN" | python3 -I -c 'import json,sys; sys.stdout.write(json.load(sys.stdin)["text"])')
echo '$ airsl run gen.lua  # .msg'; printf '%s\n' "$MSG"
for sh in /bin/zsh /bin/sh /bin/bash; do R=$S/r8; P=$(freeport); newproj $R $P; echo '[]' > $R/script.json
  python3 -I -c 'import json,sys; print(json.dumps({"hooks":{"SessionStart":[{"matcher":"startup","hooks":[{"type":"command","command":sys.argv[1]}]}]}}))' "$MSG" > $R/proj/.muse/hooks.json
  startmock $R $P $R/script.json; echo "\$ SHELL=$sh muse exec --trust-workspace --yolo r8"; runmuse $R $sh --yolo "r8"; kill $MP
  echo '$ requests holding the exact text'; findtext $R/log "$TEXT"; done

echo; echo "### R9 — skill name length and SKILL.md size (muse skills validate)"
R=$S/r9; rm -rf $R; mkdir -p $R
mk() { mkdir -p "$R/$1"; { printf -- '---\nname: "%s"\ndescription: "d"\n---\n' "$2"; [ -n "$3" ] && head -c "$3" /dev/zero | tr '\0' 'x'; } > "$R/$1/SKILL.md"; }
N128=$(python3 -I -c 'print("a"*128)'); N129=$(python3 -I -c 'print("a"*129)')
mk name128 "$N128"; mk name129 "$N129"
HDR=$(printf -- '---\nname: "big"\ndescription: "d"\n---\n' | wc -c | tr -d ' ')
mk size262144 big $((262144-HDR)); mk size262145 big $((262145-HDR))
for d in name128 name129 size262144 size262145; do echo "\$ muse skills validate $d   # SKILL.md $(wc -c < $R/$d/SKILL.md | tr -d ' ') bytes"; (cd $R && HOME=$R TBH_CREDENTIAL_BACKEND=file "$M" skills validate "$d" 2>&1 | grep -v bundled | head -3; echo "exit=${PIPESTATUS[0]}"); done

echo; echo "### R10 — does Muse load .claude/agents or .claude/settings.json hooks?"
R=$S/r10; P=$(freeport); newproj $R $P; mkdir -p $R/proj/.claude/agents $R/proj/.agents/agents
printf -- '---\nname: claude-agent\ndescription: from .claude\n---\nCLAUDE-AGENT-BODY\n' > $R/proj/.claude/agents/claude-agent.md
printf -- '---\nname: muse-agent\ndescription: from .agents\n---\nMUSE-AGENT-BODY\n' > $R/proj/.agents/agents/muse-agent.md
printf '{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"touch CLAUDE-HOOK-RAN"}]}]}}\n' > $R/proj/.claude/settings.json
echo '$ ls -R .claude .agents'; (cd $R/proj && ls -R .claude .agents)
echo '[]' > $R/script.json; startmock $R $P $R/script.json; echo '$ muse exec --trust-workspace --yolo r10'; runmuse $R /bin/zsh --yolo "r10"; kill $MP
echo '$ ls CLAUDE-HOOK-RAN'; (cd $R/proj && ls CLAUDE-HOOK-RAN 2>&1)
python3 -I - $R/log/req-001.json <<'PY'
import sys
s=open(sys.argv[1]).read(); i=s.find("Agent Definition catalog")
print("$ agent catalog in req-001:", repr(s[i:i+160]) if i>=0 else "ABSENT")
PY
rm -rf $S/r5 $S/r6 $S/r7 $S/r8 $S/r9 $S/r10
