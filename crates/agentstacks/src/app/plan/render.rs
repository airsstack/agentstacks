//! Step 5 of the plan flow, the render half: filling a tool's token values into the texts of every
//! pack and building the payload the tool's extension plans over.
//!
//! # Responsibilities
//!
//! - [`render_packs`] renders every text the pack format lists: each rendered file as a whole, with
//!   its frontmatter; the `text`, `args`, `run` and grant paths of every hook in `pack.toml` and in
//!   this tool's `native/<tool>/hooks.toml`; and this tool's native files. Each text is scanned,
//!   each token is looked up in the tool's `values` answer (an option token in the project's
//!   options), and the values are spliced in.
//! - An item with no value for a token is skipped by the core and left out of the payload. A skill
//!   the answer gives no install location for is skipped first; otherwise the first token with no
//!   value, in text order, names the reason.
//! - It keeps the final bytes of every file an extension may `copy`, the rendered root files the
//!   core writes, and the core's skips.
//! - [`Rendered`] is the flow's carrier for all of that.
//!
//! # Non-responsibilities
//!
//! Deriving the `values` payload, choosing a skip reason and building the hook command line are
//! rules in `types`. This file only calls them in order. Running the extension and recording the
//! core's writes belong to the `tools` step.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::Path;

use crate::app::plan::load::Loaded;
use crate::port::FileTree;
use crate::types::hooks::HookAction;
use crate::types::name::{ItemName, PackName, ToolName};
use crate::types::op::{
    CoreSkip, ItemId, PlanAction, PlanAgent, PlanFile, PlanHook, PlanInput, PlanNative,
    PlanNativeHook, PlanPack, PlanSkill, TokenValue, ValuesResult, first_missing,
    skip_for_missing_value, skip_root_file_for_missing_value, skip_skill_without_location,
};
use crate::types::pack::{Pack, PackSkill};
use crate::types::project::{PROJECT_FILE, SourceError, SubPath};
use crate::types::protocol::{Blocking, HookCommand, HookKind, HookModule};
use crate::types::report::{Diagnostic, Diagnostics, Location};
use crate::types::resolve::SourceKey;
use crate::types::tokens::{
    LocatedToken, TextOrigin, Token, TokenTarget, TokenValues, render, scan,
};
use crate::types::tree::TreePath;
use crate::types::{AgentMeta, SkillMeta};

/// A root file of a pack, as the core writes it under the extension's root.
pub(super) struct RootFile {
    /// The pack the file belongs to.
    pub(super) pack: PackName,
    /// The file, relative to the pack.
    pub(super) rel: TreePath,
    /// The final bytes: the rendered text of a rendered file, else the raw bytes.
    pub(super) bytes: Vec<u8>,
}

/// What rendering the packs for one tool produced.
pub(super) struct Rendered {
    /// The payload the tool's extension plans over.
    pub(super) input: PlanInput,
    /// The final bytes of every file an extension may copy, by id.
    pub(super) files: BTreeMap<ItemId, Vec<u8>>,
    /// The root files of every pack, for the core to write.
    pub(super) root_files: Vec<RootFile>,
    /// The items the core skipped because the tool gave no value.
    pub(super) skips: Vec<CoreSkip>,
    /// The refusals met while rendering: a file that could not be read, or a rendered frontmatter
    /// that no longer parses.
    pub(super) refusals: Diagnostics,
}

/// Renders every pack of the set for `tool`, using its `values` answer.
pub(super) fn render_packs<T: FileTree>(
    loaded: &Loaded<T>,
    tool: &ToolName,
    values: &ValuesResult,
) -> Rendered {
    let mut out = Rendered {
        input: PlanInput {
            tool: tool.clone(),
            root: values.root().clone(),
            packs: Vec::new(),
        },
        files: BTreeMap::new(),
        root_files: Vec::new(),
        skips: Vec::new(),
        refusals: Diagnostics::new(),
    };
    let root = match SubPath::new(values.root().as_str()) {
        Ok(root) => root,
        Err(error) => {
            out.refusals.push(Diagnostic::new(
                Location::set_level(PROJECT_FILE, Some(format!("tools.{tool}"))),
                SourceError::from(error),
            ));
            return out;
        }
    };
    for (name, pack) in loaded.set.packs() {
        let Some(entry) = loaded.entries.get(&SourceKey::Pack(name.clone())) else {
            continue;
        };
        let run = PackRender {
            loaded,
            tool,
            values,
            root: &root,
            name,
            pack,
            tree: &entry.tree,
        };
        run.render(&mut out);
    }
    out
}

/// A file read from a pack tree.
struct Out {
    /// The final bytes.
    bytes: Vec<u8>,
    /// The text, for a file that is UTF-8.
    text: Option<String>,
}

/// What reading and rendering one file of a pack came to.
enum Read {
    /// The file, rendered when it is a rendered file.
    Done(Out),
    /// The first token the tool gave no value for.
    Missing(Token),
    /// The file could not be read; the refusal is recorded.
    Unread,
}

/// One pack being rendered for one tool.
struct PackRender<'a, T> {
    loaded: &'a Loaded<T>,
    tool: &'a ToolName,
    values: &'a ValuesResult,
    root: &'a SubPath,
    name: &'a PackName,
    pack: &'a Pack,
    tree: &'a T,
}

impl<T: FileTree> PackRender<'_, T> {
    fn render(&self, out: &mut Rendered) {
        let mut into = PlanPack {
            name: self.name.clone(),
            version: self.pack.manifest().version().clone(),
            skills: Vec::new(),
            agents: Vec::new(),
            hooks: Vec::new(),
            native: PlanNative::default(),
        };
        self.skills(out, &mut into);
        self.agents(out, &mut into);
        self.hooks(out, &mut into);
        self.native(out, &mut into);
        self.root_files(out);
        out.input.packs.push(into);
    }

    /// Fills the tokens of `text`. The error is the first token the tool gave no value for.
    fn fill(&self, text: &str, origin: &TextOrigin) -> Result<String, Token> {
        // The pack loader already refused every malformed token of the texts it scans, so the
        // refusals of this scan are not reported again.
        let (tokens, _) = scan(text, origin);
        if let Some(token) = first_missing(
            tokens.iter().map(LocatedToken::token),
            self.values,
            self.name,
        ) {
            return Err(token.clone());
        }
        let mut filled = TokenValues::new();
        for located in &tokens {
            let token = located.token();
            let value = match self.values.value(token.target(), self.name) {
                TokenValue::Given(value) => value,
                TokenValue::FromOption => match token.target() {
                    TokenTarget::Option(option) => {
                        match self.loaded.set.option(self.name, option) {
                            Some(value) => value.to_string(),
                            None => continue,
                        }
                    }
                    _ => continue,
                },
                TokenValue::Missing => continue,
            };
            filled.insert(token, value);
        }
        Ok(render(text, &filled))
    }

    fn origin(&self, rel: &str) -> TextOrigin {
        TextOrigin::Lines {
            file: self.pack.dir().join(rel),
            first_line: NonZeroUsize::MIN,
        }
    }

    /// Reads the file `rel` of the pack, rendering it when the pack renders it.
    fn read(&self, rel: &str, refusals: &mut Diagnostics) -> Read {
        let bytes = match self.tree.read_file(Path::new(rel)) {
            Ok(bytes) => bytes,
            Err(error) => {
                refusals.push(Diagnostic::new(
                    Location::file(self.tree.root().join(rel)),
                    error,
                ));
                return Read::Unread;
            }
        };
        let rendered = self.pack.rendered().iter().any(|path| path.as_str() == rel);
        match (String::from_utf8(bytes), rendered) {
            (Ok(text), true) => match self.fill(&text, &self.origin(rel)) {
                Ok(filled) => Read::Done(Out {
                    bytes: filled.clone().into_bytes(),
                    text: Some(filled),
                }),
                Err(token) => Read::Missing(token),
            },
            (Ok(text), false) => Read::Done(Out {
                bytes: text.clone().into_bytes(),
                text: Some(text),
            }),
            (Err(raw), _) => Read::Done(Out {
                bytes: raw.into_bytes(),
                text: None,
            }),
        }
    }

    fn skills(&self, out: &mut Rendered, into: &mut PlanPack) {
        for skill in self.pack.skills() {
            let Some(location) = self.values.skill(self.name, skill.name()) else {
                out.skips.push(skip_skill_without_location(
                    self.tool,
                    self.name,
                    skill.name(),
                ));
                continue;
            };
            if let Some(rendered) = self.skill(skill, &location.dir, out) {
                into.skills.push(rendered);
            }
        }
    }

    /// The skill rendered for the payload; `None` when it was skipped or refused.
    fn skill(
        &self,
        skill: &PackSkill,
        dir: &crate::types::op::OpPath,
        out: &mut Rendered,
    ) -> Option<PlanSkill> {
        let id = ItemId::skill(self.name, skill.name());
        let mut files = Vec::new();
        // Held back until every file of the skill rendered, so a skipped skill leaves no bytes.
        let mut bytes = Vec::new();
        let mut meta = None;
        for rel in skill.files() {
            let path = format!("skills/{}/{rel}", skill.name());
            let file = match self.read(&path, &mut out.refusals) {
                Read::Done(file) => file,
                Read::Missing(token) => {
                    out.skips
                        .push(skip_for_missing_value(self.tool, &id, &token));
                    return None;
                }
                Read::Unread => return None,
            };
            let file_id = ItemId::skill_file(self.name, skill.name(), rel);
            let mut text = file.text.clone();
            if rel.as_str() == "SKILL.md" {
                let parsed = SkillMeta::parse(
                    &self.pack.dir().join(&path),
                    file.text.as_deref().unwrap_or_default(),
                    skill.name(),
                );
                match parsed {
                    Ok(parsed) => {
                        text = file
                            .text
                            .as_deref()
                            .map(|whole| parsed.body_start().body(whole).to_owned());
                        meta = Some(parsed);
                    }
                    Err(found) => {
                        out.refusals.extend(found);
                        return None;
                    }
                }
            }
            bytes.push((file_id.clone(), file.bytes));
            files.push(PlanFile {
                id: file_id,
                rel: rel.clone(),
                text,
            });
        }
        let meta = meta?;
        out.files.extend(bytes);
        Some(PlanSkill {
            id,
            name: skill.name().clone(),
            dir: dir.clone(),
            meta,
            files,
        })
    }

    fn agents(&self, out: &mut Rendered, into: &mut PlanPack) {
        for agent in self.pack.agents() {
            let id = ItemId::agent(self.name, agent.name());
            let path = agent.file().as_str();
            let file = match self.read(path, &mut out.refusals) {
                Read::Done(file) => file,
                Read::Missing(token) => {
                    out.skips
                        .push(skip_for_missing_value(self.tool, &id, &token));
                    continue;
                }
                Read::Unread => continue,
            };
            let whole = file.text.unwrap_or_default();
            match AgentMeta::parse(&self.pack.dir().join(path), &whole, agent.name()) {
                Ok(meta) => {
                    let text = meta.body_start().body(&whole).to_owned();
                    into.agents.push(PlanAgent {
                        id,
                        name: agent.name().clone(),
                        meta,
                        text,
                    });
                }
                Err(found) => out.refusals.extend(found),
            }
        }
    }

    fn hooks(&self, out: &mut Rendered, into: &mut PlanPack) {
        let file = self.pack.dir().join("pack.toml");
        for (index, hook) in self.pack.manifest().hooks().iter().enumerate() {
            let id = ItemId::hook(self.name, index);
            let prefix = format!("hooks[{index}]");
            match self.action(hook.action(), &file, &prefix, HookKind::Neutral) {
                Ok(action) => into.hooks.push(PlanHook {
                    id,
                    event: hook.event(),
                    matches: hook.matches().map(<[_]>::to_vec),
                    agents: hook.agents().map(<[ItemName]>::to_vec),
                    action,
                }),
                Err(token) => out
                    .skips
                    .push(skip_for_missing_value(self.tool, &id, &token)),
            }
        }
    }

    fn native(&self, out: &mut Rendered, into: &mut PlanPack) {
        for dir in self
            .pack
            .native()
            .iter()
            .filter(|dir| dir.tool() == self.tool)
        {
            for rel in dir.files() {
                let id = ItemId::native_file(self.name, self.tool, rel);
                let path = format!("native/{}/{rel}", self.tool);
                match self.read(&path, &mut out.refusals) {
                    Read::Done(file) => {
                        out.files.insert(id.clone(), file.bytes);
                        into.native.files.push(PlanFile {
                            id,
                            rel: rel.clone(),
                            text: file.text,
                        });
                    }
                    Read::Missing(token) => out
                        .skips
                        .push(skip_for_missing_value(self.tool, &id, &token)),
                    Read::Unread => {}
                }
            }
            let Some(hooks) = dir.hooks() else {
                continue;
            };
            let file = self
                .pack
                .dir()
                .join(format!("native/{}/hooks.toml", self.tool));
            for (index, hook) in hooks.hooks().iter().enumerate() {
                let id = ItemId::native_hook(self.name, index);
                let prefix = format!("hooks[{index}]");
                match self.action(hook.action(), &file, &prefix, HookKind::Native) {
                    Ok(action) => into.native.hooks.push(PlanNativeHook {
                        id,
                        event: hook.event().to_owned(),
                        matches: hook.matches().map(<[String]>::to_vec),
                        action,
                    }),
                    Err(token) => out
                        .skips
                        .push(skip_for_missing_value(self.tool, &id, &token)),
                }
            }
        }
    }

    fn root_files(&self, out: &mut Rendered) {
        for rel in self.pack.root_files() {
            match self.read(rel.as_str(), &mut out.refusals) {
                Read::Done(file) => out.root_files.push(RootFile {
                    pack: self.name.clone(),
                    rel: rel.clone(),
                    bytes: file.bytes,
                }),
                Read::Missing(token) => out.skips.push(skip_root_file_for_missing_value(
                    self.tool, self.name, rel, &token,
                )),
                Read::Unread => {}
            }
        }
    }

    /// What a hook does, with every text field of it rendered. The error is the first token the
    /// tool gave no value for, in field order: `run`, `args`, then the grant paths of a Lua hook,
    /// or the `text` of a message.
    fn action(
        &self,
        action: &HookAction,
        file: &Path,
        prefix: &str,
        kind: HookKind,
    ) -> Result<PlanAction, Token> {
        let mut fields: Vec<(String, &str)> = Vec::new();
        match action {
            HookAction::Message(message) => fields.push(("text".to_owned(), message.text())),
            HookAction::Lua(lua) => {
                fields.push(("run".to_owned(), lua.run().written()));
                for (index, arg) in lua.args().iter().enumerate() {
                    fields.push((format!("args[{index}]"), arg));
                }
                for (index, path) in lua.grants().read_paths().iter().enumerate() {
                    fields.push((format!("grants.read[{index}]"), path));
                }
                for (index, path) in lua.grants().write_paths().iter().enumerate() {
                    fields.push((format!("grants.write[{index}]"), path));
                }
            }
        }
        let mut rendered = Vec::new();
        for (suffix, text) in &fields {
            let origin = TextOrigin::Field {
                file: file.to_path_buf(),
                key_path: format!("{prefix}.{suffix}"),
            };
            rendered.push(self.fill(text, &origin)?);
        }
        let mut parts = rendered.into_iter();
        Ok(match action {
            HookAction::Message(_) => PlanAction::Message {
                text: parts.next().unwrap_or_default(),
            },
            HookAction::Lua(lua) => {
                let _run = parts.next();
                let args: Vec<String> = parts.by_ref().take(lua.args().len()).collect();
                let read: Vec<String> = parts
                    .by_ref()
                    .take(lua.grants().read_paths().len())
                    .collect();
                let write: Vec<String> = parts.collect();
                let command = HookCommand::new(
                    self.root.clone(),
                    HookModule::new(self.name, lua.run()),
                    Blocking::from(lua.may_block()),
                    kind,
                )
                .with_args(args)
                .with_read(read)
                .with_write(write)
                .with_env(lua.grants().env().to_vec())
                .with_exec(lua.grants().exec().to_vec())
                .argv();
                PlanAction::Lua {
                    may_block: lua.may_block(),
                    command,
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::plan::testing::{
        LoadedTrees, PROJECT, TestResult, loaded, pack, shown, single, sub, tool, values_for,
    };
    use crate::port::FakeFileTree;
    use crate::types::op::SkipSubject;

    const VALUES: &str = r#"{"root":".claude/agentstacks","skills":{"core/status":{"invoke":"/status","dir":".claude/skills/status"}},"agents":{"core/coder":"coder"},"vars":{"tmpdir":"$TMPDIR"}}"#;
    const ONLY_STATUS: &str = r#"{"root":".claude/agentstacks","skills":{"core/status":{"invoke":"/status","dir":".claude/skills/status"}}}"#;
    const OPTIONS_PROJECT: &str = "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\npath = \"packs/core\"\n\n[packs.core.options]\narchive_keep = 50\n";

    fn manifest(extra: &str) -> String {
        format!(
            "[pack]\nname = \"core\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\n{extra}"
        )
    }

    fn markdown(name: &str, body: &str) -> String {
        format!("---\nname: {name}\ndescription: d\n---\n{body}\n")
    }

    fn tree(manifest_text: &str, files: &[(&str, &str)]) -> FakeFileTree {
        files.iter().fold(
            FakeFileTree::new("packs/core").file("pack.toml", manifest_text.as_bytes()),
            |tree, (rel, text)| tree.file(rel, text.as_bytes()),
        )
    }

    fn load(
        project: &str,
        pack_tree: FakeFileTree,
    ) -> Result<LoadedTrees, Box<dyn std::error::Error>> {
        loaded(&single(project, pack_tree)?).map_err(|found| shown(found).join("; ").into())
    }

    fn render(loaded: &LoadedTrees, json: &str) -> Result<Rendered, Box<dyn std::error::Error>> {
        let values = values_for(&loaded.set, "claude", json)?;
        Ok(render_packs(loaded, &tool("claude")?, &values))
    }

    fn reasons(rendered: &Rendered) -> Vec<String> {
        rendered
            .skips
            .iter()
            .map(|skip| skip.reason().to_string())
            .collect()
    }

    fn subjects(rendered: &Rendered) -> Vec<String> {
        rendered
            .skips
            .iter()
            .map(|skip| match skip.subject() {
                SkipSubject::Item(id) => id.to_string(),
                SkipSubject::RootFile { pack, rel } => format!("{pack}:{rel}"),
            })
            .collect()
    }

    fn core(rendered: &Rendered) -> Result<&PlanPack, Box<dyn std::error::Error>> {
        rendered.input.packs.first().ok_or_else(|| "a pack".into())
    }

    #[test]
    fn render_packs_fills_every_token_kind_from_the_values() -> TestResult {
        let body = "root={{root}} rootcore={{root core}} skill={{skill core/status}} dir={{skill_dir core/status}} agent={{agent core/coder}} tmp={{var tmpdir}} keep={{option archive_keep}}";
        let pack_tree = tree(
            &manifest(
                "[options.archive_keep]\ntype = \"integer\"\ndefault = 20\ndescription = \"d\"\n",
            ),
            &[
                ("skills/status/SKILL.md", &markdown("status", body)),
                ("agents/coder.md", &markdown("coder", "b")),
            ],
        );
        let rendered = render(&load(OPTIONS_PROJECT, pack_tree)?, VALUES)?;
        assert!(rendered.refusals.is_empty());
        let skill = core(&rendered)?.skills.first().ok_or("a skill")?;
        let file = skill.files.first().ok_or("SKILL.md")?;
        assert_eq!(
            file.text.as_deref(),
            Some(
                "root=.claude/agentstacks/core rootcore=.claude/agentstacks/core skill=/status dir=.claude/skills/status agent=coder tmp=$TMPDIR keep=50\n"
            )
        );
        Ok(())
    }

    #[test]
    fn render_packs_skips_a_skill_with_a_missing_value() -> TestResult {
        let pack_tree = tree(
            &manifest(""),
            &[
                (
                    "skills/status/SKILL.md",
                    &markdown("status", "{{agent core/coder}}"),
                ),
                ("agents/coder.md", &markdown("coder", "b")),
            ],
        );
        let rendered = render(&load(PROJECT, pack_tree)?, ONLY_STATUS)?;
        assert_eq!(
            reasons(&rendered),
            ["claude gave no value for {{agent core/coder}}"]
        );
        assert_eq!(subjects(&rendered), ["core/skill/status"]);
        assert!(core(&rendered)?.skills.is_empty());
        assert_eq!(core(&rendered)?.agents.len(), 1);
        Ok(())
    }

    #[test]
    fn render_packs_names_the_first_missing_token_in_text_order() -> TestResult {
        let pack_tree = tree(
            &manifest(""),
            &[
                ("skills/status/SKILL.md", &markdown("status", "ok")),
                (
                    "agents/coder.md",
                    &markdown("coder", "{{var home}} then {{agent core/coder}}"),
                ),
            ],
        );
        let rendered = render(&load(PROJECT, pack_tree)?, ONLY_STATUS)?;
        assert_eq!(
            reasons(&rendered),
            ["claude gave no value for {{var home}}"]
        );
        assert_eq!(subjects(&rendered), ["core/agent/coder"]);
        Ok(())
    }

    #[test]
    fn render_packs_prefers_a_missing_skill_entry_over_a_missing_token() -> TestResult {
        let pack_tree = tree(
            &manifest(""),
            &[(
                "skills/status/SKILL.md",
                &markdown("status", "{{var tmpdir}}"),
            )],
        );
        let rendered = render(
            &load(PROJECT, pack_tree)?,
            r#"{"root":".claude/agentstacks"}"#,
        )?;
        assert_eq!(
            reasons(&rendered),
            ["claude gave no install location for skill core/status"]
        );
        Ok(())
    }

    #[test]
    fn render_packs_skips_only_the_agent_hook_or_native_item_missing_a_value() -> TestResult {
        let manifest_text = manifest(
            "[requires]\nairsl = \">=0.1.4\"\n[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"hooks/x.lua\"\nargs = [\"{{var home}}\"]\n[[hooks]]\nkind = \"message\"\nevent = \"session.end\"\ntext = \"ok\"\n",
        );
        let native_hooks = "[[hooks]]\nkind = \"message\"\nevent = \"Stop\"\ntext = \"{{var home}}\"\n[[hooks]]\nkind = \"message\"\nevent = \"Stop\"\ntext = \"fine\"\n";
        let pack_tree = tree(
            &manifest_text,
            &[
                ("skills/status/SKILL.md", &markdown("status", "ok")),
                ("agents/coder.md", &markdown("coder", "{{var home}}")),
                ("agents/other.md", &markdown("other", "ok")),
                ("hooks/x.lua", "return 1"),
                ("native/claude/a.md", "{{var home}}"),
                ("native/claude/b.md", "ok"),
                ("native/claude/hooks.toml", native_hooks),
            ],
        );
        let rendered = render(&load(PROJECT, pack_tree)?, ONLY_STATUS)?;
        assert_eq!(
            subjects(&rendered),
            [
                "core/agent/coder",
                "core/hook/0",
                "core/native/claude/a.md",
                "core/native-hook/0"
            ]
        );
        let pack = core(&rendered)?;
        assert_eq!(pack.skills.len(), 1);
        assert_eq!(
            pack.agents
                .iter()
                .map(|a| a.id.to_string())
                .collect::<Vec<_>>(),
            ["core/agent/other"]
        );
        assert_eq!(
            pack.hooks
                .iter()
                .map(|h| h.id.to_string())
                .collect::<Vec<_>>(),
            ["core/hook/1"]
        );
        assert_eq!(
            pack.native
                .files
                .iter()
                .map(|f| f.id.to_string())
                .collect::<Vec<_>>(),
            ["core/native/claude/b.md"]
        );
        assert_eq!(
            pack.native
                .hooks
                .iter()
                .map(|h| h.id.to_string())
                .collect::<Vec<_>>(),
            ["core/native-hook/1"]
        );
        Ok(())
    }

    #[test]
    fn render_packs_keeps_no_bytes_of_a_skill_skipped_partway() -> TestResult {
        let manifest_text = format!("render = [\"skills/status/extra.md\"]\n{}", manifest(""));
        let pack_tree = tree(
            &manifest_text,
            &[
                ("skills/status/SKILL.md", &markdown("status", "ok")),
                ("skills/status/extra.md", "{{var home}}"),
            ],
        );
        let rendered = render(&load(PROJECT, pack_tree)?, ONLY_STATUS)?;
        assert_eq!(subjects(&rendered), ["core/skill/status"]);
        assert!(rendered.files.is_empty());
        Ok(())
    }

    #[test]
    fn render_packs_makes_a_core_skip_for_a_root_file_missing_a_value() -> TestResult {
        let manifest_text = format!("render = [\"data.txt\"]\n{}", manifest(""));
        let pack_tree = tree(
            &manifest_text,
            &[
                ("skills/status/SKILL.md", &markdown("status", "ok")),
                ("data.txt", "{{var home}}"),
            ],
        );
        let rendered = render(&load(PROJECT, pack_tree)?, ONLY_STATUS)?;
        assert_eq!(subjects(&rendered), ["core:data.txt"]);
        assert_eq!(
            reasons(&rendered),
            ["claude gave no value for {{var home}}"]
        );
        assert!(rendered.root_files.is_empty());
        Ok(())
    }

    #[test]
    fn render_packs_keeps_raw_bytes_for_unrendered_files() -> TestResult {
        let logo: &[u8] = &[0x89, 0xFF, 0x00, 0x7B, 0x7B];
        let pack_tree = tree(
            &manifest(""),
            &[("skills/status/SKILL.md", &markdown("status", "Body"))],
        )
        .file("skills/status/logo.png", logo);
        let rendered = render(&load(PROJECT, pack_tree)?, ONLY_STATUS)?;
        let skill = core(&rendered)?.skills.first().ok_or("a skill")?;
        let png = skill
            .files
            .iter()
            .find(|file| file.rel.as_str() == "logo.png")
            .ok_or("logo.png")?;
        assert_eq!(png.text, None);
        assert_eq!(rendered.files.get(&png.id).map(Vec::as_slice), Some(logo));
        let skill_md = skill
            .files
            .iter()
            .find(|file| file.rel.as_str() == "SKILL.md")
            .ok_or("SKILL.md")?;
        assert_eq!(skill_md.text.as_deref(), Some("Body\n"));
        assert_eq!(
            rendered.files.get(&skill_md.id).map(Vec::as_slice),
            Some(markdown("status", "Body").as_bytes())
        );
        Ok(())
    }

    #[test]
    fn render_packs_builds_the_hook_command_argv() -> TestResult {
        let manifest_text = manifest(
            "[requires]\nairsl = \">=0.1.4\"\n[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"hooks/x.lua\"\nargs = [\"a\"]\n[hooks.grants]\nread = [\"{{root}}/data\"]\nenv = [\"HOME\"]\n",
        );
        let pack_tree = tree(
            &manifest_text,
            &[
                ("skills/status/SKILL.md", &markdown("status", "ok")),
                ("hooks/x.lua", "return 1"),
            ],
        );
        let loaded = load(PROJECT, pack_tree)?;
        let rendered = render(&loaded, ONLY_STATUS)?;
        let source = loaded.set.pack(&pack("core")?).ok_or("the pack")?;
        let hook = source.manifest().hooks().first().ok_or("a hook")?;
        let HookAction::Lua(lua) = hook.action() else {
            return Err("a lua hook".into());
        };
        let expected = HookCommand::new(
            sub(".claude/agentstacks")?,
            HookModule::new(&pack("core")?, lua.run()),
            Blocking::MayNot,
            HookKind::Neutral,
        )
        .with_args(vec!["a".to_owned()])
        .with_read(vec![".claude/agentstacks/core/data".to_owned()])
        .with_env(lua.grants().env().to_vec())
        .argv();
        let planned = core(&rendered)?.hooks.first().ok_or("a hook")?;
        assert_eq!(
            planned.action,
            PlanAction::Lua {
                may_block: false,
                command: expected
            }
        );
        Ok(())
    }
}
