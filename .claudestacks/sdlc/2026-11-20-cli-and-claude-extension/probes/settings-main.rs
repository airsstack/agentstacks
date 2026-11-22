//! Probe: settings.json merge facts (S1-S6 via a real plan run) and parse checks (S7).
use std::path::Path;

use agentstacks::fs::{StdFileTree, StdWorkspace};
use agentstacks::host::AirslAdapter;
use agentstacks::process::{GitCli, SystemBinary};
use agentstacks::types::{Extension, ProjectFile};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut argv = std::env::args().skip(1);
    let root = argv.next().ok_or("usage: settings-probe <project> <s7-ext-dir>")?;
    let ext_dir = argv.next().ok_or("usage: settings-probe <project> <s7-ext-dir>")?;

    println!("===== S1-S6 (plan run)");
    let workspace = StdWorkspace::open(&root)?;
    let git = GitCli::new("git", std::env::var_os("HOME"));
    let binary = SystemBinary::from_env();
    let adapter = AirslAdapter::new();
    match agentstacks::app::plan::plan(&workspace, &git, &binary, &adapter) {
        Err(found) => {
            println!("PLAN: Err");
            for d in found.into_sorted() {
                println!("  {d}");
            }
        }
        Ok(plan) => {
            println!("PLAN: Ok");
            for step in plan.steps() {
                let Some(bytes) = step.new.as_ref() else { continue };
                let name = step.path.to_string();
                if name == "out.json" {
                    let value: serde_json::Value = serde_json::from_slice(bytes)?;
                    println!("--- out.json (pretty, keys sorted)\n{}", serde_json::to_string_pretty(&value)?);
                } else if name.ends_with(".json") {
                    println!("--- {name} ({} bytes) debug-escaped:\n{:?}", bytes.len(), String::from_utf8_lossy(bytes));
                    println!("--- {name} raw:\n{}<EOF>", String::from_utf8_lossy(bytes));
                }
            }
        }
    }

    println!("===== S7a Extension::load({ext_dir})");
    match Extension::load(&StdFileTree::new(&ext_dir)) {
        Ok(ext) => println!("Ok: manifest={:?} reads={:?}", ext.manifest(), ext.reads()),
        Err(found) => {
            println!("Err:");
            for d in found.into_sorted() {
                println!("  {d}");
            }
        }
    }

    println!("===== S7b ProjectFile::parse");
    let text = "api = 0\n\n[tools.claude]\npath = \"ext\"\n\n[packs.team]\npath = \"packs/team\"\n";
    println!("input: {text:?}");
    match ProjectFile::parse(Path::new("agentstacks.toml"), text) {
        Ok(p) => println!("Ok: {p:?}"),
        Err(found) => {
            println!("Err:");
            for d in found.into_sorted() {
                println!("  {d}");
            }
        }
    }
    Ok(())
}
