//! The key path a refusal about a field of `pack.toml` or `hooks.toml` is located at.
//!
//! It is a type of its own so the pack loader and the set checks build `hooks[1].agents[0]` the
//! same way, one segment at a time, instead of each spelling the brackets and dots with `format!`.
//!
//! # Responsibilities
//!
//! - [`KeyPath`] starts at a top-level key, appends a list index (`key[1]`) or a child key
//!   (`.run`), and turns into the text a [`Location`](crate::types::Location) holds.
//!
//! # Non-responsibilities
//!
//! Reading the fields themselves belongs to the TOML readers, which build the paths of the fields
//! they read from their own table prefix.

use std::fmt;

/// A key path built one segment at a time, such as `hooks[1].agents[0]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::types) struct KeyPath(String);

impl KeyPath {
    /// The path of the top-level key `key`.
    pub(in crate::types) fn new(key: &str) -> Self {
        Self(key.to_owned())
    }

    /// The path of item `index` of the list at this path: `<path>[<index>]`.
    pub(in crate::types) fn item(mut self, index: usize) -> Self {
        self.0.push('[');
        self.0.push_str(&index.to_string());
        self.0.push(']');
        self
    }

    /// The path of the key `key` of the table at this path: `<path>.<key>`.
    pub(in crate::types) fn key(mut self, key: &str) -> Self {
        self.0.push('.');
        self.0.push_str(key);
        self
    }
}

impl From<KeyPath> for String {
    fn from(path: KeyPath) -> Self {
        path.0
    }
}

impl fmt::Display for KeyPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_path_new_is_the_key_itself() {
        assert_eq!(KeyPath::new("render").to_string(), "render");
    }

    #[test]
    fn key_path_item_appends_an_index() {
        assert_eq!(KeyPath::new("exclude").item(2).to_string(), "exclude[2]");
    }

    #[test]
    fn key_path_key_appends_a_child_key() {
        assert_eq!(
            KeyPath::new("hooks").item(1).key("run").to_string(),
            "hooks[1].run"
        );
    }

    #[test]
    fn key_path_nests_lists_and_tables() {
        let path = KeyPath::new("hooks").item(0).key("agents").item(3);
        assert_eq!(String::from(path), "hooks[0].agents[3]");
    }
}
