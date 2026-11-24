# core

The `core` pack. It adds three agents (`coder`, `reviewer`, `explorer`) and an
`orchestrate` skill that runs them in order, so a task is coded, checked and reviewed before it
reaches you. It also carries the pack's hooks: reply rules, project snapshots and the dispatcher
that points the agent at the right guideline.

## Install

Its hooks run on `airsl` and exit silently when it is missing, so install that first:

```sh
curl -LsSf https://airsstack.github.io/airsl/install.sh | sh
```

Then, in your project:

```text
agentstacks init --tool claude --pack core
```

Every skill is prefixed with the pack name, so `discuss` runs as `/core-discuss`.

## Attribution

The reply rules in the `discuss` skill's communication protocol replace an earlier terseness
mode that was inspired by the [caveman](https://github.com/juliusbrussee/caveman) plugin.
That earlier mode kept its state in `~/.airsstack/cc/concise.json`. Nothing reads that file any
more; it can be deleted.

## License

Apache-2.0. See [LICENSE](https://github.com/airsstack/agentstacks/blob/main/LICENSE).
