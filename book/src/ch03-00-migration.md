# Moving from the claudestacks Plugins

This chapter is for you if you installed plugins from the claudestacks marketplace in Claude Code.
The plugins became packs: the same skills, agents and hooks, installed into each project by
agentstacks instead of machine-wide by the plugin system.

## What maps to what

| claudestacks plugin | agentstacks pack |
|---|---|
| `claudestacks` | `core` |
| `claudestacks-sdlc` | `sdlc` |
| `claudestacks-journal` | `journal` |
| `claudestacks-guideline-rust` | `guideline-rust` |
| `claudestacks-guideline-python` | `guideline-python` |
| `claudestacks-cmux` | not converted: keep the plugin if you use it |

## Install agentstacks

```sh
curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh
```

It installs airsl when that is missing. See [Installation](ch01-01-installation.md) for the other
routes.

## Remove the plugins

Remove the plugins first, so their skills and hooks do not run twice beside the packs. For each
plugin:

```sh
claude plugin uninstall <plugin>@claudestacks --scope <scope>
```

Use the scope you installed it at (the default is user). Or remove the whole marketplace:

```sh
claude plugin marketplace remove claudestacks
```

That uninstalls every plugin installed from it, `claudestacks-cmux` included.

## Set up each project

In each project, pick the packs you used. `sdlc` and `journal` require `core`.

```sh
agentstacks init --tool claude --pack core --pack sdlc --pack journal --pack guideline-rust
```

Review the plan, answer the apply prompt, then commit `agentstacks.toml`, `agentstacks.lock` and
`.claude/`. [A First Project](ch01-02-first-project.md) walks through each step.

Packs install per project, never machine-wide.

## What changed

Commands and agents have new names, because a native install has no plugin namespace and the
extension names things by pack.

| Old (plugin) | New (pack) |
|---|---|
| `/claudestacks:x` | `/core-x` |
| `/claudestacks:statusline` | `/core-statusline` |
| `/claudestacks-sdlc:x` | `/sdlc-x` |
| `/claudestacks-journal:journal-x` | `/journal-x` |
| `/claudestacks-journal:journal-capture` | `/journal-capture` |
| `/claudestacks-journal:journal-helped` | `/journal-helped` |
| `/claudestacks-journal:journal-link` | `/journal-link` |
| `/claudestacks-journal:journal-note` | `/journal-note` |
| `/claudestacks-journal:journal-recall` | `/journal-recall` |
| `/claudestacks-journal:journal-review` | `/journal-review` |
| `/claudestacks-journal:journal-setup` | `/journal-setup` |
| `/claudestacks-guideline-rust:rust-guidelines` | `/guideline-rust-guidelines` |
| `/claudestacks-guideline-python:python-guidelines` | `/guideline-python-guidelines` |
| `claudestacks:coder` | `core-coder` |
| `claudestacks-sdlc:chain-reader` | `sdlc-chain-reader` |
| `claudestacks-journal:journal-capture` | `journal-capture` |
| `claudestacks-journal:journal-curator` | `journal-curator` |
| `claudestacks-journal:journal-note` | `journal-note` |
| `claudestacks-journal:journal-recall` | `journal-recall` |

The other changes:

- airsl must be on `PATH` and at least 0.1.5. `plan` and `doctor` report a missing or old airsl.
  The hook wrappers' fallback lookup (`$AIRSL_BIN`, `~/.local/bin`, `~/.cargo/bin`) is gone.
- `style_reinject` is set in `agentstacks.toml`, under `[packs.core.options]`, not in the plugin's
  user configuration.
- The statusline skill is `/core-statusline`. It always detects its source as inside the project
  and recommends project scope.
- The enforce hook finds guideline packs installed in the same project, not plugins in the
  registry.
- `sdlc` requires `core`. The plugin declared no dependency, but its skills name core's agents and
  its agent-stop hook runs core's check.
- `journal` requires `core` too.
- The enforce pointer names `/guideline-rust-guidelines`, where it used to say
  `claudestacks-guideline-rust:rust-guidelines`.
- The enforce doctor has no gate 1 and no parity section. Its "no manifests" stop reads
  `no pack ships a usable enforcement.json`.
- The SessionEnd reminder appears in `plan` as a skip. Its output was discarded before too.
- A `SubagentHandback` violation is now a deny with a reason, where it used to be exit 2 with the
  text on stderr. Both block the call and return the reason to the subagent, which retries. Only the
  error text differs: it no longer shows the hook command in brackets.

## What stays

- `.claudestacks/sdlc/` chain files are read and written at the same paths.
- The journal vault stays at `$AIRSSTACK_HOME/journal` (default `~/.airsstack/journal`).
- The snapshot store stays at `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/`.

## claudevs users

claudevs is released from this repository now. Install the prebuilt binary:

```sh
curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh
```

`cargo install claudevs-cli` is unchanged. The old claudestacks installer URL will forward
here. See the claudevs [Installation](ch08-01-installation.md).
