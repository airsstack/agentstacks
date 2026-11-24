# A First Project

This chapter sets up Claude Code in a project with three official packs: `core`, `sdlc` and
`guideline-rust`. (`sdlc` requires `core`, so the two always come together.) The counts and
files shown are those of `packs-v0.1.0`.

## Init

In the project directory:

```sh
agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust
```

`init` writes `agentstacks.toml`, prints the plan, one line per file, and asks before applying. The
plan ends:

```text
76 to write, 0 to delete, 0 unchanged, 1 skipped
```

Answering yes applies at once, as `--yes` does, and prints:

```text
applied: 76 written, 0 deleted
```

With no terminal and no `--yes`, `init` still writes `agentstacks.toml` but applies nothing. It
ends with:

```text
76 to write, 0 to delete, 0 unchanged, 1 skipped
stdin is not a terminal; pass --yes to apply
agentstacks.toml written; nothing applied (run apply)
```

Then `agentstacks apply --yes` applies it and ends with `applied: 76 written, 0 deleted`.

## The project file

The `agentstacks.toml` that `init` wrote is the committed record of what the project uses:

```toml
api = 0

[tools.claude]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "extensions/claude"

[packs.core]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "packs/core"

[packs.guideline-rust]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "packs/guideline-rust"

[packs.sdlc]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
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

`agentstacks update` looks for a newer `packs-v*` tag. While `packs-v0.1.0` is the newest, it
ends with:

```text
nothing to update
```

## Commit

Commit `agentstacks.toml`, `agentstacks.lock` and `.claude/`. A teammate runs `agentstacks apply`
and gets the same setup.

Next: [The Command Line](ch04-00-cli.md) describes the prompt, `--yes` and the exit codes, and
[Plan, Apply and the Lock](ch05-00-plan-and-apply.md) describes what each step does.
