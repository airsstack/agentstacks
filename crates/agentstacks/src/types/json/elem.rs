//! One element of the key path a JSON edit follows.
//!
//! # Responsibilities
//!
//! - [`PathElem`]: a path element as the caller read it, a key or the type word of a non-string.
//!
//! # Non-responsibilities
//!
//! Refusing a non-string element, and the order in which that refusal is checked, belong to the
//! code that edits the document.

use crate::types::op::LuaType;

/// One element of a key path: a string key, or the type of an element that is not a string.
///
/// # Examples
///
/// ```
/// use agentstacks::types::json::PathElem;
/// use agentstacks::types::op::LuaType;
///
/// let path = [
///     PathElem::Key("hooks".to_owned()),
///     PathElem::NotString(LuaType::Number),
/// ];
/// assert_eq!(path.len(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathElem {
    /// A string element, the key it names.
    Key(String),
    /// An element that is not a string, with the type it has.
    NotString(LuaType),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_elem_keeps_a_key_or_a_type_word() {
        let key = PathElem::Key("a".to_owned());
        assert!(matches!(&key, PathElem::Key(name) if name == "a"));
        let word = PathElem::NotString(LuaType::Number);
        assert!(matches!(word, PathElem::NotString(LuaType::Number)));
    }
}
