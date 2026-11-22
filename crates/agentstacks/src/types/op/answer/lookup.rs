//! What a checked `values` answer gives for a token.
//!
//! # Responsibilities
//!
//! - [`ValuesResult::value`] looks a token target up: `{{root}}` is the root and the pack's
//!   name, a skill token is its `invoke` or `dir`, an agent or a var token is the string the
//!   extension gave, and a token the extension gave nothing for is missing.
//! - [`TokenValue`] says which of those it is. An option token is not the extension's to
//!   answer, so it is left to the core.
//!
//! # Non-responsibilities
//!
//! Filling the values into a text, and resolving an option, belong to the plan flow. What a
//! missing value means for an item belongs to the missing-value rule.

use crate::types::name::PackName;
use crate::types::op::answer::ValuesResult;
use crate::types::tokens::TokenTarget;

/// What a `values` answer gives for one token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenValue {
    /// The text the token is replaced with.
    Given(String),
    /// The extension gave nothing for this token.
    Missing,
    /// An option token, which the core resolves from the project's options.
    FromOption,
}

impl ValuesResult {
    /// The value of `target`, a token in a text of the pack `pack`.
    ///
    /// `pack` completes a token that names no pack: `{{root}}`, and `{{skill status}}`.
    #[must_use]
    pub fn value(&self, target: &TokenTarget, pack: &PackName) -> TokenValue {
        let given = |found: Option<String>| found.map_or(TokenValue::Missing, TokenValue::Given);
        match target {
            TokenTarget::Root(named) => {
                let owner = named.as_ref().unwrap_or(pack);
                TokenValue::Given(self.root().join(owner.as_str()).to_string())
            }
            TokenTarget::Skill(item) => given(
                self.skill(item.pack().unwrap_or(pack), item.item())
                    .map(|skill| skill.invoke.clone()),
            ),
            TokenTarget::SkillDir(item) => given(
                self.skill(item.pack().unwrap_or(pack), item.item())
                    .map(|skill| skill.dir.to_string()),
            ),
            TokenTarget::Agent(item) => given(
                self.agent(item.pack().unwrap_or(pack), item.item())
                    .map(str::to_owned),
            ),
            TokenTarget::Var(var) => given(self.var(*var).map(str::to_owned)),
            TokenTarget::Option(_) => TokenValue::FromOption,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::op::testing::{tokens, values_result};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn value_of(text: &str) -> Result<TokenValue, Box<dyn std::error::Error>> {
        let token = tokens(text).into_iter().next().ok_or("a token")?;
        Ok(values_result()?.value(token.target(), &PackName::new("core")?))
    }

    fn given(text: &str) -> TokenValue {
        TokenValue::Given(text.to_owned())
    }

    #[test]
    fn root_token_is_root_slash_pack() -> TestResult {
        assert_eq!(value_of("{{root}}")?, given(".claude/agentstacks/core"));
        assert_eq!(
            value_of("{{root core}}")?,
            given(".claude/agentstacks/core")
        );
        assert_eq!(
            value_of("{{root sdlc}}")?,
            given(".claude/agentstacks/sdlc")
        );
        Ok(())
    }

    #[test]
    fn skill_and_skill_dir_tokens_read_invoke_and_dir() -> TestResult {
        assert_eq!(value_of("{{skill core/status}}")?, given("/status"));
        assert_eq!(
            value_of("{{skill_dir core/status}}")?,
            given(".claude/skills/status")
        );
        assert_eq!(value_of("{{skill status}}")?, given("/status"));
        Ok(())
    }

    #[test]
    fn agent_and_var_tokens_read_their_maps() -> TestResult {
        assert_eq!(value_of("{{agent core/coder}}")?, given("coder"));
        assert_eq!(value_of("{{var tmpdir}}")?, given("$TMPDIR"));
        Ok(())
    }

    #[test]
    fn option_token_is_left_to_the_core() -> TestResult {
        assert_eq!(value_of("{{option archive_keep}}")?, TokenValue::FromOption);
        Ok(())
    }

    #[test]
    fn absent_values_are_missing() -> TestResult {
        for text in [
            "{{skill core/other}}",
            "{{skill_dir core/other}}",
            "{{agent core/x}}",
            "{{var home}}",
        ] {
            assert_eq!(value_of(text)?, TokenValue::Missing, "{text}");
        }
        Ok(())
    }
}
