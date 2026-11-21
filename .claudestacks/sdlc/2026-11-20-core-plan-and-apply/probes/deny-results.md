# cargo-deny on the real workspace plus airsl (2026-11-22)

A copy of the worktree (no target, .git, .claude, .claudestacks) with three edits:
workspace deps `airsl = "0.1.4"` and `serde_json = { version = "1.0.154", features = ["preserve_order"] }`,
both added to crates/agentstacks, and `"Zlib"` added to deny.toml [licenses] allow. cargo-deny 0.20.2.

## Run 1: deny.toml bans as committed (multiple-versions = "deny", skip-tree = [])
```text
$ cargo deny check   # error headlines and summary
error[duplicate]: found 2 duplicate entries for crate 'block-buffer'
error[duplicate]: found 2 duplicate entries for crate 'cpufeatures'
error[duplicate]: found 2 duplicate entries for crate 'crypto-common'
error[duplicate]: found 2 duplicate entries for crate 'digest'
error[duplicate]: found 2 duplicate entries for crate 'serde_spanned'
error[duplicate]: found 2 duplicate entries for crate 'sha2'
error[duplicate]: found 2 duplicate entries for crate 'toml'
error[duplicate]: found 2 duplicate entries for crate 'toml_datetime'
error[duplicate]: found 2 duplicate entries for crate 'winnow'
advisories ok, bans FAILED, licenses ok, sources ok
```

## Run 2: skip-tree for airsl's subtree
```text
80:skip-tree = [{ crate = "airsl@0.1.4", reason = "airsl 0.1.4 pins sha2 0.10 and toml 0.8; agentstacks uses sha2 0.11 and toml 1" }]
$ cargo deny check   # error headlines and summary
advisories ok, bans ok, licenses ok, sources ok
$ cargo tree -i sha2@0.10.9 -e normal
sha2 v0.10.9
└── airsl v0.1.4
    └── agentstacks v0.0.0 (<probe copy>)
$ cargo tree -i toml@0.8.23 -e normal
toml v0.8.23
└── airsl v0.1.4
    └── agentstacks v0.0.0 (<probe copy>)
```
