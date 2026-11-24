# init with official names

## 1. Today, against the real public repository (no packs-v tag)

```text
--- 1 init official names, real repo, no tag
--pack "core": no packs-v release tag in https://github.com/airsstack/agentstacks
--tool "claude": no packs-v release tag in https://github.com/airsstack/agentstacks
exit=1
```

## 2. With a packs-v0.1.0 tag (official URL redirected to a bare copy of HEAD via insteadOf)

```text
--- init
  airsl    0.1.5 (new pin)

50 to write, 0 to delete, 0 unchanged, 1 skipped
stdin is not a terminal; pass --yes to apply
agentstacks.toml written; nothing applied (run apply)
init exit=1
--- agentstacks.toml
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
--- plan (tail)

  airsl    0.1.5 (new pin)

50 to write, 0 to delete, 0 unchanged, 1 skipped
plan exit=0
--- --version
agentstacks 0.0.0
```

## 3. anonymous git ls-remote --tags https://github.com/airsstack/agentstacks

```text
exit=0
```
