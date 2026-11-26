# journal

The `journal` pack: a memory for coding agents that you can read. Notes are plain Markdown files in
one Obsidian-compatible vault on your machine (`~/.airsstack/journal` by default). The agent writes
them, you read and edit them, and the agent recalls them later instead of working the same thing
out again.

## Install

Every hook and script in the pack runs on `airsl`, so install that first:

```sh
curl -LsSf https://airsstack.github.io/airsl/install.sh | sh
```

The pack requires `core`, and its review command uses the `context-handoff` skill from that pack.
Install both:

```text
agentstacks init --tool claude --pack core --pack journal
```

On OpenCode, pass `--tool opencode` (or several tools). `capture` and `note` read the session through
`opencode session export`, which needs `opencode` on `PATH` and OpenCode's background service
running; otherwise they write without the transcript and say why.

On Muse Code, pass `--tool musecode`. The pack ships no transcript lookup for it, so `capture` and
`note` write without the transcript and key the note to the first eight characters of the session id.

## Documentation

The note format is in [`references/note-schema.md`](https://github.com/airsstack/agentstacks/blob/main/packs/journal/references/note-schema.md). Each command that
captures, recalls or reviews notes is a skill under [`skills/`](https://github.com/airsstack/agentstacks/tree/main/packs/journal/skills/), rendered as
`/journal-<name>`.
