# hooks-map: claudestacks plugin hooks, scripts, skills (read-only locate)

Root: /Users/hiraq/Projects/airsstack/claudestacks/plugins/ (claudestacks-cmux excluded). No git run.
Abbrev: CP = ${CLAUDE_PLUGIN_ROOT}. "sh-grants" = the airsl run line in the wrapper.

## 1. HOOKS (one row per hooks.json entry)

| # | plugin | hooks.json (line) | event | matcher | wrapper .sh | airsl command (verbatim from wrapper) | Lua entry |
|---|---|---|---|---|---|---|---|
| H1 | claudestacks-journal | :3-13 | SessionStart | `startup\|resume\|clear` | scripts/session-start.sh (hooks.json:9) | `"$AIRSL" run --fail-open --policy confined --allow-env AIRSSTACK_HOME --allow-env HOME --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" --allow-read . "$DIR/session-start.lua"` (session-start.sh:34-38) | scripts/session-start.lua |
| H2 | claudestacks-sdlc | :3-12 | SessionStart | none | scripts/attention.sh (hooks.json:8) | `"$AIRSL" run --fail-open --policy confined --allow-read / "$DIR/attention.lua"` (attention.sh:28-30) | scripts/attention.lua |
| H3 | claudestacks | :3-12 | SessionStart | `startup\|resume\|clear` | hooks/preflight.sh (hooks.json:9) | no airsl run; echo-only status (preflight.sh:37-49); echoes airsl-missing STATUS text | none (inline `echo` at preflight.sh:18 names `/claudestacks:snapshot-load`) |
| H4 | claudestacks | :13-20 | SessionStart | `compact` | hooks/rearm.sh (hooks.json:18) | `"$AIRSL" run --fail-open --policy confined --allow-env TMPDIR --allow-read "$SENTINELS" --allow-write "$SENTINELS" "$DIR/rearm.lua"` (rearm.sh:30-33) | hooks/rearm.lua |
| H5 | claudestacks | :22-29 | SessionStart | none (no matcher) | hooks/style.sh (hooks.json:26) | `"$AIRSL" run --fail-open --policy confined --allow-env CLAUDE_PLUGIN_OPTION_STYLE_REINJECT --allow-read "$PLUGIN" "$PLUGIN/scripts/style.lua" "$PLUGIN/skills/discuss/references/protocol.md" "$@"` (style.sh:29-32; no `--turn` in this entry) | scripts/style.lua |
| H6 | claudestacks | :31-40 | SessionEnd | none | none (inline command) | `echo 'Reminder: run /claudestacks:snapshot-save to flush durable session learnings into project memory (durability gate applies — thin sessions write nothing).'` (hooks.json:36) | none |
| H7 | claudestacks | :41-50 | UserPromptSubmit | none | hooks/style.sh (hooks.json:46) with arg `--turn` | same as H5, argv gets `--turn` via `"$@"` (style.sh:32) | scripts/style.lua (with `--turn`, style.lua:14,19) |
| H8 | claudestacks | :51-60 | PreToolUse | `Read\|Edit\|Write` | hooks/enforce.sh (hooks.json:57) | `"$AIRSL" run --fail-open --policy confined --allow-env HOME --allow-env TMPDIR --allow-env AIRSSTACK_HOME --allow-env AIRSSTACK_ENFORCE_REGISTRY --allow-read / --allow-write "$SENTINELS" "$DIR/enforce.lua"` (enforce.sh:37-41; SENTINELS=`${TMPDIR:-/tmp}` enforce.sh:35) | hooks/enforce.lua |
| H9 | claudestacks | :61-73 | PreToolUse | `^SubagentHandback$` | hooks/handoff-check.sh (hooks.json:66) | `VIOLATIONS=$("$AIRSL" run --fail-open --policy confined --allow-env HOME --allow-env TMPDIR --allow-read / "$DIR/../scripts/handoff_check_hook.lua" 2>/dev/null) \|\| exit 0` (handoff-check.sh:31-34) | scripts/handoff_check_hook.lua |
| H10 | claudestacks | :61-73 (2nd command, same matcher) | PreToolUse | `^SubagentHandback$` | hooks/comm-check.sh (hooks.json:70) | `VIOLATIONS=$("$AIRSL" run --fail-open --policy confined --allow-env HOME --allow-env TMPDIR --allow-read / "$DIR/../scripts/comm_check_hook.lua" 2>/dev/null) \|\| exit 0` (comm-check.sh:30-33) | scripts/comm_check_hook.lua |
| H11 | claudestacks | :75-88 | PostToolUse | `Write` | hooks/handoff-check.sh (hooks.json:81) | same as H9 | scripts/handoff_check_hook.lua |
| H12 | claudestacks | :75-88 (2nd) | PostToolUse | `Write` | hooks/comm-check.sh (hooks.json:85) | same as H10 | scripts/comm_check_hook.lua |
| H13 | claudestacks | :90-99 | SubagentStop | `^claudestacks:(coder\|explorer\|reviewer)$\|^claudestacks-sdlc:(task-briefer\|chain-reader\|artifact-reviewer)$\|^claudestacks-journal:journal-curator$` | hooks/handoff-check.sh (hooks.json:96) | same as H9 | scripts/handoff_check_hook.lua |
| H14 | claudestacks | :100-107 | SubagentStop | none | hooks/comm-check.sh (hooks.json:104) | same as H10 | scripts/comm_check_hook.lua |

Exit behaviour of the wrappers:
- enforce.sh, rearm.sh, style.sh, attention.sh, session-start.sh, status.sh, preflight.sh: always exit 0.
- handoff-check.sh, comm-check.sh: `[ -n "$VIOLATIONS" ] || exit 0` (handoff-check.sh:36, comm-check.sh:35). If output starts with `{` it is printed to stdout and exit 0 (handoff-check.sh:39-41, comm-check.sh:38-40). Otherwise printed to stderr and `exit 2` (handoff-check.sh:43-44 area; comm-check.sh:42-43). exit 2 blocks on PreToolUse(SubagentHandback) only.

### Per Lua entry

**H8 hooks/enforce.lua** — top-level script; calls `main()` at :162, except `--explain` path :155-160. Ends with `if arg[1] == "--explain" ... else main() end`. Not a module.
- (a) payload fields: `payload.tool_input` (:121), `tool_input.file_path` (:122), `payload.cwd` (:128), `payload.session_id` (:136), `payload.agent_id` (:137). Via lib/enforce.lua `resolve` no further payload reads.
- (b) output: `airsstack.hook.context("PreToolUse", ...)` at :152. Shape: additionalContext text of joined pointers (lib/enforce.lua:377-383 `M.pointer` text: "<stack> work is in play. The <skill> skill is MANDATORY ..."). `--explain` mode: `airsstack.stdio.write` :160; usage error `airsstack.stdio.error` :157.
- (c) env: `airsstack.env` via `env.get` in settings(): `AIRSSTACK_HOME` (:28), `HOME` (:30); lib/enforce.lua:200 `AIRSSTACK_ENFORCE_REGISTRY`, :204 `HOME`; lib/enforce.lua:387 `TMPDIR`. `airsstack.time.now()` :131.
- (d) paths: registry read `$AIRSSTACK_ENFORCE_REGISTRY` or `$HOME/.claude/plugins/installed_plugins.json` (lib/enforce.lua:199-205). Manifest `<installPath>/enforcement.json` (lib/enforce.lua:295). Sentinels `${TMPDIR:-/tmp}/claudestacks-enforce-*` (lib/enforce.lua:24-25 prefix; :386-395 dir). SDD root `<AIRSSTACK_HOME or $HOME/.airsstack>/cc/plugins/sdd` (lib/enforce.lua:208-210; enforce.lua:30 default home). cwd from `payload.cwd` else `path.absolute(".")` (enforce.lua:128-129). Reads source files under `cwd` for `.git` (lib/enforce.lua:85-180). Read of file_path only for path matching, not content.
- (e) hardcoded: `"@claudestacks"` marketplace suffix (lib/enforce.lua:24); `"claudestacks-enforce-"` prefix (:25); agent default `"main"` (enforce.lua:138); explain identity `"enforce-doctor"`/`"doctor"` (enforce.lua:50). Skill/agent names come from each manifest, not hardcoded. Comment-only mention of `explorer` at lib/enforce.lua:397.
- (f) top-level script.

**H4 hooks/rearm.lua** — top-level script (requires lib.enforce :13; body :15-21, no function wrapper).
- (a) `payload.session_id` (:20). Payload read via `pcall(airsstack.hook.payload)` (:15).
- (b) no output (writes nothing to stdout; side effect is unlinking sentinels via `enforce.clear_session` :21; lib/enforce.lua:447).
- (c) env: `airsstack.env` passed into `enforce.sentinel_dir` → `TMPDIR` (lib/enforce.lua:387).
- (d) sentinel dir `${TMPDIR:-/tmp}` (lib/enforce.lua:386-394).
- (e) none.
- (f) top-level script.

**H5/H7 scripts/style.lua** — top-level script (requires lib.style :11; body :13-40).
- (a) none from payload except `payload.hook_event_name` (:36).
- (b) `airsstack.hook.context(event, rules)` :40; event from payload else `"UserPromptSubmit"` if `--turn` (arg[2]) else `"SessionStart"` (:37-39). Shape: additionalContext = the `## Reply rules` section (lib/style.lua:9 HEADING, :13 reply_rules).
- (c) `CLAUDE_PLUGIN_OPTION_STYLE_REINJECT` via `airsstack.env.get` pcall (:20); value parsed lib/style.lua:37-43 (only "false"/"0" disable).
- (d) reads `arg[1]` protocol path = `$PLUGIN/skills/discuss/references/protocol.md` (style.sh:32); `$PLUGIN` = `$DIR/..` (style.sh:10).
- (e) heading `"## Reply rules"` (lib/style.lua:9); fallback event names "SessionStart"/"UserPromptSubmit" (style.lua:38).
- (f) top-level script.

**H9/H11/H13 scripts/handoff_check_hook.lua** — top-level script (requires lib.handoff_report :23; body :34-63). Comment :6-8 says it lives in scripts/ because require resolves relative to script dir.
- (a) `payload.hook_event_name` (:59); plus lib/handoff_report.lua:362-377 `file_from_payload`: `payload.tool_input` (:366), `tool_input.file_path` (PostToolUse, :371), `tool_input.message` (PreToolUse, :373), `payload.last_assistant_message` (SubagentStop, :375), `payload.cwd` (:367).
- (b) PostToolUse: `airsstack.hook.emit({decision="block", reason=...})` :60. Other events: `stdio.write(reason.."\n")` to stdout :62 (wrapper turns stdout into exit 2). Shape of reason: `"handoff report does not conform to the protocol:\n"` + lines `id: line` (:57, :29).
- (c) env via lib/handoff_report.lua:11 `env`, :133 `pcall(env.get, "TMPDIR")`. Grants: HOME, TMPDIR (handoff-check.sh:32).
- (d) file paths: the report file named by payload (read via `fs.read_lines`, lib/handoff_report.lua:229); TMPDIR tier `${TMPDIR}/<driver>-<slug>.md` direct child (lib/handoff_report.lua:126-135); handoff root test lib/handoff_report.lua:98; path resolution vs `payload.cwd` (:367).
- (e) hardcoded: none found in this script; lib/handoff_report.lua has none of coder/reviewer/explorer names (grep).
- (f) top-level script.

**H10/H12/H14 scripts/comm_check_hook.lua** — top-level script (requires lib.handoff_report :10, lib.comm_report :11; body :13-31).
- (a) `payload.hook_event_name` (lib/comm_report.lua:145 via :23 arg); `report.file_from_payload(payload)` as H9 (:18).
- (b) `comm.hook_output` lib/comm_report.lua:145-158: PostToolUse → `{kind="emit", value={decision="block", reason=...}}` → `airsstack.hook.emit` (comm_check_hook.lua:27-28); else `{kind="stdout"}` → `airsstack.stdio.write` (:30). Reason text `"communication protocol report does not conform:\n"` (comm_report.lua:150).
- (c) as H9 (HOME, TMPDIR grants only; handoff_report.lua:133 pcall env.get TMPDIR).
- (d) same report file resolution as H9 (lib/handoff_report.lua:362-377).
- (e) `M.VERSION = "discuss/1"` (lib/comm_report.lua:13) — not emitted by the hook output; no agent/skill names.
- (f) top-level script.

**H2 claudestacks-sdlc scripts/attention.lua** — top-level script (requires lib.status :10; body :13-20).
- (a) `payload.cwd` (:14-15) via `pcall(airsstack.hook.payload)` (:13).
- (b) stdout: `airsstack.stdio.write(text .. "\n")` (:20), only when text non-empty. No hook.context/emit.
- (c) none.
- (d) reads `path.join(cwd, status.ROOT)` → `<cwd>/.claudestacks/sdlc` (attention.lua:18; lib/status.lua:21; lib/status.lua:805-811 `M.attention` reads the chain files via fs).
- (e) status line text `"claudestacks-sdlc: %d open, %d need attention — full board: /claudestacks-sdlc:status"` (lib/status.lua:825).
- (f) top-level script.

**H1 claudestacks-journal scripts/session-start.lua** — top-level script (requires lib.index :13, lib.orientation :14, lib.vault :15; body :19-73).
- (a) no payload read (grep: no `payload` in file).
- (b) `airsstack.hook.context("SessionStart", card)` :72, only when card non-empty (:71). Card from lib/orientation.lua (`M.render` :116).
- (c) env via lib/vault.lua:126-128 `AIRSSTACK_HOME`, `HOME`.
- (d) vault root `$AIRSSTACK_HOME/journal` else `$HOME/.airsstack/journal` (lib/vault.lua:124-131; session-start.lua:19). Creates `daily sessions notes mocs .index` under root (:23-25). Index marker `<root>/.index/summaries.tsv` (:20); scans `<root>/{daily,sessions,notes,mocs}/*.md` mtimes (:39-54, lib/index.lua:17 NOTE_DIRS). Rebuild writes `.index/*` (lib/index.lua:302-320). Project key from `vault.project_base()` with dir nil → `path.absolute(".")` (lib/vault.lua:20-22, :96-101).
- (e) none in code; dir names hardcoded (:23).
- (f) top-level script.

**H3 preflight.sh** has no Lua entry. **H6** has no entry.

## 2. Wrapper .sh files

Full paste: /Users/hiraq/Projects/airsstack/claudestacks/plugins/claudestacks/hooks/enforce.sh (43 lines, read at the time of this map):

```sh
#!/bin/sh
# claudestacks rule-enforcement dispatcher launcher.
#
# PreToolUse exit 2 BLOCKS the tool call, and the matcher covers Read — a propagated failure would
# block every file read. This launcher therefore never propagates a child's status: every path ends
# in `exit 0`. Deliberately no `exec`, which would replace the shell and hand airsl's status
# straight back to Claude Code.
#
# The read grant is `/` because the dispatcher is handed an arbitrary file_path by the tool call it
# is inspecting, and must stat it, its ancestors, and the install cache. What the confinement still
# buys over `--policy trusted` is real: no `io`, no `os.execute`, writes only under the sentinel
# directory, and `git` as the only program it can run.

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" 2>/dev/null && pwd) || exit 0
[ -n "$DIR" ] || exit 0
# Resolve airsl without relying on PATH. Hooks are spawned by the CLI rather than a login shell,
# so a profile may never have been sourced and an installed binary under ~/.local/bin or
# ~/.cargo/bin can be present but invisible — which reads as "airsl is not installed" and
# silently disables the hook.
AIRSL=""
if [ -n "${AIRSL_BIN:-}" ] && [ -x "${AIRSL_BIN:-}" ]; then
  AIRSL="$AIRSL_BIN"
elif command -v airsl >/dev/null 2>&1; then
  AIRSL=airsl
else
  for candidate in "$HOME/.local/bin/airsl" "${CARGO_HOME:-$HOME/.cargo}/bin/airsl" "$HOME/.cargo/bin/airsl"; do
    if [ -x "$candidate" ]; then
      AIRSL="$candidate"
      break
    fi
  done
fi
[ -n "$AIRSL" ] || exit 0

SENTINELS="${TMPDIR:-/tmp}"

"$AIRSL" run --fail-open --policy confined \
  --allow-env HOME --allow-env TMPDIR \
  --allow-env AIRSSTACK_HOME --allow-env AIRSSTACK_ENFORCE_REGISTRY \
  --allow-read / --allow-write "$SENTINELS" \
  "$DIR/enforce.lua" || exit 0

exit 0
```

Differences for the other wrappers (vs enforce.sh):
- rearm.sh:28-33: no HOME/AIRSSTACK grants; `--allow-env TMPDIR --allow-read "$SENTINELS" --allow-write "$SENTINELS"`; runs rearm.lua. Exit 0 always.
- style.sh:10-11: adds `PLUGIN` (`$DIR/..`); no --allow-write; `--allow-env CLAUDE_PLUGIN_OPTION_STYLE_REINJECT --allow-read "$PLUGIN"`; args `scripts/style.lua protocol.md "$@"`; exit 0 always. Also no `[ -n "$AIRSL" ]` difference (same).
- handoff-check.sh:31-44: `--allow-env HOME --allow-env TMPDIR --allow-read /`; no --allow-write; stdout captured; stderr /dev/null (:34); exit 2 only when violations are on stdout and do not start with `{`; otherwise exit 0.
- comm-check.sh:30-43: identical grants to handoff-check.sh; runs scripts/comm_check_hook.lua; same exit rules.
- attention.sh:26-30: no airsl PATH check difference; no grants except `--allow-read /`; no HOME/TMPDIR; exit 0.
- session-start.sh:34-38 (journal): `--allow-env AIRSSTACK_HOME --allow-env HOME --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" --allow-read .`; `HOME_ROOT="${AIRSSTACK_HOME:-$HOME/.airsstack}"` (session-start.sh:32); exit 0.
- status.sh (sdlc, not a hook; used by skills): `--allow-read .`, runs status.lua; exit 0 (status.sh:27-31).
- preflight.sh: no airsl run. If AIRSL empty, prints STATUS block (preflight.sh:37-49), exit 0.

## 3. enforce

Structure of /Users/hiraq/Projects/airsstack/claudestacks/plugins/claudestacks-guideline-rust/enforcement.json (full file, 1 object):
- top-level keys: `stack`, `detect`, `match`, `skill`, `phase`.
- example: `"stack": "rust"`, `"detect": ["Cargo.toml"]`, `"match": ["**/*.rs", "**/Cargo.toml"]`, `"skill": "claudestacks-guideline-rust:rust-guidelines"`, `"phase": ["code", "design"]`.

/Users/hiraq/Projects/airsstack/claudestacks/plugins/claudestacks-guideline-python/enforcement.json (full file):
- top-level keys: `stack`, `detect`, `match`, `skill`, `phase` (same keys).
- example: `"stack": "python"`, `"detect": ["pyproject.toml"]`, `"match": ["**/*.py", "**/*.pyi", "**/pyproject.toml"]`, `"skill": "claudestacks-guideline-python:python-guidelines"`, `"phase": ["code", "design"]`.

Registry walk (enforce.lua library, /Users/hiraq/Projects/airsstack/claudestacks/plugins/claudestacks/hooks/lib/enforce.lua):
- registry path: :199-205 (`AIRSSTACK_ENFORCE_REGISTRY` else `$HOME/.claude/plugins/installed_plugins.json`).
- read + keep `@claudestacks` keys only: :240-273 (`read_registry`; suffix const :24).
- select record for project: :275-292 (`select_record`).
- load manifest `<installPath>/enforcement.json`: :294-325 (`load_manifest`; file read at :295).
- phase/declares check: :327-339.
- resolve pipeline (phase, sentinel claim, GATE 1/2/3, EMIT): :496-607; emit text via `M.pointer` :377-383.
- The pipeline is called from enforce.lua main :116-153 and from explain :65.
- Top-level `enforcement.json` is read per installPath: no other path.

`--explain` (enforce-doctor skill, /Users/hiraq/Projects/airsstack/claudestacks/plugins/claudestacks/skills/enforce-doctor/SKILL.md):
- Runs `enforce.lua --explain "$ARGUMENTS"` (SKILL.md:10-16) with `airsl run --policy confined --allow-env HOME --allow-env TMPDIR --allow-env AIRSSTACK_HOME --allow-env AIRSSTACK_ENFORCE_REGISTRY --allow-read / --allow-write "${TMPDIR:-/tmp}" "${CLAUDE_PLUGIN_ROOT}/hooks/enforce.lua" --explain "$ARGUMENTS"` (SKILL.md:11-15; no --fail-open).
- enforce.lua:155-160 dispatches to `explain()` (:47-112), prints a trace via `airsstack.stdio.write` (:160): runtime, path, cwd, registry, sdd root, sentinel dir, held sentinels, trace lines from resolve, `outcome: N pointer(s)`, parity section (:83-109).
- The skill reads back outcome, deciding stage (GATE 1/2/3, no manifest, etc.), and parity (SKILL.md:26-58).

## 4. Skills that tell the model to run a command

| skill (SKILL.md) | line(s) | command verbatim |
|---|---|---|
| claudestacks/skills/enforce-doctor | :10-16 | `airsl run --policy confined --allow-env HOME --allow-env TMPDIR --allow-env AIRSSTACK_HOME --allow-env AIRSSTACK_ENFORCE_REGISTRY --allow-read / --allow-write "${TMPDIR:-/tmp}" "${CLAUDE_PLUGIN_ROOT}/hooks/enforce.lua" --explain "$ARGUMENTS"` (multi-line, backslash-continued) |
| claudestacks/skills/statusline | :69 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/install-airsl.sh"` (instruction text) |
| claudestacks/skills/statusline | :79-83 | `airsl run --policy confined --allow-env HOME --allow-exec airsl --allow-read "$HOME/.claude" --allow-read "${CLAUDE_PLUGIN_ROOT}" "${CLAUDE_PLUGIN_ROOT}/scripts/statusline-install.lua" dry-run` |
| claudestacks/skills/statusline | :89-93 | `airsl run --policy confined --allow-env HOME --allow-exec airsl --allow-read "$PWD" --allow-read "${CLAUDE_PLUGIN_ROOT}" "${CLAUDE_PLUGIN_ROOT}/scripts/statusline-install.lua" dry-run --project` |
| claudestacks/skills/statusline | :112-117 | `airsl run --policy confined --allow-env HOME --allow-exec airsl --allow-read "$HOME/.claude" --allow-write "$HOME/.claude" --allow-read "${CLAUDE_PLUGIN_ROOT}" ... "${CLAUDE_PLUGIN_ROOT}/scripts/statusline-install.lua" apply` |
| claudestacks/skills/statusline | :123-128 | `airsl run --policy confined --allow-env HOME --allow-exec airsl --allow-read "$PWD" --allow-write "$PWD/.claude" --allow-read "${CLAUDE_PLUGIN_ROOT}" ... "statusline-install.lua" apply --project` |
| claudestacks/skills/statusline | :150-153 | `airsl run --policy confined --allow-env HOME --allow-read "$HOME/.claude" --allow-write "$HOME/.claude" "${CLAUDE_PLUGIN_ROOT}/scripts/statusline-install.lua" uninstall` |
| claudestacks/skills/statusline | :159-162 | `airsl run --policy confined --allow-env HOME --allow-read "$PWD" --allow-write "$PWD/.claude" "${CLAUDE_PLUGIN_ROOT}/scripts/statusline-install.lua" uninstall --project` |
| claudestacks/skills/discuss | :27-29 | `airsl run --policy confined --allow-env HOME --allow-env AIRSSTACK_HOME --allow-read / --allow-write <root> "${CLAUDE_PLUGIN_ROOT}/scripts/discuss.lua" <command> [argument] --session ${CLAUDE_SESSION_ID}` |
| claudestacks/skills/context-handoff | :75-76 | `airsl run --policy confined --allow-read / "${CLAUDE_PLUGIN_ROOT}/scripts/handoff_report.lua" <path-to-report>` |
| claudestacks-sdlc/skills/adr | :30 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/status.sh"` |
| claudestacks-sdlc/skills/adr | :64 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/fingerprint.sh" <file>` |
| claudestacks-sdlc/skills/plan | :30 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/status.sh"` |
| claudestacks-sdlc/skills/plan | :45 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/fingerprint.sh" <authority file>` |
| claudestacks-sdlc/skills/design | :28 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/status.sh"` |
| claudestacks-sdlc/skills/design | :44 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/fingerprint.sh"` (line break before arg) |
| claudestacks-sdlc/skills/roadmap | :50 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/status.sh"` |
| claudestacks-sdlc/skills/milestone | :31 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/status.sh"` |
| claudestacks-sdlc/skills/execute | :32 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/status.sh"` |
| claudestacks-sdlc/skills/execute | :71 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/fingerprint.sh" <authority>` |
| claudestacks-sdlc/skills/status | :8 (`!` preprocessing) | `` !`sh "${CLAUDE_PLUGIN_ROOT}/scripts/status.sh"` `` |
| claudestacks-sdlc/skills/status | :29 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/fingerprint.sh" <file>` |
| claudestacks-sdlc/skills/intent | :34 | `sh "${CLAUDE_PLUGIN_ROOT}/scripts/status.sh"` |
| claudestacks-journal/skills/journal-capture | :30-33 | `airsl run --policy confined --allow-env CLAUDE_CONFIG_DIR --allow-env HOME --allow-env PWD --allow-read . --allow-read "$HOME/.claude" "${CLAUDE_PLUGIN_ROOT}/scripts/transcript-path.lua" "${CLAUDE_SESSION_ID}"` |
| claudestacks-journal/skills/journal-capture | :42-44 | `airsl run --policy confined --allow-read . "${CLAUDE_PLUGIN_ROOT}/scripts/project-key.lua"` |
| claudestacks-journal/skills/journal-capture | :59-61 | `airsl run --policy confined --allow-env AIRSSTACK_HOME --allow-env HOME --allow-write "$HOME_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/provision.lua"` |
| claudestacks-journal/skills/journal-link | :33-35 | `airsl run --policy confined --allow-read . "${CLAUDE_PLUGIN_ROOT}/scripts/project-key.lua"` |
| claudestacks-journal/skills/journal-note | :30-33 | `airsl run --policy confined --allow-env CLAUDE_CONFIG_DIR --allow-env HOME --allow-env PWD --allow-read . --allow-read "$HOME/.claude" "${CLAUDE_PLUGIN_ROOT}/scripts/transcript-path.lua" "${CLAUDE_SESSION_ID}"` |
| claudestacks-journal/skills/journal-note | :41-43 | `airsl run --policy confined --allow-read . "${CLAUDE_PLUGIN_ROOT}/scripts/project-key.lua"` |
| claudestacks-journal/skills/journal-note | :58-60 | `airsl run --policy confined --allow-env AIRSSTACK_HOME --allow-env HOME --allow-write "$HOME_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/provision.lua"` |
| claudestacks-journal/skills/journal-helped | :25-28 | `airsl run --policy confined --allow-env AIRSSTACK_HOME --allow-env HOME --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/bump-helped.lua" "<stem>"` |
| claudestacks-journal/skills/journal-recall | :27-29 | `airsl run --policy confined --allow-read . "${CLAUDE_PLUGIN_ROOT}/scripts/project-key.lua"` |
| claudestacks-journal/skills/journal-setup | :16-18 | `airsl run --policy confined --allow-env AIRSSTACK_HOME --allow-env HOME --allow-write "$HOME_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/provision.lua"` |
| claudestacks-journal/skills/journal-setup | :25-28 | `airsl run --policy confined --allow-env AIRSSTACK_HOME --allow-env HOME --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/build-index.lua"` |
| claudestacks-journal/skills/journal-review | :27-30 | `airsl run --policy confined --allow-read . "${CLAUDE_PLUGIN_ROOT}/scripts/project-key.lua"` |
| claudestacks-journal/skills/journal-review | :44-47 | `airsl run --policy confined --allow-env AIRSSTACK_HOME --allow-env HOME --allow-env AIRSSTACK_JOURNAL_BACKUP_KEEP --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" --allow-exec tar "${CLAUDE_PLUGIN_ROOT}/scripts/journal-backup.lua"` |
| claudestacks-journal/skills/journal-review | :58-61 | `airsl run --policy confined --allow-env AIRSSTACK_HOME --allow-env HOME --allow-env AIRSSTACK_JOURNAL_HUB_DEGREE --allow-read "$HOME_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/graph-health.lua" > "${TMPDIR:-/tmp}/journal-health.md"` |
| claudestacks-journal/skills/journal-review | :79-82 | `airsl run --policy confined --allow-env AIRSSTACK_HOME --allow-env HOME --allow-read "$HOME_ROOT" --allow-write "$HOME_ROOT" "${CLAUDE_PLUGIN_ROOT}/scripts/build-index.lua"` |

Skills that run no command: all others (agents/skills text only). Skills whose airsl call is inside a `sh` fence (journal-setup, journal-capture, journal-note, journal-review) are shown with their fence; see SKILL.md.

## 5. setup (sdlc) and journal-setup

- claudestacks-sdlc/skills/setup/SKILL.md (frontmatter :1-5, steps :11-23): creates `.claudestacks/sdlc/prds/` and `.claudestacks/sdlc/rfcs/` (:11-12) under the cwd repo; `.gitkeep` in each if empty (:13-14); creates `.claudestacks/sdlc/REVIEW.md` from the fenced template in `${CLAUDE_PLUGIN_ROOT}/references/review-policy.md` if absent (:15-19; file exists at claudestacks-sdlc/references/review-policy.md); does not write `.gitignore` (:20-21). No command, model-performed only.
- claudestacks-journal/skills/journal-setup/SKILL.md: runs provision.lua (:16-18) then build-index.lua (:25-28). provision.lua (claudestacks-journal/scripts/provision.lua:8-13) creates `daily sessions notes mocs .index` under the vault root, `vault.root()` = `$AIRSSTACK_HOME/journal` else `$HOME/.airsstack/journal` (lib/vault.lua:124-131). Prints `journal: vault provisioned at <root>` (provision.lua:15). Report line :31-33 names `${AIRSSTACK_HOME:-~/.airsstack}/journal/`.

## 6. user_config / CLAUDE_PLUGIN_OPTION_*

- claudestacks/.claude-plugin/plugin.json:19-31 defines `userConfig.style_reinject` (boolean, default true) and `userConfig.archive_keep` (number, default 20).
- claudestacks/hooks/style.sh:30 `--allow-env CLAUDE_PLUGIN_OPTION_STYLE_REINJECT`
- claudestacks/scripts/style.lua:8 (comment) `--allow-env CLAUDE_PLUGIN_OPTION_STYLE_REINJECT`
- claudestacks/scripts/style.lua:20 `pcall(airsstack.env.get, "CLAUDE_PLUGIN_OPTION_STYLE_REINJECT")`
- claudestacks/scripts/lib/style.lua:34 (comment) `CLAUDE_PLUGIN_OPTION_STYLE_REINJECT`
- claudestacks/skills/discuss/SKILL.md:43 `${user_config.archive_keep}` placeholder in a table cell ("` start --keep '${user_config.archive_keep}'`")
- claudestacks/scripts/lib/discuss.lua:19 (comment) `${user_config.archive_keep}`
- Test-only: claudestacks/scripts/style_test.lua:63 `"${user_config.style_reinject}"`; claudestacks/scripts/discuss_test.lua:46 `"${user_config.archive_keep}"`; claudestacks/scripts/discuss_skill_test.lua:23,26,28 (checks single-quoting of `${user_config.archive_keep}` in discuss SKILL.md).
- No `${user_config.*}` in claudestacks-sdlc, claudestacks-journal, guideline-rust, guideline-python; they have no userConfig block.

## 7. User-facing namespaced names embedded in Lua/sh (not markdown)

- claudestacks/hooks/preflight.sh:18 `'Reminder: run /claudestacks:snapshot-load for the current git branch ...'` (sh echo)
- claudestacks/hooks/hooks.json:36 `Reminder: run /claudestacks:snapshot-save ...` (SessionEnd echo; json, not Lua/sh)
- claudestacks/scripts/discuss.lua:1 (comment) `/claudestacks:discuss command line.`
- claudestacks/scripts/lib/discuss.lua:1 (comment) `... behind /claudestacks:discuss.`
- claudestacks-sdlc/scripts/lib/status.lua:825 `"claudestacks-sdlc: %d open, %d need attention — full board: /claudestacks-sdlc:status"` (emitted by M.attention path, used by H2)
- claudestacks/hooks/enforce_test.lua:18 `skill = "claudestacks-guideline-rust:rust-guidelines"` (test fixture)
- Test-fixture copies of the status string: claudestacks-sdlc/scripts/status_test.lua:1556, 1579, 1602, 1619, 1763, 1782.
- Agent names (`claudestacks:coder|explorer|reviewer`, etc.) appear only in hooks.json:92 matcher; no agent name in any Lua/sh string.

divergence:
- claudestacks/hooks/enforce.sh:9-12 comment says `git` is "the only program it can run"; enforce.sh:37-41 has no `--allow-exec git`; claudestacks/hooks/enforce.lua:17-20 says "No `--allow-exec git`". Locations only.
- claudestacks/hooks/enforce.lua:11-15 comment grants show `--allow-write "$TMPDIR"`; enforce.sh:40 uses `--allow-write "$SENTINELS"` (SENTINELS=`${TMPDIR:-/tmp}`, enforce.sh:35). rearm.lua:9-11 comment vs rearm.sh:32 same pattern.
- claudestacks/skills/enforce-doctor/SKILL.md:11-15 omits `--fail-open` present at enforce.sh:37.
- claudestacks/hooks/lib/enforce.lua:48-53 defines `M.git` (`proc.run` of git); no call site in that file (grep `M.git(` returns only the definition).

## 8. Lua tests (every *_test.lua) and requires

Resolution: `require` resolves relative to the script's own directory (claudestacks/scripts/handoff_check_hook.lua:6-7 comment; handoff_check_hook.lua is the source of that claim). Module `lib.X` = `<dir of test>/lib/X.lua`. For claudestacks/hooks tests, `lib.X` = `claudestacks/hooks/lib/X.lua`.

claudestacks (hooks/):
- hooks/enforce_test.lua — requires lib.enforce (:9) → hooks/lib/enforce.lua
- hooks/globs_test.lua — requires lib.globs (:5) → hooks/lib/globs.lua
claudestacks (scripts/):
- scripts/comm_report_test.lua — lib.comm_report (:5) → scripts/lib/comm_report.lua; lib.handoff_report (:8) → scripts/lib/handoff_report.lua
- scripts/discuss_test.lua — lib.discuss (:5) → scripts/lib/discuss.lua
- scripts/discuss_skill_test.lua — no require (reads SKILL.md text)
- scripts/handoff_report_test.lua — lib.handoff_report (:5); also lib.handoff (:51,:139 inside tests) → scripts/lib/handoff.lua
- scripts/handoff_test.lua — lib.handoff (:10) → scripts/lib/handoff.lua
- scripts/statusline_install_test.lua — `require("statusline-install")` (:10) → scripts/statusline-install.lua (top-level module in scripts/)
- scripts/statusline_test.lua — lib.statusline (:5) → scripts/lib/statusline.lua
- scripts/style_test.lua — lib.style (:5) → scripts/lib/style.lua
claudestacks-sdlc (scripts/):
- scripts/adr_test.lua — lib.adr (:9), lib.fingerprint (:10), lib.status (:11) → scripts/lib/{adr,fingerprint,status}.lua
- scripts/fingerprint_test.lua — lib.fingerprint (:9)
- scripts/roadmap_test.lua — lib.roadmap (:8), lib.status (:9)
- scripts/status_test.lua — lib.status (:9), lib.fingerprint (:10)
claudestacks-journal (scripts/):
- scripts/daily_test.lua — lib.daily (:5)
- scripts/health_test.lua — lib.health (:5), lib.vault (:6)
- scripts/index_test.lua — lib.index (:5), lib.vault (:6)
- scripts/notes_test.lua — lib.backup (:5), lib.notes (:6)
- scripts/orientation_test.lua — lib.orientation (:5)
- scripts/vault_test.lua — lib.vault (:5)
guideline-rust / guideline-python: no *_test.lua.

## 9. Frontmatter

Keys per file (values verbatim; `description` text is listed below the table).

| file | keys |
|---|---|
| claudestacks/agents/coder.md | name=coder; description; tools=[Read, Edit, Write, Grep, Glob, Bash, Skill]; model=sonnet; effort=high |
| claudestacks/agents/explorer.md | name=explorer; description; tools=[Read, Grep, Glob, Bash, Write]; model=haiku; effort=low |
| claudestacks/agents/reviewer.md | name=reviewer; description; tools=[Read, Grep, Glob, Bash, Skill, Write]; model=opus; effort=high |
| claudestacks-sdlc/agents/artifact-reviewer.md | name=artifact-reviewer; description; tools=[Read, Grep, Glob, Bash, Write]; model=opus; effort=high |
| claudestacks-sdlc/agents/chain-reader.md | name=chain-reader; description; tools=[Read, Grep, Glob, Bash, Write]; model=haiku; effort=low |
| claudestacks-sdlc/agents/task-briefer.md | name=task-briefer; description; tools=[Read, Grep, Glob, Bash, Write]; model=sonnet; effort=low |
| claudestacks-journal/agents/journal-capture.md | name=journal-capture; description; tools=[Read, Write, Edit, Bash, Glob, Grep]; model=sonnet |
| claudestacks-journal/agents/journal-curator.md | name=journal-curator; description; tools=[Read, Glob, Grep, Edit, Write, Bash]; model=opus |
| claudestacks-journal/agents/journal-note.md | name=journal-note; description; tools=[Read, Write, Edit, Bash, Glob, Grep]; model=sonnet |
| claudestacks-journal/agents/journal-recall.md | name=journal-recall; description; tools=[Read, Bash, Glob, Grep]; model=sonnet |
| claudestacks/skills/discuss/SKILL.md | name, description, argument-hint="[list \| list archive \| <id> \| <sid8>/<id> \| done]", disable-model-invocation=true |
| claudestacks/skills/statusline/SKILL.md | name, description (multi-line), disable-model-invocation=true |
| claudestacks-sdlc/skills/setup/SKILL.md | name, description, disable-model-invocation=true |
| claudestacks-journal/skills/journal-setup/SKILL.md | name, description, disable-model-invocation=true |
| all other SKILL.md files | name, description only |

Skill and agent description values (one line each, verbatim; multi-line YAML folded to one line):
- coder: Scoped implementer. Executes ONE bounded task end-to-end with strict test-driven development (test-first, red-green-refactor), enforces the active stack's guidelines, runs that stack's Definition of Done to green, and leaves the changes in the working tree. Multi-file OK. NEVER commits. Use to write or modify code for a task with a clear target.
- explorer: Read-only code locator and navigator. Answers "where is X defined", "what calls Y", "list all uses of Z", "map this directory" — returns compact file:line tables. HARD-REFUSES evaluation, judgment, bug-hunting, and fix suggestions. PREFER THIS for any broad or multi-file locating — finding where something lives, mapping an unfamiliar directory, sweeping for all uses — whether or not an implementation follows; it returns compact tables instead of dumping file bodies into the main context. Skip it for a single known-path read or when the next step needs judgment (explorer refuses that).
- reviewer: Merged code + spec reviewer. Independently RE-RUNS the active stack's Definition of Done, reviews the diff for correctness and against the stack's guidelines, AND reviews the delivered work against the spec/plan intent (scope drift, silent carry-over, missing or unauthorized requirements). One combined report, severity-tagged, report-only, no fixes. Use before a diff reaches the user for commit.
- artifact-reviewer: Reviews a draft claudestacks-sdlc artifact — a spec, a set of plans, or an ADR — against its upstream authority and the plugin's artifact-review criteria. Severity-tagged findings, report-only: never edits the artifact, never flips a status, never commits. Spawned by the design and plan skills at the intent→spec and spec→plan hops, and by the adr skill on a draft ADR.
- chain-reader: Mechanical section extractor over claudestacks-sdlc chain artifacts. Given a glob and a heading, returns each match's content VERBATIM, tagged with the file it came from, and nothing else. Does not group, categorize, count, deduplicate, or judge. Use to pull one named section out of many chain files without loading their bodies into the main context.
- task-briefer: Mechanical brief extractor over a claudestacks-sdlc plan file. Two modes: `ledger` returns one compact row per task in the plan; `brief` writes ONE task's complete verbatim content to a handoff file and returns only its summary. Also lists, without judging, every fact the task asserts about the codebase or an outside system. HARD-REFUSES judgment, implementation, and running the plan's commands. Use so a whole plan never enters the main thread's context.
- journal-capture (agent): Isolated session-story writer for the claudestacks-journal vault. Reads the session transcript JSONL and distils a grounded storytelling session note into sessions/session-<id8>.md by full overwrite, links spun-off notes, links the session into the daily note, refreshes the index, and returns a one-line receipt. Manual, model-driven, single-purpose. NEVER commits.
- journal-curator: Isolated review/distill curator for the claudestacks-journal vault. Reads the derived index, a graph-health report, and selected note bodies, then applies ADDITIVE-ONLY tidying — MOC index notes, a TL;DR layer on long notes, a daily narrative, typed depends-on/supersedes frontmatter, and high-confidence missing links. Never deletes or overwrites existing prose; never touches sessions/; never rebuilds the index; never commits; spawns nothing. Reports in the claudestacks Context Handoff summary/detail schema, to a single temp file — no session is minted.
- journal-note (agent): Isolated atomic-note writer for the claudestacks-journal vault. Reads the session transcript for a named topic and writes/updates ONE standalone insight|decision note in notes/, stamping the session join field, linking it into the daily note, refreshing the index, and returning a one-line receipt. Update-in-place on stem collision. Manual, single-purpose. NEVER commits.
- journal-recall (agent): Isolated recall reader for the claudestacks-journal vault. Reads ONLY the derived index (.index/index.json and tags.json) — never note bodies — ranks notes against a query by tag/domain match, summary/title text, the helped counter, and graph proximity, and returns a capped list of pointers (stem · summary · path · why). Two modes: recall (free query) and link (suggest link targets for a subject note). Read-only. NEVER writes, NEVER commits.
- context-handoff: Use when driving subagents that report through the filesystem — deciding where reports land, running a handoff session, and handing each agent the resolved path to the protocol. Invoke once at the top of any pipeline that spawns reporting agents, before the first spawn.
- discuss: Open, browse and close a topic-indexed discussion. Agents briefed under it hand back a short summary plus numbered topics, the detail stays on disk until a topic is opened, and a closed discussion is archived per project. Runs only when the author types /claudestacks:discuss.
- enforce-doctor: Explain why the claudestacks enforcement dispatcher does or does not fire for a given file path — runs enforce.lua --explain and reads the trace back. Read-only diagnostic. Use when a guideline pointer failed to appear for a file, when one appeared unexpectedly, or when the user says "/claudestacks:enforce-doctor <path>".
- orchestrate: Use when driving a scoped implementation task through the review pipeline — runs coder → reviewer (with explorer to locate code first) from the main thread and holds the user commit gate. Invoke when a change is substantial enough to warrant test-driven implementation plus an independent review before commit. Soft-coupled to the claudestacks agents; if they are not installed, fall back to doing the work inline and tell the user.
- process-guidelines: Use when committing or delegating work to subagents — supplies the project's process conventions, namely Conventional Commits with workspace-aware scopes, model-routing for delegated agents (judge vs execute vs trivia), and the agent-orchestration flow. Read the matching reference before applying a convention.
- snapshot-load: Use at the start of work on a branch, after /clear, or when the user says "load snapshot" / "load from memory" / "where was I" — reads the project-local snapshot index and fully reads the relevant snapshots, then reports the rehydrated state. No-arg loads the current branch's latest; an explicit topic argument switches to a branch-agnostic topic search across all snapshots (e.g. pull a snapshot from another session/branch by keyword).
- snapshot-save: Use when ending a work session, before /clear, or when the user says "save a snapshot" / "save to memory" / "snapshot save" — captures a conversation snapshot (session summary + key snippets) into the project-local snapshot store, with a light durability gate so thin sessions write nothing. No-arg captures the whole session; an explicit topic argument focuses the capture on that topic and stamps it as a match key for topic-load.
- statusline: Install, inspect, or remove the claudestacks Lua status line for Claude Code — a two-line status line showing path, git branch, context meter, output tokens, model, and rate limits. Installs machine-wide into ~/.claude, or for one project only via .claude/settings.local.json. Gates on the airsl binary, refuses to overwrite a status line it does not own, and backs up settings. Deterministic, no subagent. Use when the user says "install the status line" / "/claudestacks:statusline", or asks to remove it.
- adr: Use to record or change an architecture decision — write, approve, supersede, deprecate, amend, drop or un-drop an ADR, or finish a half-done supersede. Invoke with an ADR number or `new`.
- design: Use when a chain's intent is approved and the design is not yet settled — settles every question, edge case and fact with the author one question at a time, proves each claim with a run or citation, and writes an approved spec for the plan skill.
- distill: Use after chains complete to turn review findings that recur across chains into agent-config edits.
- execute: Use when a chain has an approved plan to carry out — builds it through parallel claudestacks coder/reviewer spawns, never asks the author anything, re-proves nothing, and reports once at the end. Invoke by chain/plan reference (e.g. "execute 2026-08-24-webhook-reliability/01").
- intent: Use before designing or building anything new — captures a problem, converts a PRD or RFC, or revisits an intent to approve, amend, drop or un-drop it.
- milestone: Use to work on one milestone of a roadmap — add, approve, amend, drop, un-drop, park scope, adopt or release a chain, or roll finished work up to done. Invoke with `<roadmap>/<NN>` or a roadmap name.
- plan: Use when a chain's spec is approved (or its intent carries spec: skipped) — refuses a spec with unproven claims, derives a plan set covering every spec requirement (files, order of work, risks, named tests) without asking or writing code, and asks once for the whole set to be accepted.
- roadmap: Use when work is larger than one chain — creates a roadmap and its first milestones under .claudestacks/sdlc/roadmaps/, or approves, amends, drops or un-drops an existing one. Optional.
- setup: Provision the committed .claudestacks/sdlc/ chain root (prds/, rfcs/, REVIEW.md). Idempotent, never overwrites. Use when the user says "/claudestacks-sdlc:setup" or asks to set up the SDLC chain in this repository.
- status: Render the claudestacks-sdlc chain status board. Use when the user says "/claudestacks-sdlc:status", asks what state the SDLC chains are in, or what to work on next.
- triage: Use by hand right after a failure surfaces (CI log, advisory, backtrace) to turn the evidence into a draft intent that enters the normal pipeline.
- journal-capture (skill): Manually capture the current session as a grounded storytelling note in the claudestacks-journal vault. Resolves the session id, transcript path, and project floor, then spawns the isolated journal-capture subagent to write sessions/session-<id8>.md by full overwrite. Re-run anytime to refresh the story. Use when the user says "capture this session" / "/journal-capture".
- journal-helped: Confirm that a recalled journal note actually aided the work — increments its helped: counter and refreshes the index via bump-helped.lua. Deterministic, no subagent. Use AFTER a note returned by /journal-recall helped solve the task, or when the user says "that note helped" / "/journal-helped <stem>".
- journal-link: Suggest existing journal notes to link from a note or topic, reusing the recall machinery. Given a stem, the main thread reads that ONE note to form the query; given free text, it uses that as the topic. Spawns the journal-recall subagent in mode=link (subject excluded) and returns link candidates (stem · summary · path · why). Use when the user says "suggest links for X" / "/journal-link X". The agent then edits the note to add the [[links]] it chooses.
- journal-note (skill): Manually capture one atomic, standalone topic note (insight|decision) into the claudestacks-journal vault. Resolves session id, transcript path, and project floor, then spawns the isolated journal-note subagent to write/update notes/<kebab>.md, stamping the session join field. Update-in-place on a repeated topic. Use when the user says "save a note on X" / "/journal-note X".
- journal-recall (skill): Recall prior journal notes as cheap ranked pointers instead of re-deriving them. Resolves the project floor, then spawns the isolated journal-recall subagent (mode=recall) to read ONLY the derived index and return pointers (stem · summary · path · why). Use BEFORE re-deriving something you may have noted before, or when the user says "recall X" / "/journal-recall X". The main thread reads at most the one note it picks.
- journal-review: Tidy the claudestacks-journal vault in one command. Backs the vault up, runs a deterministic graph-health report, then spawns the isolated opus journal-curator to apply additive-only tidying (MOCs, TL;DR layers, daily narrative, typed edges, missing links), and rebuilds the index. Use when the user says "review the journal" / "tidy the journal" / "/journal-review". Every change is additive and a backup precedes every run.
- journal-setup: Provision the claudestacks-journal vault and force a full rebuild of its derived recall index. Idempotent. Use when the user says "/claudestacks-journal:journal-setup" or asks to set up or rebuild the journal vault.
- rust-guidelines: Use when writing or editing Rust code or Cargo manifests — supplies the Rust Definition-of-Done (the exact cargo command gate every change must pass) and an index of engineering rules (strong types, static dispatch, module hygiene, doc discipline, unit-test mandate, the Microsoft pragmatic guidelines, and workspace layout). Read the matching reference before applying a rule.
- python-guidelines: Use when writing or editing Python code or pyproject.toml — supplies the Python Definition-of-Done (the exact uv, ruff, mypy and pytest command gate every change must pass) and an index of engineering rules (strict quality, modern typing, strong types, protocols and composition, `__init__.py` hygiene, modularity, docstring discipline, the test mandate, errors, idioms, project layout, uv workspaces, and the Google Python Style Guide). Read the matching reference before applying a rule.
