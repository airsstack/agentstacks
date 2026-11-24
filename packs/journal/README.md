# claudestacks-journal

A memory for Claude Code that you can read. Notes are plain Markdown files in one
Obsidian-compatible vault on your machine. Claude writes them, you read and edit them, and Claude
recalls them later instead of working the same thing out again.

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

## Documentation

The vault, the commands that capture, recall and review notes, and the session-start card are
described in
[the claudestacks-journal chapter](https://airsstack.github.io/claudestacks/ch14-00-journal-plugin.html)
of the book.
