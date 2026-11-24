# agentstacks-cli

The `agentstacks` command-line tool. It sets up agentic coding tools (Claude Code first, OpenCode
second) from one tool-neutral copy of skills, agents, commands, hooks and rules, and shows every
change before it writes anything.

## Install

```bash
curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh   # prebuilt binary, also installs airsl when missing
cargo install agentstacks-cli                                        # or build from source
```

The installed binary is named `agentstacks`.

Note: this route does not install airsl, which `plan` needs (≥0.1.5).
Install airsl yourself before running `plan`.

## Use

Run these from your project:

```bash
agentstacks init --tool claude --pack core   # write agentstacks.toml, then plan and apply
agentstacks plan                             # show what apply would do, write nothing
agentstacks apply                            # show the plan, ask, then write the files and the lock
agentstacks doctor                           # report drift from agentstacks.lock
agentstacks update                           # move to the newest pack tags, then plan and apply
```

## Documentation

- [Repository README](https://github.com/airsstack/agentstacks#readme)
- [The agentstacks book](https://airsstack.github.io/agentstacks/)
- [Moving from the claudestacks Plugins](https://airsstack.github.io/agentstacks/ch03-00-migration.html)

## License

Apache-2.0.
