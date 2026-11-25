//! Step 7 of the plan flow: looking up the binaries the plan needs and the installed `airsl`.
//!
//! # Responsibilities
//!
//! - [`check_binaries`] asks the [`Binary`] port about every requirement `collect_binaries` lists:
//!   each pack's `requires.binaries`, then each tool's `require_binary` calls. A binary that is
//!   found becomes a [`BinaryCheck`]; one that is not is refused once for each requirer.
//! - When a pack declares `requires.airsl`, it asks the port what `airsl --version` printed, and
//!   refuses an answer that is missing, unreadable or outside the packs' range. The installed
//!   version it reads is kept for the lock.
//! - [`Binaries`] is the flow's carrier for the checks, the installed version, the pin and the
//!   refusals.
//!
//! # Non-responsibilities
//!
//! Which binaries are required, how the version is read out of the printed text and whether it is
//! in range are rules in `types`. Searching `PATH` and running `airsl` belong to the adapter.

use std::path::Path;

use crate::app::plan::load::Loaded;
use crate::app::plan::read::Inputs;
use crate::app::plan::tools::Ran;
use crate::port::Binary;
use crate::types::manifest::PackVersion;
use crate::types::name::ToolName;
use crate::types::op::{
    BinaryRequirement, Origin, airsl_not_found, airsl_unparseable, collect_binaries,
    parse_airsl_version,
};
use crate::types::pack::Pack;
use crate::types::plan::{AirslAnswer, AirslPin, BinaryCheck};
use crate::types::project::PROJECT_FILE;
use crate::types::report::Diagnostics;

/// What looking up the binaries found.
pub(super) struct Binaries {
    /// The requirements that were found on `PATH`.
    pub(super) checks: Vec<BinaryCheck>,
    /// The installed `airsl` version, when it was asked for and read.
    pub(super) installed: Option<PackVersion>,
    /// The version the lock pins and the one installed, when `airsl` is needed.
    pub(super) pin: Option<AirslPin>,
    /// Every requirement that was not met.
    pub(super) refusals: Diagnostics,
}

/// Looks up every binary the packs and the extensions asked for, and the installed `airsl` when a
/// pack declares a range for it.
pub(super) fn check_binaries<T, B: Binary>(
    inputs: &Inputs,
    loaded: &Loaded<T>,
    ran: &Ran,
    binary: &B,
) -> Binaries {
    let project_file = Path::new(PROJECT_FILE);
    let packs: Vec<&Pack> = loaded.set.packs().values().collect();
    let tools: Vec<ToolName> = loaded.set.extensions().keys().cloned().collect();
    let mut found = Binaries {
        checks: Vec::new(),
        installed: None,
        pin: None,
        refusals: Diagnostics::new(),
    };
    for requirement in collect_binaries(&packs, &ran.proposals, &tools) {
        if binary.on_path(requirement.name()) {
            let origin = match &requirement {
                BinaryRequirement::Pack { pack, .. } => Origin::Core { pack: pack.clone() },
                BinaryRequirement::Extension { origin, .. } => origin.clone(),
            };
            found.checks.push(BinaryCheck {
                name: requirement.name().clone(),
                origin,
            });
        } else {
            found.refusals.push(requirement.not_found(project_file));
        }
    }

    let Some(range) = loaded.set.airsl() else {
        return found;
    };
    match binary.airsl_version() {
        AirslAnswer::NotFound => found.refusals.push(airsl_not_found(project_file, range)),
        AirslAnswer::Failed(text) => found.refusals.push(airsl_unparseable(project_file, &text)),
        AirslAnswer::Printed(text) => match parse_airsl_version(&text) {
            None => found.refusals.push(airsl_unparseable(project_file, &text)),
            Some(version) => {
                found
                    .refusals
                    .extend(range.installed_refusal(project_file, &version));
                found.installed = Some(version);
            }
        },
        spawn @ AirslAnswer::SpawnFailed(_) => found.refusals.extend(spawn.spawn_refusal()),
    }
    found.pin = found.installed.clone().map(|installed| AirslPin {
        locked: inputs.lock.as_ref().and_then(|lock| lock.airsl().cloned()),
        installed,
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::plan::read::read_inputs;
    use crate::app::plan::testing::{
        LoadedTrees, Scenario, TestResult, ext_files, loaded, lock_text, origin, shown,
    };
    use crate::port::{BinaryCall, FakeBinary, FakeFileTree};
    use crate::types::lock::LockContents;
    use crate::types::name::BinaryName;
    use crate::types::op::{Operation, Proposal, Proposals};

    const ONE_PACK: &str = "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\npath = \"packs/core\"\n";
    const TWO_PACKS: &str = "api = 1\n\n[tools.claude]\npath = \"extensions/claude\"\n\n[packs.core]\npath = \"packs/core\"\n\n[packs.sdlc]\npath = \"packs/sdlc\"\n";

    fn pack_with(name: &str, requires: &str) -> FakeFileTree {
        FakeFileTree::new(format!("packs/{name}"))
            .file(
                "pack.toml",
                format!(
                    "[pack]\nname = \"{name}\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\n[requires]\n{requires}"
                )
                .as_bytes(),
            )
            .file(
                format!("skills/{name}/SKILL.md"),
                format!("---\nname: {name}\ndescription: d\n---\nBody\n").as_bytes(),
            )
    }

    fn scenario(
        project: &str,
        packs: &[(&str, &str)],
    ) -> Result<Scenario, Box<dyn std::error::Error>> {
        let mut scenario = Scenario::new(project).path_source(
            "extensions/claude",
            ext_files("extensions/claude", "claude"),
        )?;
        for (name, requires) in packs {
            scenario = scenario.path_source(&format!("packs/{name}"), pack_with(name, requires))?;
        }
        Ok(scenario)
    }

    /// A run in which the extension of `claude` required each of `names`, one call at each line
    /// from 7.
    fn ran_requiring(names: &[&str]) -> Result<Ran, Box<dyn std::error::Error>> {
        let mut proposals = Proposals::new();
        for (index, name) in names.iter().enumerate() {
            proposals.propose(Proposal {
                op: Operation::require_binary(name)?,
                origin: origin("claude", 7 + index)?,
            })?;
        }
        Ok(Ran {
            proposals,
            runs: std::collections::BTreeMap::new(),
            skips: Vec::new(),
            files: std::collections::BTreeMap::new(),
            refusals: Diagnostics::new(),
        })
    }

    fn check(
        scenario: &Scenario,
        ran: &Ran,
        binary: &FakeBinary,
    ) -> Result<(Binaries, LoadedTrees), Box<dyn std::error::Error>> {
        let inputs = read_inputs(&scenario.workspace).map_err(|found| shown(found).join("; "))?;
        let loaded = loaded(scenario).map_err(|found| shown(found).join("; "))?;
        let found = check_binaries(&inputs, &loaded, ran, binary);
        Ok((found, loaded))
    }

    fn name(text: &str) -> Result<BinaryName, Box<dyn std::error::Error>> {
        Ok(BinaryName::new(text)?)
    }

    #[test]
    fn check_binaries_refuses_a_missing_pack_binary() -> TestResult {
        let scenario = scenario(ONE_PACK, &[("core", "binaries = [\"rg\"]\n")])?;
        let (found, _) = check(&scenario, &ran_requiring(&[])?, &FakeBinary::new())?;
        assert_eq!(
            shown(found.refusals),
            ["agentstacks.toml: packs.core: binary \"rg\" not found on PATH"]
        );
        Ok(())
    }

    #[test]
    fn check_binaries_refuses_a_missing_extension_binary() -> TestResult {
        let scenario = scenario(ONE_PACK, &[("core", "")])?;
        let (found, _) = check(&scenario, &ran_requiring(&["node"])?, &FakeBinary::new())?;
        assert_eq!(
            shown(found.refusals),
            ["claude/adapter.lua:7: require_binary: \"node\" not found on PATH"]
        );
        Ok(())
    }

    #[test]
    fn check_binaries_reports_a_binary_once_for_the_pack_and_once_for_the_extension() -> TestResult
    {
        let scenario = scenario(ONE_PACK, &[("core", "binaries = [\"rg\"]\n")])?;
        let (found, _) = check(&scenario, &ran_requiring(&["rg"])?, &FakeBinary::new())?;
        assert_eq!(
            shown(found.refusals),
            [
                "agentstacks.toml: packs.core: binary \"rg\" not found on PATH",
                "claude/adapter.lua:7: require_binary: \"rg\" not found on PATH"
            ]
        );
        Ok(())
    }

    #[test]
    fn check_binaries_checks_a_repeated_name_once() -> TestResult {
        let scenario = scenario(ONE_PACK, &[("core", "")])?;
        let binary = FakeBinary::new().with(name("node")?);
        let (found, _) = check(&scenario, &ran_requiring(&["node", "node"])?, &binary)?;
        assert!(found.refusals.is_empty());
        assert_eq!(found.checks.len(), 1);
        assert_eq!(binary.asked(), [BinaryCall::OnPath(name("node")?)]);
        Ok(())
    }

    #[test]
    fn check_binaries_refuses_airsl_not_on_path() -> TestResult {
        let scenario = scenario(
            TWO_PACKS,
            &[
                ("core", "airsl = \">=0.1.4\"\n"),
                ("sdlc", "airsl = \">=0.1.4\"\n"),
            ],
        )?;
        let (found, _) = check(&scenario, &ran_requiring(&[])?, &FakeBinary::new())?;
        assert_eq!(
            shown(found.refusals),
            ["agentstacks.toml: airsl not found on PATH (needed by core, sdlc)"]
        );
        Ok(())
    }

    #[test]
    fn check_binaries_refuses_unparseable_airsl_version_text() -> TestResult {
        let scenario = scenario(ONE_PACK, &[("core", "airsl = \">=0.1.4\"\n")])?;
        let binary = FakeBinary::new().airsl("airsl-dev\n");
        let (found, _) = check(&scenario, &ran_requiring(&[])?, &binary)?;
        assert_eq!(
            shown(found.refusals),
            [
                "agentstacks.toml: airsl --version printed \"airsl-dev\", expected \"airsl <version>\""
            ]
        );
        Ok(())
    }

    #[test]
    fn check_binaries_refuses_airsl_out_of_range() -> TestResult {
        let scenario = scenario(ONE_PACK, &[("core", "airsl = \">=0.1.4\"\n")])?;
        let binary = FakeBinary::new().airsl("airsl 0.1.3");
        let (found, _) = check(&scenario, &ran_requiring(&[])?, &binary)?;
        assert_eq!(
            shown(found.refusals),
            ["agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4"]
        );
        Ok(())
    }

    #[test]
    fn check_binaries_moves_the_pin() -> TestResult {
        let lock = lock_text(LockContents {
            airsl: Some(PackVersion::new("0.1.4")?),
            ..LockContents::default()
        })?;
        let scenario = scenario(ONE_PACK, &[("core", "airsl = \">=0.1.4\"\n")])?.lock(&lock);
        let binary = FakeBinary::new().airsl("airsl 0.1.5");
        let (found, _) = check(&scenario, &ran_requiring(&[])?, &binary)?;
        assert!(found.refusals.is_empty());
        assert_eq!(found.installed, Some(PackVersion::new("0.1.5")?));
        assert_eq!(
            found.pin,
            Some(AirslPin {
                locked: Some(PackVersion::new("0.1.4")?),
                installed: PackVersion::new("0.1.5")?
            })
        );
        Ok(())
    }

    #[test]
    fn check_binaries_skips_airsl_when_no_pack_declares_it() -> TestResult {
        let scenario = scenario(ONE_PACK, &[("core", "")])?;
        let binary = FakeBinary::new().airsl("airsl 0.1.5");
        let (found, _) = check(&scenario, &ran_requiring(&[])?, &binary)?;
        assert!(found.pin.is_none());
        assert!(!binary.asked().contains(&BinaryCall::AirslVersion));
        Ok(())
    }
}
