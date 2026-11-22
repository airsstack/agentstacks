//! A file of a skill or of a tool's native directory, as the plan payload lists it.
//!
//! # Responsibilities
//!
//! - [`PlanFile`] holds the id an extension copies back, the file's path relative to its
//!   directory, and its rendered text when it has any.
//!
//! # Non-responsibilities
//!
//! Reading and rendering the file belongs to the plan flow.

use crate::types::op::ItemId;
use crate::types::tree::TreePath;

/// One file in the plan payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFile {
    /// The id an extension copies back in `copy` or lists in `covers`.
    pub id: ItemId,
    /// The path relative to the skill directory, or to `native/<tool>/`.
    pub rel: TreePath,
    /// The rendered text of a UTF-8 file, the body alone for `SKILL.md`; `None` for a binary
    /// file, which an extension installs with `copy`.
    pub text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::name::{ItemName, PackName};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn file(rel: &str, text: Option<&str>) -> Result<PlanFile, Box<dyn std::error::Error>> {
        let path = TreePath::from_parts(&[rel]);
        Ok(PlanFile {
            id: ItemId::skill_file(&PackName::new("core")?, &ItemName::new("status")?, &path),
            rel: path,
            text: text.map(str::to_owned),
        })
    }

    #[test]
    fn plan_file_text_is_absent_for_a_binary_file() -> TestResult {
        let binary = file("logo.png", None)?;
        let text = file("SKILL.md", Some("body"))?;
        assert_eq!(binary.text, None);
        assert_eq!(text.text.as_deref(), Some("body"));
        assert_eq!(binary.id.as_str(), "core/skill/status/logo.png");
        Ok(())
    }
}
