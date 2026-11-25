use clap::{Parser, Subcommand};
use semver::{Version, VersionReq};
use toml_edit::{DocumentMut, Item, Table, value};

fn p1_semver() {
    println!("## P1 semver 1.0.28: VersionReq::matches");
    for req in [">=1.3", ">= 1.0", "^1", "*", ">=1.0, <2", "nope", "1.x"] {
        match VersionReq::parse(req) {
            Err(e) => println!("{req:?} -> parse error: {e}"),
            Ok(r) => {
                for v in ["1.0.0", "1.3.0", "2.0.0", "1.1.0-rc.1"] {
                    println!("{req:?} matches {v}: {}", r.matches(&Version::parse(v).unwrap()));
                }
            }
        }
    }
}

fn add_tool(text: &str) -> String {
    let mut doc: DocumentMut = text.parse().unwrap();
    let tools = doc.get_mut("tools").unwrap();
    let mut entry = Table::new();
    entry.insert("git", value("https://example.com/x.git"));
    entry.insert("tag", value("v1.2.0"));
    entry.insert("subdir", value("ext/mytool"));
    match tools {
        Item::Table(t) => {
            // place right after the last tools.* table
            let last = t.iter().filter_map(|(_, i)| i.as_table().and_then(|t| t.position())).max();
            if let Some(p) = last { entry.set_position(Some(p)); }
            t.insert("mytool", Item::Table(entry));
        }
        Item::Value(v) => {
            let it = v.as_inline_table_mut().unwrap();
            it.insert("mytool", toml_edit::Value::InlineTable(entry.into_inline_table()));
        }
        _ => unreachable!(),
    }
    doc.to_string()
}

fn p2_toml_edit_add() {
    println!("## P2 toml_edit 0.25.20: insert [tools.mytool]");
    let standard = "# project\napi = 1\n\n# the tool\n[tools.claude]\ngit    = \"https://github.com/airsstack/agentstacks\"\ntag    = \"packs-v1.0.0\"\nsubdir = \"extensions/claude\"\n\n[packs.core] # core pack\ngit    = \"https://github.com/airsstack/agentstacks\"\ntag    = \"packs-v1.0.0\"\nsubdir = \"packs/core\"\n";
    println!("### standard tables\n{}", add_tool(standard));
    let inline = "api = 1\ntools = { claude = { path = \"ext/claude\" } }\n\n[packs.core]\npath = \"packs/core\"\n";
    println!("### inline tools\n{}", add_tool(inline));
    let dotted = "api = 1\ntools.claude.path = \"ext/claude\"\n\n[packs.core]\npath = \"packs/core\"\n";
    println!("### dotted tools\n{}", add_tool(dotted));
}

fn p3_toml_edit_upgrade() {
    println!("## P3 toml_edit 0.25.20: api 0 -> 1 keeping decor");
    let text = "# my project\napi = 0 # pinned by hand\n\n[tools.claude]\npath = \"ext/claude\"\n";
    let mut doc: DocumentMut = text.parse().unwrap();
    let decor = doc["api"].as_value().unwrap().decor().clone();
    let mut v = toml_edit::Value::from(1i64);
    *v.decor_mut() = decor;
    doc["api"] = Item::Value(v);
    print!("{doc}");
}

fn p4_toml_examples() {
    println!("## P4 toml 1.2.0: spec examples parse");
    let pack = "exclude = [\"**/*_test.lua\"]\n\n[pack]\nname        = \"team-lint\"\nversion     = \"1.0.0\"\napi         = 1\ndescription = \"Team lint rules\"\n\n[requires]\nagentstacks = \">=1.0\"\nairsl       = \">=0.1.5\"\n";
    let ext = "[extension]\nname        = \"mytool\"\nversion     = \"1.0.0\"\napi         = 1\ndescription = \"Renders packs into mytool project config\"\nentry       = \"adapter.lua\"\n\n[requires]\nagentstacks = \">=1.0\"\n";
    for (name, text) in [("pack.toml", pack), ("extension.toml", ext)] {
        let v: toml::Table = toml::from_str(text).unwrap();
        println!("{name}: requires.agentstacks = {:?}; api = {:?}", v["requires"]["agentstacks"].as_str(), v.get("pack").or(v.get("extension")).unwrap()["api"].as_integer());
    }
}

#[derive(Debug, Parser)]
#[command(name = "agentstacks")]
struct Cli {
    #[arg(short = 'C', global = true, value_name = "DIR")]
    dir: Option<std::path::PathBuf>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Debug, Subcommand)]
enum Command {
    Plan,
    /// Add or check a tool extension.
    #[command(subcommand)]
    Ext(Ext),
}
#[derive(Debug, Subcommand)]
enum Ext {
    Add {
        source: String,
        #[arg(long, conflicts_with = "rev")]
        tag: Option<String>,
        #[arg(long)]
        rev: Option<String>,
        #[arg(long)]
        subdir: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    Test {
        extension: std::path::PathBuf,
        #[arg(long = "pack", value_name = "DIR")]
        pack: Vec<std::path::PathBuf>,
    },
}

fn p6_clap() {
    println!("## P6 clap 4.6.9: nested ext subcommands");
    for argv in [
        vec!["agentstacks", "ext", "add", "https://example.com/x.git", "--tag", "v1.2.0", "--subdir", "ext/mytool"],
        vec!["agentstacks", "-C", "proj", "ext", "add", "./ext/mytool", "--yes"],
        vec!["agentstacks", "ext", "add", "x", "--tag", "a", "--rev", "b"],
        vec!["agentstacks", "ext", "test", "ext/mytool", "--pack", "packs/a", "--pack", "packs/b", "-C", "proj"],
        vec!["agentstacks", "ext"],
        vec!["agentstacks", "ext", "test"],
    ] {
        match Cli::try_parse_from(&argv) {
            Ok(c) => println!("{:?} -> {:?}", argv[1..].join(" "), c),
            Err(e) => println!("{:?} -> exit {} : {}", argv[1..].join(" "), e.exit_code(), e.to_string().lines().next().unwrap_or("")),
        }
    }
}

fn p2b_reparse() {
    println!("## P2b toml 1.2.0: reparse the P2 outputs");
    let standard = "# project\napi = 1\n\n# the tool\n[tools.claude]\ngit    = \"https://github.com/airsstack/agentstacks\"\ntag    = \"packs-v1.0.0\"\nsubdir = \"extensions/claude\"\n\n[packs.core] # core pack\ngit    = \"https://github.com/airsstack/agentstacks\"\ntag    = \"packs-v1.0.0\"\nsubdir = \"packs/core\"\n";
    let inline = "api = 1\ntools = { claude = { path = \"ext/claude\" } }\n\n[packs.core]\npath = \"packs/core\"\n";
    let dotted = "api = 1\ntools.claude.path = \"ext/claude\"\n\n[packs.core]\npath = \"packs/core\"\n";
    for (n, t) in [("standard", standard), ("inline", inline), ("dotted", dotted)] {
        let out = add_tool(t);
        match toml::from_str::<toml::Table>(&out) {
            Ok(v) => println!("{n}: ok, tools keys = {:?}", v["tools"].as_table().unwrap().keys().collect::<Vec<_>>()),
            Err(e) => println!("{n}: PARSE ERROR {e}"),
        }
    }
}

fn p6b_clap() {
    println!("## P6b clap 4.6.9: -C placement with nested subcommands");
    for argv in [
        vec!["agentstacks", "ext", "test", "x", "-C", "proj"],
        vec!["agentstacks", "ext", "-C", "proj", "test", "x"],
        vec!["agentstacks", "ext", "add", "x", "-C", "proj"],
        vec!["agentstacks", "plan", "-C", "proj"],
        vec!["agentstacks", "ext", "test", "x", "--pack", "a", "-C", "proj"],
        vec!["agentstacks", "ext", "test", "x", "--pack=a", "-C", "proj"],
    ] {
        match Cli::try_parse_from(&argv) {
            Ok(c) => println!("{:?} -> {:?}", argv[1..].join(" "), c),
            Err(e) => println!("{:?} -> exit {} : {}", argv[1..].join(" "), e.exit_code(), e.to_string().lines().next().unwrap_or("")),
        }
    }
}

fn main() {
    p1_semver();
    p2_toml_edit_add();
    p2b_reparse();
    p3_toml_edit_upgrade();
    p4_toml_examples();
    p6_clap();
    p6b_clap();
}
