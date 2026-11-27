use clap::Parser;
#[derive(Debug, Parser)]
#[command(name = "agentstacks", version, about = "probe")]
struct Cli { #[command(subcommand)] command: Command }
#[derive(Debug, clap::Subcommand)]
enum Command { Init { #[arg(long)] dry_run: bool, #[arg(long, conflicts_with = "dry_run")] yes: bool } }
fn main() {
    for args in [&["agentstacks","init","--dry-run","--yes"][..], &["agentstacks","init","--dry-run"][..]] {
        println!("=== {args:?}");
        match Cli::try_parse_from(args) { Ok(c) => println!("Ok({c:?})"), Err(e) => println!("kind={:?} exit_code={} rendered={:?}", e.kind(), e.exit_code(), e.to_string()) }
    }
    println!("-- R6 frontmatter first-line check --");
    let skill = "---\nname: a\ndescription: d\n---\nbody\n";
    let stamped_prepend = format!("<!-- generated -->\n{skill}");
    let after = "---\nname: a\ndescription: d\n---\n<!-- generated -->\nbody\n";
    println!("original starts_with --- = {}", skill.starts_with("---\n"));
    println!("prepend-stamped starts_with --- = {}", stamped_prepend.starts_with("---\n"));
    println!("after-frontmatter starts_with --- = {}", after.starts_with("---\n"));
}
