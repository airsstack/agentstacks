# Two turns in one session, no --auto: export after a finished turn, outside-project reads, shell
# writes and removes in TMPDIR, and a background subagent reading outside the project.
SP=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad
D=$SP/tn/t3; SID=ses_zzzzprobe0000000000000001
rm -rf "$D"; mkdir -p "$D/tmp"; cp -R "$SP/ptn/proj" "$D/proj"
printf 'outside\n' > "$D/tmp/fixed.txt"; printf 'gone\n' > "$D/tmp/fixed2.txt"
cat > "$D/turn1.json" <<'EOF'
{ "main": [ { "text": "first answer" } ], "child": [] }
EOF
sed "s|@D@|$D|g" > "$D/turn2.json" <<'EOF'
{ "main": [
    { "tool": "shell", "args": { "command": "E=$(mktemp \"${TMPDIR:-/tmp}/journal-XXXXXX\"); echo \"$E\" > export-path.txt; opencode session export \"$OPENCODE_SESSION_ID\" > \"$E\"; echo $? > export.rc; cp \"$E\" export-copy.json; echo exported" } },
    { "tool": "read", "args": { "path": "@D@/tmp/fixed.txt" } },
    { "tool": "shell", "args": { "command": "rm -f \"@D@/tmp/fixed2.txt\"; echo $? > rm.rc" } },
    { "tool": "subagent", "args": { "agent": "journal-capture", "description": "probe", "prompt": "probe" } },
    { "text": "main done" } ],
  "child": [
    { "tool": "read", "args": { "path": "@D@/tmp/fixed.txt" } },
    { "tool": "shell", "args": { "command": "ls \"@D@/tmp\" > child-ls.txt; echo $? > child-ls.rc" } },
    { "tool": "write", "args": { "path": "@D@/tmp/child-write.txt", "content": "child\n" } },
    { "text": "child done" } ] }
EOF
sh "$SP/tn/run3.sh" "$D" 47341 $SID "$D/turn1.json" "first prompt"
mv "$D/requests.jsonl" "$D/requests-1.jsonl"
sh "$SP/tn/run3.sh" "$D" 47342 $SID "$D/turn2.json" "second prompt"
mv "$D/requests.jsonl" "$D/requests-2.jsonl"
