---
name: capture
description: >
  Manually capture the current session as a grounded storytelling note in the
  journal vault. Resolves the session id, transcript path, and
  project floor, then spawns the isolated {{agent capture}} subagent to write
  sessions/session-<id8>.md by full overwrite. Re-run anytime to refresh the
  story. Use when the user says "capture this session" / "{{skill capture}}".
---

# journal-capture

Capture the current session as a session story. You do the cheap deterministic
resolution here on the main thread, then hand the heavy transcript read and
storytelling to the isolated `{{agent capture}}` subagent so it never costs the
main-thread context.

## Steps

1. Resolve the session id and stem. The session id is `{{var session_id}}`.
   Compute `id8` = its first eight characters and `stem` = `session-<id8>`.
   If `{{var session_id}}` is empty, fall back to `stem` =
   `session-$(date +%Y%m%d-%H%M%S)` and note that the story cannot be keyed to
   the session id.

2. Resolve the transcript path:

   ```sh
   HOME_ROOT="${AIRSSTACK_HOME:-$HOME/.airsstack}"
airsl run --policy confined \
  --allow-env CLAUDE_CONFIG_DIR --allow-env HOME --allow-env PWD \
  --allow-read . --allow-read "$HOME/.claude" \
  "{{var project_dir}}/{{root}}/scripts/transcript-path.lua" "{{var session_id}}"
   ```

   Capture stdout as `tpath`. An empty result (non-zero exit) means no
   transcript was found — pass an empty `tpath` and let the agent degrade.

3. Resolve the project floor:

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

4. Provision the vault (idempotent, cheap):

   ```sh
   HOME_ROOT="${AIRSSTACK_HOME:-$HOME/.airsstack}"
airsl run --policy confined \
  --allow-env AIRSSTACK_HOME --allow-env HOME --allow-write "$HOME_ROOT" \
  "{{var project_dir}}/{{root}}/scripts/provision.lua"
   ```

5. Spawn the `{{agent capture}}` subagent (Task / Agent tool,
   `subagent_type: {{agent capture}}`), passing `session_id`, `id8`, `stem`,
   `tpath`, and `project`. The agent reads the transcript, overwrites
   `sessions/<stem>.md`, links the daily note, refreshes the index, and
   returns a one-line receipt.

6. Relay the agent's receipt to the user. Do not write any note file yourself.

## If the agent is unavailable

If the `{{agent capture}}` subagent cannot be spawned (the pack's agents are
not installed), tell the user and stop. Do nothing destructive.
