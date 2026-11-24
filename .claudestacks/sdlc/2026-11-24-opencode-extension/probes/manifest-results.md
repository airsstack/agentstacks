# Probe results: the spec's `extension.toml` loads (agentstacks at 8a96f78)

Project: a copy of `<scratchpad>/as-01/proj` (two tools, packs core and guideline-rust; see
`agentstacks-results.md` P1) at `<scratchpad>/as-ext/proj`, with `ext/opencode/extension.toml`
replaced by the spec §4.1 text:

```ext/opencode/extension.toml
[extension]
name        = "opencode"
version     = "0.1.0"
api         = 0
description = "Renders packs into OpenCode V2 project config"
entry       = "adapter.lua"
```

**Command**

```sh
cd <scratchpad>/as-ext/proj && <repo>/target/debug/agentstacks plan 2>&1 | tail -5; echo "exit=$?"
```

**Output**

```text
  skip     core/hook/9: opencode has no hooks

  airsl    0.1.5 (pinned)

0 to write, 0 to delete, 92 unchanged, 14 skipped
exit=0
```

**Answer**: the manifest with no `reads` key is accepted by `Extension::load`; the plan runs.
