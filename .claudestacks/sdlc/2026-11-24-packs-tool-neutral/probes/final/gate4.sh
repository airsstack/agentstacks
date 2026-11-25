# Gate 4 keeps neutral pack text free of one tool's tool names, events and store. It reads
# grep's status before filtering out native/<tool>/, as gate 3 does. It skips the READMEs and
# CHANGELOG.md (documentation and history), the Lua tests, the scripts that read one tool's
# own files (statusline*.lua, transcript-path.lua), and doc-comment-discipline.md, which lists
# such names as words to keep out of source comments.
rc=0
found=$(grep -rnE 'subagent_type|TodoWrite|Task / Agent|`(Agent|Task|Bash|Read|Write|Edit|MultiEdit|Grep|Glob|Skill|WebFetch|WebSearch|AskUserQuestion|NotebookEdit)`|CLAUDE_CONFIG_DIR|CLAUDE_SESSION_ID|\.claude/|transcript JSONL|SessionStart|PreToolUse|PostToolUse|SubagentStop|UserPromptSubmit' packs/ --exclude=README.md --exclude=CHANGELOG.md --exclude='*_test.lua' --exclude='statusline*.lua' --exclude=transcript-path.lua --exclude=doc-comment-discipline.md) || rc=$?
if [ "$rc" -gt 1 ]; then
  echo "pack text gate 4: grep failed over packs/ (exit $rc)" >&2
  exit 1
fi
left=$(printf '%s\n' "$found" | grep -vE '^packs/[^/]+/native/' || true)
if [ -n "$left" ]; then
  printf '%s\n' "$left"
  echo "pack text gate 4: tool-specific names in neutral pack text" >&2
  exit 1
fi
