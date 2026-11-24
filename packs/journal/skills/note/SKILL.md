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

2. Resolve the session id: `session_id` = `{{var session_id}}`; `id8` = its
   first eight characters. (If empty, pass an empty `id8`; the note simply
   carries no `session:` join value.)

3. Resolve the transcript path:

   ```sh
   HOME_ROOT="${AIRSSTACK_HOME:-$HOME/.airsstack}"
airsl run --policy confined \
  --allow-env CLAUDE_CONFIG_DIR --allow-env HOME --allow-env PWD \
  --allow-read . --allow-read "$HOME/.claude" \
  "{{var project_dir}}/{{root}}/scripts/transcript-path.lua" "{{var session_id}}"
   ```

   Capture stdout as `tpath` (empty/non-zero ⇒ the agent degrades).

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

6. Spawn the `{{agent note}}` subagent (Task / Agent tool,
   `subagent_type: {{agent note}}`), passing `topic`, `session_id`, `id8`,
   `tpath`, and `project`. The agent derives the kebab stem (rejecting
   `_unresolved`), creates or updates `notes/<stem>.md`, links the daily note,
   refreshes the index, and returns a one-line receipt.

7. Relay the agent's receipt to the user. Do not write any note file yourself.

## If the agent is unavailable

If the `{{agent note}}` subagent cannot be spawned, tell the user and stop. Do
nothing destructive.
