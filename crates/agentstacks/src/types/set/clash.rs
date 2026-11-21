//! The rule that no two packs ship a skill or an agent of the same name.
//!
//! It is a set-level rule because a name is unique within one pack by the layout but only a whole
//! set can show two packs sharing one.
//!
//! # Responsibilities
//!
//! - `check_name_clashes` refuses each skill name that two or more loaded packs ship, and each
//!   agent name likewise, once per name, listing the packs in name order. The refusal is located
//!   at the project file alone.
//! - A skill and an agent may share a name, so skills and agents are compared separately.
//!
//! # Non-responsibilities
//!
//! Names within one pack are unique by the layout, which is checked when the pack is loaded.

use std::collections::BTreeMap;
use std::path::Path;

use crate::types::name::ItemName;
use crate::types::name::PackName;
use crate::types::pack::ItemKind;
use crate::types::pack::Pack;
use crate::types::report::Diagnostic;
use crate::types::report::Location;
use crate::types::set::error::SetError;

/// The packs that ship each item name, in pack name order.
type Shippers<'a> = BTreeMap<&'a ItemName, Vec<&'a PackName>>;

/// Refuses each skill and each agent name that two or more of the loaded `packs` ship, in the
/// project file at `project_file`. `packs` are keyed by their `[packs.*]` name.
pub(super) fn check_name_clashes(
    project_file: &Path,
    packs: &BTreeMap<PackName, Pack>,
) -> Vec<Diagnostic> {
    let mut skills = Shippers::new();
    let mut agents = Shippers::new();
    for (key, pack) in packs {
        for skill in pack.skills() {
            skills.entry(skill.name()).or_default().push(key);
        }
        for agent in pack.agents() {
            agents.entry(agent.name()).or_default().push(key);
        }
    }
    let mut refusals = Vec::new();
    for (kind, shipped) in [(ItemKind::Skill, skills), (ItemKind::Agent, agents)] {
        for (name, shippers) in shipped {
            if shippers.len() > 1 {
                refusals.push(Diagnostic::new(
                    Location::set_level(project_file, None),
                    SetError::NameClash {
                        kind,
                        name: name.clone(),
                        packs: shippers.into_iter().cloned().collect(),
                    },
                ));
            }
        }
    }
    refusals
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::set::testing::{PROJECT_FILE, PackSpec, loaded, shown};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    fn found(specs: &[PackSpec]) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(shown(&check_name_clashes(
            Path::new(PROJECT_FILE),
            &loaded(specs)?,
        )))
    }

    fn skill(pack: &str, name: &str) -> PackSpec {
        PackSpec::new(pack).skill(name, "body")
    }

    fn agent(pack: &str, name: &str) -> PackSpec {
        PackSpec::new(pack).agent(name, "body")
    }

    #[test]
    fn same_skill_in_two_packs_is_refused() -> TestResult {
        assert_eq!(
            found(&[skill("sdlc", "status"), skill("team-lint", "status")])?,
            ["agentstacks.toml: skill \"status\" is shipped by both sdlc and team-lint"]
        );
        Ok(())
    }

    #[test]
    fn same_agent_in_two_packs_is_refused() -> TestResult {
        assert_eq!(
            found(&[agent("core", "coder"), agent("team-lint", "coder")])?,
            ["agentstacks.toml: agent \"coder\" is shipped by both core and team-lint"]
        );
        Ok(())
    }

    #[test]
    fn skill_and_agent_may_share_a_name_across_packs() -> TestResult {
        assert!(found(&[skill("a", "x"), agent("b", "x")])?.is_empty());
        Ok(())
    }

    #[test]
    fn three_or_more_packs_give_one_diagnostic_per_name() -> TestResult {
        assert_eq!(
            found(&[
                skill("x", "status"),
                skill("team-lint", "status"),
                skill("sdlc", "status")
            ])?,
            ["agentstacks.toml: skill \"status\" is shipped by sdlc, team-lint and x"]
        );
        assert_eq!(
            found(&[
                agent("d", "worker"),
                agent("a", "worker"),
                agent("c", "worker"),
                agent("b", "worker")
            ])?,
            ["agentstacks.toml: agent \"worker\" is shipped by a, b, c and d"]
        );
        Ok(())
    }
}
