# agentstacks docs

Working documentation for contributors. It lives here until the mdBook under `book/` exists, and
then moves there.

| Document | Covers | State |
|---|---|---|
| [architecture.md](architecture.md) | layers, dependency rule, where logic lives, how it is tested | built |
| [cli.md](cli.md) | the five commands, `-C`, the plan text, the prompt and `--yes`, streams and exit codes | built |
| [extensions.md](extensions.md) | how an extension is installed, loaded and run; the `api = 0` Lua surface | built |
| [hooks.md](hooks.md) | the neutral hook protocol, the shared runtime, `may_block` | built |
| [plan-and-apply.md](plan-and-apply.md) | operations, plan steps, apply order, the lock, `doctor`, sources, `update` | built |

A document describes what the code does, or, while marked *planned*, what it is agreed to do. When
the two disagree, the code is wrong or the document is out of date. Fix one of them in the same
change.
