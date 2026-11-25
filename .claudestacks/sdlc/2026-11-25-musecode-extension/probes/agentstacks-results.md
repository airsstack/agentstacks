# agentstacks probe — commit 33e3925, 2026-11-25

## A1 `init --tool musecode` today

```text
$ cargo build -q -p agentstacks-cli
$ cd "$(mktemp -d)" && target/debug/agentstacks init --tool musecode --pack core
--tool "musecode": not an official tool; pass a path, or add a git source to agentstacks.toml
exit=1
```

## A2 The spec §4.1 `extension.toml` loads (claude adapter copied under it)

```text
$ cat ext/musecode/extension.toml   # extracted from spec.md §4.1
[extension]
name        = "musecode"
version     = "0.1.0"
api         = 0
description = "Renders packs into Muse Code project config"
entry       = "adapter.lua"
reads       = [".muse/hooks.json"]
$ agentstacks init --tool ext/musecode --pack packs/demo --yes
musecode/adapter.lua:99: read: ".claude/settings.json" is not in extension.reads
agentstacks.toml written; fix the above and run apply
exit=1
```

