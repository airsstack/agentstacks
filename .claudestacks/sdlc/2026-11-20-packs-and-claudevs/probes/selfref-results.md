# Plugin text outside the namespaced-reference rows

Run 2026-11-23 in /Users/hiraq/Projects/airsstack/claudestacks/plugins. Output pasted verbatim.

~~~~text
$ grep -rnE 'CLAUDE_PLUGIN_(ROOT|OPTION)|user_config|claudestacks(-[a-z]+)*:[a-z]' claudestacks claudestacks-sdlc claudestacks-journal claudestacks-guideline-rust claudestacks-guideline-python --include='*.lua' --include='*.sh' | grep -v '_test.lua' | grep -v 'hooks/.*\.sh:\|attention.sh\|session-start.sh'
claudestacks/scripts/lib/discuss.lua:1:-- The per-session topic index behind /claudestacks:discuss.
claudestacks/scripts/lib/discuss.lua:19:-- literal `${user_config.archive_keep}` an unset option leaves in skill text — is the default.
claudestacks/scripts/lib/style.lua:34:-- Whether to re-send the rules on every prompt, from the raw `CLAUDE_PLUGIN_OPTION_STYLE_REINJECT`
claudestacks/scripts/discuss.lua:1:-- /claudestacks:discuss command line.
claudestacks/scripts/style.lua:8:--     --allow-env CLAUDE_PLUGIN_OPTION_STYLE_REINJECT --allow-read <plugin-root> \
claudestacks/scripts/style.lua:20:  local read, value = pcall(airsstack.env.get, "CLAUDE_PLUGIN_OPTION_STYLE_REINJECT")
claudestacks-sdlc/scripts/lib/status.lua:825:    "claudestacks-sdlc: %d open, %d need attention — full board: /claudestacks-sdlc:status",
$ grep -rnoE '(^|[ `"(])/(context-handoff|discuss|enforce-doctor|orchestrate|process-guidelines|snapshot-load|snapshot-save|statusline|adr|design|distill|execute|intent|milestone|plan|roadmap|setup|status|triage|journal-capture|journal-helped|journal-link|journal-note|journal-recall|journal-review|journal-setup|rust-guidelines|python-guidelines)\b' claudestacks claudestacks-sdlc claudestacks-journal claudestacks-guideline-rust claudestacks-guideline-python --include='*.md' | grep -v README
claudestacks/skills/snapshot-save/SKILL.md:16:`/snapshot-save
claudestacks/skills/snapshot-load/SKILL.md:28:`/snapshot-load
claudestacks/skills/snapshot-load/SKILL.md:37:`/snapshot-load
claudestacks-journal/references/note-schema.md:167:`/journal-review
claudestacks-journal/skills/journal-capture/SKILL.md:8:"/journal-capture
claudestacks-journal/skills/journal-note/SKILL.md:8:"/journal-note
claudestacks-journal/skills/journal-helped/SKILL.md:6: /journal-recall
claudestacks-journal/skills/journal-helped/SKILL.md:7:"/journal-helped
claudestacks-journal/skills/journal-link/SKILL.md:9:"/journal-link
claudestacks-journal/skills/journal-recall/SKILL.md:8:"/journal-recall
claudestacks-journal/skills/journal-review/SKILL.md:8:"/journal-review
claudestacks-journal/skills/journal-review/SKILL.md:19:`/journal-review
claudestacks-journal/skills/journal-review/SKILL.md:19:`/journal-review
$ grep -rnE 'subagent_type|the `journal-[a-z]+` subagent|Namespaced as' claudestacks claudestacks-sdlc claudestacks-journal claudestacks-guideline-rust claudestacks-guideline-python --include='*.md' | grep -v README
claudestacks/skills/orchestrate/SKILL.md:18:Namespaced as `claudestacks:coder`, `claudestacks:reviewer`, etc. Spawn each via the `Agent` tool's `subagent_type`. Both dials are set in each agent's frontmatter — the spawn can override `model:`, but effort comes from the definition only.
claudestacks-journal/skills/journal-capture/SKILL.md:64:5. Spawn the `journal-capture` subagent (Task / Agent tool,
claudestacks-journal/skills/journal-capture/SKILL.md:65:   `subagent_type: journal-capture`), passing `session_id`, `id8`, `stem`,
claudestacks-journal/skills/journal-capture/SKILL.md:74:If the `journal-capture` subagent cannot be spawned (the plugin's agents are
claudestacks-journal/skills/journal-note/SKILL.md:63:6. Spawn the `journal-note` subagent (Task / Agent tool,
claudestacks-journal/skills/journal-note/SKILL.md:64:   `subagent_type: journal-note`), passing `topic`, `session_id`, `id8`,
claudestacks-journal/skills/journal-note/SKILL.md:73:If the `journal-note` subagent cannot be spawned, tell the user and stop. Do
claudestacks-journal/skills/journal-link/SKILL.md:46:3. Spawn the `journal-recall` subagent (Task / Agent tool,
claudestacks-journal/skills/journal-link/SKILL.md:47:   `subagent_type: journal-recall`), passing `query`, `project`, `mode=link`,
claudestacks-journal/skills/journal-link/SKILL.md:56:If the `journal-recall` subagent cannot be spawned, tell the user and stop. Do
claudestacks-journal/skills/journal-recall/SKILL.md:40:3. Spawn the `journal-recall` subagent (Task / Agent tool,
claudestacks-journal/skills/journal-recall/SKILL.md:41:   `subagent_type: journal-recall`), passing `query`, `project`, and
claudestacks-journal/skills/journal-recall/SKILL.md:51:If the `journal-recall` subagent cannot be spawned, tell the user and stop. Do
claudestacks-journal/skills/journal-review/SKILL.md:69:5. Spawn the `journal-curator` subagent (`subagent_type: journal-curator`), passing `scope`, the
claudestacks-journal/skills/journal-review/SKILL.md:97:If the `journal-curator` subagent cannot be spawned, tell the user and stop. The
$ grep -rnE '`(coder|explorer|reviewer|task-briefer|chain-reader|artifact-reviewer|journal-capture|journal-curator|journal-note|journal-recall|context-handoff|discuss|enforce-doctor|orchestrate|process-guidelines|snapshot-load|snapshot-save|statusline)`' claudestacks claudestacks-sdlc claudestacks-journal --include='*.md' | grep -v README
claudestacks/skills/snapshot-save/SKILL.md:17:capture **mode** (mirrors `snapshot-load`):
claudestacks/skills/snapshot-save/SKILL.md:45:Keeping them in `~/.airsstack` (same root the `discuss` archive uses) gives one user-global state
claudestacks/skills/snapshot-load/SKILL.md:9:working on now." Operates the timestamped snapshot store written by `snapshot-save` (see that skill
claudestacks/skills/snapshot-load/SKILL.md:22:`snapshot-save` for the layout and the exact `<project-key>` resolution — compute it the **same way**
claudestacks/skills/snapshot-load/SKILL.md:41:1. **Resolve the store dir** (see `snapshot-save` for the `<project-key>` rule) and **read its
claudestacks/skills/process-guidelines/references/agent-orchestration.md:4:**principles**; the operational driver that runs the flow is the `orchestrate` skill — invoke that to
claudestacks/skills/process-guidelines/references/agent-orchestration.md:17:| `explorer` | haiku | Read-only locator; finds and maps code. Refuses judgment. |
claudestacks/skills/process-guidelines/references/agent-orchestration.md:18:| `coder` | sonnet | Executes one scoped task with strict TDD, runs the DoD, leaves changes in the tree. Never commits. |
claudestacks/skills/process-guidelines/references/agent-orchestration.md:19:| `reviewer` | opus | Re-runs the DoD, reviews the diff for correctness + style, AND reviews against the spec/plan intent. Report-only. |
claudestacks/skills/process-guidelines/references/agent-orchestration.md:30:1. **Locate (optional).** Spawn `explorer` when the task needs code found or a directory mapped first.
claudestacks/skills/process-guidelines/references/agent-orchestration.md:31:2. **Implement.** Spawn `coder` per scoped task. Independent tasks → parallel coder spawns.
claudestacks/skills/process-guidelines/references/agent-orchestration.md:32:3. **Review.** On the coder's receipt, spawn `reviewer` over the diff — one report covering the DoD,
claudestacks/skills/process-guidelines/references/model-routing.md:18:`explorer` low, `coder` high, `reviewer` high.
claudestacks/skills/process-guidelines/SKILL.md:26:  gate. The operational driver is the `orchestrate` skill. → `references/agent-orchestration.md`
claudestacks/skills/process-guidelines/SKILL.md:29:  `context-handoff` skill, which drivers invoke. → `/claudestacks:context-handoff`
claudestacks/skills/orchestrate/SKILL.md:14:| `explorer` | haiku · low | Read-only locator: finds and maps code as `file:line` tables. Run FIRST when the task needs code located. Refuses judgment. |
claudestacks/skills/orchestrate/SKILL.md:15:| `coder` | sonnet · high | Implements one scoped task with strict TDD, runs the active stack's DoD, leaves changes in the working tree. Never commits. |
claudestacks/skills/orchestrate/SKILL.md:16:| `reviewer` | opus · high | One combined report: re-runs the DoD + reviews the diff for style/correctness, AND reviews against the spec/plan intent. Report-only. |
claudestacks/skills/orchestrate/SKILL.md:22:1. **Locate (optional).** If the task needs code found or a directory mapped first, spawn `explorer`. Use its `file:line` tables to scope the coder's brief. Skip when the target is already clear.
claudestacks/skills/orchestrate/SKILL.md:23:2. **Batch, prove, implement.** Cut the work into batches (below), settle what each task assumes before a coder builds on it, then spawn one `coder` per task within a batch — all in one message, so they run concurrently.
claudestacks/skills/orchestrate/SKILL.md:24:3. **Review — once, when every task is done.** Spawn `reviewer` over the whole diff. It returns the DoD result + code findings + the spec/plan compliance verdict in one report. Once for the run: not per task, not per batch.
claudestacks/skills/orchestrate/SKILL.md:25:4. **Fix round — one.** Route every reviewer finding back through YOU to a FRESH `coder` spawn. The reviewer never calls the coder; you do. Then re-run only the verification each fix touches. Do NOT re-spawn the reviewer over the fixed diff — what closes the work is the reviewer's `blocking:` line, not a report that came back empty. See the review budget.
claudestacks/skills/orchestrate/SKILL.md:41:Anything failing either test starts a new batch. A batch of one is normal and fine — much work is genuinely sequential, because task N+1 uses what task N defined. Within a batch, one `coder` per task in a single message. **Never give two concurrent coders the same file.**
claudestacks/skills/orchestrate/SKILL.md:50:| **Structural — a symbol, path, or signature said to already exist** | one `explorer` spawn over the whole batch's symbols, returning `file:line` or NOT FOUND. Cheap; batch them. |
claudestacks/skills/orchestrate/SKILL.md:71:**One `reviewer` spawn for the whole run, after every task is complete. One fix round. Then you are done.**
claudestacks/skills/orchestrate/SKILL.md:73:- Route the reviewer's findings to a **fresh** `coder` spawn — the reviewer never edits and never calls the coder.
claudestacks/skills/orchestrate/SKILL.md:111:What is local to you: one `coder` per task with disjoint file sets, a single `reviewer` spawn
claudestacks/skills/context-handoff/references/protocol.md:104:source. `coder` writes with its `Write` tool. The read-only agents (`explorer`,
claudestacks/skills/context-handoff/references/protocol.md:105:`reviewer`) carry `Write` **scoped by instruction** to the handoff directory only —
claudestacks/skills/context-handoff/references/protocol.md:110:no other file through it. A `coder` additionally writes source within its task scope, but that
claudestacks/skills/context-handoff/SKILL.md:29:`coder`, `explorer` and `reviewer` live in this plugin and cite the path in their own
claudestacks-sdlc/skills/execute/SKILL.md:50:1. **Brief** — one `task-briefer` (`mode: brief`, `task: <N>`) per task, in one message.
claudestacks-sdlc/skills/execute/SKILL.md:61:   fresh `coder`, re-run only the verification it touches. No second review. Anything still
claudestacks-journal/skills/journal-capture/SKILL.md:15:storytelling to the isolated `journal-capture` subagent so it never costs the
claudestacks-journal/skills/journal-capture/SKILL.md:64:5. Spawn the `journal-capture` subagent (Task / Agent tool,
claudestacks-journal/skills/journal-capture/SKILL.md:74:If the `journal-capture` subagent cannot be spawned (the plugin's agents are
claudestacks-journal/skills/journal-note/SKILL.md:15:isolated `journal-note` subagent.
claudestacks-journal/skills/journal-note/SKILL.md:63:6. Spawn the `journal-note` subagent (Task / Agent tool,
claudestacks-journal/skills/journal-note/SKILL.md:73:If the `journal-note` subagent cannot be spawned, tell the user and stop. Do
claudestacks-journal/skills/journal-link/SKILL.md:46:3. Spawn the `journal-recall` subagent (Task / Agent tool,
claudestacks-journal/skills/journal-link/SKILL.md:56:If the `journal-recall` subagent cannot be spawned, tell the user and stop. Do
claudestacks-journal/skills/journal-recall/SKILL.md:16:`journal-recall` subagent. You never read a note body until — and unless — you
claudestacks-journal/skills/journal-recall/SKILL.md:40:3. Spawn the `journal-recall` subagent (Task / Agent tool,
claudestacks-journal/skills/journal-recall/SKILL.md:51:If the `journal-recall` subagent cannot be spawned, tell the user and stop. Do
claudestacks-journal/skills/journal-review/SKILL.md:16:`journal-curator` subagent. You hold no opinions about what to tidy — the
claudestacks-journal/skills/journal-review/SKILL.md:69:5. Spawn the `journal-curator` subagent (`subagent_type: journal-curator`), passing `scope`, the
claudestacks-journal/skills/journal-review/SKILL.md:97:If the `journal-curator` subagent cannot be spawned, tell the user and stop. The
~~~~
