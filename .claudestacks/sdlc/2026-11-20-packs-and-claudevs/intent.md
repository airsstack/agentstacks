---
status: approved
created: 2026-11-20
milestone: foundation/01
---

# Intent: Claudestacks plugins to packs, and the claudevs move

Merged on 2026-11-23 from two intents, `plugins-to-packs` and `claudevs-move`, so both halves are
designed, planned and built together as one chain. claudevs is the check that shows the packs
behave as the plugins do, so the two scopes depend on each other.

## Problem

The milestone needs the claudestacks plugins to live in `packs/` and render, through the `claude`
extension, into config that behaves like the plugins do today. Five plugins are carried over: core
(`claudestacks`), sdlc, journal, guideline-rust and guideline-python. The sixth, `claudestacks-cmux`,
is no longer needed and is not converted (decided 2026-11-23). Today they exist only as Claude Code
marketplace plugins in `../claudestacks/plugins/`. They carry namespaced
cross-references, `${CLAUDE_PLUGIN_ROOT}` paths, Lua scripts, references, enforcement data and
`sh` hook wrappers, all tied to the plugin model. Their hooks read Claude Code's raw payload and
write Claude Code's output directly. The enforce hook finds guideline data through the
installed-plugins registry. A project's `.claude/` has no plugin namespace, so a skill invoked
today as `/claudestacks-sdlc:status` loses its namespace once rendered. Four skill names match
Claude Code built-in commands or bundled skills (`status`, `plan`, `design` in sdlc; `statusline`
in core), and a project skill replaces a built-in of the same name (Claude Code skills docs,
"Resolve skills that share a name"). `statusline` configures a Claude-only settings key.

`claudevs` and `claudevs-cli`, the deterministic Claude Code setup test engine, still live in
claudestacks (`../claudestacks/crates/claudevs{,-cli}`). The milestone requires them to move here
under the same crate names. Today they check only a Claude Code plugin directory
(`.claude-plugin/plugin.json`, `hooks/hooks.json`, `${CLAUDE_PLUGIN_ROOT}`), and the
`claudevs-check` task runs over a fixture plugin corpus. Seven of their source files break the
workspace's module naming rule (`case_name.rs`, `hook_event.rs`, `plugin_name.rs`,
`plugin_version.rs`, `marketplace_name.rs`, `t_module.rs`, `hooks_file.rs`). They are built against
airsl 0.1.3 while this workspace pins 0.1.4.

Nothing shows yet that a rendered pack behaves as its plugin does. claudevs reads only plugin
directories, and the real plugins carry no claudevs cases.

## Affected systems

- New `packs/{core,sdlc,journal,guideline-rust,guideline-python}/`.
- Source: `../claudestacks/plugins/claudestacks{,-sdlc,-journal,-guideline-rust,-guideline-python}`.
- The plugins' Lua tests and the CI that runs them (`cargo make lua` today covers `extensions/` only).
- New `crates/claudevs`, `crates/claudevs-cli`. Their dependencies are `airsl`, regex, serde,
  serde_json, serde-saphyr, thiserror, toml, walkdir and tempfile, plus clap for the CLI.
- `Makefile.toml` (a `claudevs-check` equivalent), CI (a corpus job), `deny.toml`.
- The claudevs fixture corpus.
- `extensions/claude/`: the names it gives skills and agents (`lib/values.lua`, the frontmatter
  `name` it writes) and the agent names its hook matchers use.
- `types/shorthand` (`OFFICIAL_PACKS` lists `cmux` today), `docs/cli.md`, `README.md` and
  `CLAUDE.md`: the official pack list must match the five pack directories.

## Desired outcome

- The five plugins exist as packs in `packs/`, written in the pack format with no Claude-only
  constructs outside `native/claude/`.
- Rendered through the `claude` extension into a project, each pack behaves as its plugin does
  today. The ported Lua tests and an end-to-end render of the five packs show this.
- The plugins' Lua tests run in agentstacks CI under airsl.
- The plugin model's couplings are gone: hook scripts speak the neutral hook protocol, each pack's
  hooks name only its own agents, the enforce hook no longer reads the installed-plugins registry,
  options reach hooks through declared arguments, and the example references to guideline skills
  become prose. Each resulting behaviour change is called out.
- `claudevs` and `claudevs-cli` build, test and pass `cargo make dod` in this workspace under the
  same crate names. They are imported as new files; claudestacks' commit history is not carried
  over or referred to. Their behaviour is unchanged.
- The adjustments the move needs are made: airsl 0.1.4, the workspace's dependencies and features,
  and the seven file names brought in line with the module naming rule.
- CI runs the claudevs fixture corpus check.

## Constraints

- Depends on `pack-and-project-formats`, `core-plan-and-apply` and `cli-and-claude-extension`
  (all done).
- Plugin behaviour is the reference; any change in behaviour is called out, not slipped in.
- Packs are installed natively into the project (`.claude/`), never as a plugin or machine-wide.
- Rendered packs never replace a Claude Code built-in command, bundled skill or built-in agent,
  and do not collide with the user's own project skills and agents. The tool's name for a skill
  or agent comes from the extension, which namespaces it by pack (for claude, a pack prefix such
  as `/sdlc-status`; decided 2026-11-23). Pack content stays tool-neutral: skill and agent names
  in a pack carry no pack prefix (`journal/capture`, not `journal/journal-capture`).
- Every pack starts at version `0.1.0`.
- Keep the claudevs crate names (crates.io continuity); `airsl` from crates.io only.
- claudestacks is not edited or archived by this chain, and keeps working until
  `release-and-migration` archives it.

## Non-goals

- Converting `claudestacks-cmux`: it is not an official pack.
- New skills or agents beyond what the plugins ship.
- New claudevs features, including checking agentstacks-rendered `.claude/` project config
  (decided 2026-11-23; a later chain if real work needs it).
- The claudevs release and smoke workflows (`release-and-migration`).
- OpenCode rendering (`foundation/02`).
- Moving `airsclaude` (airsdk's work, roadmap non-goal).
- Publishing crates or a release (`release-and-migration`).
