# A First Project

This chapter sets up Claude Code in a project with three official packs: `core`, `sdlc` and
`guideline-rust`. (`sdlc` requires `core`, so the two always come together.) The counts and
files shown are those of the streams' `1.x` releases (for example `packs-core-v1.0.1`).

## Init

In the project directory:

```sh
agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust
```

`--tool opencode` sets up OpenCode the same way, `--tool musecode` sets up Muse Code, and the flags
may be given together. To use an extension from a checkout of this repository instead of a release,
copy it into your project, for example `extensions/opencode` to `ext/opencode`, and pass
`--tool ext/opencode` (a tool path must stay inside the project).

`init` writes `agentstacks.toml`, prints the plan as counts per owner (the full one-line-per-file
list is `agentstacks plan --verbose`), and asks before applying. The plan ends:

```text
76 to write, 0 to delete, 0 unchanged, 1 skipped
```

Answering yes applies at once, as `--yes` does, and prints:

```text
applied: 76 written, 0 deleted
note: start a new session — hooks, skills and agents load at session start
```

Hooks, skills and agents load when a session starts, so a setup applied into an already
running session takes effect in the next one. Preview an `init` without writing anything
with `agentstacks init --tool … --pack … --dry-run`, which prints the would-be
`agentstacks.toml` and its plan.

With no terminal and no `--yes`, `init` still writes `agentstacks.toml` but applies nothing. It
ends with:

```text
76 to write, 0 to delete, 0 unchanged, 1 skipped
stdin is not a terminal; pass --yes to apply
agentstacks.toml written; nothing applied (run apply)
```

Then `agentstacks apply --yes` applies it and ends with `applied: 76 written, 0 deleted`
and the new-session note above.

## The project file

The `agentstacks.toml` that `init` wrote is the committed record of what the project uses:

```toml
api = 1

[tools.claude]
git = "https://github.com/airsstack/agentstacks"
tag = "ext-claude-v1.1.0"
subdir = "extensions/claude"

[packs.core]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-core-v1.0.1"
subdir = "packs/core"

[packs.guideline-rust]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-guideline-rust-v1.0.0"
subdir = "packs/guideline-rust"

[packs.sdlc]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-sdlc-v1.1.0"
subdir = "packs/sdlc"
```

## What was skipped

The one skipped item is shown in the plan:

```text
  skip     core/hook/3: claude shows no hook output to the model at session end
```

An item a tool cannot express is shown as a skip with its reason, never dropped.

## Plan, doctor, update

`agentstacks plan` shows what an apply would change now. Right after the apply, nothing:

```text
0 to write, 0 to delete, 76 unchanged, 1 skipped
```

`agentstacks doctor` checks the project against `agentstacks.lock` and prints:

```text
healthy
```

`agentstacks update` looks for a newer tag in each artifact's own stream. While every pinned
stream is current, it ends with:

```text
nothing to update
```

## Commit

Commit `agentstacks.toml`, `agentstacks.lock` and `.claude/`. A teammate runs `agentstacks apply`
and gets the same setup.

Next: [The Command Line](ch04-00-cli.md) describes the prompt, `--yes` and the exit codes, and
[Plan, Apply and the Lock](ch05-00-plan-and-apply.md) describes what each step does.
