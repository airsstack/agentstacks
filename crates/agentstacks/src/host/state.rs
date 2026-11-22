//! What a run of one tool's engine remembers, shared with the Lua functions it installs.
//!
//! # Responsibilities
//!
//! - [`Phase`] is which handler the engine is running: the entry file, `values` or `plan`.
//! - [`RunState`] holds the tool's name, the phase, the refusals recorded so far, which stay
//!   even when the extension catches the Lua error that raised them, and the modules `require` is
//!   loading and has loaded.
//! - [`Loan`] is what the `plan` handler works on while it runs: the payload it was given, the
//!   proposals so far, what `read` can answer, and what `read` has answered. The session lends it
//!   to the state for one dispatch and takes it back.
//! - [`SharedState`] is the cheap-to-clone handle the installed Lua functions capture, because an
//!   installed function is `'static` and cannot borrow anything from the caller.
//!
//! # Non-responsibilities
//!
//! Deciding when a phase starts or ends belongs to the session that drives the engine.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, PoisonError};

use crate::host::files::LuaModule;
use crate::host::read::ReadSnapshot;
use crate::types::name::ToolName;
use crate::types::op::{ApiFunction, OpError, PlanInput, Proposals};
use crate::types::plan::Reads;
use crate::types::report::{Diagnostic, Diagnostics};

/// Which part of a run the engine is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    /// The entry file is running at the top level.
    Load,
    /// The `values` handler is running.
    Values,
    /// The `plan` handler is running.
    Plan,
}

/// What the `plan` handler works on while it runs.
#[derive(Debug)]
pub(super) struct Loan {
    /// The payload the handler was given, which the ids it copies back are looked up in.
    pub(super) input: PlanInput,
    /// The proposals of the run so far, which the handler adds to.
    pub(super) proposals: Proposals,
    /// What each declared `reads` path answers.
    pub(super) snapshot: ReadSnapshot,
    /// What the handler's `read` calls have returned.
    pub(super) reads: Reads,
}

/// The state of one tool's run.
#[derive(Debug)]
pub(super) struct RunState {
    tool: ToolName,
    phase: Phase,
    sticky: Diagnostics,
    loan: Option<Loan>,
    loading: Vec<LuaModule>,
    loaded: BTreeSet<LuaModule>,
}

impl RunState {
    /// The state of a run of `tool`, in the [`Phase::Load`] phase with no refusal recorded.
    const fn new(tool: ToolName) -> Self {
        Self {
            tool,
            phase: Phase::Load,
            sticky: Diagnostics::new(),
            loan: None,
            loading: Vec::new(),
            loaded: BTreeSet::new(),
        }
    }

    /// The tool whose extension runs.
    pub(super) const fn tool(&self) -> &ToolName {
        &self.tool
    }

    /// The phase the engine is in.
    #[cfg(test)]
    pub(super) const fn phase(&self) -> Phase {
        self.phase
    }

    /// Moves the engine to `phase`.
    pub(super) const fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
    }

    /// Records a refusal that holds even if the extension catches the error that raised it.
    pub(super) fn record(&mut self, refusal: Diagnostic) {
        self.sticky.push(refusal);
    }

    /// Hands back every refusal recorded so far and forgets them.
    pub(super) fn take_recorded(&mut self) -> Diagnostics {
        std::mem::take(&mut self.sticky)
    }

    /// Starts the `plan` handler's run on `loan`.
    pub(super) fn begin_plan(&mut self, loan: Loan) {
        self.loan = Some(loan);
        self.phase = Phase::Plan;
    }

    /// Ends the `plan` handler's run and takes back what it worked on. The engine is back in the
    /// [`Phase::Load`] phase.
    pub(super) const fn end_plan(&mut self) -> Option<Loan> {
        self.phase = Phase::Load;
        self.loan.take()
    }

    /// What the `plan` handler works on, for a call of `function`.
    ///
    /// # Errors
    ///
    /// [`OpError::OutsidePlanHandler`] unless the `plan` handler is running.
    pub(super) const fn plan_loan(&mut self, function: ApiFunction) -> Result<&mut Loan, OpError> {
        match (self.phase, self.loan.as_mut()) {
            (Phase::Plan, Some(loan)) => Ok(loan),
            _ => Err(OpError::OutsidePlanHandler { function }),
        }
    }

    /// The modules being loaded, outermost first.
    pub(super) fn loading(&self) -> &[LuaModule] {
        &self.loading
    }

    /// Starts loading `module`.
    pub(super) fn start_loading(&mut self, module: LuaModule) {
        self.loading.push(module);
    }

    /// Stops loading the module that was started last.
    pub(super) fn stop_loading(&mut self) {
        self.loading.pop();
    }

    /// Whether `module` has finished loading.
    pub(super) fn is_loaded(&self, module: &LuaModule) -> bool {
        self.loaded.contains(module)
    }

    /// Records that `module` has finished loading.
    pub(super) fn mark_loaded(&mut self, module: LuaModule) {
        self.loaded.insert(module);
    }
}

/// A [`RunState`] shared between the session and the Lua functions of the engine.
#[derive(Debug, Clone)]
pub(super) struct SharedState(Arc<Mutex<RunState>>);

impl SharedState {
    /// A fresh state for a run of `tool`.
    pub(super) fn new(tool: ToolName) -> Self {
        Self(Arc::new(Mutex::new(RunState::new(tool))))
    }

    /// Runs `f` on the state. A panic in an earlier holder leaves the state usable, so a poisoned
    /// lock is taken over rather than refused.
    pub(super) fn with<R>(&self, f: impl FnOnce(&mut RunState) -> R) -> R {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        f(&mut state)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;
    use crate::host::testing::plan_input;
    use crate::types::report::{FieldError, Location};

    type TestResult = Result<(), Box<dyn Error>>;

    fn shared() -> Result<SharedState, Box<dyn Error>> {
        Ok(SharedState::new(ToolName::new("claude")?))
    }

    #[test]
    fn a_run_starts_in_the_load_phase_with_nothing_recorded() -> TestResult {
        let state = shared()?;
        assert_eq!(state.with(|run| run.phase()), Phase::Load);
        assert_eq!(state.with(|run| run.tool().to_string()), "claude");
        assert!(state.with(RunState::take_recorded).is_empty());
        Ok(())
    }

    #[test]
    fn a_phase_change_is_seen_through_every_clone() -> TestResult {
        let state = shared()?;
        let other = state.clone();
        for phase in [Phase::Values, Phase::Plan, Phase::Load] {
            state.with(|run| run.set_phase(phase));
            assert_eq!(other.with(|run| run.phase()), phase);
        }
        Ok(())
    }

    #[test]
    fn recorded_refusals_are_handed_back_once() -> TestResult {
        let state = shared()?;
        state.with(|run| {
            run.record(Diagnostic::new(Location::file("a"), FieldError::Required));
        });
        assert_eq!(state.with(RunState::take_recorded).len(), 1);
        assert!(state.with(RunState::take_recorded).is_empty());
        Ok(())
    }

    fn loan() -> Result<Loan, Box<dyn Error>> {
        Ok(Loan {
            input: plan_input()?,
            proposals: Proposals::new(),
            snapshot: ReadSnapshot::default(),
            reads: Reads::default(),
        })
    }

    #[test]
    fn the_loan_is_refused_outside_the_plan_phase() -> TestResult {
        let state = shared()?;
        let refused = state.with(|run| {
            run.plan_loan(ApiFunction::Skip)
                .err()
                .map(|error| error.to_string())
        });
        assert_eq!(
            refused.as_deref(),
            Some("skip: only allowed inside the plan handler")
        );
        Ok(())
    }

    #[test]
    fn a_plan_phase_lends_the_loan_and_takes_it_back() -> TestResult {
        let state = shared()?;
        let lent = loan()?;
        state.with(|run| run.begin_plan(lent));
        assert_eq!(state.with(|run| run.phase()), Phase::Plan);
        assert!(state.with(|run| run.plan_loan(ApiFunction::Write).is_ok()));
        assert!(state.with(RunState::end_plan).is_some());
        assert_eq!(state.with(|run| run.phase()), Phase::Load);
        assert!(state.with(|run| run.plan_loan(ApiFunction::Write).is_err()));
        assert!(state.with(RunState::end_plan).is_none());
        Ok(())
    }

    #[test]
    fn modules_are_loading_until_stopped_and_loaded_once_marked() -> TestResult {
        let state = shared()?;
        let util = LuaModule::new("lib.util").ok_or("a module name")?;
        let other = LuaModule::new("lib.other").ok_or("a module name")?;
        state.with(|run| {
            run.start_loading(util.clone());
            run.start_loading(other.clone());
        });
        assert_eq!(
            state.with(|run| run.loading().to_vec()),
            [util.clone(), other.clone()]
        );
        state.with(RunState::stop_loading);
        assert_eq!(
            state.with(|run| run.loading().to_vec()),
            std::slice::from_ref(&util)
        );
        assert!(!state.with(|run| run.is_loaded(&util)));
        state.with(|run| run.mark_loaded(util.clone()));
        assert!(state.with(|run| run.is_loaded(&util)));
        assert!(!state.with(|run| run.is_loaded(&other)));
        Ok(())
    }
}
