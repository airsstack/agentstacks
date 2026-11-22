//! The shape refusal of a project path, with neither location nor field.
//!
//! An extension names a project path in a call (`write`'s `path`) and in the `values` it returns
//! (`values.root`). Both are refused in the same four ways, so the four wordings exist once here,
//! and the refusal that carries one adds only the function and field, or the key path.
//!
//! # Responsibilities
//!
//! - [`PathShape`] displays `empty`, `"<path>" must be relative`,
//!   `"<path>" must stay inside the project` or `"<path>" must use / as the separator`.
//!
//! # Non-responsibilities
//!
//! Deciding which shape a path has belongs to the path type that constructs this error.

use crate::types::report::MustBeRelative;
use crate::types::report::OutsideProject;
use crate::types::report::Quoted;

/// Why a project path is refused.
///
/// # Examples
///
/// ```
/// use agentstacks::types::op::PathShape;
///
/// assert_eq!(PathShape::Empty.to_string(), "empty");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathShape {
    /// The path is empty, or normalises to nothing.
    #[error("empty")]
    Empty,
    /// The path is absolute.
    #[error(transparent)]
    MustBeRelative(MustBeRelative),
    /// The path climbs out of the project.
    #[error(transparent)]
    OutsideProject(OutsideProject),
    /// The path uses a separator other than `/`.
    #[error("{0} must use / as the separator")]
    Separator(Quoted),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_shape_empty() {
        assert_eq!(PathShape::Empty.to_string(), "empty");
    }

    #[test]
    fn path_shape_must_be_relative() {
        let shape = PathShape::MustBeRelative(MustBeRelative::new("/x"));
        assert_eq!(shape.to_string(), "\"/x\" must be relative");
    }

    #[test]
    fn path_shape_outside_project() {
        let shape = PathShape::OutsideProject(OutsideProject::new("../x"));
        assert_eq!(shape.to_string(), "\"../x\" must stay inside the project");
    }

    #[test]
    fn path_shape_separator() {
        let shape = PathShape::Separator(Quoted::new("a\\b"));
        assert_eq!(shape.to_string(), "\"a\\b\" must use / as the separator");
    }
}
