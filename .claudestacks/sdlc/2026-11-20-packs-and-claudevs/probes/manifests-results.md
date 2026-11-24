# Pack manifests through the real loader

Run 2026-11-23 in /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/manifests. Binary: agentstacks built from HEAD 80fd75f (the worktree), airsl 0.1.4.
Each pack.toml is the spec's text with one edit: `">=0.1.5"` → `">=0.1.4"` (0.1.5 is not released).
Content files are stubs. To get past the cross-pack name clash rule (set/clash.rs) this run names two stub skills journal/vault-setup and guideline-python/py-guidelines; the spec removes that rule. Output pasted verbatim.

~~~~text
--- packs/core/pack.toml
exclude = ["README.md", "**/*_test.lua"]

[pack]
name        = "core"
version     = "0.1.0"
api         = 0
description = "Core workflow: coder, explorer and reviewer agents, orchestration, handoff and communication checks, guideline enforcement"

[requires]
airsl = ">=0.1.4"

[options.style_reinject]
type        = "boolean"
default     = true
description = "Re-inject the reply rules on every prompt; they are always sent at session start, /clear and compaction"

[options.archive_keep]
type        = "integer"
default     = 20
description = "Discussions kept per project besides the current one"

[[hooks]]
kind  = "message"
event = "session.start"
match = ["startup", "resume", "clear"]
text  = "Reminder: run {{skill snapshot-load}} for the current git branch to rehydrate relevant project memory before starting work."

[[hooks]]
kind  = "lua"
event = "session.start"
match = ["compact"]
run   = "hooks/rearm.lua"
[hooks.grants]
env   = ["TMPDIR"]
read  = ["{{var tmpdir}}"]
write = ["{{var tmpdir}}"]

[[hooks]]
kind  = "lua"
event = "session.start"
run   = "scripts/style.lua"
args  = ["{{skill_dir discuss}}/references/protocol.md"]
[hooks.grants]
read = ["{{skill_dir discuss}}"]

[[hooks]]
kind  = "message"
event = "session.end"
text  = "Reminder: run {{skill snapshot-save}} to flush durable session learnings into project memory (durability gate applies — thin sessions write nothing)."

[[hooks]]
kind  = "lua"
event = "prompt.submit"
run   = "scripts/style.lua"
args  = ["{{skill_dir discuss}}/references/protocol.md", "{{option style_reinject}}"]
[hooks.grants]
read = ["{{skill_dir discuss}}"]

[[hooks]]
kind  = "lua"
event = "tool.before"
match = ["read", "edit", "write"]
run   = "hooks/enforce.lua"
args  = ["{{root}}"]
[hooks.grants]
env   = ["HOME", "TMPDIR", "AIRSSTACK_HOME"]
read  = ["/"]
write = ["{{var tmpdir}}"]

[[hooks]]
kind      = "lua"
event     = "tool.after"
match     = ["write"]
run       = "scripts/handoff_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]

[[hooks]]
kind      = "lua"
event     = "tool.after"
match     = ["write"]
run       = "scripts/comm_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]

[[hooks]]
kind      = "lua"
event     = "agent.stop"
agents    = ["coder", "explorer", "reviewer"]
run       = "scripts/handoff_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]

[[hooks]]
kind      = "lua"
event     = "agent.stop"
run       = "scripts/comm_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]
--- packs/sdlc/pack.toml
exclude = ["README.md", "**/*_test.lua"]

[pack]
name        = "sdlc"
version     = "0.1.0"
api         = 0
description = "Intent, spec, plan and execute chain with a status board, roadmaps and ADRs"

[requires]
packs = ["core"]
airsl = ">=0.1.4"

[[hooks]]
kind  = "lua"
event = "session.start"
run   = "scripts/attention.lua"
args  = ["{{skill status}}"]
[hooks.grants]
read = ["/"]

[[hooks]]
kind      = "lua"
event     = "agent.stop"
agents    = ["task-briefer", "chain-reader", "artifact-reviewer"]
run       = "{{root core}}/scripts/handoff_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]
--- packs/journal/pack.toml
exclude = ["README.md", "**/*_test.lua"]

[pack]
name        = "journal"
version     = "0.1.0"
api         = 0
description = "Session journal: capture, atomic notes, recall and review over a local vault"

[requires]
packs = ["core"]
airsl = ">=0.1.4"

[[hooks]]
kind  = "lua"
event = "session.start"
match = ["startup", "resume", "clear"]
run   = "scripts/session-start.lua"
[hooks.grants]
env   = ["AIRSSTACK_HOME", "HOME"]
read  = ["{{var airsstack_home}}", "{{var project_dir}}"]
write = ["{{var airsstack_home}}"]

[[hooks]]
kind      = "lua"
event     = "agent.stop"
agents    = ["curator"]
run       = "{{root core}}/scripts/handoff_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]
--- packs/guideline-rust/pack.toml
render  = ["enforcement.json"]
exclude = ["README.md"]

[pack]
name        = "guideline-rust"
version     = "0.1.0"
api         = 0
description = "Rust engineering rules and the Definition of Done gate"
--- packs/guideline-python/pack.toml
render  = ["enforcement.json"]
exclude = ["README.md"]

[pack]
name        = "guideline-python"
version     = "0.1.0"
api         = 0
description = "Python engineering rules and the Definition of Done gate"
--- packs/core/native/claude/hooks.toml
[[hooks]]
kind      = "lua"
event     = "PreToolUse"
match     = ["^SubagentHandback$"]
run       = "scripts/handoff_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]

[[hooks]]
kind      = "lua"
event     = "PreToolUse"
match     = ["^SubagentHandback$"]
run       = "scripts/comm_check_hook.lua"
may_block = true
[hooks.grants]
env  = ["HOME", "TMPDIR"]
read = ["/"]
$ agentstacks init --tool ./ext-claude --pack ./packs/core --pack ./packs/sdlc --pack ./packs/journal --pack ./packs/guideline-rust --pack ./packs/guideline-python --yes
agentstacks plan

  create   .claude/agents/artifact-reviewer.md                      sdlc
  create   .claude/agents/capture.md                                journal
  create   .claude/agents/chain-reader.md                           sdlc
  create   .claude/agents/coder.md                                  core
  create   .claude/agents/curator.md                                journal
  create   .claude/agents/explorer.md                               core
  create   .claude/agents/note.md                                   journal
  create   .claude/agents/recall.md                                 journal
  create   .claude/agents/reviewer.md                               core
  create   .claude/agents/task-briefer.md                           sdlc
  create   .claude/agentstacks/core/hooks/doctor.lua                core
  create   .claude/agentstacks/core/hooks/enforce.lua               core
  create   .claude/agentstacks/core/hooks/rearm.lua                 core
  create   .claude/agentstacks/core/scripts/comm_check_hook.lua     core
  create   .claude/agentstacks/core/scripts/handoff_check_hook.lua  core
  create   .claude/agentstacks/core/scripts/style.lua               core
  create   .claude/agentstacks/guideline-python/enforcement.json    guideline-python
  create   .claude/agentstacks/guideline-rust/enforcement.json      guideline-rust
  create   .claude/agentstacks/journal/scripts/session-start.lua    journal
  create   .claude/agentstacks/runtime.lua                          claude
  create   .claude/agentstacks/sdlc/scripts/attention.lua           sdlc
  create   .claude/agentstacks/translate.lua                        claude
  create   .claude/settings.json                                    shared
  create   .claude/skills/adr/SKILL.md                              sdlc
  create   .claude/skills/capture/SKILL.md                          journal
  create   .claude/skills/context-handoff/SKILL.md                  core
  create   .claude/skills/core-statusline/SKILL.md                  core
  create   .claude/skills/design/SKILL.md                           sdlc
  create   .claude/skills/discuss/SKILL.md                          core
  create   .claude/skills/discuss/references/protocol.md            core
  create   .claude/skills/distill/SKILL.md                          sdlc
  create   .claude/skills/enforce-doctor/SKILL.md                   core
  create   .claude/skills/execute/SKILL.md                          sdlc
  create   .claude/skills/guidelines/SKILL.md                       guideline-rust
  create   .claude/skills/helped/SKILL.md                           journal
  create   .claude/skills/intent/SKILL.md                           sdlc
  create   .claude/skills/link/SKILL.md                             journal
  create   .claude/skills/milestone/SKILL.md                        sdlc
  create   .claude/skills/note/SKILL.md                             journal
  create   .claude/skills/orchestrate/SKILL.md                      core
  create   .claude/skills/plan/SKILL.md                             sdlc
  create   .claude/skills/process-guidelines/SKILL.md               core
  create   .claude/skills/py-guidelines/SKILL.md                    guideline-python
  create   .claude/skills/recall/SKILL.md                           journal
  create   .claude/skills/review/SKILL.md                           journal
  create   .claude/skills/roadmap/SKILL.md                          sdlc
  create   .claude/skills/setup/SKILL.md                            sdlc
  create   .claude/skills/snapshot-load/SKILL.md                    core
  create   .claude/skills/snapshot-save/SKILL.md                    core
  create   .claude/skills/status/SKILL.md                           sdlc
  create   .claude/skills/triage/SKILL.md                           sdlc
  create   .claude/skills/vault-setup/SKILL.md                      journal
  skip     core/hook/3: claude shows no hook output to the model at session end

  airsl    0.1.4 (new pin)

52 to write, 0 to delete, 0 unchanged, 1 skipped
applied: 52 written, 0 deleted
exit=0
$ find .claude -type f | sort
.claude/agents/artifact-reviewer.md
.claude/agents/capture.md
.claude/agents/chain-reader.md
.claude/agents/coder.md
.claude/agents/curator.md
.claude/agents/explorer.md
.claude/agents/note.md
.claude/agents/recall.md
.claude/agents/reviewer.md
.claude/agents/task-briefer.md
.claude/agentstacks/core/hooks/doctor.lua
.claude/agentstacks/core/hooks/enforce.lua
.claude/agentstacks/core/hooks/rearm.lua
.claude/agentstacks/core/scripts/comm_check_hook.lua
.claude/agentstacks/core/scripts/handoff_check_hook.lua
.claude/agentstacks/core/scripts/style.lua
.claude/agentstacks/guideline-python/enforcement.json
.claude/agentstacks/guideline-rust/enforcement.json
.claude/agentstacks/journal/scripts/session-start.lua
.claude/agentstacks/runtime.lua
.claude/agentstacks/sdlc/scripts/attention.lua
.claude/agentstacks/translate.lua
.claude/settings.json
.claude/skills/adr/SKILL.md
.claude/skills/capture/SKILL.md
.claude/skills/context-handoff/SKILL.md
.claude/skills/core-statusline/SKILL.md
.claude/skills/design/SKILL.md
.claude/skills/discuss/references/protocol.md
.claude/skills/discuss/SKILL.md
.claude/skills/distill/SKILL.md
.claude/skills/enforce-doctor/SKILL.md
.claude/skills/execute/SKILL.md
.claude/skills/guidelines/SKILL.md
.claude/skills/helped/SKILL.md
.claude/skills/intent/SKILL.md
.claude/skills/link/SKILL.md
.claude/skills/milestone/SKILL.md
.claude/skills/note/SKILL.md
.claude/skills/orchestrate/SKILL.md
.claude/skills/plan/SKILL.md
.claude/skills/process-guidelines/SKILL.md
.claude/skills/py-guidelines/SKILL.md
.claude/skills/recall/SKILL.md
.claude/skills/review/SKILL.md
.claude/skills/roadmap/SKILL.md
.claude/skills/setup/SKILL.md
.claude/skills/snapshot-load/SKILL.md
.claude/skills/snapshot-save/SKILL.md
.claude/skills/status/SKILL.md
.claude/skills/triage/SKILL.md
.claude/skills/vault-setup/SKILL.md
$ cat .claude/settings.json
{
  "hooks": {
    "PostToolUse": [
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"/\" \"--allow-env\" \"HOME\" \"--allow-env\" \"TMPDIR\" \".claude/agentstacks/runtime.lua\" \"core.scripts.handoff_check_hook\" \"1\" \"neutral\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "Write"
      },
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"/\" \"--allow-env\" \"HOME\" \"--allow-env\" \"TMPDIR\" \".claude/agentstacks/runtime.lua\" \"core.scripts.comm_check_hook\" \"1\" \"neutral\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "Write"
      }
    ],
    "PreToolUse": [
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"/\" \"--allow-write\" \"$TMPDIR\" \"--allow-env\" \"HOME\" \"--allow-env\" \"TMPDIR\" \"--allow-env\" \"AIRSSTACK_HOME\" \".claude/agentstacks/runtime.lua\" \"core.hooks.enforce\" \"0\" \"neutral\" \".claude/agentstacks/core\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "Read|Edit|Write"
      },
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"/\" \"--allow-env\" \"HOME\" \"--allow-env\" \"TMPDIR\" \".claude/agentstacks/runtime.lua\" \"core.scripts.handoff_check_hook\" \"1\" \"native\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "^SubagentHandback$"
      },
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"/\" \"--allow-env\" \"HOME\" \"--allow-env\" \"TMPDIR\" \".claude/agentstacks/runtime.lua\" \"core.scripts.comm_check_hook\" \"1\" \"native\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "^SubagentHandback$"
      }
    ],
    "SessionStart": [
      {
        "hooks": [
          {
            "command": "printf '%s\\n' '{\"hookSpecificOutput\":{\"additionalContext\":\"Reminder: run /snapshot-load for the current git branch to rehydrate relevant project memory before starting work.\",\"hookEventName\":\"SessionStart\"}}' # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "startup|resume|clear"
      },
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"$TMPDIR\" \"--allow-write\" \"$TMPDIR\" \"--allow-env\" \"TMPDIR\" \".claude/agentstacks/runtime.lua\" \"core.hooks.rearm\" \"0\" \"neutral\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "compact"
      },
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \".claude/skills/discuss\" \".claude/agentstacks/runtime.lua\" \"core.scripts.style\" \"0\" \"neutral\" \".claude/skills/discuss/references/protocol.md\" # agentstacks",
            "type": "command"
          }
        ]
      },
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"${AIRSSTACK_HOME:-$HOME/.airsstack}\" \"--allow-read\" \"${CLAUDE_PROJECT_DIR}\" \"--allow-write\" \"${AIRSSTACK_HOME:-$HOME/.airsstack}\" \"--allow-env\" \"AIRSSTACK_HOME\" \"--allow-env\" \"HOME\" \".claude/agentstacks/runtime.lua\" \"journal.scripts.session-start\" \"0\" \"neutral\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "startup|resume|clear"
      },
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"/\" \".claude/agentstacks/runtime.lua\" \"sdlc.scripts.attention\" \"0\" \"neutral\" \"/status\" # agentstacks",
            "type": "command"
          }
        ]
      }
    ],
    "SubagentStop": [
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"/\" \"--allow-env\" \"HOME\" \"--allow-env\" \"TMPDIR\" \".claude/agentstacks/runtime.lua\" \"core.scripts.handoff_check_hook\" \"1\" \"neutral\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "coder|explorer|reviewer"
      },
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"/\" \"--allow-env\" \"HOME\" \"--allow-env\" \"TMPDIR\" \".claude/agentstacks/runtime.lua\" \"core.scripts.comm_check_hook\" \"1\" \"neutral\" # agentstacks",
            "type": "command"
          }
        ]
      },
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"/\" \"--allow-env\" \"HOME\" \"--allow-env\" \"TMPDIR\" \".claude/agentstacks/runtime.lua\" \"core.scripts.handoff_check_hook\" \"1\" \"neutral\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "curator"
      },
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \"/\" \"--allow-env\" \"HOME\" \"--allow-env\" \"TMPDIR\" \".claude/agentstacks/runtime.lua\" \"core.scripts.handoff_check_hook\" \"1\" \"neutral\" # agentstacks",
            "type": "command"
          }
        ],
        "matcher": "task-briefer|chain-reader|artifact-reviewer"
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "command": "cd \"$CLAUDE_PROJECT_DIR\" && \"airsl\" \"run\" \"--fail-open\" \"--policy\" \"confined\" \"--allow-read\" \".claude/skills/discuss\" \".claude/agentstacks/runtime.lua\" \"core.scripts.style\" \"0\" \"neutral\" \".claude/skills/discuss/references/protocol.md\" \"true\" # agentstacks",
            "type": "command"
          }
        ]
      }
    ]
  }
}
$ cat .claude/agentstacks/guideline-rust/enforcement.json
{"stack": "rust", "skill": "/guidelines"}
$ cat .claude/skills/core-statusline/SKILL.md
---
name: core-statusline
description: stub
disable-model-invocation: true
---
Run ${CLAUDE_PROJECT_DIR}/.claude/agentstacks/core/scripts/statusline-install.lua
$ agentstacks plan
agentstacks plan

  skip     core/hook/3: claude shows no hook output to the model at session end

  airsl    0.1.4 (pinned)

0 to write, 0 to delete, 52 unchanged, 1 skipped
exit=0
~~~~

## Two proposals of one install path

A second project: pack `a` ships skill `x`; pack `b` ships `native/claude/skills/x/SKILL.md`.
Both make the claude extension propose `.claude/skills/x/SKILL.md`.

~~~~text
--- packs/a/pack.toml
[pack]
name = "a"
version = "0.1.0"
api = 0
description = "a"
--- packs/b/pack.toml
[pack]
name = "b"
version = "0.1.0"
api = 0
description = "b"
--- packs/a/skills/x/SKILL.md
---
name: x
description: stub
---
stub
--- packs/b/native/claude/skills/x/SKILL.md
---
name: x
description: stub
---
native stub
--- packs/b/skills/w/SKILL.md
---
name: w
description: stub
---
stub
$ agentstacks init --tool ./ext-claude --pack ./packs/a --pack ./packs/b --yes
claude/adapter.lua:135: copy: path ".claude/skills/x/SKILL.md" is already proposed at adapter.lua:41
agentstacks.toml written; fix the above and run apply
exit=1
~~~~

## The cross-pack name clash rule (today)

A third project: packs `c` and `d` both ship skill `setup`.

~~~~text
$ agentstacks init --tool ./ext-claude --pack ./packs/c --pack ./packs/d --yes
agentstacks.toml: skill "setup" is shipped by both c and d
agentstacks.toml written; fix the above and run apply
exit=1
~~~~
