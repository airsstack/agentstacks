//! Probe crate: toml_edit retag (T1), clap required Vec (T2), `sh -c` quoting (T3).
use std::os::unix::fs::PermissionsExt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::process::Command;

use clap::{Parser, Subcommand};
use toml_edit::{DocumentMut, Value};

fn retag(label: &str, input: &str) {
    println!("--- T1 {label} input ---\n{input}--- end input ---");
    let mut doc: DocumentMut = input.parse().expect("parse");
    let res = catch_unwind(AssertUnwindSafe(|| {
        let slot = doc["packs"]["core"]["tag"].as_value_mut();
        let some = slot.is_some();
        if let Some(old) = slot {
            let decor = old.decor().clone();
            let mut new = Value::from("packs-v0.2.0");
            *new.decor_mut() = decor;
            *old = new;
        }
        some
    }));
    match res {
        Ok(some) => println!("as_value_mut() is_some = {some}"),
        Err(_) => println!("PANICKED"),
    }
    let out = doc.to_string();
    println!("--- T1 {label} output ---\n{out}--- end output ---");
    match out.parse::<toml::Table>() {
        Ok(t) => println!(
            "toml reparse packs.core.tag = {:?}",
            t.get("packs").and_then(|p| p.get("core")).and_then(|c| c.get("tag"))
        ),
        Err(e) => println!("toml reparse error: {e}"),
    }
}

#[derive(Parser, Debug)]
#[command(name = "agentstacks")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Create agentstacks.toml.
    Init {
        /// Tool to set up.
        #[arg(long = "tool", value_name = "TOOL", required = true)]
        tool: Vec<String>,
        /// Pack to install.
        #[arg(long = "pack", value_name = "PACK", required = true)]
        pack: Vec<String>,
    },
}

fn run_sh(base: &PathBuf, cmd: &str) {
    println!("exact command string: {cmd}");
    let bin = base.join("bin");
    let proj = base.join("project");
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    let out = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .env("PATH", path)
        .env("TMPDIR", "/tmp/probe-tmp")
        .env("CLAUDE_PROJECT_DIR", &proj)
        .output()
        .expect("spawn sh");
    println!("exit: {:?}", out.status.code());
    println!("stdout:\n{}--- end stdout ---", String::from_utf8_lossy(&out.stdout));
    println!("stderr:\n{}--- end stderr ---", String::from_utf8_lossy(&out.stderr));
}

fn main() {
    std::panic::set_hook(Box::new(|info| println!("panic: {info}")));

    // T1
    retag(
        "(a) inline table",
        "api = 0\n\n[packs]\ncore = { git = \"https://x\", tag = \"packs-v0.1.0\", subdir = \"packs/core\" } # c\n",
    );
    retag(
        "(b) dotted keys",
        "api = 0\npacks.core.git = \"https://x\"\npacks.core.tag = \"packs-v0.1.0\" # c\npacks.core.subdir = \"packs/core\"\n",
    );
    let c = "api = 0\n\n[packs.core]\ngit = \"https://x\"\ntag = \"packs-v0.1.0\"   # pinned\nsubdir = \"packs/core\"\n";
    retag("(c) standard table", c);

    println!("--- T1 (d1) doc[\"packs\"][\"nope\"][\"tag\"].as_value_mut() on (c) ---");
    let mut doc: DocumentMut = c.parse().expect("parse");
    let r = catch_unwind(AssertUnwindSafe(|| {
        let v = doc["packs"]["nope"]["tag"].as_value_mut();
        format!("{v:?}")
    }));
    println!("result: {r:?}");
    println!("doc after:\n{doc}--- end doc ---");

    println!("--- T1 (d2) only `&mut doc[\"packs\"][\"nope\"]` on (c) ---");
    let mut doc: DocumentMut = c.parse().expect("parse");
    let r = catch_unwind(AssertUnwindSafe(|| {
        let item = &mut doc["packs"]["nope"];
        format!("{item:?}")
    }));
    println!("item: {r:?}");
    println!("doc after:\n{doc}--- end doc ---");

    // T2
    println!("--- T2 init --tool claude (no --pack) ---");
    match Cli::try_parse_from(["agentstacks", "init", "--tool", "claude"]) {
        Ok(c) => println!("UNEXPECTED Ok: {c:?}"),
        Err(e) => {
            println!("kind: {:?}", e.kind());
            println!("exit_code: {}", e.exit_code());
            println!("render:\n{}--- end render ---", e.render());
        }
    }
    println!("--- T2 init --help ---");
    match Cli::try_parse_from(["agentstacks", "init", "--help"]) {
        Ok(c) => println!("UNEXPECTED Ok: {c:?}"),
        Err(e) => {
            println!("kind: {:?}", e.kind());
            println!("exit_code: {}", e.exit_code());
            println!("render:\n{}--- end render ---", e.render());
        }
    }

    // T3
    let base = PathBuf::from(std::env::args().nth(1).expect("base dir arg"));
    std::fs::create_dir_all(base.join("bin")).expect("mkdir bin");
    std::fs::create_dir_all(base.join("project")).expect("mkdir project");
    let stub = base.join("bin").join("airsl");
    std::fs::write(&stub, "#!/bin/sh\necho \"cwd=$(pwd)\"\nfor a in \"$@\"; do printf '[%s]\\n' \"$a\"; done\n")
        .expect("write stub");
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    println!("--- T3 base={} ---", base.display());
    println!("--- T3 cmd1 ---");
    run_sh(
        &base,
        r##"cd "$CLAUDE_PROJECT_DIR" && "airsl" "run" "--fail-open" "--policy" "confined" "--allow-read" ".claude/agentstacks/core" "--allow-write" "$TMPDIR" "--allow-env" "HOME" ".claude/agentstacks/runtime.lua" "core.hooks.enforce" "0" "neutral" "--x" "a \"q\" \\b \`c\`" # agentstacks"##,
    );
    println!("--- T3 cmd2 ---");
    run_sh(
        &base,
        r##"printf '%s\n' '{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":"it'"'"'s a\\b test"}}' # agentstacks"##,
    );
    println!("--- T3 cmd3 ---");
    run_sh(&base, "echo hi # agentstacks");
}
