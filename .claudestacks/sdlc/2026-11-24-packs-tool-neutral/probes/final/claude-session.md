# Session key and transcript

The journal's capture and note skills follow this file with the session id and
the project directory they resolved.

- `id8`: the first eight characters of the session id.
- `tpath`: run

  ```sh
  airsl run --policy confined \
    --allow-env CLAUDE_CONFIG_DIR --allow-env HOME --allow-env PWD \
    --allow-read . --allow-read "$HOME/.claude" \
    "<project_dir>/{{root}}/scripts/transcript-path.lua" "<session_id>"
  ```

  and take its stdout as `tpath`. An empty result or a non-zero exit means no
  transcript was found: pass an empty `tpath`.
- `cleanup`: empty.
