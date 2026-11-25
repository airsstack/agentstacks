# Probe results: pack inventory (C3)

Date: 2026-11-25. Repo: `$R` = `/Users/hiraq/.herdr/worktrees/agentstacks/extension-musecode`
(branch `extension/musecode`, at `33e3925`). Every block is a log written by `$P/scripts/run.sh`
(`$ <cmd>`, raw stdout+stderr, `[exit N]`), pasted byte for byte.
`$P` = `/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe`.

## C3.1 Hooks, agents, skills, `{{var}}` uses and `native/` files

The inventory script parses every `pack.toml` and `native/<tool>/hooks.toml` with Python's
`tomllib` (Python 3.12 via `uv`, isolated mode) and reads agent/skill frontmatter as `key: value`
lines between the `---` fences. `{{var <name>}}` is counted over every file under `packs/` except
`*_test.lua` (test files are excluded from the packs by `exclude = ["README.md", "**/*_test.lua"]`).

~~~~python
"""Pack inventory for the musecode spec. usage: python -I inventory.py <packs-dir>"""
import re
import sys
import tomllib
from collections import Counter
from pathlib import Path

root = Path(sys.argv[1])
PACKS = ["core", "sdlc", "journal", "guideline-rust", "guideline-python"]


def front(path):
    text = path.read_text()
    if not text.startswith("---\n"):
        return {}
    block = text[4:text.index("\n---", 4)]
    out = {}
    for line in block.splitlines():
        m = re.match(r"^([A-Za-z_-]+):\s*(.*)$", line)
        if m:
            out[m.group(1)] = m.group(2)
    return out


def hook_rows(label, hooks):
    for i, h in enumerate(hooks):
        print(
            f"{label}\t[{i}]\tevent={h.get('event')}\tkind={h.get('kind')}"
            f"\tmatch={h.get('match', '-')}\tagents={h.get('agents', '-')}"
            f"\tmay_block={h.get('may_block', '-')}\trun={h.get('run', '-')}"
            f"\targs={h.get('args', '-')}"
        )


print("== HOOKS (pack.toml and native/<tool>/hooks.toml)")
events = Counter()
for p in PACKS:
    hooks = tomllib.loads((root / p / "pack.toml").read_text()).get("hooks", [])
    hook_rows(f"{p}/pack.toml", hooks)
    for h in hooks:
        events[(h["event"], h["kind"])] += 1
    for native in sorted((root / p / "native").glob("*/hooks.toml")):
        hook_rows(str(native.relative_to(root)), tomllib.loads(native.read_text()).get("hooks", []))
print("-- neutral hooks per (event, kind)")
for (e, k), n in sorted(events.items()):
    print(f"{e}\t{k}\t{n}")
print(f"-- neutral total {sum(events.values())}")

print("\n== AGENTS")
for f in sorted(root.glob("*/agents/*.md")):
    fm = front(f)
    print(f"{f.parts[-3]}\tname={fm.get('name')}\tmodel={fm.get('model', '-')}"
          f"\teffort={fm.get('effort', '-')}\ttools={fm.get('tools', '-')}")

print("\n== SKILLS (neutral skills/ and native/<tool>/skills/)")
for f in sorted(list(root.glob("*/skills/*/SKILL.md")) + list(root.glob("*/native/*/skills/*/SKILL.md"))):
    fm = front(f)
    rel = f.relative_to(root)
    print(f"{rel.parts[0]}\t{rel.parent}\tname={fm.get('name')}\tinvocation={fm.get('invocation', '-')}"
          f"\targument_hint={'yes' if 'argument_hint' in fm else 'no'}"
          f"\tother_keys={sorted(k for k in fm if k not in ('name', 'description', 'invocation', 'argument_hint')) or '-'}")

print("\n== {{var <name>}} USES (all files under packs/, excluding *_test.lua)")
by_var, by_pack = Counter(), Counter()
for f in sorted(root.rglob("*")):
    if not f.is_file() or f.name.endswith("_test.lua"):
        continue
    try:
        text = f.read_text()
    except UnicodeDecodeError:
        continue
    for m in re.finditer(r"\{\{var ([a-z_]+)\}\}", text):
        rel = f.relative_to(root)
        by_var[m.group(1)] += 1
        by_pack[(rel.parts[0], m.group(1))] += 1
        line = text.count("\n", 0, m.start()) + 1
        print(f"{rel}:{line}\t{m.group(1)}")
print("-- per var")
for v, n in sorted(by_var.items()):
    print(f"{v}\t{n}")
print("-- per pack, var")
for (p, v), n in sorted(by_pack.items()):
    print(f"{p}\t{v}\t{n}")

print("\n== native/ DIRECTORIES")
for d in sorted(root.glob("*/native/*")):
    for f in sorted(x for x in d.rglob("*") if x.is_file()):
        print(f.relative_to(root))
~~~~

~~~~text
$ uv run --no-project --python 3.12 python -I /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/scripts/inventory.py /Users/hiraq/.herdr/worktrees/agentstacks/extension-musecode/packs
== HOOKS (pack.toml and native/<tool>/hooks.toml)
core/pack.toml	[0]	event=session.start	kind=message	match=['startup', 'resume', 'clear']	agents=-	may_block=-	run=-	args=-
core/pack.toml	[1]	event=session.start	kind=lua	match=['compact']	agents=-	may_block=-	run=hooks/rearm.lua	args=-
core/pack.toml	[2]	event=session.start	kind=lua	match=-	agents=-	may_block=-	run=scripts/style.lua	args=['{{skill_dir discuss}}/references/protocol.md']
core/pack.toml	[3]	event=session.end	kind=message	match=-	agents=-	may_block=-	run=-	args=-
core/pack.toml	[4]	event=prompt.submit	kind=lua	match=-	agents=-	may_block=-	run=scripts/style.lua	args=['{{skill_dir discuss}}/references/protocol.md', '{{option style_reinject}}']
core/pack.toml	[5]	event=tool.before	kind=lua	match=['read', 'edit', 'write']	agents=-	may_block=-	run=hooks/enforce.lua	args=['{{root}}']
core/pack.toml	[6]	event=tool.after	kind=lua	match=['write']	agents=-	may_block=True	run=scripts/handoff_check_hook.lua	args=-
core/pack.toml	[7]	event=tool.after	kind=lua	match=['write']	agents=-	may_block=True	run=scripts/comm_check_hook.lua	args=-
core/pack.toml	[8]	event=agent.stop	kind=lua	match=-	agents=['coder', 'explorer', 'reviewer']	may_block=True	run=scripts/handoff_check_hook.lua	args=-
core/pack.toml	[9]	event=agent.stop	kind=lua	match=-	agents=-	may_block=True	run=scripts/comm_check_hook.lua	args=-
core/native/claude/hooks.toml	[0]	event=PreToolUse	kind=lua	match=['^SubagentHandback$']	agents=-	may_block=True	run=scripts/handoff_check_hook.lua	args=-
core/native/claude/hooks.toml	[1]	event=PreToolUse	kind=lua	match=['^SubagentHandback$']	agents=-	may_block=True	run=scripts/comm_check_hook.lua	args=-
sdlc/pack.toml	[0]	event=session.start	kind=lua	match=-	agents=-	may_block=-	run=scripts/attention.lua	args=['{{skill status}}']
sdlc/pack.toml	[1]	event=agent.stop	kind=lua	match=-	agents=['task-briefer', 'chain-reader', 'artifact-reviewer']	may_block=True	run={{root core}}/scripts/handoff_check_hook.lua	args=-
journal/pack.toml	[0]	event=session.start	kind=lua	match=['startup', 'resume', 'clear']	agents=-	may_block=-	run=scripts/session-start.lua	args=-
journal/pack.toml	[1]	event=agent.stop	kind=lua	match=-	agents=['curator']	may_block=True	run={{root core}}/scripts/handoff_check_hook.lua	args=-
-- neutral hooks per (event, kind)
agent.stop	lua	4
prompt.submit	lua	1
session.end	message	1
session.start	lua	4
session.start	message	1
tool.after	lua	2
tool.before	lua	1
-- neutral total 14

== AGENTS
core	name=coder	model=balanced	effort=high	tools=[read, edit, write, grep, glob, shell, skill]
core	name=explorer	model=fast	effort=low	tools=[read, grep, glob, shell, write]
core	name=reviewer	model=deep	effort=high	tools=[read, grep, glob, shell, skill, write]
journal	name=capture	model=balanced	effort=-	tools=[read, write, edit, shell, glob, grep]
journal	name=curator	model=deep	effort=-	tools=[read, glob, grep, edit, write, shell]
journal	name=note	model=balanced	effort=-	tools=[read, write, edit, shell, glob, grep]
journal	name=recall	model=balanced	effort=-	tools=[read, shell, glob, grep]
sdlc	name=artifact-reviewer	model=deep	effort=high	tools=[read, grep, glob, shell, write]
sdlc	name=chain-reader	model=fast	effort=low	tools=[read, grep, glob, shell, write]
sdlc	name=task-briefer	model=balanced	effort=low	tools=[read, grep, glob, shell, write]

== SKILLS (neutral skills/ and native/<tool>/skills/)
core	core/native/claude/skills/core-statusline	name=core-statusline	invocation=-	argument_hint=no	other_keys=['disable-model-invocation']
core	core/skills/context-handoff	name=context-handoff	invocation=-	argument_hint=no	other_keys=-
core	core/skills/discuss	name=discuss	invocation=user	argument_hint=yes	other_keys=-
core	core/skills/enforce-doctor	name=enforce-doctor	invocation=-	argument_hint=no	other_keys=-
core	core/skills/orchestrate	name=orchestrate	invocation=-	argument_hint=no	other_keys=-
core	core/skills/process-guidelines	name=process-guidelines	invocation=-	argument_hint=no	other_keys=-
core	core/skills/snapshot-load	name=snapshot-load	invocation=-	argument_hint=no	other_keys=-
core	core/skills/snapshot-save	name=snapshot-save	invocation=-	argument_hint=no	other_keys=-
guideline-python	guideline-python/skills/guidelines	name=guidelines	invocation=-	argument_hint=no	other_keys=-
guideline-rust	guideline-rust/skills/guidelines	name=guidelines	invocation=-	argument_hint=no	other_keys=-
journal	journal/skills/capture	name=capture	invocation=-	argument_hint=no	other_keys=-
journal	journal/skills/helped	name=helped	invocation=-	argument_hint=no	other_keys=-
journal	journal/skills/link	name=link	invocation=-	argument_hint=no	other_keys=-
journal	journal/skills/note	name=note	invocation=-	argument_hint=no	other_keys=-
journal	journal/skills/recall	name=recall	invocation=-	argument_hint=no	other_keys=-
journal	journal/skills/review	name=review	invocation=-	argument_hint=no	other_keys=-
journal	journal/skills/setup	name=setup	invocation=user	argument_hint=no	other_keys=-
sdlc	sdlc/skills/adr	name=adr	invocation=-	argument_hint=no	other_keys=-
sdlc	sdlc/skills/design	name=design	invocation=-	argument_hint=no	other_keys=-
sdlc	sdlc/skills/distill	name=distill	invocation=-	argument_hint=no	other_keys=-
sdlc	sdlc/skills/execute	name=execute	invocation=-	argument_hint=no	other_keys=-
sdlc	sdlc/skills/intent	name=intent	invocation=-	argument_hint=no	other_keys=-
sdlc	sdlc/skills/milestone	name=milestone	invocation=-	argument_hint=no	other_keys=-
sdlc	sdlc/skills/plan	name=plan	invocation=-	argument_hint=no	other_keys=-
sdlc	sdlc/skills/roadmap	name=roadmap	invocation=-	argument_hint=no	other_keys=-
sdlc	sdlc/skills/setup	name=setup	invocation=user	argument_hint=no	other_keys=-
sdlc	sdlc/skills/status	name=status	invocation=-	argument_hint=no	other_keys=-
sdlc	sdlc/skills/triage	name=triage	invocation=-	argument_hint=no	other_keys=-

== {{var <name>}} USES (all files under packs/, excluding *_test.lua)
core/native/claude/skills/core-statusline/SKILL.md:68	project_dir
core/native/claude/skills/core-statusline/SKILL.md:80	project_dir
core/native/claude/skills/core-statusline/SKILL.md:81	project_dir
core/native/claude/skills/core-statusline/SKILL.md:82	project_dir
core/native/claude/skills/core-statusline/SKILL.md:90	project_dir
core/native/claude/skills/core-statusline/SKILL.md:91	project_dir
core/native/claude/skills/core-statusline/SKILL.md:92	project_dir
core/native/claude/skills/core-statusline/SKILL.md:114	project_dir
core/native/claude/skills/core-statusline/SKILL.md:115	project_dir
core/native/claude/skills/core-statusline/SKILL.md:116	project_dir
core/native/claude/skills/core-statusline/SKILL.md:125	project_dir
core/native/claude/skills/core-statusline/SKILL.md:126	project_dir
core/native/claude/skills/core-statusline/SKILL.md:127	project_dir
core/native/claude/skills/core-statusline/SKILL.md:152	project_dir
core/native/claude/skills/core-statusline/SKILL.md:161	project_dir
core/pack.toml:35	tmpdir
core/pack.toml:36	tmpdir
core/pack.toml:68	tmpdir
core/skills/context-handoff/SKILL.md:12	project_dir
core/skills/context-handoff/SKILL.md:75	project_dir
core/skills/discuss/SKILL.md:10	arguments
core/skills/discuss/SKILL.md:12	project_dir
core/skills/discuss/SKILL.md:29	project_dir
core/skills/discuss/SKILL.md:29	session_id
core/skills/discuss/SKILL.md:33	project_dir
core/skills/discuss/SKILL.md:41	arguments
core/skills/discuss/SKILL.md:55	project_dir
core/skills/enforce-doctor/SKILL.md:6	arguments
core/skills/enforce-doctor/SKILL.md:13	tmpdir
core/skills/enforce-doctor/SKILL.md:14	project_dir
core/skills/enforce-doctor/SKILL.md:14	project_dir
core/skills/enforce-doctor/SKILL.md:14	arguments
core/skills/enforce-doctor/SKILL.md:17	arguments
journal/native/opencode/skills/journal-capture/session.md:17	tmpdir
journal/pack.toml:20	airsstack_home
journal/pack.toml:20	project_dir
journal/pack.toml:21	airsstack_home
journal/skills/capture/SKILL.md:21	session_id
journal/skills/capture/SKILL.md:21	project_dir
journal/skills/capture/SKILL.md:40	project_dir
journal/skills/capture/SKILL.md:57	project_dir
journal/skills/helped/SKILL.md:28	project_dir
journal/skills/link/SKILL.md:35	project_dir
journal/skills/note/SKILL.md:23	session_id
journal/skills/note/SKILL.md:23	project_dir
journal/skills/note/SKILL.md:41	project_dir
journal/skills/note/SKILL.md:58	project_dir
journal/skills/recall/SKILL.md:29	project_dir
journal/skills/review/SKILL.md:29	project_dir
journal/skills/review/SKILL.md:47	project_dir
journal/skills/review/SKILL.md:61	project_dir
journal/skills/review/SKILL.md:82	project_dir
journal/skills/setup/SKILL.md:18	project_dir
journal/skills/setup/SKILL.md:28	project_dir
sdlc/skills/adr/SKILL.md:14	project_dir
sdlc/skills/adr/SKILL.md:15	project_dir
sdlc/skills/adr/SKILL.md:30	project_dir
sdlc/skills/adr/SKILL.md:64	project_dir
sdlc/skills/adr/SKILL.md:95	project_dir
sdlc/skills/design/SKILL.md:19	project_dir
sdlc/skills/design/SKILL.md:20	project_dir
sdlc/skills/design/SKILL.md:28	project_dir
sdlc/skills/design/SKILL.md:44	project_dir
sdlc/skills/design/SKILL.md:109	project_dir
sdlc/skills/distill/SKILL.md:12	project_dir
sdlc/skills/execute/SKILL.md:25	project_dir
sdlc/skills/execute/SKILL.md:32	project_dir
sdlc/skills/execute/SKILL.md:71	project_dir
sdlc/skills/intent/SKILL.md:14	project_dir
sdlc/skills/intent/SKILL.md:15	project_dir
sdlc/skills/intent/SKILL.md:34	project_dir
sdlc/skills/milestone/SKILL.md:15	project_dir
sdlc/skills/milestone/SKILL.md:16	project_dir
sdlc/skills/milestone/SKILL.md:31	project_dir
sdlc/skills/plan/SKILL.md:20	project_dir
sdlc/skills/plan/SKILL.md:21	project_dir
sdlc/skills/plan/SKILL.md:30	project_dir
sdlc/skills/plan/SKILL.md:45	project_dir
sdlc/skills/plan/SKILL.md:107	project_dir
sdlc/skills/roadmap/SKILL.md:14	project_dir
sdlc/skills/roadmap/SKILL.md:15	project_dir
sdlc/skills/roadmap/SKILL.md:50	project_dir
sdlc/skills/setup/SKILL.md:17	project_dir
sdlc/skills/status/SKILL.md:8	project_dir
sdlc/skills/status/SKILL.md:15	project_dir
sdlc/skills/status/SKILL.md:29	project_dir
sdlc/skills/triage/SKILL.md:14	project_dir
sdlc/skills/triage/SKILL.md:15	project_dir
-- per var
airsstack_home	2
arguments	5
project_dir	73
session_id	3
tmpdir	5
-- per pack, var
core	arguments	5
core	project_dir	23
core	session_id	1
core	tmpdir	4
journal	airsstack_home	2
journal	project_dir	16
journal	session_id	2
journal	tmpdir	1
sdlc	project_dir	34

== native/ DIRECTORIES
core/native/claude/hooks.toml
core/native/claude/skills/core-statusline/SKILL.md
journal/native/claude/skills/journal-capture/session.md
journal/native/opencode/skills/journal-capture/session.md
[exit 0]

~~~~

Answer:
- Hooks: 14 neutral hooks (core 10, sdlc 2, journal 2; guideline packs none), plus 2 native hooks
  in `core/native/claude/hooks.toml` (both `PreToolUse`, `match = ["^SubagentHandback$"]`). Per
  event: `session.start` 5 (1 message + 4 lua), `session.end` 1 (message), `prompt.submit` 1 (lua),
  `tool.before` 1 (lua, `match = [read, edit, write]`), `tool.after` 2 (lua, `match = [write]`,
  both `may_block`), `agent.stop` 4 (lua, all `may_block`; 3 with `agents` lists, 1 for every
  agent). `may_block = true` on 6 neutral + 2 native hooks. Lua `run` targets:
  `hooks/rearm.lua`, `scripts/style.lua` (×2), `hooks/enforce.lua`, `scripts/handoff_check_hook.lua`
  (core ×2, plus sdlc and journal via `{{root core}}/scripts/handoff_check_hook.lua`),
  `scripts/comm_check_hook.lua` (×2), `scripts/attention.lua`, `scripts/session-start.lua`. Args use
  `{{skill_dir discuss}}`, `{{option style_reinject}}`, `{{root}}` and `{{skill status}}`.
- Agents: 10. `model`: balanced 5 (coder, capture, note, recall, task-briefer), deep 3 (reviewer,
  curator, artifact-reviewer), fast 2 (explorer, chain-reader). `effort`: high 3 (coder, reviewer,
  artifact-reviewer), low 3 (explorer, chain-reader, task-briefer), absent 4 (all journal agents);
  `medium` is unused. `tools`: every agent has `read`, `grep`, `glob`, `shell`; `write` 9 (all but
  recall); `edit` 4 (coder, capture, curator, note); `skill` 2 (coder, reviewer).
- Skills: 27 neutral (core 7, sdlc 11, journal 7, guideline-rust 1, guideline-python 1) plus 1
  native (`core/native/claude/skills/core-statusline`, which carries the Claude key
  `disable-model-invocation`). `invocation: user` on 3 (core/discuss, journal/setup, sdlc/setup);
  no skill sets `invocation: any` explicitly. `argument_hint` on 1 (core/discuss).
- `{{var}}`: 88 uses. `project_dir` 73, `tmpdir` 5, `arguments` 5, `session_id` 3,
  `airsstack_home` 2, `home` 0. Per pack: core 33 (project_dir 23, arguments 5, tmpdir 4,
  session_id 1), sdlc 34 (project_dir 34), journal 21 (project_dir 16, session_id 2,
  airsstack_home 2, tmpdir 1); guideline packs 0. Of these, 15 `project_dir` are in
  `core/native/claude/…/core-statusline/SKILL.md` and 1 `tmpdir` in
  `journal/native/opencode/…/session.md`; 6 are in `pack.toml` hook grants (tmpdir 3 in
  `core/pack.toml:35,36,68`; airsstack_home 2 and project_dir 1 in `journal/pack.toml:20-21`).
- `native/`: `core/native/claude/{hooks.toml, skills/core-statusline/SKILL.md}`,
  `journal/native/claude/skills/journal-capture/session.md`,
  `journal/native/opencode/skills/journal-capture/session.md`. No other tool has a `native/` dir.

## C3.2 The neutral format's allowed values (core source)

~~~~text
$ grep -n '=> "' crates/agentstacks/src/types/frontmatter/agent/tool.rs
99:            Self::Read => "read",
100:            Self::Write => "write",
101:            Self::Edit => "edit",
102:            Self::Grep => "grep",
103:            Self::Glob => "glob",
104:            Self::Shell => "shell",
105:            Self::Skill => "skill",
[exit 0]

$ grep -n '=> "' crates/agentstacks/src/types/frontmatter/model.rs
84:            Self::Fast => "fast",
85:            Self::Balanced => "balanced",
86:            Self::Deep => "deep",
[exit 0]

$ grep -n '=> "' crates/agentstacks/src/types/frontmatter/effort.rs
83:            Self::Low => "low",
84:            Self::Medium => "medium",
85:            Self::High => "high",
[exit 0]

$ grep -n '=> "' crates/agentstacks/src/types/frontmatter/invocation.rs
83:            Self::User => "user",
84:            Self::Any => "any",
[exit 0]

$ grep -n 'const KEYS' -A 7 crates/agentstacks/src/types/frontmatter/skill.rs crates/agentstacks/src/types/frontmatter/agent/meta.rs
crates/agentstacks/src/types/frontmatter/skill.rs:37:const KEYS: [&str; 5] = [
crates/agentstacks/src/types/frontmatter/skill.rs-38-    "name",
crates/agentstacks/src/types/frontmatter/skill.rs-39-    "description",
crates/agentstacks/src/types/frontmatter/skill.rs-40-    "invocation",
crates/agentstacks/src/types/frontmatter/skill.rs-41-    "argument_hint",
crates/agentstacks/src/types/frontmatter/skill.rs-42-    "native",
crates/agentstacks/src/types/frontmatter/skill.rs-43-];
crates/agentstacks/src/types/frontmatter/skill.rs-44-
--
crates/agentstacks/src/types/frontmatter/agent/meta.rs:41:const KEYS: [&str; 6] = ["name", "description", "model", "effort", "tools", "native"];
crates/agentstacks/src/types/frontmatter/agent/meta.rs-42-
crates/agentstacks/src/types/frontmatter/agent/meta.rs-43-/// An agent's checked frontmatter.
crates/agentstacks/src/types/frontmatter/agent/meta.rs-44-///
crates/agentstacks/src/types/frontmatter/agent/meta.rs-45-/// # Examples
crates/agentstacks/src/types/frontmatter/agent/meta.rs-46-///
crates/agentstacks/src/types/frontmatter/agent/meta.rs-47-/// ```
crates/agentstacks/src/types/frontmatter/agent/meta.rs-48-/// use std::path::Path;
[exit 0]

$ grep -n '"[a-z.]*" => Ok' crates/agentstacks/src/types/hooks/event.rs
67:            "session.start" => Ok(Self::SessionStart),
68:            "session.end" => Ok(Self::SessionEnd),
69:            "prompt.submit" => Ok(Self::PromptSubmit),
70:            "tool.before" => Ok(Self::ToolBefore),
71:            "tool.after" => Ok(Self::ToolAfter),
72:            "agent.stop" => Ok(Self::AgentStop),
[exit 0]

$ sed -n 77,97p crates/agentstacks/src/types/hooks/event.rs
    /// The `match` values this event accepts, or `None` when `match` is not allowed on it.
    #[must_use]
    pub const fn match_values(self) -> Option<&'static [MatchValue]> {
        match self {
            Self::SessionStart => Some(&[
                MatchValue::Startup,
                MatchValue::Resume,
                MatchValue::Clear,
                MatchValue::Compact,
            ]),
            Self::ToolBefore | Self::ToolAfter => {
                Some(&[MatchValue::Read, MatchValue::Edit, MatchValue::Write])
            }
            Self::SessionEnd | Self::PromptSubmit | Self::AgentStop => None,
        }
    }

    /// Whether the event takes an `agents` selector.
    #[must_use]
    pub const fn allows_agents(self) -> bool {
        matches!(self, Self::AgentStop)
[exit 0]

$ grep -n '=> "' crates/agentstacks/src/types/hooks/matcher.rs
84:            Self::Startup => "startup",
85:            Self::Resume => "resume",
86:            Self::Clear => "clear",
87:            Self::Compact => "compact",
88:            Self::Read => "read",
89:            Self::Edit => "edit",
90:            Self::Write => "write",
[exit 0]

$ grep -n '"message" =>\|"lua" =>' crates/agentstacks/src/types/hooks/action.rs
85:            "message" => MessageHook::read(fields, tokens).map(Self::Message),
86:            "lua" => LuaHook::read(fields, binaries, tokens).map(Self::Lua),
[exit 0]

$ grep -n '=> "' crates/agentstacks/src/types/tokens/var.rs crates/agentstacks/src/types/tokens/kind.rs
crates/agentstacks/src/types/tokens/var.rs:99:            Self::SessionId => "session_id",
crates/agentstacks/src/types/tokens/var.rs:100:            Self::Arguments => "arguments",
crates/agentstacks/src/types/tokens/var.rs:101:            Self::Tmpdir => "tmpdir",
crates/agentstacks/src/types/tokens/var.rs:102:            Self::Home => "home",
crates/agentstacks/src/types/tokens/var.rs:103:            Self::AirsstackHome => "airsstack_home",
crates/agentstacks/src/types/tokens/var.rs:104:            Self::ProjectDir => "project_dir",
crates/agentstacks/src/types/tokens/kind.rs:107:            Self::Skill => "skill",
crates/agentstacks/src/types/tokens/kind.rs:108:            Self::Agent => "agent",
crates/agentstacks/src/types/tokens/kind.rs:109:            Self::SkillDir => "skill_dir",
crates/agentstacks/src/types/tokens/kind.rs:110:            Self::Root => "root",
crates/agentstacks/src/types/tokens/kind.rs:111:            Self::Var => "var",
crates/agentstacks/src/types/tokens/kind.rs:112:            Self::Option => "option",
[exit 0]

$ sed -n 9,19p crates/agentstacks/src/types/hooks/native/hook.rs
//! - [`NativeHook`] holds a tool-specific event and `match` as opaque strings, and the same
//!   [`HookAction`] a neutral hook has.
//! - `NativeHook::read` reads one `[[hooks]]` item of `native/<tool>/hooks.toml`: `event` is
//!   required text, `match` is an optional list of strings, and the action, with its grants,
//!   policy, `may_block`, `exec` and `run` rules, is read exactly as for a neutral hook.
//! - `agents` is not accepted and is refused as an unknown key.
//!
//! # Non-responsibilities
//!
//! Whether the tool knows the event or the `match` value belongs to the tool's extension; the
//! core keeps both opaque.
[exit 0]

~~~~

Answer (all in `$R/crates/agentstacks/src/`):
- Agent `tools`: `read`, `write`, `edit`, `grep`, `glob`, `shell`, `skill` (`types/frontmatter/agent/tool.rs:99-105`).
- Agent `model`: `fast`, `balanced`, `deep` (`types/frontmatter/model.rs:84-86`).
- Agent `effort`: `low`, `medium`, `high` (`types/frontmatter/effort.rs:83-85`).
- Agent keys: `name`, `description`, `model`, `effort`, `tools`, `native` (`types/frontmatter/agent/meta.rs:41`).
- Skill `invocation`: `user`, `any` (`types/frontmatter/invocation.rs:83-84`). Skill keys: `name`, `description`, `invocation`, `argument_hint`, `native` (`types/frontmatter/skill.rs:37-43`).
- Hook events: `session.start`, `session.end`, `prompt.submit`, `tool.before`, `tool.after`, `agent.stop` (`types/hooks/event.rs:67-72`). `match`: `session.start` takes `startup|resume|clear|compact`; `tool.before`/`tool.after` take `read|edit|write`; the other three take none (`types/hooks/event.rs:79-91`, words at `types/hooks/matcher.rs:84-90`). `agents` only on `agent.stop` (`types/hooks/event.rs:96-97`).
- Hook kinds: `message`, `lua` (`types/hooks/action.rs:85-86`).
- Native hooks: `event` and `match` are opaque tool text; `agents` is refused (`types/hooks/native/hook.rs:9-19`).
- Token kinds: `skill`, `agent`, `skill_dir`, `root`, `var`, `option` (`types/tokens/kind.rs:107-112`); vars: `session_id`, `arguments`, `tmpdir`, `home`, `airsstack_home`, `project_dir` (`types/tokens/var.rs:99-104`).

## C3.3 How pack Lua hooks use `event.path`

~~~~text
$ grep -rn 'event\.path' packs --include='*.lua' | grep -v '_test.lua'
packs/core/hooks/enforce.lua:21:  local file_path = event.path
packs/core/scripts/lib/handoff_report.lua:110:  -- load-bearing for `tool.after`, which is handed an arbitrary `event.path`. Without it, every
packs/core/scripts/lib/handoff_report.lua:351:--   `tool.after`   `event.path`, tested against the handoff root directly — an ordinary Markdown
packs/core/scripts/lib/handoff_report.lua:376:    return qualifying(event.path, cwd)
[exit 0]

$ grep -rln 'event\.path\|\.path\b' packs/*/hooks packs/*/scripts --include='*_test.lua' | head; grep -rn 'path = "' packs/core/hooks/enforce_test.lua packs/core/scripts/handoff_check_hook_test.lua packs/core/scripts/comm_check_hook_test.lua | head -20
packs/core/hooks/enforce_test.lua
packs/core/scripts/handoff_test.lua
packs/core/scripts/discuss_test.lua
packs/core/scripts/handoff_report_test.lua
packs/core/scripts/statusline_install_test.lua
packs/core/scripts/comm_report_test.lua
packs/core/scripts/handoff_check_hook_test.lua
packs/core/scripts/comm_check_hook_test.lua
packs/journal/scripts/notes_test.lua
packs/journal/scripts/index_test.lua
[exit 0]

$ sed -n 20,24p packs/core/hooks/enforce.lua; sed -n 33,33p packs/core/hooks/enforce.lua
return function(event)
  local file_path = event.path
  if type(file_path) ~= "string" or file_path == "" then
    return nil
  end
  local cwd = type(event.cwd) == "string" and event.cwd ~= "" and event.cwd or path.absolute(".")
[exit 0]

$ sed -n 104,112p packs/core/hooks/lib/enforce.lua; sed -n 129,136p packs/core/hooks/lib/enforce.lua; sed -n 222,228p packs/core/hooks/lib/enforce.lua
-- Inside a repository: the path relative to the git toplevel, which is what `match` is documented
-- to mean. Outside any repository: the basename, which preserves coverage there rather than
-- silently dropping it.
function M.path_for_matching(file_path, cwd)
  local absolute = path.absolute(file_path)
  local target = M.realpath(absolute) or absolute

  local top = M.toplevel(cwd)
  if top then
-- Whether `file_path` is an SDD spec or plan under the HOME-global root.
--
-- The directory name must appear as a whole path SEGMENT. A substring check on '/specs/' matched
-- `<key>/a/specs/b/plans/c.md` by accident, which is a phase misclassification rather than a
-- cosmetic slip: it swaps the Definition of Done for the architecture rules.
function M.is_design_doc(file_path, home)
  local target = path.absolute(file_path)
  local root = path.absolute(M.sdd_root(home))
-- The file's directory, not the session's: searching upward from `cwd` is wrong for any file
-- outside it, and `cwd` survives only as the fallback when the path has no directory component.
function M.marker_active(file_path, markers, cwd)
  local directory = path.dirname(path.absolute(file_path))
  if directory == "" then
    directory = cwd or "."
  end
[exit 0]

$ sed -n 68,88p packs/core/scripts/lib/handoff_report.lua
-- Makes `file` absolute, resolving a relative path against `cwd` first, or nil when there is
-- nothing to resolve it against.
--
-- A relative path with no `cwd` is refused rather than falling back to the process's own working
-- directory: with that fallback the same relative handoff path would validate from one directory
-- and fail to open from another. The "Return & routing contract" section of
-- `skills/context-handoff/references/protocol.md` requires a session-tree path to be returned
-- relative to the worktree root, so resolving against the caller's `cwd` is the specified normal
-- case, not an edge case to approximate.
local function resolve_against(file, cwd)
  if type(file) ~= "string" or file == "" then
    return nil
  end
  if path.is_absolute(file) then
    return path.normalize(file)
  end
  if type(cwd) ~= "string" or cwd == "" then
    return nil
  end
  return path.normalize(path.join(cwd, file))
end
[exit 0]

$ sed -n 145,147p packs/core/scripts/lib/handoff_report.lua; sed -n 374,376p packs/core/scripts/lib/handoff_report.lua
local function qualifying(candidate, cwd)
  local resolved = resolve_against(candidate, cwd)
  if resolved and M.under_handoff_root(resolved, nil) then
  local cwd = type(event.cwd) == "string" and event.cwd or nil
  if event.event == "tool.after" then
    return qualifying(event.path, cwd)
[exit 0]

$ sed -n 16,18p packs/core/scripts/comm_check_hook.lua
return function(event)
  local file = report.report_path(event)
  if not file then
[exit 0]

$ grep -n 'path' book/src/ch06-02-hooks.md | sed -n 1,5p
40:- Each segment of the `run` path (after the `{{root <pack>}}` form, with `.lua` removed from the
58:- The core builds it with project-relative paths. A pack hook's grants map one to one onto the
122:| `tool` (`read`, `edit` or `write`), `path` | `tool.before`, `tool.after` |
[exit 0]

$ grep -n 'e.path\|, e.path\|path = trim' extensions/claude/lib/translate.lua extensions/opencode/lib/translate.lua
extensions/claude/lib/translate.lua:53:      e.path = str(p.tool_input.file_path)
extensions/opencode/lib/translate.lua:46:      files[#files + 1] = { tool = "write", path = trim(add) }
extensions/opencode/lib/translate.lua:48:      files[#files + 1] = { tool = "edit", path = trim(update) }
extensions/opencode/lib/translate.lua:50:      files[#files].path = trim(move)
extensions/opencode/lib/translate.lua:80:        one.tool, one.path = file.tool, file.path
extensions/opencode/lib/translate.lua:93:  e.tool, e.path = tool, str(input.path)
[exit 0]

~~~~

`airsstack.path.absolute` semantics, probed with the same runtime the hooks run under (airsl
0.1.5), from cwd `$P/abs` (an empty dir, removed afterwards):

~~~~text
$ airsl --version
airsl 0.1.5
[exit 0]

$ cat /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/scripts/abs.lua
-- Probe: how airsstack.path.absolute / is_absolute treat a relative path (airsl 0.1.5).
local path = airsstack.path
print("is_absolute(src/a.rs) = " .. tostring(path.is_absolute("src/a.rs")))
print("is_absolute(/x/src/a.rs) = " .. tostring(path.is_absolute("/x/src/a.rs")))
print("absolute(src/a.rs) = " .. tostring(path.absolute("src/a.rs")))
print("absolute(.) = " .. tostring(path.absolute(".")))
[exit 0]

$ cd /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/abs && airsl run /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/scripts/abs.lua
is_absolute(src/a.rs) = false
is_absolute(/x/src/a.rs) = true
absolute(src/a.rs) = /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/abs/src/a.rs
absolute(.) = /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/abs
[exit 0]

~~~~

Answer: two hooks read `event.path`, and both accept either form, but resolve a relative one
differently:
- `core/hooks/enforce.lua` (`tool.before`): passes `event.path` to `lib/enforce.lua`, which calls
  `path.absolute(file_path)` (`lib/enforce.lua:108,135,225`). The probe shows `path.absolute`
  resolves a relative path against the hook process's own working directory, not `event.cwd`. So a
  relative `event.path` is only right when the hook process runs in the directory the path is
  relative to; an absolute path is always right. (`event.cwd` is used only to find the git toplevel
  and as the marker-search fallback.)
- `core/scripts/lib/handoff_report.lua` (`tool.after`, via `handoff_check_hook.lua` and
  `comm_check_hook.lua`): `resolve_against(event.path, event.cwd)` keeps an absolute path, joins a
  relative one onto `event.cwd`, and returns nil (no check) for a relative path with no `cwd`
  (`handoff_report.lua:77-88,145-147,374-376`).
- Both translators today pass the tool's own path through unchanged: Claude
  `e.path = str(p.tool_input.file_path)` (`extensions/claude/lib/translate.lua:53`), OpenCode
  `e.path = str(input.path)` / patch paths (`extensions/opencode/lib/translate.lua:46-50,93`). The
  book's event table (`book/src/ch06-02-hooks.md:122`) does not say whether `path` is absolute.
  The safe contract for a new translator is an absolute `event.path` (or relative to `event.cwd`
  with the hook process started in `event.cwd`).
