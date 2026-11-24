---
name: enforce-doctor
description: Explain why the enforcement hook does or does not fire for a given file path — runs the core pack's doctor script and reads the trace back. Read-only diagnostic. Use when a guideline pointer failed to appear for a file, when one appeared unexpectedly, or when the user says "{{skill enforce-doctor}} <path>".
---

Diagnose the rule-enforcement hook for the path `{{var arguments}}`.

Run exactly this, from the current working directory:

```
airsl run --policy confined \
  --allow-env HOME --allow-env TMPDIR --allow-env AIRSSTACK_HOME \
  --allow-read / --allow-write "{{var tmpdir}}" \
  "{{var project_dir}}/{{root}}/hooks/doctor.lua" "{{var project_dir}}/{{root}}" "{{var arguments}}"
```

If `{{var arguments}}` is empty, ask which file path to diagnose and stop; do not guess one.

In a worktree-isolated session the guard refuses an `airsl` command carrying a variable it
cannot resolve — here `${TMPDIR:-/tmp}` comes back as "a value computed at runtime". Resolve it
first with plain `echo "$TMPDIR"` — plain commands may use variables — and rerun with that
absolute path in place of the variable.

Then read the trace back to the user, in this order:

1. **The outcome.** `outcome: 0 pointer(s)` means the hook would stay silent
   for this path; anything higher means it would inject that many guideline pointers.
2. **The stage that ended it.** The hook has several paths that all end in silence and are
   indistinguishable from outside. The trace names exactly one:
   - `no pack ships a usable enforcement.json`: no installed pack carries a readable
     `enforcement.json`. Add a guideline pack to `agentstacks.toml`, then run
     `agentstacks apply`.
   - `GATE 2` — no `detect` marker sits at or above the file's directory.
   - `GATE 3` — no `match` glob hit the file's repo-relative path.
   - `already claimed` — the pointer already fired once for this stack, phase,
     session and agent context. Expected, not a fault.

Do not paste the whole trace unless the user asks for it. Report the outcome, the
deciding stage, and — when the cause is fixable — the one action that fixes it.

One thing about the trace is easy to misread: **`sentinels held: ...` names stacks and phases,
not who claimed them.** A second run of the doctor on the same path in the same session reports
`already claimed` for a sentinel the doctor's own first run set, not one a real hook set — the
doctor runs under its own dedicated session/agent identity, so running it twice in a row on the
same path is expected to look "used up" the second time. That is the doctor observing itself, not
the hook.
