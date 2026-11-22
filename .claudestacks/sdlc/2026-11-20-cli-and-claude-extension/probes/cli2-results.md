# cli2 probes: toml_edit retag, clap required Vec, `sh -c` quoting, airsl test

Probe crate: throwaway `rust-probes2` (edition 2024, toolchain from the worktree's
`rust-toolchain.toml`). Source: `cli2-main.rs` (this directory). Run once with
`cargo run -q -- <scratch>/rust-probes2/t3 > out.txt 2>&1` (cargo exit 0).

Toolchain: `rustc 1.94.1 (e408947bf 2026-03-25)`, `cargo 1.94.1 (29ea6fb6a 2026-03-24)`.

Versions from Cargo.lock:

| crate | version |
|---|---|
| toml_edit | 0.25.20+spec-1.1.0 |
| toml | 1.2.2+spec-1.1.0 |
| clap | 4.6.9 |
| clap_builder | 4.6.7 |
| clap_derive | 4.6.9 |
| airsl (binary on PATH) | airsl 0.1.4 |

## Raw output (T1, T2, T3), byte for byte

```text
--- T1 (a) inline table input ---
api = 0

[packs]
core = { git = "https://x", tag = "packs-v0.1.0", subdir = "packs/core" } # c
--- end input ---
as_value_mut() is_some = true
--- T1 (a) inline table output ---
api = 0

[packs]
core = { git = "https://x", tag = "packs-v0.2.0", subdir = "packs/core" } # c
--- end output ---
toml reparse packs.core.tag = Some(String("packs-v0.2.0"))
--- T1 (b) dotted keys input ---
api = 0
packs.core.git = "https://x"
packs.core.tag = "packs-v0.1.0" # c
packs.core.subdir = "packs/core"
--- end input ---
as_value_mut() is_some = true
--- T1 (b) dotted keys output ---
api = 0
packs.core.git = "https://x"
packs.core.tag = "packs-v0.2.0" # c
packs.core.subdir = "packs/core"
--- end output ---
toml reparse packs.core.tag = Some(String("packs-v0.2.0"))
--- T1 (c) standard table input ---
api = 0

[packs.core]
git = "https://x"
tag = "packs-v0.1.0"   # pinned
subdir = "packs/core"
--- end input ---
as_value_mut() is_some = true
--- T1 (c) standard table output ---
api = 0

[packs.core]
git = "https://x"
tag = "packs-v0.2.0"   # pinned
subdir = "packs/core"
--- end output ---
toml reparse packs.core.tag = Some(String("packs-v0.2.0"))
--- T1 (d1) doc["packs"]["nope"]["tag"].as_value_mut() on (c) ---
result: Ok("None")
doc after:
api = 0

[packs]
nope = {}

[packs.core]
git = "https://x"
tag = "packs-v0.1.0"   # pinned
subdir = "packs/core"
--- end doc ---
--- T1 (d2) only `&mut doc["packs"]["nope"]` on (c) ---
item: Ok("None")
doc after:
api = 0

[packs.core]
git = "https://x"
tag = "packs-v0.1.0"   # pinned
subdir = "packs/core"
--- end doc ---
--- T2 init --tool claude (no --pack) ---
kind: MissingRequiredArgument
exit_code: 2
render:
error: the following required arguments were not provided:
  --pack <PACK>

Usage: agentstacks init --tool <TOOL> --pack <PACK>

For more information, try '--help'.
--- end render ---
--- T2 init --help ---
kind: DisplayHelp
exit_code: 0
render:
Create agentstacks.toml

Usage: agentstacks init --tool <TOOL> --pack <PACK>

Options:
      --tool <TOOL>  Tool to set up
      --pack <PACK>  Pack to install
  -h, --help         Print help
--- end render ---
--- T3 base=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/rust-probes2/t3 ---
--- T3 cmd1 ---
exact command string: cd "$CLAUDE_PROJECT_DIR" && "airsl" "run" "--fail-open" "--policy" "confined" "--allow-read" ".claude/agentstacks/core" "--allow-write" "$TMPDIR" "--allow-env" "HOME" ".claude/agentstacks/runtime.lua" "core.hooks.enforce" "0" "neutral" "--x" "a \"q\" \\b \`c\`" # agentstacks
exit: Some(0)
stdout:
cwd=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/rust-probes2/t3/project
[run]
[--fail-open]
[--policy]
[confined]
[--allow-read]
[.claude/agentstacks/core]
[--allow-write]
[/tmp/probe-tmp]
[--allow-env]
[HOME]
[.claude/agentstacks/runtime.lua]
[core.hooks.enforce]
[0]
[neutral]
[--x]
[a "q" \b `c`]
--- end stdout ---
stderr:
--- end stderr ---
--- T3 cmd2 ---
exact command string: printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"it'"'"'s a\\b test"}}' # agentstacks
exit: Some(0)
stdout:
{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"it's a\\b test"}}
--- end stdout ---
stderr:
--- end stderr ---
--- T3 cmd3 ---
exact command string: echo hi # agentstacks
exit: Some(0)
stdout:
hi
--- end stdout ---
stderr:
--- end stderr ---
```

## T4: `airsl test --help | head -5` (airsl 0.1.4), raw

```text
Run the Lua test files under a directory.

A test file is one named `*_test.lua` or `test_*.lua`. It returns a table of named functions; each is a test, and a test passes by returning without raising — so Lua's own `assert` is the whole assertion surface and there is nothing new to learn.

Usage: airsl test [OPTIONS] [PATH]
exit 0
```

## Findings

- **T1** Retag via `doc["packs"]["core"]["tag"].as_value_mut()` returns `Some` for inline table
  (a), dotted keys (b) and standard table (c); copying the old `Decor` onto the new `Value` keeps
  every trailing comment and spacing, and every other byte is unchanged. `toml` 1.2.2 reparses each
  output to `packs-v0.2.0`.
- **T1 (d)** On a missing pack, `doc["packs"]["nope"]["tag"].as_value_mut()` returns `None` but
  the IndexMut chain **mutates the document**: it renders `[packs]\nnope = {}`. Indexing only
  `doc["packs"]["nope"]` inserts an `Item::None` that does not serialise (doc text unchanged).
  So check presence with `get`/`get_mut` before using IndexMut on a possibly missing key.
- **T2** clap 4.6.9: missing `--pack` gives kind `MissingRequiredArgument`, exit code 2, message
  shown above; `init --help` is `DisplayHelp`, exit code 0.
- **T3** `sh -c` with double-quoted args: `$CLAUDE_PROJECT_DIR` and `$TMPDIR` expand, `\"`, `\\`
  and `` \` `` unescape to `a "q" \b `c``, the trailing `# agentstacks` is a comment (not passed
  as an argument). The `printf '%s\n' '<json>'` form with `'"'"'` for a single quote prints the
  JSON unchanged (`it's`, `a\\b` kept). `echo hi # agentstacks` prints `hi`. All exit 0.
- **T4** `airsl test` exists in 0.1.4: runs `*_test.lua` / `test_*.lua` files under a directory;
  usage `airsl test [OPTIONS] [PATH]`.
