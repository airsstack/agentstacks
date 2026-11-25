# Muse Code probe results — muse 1.4.25 (1.4.25-R9361.1), macOS arm64, 2026-11-25
Binary: scratchpad/muse/bin/muse-bin-1.4.25-R9361.1 (installed via install.sh, MUSE_NO_MODIFY_PATH=1, no login needed)
Isolation: HOME + XDG_CONFIG_HOME + TMPDIR in scratchpad. Mock model: harness/mock.mjs (Responses SSE).
Raw hook dumps: probe/out (echo), probe/out3 (mock); model requests: mlog0, mlog1.

1. Workspace root = launch cwd ("workspace root: … (cwd default)"), NOT git root. Started in sub/, project .muse/hooks.json did not load (no hook fired).
2. Hook process: PWD = workspace root. Env allowlist: HOME LANG LOGNAME PATH SHELL TERM TMPDIR USER + MUSE_SESSION_ID MUSE_THREAD_ID TRACEPARENT. AIRSSTACK_HOME and FOO_PROBE stripped. No project-dir var.
3. echo provider fires: SessionStart(source=startup), UserPromptSubmit(prompt), Stop(last_assistant_message), SubagentStart/SubagentStop(subagent_id=goal-reminder, built-in reminder), SessionEnd(reason=other).
4. Tool payload tool_name = native name: read_file / edit_file / write_file (bash, search also exist; no apply_patch by default). Matcher "Read|Edit|Write" matches them; "Edit" matches edit_file only; "Write" only write_file.
5. PreToolUse JSON {"hookSpecificOutput":{"permissionDecision":"deny","permissionDecisionReason":R}} blocked write_file; model saw "tool blocked by hook: R"; b.txt not written.
6. additionalContext: SessionStart → in system/developer context; UserPromptSubmit → developer message after the user prompt; PostToolUse → developer message after the tool output.
7. Skills: .agents/skills/<id>/SKILL.md (scope project) and .claude/skills/ both listed. Same id in both → .agents wins, .claude copy gets diagnostic skill-shadowed. disable-model-invocation:true → activation user-invocable-only, model-invocable="false" in catalog. `muse skills validate` → "valid".
8. Agents: .agents/agents/core-coder.md (name, description, model: inherit, effort: high, tools: [Read, Edit]) listed in "Agent Definition catalog". core-bad.md with unknown key `foo` silently absent; no warning printed on exec stderr.
9. Muse also reads the real ~/.claude/CLAUDE.md ("Including your Claude Code personal rules").
Not yet probed: SessionStart sources resume/clear/compact; SubagentStop matcher on a pack agent; exit-2 blocking; bash-tool env; agent `tools` name validation.
