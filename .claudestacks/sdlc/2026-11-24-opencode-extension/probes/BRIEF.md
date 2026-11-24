# Probe brief (shared by every probe agent of this chain)

Chain: `.claudestacks/sdlc/2026-11-24-opencode-extension/` (read `intent.md` there first).
Goal: prove facts about **OpenCode v2.4.1** that the design of the official agentstacks `opencode`
extension depends on. A probe answers one question about what already exists. You never build the
feature.

## Paths

- Chain dir: `/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/foundation-02-opencode/.claudestacks/sdlc/2026-11-24-opencode-extension`
- Scratchpad (all probe projects live here, one dir per probe, `<topic>-NN/`):
  `/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad`
- OpenCode binary: `/Users/hiraq/.opencode/bin/opencode` (v2.4.1, not on PATH).
- OpenCode source at tag v2.4.1: `<scratchpad>/oc-src/anomalyco-opencode-6f05140/`
  (`packages/core/src/plugin/`, `packages/plugin/src/`, `packages/core/src/tool/`, …).
- V2 docs at that tag, copied: `<chain>/probes/docs/*.mdx` and `<chain>/probes/docs/build-plugins/*.mdx`.

## Harness (a local mock model; nothing leaves the machine)

`sh <chain>/probes/harness/run.sh <probe-dir> <port> "<prompt>" [extra 'opencode run' flags]`

- `<probe-dir>/proj/` is the project (create files there first); `<probe-dir>/scenario.json` is a
  JSON array of steps the mock model plays, one per model request that offers tools:
  `{"tool":"<name>","args":{...}}` (one tool call) or `{"text":"..."}` (final answer). Requests
  with no tools (title generation) get "title". When steps run out it answers "done".
- Outputs: `<probe-dir>/out.txt` (`opencode run --format json` events, then `exit=N`),
  `<probe-dir>/err.txt`, `<probe-dir>/requests.jsonl` (every request body OpenCode sent the model:
  system prompt, tool list with schemas, messages incl. tool results — read it with `node -e`).
- It isolates OpenCode: HOME and XDG dirs under `<probe-dir>/home`, `--standalone`, `--auto`,
  `OPENCODE_DISABLE_MODELS_FETCH=1`. Global config at `<probe-dir>/home/.config/opencode/opencode.json`
  sets the mock provider; put project config in `proj/`.
- Built-in tool names seen (smoke probe): `edit,generate_image,generate_video,glob,grep,question,read,shell,skill,subagent,webfetch,websearch,write,execute`.
  `read` takes `path` (not `filePath`). Read each tool's JSON schema from `requests.jsonl`.
- Other CLI commands (`debug agents`, `debug config`, `session …`) need the same isolation: export
  `HOME=<probe-dir>/home XDG_CONFIG_HOME=$HOME/.config XDG_DATA_HOME=$HOME/.local/share XDG_CACHE_HOME=$HOME/.cache XDG_STATE_HOME=$HOME/.local/state OPENCODE_DISABLE_MODELS_FETCH=1`
  and `cd <probe-dir>/proj` first.
- Use only the port range you were given; one port per concurrent run.

## Hard rules

- Never run OpenCode without the isolation above. Never touch the user's real `~/.config/opencode`,
  `~/.local/share/opencode` or `~/.opencode`. Never `opencode service start`, `plugin add`, `upgrade`.
- Write only under the scratchpad and the one results file you were given. Never edit `intent.md`,
  any spec, other agents' results files, or anything else in the repo. Never commit.
- Run each command once. No retry loops. If something fails, record the failure as the result.
- Proof only from real output, a `file:line` in the v2.4.1 source or docs, or a search plus a
  control (for "X does not exist"). Never from memory or the web.

## Results file format (`<chain>/probes/<topic>-results.md`)

For each probe, a `## Pn: <question>` section with:

1. **Setup**: every file you created, verbatim, in fenced blocks (path as the info line).
2. **Command**: exact command(s).
3. **Output**: raw output pasted byte for byte (never retyped). You may cut long output, marked
   `[… N lines cut]`. For `requests.jsonl` extracts, paste the `node -e` command and its raw output.
4. **Answer**: one or two sentences, only what the output shows.

Source citations: copy each cited source file into `<chain>/probes/src/<same relative path>` and
cite `probes/src/<path>:<line>`. Do the same for docs: `probes/docs/<file>.mdx:<line>`.

Final reply to the orchestrator: a compact table `question | answer | evidence (results-file
section)`, plus anything surprising. At most 500 words.
