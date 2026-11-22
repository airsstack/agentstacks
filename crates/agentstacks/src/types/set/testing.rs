//! Builders the unit tests of the set checks share.
//!
//! # Responsibilities
//!
//! - [`PackSpec`] describes a small pack and loads it through [`Pack::load`] over an in-memory
//!   tree, so no test builds a pack by hand. A pack directory shows as `packs/<name>`.
//! - [`extension`] loads a minimal extension the same way, and [`project`] parses a project file
//!   whose entries are `path` sources.
//!
//! # Non-responsibilities
//!
//! Compiled for tests only; it asserts nothing itself.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

use crate::port::FakeFileTree;
use crate::types::extension::Extension;
use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::pack::Pack;
use crate::types::project::ProjectFile;
use crate::types::report::Diagnostic;
use crate::types::report::Diagnostics;
use crate::types::set::check::check;
use crate::types::set::valid::ValidSet;

/// How the tests name the project file.
pub(super) const PROJECT_FILE: &str = "agentstacks.toml";

/// The refusals as the text the author reads.
pub(super) fn shown(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics.iter().map(ToString::to_string).collect()
}

/// The pack name `text`.
pub(super) fn name(text: &str) -> Result<PackName, Box<dyn Error>> {
    Ok(PackName::new(text)?)
}

/// The loaded packs of `specs`, each under its key.
pub(super) fn loaded(specs: &[PackSpec]) -> Result<BTreeMap<PackName, Pack>, Box<dyn Error>> {
    let mut packs = BTreeMap::new();
    for spec in specs {
        packs.insert(name(spec.key())?, spec.load()?);
    }
    Ok(packs)
}

/// A Markdown file whose frontmatter is lines 1 to 4 and whose line 6 is `tail`.
pub(super) fn doc(name: &str, tail: &str) -> String {
    format!("---\nname: {name}\ndescription: d\n---\n\n{tail}\n")
}

/// The project file listing `packs` (each a name with the raw lines of its `options` table) and
/// `tools`, every entry a `path` source.
pub(super) fn project(
    packs: &[(&str, &str)],
    tools: &[&str],
) -> Result<ProjectFile, Box<dyn Error>> {
    let mut parts = vec!["api = 0\n".to_owned()];
    for tool in tools {
        parts.push(format!("[tools.{tool}]\npath = \"extensions/{tool}\"\n"));
    }
    for (name, options) in packs {
        parts.push(format!("[packs.{name}]\npath = \"packs/{name}\"\n"));
        if !options.is_empty() {
            parts.push(format!("[packs.{name}.options]\n{options}\n"));
        }
    }
    ProjectFile::parse(Path::new(PROJECT_FILE), &parts.concat())
        .map_err(|found| shown(&found.into_sorted()).join("; ").into())
}

/// An extension named `name` that speaks `api`, loaded from an in-memory directory.
pub(super) fn extension_api(name: &str, api: i64) -> Result<Extension, Diagnostics> {
    let manifest = format!(
        "[extension]\nname = \"{name}\"\nversion = \"0.1.0\"\napi = {api}\ndescription = \"d\"\nentry = \"adapter.lua\"\n"
    );
    let tree = FakeFileTree::new(format!("extensions/{name}"))
        .file("extension.toml", manifest.as_bytes())
        .file("adapter.lua", b"return {}\n");
    Extension::load(&tree)
}

/// An extension named `name`, loaded from an in-memory directory.
pub(super) fn extension(name: &str) -> Result<Extension, Box<dyn Error>> {
    extension_api(name, 0).map_err(|found| shown(&found.into_sorted()).join("; ").into())
}

/// A pack under construction.
#[derive(Debug, Clone)]
pub(super) struct PackSpec {
    name: String,
    key: Option<String>,
    requires: Vec<String>,
    airsl: Option<String>,
    top: String,
    tables: String,
    skills: Vec<(String, String)>,
    agents: Vec<(String, String)>,
    files: Vec<(String, String)>,
}

impl PackSpec {
    /// A pack called `name` that ships one skill of its own unless something else is added.
    pub(super) fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            key: None,
            requires: Vec::new(),
            airsl: None,
            top: String::new(),
            tables: String::new(),
            skills: Vec::new(),
            agents: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Lists the pack under `key` in the project file, which differs from its own name.
    pub(super) fn listed_as(mut self, key: &str) -> Self {
        self.key = Some(key.to_owned());
        self
    }

    /// The key the pack is listed under.
    pub(super) fn key(&self) -> &str {
        self.key.as_deref().unwrap_or(&self.name)
    }

    /// Requires the packs `names`.
    pub(super) fn requires(mut self, names: &[&str]) -> Self {
        self.requires = names.iter().map(|name| (*name).to_owned()).collect();
        self
    }

    /// Declares `requires.airsl = range`.
    pub(super) fn airsl(mut self, range: &str) -> Self {
        self.airsl = Some(range.to_owned());
        self
    }

    /// Appends raw TOML keys that come before `[pack]`, such as `render` and `exclude`.
    pub(super) fn top(mut self, text: &str) -> Self {
        self.top.push_str(text);
        self
    }

    /// Appends raw TOML tables (options, hooks) after `[requires]`.
    pub(super) fn tables(mut self, text: &str) -> Self {
        self.tables.push_str(text);
        self
    }

    /// Declares an integer option `name` with the default `default`.
    pub(super) fn integer_option(self, name: &str, default: i64) -> Self {
        self.tables(&format!(
            "[options.{name}]\ntype = \"integer\"\ndefault = {default}\ndescription = \"d\"\n"
        ))
    }

    /// Declares a string option `name` with no default.
    pub(super) fn required_string_option(self, name: &str) -> Self {
        self.tables(&format!(
            "[options.{name}]\ntype = \"string\"\ndescription = \"d\"\n"
        ))
    }

    /// Ships the skill `name` whose line 6 is `tail`.
    pub(super) fn skill(mut self, name: &str, tail: &str) -> Self {
        self.skills.push((name.to_owned(), tail.to_owned()));
        self
    }

    /// Ships the agent `name` whose line 6 is `tail`.
    pub(super) fn agent(mut self, name: &str, tail: &str) -> Self {
        self.agents.push((name.to_owned(), tail.to_owned()));
        self
    }

    /// Adds the file `path` holding `text`.
    pub(super) fn file(mut self, path: &str, text: &str) -> Self {
        self.files.push((path.to_owned(), text.to_owned()));
        self
    }

    fn manifest_text(&self) -> String {
        let mut parts = vec![
            self.top.clone(),
            format!(
                "[pack]\nname = \"{}\"\nversion = \"0.1.0\"\napi = 0\ndescription = \"d\"\n",
                self.name
            ),
        ];
        if !self.requires.is_empty() || self.airsl.is_some() {
            parts.push("[requires]\n".to_owned());
            if !self.requires.is_empty() {
                let listed: Vec<String> = self
                    .requires
                    .iter()
                    .map(|name| format!("\"{name}\""))
                    .collect();
                parts.push(format!("packs = [{}]\n", listed.join(", ")));
            }
            if let Some(range) = &self.airsl {
                parts.push(format!("airsl = \"{range}\"\n"));
            }
        }
        parts.push(self.tables.clone());
        parts.concat()
    }

    /// The in-memory directory of the pack.
    fn tree(&self) -> FakeFileTree {
        let mut tree = FakeFileTree::new(format!("packs/{}", self.name))
            .file("pack.toml", self.manifest_text().as_bytes());
        let ships_something =
            !self.skills.is_empty() || !self.agents.is_empty() || self.tables.contains("[[hooks]]");
        if !ships_something {
            let own = format!("{}-main", self.name);
            tree = tree.file(
                format!("skills/{own}/SKILL.md"),
                doc(&own, "body").as_bytes(),
            );
        }
        for (name, tail) in &self.skills {
            tree = tree.file(
                format!("skills/{name}/SKILL.md"),
                doc(name, tail).as_bytes(),
            );
        }
        for (name, tail) in &self.agents {
            tree = tree.file(format!("agents/{name}.md"), doc(name, tail).as_bytes());
        }
        for (path, text) in &self.files {
            tree = tree.file(path, text.as_bytes());
        }
        tree
    }

    /// Loads the pack, handing back its refusals as `Pack::load` returns them.
    pub(super) fn load_raw(&self) -> Result<Pack, Diagnostics> {
        Pack::load(&self.tree())
    }

    /// Loads the pack; a refusal becomes the error text.
    pub(super) fn load(&self) -> Result<Pack, Box<dyn Error>> {
        self.load_raw()
            .map_err(|found| shown(&found.into_sorted()).join("; ").into())
    }
}

/// Runs the whole-set check over `packs` (each with the raw lines of its `options` table) and the
/// extensions named `tools`, all listed in the project file and all loaded. The refusals of a
/// failed check come back as the text the author reads.
pub(super) fn run_check(
    packs: &[(PackSpec, &str)],
    tools: &[&str],
) -> Result<Result<ValidSet, Vec<String>>, Box<dyn Error>> {
    let extensions = tools
        .iter()
        .map(|tool| (*tool, extension_api(tool, 0)))
        .collect();
    run_check_with(packs, extensions)
}

/// Runs the whole-set check like [`run_check`], with the load result of each extension given.
///
/// The loaders hand each listed key the result held under it. A key without one gets an empty
/// refusal set, which the check turns into a failed run.
pub(super) fn run_check_with(
    packs: &[(PackSpec, &str)],
    tools: Vec<(&str, Result<Extension, Diagnostics>)>,
) -> Result<Result<ValidSet, Vec<String>>, Box<dyn Error>> {
    let mut listed = Vec::new();
    let mut loaded = BTreeMap::new();
    for (spec, options) in packs {
        listed.push((spec.key(), *options));
        loaded.insert(name(spec.key())?, spec.load_raw());
    }
    let tool_names: Vec<&str> = tools.iter().map(|(tool, _)| *tool).collect();
    let mut extensions = BTreeMap::new();
    for (tool, result) in tools {
        extensions.insert(ToolName::new(tool)?, result);
    }
    let project = project(&listed, &tool_names)?;
    Ok(check(
        Path::new(PROJECT_FILE),
        project,
        |key, _entry| {
            loaded
                .remove(key)
                .unwrap_or_else(|| Err(Diagnostics::new()))
        },
        |key, _source| {
            extensions
                .remove(key)
                .unwrap_or_else(|| Err(Diagnostics::new()))
        },
    )
    .map_err(|refusals| shown(&refusals.into_sorted())))
}
