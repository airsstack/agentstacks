# CLI and claude extension: dependency probes (R1-R5)

Run 2026-11-22 on aarch64-apple-darwin. Toolchain: the worktree `rust-toolchain.toml` copied into the probe directory, so every probe ran on:

```text
rustc 1.94.1 (e408947bf 2026-03-25)
cargo 1.94.1 (29ea6fb6a 2026-03-24)
cargo-deny 0.20.2
```

Each probe is a throwaway cargo crate; versions below are copied from that crate's `Cargo.lock`. Outputs are captured to files by the shell and pasted here unmodified (`cat`). Each probe's `main.rs` is saved beside this file as `cli-r<N>-main.rs`.

## R1 clap derive: subcommands, required repeatable options, global `-C`, exit codes

Question: with clap 4.6.9 derive, how do `required = true` + `Vec<String>` options behave, what are the exact error texts, `ErrorKind`s and exit codes for a missing required arg, an unknown subcommand, no subcommand, `--help` and `--version`, and which stream does each go to?

Versions (probe `Cargo.lock`):

```text
clap version = "4.6.9"
clap_builder version = "4.6.7"
clap_derive version = "4.6.9"
```

Manifest dependency: `clap = { version = "=4.6.9", features = ["derive"] }`

Source (`cli-r1-main.rs`):

```rust
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "agentstacks", version, about = "probe")]
struct Cli {
    /// Run as if started in DIR.
    #[arg(short = 'C', global = true, value_name = "DIR")]
    dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// init
    Init {
        #[arg(long = "tool", value_name = "T", required = true)]
        tool: Vec<String>,
        #[arg(long = "pack", value_name = "P", required = true)]
        pack: Vec<String>,
    },
    /// plan
    Plan,
    /// apply
    Apply {
        #[arg(long)]
        yes: bool,
    },
    /// doctor
    Doctor,
    /// update
    Update,
}

fn show(args: &[&str]) {
    println!("=== try_parse_from {args:?}");
    match Cli::try_parse_from(args) {
        Ok(cli) => println!("Ok({cli:?})"),
        Err(e) => {
            println!("kind={:?} exit_code={} use_stderr={}", e.kind(), e.exit_code(), e.use_stderr());
            println!("rendered:\n{}", e.render());
        }
    }
}

fn main() {
    if std::env::var_os("PROBE_TRY").is_some() {
        show(&["agentstacks", "init", "--tool", "claude"]);
        show(&["agentstacks", "init", "--tool", "claude", "--pack", "core", "--pack", "sdlc"]);
        show(&["agentstacks", "init", "--tool", "claude", "--pack"]);
        show(&["agentstacks", "init"]);
        show(&["agentstacks", "frobnicate"]);
        show(&["agentstacks"]);
        show(&["agentstacks", "--help"]);
        show(&["agentstacks", "--version"]);
        show(&["agentstacks", "plan", "-C", "/tmp/x"]);
        show(&["agentstacks", "-C", "/tmp/x", "apply", "--yes"]);
        show(&["agentstacks", "apply"]);
        return;
    }
    let cli = Cli::parse();
    println!("{cli:?}");
}
```

Commands: `cargo build`, then `PROBE_TRY=1 ./target/debug/r1` (the `try_parse_from` table) and the real binary once per argument set, stdout and stderr captured separately, `exit=$?` printed after each.

Raw output:

```text
$ PROBE_TRY=1 ./target/debug/r1
=== try_parse_from ["agentstacks", "init", "--tool", "claude"]
kind=MissingRequiredArgument exit_code=2 use_stderr=true
rendered:
error: the following required arguments were not provided:
  --pack <P>

Usage: agentstacks init --tool <T> --pack <P>

For more information, try '--help'.

=== try_parse_from ["agentstacks", "init", "--tool", "claude", "--pack", "core", "--pack", "sdlc"]
Ok(Cli { dir: None, command: Init { tool: ["claude"], pack: ["core", "sdlc"] } })
=== try_parse_from ["agentstacks", "init", "--tool", "claude", "--pack"]
kind=InvalidValue exit_code=2 use_stderr=true
rendered:
error: a value is required for '--pack <P>' but none was supplied

For more information, try '--help'.

=== try_parse_from ["agentstacks", "init"]
kind=MissingRequiredArgument exit_code=2 use_stderr=true
rendered:
error: the following required arguments were not provided:
  --tool <T>
  --pack <P>

Usage: agentstacks init --tool <T> --pack <P>

For more information, try '--help'.

=== try_parse_from ["agentstacks", "frobnicate"]
kind=InvalidSubcommand exit_code=2 use_stderr=true
rendered:
error: unrecognized subcommand 'frobnicate'

Usage: agentstacks [OPTIONS] <COMMAND>

For more information, try '--help'.

=== try_parse_from ["agentstacks"]
kind=DisplayHelpOnMissingArgumentOrSubcommand exit_code=2 use_stderr=true
rendered:
probe

Usage: agentstacks [OPTIONS] <COMMAND>

Commands:
  init    init
  plan    plan
  apply   apply
  doctor  doctor
  update  update
  help    Print this message or the help of the given subcommand(s)

Options:
  -C <DIR>       Run as if started in DIR
  -h, --help     Print help
  -V, --version  Print version

=== try_parse_from ["agentstacks", "--help"]
kind=DisplayHelp exit_code=0 use_stderr=false
rendered:
probe

Usage: agentstacks [OPTIONS] <COMMAND>

Commands:
  init    init
  plan    plan
  apply   apply
  doctor  doctor
  update  update
  help    Print this message or the help of the given subcommand(s)

Options:
  -C <DIR>       Run as if started in DIR
  -h, --help     Print help
  -V, --version  Print version

=== try_parse_from ["agentstacks", "--version"]
kind=DisplayVersion exit_code=0 use_stderr=false
rendered:
agentstacks 0.1.0

=== try_parse_from ["agentstacks", "plan", "-C", "/tmp/x"]
Ok(Cli { dir: Some("/tmp/x"), command: Plan })
=== try_parse_from ["agentstacks", "-C", "/tmp/x", "apply", "--yes"]
Ok(Cli { dir: Some("/tmp/x"), command: Apply { yes: true } })
=== try_parse_from ["agentstacks", "apply"]
Ok(Cli { dir: None, command: Apply { yes: false } })
$ ./target/debug/r1 init --tool claude
exit=2
--- stdout:
--- stderr:
error: the following required arguments were not provided:
  --pack <P>

Usage: r1 init --tool <T> --pack <P>

For more information, try '--help'.
$ ./target/debug/r1 frobnicate
exit=2
--- stdout:
--- stderr:
error: unrecognized subcommand 'frobnicate'

Usage: r1 [OPTIONS] <COMMAND>

For more information, try '--help'.
$ ./target/debug/r1 
exit=2
--- stdout:
--- stderr:
probe

Usage: r1 [OPTIONS] <COMMAND>

Commands:
  init    init
  plan    plan
  apply   apply
  doctor  doctor
  update  update
  help    Print this message or the help of the given subcommand(s)

Options:
  -C <DIR>       Run as if started in DIR
  -h, --help     Print help
  -V, --version  Print version
$ ./target/debug/r1 --help
exit=0
--- stdout:
probe

Usage: r1 [OPTIONS] <COMMAND>

Commands:
  init    init
  plan    plan
  apply   apply
  doctor  doctor
  update  update
  help    Print this message or the help of the given subcommand(s)

Options:
  -C <DIR>       Run as if started in DIR
  -h, --help     Print help
  -V, --version  Print version
--- stderr:
$ ./target/debug/r1 --version
exit=0
--- stdout:
agentstacks 0.1.0
--- stderr:
$ ./target/debug/r1 -C /tmp/x apply --yes
exit=0
--- stdout:
Cli { dir: Some("/tmp/x"), command: Apply { yes: true } }
--- stderr:
$ ./target/debug/r1 plan -C /tmp/x
exit=0
--- stdout:
Cli { dir: Some("/tmp/x"), command: Plan }
--- stderr:
```

## R2 toml_edit: in-place edit and a new document, read back by toml 1

Question: newest `toml_edit` that resolves with `toml = "1"`; (a) does setting `packs.core.tag` keep comments, the trailing `# pinned` comment and alignment; (b) the exact text of a document built from nothing; (c) does (b) parse with `toml::from_str::<toml::Table>`.

Versions (probe `Cargo.lock`; `toml` pinned `=1.2.0` to match the workspace `Cargo.lock`; `cargo search toml_edit` reported `0.25.20+spec-1.1.0` as newest):

```text
toml_edit version = "0.25.20+spec-1.1.0"
toml version = "1.2.0+spec-1.1.0"
toml_parser version = "1.1.6+spec-1.1.0"
toml_datetime version = "1.1.3+spec-1.1.0"
toml_writer version = "1.2.0+spec-1.1.0"
```

Source (`cli-r2-main.rs`):

```rust
use toml_edit::{DocumentMut, Item, Table, value};

const INPUT: &str = r#"# my project
api = 0

[tools.claude]  # the claude extension
git    = "https://github.com/airsstack/agentstacks"
tag    = "packs-v0.1.0"
subdir = "extensions/claude"

[packs.core]
git     = "https://github.com/airsstack/agentstacks"
tag     = "packs-v0.1.0"   # pinned
subdir  = "packs/core"
options = { archive_keep = 50 }
"#;

fn source(subdir: &str) -> Table {
    let mut t = Table::new();
    t.insert("git", value("https://github.com/airsstack/agentstacks"));
    t.insert("tag", value("packs-v0.1.0"));
    t.insert("subdir", value(subdir));
    t
}

fn main() {
    // (a) edit packs.core.tag in place
    let mut doc: DocumentMut = INPUT.parse().expect("parse input");
    doc["packs"]["core"]["tag"] = value("packs-v0.2.0");
    let out_a = doc.to_string();
    println!("=== (a) output Debug (exact bytes):\n{out_a:?}");
    println!("=== (a) output text:\n{out_a}=== end (a)");

    // (a2) same edit, but keep the existing value's decor (prefix/suffix) explicitly
    let mut doc2: DocumentMut = INPUT.parse().expect("parse input");
    if let Some(v) = doc2["packs"]["core"]["tag"].as_value_mut() {
        let decor = v.decor().clone();
        *v = toml_edit::Value::from("packs-v0.2.0");
        *v.decor_mut() = decor;
    }
    let out_a2 = doc2.to_string();
    println!("=== (a2) decor-preserving edit Debug:\n{out_a2:?}");
    println!("=== (a2) text:\n{out_a2}=== end (a2)");

    // (b) new document from nothing
    let mut doc = DocumentMut::new();
    doc.insert("api", value(0));
    let mut tools = Table::new();
    tools.set_implicit(true);
    tools.insert("claude", Item::Table(source("extensions/claude")));
    doc.insert("tools", Item::Table(tools));
    let mut packs = Table::new();
    packs.set_implicit(true);
    packs.insert("core", Item::Table(source("packs/core")));
    packs.insert("sdlc", Item::Table(source("packs/sdlc")));
    doc.insert("packs", Item::Table(packs));
    let out_b = doc.to_string();
    println!("=== (b) output Debug (exact bytes):\n{out_b:?}");
    println!("=== (b) output text:\n{out_b}=== end (b)");

    // (b2) same but without set_implicit, to show the difference
    let mut doc = DocumentMut::new();
    doc.insert("api", value(0));
    let mut tools = Table::new();
    tools.insert("claude", Item::Table(source("extensions/claude")));
    doc.insert("tools", Item::Table(tools));
    let out_b2 = doc.to_string();
    println!("=== (b2) without set_implicit:\n{out_b2}=== end (b2)");

    // (c) parse (b) and (a) with toml 1
    match toml::from_str::<toml::Table>(&out_b) {
        Ok(t) => println!("=== (c) toml::from_str(b) Ok: {t:?}"),
        Err(e) => println!("=== (c) toml::from_str(b) Err: {e}"),
    }
    match toml::from_str::<toml::Table>(&out_a) {
        Ok(t) => println!("=== (c) toml::from_str(a) Ok: {t:?}"),
        Err(e) => println!("=== (c) toml::from_str(a) Err: {e}"),
    }
}
```

Command: `cargo run --quiet`

Raw output:

```text
=== (a) output Debug (exact bytes):
"# my project\napi = 0\n\n[tools.claude]  # the claude extension\ngit    = \"https://github.com/airsstack/agentstacks\"\ntag    = \"packs-v0.1.0\"\nsubdir = \"extensions/claude\"\n\n[packs.core]\ngit     = \"https://github.com/airsstack/agentstacks\"\ntag     = \"packs-v0.2.0\"\nsubdir  = \"packs/core\"\noptions = { archive_keep = 50 }\n"
=== (a) output text:
# my project
api = 0

[tools.claude]  # the claude extension
git    = "https://github.com/airsstack/agentstacks"
tag    = "packs-v0.1.0"
subdir = "extensions/claude"

[packs.core]
git     = "https://github.com/airsstack/agentstacks"
tag     = "packs-v0.2.0"
subdir  = "packs/core"
options = { archive_keep = 50 }
=== end (a)
=== (a2) decor-preserving edit Debug:
"# my project\napi = 0\n\n[tools.claude]  # the claude extension\ngit    = \"https://github.com/airsstack/agentstacks\"\ntag    = \"packs-v0.1.0\"\nsubdir = \"extensions/claude\"\n\n[packs.core]\ngit     = \"https://github.com/airsstack/agentstacks\"\ntag     = \"packs-v0.2.0\"   # pinned\nsubdir  = \"packs/core\"\noptions = { archive_keep = 50 }\n"
=== (a2) text:
# my project
api = 0

[tools.claude]  # the claude extension
git    = "https://github.com/airsstack/agentstacks"
tag    = "packs-v0.1.0"
subdir = "extensions/claude"

[packs.core]
git     = "https://github.com/airsstack/agentstacks"
tag     = "packs-v0.2.0"   # pinned
subdir  = "packs/core"
options = { archive_keep = 50 }
=== end (a2)
=== (b) output Debug (exact bytes):
"api = 0\n\n[tools.claude]\ngit = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\nsubdir = \"extensions/claude\"\n\n[packs.core]\ngit = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\nsubdir = \"packs/core\"\n\n[packs.sdlc]\ngit = \"https://github.com/airsstack/agentstacks\"\ntag = \"packs-v0.1.0\"\nsubdir = \"packs/sdlc\"\n"
=== (b) output text:
api = 0

[tools.claude]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "extensions/claude"

[packs.core]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "packs/core"

[packs.sdlc]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "packs/sdlc"
=== end (b)
=== (b2) without set_implicit:
api = 0

[tools]

[tools.claude]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "extensions/claude"
=== end (b2)
=== (c) toml::from_str(b) Ok: {"api": Integer(0), "packs": Table({"core": Table({"git": String("https://github.com/airsstack/agentstacks"), "subdir": String("packs/core"), "tag": String("packs-v0.1.0")}), "sdlc": Table({"git": String("https://github.com/airsstack/agentstacks"), "subdir": String("packs/sdlc"), "tag": String("packs-v0.1.0")})}), "tools": Table({"claude": Table({"git": String("https://github.com/airsstack/agentstacks"), "subdir": String("extensions/claude"), "tag": String("packs-v0.1.0")})})}
=== (c) toml::from_str(a) Ok: {"api": Integer(0), "packs": Table({"core": Table({"git": String("https://github.com/airsstack/agentstacks"), "options": Table({"archive_keep": Integer(50)}), "subdir": String("packs/core"), "tag": String("packs-v0.2.0")})}), "tools": Table({"claude": Table({"git": String("https://github.com/airsstack/agentstacks"), "subdir": String("extensions/claude"), "tag": String("packs-v0.1.0")})})}
```

## R3 similar: unified diff of a settings-like JSON file

Question: exact bytes of `TextDiff::from_lines(old, new).unified_diff().context_radius(3).header("a", "b")` when a `hooks` key is added after `permissions`; plus identical inputs, empty old, and missing trailing newline; license.

Versions (probe `Cargo.lock`; `cargo search similar` reported `3.2.1` as newest):

```text
similar version = "3.2.1"
```

License, from the registry copy of its manifest:

```text
$ grep -E '^license' ~/.cargo/registry/src/*/similar-3.2.1/Cargo.toml
license = "Apache-2.0"
```

Source (`cli-r3-main.rs`):

```rust
use similar::TextDiff;

const OLD: &str = "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(ls:*)\"\n    ]\n  }\n}\n";
const NEW: &str = "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(ls:*)\"\n    ]\n  },\n  \"hooks\": {\n    \"SessionEnd\": [\n      {\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"airsl run .claude/hooks/end.lua\"\n          }\n        ]\n      }\n    ]\n  }\n}\n";

fn main() {
    let diff = TextDiff::from_lines(OLD, NEW);
    let out = diff.unified_diff().context_radius(3).header("a", "b").to_string();
    println!("=== Debug (exact bytes):\n{out:?}");
    println!("=== text:");
    print!("{out}");
    println!("=== end");
    let same = TextDiff::from_lines(OLD, OLD).unified_diff().context_radius(3).header("a", "b").to_string();
    println!("=== identical inputs -> {same:?}");
    // file created from nothing (old empty)
    let created = TextDiff::from_lines("", "x\ny\n").unified_diff().context_radius(3).header("a", "b").to_string();
    println!("=== empty old -> {created:?}");
    // missing trailing newline on new
    let nonl = TextDiff::from_lines("a\n", "a\nb").unified_diff().context_radius(3).header("a", "b").to_string();
    println!("=== new without trailing newline -> {nonl:?}");
}
```

Command: `cargo run --quiet`

Raw output:

```text
=== Debug (exact bytes):
"--- a\n+++ b\n@@ -3,5 +3,17 @@\n     \"allow\": [\n       \"Bash(ls:*)\"\n     ]\n+  },\n+  \"hooks\": {\n+    \"SessionEnd\": [\n+      {\n+        \"hooks\": [\n+          {\n+            \"type\": \"command\",\n+            \"command\": \"airsl run .claude/hooks/end.lua\"\n+          }\n+        ]\n+      }\n+    ]\n   }\n }\n"
=== text:
--- a
+++ b
@@ -3,5 +3,17 @@
     "allow": [
       "Bash(ls:*)"
     ]
+  },
+  "hooks": {
+    "SessionEnd": [
+      {
+        "hooks": [
+          {
+            "type": "command",
+            "command": "airsl run .claude/hooks/end.lua"
+          }
+        ]
+      }
+    ]
   }
 }
=== end
=== identical inputs -> ""
=== empty old -> "--- a\n+++ b\n@@ -0,0 +1,2 @@\n+x\n+y\n"
=== new without trailing newline -> "--- a\n+++ b\n@@ -1 +1,2 @@\n a\n+b\n\\ No newline at end of file\n"
```

## R4 std::io::IsTerminal and read_line on a closed stdin

Question: `stdin().is_terminal()` with piped stdin and with `/dev/null`; what `read_line` returns on `/dev/null`.

Versions: std only (rustc 1.94.1, above). No crates.

Source (`cli-r4-main.rs`):

```rust
use std::io::{self, BufRead, IsTerminal};

fn main() {
    let stdin = io::stdin();
    println!("stdin.is_terminal() = {}", stdin.is_terminal());
    println!("stdout.is_terminal() = {}", io::stdout().is_terminal());
    let mut line = String::new();
    let r = stdin.lock().read_line(&mut line);
    println!("read_line -> {r:?}, line = {line:?}");
}
```

Commands and raw output (the `script(1)` row is only a check that a pty reads as a terminal; its `read_line` result reflects `script` forwarding EOF, not interactive typing):

```text
$ echo y | ./target/debug/r4
stdin.is_terminal() = false
stdout.is_terminal() = false
read_line -> Ok(2), line = "y\n"
exit=0
$ ./target/debug/r4 < /dev/null
stdin.is_terminal() = false
stdout.is_terminal() = false
read_line -> Ok(0), line = ""
exit=0
$ printf "yes" | ./target/debug/r4
stdin.is_terminal() = false
stdout.is_terminal() = false
read_line -> Ok(3), line = "yes"
exit=0
$ printf 'y\n' | script -q /dev/null ./target/debug/r4 | tr -d '\r'   # script(1) gives r4 a pty as stdin/stdout
^Dy
stdin.is_terminal() = true
stdout.is_terminal() = true
read_line -> Ok(0), line = ""
exit=0
```

## R5 cargo-deny and duplicate versions with toml_edit + similar added

Question: does the workspace policy (`deny.toml`) still pass, and what duplicates appear, after adding `toml_edit` and `similar` to `[workspace.dependencies]` and to `crates/agentstacks-cli`? Run on an rsync copy of the worktree (excluding `target/` and `.git`); the real worktree was not touched.

Manifest changes in the copy (`diff` real copy):

```text
$ diff <worktree>/Cargo.toml Cargo.toml
24a25,26
> toml_edit = "0.25.20"
> similar = "3.2.1"
$ diff <worktree>/crates/agentstacks-cli/Cargo.toml crates/agentstacks-cli/Cargo.toml
21a22,23
> toml_edit = { workspace = true }
> similar = { workspace = true }
```

Resolved versions (copy `Cargo.lock`):

```text
toml_edit version = "0.22.27"
toml_edit version = "0.25.20+spec-1.1.0"
similar version = "3.2.1"
toml version = "0.8.23"
toml version = "1.2.0+spec-1.1.0"
winnow version = "0.7.15"
winnow version = "1.0.4"
```

Commands, run in the copy, and raw output:

```text
$ cargo tree -d -e normal
    Updating crates.io index
     Locking 4 packages to latest Rust 1.94 compatible versions
      Adding similar v3.2.1
    Updating toml_datetime v1.1.2+spec-1.1.0 -> v1.1.3+spec-1.1.0
      Adding toml_edit v0.25.20+spec-1.1.0
    Updating toml_parser v1.1.5+spec-1.1.0 -> v1.1.6+spec-1.1.0
block-buffer v0.10.4
└── digest v0.10.7
    ├── sha1 v0.10.7
    │   └── airsl v0.1.4
    │       └── agentstacks v0.0.0 (/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/rust-probes/r5ws/crates/agentstacks)
    └── sha2 v0.10.9
        └── airsl v0.1.4 (*)

block-buffer v0.12.1
└── digest v0.11.3
    └── sha2 v0.11.1
        └── agentstacks v0.0.0 (/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/rust-probes/r5ws/crates/agentstacks)

cpufeatures v0.2.17
├── sha1 v0.10.7 (*)
└── sha2 v0.10.9 (*)

cpufeatures v0.3.1
└── sha2 v0.11.1 (*)

crypto-common v0.1.7
└── digest v0.10.7 (*)

crypto-common v0.2.2
└── digest v0.11.3 (*)

digest v0.10.7 (*)

digest v0.11.3 (*)

serde_spanned v0.6.9
├── toml v0.8.23
│   └── airsl v0.1.4 (*)
└── toml_edit v0.22.27
    └── toml v0.8.23 (*)

serde_spanned v1.1.2
└── toml v1.2.0+spec-1.1.0
    └── agentstacks v0.0.0 (/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/rust-probes/r5ws/crates/agentstacks)

sha2 v0.10.9 (*)

sha2 v0.11.1 (*)

toml v0.8.23 (*)

toml v1.2.0+spec-1.1.0 (*)

toml_datetime v0.6.11
├── toml v0.8.23 (*)
└── toml_edit v0.22.27 (*)

toml_datetime v1.1.3+spec-1.1.0
├── toml v1.2.0+spec-1.1.0 (*)
└── toml_edit v0.25.20+spec-1.1.0
    └── agentstacks-cli v0.0.0 (/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/rust-probes/r5ws/crates/agentstacks-cli)

toml_edit v0.22.27 (*)

toml_edit v0.25.20+spec-1.1.0 (*)

winnow v0.7.15
└── toml_edit v0.22.27 (*)

winnow v1.0.4
├── toml v1.2.0+spec-1.1.0 (*)
├── toml_edit v0.25.20+spec-1.1.0 (*)
└── toml_parser v1.1.6+spec-1.1.0
    ├── toml v1.2.0+spec-1.1.0 (*)
    └── toml_edit v0.25.20+spec-1.1.0 (*)
exit=0
$ cargo deny check
warning[license-not-encountered]: license was not encountered
   ┌─ /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/rust-probes/r5ws/deny.toml:45:6
   │
45 │     "CC0-1.0",
   │      ━━━━━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/rust-probes/r5ws/deny.toml:46:6
   │
46 │     "CDLA-Permissive-2.0",
   │      ━━━━━━━━━━━━━━━━━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/rust-probes/r5ws/deny.toml:47:6
   │
47 │     "ISC",
   │      ━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-cli-command-claude-ext/d352388e-4150-435e-8b29-db273ed91466/scratchpad/rust-probes/r5ws/deny.toml:49:6
   │
49 │     "MIT-0",
   │      ━━━━━ unmatched license allowance

advisories ok, bans ok, licenses ok, sources ok
exit=0
```

## Findings (read from the raw output above)

- R1: missing `--pack` is `ErrorKind::MissingRequiredArgument`, `exit_code() = 2`, stderr, text `error: the following required arguments were not provided:\n  --pack <P>`; `--pack` with no value is `InvalidValue` (exit 2); unknown subcommand is `InvalidSubcommand` (exit 2, `error: unrecognized subcommand 'frobnicate'`); no subcommand is `DisplayHelpOnMissingArgumentOrSubcommand`, exit 2, full help on stderr; `--help` is `DisplayHelp`, exit 0, stdout; `--version` is `DisplayVersion`, exit 0, stdout, `<name> <CARGO_PKG_VERSION>`. A `global = true` `-C` is accepted before or after the subcommand. The real binary prints `Usage: <argv[0] file name>` (here `r1`), not `#[command(name)]`; `try_parse_from` uses the given argv[0] (`agentstacks`). `--version` uses `#[command(name)]`.
- R2: plain `doc["packs"]["core"]["tag"] = value(..)` keeps key alignment and all other comments but DROPS the value's trailing `   # pinned`; copying the old value's `decor()` onto the new value keeps it. A new document needs `set_implicit(true)` on the parent `tools`/`packs` tables, otherwise an empty `[tools]` header is written. Output of (b) parses with `toml` 1.2.0.
- R3: `similar` 3.2.1, license `Apache-2.0` (in the deny allow list). Identical inputs give an empty string; a missing trailing newline adds `\ No newline at end of file`.
- R4: piped and `/dev/null` stdin both report `is_terminal() = false`; `read_line` on `/dev/null` returns `Ok(0)` with an empty line.
- R5: `cargo deny check` passes (advisories, bans, licenses, sources ok; only the pre-existing unmatched-license warnings). The new duplicates (`toml_edit` 0.22/0.25, `winnow` 0.7/1.0) all sit under `airsl`, covered by the existing `skip-tree`. Adding `toml_edit` bumped `toml_datetime` 1.1.2 -> 1.1.3 and `toml_parser` 1.1.5 -> 1.1.6 in the lock.
