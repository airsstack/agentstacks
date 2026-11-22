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
