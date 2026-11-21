//! Builders the unit tests of the pack checks share.
//!
//! # Responsibilities
//!
//! - Builds a [`Walked`] and a [`PackLayout`] from a list of paths, and a [`PackManifest`] from
//!   `pack.toml` text, so each check is tested on the smallest pack that shows its rule.
//!
//! # Non-responsibilities
//!
//! Compiled for tests only; it asserts nothing itself.

use std::error::Error;
use std::path::{Path, PathBuf};

use crate::types::manifest::PackManifest;
use crate::types::pack::layout::PackLayout;
use crate::types::report::Diagnostics;
use crate::types::tree::TreePath;
use crate::types::tree::Walked;

/// The directory the test packs claim to live in.
pub(super) const DIR: &str = "packs/core";

/// The `[pack]` table every test manifest starts its tables with.
const IDENTITY: &str =
    "[pack]\nname = \"core\"\nversion = \"0.1.0\"\napi = 0\ndescription = \"d\"\n";

/// The walked paths `texts`, as the walker would list them.
pub(super) fn paths(texts: &[&str]) -> Vec<TreePath> {
    texts
        .iter()
        .filter_map(|text| TreePath::from_relative(Path::new(text)).ok())
        .collect()
}

/// The walk of a pack holding the files `texts`, in which the walk also refused the entries
/// `refused`.
pub(super) fn refused_walk(texts: &[&str], refused: &[&str]) -> Walked {
    Walked::new(
        paths(texts),
        Vec::new(),
        refused.iter().map(PathBuf::from).collect(),
    )
}

/// The layout of a pack holding the files `texts`, refusals ignored; empty when `texts` holds no
/// `pack.toml`.
pub(super) fn layout(texts: &[&str]) -> PackLayout {
    PackLayout::classify(Path::new(DIR), &refused_walk(texts, &[]))
        .map(|(layout, _)| layout)
        .unwrap_or_default()
}

/// The manifest with `top` (keys before the first table) and `rest` (after `[pack]`).
pub(super) fn manifest_with(top: &str, rest: &str) -> Result<PackManifest, Box<dyn Error>> {
    let text = format!("{top}\n{IDENTITY}{rest}");
    let file = Path::new(DIR).join("pack.toml");
    PackManifest::parse(&file, &text).map_err(|diagnostics| {
        let shown: Vec<String> = diagnostics.into_iter().map(|d| d.to_string()).collect();
        shown.join("; ").into()
    })
}

/// The manifest with `rest` after `[pack]` and nothing before it.
pub(super) fn manifest(rest: &str) -> Result<PackManifest, Box<dyn Error>> {
    manifest_with("", rest)
}

/// The refusals as the text the author reads.
pub(super) fn shown(diagnostics: Diagnostics) -> Vec<String> {
    diagnostics
        .into_sorted()
        .iter()
        .map(ToString::to_string)
        .collect()
}
