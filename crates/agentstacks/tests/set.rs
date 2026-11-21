//! Checks the whole-set fixtures through the public API over the real disk.
//!
//! Each fixture is a project: an `agentstacks.toml` whose entries are `path` sources, with the
//! packs under `packs/` and the extensions under `extensions/`. The valid fixture shows every
//! feature of a set at once; each invalid fixture is valid except for the one refusal its name
//! says, and the test asserts that refusal's exact text.

use std::cell::RefCell;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use agentstacks::fs::StdFileTree;
use agentstacks::types::{
    Diagnostics, Extension, OptionName, OptionValue, Pack, PackName, ProjectFile, Source, ValidSet,
    check_pack_set,
};

type TestResult = Result<(), Box<dyn Error>>;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sets")
        .join(name)
}

/// The directory a `path` source names, inside the fixture `dir`.
fn directory(dir: &Path, source: &Source) -> Result<PathBuf, Box<dyn Error>> {
    match source {
        Source::Path(local) => Ok(dir.join(local.as_str())),
        Source::Git { .. } => Err("the fixtures use path sources only".into()),
    }
}

/// Parses the fixture's project file, loads every pack and extension it lists and checks the set.
/// The refusals of a failed check come back as the text the author reads.
fn run(name: &str) -> Result<Result<ValidSet, Vec<String>>, Box<dyn Error>> {
    let dir = fixture(name);
    let file = Path::new("agentstacks.toml");
    let text = fs::read_to_string(dir.join(file))?;
    let project = ProjectFile::parse(file, &text).map_err(|found| {
        let shown: Vec<String> = found
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        shown.join("; ")
    })?;
    // Each loader reads the directory the entry's `path` source names. A source that is not a
    // path is a fixture mistake, kept here and returned after the check.
    let mistake = RefCell::new(None);
    let load = |source: &Source| match directory(&dir, source) {
        Ok(path) => Some(StdFileTree::new(path)),
        Err(error) => {
            mistake.replace(Some(error));
            None
        }
    };
    let checked = check_pack_set(
        file,
        project,
        |_key, entry| {
            load(entry.source()).map_or_else(|| Err(Diagnostics::new()), |t| Pack::load(&t))
        },
        |_key, source| {
            load(source).map_or_else(|| Err(Diagnostics::new()), |t| Extension::load(&t))
        },
    );
    if let Some(error) = mistake.into_inner() {
        return Err(error);
    }
    Ok(checked.map_err(|refusals| {
        refusals
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect()
    }))
}

#[test]
fn check_accepts_a_realistic_project() -> TestResult {
    let set = run("valid-realistic")?.map_err(|refusals| refusals.join("; "))?;
    assert_eq!(set.packs().len(), 3);
    assert_eq!(set.extensions().len(), 1);
    let option = |pack: &str, name: &str| -> Result<Option<OptionValue>, Box<dyn Error>> {
        Ok(set
            .option(&PackName::new(pack)?, &OptionName::new(name)?)
            .cloned())
    };
    assert_eq!(
        option("core", "archive_keep")?,
        Some(OptionValue::Integer(50))
    );
    assert_eq!(
        option("core", "style_reinject")?,
        Some(OptionValue::Boolean(true))
    );
    assert_eq!(
        option("sdlc", "base_branch")?,
        Some(OptionValue::String("main".to_owned()))
    );
    let range = set.airsl().ok_or("core and guideline-rust declare airsl")?;
    let needs: Vec<&str> = range
        .needs()
        .iter()
        .map(|(pack, _)| pack.as_str())
        .collect();
    assert_eq!(needs, ["core", "guideline-rust"]);
    Ok(())
}

#[test]
fn check_reports_each_set_refusal() -> TestResult {
    // (fixture, the refusal's text; `<fx>` stands for the fixture directory)
    let cases = [
        (
            "pack-key-mismatch",
            "agentstacks.toml: packs.core: source contains pack \"kore\"",
        ),
        (
            "tool-key-mismatch",
            "agentstacks.toml: tools.claude: source contains extension \"claud\"",
        ),
        (
            "requires-unlisted",
            "agentstacks.toml: packs.sdlc: sdlc requires pack \"core\", which agentstacks.toml does not list (add it: --pack core)",
        ),
        (
            "requires-cycle",
            "agentstacks.toml: requires cycle: a → b → a",
        ),
        (
            "target-not-required",
            "<fx>/packs/sdlc/skills/plan/SKILL.md:68: {{skill guideline-rust/rust-guidelines}} targets pack \"guideline-rust\", which sdlc does not require. Add it to requires.packs or remove the ref.",
        ),
        (
            "target-missing-agent",
            "<fx>/packs/sdlc/skills/a/SKILL.md:6: pack \"core\" has no agent \"codr\"",
        ),
        (
            "target-missing-skill-dir",
            "<fx>/packs/sdlc/skills/a/SKILL.md:6: pack \"core\" has no skill \"coder\"",
        ),
        (
            "option-token-undeclared",
            "<fx>/packs/core/skills/a/SKILL.md:6: pack \"core\" has no option \"archive_kep\"",
        ),
        (
            "hook-run-missing",
            "<fx>/packs/guideline-rust/pack.toml: hooks[1].run: \"{{root core}}/hooks/enforc.lua\" does not exist in core",
        ),
        (
            "hook-run-excluded",
            "<fx>/packs/guideline-rust/pack.toml: hooks[0].run: \"{{root core}}/hooks/old.lua\" does not exist in core",
        ),
        (
            "option-required",
            "agentstacks.toml: packs.core: option \"x\" is required",
        ),
        (
            "option-wrong-type",
            "agentstacks.toml: packs.core.options.archive_keep: option archive_keep expects integer, got string",
        ),
        (
            "option-undeclared",
            "agentstacks.toml: packs.core.options.archive_kep: no option \"archive_kep\"",
        ),
        (
            "airsl-disjoint",
            "agentstacks.toml: airsl: core needs >=0.2, sdlc needs <0.2",
        ),
        (
            "skill-clash",
            "agentstacks.toml: skill \"status\" is shipped by both sdlc and team-lint",
        ),
        (
            "skill-clash-three",
            "agentstacks.toml: skill \"status\" is shipped by sdlc, team-lint and x",
        ),
        (
            "agent-clash",
            "agentstacks.toml: agent \"coder\" is shipped by both core and team-lint",
        ),
    ];
    for (name, expected) in cases {
        let expected = expected.replace("<fx>", &fixture(name).display().to_string());
        let refused = run(name)?.err().ok_or_else(|| format!("{name} passed"))?;
        assert_eq!(refused, [expected], "fixture {name}");
    }
    Ok(())
}

#[test]
fn check_skips_follow_on_errors_from_failed_packs() -> TestResult {
    let fx = fixture("failed-pack");
    let refused = run("failed-pack")?.err().ok_or("failed-pack passed")?;
    assert_eq!(
        refused,
        [
            // The fixture path is absolute, so it sorts before the project file's relative name.
            format!(
                "{}/packs/core/pack.toml: core: api 1 not supported (this agentstacks speaks 0)",
                fx.display()
            ),
            "agentstacks.toml: skill \"status\" is shipped by both sdlc and team-lint".to_owned(),
        ]
    );
    Ok(())
}
