//! The skip the core makes when a tool gave no value for something a text needs.
//!
//! A text that uses a token the extension gave no value for cannot be installed for that tool.
//! The core skips it with a reason that names the tool and the token, so the gap is visible
//! rather than silent.
//!
//! # Responsibilities
//!
//! - [`first_missing`] finds the first token, in the order given, that the answer has no value
//!   for.
//! - [`skip_for_missing_value`] skips the item the text belongs to, which is the whole skill for
//!   a file of a skill, and [`skip_root_file_for_missing_value`] skips a root file of a pack.
//! - [`skip_skill_without_location`] skips a skill the extension gave no install location for;
//!   that comes before any token.
//!
//! # Non-responsibilities
//!
//! Scanning and rendering the texts, and deciding the order the items are checked in, belong
//! to the plan flow.

use crate::types::name::{ItemName, PackName, ToolName};
use crate::types::op::answer::{TokenValue, ValuesResult};
use crate::types::op::{CoreSkip, ItemId, SkipReason, SkipSubject};
use crate::types::tokens::Token;
use crate::types::tree::TreePath;

/// The first of `tokens` that `values` gives no value for, for a text of the pack `pack`.
#[must_use]
pub fn first_missing<'a>(
    tokens: impl IntoIterator<Item = &'a Token>,
    values: &ValuesResult,
    pack: &PackName,
) -> Option<&'a Token> {
    tokens
        .into_iter()
        .find(|token| values.value(token.target(), pack) == TokenValue::Missing)
}

/// The reason a token with no value gives.
fn no_value(tool: &ToolName, token: &Token) -> SkipReason {
    SkipReason::core(format!("{tool} gave no value for {token}"))
}

/// The skip of `item`, which `tool` gave no value for `token` in. A file of a skill skips the
/// whole skill.
#[must_use]
pub fn skip_for_missing_value(tool: &ToolName, item: &ItemId, token: &Token) -> CoreSkip {
    let subject = item.group().unwrap_or_else(|| item.clone());
    CoreSkip::new(SkipSubject::Item(subject), no_value(tool, token))
}

/// The skip of the root file `rel` of `pack`, which `tool` gave no value for `token` in.
#[must_use]
pub fn skip_root_file_for_missing_value(
    tool: &ToolName,
    pack: &PackName,
    rel: &TreePath,
    token: &Token,
) -> CoreSkip {
    let subject = SkipSubject::RootFile {
        pack: pack.clone(),
        rel: rel.clone(),
    };
    CoreSkip::new(subject, no_value(tool, token))
}

/// The skip of `skill` of `pack`, which `tool` gave no install location for.
#[must_use]
pub fn skip_skill_without_location(tool: &ToolName, pack: &PackName, skill: &ItemName) -> CoreSkip {
    let reason = SkipReason::core(format!(
        "{tool} gave no install location for skill {pack}/{skill}"
    ));
    CoreSkip::new(SkipSubject::Item(ItemId::skill(pack, skill)), reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::op::testing::{rel, tokens, values_result};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn claude() -> Result<ToolName, Box<dyn std::error::Error>> {
        Ok(ToolName::new("claude")?)
    }

    fn core() -> Result<PackName, Box<dyn std::error::Error>> {
        Ok(PackName::new("core")?)
    }

    fn token(text: &str) -> Result<Token, Box<dyn std::error::Error>> {
        Ok(tokens(text).into_iter().next().ok_or("a token")?)
    }

    fn subject(skip: &CoreSkip) -> String {
        match skip.subject() {
            SkipSubject::Item(id) => id.to_string(),
            SkipSubject::RootFile { pack, rel } => format!("{pack}:{rel}"),
        }
    }

    #[test]
    fn first_missing_is_the_first_in_text_order() -> TestResult {
        let scanned = tokens("{{var tmpdir}} {{agent core/x}}\n{{var home}}");
        let values = values_result()?;
        let found = first_missing(&scanned, &values, &core()?);
        assert_eq!(
            found.map(ToString::to_string).as_deref(),
            Some("{{agent core/x}}")
        );
        let none = tokens("{{var tmpdir}} {{option a}}");
        assert_eq!(first_missing(&none, &values, &core()?), None);
        Ok(())
    }

    #[test]
    fn missing_value_skips_the_whole_skill() -> TestResult {
        let id = ItemId::skill_file(&core()?, &ItemName::new("status")?, &rel(&["SKILL.md"]));
        let skip = skip_for_missing_value(&claude()?, &id, &token("{{skill core/status}}")?);
        assert_eq!(subject(&skip), "core/skill/status");
        assert_eq!(
            skip.reason().as_str(),
            "claude gave no value for {{skill core/status}}"
        );
        let group = ItemId::skill(&core()?, &ItemName::new("status")?);
        let again = skip_for_missing_value(&claude()?, &group, &token("{{var home}}")?);
        assert_eq!(subject(&again), "core/skill/status");
        Ok(())
    }

    #[test]
    fn missing_value_skips_other_items_as_themselves() -> TestResult {
        let ids = [
            ItemId::agent(&core()?, &ItemName::new("coder")?),
            ItemId::hook(&core()?, 0),
            ItemId::native_file(&core()?, &claude()?, &rel(&["x.md"])),
            ItemId::native_hook(&core()?, 0),
        ];
        for id in ids {
            let skip = skip_for_missing_value(&claude()?, &id, &token("{{var home}}")?);
            assert_eq!(subject(&skip), id.to_string());
            assert_eq!(
                skip.reason().as_str(),
                "claude gave no value for {{var home}}"
            );
        }
        Ok(())
    }

    #[test]
    fn missing_value_in_a_root_file_is_a_core_skip_of_that_file() -> TestResult {
        let skip = skip_root_file_for_missing_value(
            &claude()?,
            &core()?,
            &rel(&["hooks", "x.lua"]),
            &token("{{var home}}")?,
        );
        assert_eq!(subject(&skip), "core:hooks/x.lua");
        assert_eq!(
            skip.reason().as_str(),
            "claude gave no value for {{var home}}"
        );
        Ok(())
    }

    #[test]
    fn skill_without_entry_has_no_install_location() -> TestResult {
        let skip = skip_skill_without_location(&claude()?, &core()?, &ItemName::new("status")?);
        assert_eq!(subject(&skip), "core/skill/status");
        assert_eq!(
            skip.reason().as_str(),
            "claude gave no install location for skill core/status"
        );
        Ok(())
    }
}
