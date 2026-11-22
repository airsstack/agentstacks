//! A project-relative path an operation names, held in its normalised form.
//!
//! An extension proposes a path in `write` and `copy`, and returns one in its `values`. Both are
//! held as one type so a duplicate check, a lock lookup and a lock entry all see one form.
//!
//! # Responsibilities
//!
//! - [`OpPath`] removes `.` and empty segments (`./a` is `a`, `a//b` is `a/b`) and is compared
//!   and ordered by the normalised text in bytes, so it is case-sensitive.
//! - It refuses, in this order, a path that is empty (or normalises to nothing), absolute,
//!   climbs out of the project with `..`, or uses `\` as a separator. The first failing check is
//!   the refusal.
//! - [`OpPath::proposed`] words that refusal for the function that was called, and also refuses
//!   the two files agentstacks writes itself. [`OpPath::new`] returns the bare [`PathShape`], for
//!   a place whose location already names the field.
//!
//! # Non-responsibilities
//!
//! Whether a path was already proposed, and whether it may be written, belong to the proposal
//! record and the step rules.

use std::fmt;

use crate::types::op::{ApiFunction, OpError, PathShape};
use crate::types::report::{MustBeRelative, OutsideProject, Quoted};

/// The files agentstacks writes itself, which an extension may not propose.
const OWN_FILES: [&str; 2] = ["agentstacks.lock", "agentstacks.toml"];

/// A project-relative path, in its normalised form.
///
/// # Examples
///
/// ```
/// use agentstacks::types::op::OpPath;
///
/// let path = OpPath::new("./.claude//agentstacks")?;
/// assert_eq!(path.as_str(), ".claude/agentstacks");
/// assert_eq!(path.join("runtime.lua").as_str(), ".claude/agentstacks/runtime.lua");
///
/// let refused = OpPath::new("../x").err().map(|shape| shape.to_string());
/// assert_eq!(refused.as_deref(), Some("\"../x\" must stay inside the project"));
/// # Ok::<(), agentstacks::types::op::PathShape>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpPath(String);

/// The segments of `text` that are neither empty nor `.`.
fn kept_segments(text: &str) -> impl Iterator<Item = &str> {
    text.split('/')
        .filter(|segment| !segment.is_empty() && *segment != ".")
}

/// The normalised text of `text`, or the shape it is refused for.
fn normalise(text: &str) -> Result<String, PathShape> {
    let absolute = text.starts_with('/');
    if text.trim().is_empty() || (!absolute && kept_segments(text).next().is_none()) {
        return Err(PathShape::Empty);
    }
    if absolute {
        return Err(PathShape::MustBeRelative(MustBeRelative::new(text)));
    }
    if text.split('/').any(|segment| segment == "..") {
        return Err(PathShape::OutsideProject(OutsideProject::new(text)));
    }
    if text.contains('\\') {
        return Err(PathShape::Separator(Quoted::new(text)));
    }
    Ok(kept_segments(text).collect::<Vec<_>>().join("/"))
}

impl OpPath {
    /// The path `text`, for a place whose location already names the field (`values.root`, a
    /// skill's `dir`).
    ///
    /// # Errors
    ///
    /// The [`PathShape`] of the first check `text` fails: empty, absolute, outside the project,
    /// or not `/`-separated.
    pub fn new(text: &str) -> Result<Self, PathShape> {
        normalise(text).map(Self)
    }

    /// The path an extension gave to `function`.
    ///
    /// # Errors
    ///
    /// [`OpError::OpPath`] with the field `path` when the shape is refused, and
    /// [`OpError::PathManaged`] when the normalised path is `agentstacks.lock` or
    /// `agentstacks.toml`.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::op::{ApiFunction, OpPath};
    ///
    /// assert!(OpPath::proposed(".claude/x", ApiFunction::Write).is_ok());
    /// let refused = OpPath::proposed("/x", ApiFunction::Write).err().map(|e| e.to_string());
    /// assert_eq!(refused.as_deref(), Some("write: path \"/x\" must be relative"));
    /// ```
    pub fn proposed(text: &str, function: ApiFunction) -> Result<Self, OpError> {
        let normalised = normalise(text).map_err(|shape| OpError::OpPath {
            function,
            field: "path".to_owned(),
            shape,
        })?;
        if OWN_FILES.contains(&normalised.as_str()) {
            return Err(OpError::PathManaged {
                function,
                path: Quoted::new(normalised),
            });
        }
        Ok(Self(normalised))
    }

    /// This path with the already-valid relative `child` appended, normalised.
    #[must_use]
    pub fn join(&self, child: &str) -> Self {
        Self(
            kept_segments(&self.0)
                .chain(kept_segments(child))
                .collect::<Vec<_>>()
                .join("/"),
        )
    }

    /// The normalised text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OpPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn proposed(text: &str) -> Result<String, String> {
        OpPath::proposed(text, ApiFunction::Write)
            .map(|path| path.as_str().to_owned())
            .map_err(|error| error.to_string())
    }

    fn bare(text: &str) -> Result<String, String> {
        OpPath::new(text)
            .map(|path| path.as_str().to_owned())
            .map_err(|shape| shape.to_string())
    }

    #[test]
    fn op_path_normalises_dot_and_empty_segments() {
        assert_eq!(proposed("./a"), Ok("a".to_owned()));
        assert_eq!(proposed("a//b"), Ok("a/b".to_owned()));
        assert_eq!(proposed("a/./b/"), Ok("a/b".to_owned()));
        assert_eq!(proposed(".claude/x"), Ok(".claude/x".to_owned()));
    }

    #[test]
    fn op_path_refuses_empty() {
        for text in ["", "  ", ".", "./"] {
            assert_eq!(
                proposed(text),
                Err("write: path: empty".to_owned()),
                "{text:?}"
            );
        }
    }

    #[test]
    fn op_path_refuses_a_leading_slash() {
        assert_eq!(
            proposed("/x"),
            Err("write: path \"/x\" must be relative".to_owned())
        );
    }

    #[test]
    fn op_path_refuses_a_dot_dot_segment() {
        assert_eq!(
            proposed("../x"),
            Err("write: path \"../x\" must stay inside the project".to_owned())
        );
        assert_eq!(
            proposed("a/../b"),
            Err("write: path \"a/../b\" must stay inside the project".to_owned())
        );
    }

    #[test]
    fn op_path_refuses_a_backslash() {
        assert_eq!(
            proposed("a\\b"),
            Err("write: path \"a\\b\" must use / as the separator".to_owned())
        );
    }

    #[test]
    fn op_path_first_failing_cell_wins() {
        assert_eq!(
            proposed("/a\\b"),
            Err("write: path \"/a\\b\" must be relative".to_owned())
        );
        assert_eq!(
            proposed("../a\\b"),
            Err("write: path \"../a\\b\" must stay inside the project".to_owned())
        );
    }

    #[test]
    fn op_path_proposed_refuses_agentstacks_own_files() {
        for (text, shown) in [
            ("agentstacks.lock", "agentstacks.lock"),
            ("./agentstacks.lock", "agentstacks.lock"),
            ("agentstacks.toml", "agentstacks.toml"),
        ] {
            assert_eq!(
                proposed(text),
                Err(format!("write: path \"{shown}\" is managed by agentstacks")),
                "{text}"
            );
            assert!(OpPath::new(text).is_ok(), "{text}");
        }
    }

    #[test]
    fn op_path_proposed_names_the_function() {
        let error = OpPath::proposed("/x", ApiFunction::Copy)
            .err()
            .map(|error| error.to_string());
        assert_eq!(error.as_deref(), Some("copy: path \"/x\" must be relative"));
    }

    #[test]
    fn op_path_new_returns_the_bare_shape() {
        assert_eq!(bare("."), Err("empty".to_owned()));
        assert_eq!(
            bare("../x"),
            Err("\"../x\" must stay inside the project".to_owned())
        );
        assert_eq!(
            bare("a\\b"),
            Err("\"a\\b\" must use / as the separator".to_owned())
        );
        assert_eq!(bare("/x"), Err("\"/x\" must be relative".to_owned()));
    }

    #[test]
    fn op_path_equal_after_normalising() -> TestResult {
        assert_eq!(OpPath::new("./a//b")?, OpPath::new("a/b")?);
        assert!(OpPath::new("A.md")? < OpPath::new("a.md")?);
        Ok(())
    }

    #[test]
    fn op_path_join_builds_root_paths() -> TestResult {
        let root = OpPath::new(".claude/agentstacks")?;
        let pack = root.join("core");
        assert_eq!(pack.as_str(), ".claude/agentstacks/core");
        assert_eq!(
            pack.join("hooks/enforce.lua").as_str(),
            ".claude/agentstacks/core/hooks/enforce.lua"
        );
        assert_eq!(
            root.join("translate.lua").as_str(),
            ".claude/agentstacks/translate.lua"
        );
        assert_eq!(pack.to_string(), ".claude/agentstacks/core");
        Ok(())
    }
}
