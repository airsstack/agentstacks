//! The rules over which packs require which.
//!
//! It is a set-level rule because a requirement points at another entry of the project file and a
//! cycle spans several packs, so no single pack can be refused for it alone.
//!
//! # Responsibilities
//!
//! - `check_requires` refuses each `requires.packs` entry of a loaded pack that the project file
//!   does not list, at that pack's entry in the project file.
//! - It refuses each group of packs that require one another in a loop, once per group, at the
//!   project file alone. The cycle prints as the shortest path from the group's smallest name
//!   back to itself, ties broken by name order.
//! - A pack that failed to load is not in the graph: its requirements are unknown, so it adds no
//!   edge and nothing is reported about it.
//!
//! # Non-responsibilities
//!
//! A pack requiring itself is refused when its `pack.toml` is read. Whether a required pack's
//! items exist belongs to the token and hook checks.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use crate::types::name::PackName;
use crate::types::pack::Pack;
use crate::types::project::ProjectFile;
use crate::types::report::Diagnostic;
use crate::types::report::Location;
use crate::types::set::error::SetError;

/// The loaded packs each loaded pack requires, in name order.
type Edges<'a> = BTreeMap<&'a PackName, Vec<&'a PackName>>;

/// Refuses requirements the project file at `project_file` does not list, and requirement cycles,
/// among the loaded `packs` of `project`.
pub(crate) fn check_requires(
    project_file: &Path,
    project: &ProjectFile,
    packs: &BTreeMap<PackName, Pack>,
) -> Vec<Diagnostic> {
    let mut refusals = Vec::new();
    let mut edges = Edges::new();
    for (key, pack) in packs {
        let outgoing = edges.entry(key).or_default();
        for required in pack.manifest().requires().packs() {
            if !project.packs().contains_key(required) {
                refusals.push(Diagnostic::new(
                    Location::set_level(project_file, Some(format!("packs.{key}"))),
                    SetError::RequiresUnlisted {
                        pack: key.clone(),
                        required: required.clone(),
                    },
                ));
            } else if packs.contains_key(required) {
                outgoing.push(required);
            }
        }
        outgoing.sort();
    }
    for cycle in cycles(&edges) {
        refusals.push(Diagnostic::new(
            Location::set_level(project_file, None),
            SetError::RequiresCycle {
                path: cycle.into_iter().cloned().collect(),
            },
        ));
    }
    refusals
}

/// One shortest cycle for each group of packs that reach one another, groups in the order of their
/// smallest name.
fn cycles<'a>(edges: &Edges<'a>) -> Vec<Vec<&'a PackName>> {
    let reach: BTreeMap<&PackName, BTreeSet<&PackName>> = edges
        .keys()
        .map(|&pack| (pack, reachable(edges, pack)))
        .collect();
    let mut assigned: BTreeSet<&PackName> = BTreeSet::new();
    let mut found = Vec::new();
    for (&start, from_start) in &reach {
        if assigned.contains(start) {
            continue;
        }
        assigned.insert(start);
        let group: Vec<&PackName> = from_start
            .iter()
            .copied()
            .filter(|other| reach.get(other).is_some_and(|back| back.contains(start)))
            .collect();
        assigned.extend(group.iter().copied());
        if !group.is_empty() {
            found.extend(shortest_cycle(edges, start));
        }
    }
    found
}

/// The packs reached from `start` along one or more edges; `start` itself only when it lies on a
/// cycle.
fn reachable<'a>(edges: &Edges<'a>, start: &PackName) -> BTreeSet<&'a PackName> {
    let mut seen = BTreeSet::new();
    let mut queue: VecDeque<&PackName> = VecDeque::from([start]);
    while let Some(pack) = queue.pop_front() {
        for &next in edges.get(pack).map_or(&[][..], Vec::as_slice) {
            if seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    seen
}

/// The shortest path from `start` back to `start`, the first of the shortest in name order; `None`
/// when `start` lies on no cycle.
///
/// The search is breadth first with each pack's requirements taken in name order, so the first
/// pack found with an edge back to `start` ends the shortest path that is smallest by name.
fn shortest_cycle<'a>(edges: &Edges<'a>, start: &'a PackName) -> Option<Vec<&'a PackName>> {
    let mut parent: BTreeMap<&PackName, &PackName> = BTreeMap::new();
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(pack) = queue.pop_front() {
        let next = edges.get(pack).map_or(&[][..], Vec::as_slice);
        if next.contains(&start) {
            let mut path = vec![start];
            let mut current = pack;
            while current != start {
                path.push(current);
                current = parent.get(current)?;
            }
            path.push(start);
            path.reverse();
            return Some(path);
        }
        for &required in next {
            if seen.insert(required) {
                parent.insert(required, pack);
                queue.push_back(required);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::set::testing::{PROJECT_FILE, PackSpec, loaded, project, shown};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    /// Checks the packs `specs`, all listed by the project, plus the listed names `failed` that
    /// never loaded.
    fn check(specs: &[PackSpec], failed: &[&str]) -> Result<Vec<String>, Box<dyn Error>> {
        let loaded = loaded(specs)?;
        let mut listed: Vec<(&str, &str)> = loaded.keys().map(|key| (key.as_str(), "")).collect();
        listed.extend(failed.iter().map(|name| (*name, "")));
        let project = project(&listed, &["claude"])?;
        Ok(shown(&check_requires(
            Path::new(PROJECT_FILE),
            &project,
            &loaded,
        )))
    }

    fn requiring(name: &str, required: &[&str]) -> PackSpec {
        PackSpec::new(name).requires(required)
    }

    #[test]
    fn unlisted_requirement_is_refused_with_the_add_hint() -> TestResult {
        let found = check(&[requiring("sdlc", &["core"])], &[])?;
        assert_eq!(
            found,
            [
                "agentstacks.toml: packs.sdlc: sdlc requires pack \"core\", which agentstacks.toml does not list (add it: --pack core)"
            ]
        );
        Ok(())
    }

    #[test]
    fn each_cycle_is_reported_once_from_its_smallest_name() -> TestResult {
        let found = check(&[requiring("b", &["a"]), requiring("a", &["b"])], &[])?;
        assert_eq!(found, ["agentstacks.toml: requires cycle: a → b → a"]);
        let found = check(
            &[
                requiring("b", &["a"]),
                requiring("a", &["b"]),
                requiring("d", &["c"]),
                requiring("c", &["d"]),
            ],
            &[],
        )?;
        assert_eq!(
            found,
            [
                "agentstacks.toml: requires cycle: a → b → a",
                "agentstacks.toml: requires cycle: c → d → c"
            ]
        );
        Ok(())
    }

    #[test]
    fn cycle_prints_the_shortest_path_back_to_the_smallest_name() -> TestResult {
        let found = check(
            &[
                requiring("a", &["b"]),
                requiring("b", &["c"]),
                requiring("c", &["a"]),
            ],
            &[],
        )?;
        assert_eq!(found, ["agentstacks.toml: requires cycle: a → b → c → a"]);
        let found = check(
            &[
                requiring("a", &["b"]),
                requiring("b", &["a", "c"]),
                requiring("c", &["a"]),
            ],
            &[],
        )?;
        assert_eq!(found, ["agentstacks.toml: requires cycle: a → b → a"]);
        Ok(())
    }

    #[test]
    fn a_pack_reaching_a_cycle_without_joining_it_is_not_part_of_it() -> TestResult {
        let found = check(
            &[
                requiring("a", &["b"]),
                requiring("b", &["c"]),
                requiring("c", &["b"]),
            ],
            &[],
        )?;
        assert_eq!(found, ["agentstacks.toml: requires cycle: b → c → b"]);
        Ok(())
    }

    #[test]
    fn diamond_is_not_a_cycle() -> TestResult {
        let found = check(
            &[
                requiring("a", &["b", "c"]),
                requiring("b", &["d"]),
                requiring("c", &["d"]),
                requiring("d", &[]),
            ],
            &[],
        )?;
        assert!(found.is_empty(), "{found:?}");
        Ok(())
    }

    #[test]
    fn requirements_of_failed_packs_are_unknown() -> TestResult {
        // `b` is listed but failed to load, and its manifest would have required `a`.
        let found = check(&[requiring("a", &["b"])], &["b"])?;
        assert!(found.is_empty(), "{found:?}");
        Ok(())
    }
}
