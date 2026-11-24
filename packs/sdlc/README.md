# sdlc

The `sdlc` pack: an AI-native software development lifecycle for coding agents. A change goes from
intent to design to plan to execute, each step leaves its result in a committed Markdown file, and
a status board reads those files and tells you which step is next. It follows Anthropic's
[AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook) at the scale of one
author. Roadmaps, milestones and architecture decision records are optional layers above the chains.

## Install

```text
agentstacks init --tool claude --pack core --pack sdlc
```

Every skill is prefixed with the pack name, so `design` runs as `/sdlc-design`. The pack works
without `airsl`, but its status board and session-start summary come from it. Some steps use the
agents from the `core` pack, so install that too.

## Documentation

The artifact layout, states and frontmatter keys are in
[`references/artifact-chain.md`](references/artifact-chain.md); the file shapes are in
[`references/templates.md`](references/templates.md).

## Attribution

Stage model, artifact shapes and feedback loops follow Anthropic's
[AI-native SDLC playbook](https://claude.com/blog/the-ai-native-sdlc-playbook). The
execution discipline descends from `claudestacks-sdd`, adapted from
[superpowers](https://github.com/obra/superpowers).

## License

Apache-2.0. See [LICENSE](../../LICENSE).
