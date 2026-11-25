//! The `api` of an existing `agentstacks.toml` moved from `0` to the current one with its comments
//! kept.
//!
//! `update` brings a project written for the `0.x` releases up to the `api` this agentstacks
//! speaks. The file has comments, alignment and a layout of its own, so this rule changes only the
//! number and leaves every other byte as it was.
//!
//! # Responsibilities
//!
//! - [`upgrade_project`] sets the top-level `api` to [`API`], keeping the
//!   comment and spacing around the old value.
//! - It refuses an `api` that is no longer the legacy `0` (the file was edited after `update` read
//!   it), so a hand edit is never overwritten.
//!
//! # Non-responsibilities
//!
//! Reading and writing the file, and checking that the text is a valid project file, belong to the
//! flow that calls this. Deciding that a project is legacy belongs to [`check`].

use toml_edit::{Item, Value};

use crate::types::api::{API, ApiCheck, check};
use crate::types::project::document::parse_document;
use crate::types::project::{PROJECT_FILE, ProjectError};
use crate::types::report::{Diagnostic, Location};

/// `text` with its `api` moved from `0` to [`API`].
///
/// Comments, spacing and every other byte of `text` are kept.
///
/// # Errors
///
/// Returns a [`Diagnostic`] located at the file when `api` is missing, not an integer or not the
/// legacy `0` (`api changed during update (run update again)`), and one located at the file when
/// `text` is not TOML, in the parser's own words.
///
/// # Examples
///
/// ```
/// use agentstacks::types::project::upgrade_project;
///
/// let text = "api = 0 # pinned\n[tools.claude]\npath = \"ext/claude\"\n";
/// let out = upgrade_project(text).map_err(|refusal| refusal.to_string())?;
/// assert_eq!(out, "api = 1 # pinned\n[tools.claude]\npath = \"ext/claude\"\n");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn upgrade_project(text: &str) -> Result<String, Diagnostic> {
    let mut document = parse_document(text)?;
    let slot = document
        .get_mut("api")
        .and_then(Item::as_value_mut)
        .filter(|slot| slot.as_integer().map(check) == Some(ApiCheck::Legacy))
        .ok_or_else(|| Diagnostic::new(Location::file(PROJECT_FILE), ProjectError::ApiChanged))?;
    let decor = slot.decor().clone();
    *slot = Value::from(API);
    *slot.decor_mut() = decor;
    Ok(document.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upgrade(text: &str) -> Result<String, String> {
        upgrade_project(text).map_err(|refusal| refusal.to_string())
    }

    #[test]
    fn api_zero_becomes_one_keeping_comment() {
        let text =
            "# my project\napi = 0 # pinned by hand\n\n[tools.claude]\npath = \"ext/claude\"\n";
        assert_eq!(
            upgrade(text),
            Ok(
                "# my project\napi = 1 # pinned by hand\n\n[tools.claude]\npath = \"ext/claude\"\n"
                    .to_owned()
            )
        );
    }

    #[test]
    fn api_changed_since_read_is_refused() {
        for text in [
            "api = 1\n",
            "api = 2\n",
            "api = \"0\"\n",
            "[tools.claude]\n",
        ] {
            assert_eq!(
                upgrade(text),
                Err("agentstacks.toml: api changed during update (run update again)".to_owned()),
                "{text:?}"
            );
        }
    }

    #[test]
    fn rest_of_file_unchanged() {
        let text =
            "# top\n\n   api   =   0   # late\n\n[packs.core]   # a pack\npath =  \"packs/core\"\n";
        let out = upgrade(text);
        assert_eq!(out, Ok(text.replacen("0   # late", "1   # late", 1)));
    }

    #[test]
    fn text_that_is_not_toml_is_refused_at_the_file() {
        let refused = upgrade("api = \n");
        assert!(
            matches!(&refused, Err(message) if message.starts_with("agentstacks.toml: ")),
            "{refused:?}"
        );
    }
}
