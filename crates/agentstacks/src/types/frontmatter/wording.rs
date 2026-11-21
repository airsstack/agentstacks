//! Which wording a wrong-type refusal of a string field uses.
//!
//! The enum holds no logic itself; its tests drive
//! [`FrontmatterFields`](crate::types::frontmatter::fields::FrontmatterFields) with each variant, so each variant's
//! message is pinned in the file that defines it.
//!
//! # Responsibilities
//!
//! - [`StringWording`] picks between the generic `expected string, got <type>` and the spelled-out
//!   `expected a string, got <type>` that `argument_hint` has.
//!
//! # Non-responsibilities
//!
//! Reading the value and finding it is not a string belongs to
//! [`FrontmatterFields`](crate::types::frontmatter::fields::FrontmatterFields).

/// How a string field words its wrong-type refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StringWording {
    /// `expected string, got <type>`, the generic shape.
    Generic,
    /// `expected a string, got <type>`.
    WithArticle,
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::format::YamlValue;
    use crate::types::frontmatter::fields::FrontmatterFields;

    fn refusal_of(value: YamlValue, wording: StringWording) -> Vec<String> {
        let mut fields =
            FrontmatterFields::new(Path::new("SKILL.md"), vec![("k".to_owned(), value)]);
        assert_eq!(fields.optional_string("k", wording), None);
        fields
            .finish(&["k"])
            .0
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn generic_wording_says_expected_string() {
        assert_eq!(
            refusal_of(YamlValue::Int(3), StringWording::Generic),
            ["SKILL.md: k: expected string, got integer"]
        );
    }

    #[test]
    fn article_wording_says_expected_a_string() {
        assert_eq!(
            refusal_of(YamlValue::Bool(true), StringWording::WithArticle),
            ["SKILL.md: k: expected a string, got boolean"]
        );
    }

    #[test]
    fn wordings_are_distinct_and_copy() {
        let wording = StringWording::Generic;
        let copy = wording;
        assert_eq!(wording, copy);
        assert_ne!(StringWording::Generic, StringWording::WithArticle);
    }
}
