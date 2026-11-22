# cargo deny and the workspace path dependency (2026-11-22)

Run in the repository root at commit `4a5d405`, with `Cargo.toml` line 14 edited for D1 and restored for D2. ANSI colour codes stripped with `sed`.

## D1 — path only

```text
$ cargo deny check bans   # workspace dependency: agentstacks = { path = "crates/agentstacks" }
bug[unresolved-workspace-dependency]: failed to resolve a workspace dependency
   ┌─ /Users/hiraq/.herdr/worktrees/agentstacks/cli-command-claude-ext/crates/agentstacks-cli/Cargo.toml:21:29
   │
21 │ agentstacks = { workspace = true }
   │               ──────────────━━━━──
   │                             │
   │                             usage of workspace dependency
   │
   ├ agentstacks-cli v0.0.0

bug[unresolved-workspace-dependency]: failed to resolve a workspace dependency
   ┌─ /Users/hiraq/.herdr/worktrees/agentstacks/cli-command-claude-ext/crates/agentstacks-cli/Cargo.toml:26:29
   │
26 │ agentstacks = { workspace = true, features = ["testing"] }
   │               ──────────────━━━━──────────────────────────
   │                             │
   │                             usage of workspace dependency
   │
   ├ agentstacks-cli v0.0.0 (*)

error[wildcard]: found 2 wildcard dependencies for crate 'agentstacks-cli'
   ┌─ /Users/hiraq/.herdr/worktrees/agentstacks/cli-command-claude-ext/crates/agentstacks-cli/Cargo.toml:21:15
   │
21 │ agentstacks = { workspace = true }
   │               ━━━━━━━━━━━━━━━━━━━━ wildcard dependency
   ·
26 │ agentstacks = { workspace = true, features = ["testing"] }
   │               ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ wildcard dependency
   │
   ├ agentstacks-cli v0.0.0

bans FAILED
```

## D2 — path and version

```text
$ cargo deny check   # workspace dependency: agentstacks = { path = "crates/agentstacks", version = "0.0.0" }
warning[license-not-encountered]: license was not encountered
   ┌─ /Users/hiraq/.herdr/worktrees/agentstacks/cli-command-claude-ext/deny.toml:45:6
   │
45 │     "CC0-1.0",
   │      ━━━━━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /Users/hiraq/.herdr/worktrees/agentstacks/cli-command-claude-ext/deny.toml:46:6
   │
46 │     "CDLA-Permissive-2.0",
   │      ━━━━━━━━━━━━━━━━━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /Users/hiraq/.herdr/worktrees/agentstacks/cli-command-claude-ext/deny.toml:47:6
   │
47 │     "ISC",
   │      ━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /Users/hiraq/.herdr/worktrees/agentstacks/cli-command-claude-ext/deny.toml:49:6
   │
49 │     "MIT-0",
   │      ━━━━━ unmatched license allowance

advisories ok, bans ok, licenses ok, sources ok
$ cargo deny --version
cargo-deny 0.20.2
```
