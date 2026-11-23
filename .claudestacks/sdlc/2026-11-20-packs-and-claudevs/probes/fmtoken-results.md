# Tokens in frontmatter values

Run 2026-11-23 in /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/4e861342-4f1c-401f-96af-1faf5ebe44a5/scratchpad/fmtoken. Binary: agentstacks built from this worktree (HEAD 9b7c846 code = 80fd75f), shipped claude extension. Output pasted verbatim.

~~~~text
--- packs/j/pack.toml
[pack]
name = "j"
version = "0.1.0"
api = 0
description = "j"
--- packs/j/skills/recall/SKILL.md
---
name: recall
description: Recall notes. Use when the user says "{{skill recall}} X".
---
Spawn {{agent curator}}.
--- packs/j/skills/helped/SKILL.md
---
name: helped
description: Use AFTER a note returned by {{skill recall}} helped.
---
body
--- packs/j/agents/curator.md
---
name: curator
description: Spawned by {{skill recall}}.
---
body
$ agentstacks init --tool ./ext-claude --pack ./packs/j --yes
agentstacks plan

  create   .claude/agents/curator.md       j
  create   .claude/skills/helped/SKILL.md  j
  create   .claude/skills/recall/SKILL.md  j

3 to write, 0 to delete, 0 unchanged, 0 skipped
applied: 3 written, 0 deleted
exit=0
--- .claude/skills/recall/SKILL.md
---
name: "recall"
description: "Recall notes. Use when the user says \"/recall X\"."
---
Spawn curator.
--- .claude/skills/helped/SKILL.md
---
name: "helped"
description: "Use AFTER a note returned by /recall helped."
---
body
--- .claude/agents/curator.md
---
name: "curator"
description: "Spawned by /recall."
---
body
~~~~
