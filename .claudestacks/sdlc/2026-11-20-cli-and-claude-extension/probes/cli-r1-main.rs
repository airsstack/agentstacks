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
