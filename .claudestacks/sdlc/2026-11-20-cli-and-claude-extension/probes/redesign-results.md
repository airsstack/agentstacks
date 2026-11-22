# Redesign probes (2026-11-22): gaps G1-G6

Toolchain: G1 and G2 first ran in the scratchpad under the default cargo 1.99.0, then were re-run with `cargo +1.94.1` (the pinned `rust-toolchain.toml`, cargo 1.94.1 (29ea6fb6a 2026-03-24)). G1 first run: `diff` of the two outputs is empty. G1 second run: identical (`P3b agentstacks.toml:1: unclosed table, expected `]``). G2: identical (`main TESTING=false`, `test TESTING=true`, release `main TESTING=false`). G2b ran inside a copy holding `rust-toolchain.toml`, on 1.94.1. Crates: agentstacks (path, this worktree at 951fa02), toml =1.2.0, toml_edit =0.25.20.

## G1, G4: Diagnostic display and parse errors

### src/main.rs (first run)
```rust
use agentstacks::types::report::{Diagnostic, Location};
use agentstacks::types::resolve::FetchError;
fn main() {
    let d = Diagnostic::new(Location::file("init"), FetchError::GitNotFound { needed_by: vec!["--tool \"claude\"".into(), "--pack \"core\"".into()] });
    println!("G1a {d}");
    let d = Diagnostic::new(Location::file("--pack \"core\""), FetchError::GitNotFound { needed_by: vec![] });
    println!("G1b {d}");
    for bad in ["[packs.core\ntag = 1\n", "a = 1\na = 2\n", "[packs.core]\ntag = \"x\"\n[packs.core]\n"] {
        let e = bad.parse::<toml_edit::DocumentMut>().err().map(|e| e.to_string());
        let t = toml::from_str::<toml::Table>(bad).err().map(|e| e.to_string());
        println!("P3 input={bad:?}\n  toml_edit={e:?}\n  toml={t:?}");
    }
}
```
### output
```
G1a init: git not found on PATH (needed by --tool "claude", --pack "core")
G1b --pack "core": git not found on PATH (needed by )
P3 input="[packs.core\ntag = 1\n"
  toml_edit=Some("TOML parse error at line 1, column 12\n  |\n1 | [packs.core\n  |            ^\nunclosed table, expected `]`\n")
  toml=Some("TOML parse error at line 1, column 12\n  |\n1 | [packs.core\n  |            ^\nunclosed table, expected `]`\n")
P3 input="a = 1\na = 2\n"
  toml_edit=Some("TOML parse error at line 2, column 1\n  |\n2 | a = 2\n  | ^\nduplicate key\n")
  toml=Some("TOML parse error at line 2, column 1\n  |\n2 | a = 2\n  | ^\nduplicate key\n")
P3 input="[packs.core]\ntag = \"x\"\n[packs.core]\n"
  toml_edit=Some("TOML parse error at line 3, column 8\n  |\n3 | [packs.core]\n  |        ^^^^\nduplicate key\n")
  toml=Some("TOML parse error at line 3, column 8\n  |\n3 | [packs.core]\n  |        ^^^^\nduplicate key\n")
```
### `cargo tree -i toml_parser`
```
toml_parser v1.1.6+spec-1.1.0
├── toml v1.2.0+spec-1.1.0
│   ├── agentstacks v0.0.0 (/Users/hiraq/.herdr/worktrees/agentstacks/cli-command-claude-ext/crates/agentstacks)
│   │   └── g1 v0.0.0 (<scratch>/g1)
│   └── g1 v0.0.0 (<scratch>/g1)
└── toml_edit v0.25.20+spec-1.1.0
    └── g1 v0.0.0 (<scratch>/g1)
```
### src/main.rs (second run)
```rust
use std::path::Path;
fn main() {
    let r = agentstacks::types::project::ProjectFile::parse(Path::new("agentstacks.toml"), "[packs.core\n");
    for d in r.err().map(|d| d.into_sorted()).unwrap_or_default() { println!("P3b {d}"); }
}
```
### output
```
P3b agentstacks.toml:1: unclosed table, expected `]`
```

## G2: test fakes behind a feature, used from a dependent crate (resolver 3)

### Cargo.toml
```
[workspace]
resolver = "3"
members = ["lib", "cli"]
```
### lib/Cargo.toml
```
[package]
name = "lib"
version = "0.0.0"
edition = "2024"
[features]
testing = []
```
### lib/src/lib.rs
```
pub const TESTING: bool = cfg!(feature = "testing");
#[cfg(any(test, feature = "testing"))]
pub mod fake { pub struct FakeWorkspace; impl FakeWorkspace { pub fn new() -> Self { Self } } }
```
### cli/Cargo.toml
```
[package]
name = "cli"
version = "0.0.0"
edition = "2024"
[dependencies]
lib = { path = "../lib" }
[dev-dependencies]
lib = { path = "../lib", features = ["testing"] }
```
### cli/src/main.rs
```
fn main() { println!("main TESTING={}", lib::TESTING); }
#[cfg(test)]
mod tests { #[test] fn fake_visible() { let _ = lib::fake::FakeWorkspace::new(); println!("test TESTING={}", lib::TESTING); assert!(lib::TESTING); } }
```
### output
```
$ cargo run -q -p cli
main TESTING=false
$ cargo test -q -p cli -- --nocapture
running 1 test
test TESTING=true
.
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
$ cargo build -q -p cli --release && ./target/release/cli
main TESTING=false
```

## G2b: the real fakes compiled under `--features testing` with the gate's lints

Setup in a scratch copy (`git archive HEAD` of the workspace manifests and `crates/agentstacks`, members narrowed to `crates/agentstacks`):
```
printf '\n[features]\ntesting = []\n' >> crates/agentstacks/Cargo.toml
for f in crates/agentstacks/src/port/mod.rs crates/agentstacks/src/port/*/mod.rs; do
  perl -0pi -e 's/#\[cfg\(test\)\]\n(pub mod fake|mod fake|#\[doc\(inline\)\]\npub use)/#[cfg(any(test, feature = "testing"))]\n$1/g' $f
done
```
Gated lines per file (`grep -c` of `feature = "testing"`):
```
crates/agentstacks/src/port/binary/mod.rs:3
crates/agentstacks/src/port/git/mod.rs:3
crates/agentstacks/src/port/mod.rs:12
crates/agentstacks/src/port/adapter/mod.rs:5
crates/agentstacks/src/port/workspace/mod.rs:3
```
Output (cargo 1.94.1 (29ea6fb6a 2026-03-24)):
```
$ cargo clippy -q -p agentstacks --lib --features testing -- -D warnings
exit=0
$ RUSTDOCFLAGS="-D warnings" cargo doc -q -p agentstacks --features testing --no-deps
exit=0
$ ls target/doc/agentstacks/port/ | grep -i fake
fake
struct.FakeAdapter.html
struct.FakeBinary.html
struct.FakeFileTree.html
struct.FakeGit.html
struct.FakeWorkspace.html
```
