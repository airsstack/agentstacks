//! The metadata a skill's frontmatter declares.
//!
//! Skills and agents have different keys and different vocabularies, so each has its own type
//! rather than one struct with optional fields. The checks they share live in
//! `FrontmatterFields`.
//!
//! # Responsibilities
//!
//! - [`SkillMeta::parse`] splits a `SKILL.md`, parses its frontmatter and checks every key. A
//!   missing block, a syntax error or a block that is not a mapping is the file's only diagnostic;
//!   otherwise every field error of the file is reported together.
//! - The keys are `name`, `description`, `invocation`, `argument_hint` and `native`. The free-text
//!   values `description` and `argument_hint` are scanned for tokens once; the tokens are kept.
//! - The returned [`SkillMeta`] exposes where the body begins, so the body is scanned once by the
//!   caller.
//!
//! # Non-responsibilities
//!
//! Where a token may appear and what it may point at is checked over all tokens of a pack and of a
//! set of packs, not here. Values under `native` are opaque.

use std::collections::BTreeMap;
use std::path::Path;

use super::fields::FrontmatterFields;
use super::wording::StringWording;
use crate::types::format::YamlValue;
use crate::types::frontmatter::block::Frontmatter;
use crate::types::frontmatter::body::BodyStart;
use crate::types::frontmatter::invocation::Invocation;
use crate::types::name::ItemName;
use crate::types::name::ToolName;
use crate::types::report::Diagnostics;
use crate::types::tokens::LocatedToken;

/// The keys a skill's frontmatter may hold.
const KEYS: [&str; 5] = [
    "name",
    "description",
    "invocation",
    "argument_hint",
    "native",
];

/// A skill's checked frontmatter.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::{Invocation, ItemName, SkillMeta};
///
/// let text = "---\nname: review\ndescription: Reviews a diff.\ninvocation: user\n---\nBody\n";
/// let name = ItemName::new("review")?;
/// let meta = SkillMeta::parse(Path::new("skills/review/SKILL.md"), text, &name)
///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
/// assert_eq!(meta.name().as_str(), "review");
/// assert_eq!(meta.description(), "Reviews a diff.");
/// assert_eq!(meta.invocation(), Invocation::User);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct SkillMeta {
    name: ItemName,
    description: String,
    invocation: Invocation,
    argument_hint: Option<String>,
    native: BTreeMap<ToolName, YamlValue>,
    tokens: Vec<LocatedToken>,
    body_start: BodyStart,
}

impl SkillMeta {
    /// Parses the frontmatter of `text`, the content of the `SKILL.md` at `file`. `expected_name`
    /// is the name of the skill's directory.
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
    /// use agentstacks::types::{ItemName, SkillMeta};
    ///
    /// let file = Path::new("skills/review/SKILL.md");
    /// let name = ItemName::new("review")?;
    ///
    /// let good = "---\nname: review\ndescription: Reviews a diff.\n---\n";
    /// assert!(SkillMeta::parse(file, good, &name).is_ok());
    ///
    /// let wrong_name = "---\nname: other\ndescription: Reviews a diff.\n---\n";
    /// let refusals = SkillMeta::parse(file, wrong_name, &name).err().unwrap_or_default();
    /// let shown: Vec<String> = refusals.into_sorted().iter().map(ToString::to_string).collect();
    /// assert_eq!(shown, ["skills/review/SKILL.md: name \"other\" does not match \"review\""]);
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
        let invocation = fields
            .optional_string("invocation", StringWording::Generic)
            .and_then(|value| match value.parse::<Invocation>() {
                Ok(invocation) => Some(invocation),
                Err(error) => {
                    fields.refuse_at("invocation", error);
                    None
                }
            })
            .unwrap_or_default();
        let argument_hint = fields.optional_string("argument_hint", StringWording::WithArticle);
        if let Some(hint) = &argument_hint {
            fields.scan_text("argument_hint", hint);
        }
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
            invocation,
            argument_hint,
            native,
            tokens,
            body_start: split.body_start(),
        })
    }

    /// The skill's name, equal to its directory name.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{ItemName, SkillMeta};
    ///
    /// let text = "---\nname: review\ndescription: d\n---\n";
    /// let name = ItemName::new("review")?;
    /// let meta = SkillMeta::parse(Path::new("SKILL.md"), text, &name)
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
    /// use agentstacks::types::{ItemName, SkillMeta};
    ///
    /// let text = "---\nname: review\ndescription: Reviews a diff.\n---\n";
    /// let meta = SkillMeta::parse(Path::new("SKILL.md"), text, &ItemName::new("review")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// assert_eq!(meta.description(), "Reviews a diff.");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Who may start the skill; `any` when the frontmatter does not say.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{Invocation, ItemName, SkillMeta};
    ///
    /// let text = "---\nname: review\ndescription: d\n---\n";
    /// let meta = SkillMeta::parse(Path::new("SKILL.md"), text, &ItemName::new("review")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// assert_eq!(meta.invocation(), Invocation::Any);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn invocation(&self) -> Invocation {
        self.invocation
    }

    /// The argument hint, when there is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{ItemName, SkillMeta};
    ///
    /// let text = "---\nname: review\ndescription: d\nargument_hint: a PR number\n---\n";
    /// let meta = SkillMeta::parse(Path::new("SKILL.md"), text, &ItemName::new("review")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// assert_eq!(meta.argument_hint(), Some("a PR number"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn argument_hint(&self) -> Option<&str> {
        self.argument_hint.as_deref()
    }

    /// The tool-specific values, keyed by tool.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{ItemName, SkillMeta};
    ///
    /// let text = "---\nname: review\ndescription: d\nnative:\n  claude:\n    context: fork\n---\n";
    /// let meta = SkillMeta::parse(Path::new("SKILL.md"), text, &ItemName::new("review")?)
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
    /// use agentstacks::types::{ItemName, SkillMeta};
    ///
    /// let text = "---\nname: review\ndescription: Runs \"{{agent reviewer}}\".\n---\n";
    /// let meta = SkillMeta::parse(Path::new("SKILL.md"), text, &ItemName::new("review")?)
    ///     .map_err(|refusals| format!("{} refusals", refusals.len()))?;
    /// let tokens: Vec<String> = meta.tokens().iter().map(|located| located.token().to_string()).collect();
    /// assert_eq!(tokens, ["{{agent reviewer}}"]);
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
    /// use agentstacks::types::{ItemName, SkillMeta};
    ///
    /// let text = "---\nname: review\ndescription: d\n---\nBody\n";
    /// let meta = SkillMeta::parse(Path::new("SKILL.md"), text, &ItemName::new("review")?)
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
    use crate::types::TokenKind;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "skills/discuss/SKILL.md";

    fn parse(text: &str) -> Result<SkillMeta, Vec<String>> {
        let expected = ItemName::new("discuss").map_err(|e| vec![e.to_string()])?;
        SkillMeta::parse(Path::new(FILE), text, &expected)
            .map_err(|ds| ds.into_sorted().iter().map(ToString::to_string).collect())
    }

    fn with_block(block: &str) -> String {
        format!("---\n{block}---\n# Title\n")
    }

    fn refusals(block: &str) -> Vec<String> {
        parse(&with_block(block)).err().unwrap_or_default()
    }

    #[test]
    fn skill_meta_reads_every_key() -> TestResult {
        let block = "name: discuss\ndescription: Talk it through\ninvocation: user\n\
                     argument_hint: \"[list | <id> | done]\"\nnative:\n  claude: { model: x }\n";
        let meta = parse(&with_block(block)).map_err(|e| e.join("; "))?;
        assert_eq!(meta.name().as_str(), "discuss");
        assert_eq!(meta.description(), "Talk it through");
        assert_eq!(meta.invocation(), Invocation::User);
        assert_eq!(meta.argument_hint(), Some("[list | <id> | done]"));
        let tools: Vec<&str> = meta.native().keys().map(ToolName::as_str).collect();
        assert_eq!(tools, ["claude"]);
        Ok(())
    }

    #[test]
    fn skill_meta_defaults_invocation_to_any() -> TestResult {
        let meta =
            parse(&with_block("name: discuss\ndescription: d\n")).map_err(|e| e.join("; "))?;
        assert_eq!(meta.invocation(), Invocation::Any);
        assert_eq!(meta.argument_hint(), None);
        Ok(())
    }

    #[test]
    fn skill_meta_refuses_each_rule() {
        let cases = [
            ("description: d\n", "name: required"),
            ("name: discuss\n", "description: required"),
            ("name: discuss\ndescription: \"  \"\n", "description: empty"),
            (
                "name: \"{{skill discuss}}\"\ndescription: d\n",
                "name must be a literal",
            ),
            (
                "name: talk\ndescription: d\n",
                "name \"talk\" does not match \"discuss\"",
            ),
            (
                "name: discuss\ndescription: d\ndisable-model-invocation: true\n",
                "unknown key \"disable-model-invocation\"",
            ),
            (
                "name: discuss\ndescription: d\ninvocation: manual\n",
                "invocation: \"manual\" (expected user, any)",
            ),
            (
                "name: discuss\ndescription: d\nargument_hint: yes\n",
                "argument_hint: expected a string, got boolean",
            ),
            (
                "name: discuss\ndescription: d\nnative: { Claude: { model: x } }\n",
                "native.Claude: \"Claude\" is not a valid tool name",
            ),
        ];
        for (block, message) in cases {
            assert_eq!(refusals(block), [format!("{FILE}: {message}")], "{block}");
        }
    }

    #[test]
    fn skill_meta_refuses_agent_only_keys() {
        let block = "name: discuss\ndescription: d\nmodel: fast\n";
        assert_eq!(refusals(block), [format!("{FILE}: unknown key \"model\"")]);
    }

    #[test]
    fn skill_meta_reports_every_field_error_at_once() {
        let block = "description: \"\"\ninvocation: manual\nextra: 1\n";
        assert_eq!(
            refusals(block),
            [
                format!("{FILE}: name: required"),
                format!("{FILE}: description: empty"),
                format!("{FILE}: invocation: \"manual\" (expected user, any)"),
                format!("{FILE}: unknown key \"extra\""),
            ]
        );
    }

    #[test]
    fn skill_meta_keeps_frontmatter_tokens_and_body_start() -> TestResult {
        let text = "---\nname: discuss\ndescription: \"see {{skill sdlc/status}}\"\n---\n\
                    # Title\nbody {{skill other/thing}}\n";
        let meta = parse(text).map_err(|e| e.join("; "))?;
        assert_eq!(meta.tokens().len(), 1);
        assert_eq!(
            meta.tokens().first().map(|t| t.token().kind()),
            Some(TokenKind::Skill)
        );
        assert_eq!(meta.body_start().line().get(), 5);
        assert!(meta.body_start().body(text).starts_with("# Title"));
        Ok(())
    }

    #[test]
    fn skill_meta_scans_argument_hint_for_tokens() -> TestResult {
        let block = "name: discuss\ndescription: d\nargument_hint: \"[{{var arguments}}]\"\n";
        let meta = parse(&with_block(block)).map_err(|e| e.join("; "))?;
        assert_eq!(
            meta.tokens()
                .iter()
                .map(|t| t.token().kind())
                .collect::<Vec<_>>(),
            [TokenKind::Var]
        );
        assert_eq!(
            meta.tokens()
                .first()
                .map(|t| t.location().to_string())
                .as_deref(),
            Some("skills/discuss/SKILL.md: argument_hint")
        );
        Ok(())
    }

    #[test]
    fn skill_meta_refuses_malformed_token_in_argument_hint() {
        let block = "name: discuss\ndescription: d\nargument_hint: \"run {{skil x}}\"\n";
        assert_eq!(
            refusals(block),
            [format!(
                "{FILE}: argument_hint: unknown token kind \"skil\""
            )]
        );
    }

    #[test]
    fn skill_meta_refuses_a_name_yaml_reads_as_boolean() {
        let block = "name: on\ndescription: d\n";
        assert_eq!(
            refusals(block),
            [format!("{FILE}: name: expected string, got boolean")]
        );
    }

    #[test]
    fn skill_meta_syntax_error_stops_the_file() {
        let block = "description: {{skill status}} runs it\n";
        assert_eq!(
            refusals(block),
            [format!(
                "{FILE}:2: expected string scalar (a value starting with {{{{ must be quoted)"
            )]
        );
    }
}
