//! Checking a whole project: the one entry point that turns loaded parts into a [`ValidSet`].
//!
//! It is the only function that builds a [`ValidSet`], and it drives the loading itself, so a
//! checked set can never be assembled from entries that were left unloaded or hand-picked.
//!
//! # Responsibilities
//!
//! - [`check`] takes the parsed project file and two loaders, one for packs and one for tools. It
//!   calls the loader exactly once for each pack and tool the project lists, runs every whole-set
//!   check, and returns a [`ValidSet`] or every refusal in one [`Diagnostics`]: the load refusals
//!   of each entry that failed together with those of the checks.
//! - The checks run in this order: fetched names, requirements and cycles, token targets and
//!   option tokens, hook `run`s into other packs, option values, airsl ranges.
//! - Every check reads only the entries that loaded, so an entry that failed to load causes no
//!   second refusal, while every refusal that does not depend on it is still reported.
//! - No [`ValidSet`] is returned when any listed entry failed to load.
//!
//! # Non-responsibilities
//!
//! Loading the entries, and fetching their sources, happen before and elsewhere. Probing the
//! installed airsl belongs to the stage that plans an install.

use std::collections::BTreeMap;
use std::path::Path;

use crate::types::extension::Extension;
use crate::types::name::PackName;
use crate::types::name::ToolName;
use crate::types::pack::Pack;
use crate::types::project::PackEntry;
use crate::types::project::ProjectFile;
use crate::types::project::Source;
use crate::types::report::Diagnostics;
use crate::types::set::airsl::AirslRange;
use crate::types::set::fetched::check_fetched_names;
use crate::types::set::graph::check_requires;
use crate::types::set::options::resolve_options;
use crate::types::set::runs::check_hook_runs;
use crate::types::set::targets::check_token_targets;
use crate::types::set::valid::ValidSet;

/// Validates the project `project`, read from the file `project_file`, as a whole.
///
/// `load_pack` is called once for each `[packs.*]` entry, with its key and its entry, and
/// `load_tool` once for each `[tools.*]` entry, with its key and its source. Each returns what
/// loading that entry gave. Because the check drives the loading, no listed entry can be left
/// without a load result.
///
/// # Errors
///
/// Returns every refusal when any entry failed to load or any check refused;
/// [`Diagnostics::into_sorted`] gives them in output order.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::fs::StdFileTree;
/// use agentstacks::types::{Extension, Pack, ProjectFile, check_pack_set};
///
/// let fixtures = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
/// let text = "api = 0\n\
///     [tools.claude]\n\
///     path = \"extensions/claude\"\n\
///     [packs.core]\n\
///     path = \"packs/core\"\n";
/// let project = ProjectFile::parse(Path::new("agentstacks.toml"), text)
///     .map_err(|refusals| format!("{refusals:?}"))?;
///
/// let set = check_pack_set(
///     Path::new("agentstacks.toml"),
///     project,
///     |_key, _entry| Pack::load(&StdFileTree::new(format!("{fixtures}/packs/valid-hooks"))),
///     |_key, _source| {
///         Extension::load(&StdFileTree::new(format!("{fixtures}/extensions/valid/minimal")))
///     },
/// )
/// .map_err(|refusals| format!("{refusals:?}"))?;
/// assert_eq!(set.packs().len(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn check<LoadPack, LoadTool>(
    project_file: &Path,
    project: ProjectFile,
    mut load_pack: LoadPack,
    mut load_tool: LoadTool,
) -> Result<ValidSet, Diagnostics>
where
    LoadPack: FnMut(&PackName, &PackEntry) -> Result<Pack, Diagnostics>,
    LoadTool: FnMut(&ToolName, &Source) -> Result<Extension, Diagnostics>,
{
    let mut diagnostics = Diagnostics::new();
    let (loaded_packs, all_packs) = load_all(project.packs(), &mut load_pack, &mut diagnostics);
    let (loaded_tools, all_tools) = load_all(project.tools(), &mut load_tool, &mut diagnostics);

    diagnostics.extend(check_fetched_names(
        project_file,
        &loaded_packs,
        &loaded_tools,
    ));
    diagnostics.extend(check_requires(project_file, &project, &loaded_packs));
    diagnostics.extend(check_token_targets(&loaded_packs));
    diagnostics.extend(check_hook_runs(&loaded_packs));
    let mut options = BTreeMap::new();
    for (key, pack) in &loaded_packs {
        let Some(entry) = project.packs().get(key) else {
            continue;
        };
        match resolve_options(
            project_file,
            key,
            pack.manifest().options(),
            entry.options(),
        ) {
            Ok(resolved) => {
                options.insert(key.clone(), resolved);
            }
            Err(refusals) => diagnostics.extend(refusals),
        }
    }
    let airsl = AirslRange::of_packs(&loaded_packs);
    diagnostics.extend(
        airsl
            .as_ref()
            .and_then(|range| range.disjoint_refusal(project_file)),
    );

    if !(all_packs && all_tools && diagnostics.is_empty()) {
        return Err(diagnostics);
    }
    Ok(ValidSet::new(
        project,
        loaded_packs,
        loaded_tools,
        options,
        airsl,
    ))
}

/// Calls `load` once for each of `listed`, in key order, and moves the entries that loaded into a
/// new map. The refusals of the entries that failed are added to `diagnostics`. The flag is `false`
/// when any entry failed to load.
fn load_all<K: Ord + Clone, E, T>(
    listed: &BTreeMap<K, E>,
    load: &mut impl FnMut(&K, &E) -> Result<T, Diagnostics>,
    diagnostics: &mut Diagnostics,
) -> (BTreeMap<K, T>, bool) {
    let mut loaded = BTreeMap::new();
    let mut all = true;
    for (key, listing) in listed {
        match load(key, listing) {
            Ok(entry) => {
                loaded.insert(key.clone(), entry);
            }
            Err(refusals) => {
                all = false;
                diagnostics.extend(refusals);
            }
        }
    }
    (loaded, all)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::project::LocalPath;
    use crate::types::set::testing::{
        PROJECT_FILE, PackSpec, extension_api, project, run_check, run_check_with, shown,
    };
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    const API_ONE: &str =
        "[pack]\nname = \"core\"\nversion = \"0.1.0\"\napi = 1\ndescription = \"d\"\n";
    const CORE_API: &str =
        "packs/core/pack.toml: core: api 1 not supported (this agentstacks speaks 0)";

    /// A `core` whose `pack.toml` asks for api 1, so it fails to load.
    fn failing_core() -> PackSpec {
        PackSpec::new("core")
            .skill("status", "body")
            .file("pack.toml", API_ONE)
    }

    fn refusals(packs: &[(PackSpec, &str)]) -> Result<Vec<String>, Box<dyn Error>> {
        run_check(packs, &["claude"])?
            .err()
            .ok_or_else(|| "the check passed, expected refusals".into())
    }

    #[test]
    fn check_runs_every_check_and_sorts_the_result() -> TestResult {
        let kore = PackSpec::new("kore")
            .listed_as("core")
            .required_string_option("x");
        let sdlc = PackSpec::new("sdlc")
            .requires(&["core", "gone"])
            .airsl("<0.2")
            .skill("status", "{{agent core/codr}}\n{{option nope}}");
        let team_lint = PackSpec::new("team-lint")
            .requires(&["core"])
            .airsl(">=0.2")
            .skill("status", "body")
            .tables(
                "[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"{{root core}}/hooks/nope.lua\"\n",
            );
        let found = refusals(&[
            (PackSpec::new("a").requires(&["b"]), ""),
            (PackSpec::new("b").requires(&["a"]), ""),
            (kore, ""),
            (sdlc, ""),
            (team_lint, ""),
        ])?;
        assert_eq!(
            found,
            [
                "agentstacks.toml: packs.core: source contains pack \"kore\"",
                "agentstacks.toml: packs.sdlc: sdlc requires pack \"gone\", which agentstacks.toml does not list (add it: --pack gone)",
                "agentstacks.toml: requires cycle: a → b → a",
                "agentstacks.toml: packs.core: option \"x\" is required",
                "agentstacks.toml: airsl: sdlc needs <0.2, team-lint needs >=0.2",
                "packs/sdlc/skills/status/SKILL.md:6: pack \"core\" has no agent \"codr\"",
                "packs/sdlc/skills/status/SKILL.md:7: pack \"sdlc\" has no option \"nope\"",
                "packs/team-lint/pack.toml: hooks[0].run: \"{{root core}}/hooks/nope.lua\" does not exist in core",
            ]
        );
        Ok(())
    }

    #[test]
    fn check_suppresses_follow_on_errors_from_failed_loads() -> TestResult {
        let sdlc = PackSpec::new("sdlc")
            .requires(&["core"])
            .airsl(">=0.1.4")
            .skill("status", "{{agent core/coder}}")
            .tables(
                "[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"{{root core}}/hooks/x.lua\"\n",
            );
        assert_eq!(refusals(&[(failing_core(), ""), (sdlc, "")])?, [CORE_API]);
        Ok(())
    }

    #[test]
    fn check_still_reports_unrelated_errors_beside_a_failed_load() -> TestResult {
        let sdlc = PackSpec::new("sdlc")
            .requires(&["core"])
            .skill("status", "{{skill guideline-rust/rust-guidelines}}");
        let team_lint = PackSpec::new("team-lint").skill("status", "body");
        assert_eq!(
            refusals(&[(failing_core(), ""), (sdlc, ""), (team_lint, "")])?,
            [
                CORE_API,
                "packs/sdlc/skills/status/SKILL.md:6: {{skill guideline-rust/rust-guidelines}} targets pack \"guideline-rust\", which sdlc does not require. Add it to requires.packs or remove the ref.",
            ]
        );
        Ok(())
    }

    #[test]
    fn failed_loads_are_not_name_checked() -> TestResult {
        let kore = PackSpec::new("kore")
            .listed_as("core")
            .file("pack.toml", &API_ONE.replace("\"core\"", "\"kore\""));
        let tools = vec![("claude", extension_api("claud", 1))];
        let found = run_check_with(&[(kore, "")], tools)?
            .err()
            .ok_or("the check passed")?;
        assert_eq!(
            found,
            [
                "extensions/claud/extension.toml: extension.api: 1 not supported (this agentstacks speaks 0)",
                "packs/kore/pack.toml: kore: api 1 not supported (this agentstacks speaks 0)",
            ]
        );
        Ok(())
    }

    #[test]
    fn check_never_returns_a_valid_set_after_a_failed_load() -> TestResult {
        let tools = vec![("claude", extension_api("claude", 1))];
        let found = run_check_with(&[(PackSpec::new("core"), "")], tools)?
            .err()
            .ok_or("the check passed")?;
        assert_eq!(
            found,
            [
                "extensions/claude/extension.toml: extension.api: 1 not supported (this agentstacks speaks 0)"
            ]
        );
        Ok(())
    }

    #[test]
    fn check_loads_each_listed_entry_exactly_once_with_its_source() -> TestResult {
        let listed = project(&[("core", ""), ("sdlc", "")], &["claude", "codex"])?;
        let mut packs_seen = Vec::new();
        let mut tools_seen = Vec::new();
        let result = check(
            Path::new(PROJECT_FILE),
            listed,
            |key, entry| {
                packs_seen.push((key.to_string(), entry.source().clone()));
                PackSpec::new(key.as_str()).load_raw()
            },
            |key, source| {
                tools_seen.push((key.to_string(), source.clone()));
                extension_api(key.as_str(), 0)
            },
        );
        assert!(result.is_ok());
        let path = |text: &str| -> Result<Source, Box<dyn Error>> {
            Ok(Source::Path(LocalPath::new(text)?))
        };
        assert_eq!(
            packs_seen,
            [
                ("core".to_owned(), path("packs/core")?),
                ("sdlc".to_owned(), path("packs/sdlc")?),
            ]
        );
        assert_eq!(
            tools_seen,
            [
                ("claude".to_owned(), path("extensions/claude")?),
                ("codex".to_owned(), path("extensions/codex")?),
            ]
        );
        Ok(())
    }

    #[test]
    fn check_keeps_loading_the_other_entries_after_one_fails() -> TestResult {
        let listed = project(&[("core", ""), ("sdlc", "")], &["claude"])?;
        let mut loaded = Vec::new();
        let found = check(
            Path::new(PROJECT_FILE),
            listed,
            |key, _entry| {
                loaded.push(key.to_string());
                if key.as_str() == "core" {
                    failing_core().load_raw()
                } else {
                    PackSpec::new("sdlc").load_raw()
                }
            },
            |key, _source| extension_api(key.as_str(), 0),
        )
        .err()
        .ok_or("a failed load returns no ValidSet")?;
        assert_eq!(loaded, ["core", "sdlc"]);
        assert_eq!(shown(&found.into_sorted()), [CORE_API]);
        Ok(())
    }

    #[test]
    fn check_returns_the_set_when_every_entry_loads_and_agrees() -> TestResult {
        let core = PackSpec::new("core").skill("discuss", "body");
        let sdlc = PackSpec::new("sdlc")
            .requires(&["core"])
            .skill("plan", "{{skill core/discuss}}");
        let set = run_check(&[(core, ""), (sdlc, "")], &["claude"])?
            .map_err(|refusals| refusals.join("; "))?;
        assert_eq!(set.packs().len(), 2);
        Ok(())
    }
}
