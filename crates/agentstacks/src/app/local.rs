//! A directory of the project, opened as a source.
//!
//! # Responsibilities
//!
//! - [`local_tree`] opens the directory a path source names and refuses a directory that leads
//!   outside the project, or that cannot be looked at, with a diagnostic placed where the author
//!   can find it.
//!
//! # Non-responsibilities
//!
//! What the directory holds, a pack or an extension, is read by the loader the flow calls next.

use crate::port::Workspace;
use crate::types::plan::StepError;
use crate::types::project::LocalPath;
use crate::types::report::{Diagnostic, Location};

/// The files under `path`, as the workspace serves them.
///
/// A directory that leads outside the project is refused at `at`, the argument that named it. An
/// I/O failure of looking at it is located at the directory itself.
pub(super) fn local_tree<W: Workspace>(
    workspace: &W,
    path: &LocalPath,
    at: Location,
) -> Result<W::Tree, Diagnostic> {
    match workspace.source_tree(path) {
        Ok(Some(tree)) => Ok(tree),
        Ok(None) => Err(Diagnostic::new(at, StepError::Outside)),
        Err(error) => Err(Diagnostic::new(Location::file(path.as_str()), error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::{FakeFileTree, FakeWorkspace, WorkspaceCall};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn shown(refusal: &Diagnostic) -> String {
        refusal.to_string()
    }

    #[test]
    fn a_directory_of_the_project_is_opened() -> TestResult {
        let dir = LocalPath::new("ext/x")?;
        let workspace = FakeWorkspace::new().source(&dir, FakeFileTree::new("ext/x"));
        assert!(local_tree(&workspace, &dir, Location::verbatim("--tool \"ext/x\"")).is_ok());
        Ok(())
    }

    #[test]
    fn a_directory_outside_the_project_is_refused_at_the_argument() -> TestResult {
        let dir = LocalPath::new("ext/link")?;
        let workspace = FakeWorkspace::new().source_outside(&dir);
        let refusal = local_tree(&workspace, &dir, Location::verbatim("--tool \"ext/link\""))
            .err()
            .ok_or("an outside directory must be refused")?;
        assert_eq!(
            shown(&refusal),
            format!("--tool \"ext/link\": {}", StepError::Outside)
        );
        Ok(())
    }

    #[test]
    fn an_io_failure_is_located_at_the_directory() -> TestResult {
        let dir = LocalPath::new("ext/x")?;
        let workspace = FakeWorkspace::new().fail(WorkspaceCall::SourceTree(dir.clone()));
        let refusal = local_tree(&workspace, &dir, Location::verbatim("--tool \"ext/x\""))
            .err()
            .ok_or("an I/O failure must be refused")?;
        assert_eq!(
            shown(&refusal),
            format!("ext/x: {}", FakeWorkspace::denied())
        );
        Ok(())
    }
}
