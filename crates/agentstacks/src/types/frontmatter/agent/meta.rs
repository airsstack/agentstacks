//! The metadata an agent's frontmatter declares.
//!
//! Agents and skills have different keys and different vocabularies, so each has its own type
//! rather than one struct with optional fields. The checks they share live in
//! `FrontmatterFields`.
//!
//! # Responsibilities
//!
//! - [`AgentMeta::parse`] splits an `agents/<name>.md`, parses its frontmatter and checks every
//!   key. A missing block, a syntax error or a block that is not a mapping is the file's only
//!   diagnostic; otherwise every field error of the file is reported together.
//! - The keys are `name`, `description`, `model`, `effort`, `tools` and `native`. The free-text
//!   value `description` is scanned for tokens once; the tokens are kept.
//! - The returned [`AgentMeta`] exposes where the body begins, so the body is scanned once by the
//!   caller.
//!
//! # Non-responsibilities
//!
//! Where a token may appear and what it may point at is checked over all tokens of a pack and of a
//! set of packs, not here. Values under `native` are opaque.

use std::collections::BTreeMap;
use std::path::Path;

use crate::types::format::YamlValue;
use crate::types::frontmatter::agent::tool::AgentTool;
use crate::types::frontmatter::block::Frontmatter;
use crate::types::frontmatter::body::BodyStart;
use crate::types::frontmatter::effort::Effort;
use crate::types::frontmatter::error::FrontmatterError;
use crate::types::frontmatter::fields::FrontmatterFields;
use crate::types::frontmatter::model::ModelTier;
use crate::types::frontmatter::wording::StringWording;
use crate::types::name::ItemName;
use crate::types::name::ToolName;
use crate::types::report::Diagnostics;
use crate::types::tokens::LocatedToken;

/// The keys an agent's frontmatter may hold.
const KEYS: [&str; 6] = ["name", "description", "model", "effort", "tools", "native"];

/// An agent's checked frontmatter.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{AgentMeta, AgentTool, Effort, ItemName, ModelTier};
///
/// let text = "---\nname: reviewer\ndescription: Reviews code.\nmodel: deep\neffort: high\ntools: [read, grep]\n---\nBody\n";
/// let name = ItemName::new("reviewer")?;
/// let meta = AgentMeta::parse(Path::new("agents/reviewer.md"), text, &name)
///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
/// assert_eq!(meta.model(), Some(ModelTier::Deep));
/// assert_eq!(meta.effort(), Some(Effort::High));
/// assert_eq!(meta.tools(), Some(&[AgentTool::Read, AgentTool::Grep][..]));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct AgentMeta {
    name: ItemName,
    description: String,
    model: Option<ModelTier>,
    effort: Option<Effort>,
    tools: Option<Vec<AgentTool>>,
    native: BTreeMap<ToolName, YamlValue>,
    tokens: Vec<LocatedToken>,
    body_start: BodyStart,
}

impl AgentMeta {
    /// Parses the frontmatter of `text`, the content of the agent file at `file`. `expected_name`
    /// is the file's stem.
    ///
    /// # Errors
    ///
    /// Returns every diagnostic found. A missing frontmatter block, a YAML syntax error or a block
    /// that is not a mapping gives that one diagnostic; otherwise each refused key or value gives
    /// one.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{AgentMeta, ItemName};
    ///
    /// let file = Path::new("agents/reviewer.md");
    /// let name = ItemName::new("reviewer")?;
    ///
    /// let good = "---\nname: reviewer\ndescription: Reviews code.\n---\n";
    /// assert!(AgentMeta::parse(file, good, &name).is_ok());
    ///
    /// let repeated = "---\nname: reviewer\ndescription: d\ntools: [read, read]\n---\n";
    /// let refusals = AgentMeta::parse(file, repeated, &name).err().unwrap_or_default();
    /// let shown: Vec<String> = refusals.into_sorted().iter().map(ToString::to_string).collect();
    /// assert_eq!(shown, ["agents/reviewer.md: tools[1]: \"read\" listed twice"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn parse(file: &Path, text: &str, expected_name: &ItemName) -> Result<Self, Diagnostics> {
        let split = Frontmatter::split(file, text).map_err(Diagnostics::from)?;
        let entries = split.mapping().map_err(Diagnostics::from)?;
        let mut fields = FrontmatterFields::new(file, entries);

        let name = fields.name(expected_name);
        let description = fields.required_string("description");
        if let Some(description) = &description {
            fields.scan_text("description", description);
        }
        let model = Self::vocabulary::<ModelTier>(&mut fields, "model");
        let effort = Self::vocabulary::<Effort>(&mut fields, "effort");
        let tools = Self::read_tools(&mut fields);
        let native = fields.native();
        let (diagnostics, tokens) = fields.finish(&KEYS);

        let (Some(name), Some(description)) = (name, description) else {
            debug_assert!(
                !diagnostics.is_empty(),
                "a missing name or description is always refused"
            );
            return Err(diagnostics);
        };
        diagnostics.into_result(Self {
            name,
            description,
            model,
            effort,
            tools,
            native,
            tokens,
            body_start: split.body_start(),
        })
    }

    /// An optional string field read through `FromStr`, refused at its own key.
    fn vocabulary<T>(fields: &mut FrontmatterFields, key: &str) -> Option<T>
    where
        T: std::str::FromStr<Err = FrontmatterError>,
    {
        let value = fields.optional_string(key, StringWording::Generic)?;
        match value.parse::<T>() {
            Ok(parsed) => Some(parsed),
            Err(error) => {
                fields.refuse_at(key, error);
                None
            }
        }
    }

    /// The optional `tools` array: each item an [`AgentTool`], none listed twice. An item that is
    /// refused is located at `tools[<index>]`.
    fn read_tools(fields: &mut FrontmatterFields) -> Option<Vec<AgentTool>> {
        let items = fields.optional_array("tools")?;
        let mut tools: Vec<AgentTool> = Vec::new();
        for (index, item) in items {
            let key_path = format!("tools[{index}]");
            match item.parse::<AgentTool>() {
                Ok(tool) if tools.contains(&tool) => {
                    fields.refuse_at(&key_path, FrontmatterError::DuplicateTool { value: item });
                }
                Ok(tool) => tools.push(tool),
                Err(error) => fields.refuse_at(&key_path, error),
            }
        }
        Some(tools)
    }

    /// The agent's name, equal to its file stem.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{AgentMeta, ItemName};
    ///
    /// let text = "---\nname: reviewer\ndescription: d\n---\n";
    /// let name = ItemName::new("reviewer")?;
    /// let meta = AgentMeta::parse(Path::new("agents/reviewer.md"), text, &name)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// assert_eq!(meta.name(), &name);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn name(&self) -> &ItemName {
        &self.name
    }

    /// The description.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{AgentMeta, ItemName};
    ///
    /// let text = "---\nname: reviewer\ndescription: Reviews code.\n---\n";
    /// let meta = AgentMeta::parse(Path::new("agents/reviewer.md"), text, &ItemName::new("reviewer")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// assert_eq!(meta.description(), "Reviews code.");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// The model tier, when the frontmatter names one.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{AgentMeta, ItemName, ModelTier};
    ///
    /// let text = "---\nname: reviewer\ndescription: d\nmodel: fast\n---\n";
    /// let meta = AgentMeta::parse(Path::new("agents/reviewer.md"), text, &ItemName::new("reviewer")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// assert_eq!(meta.model(), Some(ModelTier::Fast));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn model(&self) -> Option<ModelTier> {
        self.model
    }

    /// The reasoning effort, when the frontmatter names one.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{AgentMeta, Effort, ItemName};
    ///
    /// let text = "---\nname: reviewer\ndescription: d\neffort: low\n---\n";
    /// let meta = AgentMeta::parse(Path::new("agents/reviewer.md"), text, &ItemName::new("reviewer")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// assert_eq!(meta.effort(), Some(Effort::Low));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn effort(&self) -> Option<Effort> {
        self.effort
    }

    /// The tools, in the order listed, when the frontmatter lists them.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{AgentMeta, AgentTool, ItemName};
    ///
    /// let text = "---\nname: reviewer\ndescription: d\ntools: [shell, read]\n---\n";
    /// let meta = AgentMeta::parse(Path::new("agents/reviewer.md"), text, &ItemName::new("reviewer")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// assert_eq!(meta.tools(), Some(&[AgentTool::Shell, AgentTool::Read][..]));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn tools(&self) -> Option<&[AgentTool]> {
        self.tools.as_deref()
    }

    /// The tool-specific values, keyed by tool.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{AgentMeta, ItemName};
    ///
    /// let text = "---\nname: reviewer\ndescription: d\nnative:\n  claude:\n    color: blue\n---\n";
    /// let meta = AgentMeta::parse(Path::new("agents/reviewer.md"), text, &ItemName::new("reviewer")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// assert_eq!(meta.native().len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn native(&self) -> &BTreeMap<ToolName, YamlValue> {
        &self.native
    }

    /// The tokens of the frontmatter values, in the order they were scanned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{AgentMeta, ItemName};
    ///
    /// let text = "---\nname: reviewer\ndescription: Uses \"{{skill review}}\".\n---\n";
    /// let meta = AgentMeta::parse(Path::new("agents/reviewer.md"), text, &ItemName::new("reviewer")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// let tokens: Vec<String> = meta.tokens().iter().map(|located| located.token().to_string()).collect();
    /// assert_eq!(tokens, ["{{skill review}}"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn tokens(&self) -> &[LocatedToken] {
        &self.tokens
    }

    /// Where the body begins.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{AgentMeta, ItemName};
    ///
    /// let text = "---\nname: reviewer\ndescription: d\n---\nBody\n";
    /// let meta = AgentMeta::parse(Path::new("agents/reviewer.md"), text, &ItemName::new("reviewer")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// assert_eq!(meta.body_start().body(text), "Body\n");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn body_start(&self) -> BodyStart {
        self.body_start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "agents/coder.md";

    fn parse(text: &str) -> Result<AgentMeta, Vec<String>> {
        let expected = ItemName::new("coder").map_err(|e| vec![e.to_string()])?;
        AgentMeta::parse(Path::new(FILE), text, &expected)
            .map_err(|ds| ds.into_sorted().iter().map(ToString::to_string).collect())
    }

    fn with_block(block: &str) -> String {
        format!("---\n{block}---\n# Title\n")
    }

    fn refusals(block: &str) -> Vec<String> {
        parse(&with_block(block)).err().unwrap_or_default()
    }

    #[test]
    fn agent_meta_reads_every_key() -> TestResult {
        let block = "name: coder\ndescription: Writes code\nmodel: balanced\neffort: high\n\
                     tools: [read, edit, write, grep, glob, shell, skill]\n\
                     native:\n  claude: { model: claude-opus-5-5 }\n";
        let meta = parse(&with_block(block)).map_err(|e| e.join("; "))?;
        assert_eq!(meta.name().as_str(), "coder");
        assert_eq!(meta.description(), "Writes code");
        assert_eq!(meta.model(), Some(ModelTier::Balanced));
        assert_eq!(meta.effort(), Some(Effort::High));
        let expected = [
            AgentTool::Read,
            AgentTool::Edit,
            AgentTool::Write,
            AgentTool::Grep,
            AgentTool::Glob,
            AgentTool::Shell,
            AgentTool::Skill,
        ];
        assert_eq!(meta.tools(), Some(expected.as_slice()));
        let claude = ToolName::new("claude")?;
        assert_eq!(
            meta.native().get(&claude),
            Some(&YamlValue::Map(vec![(
                "model".to_owned(),
                YamlValue::Str("claude-opus-5-5".to_owned())
            )]))
        );
        Ok(())
    }

    #[test]
    fn agent_meta_allows_missing_optional_fields() -> TestResult {
        let meta = parse(&with_block("name: coder\ndescription: d\n")).map_err(|e| e.join("; "))?;
        assert_eq!(meta.model(), None);
        assert_eq!(meta.effort(), None);
        assert_eq!(meta.tools(), None);
        Ok(())
    }

    #[test]
    fn agent_meta_keeps_an_empty_tools_list_distinct_from_absent() -> TestResult {
        let meta = parse(&with_block("name: coder\ndescription: d\ntools: []\n"))
            .map_err(|e| e.join("; "))?;
        assert_eq!(meta.tools(), Some([].as_slice()));
        Ok(())
    }

    #[test]
    fn agent_meta_refuses_each_rule() {
        let head = "name: coder\ndescription: d\n";
        let cases = [
            ("description: d\n".to_owned(), "name: required"),
            ("name: coder\n".to_owned(), "description: required"),
            (
                "name: codr\ndescription: d\n".to_owned(),
                "name \"codr\" does not match \"coder\"",
            ),
            (
                format!("{head}model: sonnet\n"),
                "model: \"sonnet\" (expected fast, balanced, deep)",
            ),
            (
                format!("{head}effort: max\n"),
                "effort: \"max\" (expected low, medium, high)",
            ),
            (
                format!("{head}tools: [bash]\n"),
                "tools[0]: unknown \"bash\"",
            ),
            (
                format!("{head}tools: [read, read]\n"),
                "tools[1]: \"read\" listed twice",
            ),
            (
                format!("{head}tools: read\n"),
                "tools: expected array, got string",
            ),
            (
                format!("{head}tools: [1]\n"),
                "tools[0]: expected string, got integer",
            ),
            (
                format!("{head}model: 3\n"),
                "model: expected string, got integer",
            ),
        ];
        for (block, message) in cases {
            assert_eq!(refusals(&block), [format!("{FILE}: {message}")], "{block}");
        }
    }

    #[test]
    fn agent_meta_refuses_skill_only_keys() {
        let block = "name: coder\ndescription: d\ninvocation: user\n";
        assert_eq!(
            refusals(block),
            [format!("{FILE}: unknown key \"invocation\"")]
        );
    }

    #[test]
    fn agent_meta_reports_every_field_error_at_once() {
        let block = "name: coder\ndescription: d\nmodel: sonnet\neffort: max\n\
                     tools: [bash, read, read]\n";
        assert_eq!(
            refusals(block),
            [
                format!("{FILE}: model: \"sonnet\" (expected fast, balanced, deep)"),
                format!("{FILE}: effort: \"max\" (expected low, medium, high)"),
                format!("{FILE}: tools[0]: unknown \"bash\""),
                format!("{FILE}: tools[2]: \"read\" listed twice"),
            ]
        );
    }

    #[test]
    fn agent_meta_keeps_frontmatter_tokens() -> TestResult {
        let block = "name: coder\ndescription: \"hands off to {{agent core/coder}}\"\n";
        let meta = parse(&with_block(block)).map_err(|e| e.join("; "))?;
        assert_eq!(meta.tokens().len(), 1);
        assert_eq!(
            meta.tokens()
                .first()
                .map(|t| t.location().to_string())
                .as_deref(),
            Some("agents/coder.md: description")
        );
        Ok(())
    }
}
