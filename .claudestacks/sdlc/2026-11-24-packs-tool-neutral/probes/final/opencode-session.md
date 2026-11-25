# Session key and transcript

The journal's capture and note skills follow this file with the session id and
the project directory they resolved.

- `id8`: the last eight characters of the session id, in lower case. Its first
  characters come from the time and repeat across sessions.
- `tpath` and `cleanup`: run `opencode service status`.
  - Exit 127 means `opencode` is not on `PATH`. Tell the user `opencode is not
    on PATH, so the journal was written without the transcript`, and pass an
    empty `tpath` and an empty `cleanup`.
  - If it prints `stopped`, this session does not run on OpenCode's background
    service, and an export would start the service, which marks this session
    as restarted. Tell the user `this OpenCode session does not run on the
    background service, so the journal was written without the transcript`,
    and pass an empty `tpath` and an empty `cleanup`.
  - Otherwise run `mktemp "{{var tmpdir}}/journal-XXXXXX"` and take its output
    as `<export>`, then run

    ```sh
    opencode session export "<session_id>" > "<export>"
    ```

    On exit 0, `tpath` is `<export>` and `cleanup` is `rm -f "<export>"`. On a
    non-zero exit, run `rm -f "<export>"` yourself, then pass an empty `tpath`
    and an empty `cleanup`.
