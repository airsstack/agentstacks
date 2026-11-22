//! Probe: run the real agentstacks plan (and optionally apply) against a fixture project.
use agentstacks::fs::StdWorkspace;
use agentstacks::host::AirslAdapter;
use agentstacks::process::{GitCli, SystemBinary};
use agentstacks::types::lock::write_lock;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut argv = std::env::args().skip(1);
    let root = argv.next().ok_or("usage: host-probe <project> [apply]")?;
    let do_apply = argv.next().as_deref() == Some("apply");

    let workspace = StdWorkspace::open(&root)?;
    let git = GitCli::new("git", std::env::var_os("HOME"));
    let binary = SystemBinary::from_env();
    let adapter = AirslAdapter::new();

    let plan = match agentstacks::app::plan::plan(&workspace, &git, &binary, &adapter) {
        Ok(plan) => plan,
        Err(found) => {
            println!("PLAN: Err");
            for d in found.into_sorted() {
                println!("  {d}");
            }
            return Ok(());
        }
    };
    println!("PLAN: Ok");
    println!("== steps");
    for step in plan.steps() {
        let owner = step
            .owner
            .as_ref()
            .map_or_else(|| "-".to_owned(), ToString::to_string);
        println!("- {} | {:?} | owner={} | origin={:?}", step.path, step.action, owner, step.origin);
    }
    println!("== step contents");
    for step in plan.steps() {
        let Some(bytes) = step.new.as_ref() else { continue };
        println!("--- {} ({} bytes)", step.path, bytes.len());
        if step.path.to_string() == "payload.json" {
            println!("raw: {}", String::from_utf8_lossy(bytes));
            let value: serde_json::Value = serde_json::from_slice(bytes)?;
            println!("pretty:\n{}", serde_json::to_string_pretty(&value)?);
        } else {
            println!("{}", String::from_utf8_lossy(bytes));
        }
    }
    println!("== skips");
    for s in plan.skips() {
        println!("- {s:?}");
    }
    println!("== notes");
    for n in plan.notes() {
        println!("- {n}");
    }
    println!("== binaries");
    for b in plan.binaries() {
        println!("- {b:?}");
    }
    println!("== airsl pin");
    println!("{:?}", plan.airsl());
    println!("== new lock");
    print!("{}", write_lock(plan.lock()));

    if do_apply {
        println!("== apply");
        match agentstacks::app::apply::apply(&plan, &workspace) {
            Ok(()) => println!("APPLY: Ok"),
            Err(found) => {
                println!("APPLY: Err");
                for d in found.into_sorted() {
                    println!("  {d}");
                }
            }
        }
    }
    Ok(())
}
