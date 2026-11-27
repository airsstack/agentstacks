use std::ffi::OsStr;
use std::path::Path;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "agentstacks", version, about = "probe")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    Plan {
        #[arg(long, conflicts_with = "verbose")]
        summary: bool,
        #[arg(long)]
        verbose: bool,
    },
    Init {
        #[arg(long)]
        dry_run: bool,
    },
}

fn show(args: &[&str]) {
    println!("=== try_parse_from {args:?}");
    match Cli::try_parse_from(args) {
        Ok(cli) => println!("Ok({cli:?})"),
        Err(e) => println!("kind={:?} exit_code={} rendered={:?}", e.kind(), e.exit_code(), e.to_string()),
    }
}

fn main() {
    println!("-- R1 clap flag parsing --");
    show(&["agentstacks", "plan", "--summary"]);
    show(&["agentstacks", "plan", "--verbose"]);
    show(&["agentstacks", "plan", "--summary", "--verbose"]);
    show(&["agentstacks", "plan", "--bogus"]);
    show(&["agentstacks", "init", "--dry-run"]);

    println!("-- R2 cache_home / url_dir (agentstacks 1.0.0) --");
    for home in [None, Some(""), Some("/home/u"), Some("/sandbox/home")] {
        let r = agentstacks::types::resolve::cache_home(home.map(OsStr::new));
        println!("cache_home({home:?}) = {r:?}");
    }
    let url = agentstacks::types::GitUrl::new("https://github.com/airsstack/agentstacks").expect("url");
    println!("url_dir(/home/u) = {}", agentstacks::types::resolve::url_dir(Path::new("/home/u"), &url).display());
    println!("url_dir(/sandbox/home) = {}", agentstacks::types::resolve::url_dir(Path::new("/sandbox/home"), &url).display());

    println!("-- R3 serde_json comment tolerance --");
    for text in ["{\"a\":1}", "# Generated\n{\"a\":1}", "// Generated\n{\"a\":1}", "<!-- Generated -->\n{\"a\":1}"] {
        let r: Result<serde_json::Value, _> = serde_json::from_str(text);
        println!("parse({text:?}) = {}", r.map(|_| "Ok".to_string()).unwrap_or_else(|e| format!("Err: {e}")));
    }

    println!("-- R4 semver VersionReq --");
    let req = semver::VersionReq::parse(">=0.1.5").expect("req");
    for v in ["0.1.4", "0.1.5", "0.1.6", "0.2.0", "1.0.0"] {
        let ver = semver::Version::parse(v).expect("v");
        println!("'>=0.1.5' matches {v} = {}", req.matches(&ver));
    }

    println!("-- R5 std env empty vs unset --");
    // var_os returns Some("") for set-but-empty, None for unset: probe via current process env manipulation is unsafe in edition2024; report API contract via var_os on a name we control through args is not possible here. Print PATH presence as control.
    println!("var_os(PATH).is_some() = {}", std::env::var_os("PATH").is_some());
    println!("var_os(DEFINITELY_UNSET_XYZ_123).is_some() = {}", std::env::var_os("DEFINITELY_UNSET_XYZ_123").is_some());
}
