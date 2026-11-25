//! The conformance pack that `ext test` installs into the scratch project.
//!
//! The files are compiled into the binary, so the check needs no pack on disk and no network.
//!
//! # Responsibilities
//!
//! - [`NAME`] is the pack name the scratch project lists the pack under, which no `--pack` may
//!   take.
//! - [`CONFORMANCE`] holds every file of the pack as its path relative to the pack and its bytes.
//!   The pack uses every neutral item kind, hook event, token kind, var and option type, and has
//!   no `native/` directory, because the staging rule adds that for the tool under test.
//!
//! # Non-responsibilities
//!
//! Placing the files in the scratch project belongs to [`stage`](crate::types::check::stage()).

/// The name the conformance pack is installed under.
pub const NAME: &str = "conformance";

/// Every file of the conformance pack: its path relative to the pack, and its bytes. Sorted by
/// path.
pub const CONFORMANCE: &[(&str, &[u8])] = &[
    (
        "agents/checker.md",
        include_bytes!("../../../assets/conformance/agents/checker.md") as &[u8],
    ),
    (
        "data/settings.json",
        include_bytes!("../../../assets/conformance/data/settings.json") as &[u8],
    ),
    (
        "hooks/event.lua",
        include_bytes!("../../../assets/conformance/hooks/event.lua") as &[u8],
    ),
    (
        "hooks/event_test.lua",
        include_bytes!("../../../assets/conformance/hooks/event_test.lua") as &[u8],
    ),
    (
        "pack.toml",
        include_bytes!("../../../assets/conformance/pack.toml") as &[u8],
    ),
    (
        "skills/check/SKILL.md",
        include_bytes!("../../../assets/conformance/skills/check/SKILL.md") as &[u8],
    ),
    (
        "skills/check/logo.png",
        include_bytes!("../../../assets/conformance/skills/check/logo.png") as &[u8],
    ),
    (
        "skills/check/references/notes.md",
        include_bytes!("../../../assets/conformance/skills/check/references/notes.md") as &[u8],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::FakeFileTree;
    use crate::types::pack::Pack;

    fn tree() -> FakeFileTree {
        CONFORMANCE.iter().fold(
            FakeFileTree::new("packs/conformance"),
            |tree, (path, bytes)| tree.file(path, bytes),
        )
    }

    #[test]
    fn conformance_paths_are_sorted_and_unique() {
        let paths: Vec<&str> = CONFORMANCE.iter().map(|(path, _)| *path).collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(paths, sorted);
    }

    #[test]
    fn conformance_files_are_not_empty() {
        let empty: Vec<&str> = CONFORMANCE
            .iter()
            .filter(|(_, bytes)| bytes.is_empty())
            .map(|(path, _)| *path)
            .collect();
        assert!(empty.is_empty(), "empty files: {empty:?}");
    }

    #[test]
    fn conformance_is_a_pack_named_conformance() -> Result<(), Box<dyn std::error::Error>> {
        let pack = Pack::load(&tree()).map_err(|refusals| format!("{refusals:?}"))?;
        assert_eq!(pack.manifest().name().as_str(), NAME);
        Ok(())
    }
}
