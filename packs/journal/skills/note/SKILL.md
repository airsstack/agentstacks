---
name: note
description: >
  Manually capture one atomic, standalone topic note (insight|decision) into
  the journal vault. Resolves session id, transcript path, and
  project floor, then spawns the isolated {{agent note}} subagent to write/update
  notes/<kebab>.md, stamping the session join field. Update-in-place on a
  repeated topic. Use when the user says "save a note on X" / "{{skill note}} X".
---

# journal-note

Capture one atomic note about a specific topic. Do the cheap deterministic
resolution here, then hand the transcript read and note authoring to the
isolated `{{agent note}}` subagent.

## Steps

1. Take the user's `topic` (the note subject, e.g. "auth rbac architecture").
   If the user gave no topic, ask for one before proceeding.

2. Resolve the session id and the project directory. The session id is
   `{{var session_id}}` and the project directory is `{{var project_dir}}`. If
   either still reads as a `${…}` shell variable, read its value with `echo`.

3. If the session id is empty, do not read `session.md`: pass an empty `id8`,
   an empty `tpath` and an empty `cleanup`; the note then carries no `session:`
   join value. Otherwise read
   `<project_dir>/{{skill_dir journal/capture}}/session.md` and follow it with
   that session id. It gives `id8`, the eight-character session key; `tpath`,
   the transcript file; and `cleanup`, a command that removes a temporary
   transcript (often empty). If `session.md` does not exist, this tool has no
   transcript lookup: take `id8` = the first eight characters of the session
   id, and pass an empty `tpath` and an empty `cleanup`.

4. Resolve the project floor:

   ```sh
   airsl run --policy confined \
  --allow-read . \
  "{{var project_dir}}/{{root}}/scripts/project-key.lua"
   ```

   Capture stdout as `project`.

   In a worktree-isolated session the guard refuses **any** `airsl` command in this skill
   that carries a variable it cannot resolve — `$TMPDIR`, `$PWD`
   and `$HOME_ROOT` each come back as "a value computed at runtime"; `$HOME` is accepted.
   That covers every block below, not only this one. Resolve each with a plain `echo`
   first — plain commands may use variables — and paste the absolute paths in.

5. Provision the vault (idempotent):

   ```sh
   HOME_ROOT="${AIRSSTACK_HOME:-$HOME/.airsstack}"
airsl run --policy confined \
  --allow-env AIRSSTACK_HOME --allow-env HOME --allow-write "$HOME_ROOT" \
  "{{var project_dir}}/{{root}}/scripts/provision.lua"
   ```

6. Spawn the `{{agent note}}` subagent, passing `topic`, `session_id`, `id8`,
   `tpath`, `cleanup`, and `project`. The agent derives the kebab stem
   (rejecting `_unresolved`), creates or updates `notes/<stem>.md`, links the
   daily note, refreshes the index, runs `cleanup`, and returns a one-line
   receipt.

7. When the agent's receipt arrives, relay it to the user. Do not write any
   note file yourself.

## If the agent is unavailable

If the `{{agent note}}` subagent cannot be spawned, tell the user, run
`cleanup` if it is set, and stop. Do nothing destructive.
