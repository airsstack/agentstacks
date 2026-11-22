//! The engine port, implemented by running each tool's extension in the airsl sandbox.
//!
//! # Responsibilities
//!
//! - [`AirslAdapter`] implements [`Adapter`]. Its [`Session`] is one extension's engine, so the
//!   entry file runs once, in [`Adapter::open`], and the handlers it registered serve the
//!   `values` and then the `plan` call.
//!
//! # Non-responsibilities
//!
//! Everything a call does belongs to the session. The adapter holds no state of its own, so one
//! value serves every tool.

use crate::host::session::Session;
use crate::port::{Adapter, FileTree, Workspace};
use crate::types::extension::Extension;
use crate::types::op::{PlanInput, Proposals, ValuesInput, ValuesResult};
use crate::types::plan::Reads;
use crate::types::report::Diagnostics;

/// Runs tool extensions in the airsl sandbox, with no grants.
///
/// An extension only proposes: it has no file, network or process access, and this adapter never
/// writes. Opening an extension reads its `.lua` files into memory and runs its entry file, and
/// every refusal found along the way is returned together.
///
/// # Examples
///
/// ```
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::host::AirslAdapter;
/// use agentstacks::port::Adapter;
/// use agentstacks::types::op::ValuesInput;
/// use agentstacks::types::{Extension, ToolName};
///
/// let dir = concat!(
///     env!("CARGO_MANIFEST_DIR"),
///     "/tests/fixtures/extensions/valid/minimal"
/// );
/// let tree = StdFileTree::new(dir);
/// let extension = Extension::load(&tree).map_err(|found| format!("{found:?}"))?;
///
/// // The entry file of this extension is `return {}`: it registers no handler.
/// let adapter = AirslAdapter::new();
/// let mut session = adapter
///     .open(&extension, &tree)
///     .map_err(|found| format!("{found:?}"))?;
///
/// let input = ValuesInput {
///     tool: ToolName::new("claude")?,
///     packs: Vec::new(),
///     vars: Vec::new(),
/// };
/// let refused: Vec<String> = adapter
///     .values(&mut session, &input)
///     .err()
///     .map(|found| found.into_iter().map(|d| d.to_string()).collect())
///     .unwrap_or_default();
/// assert_eq!(refused, ["claude/adapter.lua: registers no values handler"]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct AirslAdapter;

impl AirslAdapter {
    /// An adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Adapter for AirslAdapter {
    type Session = Session;

    fn open<T: FileTree>(
        &self,
        extension: &Extension,
        files: &T,
    ) -> Result<Self::Session, Diagnostics> {
        Session::open(extension, files)
    }

    fn values(
        &self,
        session: &mut Self::Session,
        input: &ValuesInput,
    ) -> Result<ValuesResult, Diagnostics> {
        session.values(input)
    }

    fn plan<W: Workspace>(
        &self,
        session: &mut Self::Session,
        input: &PlanInput,
        workspace: &W,
        proposals: &mut Proposals,
    ) -> Result<Reads, Diagnostics> {
        session.plan(input, workspace, proposals)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::host::testing::{extension, plan_input, values_input};
    use crate::port::FakeWorkspace;

    type TestResult = Result<(), Box<dyn Error>>;

    /// Opens, asks for the values and plans, all through the port.
    fn drive<A: Adapter>(
        adapter: &A,
        entry: &str,
    ) -> Result<(Result<ValuesResult, Diagnostics>, usize), Box<dyn Error>> {
        let (loaded, tree) = extension(entry, &[], &[])?;
        let mut session = adapter
            .open(&loaded, &tree)
            .map_err(|found| format!("{found:?}"))?;
        let values = adapter.values(&mut session, &values_input()?);
        let mut proposals = Proposals::new();
        adapter
            .plan(
                &mut session,
                &plan_input()?,
                &FakeWorkspace::new(),
                &mut proposals,
            )
            .map_err(|found| format!("{found:?}"))?;
        Ok((values, proposals.iter().count()))
    }

    #[test]
    fn the_port_runs_open_values_and_plan_on_one_engine() -> TestResult {
        let (values, proposed) = drive(
            &AirslAdapter::new(),
            "agentstacks.on('values', function() return { root = '.x' } end)\nagentstacks.on('plan', function()\n  agentstacks.require_binary{name='git'}\nend)",
        )?;
        let values = values.map_err(|found| format!("{found:?}"))?;
        assert_eq!(values.root().as_str(), ".x");
        assert_eq!(proposed, 1);
        Ok(())
    }

    #[test]
    fn one_adapter_serves_two_tools() -> TestResult {
        let adapter = AirslAdapter::new();
        let entries = [
            "leaked = 1\nagentstacks.on('plan', function() end)",
            "agentstacks.on('plan', function() end)",
        ];
        for entry in entries {
            let (values, proposed) = drive(&adapter, entry)?;
            assert!(values.is_err(), "no values handler is registered");
            assert_eq!(proposed, 0);
        }
        Ok(())
    }
}
