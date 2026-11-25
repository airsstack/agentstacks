//! The airsl versions every pack of a set accepts.
//!
//! It is separate from the set check because later stages test an installed airsl against the same
//! ranges, and they hold an [`AirslRange`] without holding the loaded packs.
//!
//! # Responsibilities
//!
//! - [`AirslRange`] holds the `requires.airsl` range of each pack that declares one, in pack name
//!   order, each as written, together with the versions all of them admit together.
//! - [`AirslRange::is_satisfiable`] tells whether any version satisfies every range. The check
//!   over a whole set refuses a set where none does, naming every pack and its range.
//! - [`AirslRange::admits`] tells whether an installed version satisfies every range, by the
//!   `semver` crate's own matching, and [`AirslRange::installed_refusal`] is the refusal when it
//!   does not.
//!
//! # Non-responsibilities
//!
//! Finding the installed airsl, and pinning it, belong to the code that probes the binary.
//! Intersecting is done over release versions; how a pre-release is treated is left to `semver`
//! at the installed-version check.

use std::collections::BTreeMap;
use std::path::Path;

use crate::types::manifest::PackVersion;
use crate::types::manifest::VersionRange;
use crate::types::name::PackName;
use crate::types::pack::Pack;
use crate::types::report::Diagnostic;
use crate::types::report::Location;
use crate::types::set::error::SetError;
use crate::types::set::interval::VersionInterval;

/// The `requires.airsl` ranges of the packs of a set.
///
/// # Examples
///
/// ```
/// use agentstacks::types::{AirslRange, PackName, PackVersion, VersionRange};
///
/// let range = AirslRange::new(vec![
///     (PackName::new("sdlc")?, VersionRange::new("<0.3")?),
///     (PackName::new("core")?, VersionRange::new(">=0.1.4")?),
/// ])
/// .ok_or("a pack declares a range")?;
/// assert!(range.is_satisfiable());
/// assert!(range.admits(&PackVersion::new("0.2.0")?));
/// assert!(!range.admits(&PackVersion::new("0.1.3")?));
/// assert_eq!(range.needs()[0].0.as_str(), "core");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirslRange {
    needs: Vec<(PackName, VersionRange)>,
    together: VersionInterval,
}

impl AirslRange {
    /// The range of the packs `needs` declare, sorted by pack name; `None` when `needs` is empty,
    /// because no pack then restricts airsl.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::AirslRange;
    ///
    /// assert!(AirslRange::new(Vec::new()).is_none());
    /// ```
    #[must_use]
    pub fn new(mut needs: Vec<(PackName, VersionRange)>) -> Option<Self> {
        if needs.is_empty() {
            return None;
        }
        needs.sort_by(|(left, _), (right, _)| left.cmp(right));
        let together = needs
            .iter()
            .map(|(_, req)| VersionInterval::of_req(req.as_version_req()))
            .fold(VersionInterval::UNBOUNDED, |both, next| {
                both.intersect(&next)
            });
        Some(Self { needs, together })
    }

    /// The range of the loaded `packs` that declare `requires.airsl`, each under its `[packs.*]`
    /// key; `None` when no pack declares one.
    pub(crate) fn of_packs(packs: &BTreeMap<PackName, Pack>) -> Option<Self> {
        let needs = packs
            .iter()
            .filter_map(|(key, pack)| {
                let req = pack.manifest().requires().airsl()?;
                Some((key.clone(), req.clone()))
            })
            .collect();
        Self::new(needs)
    }

    /// Each pack that declares a range, in pack name order, with the range as written.
    #[must_use]
    pub fn needs(&self) -> &[(PackName, VersionRange)] {
        &self.needs
    }

    /// Whether some version satisfies every pack's range.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::{AirslRange, PackName, VersionRange};
    ///
    /// let disjoint = AirslRange::new(vec![
    ///     (PackName::new("core")?, VersionRange::new(">=0.2")?),
    ///     (PackName::new("sdlc")?, VersionRange::new("<0.2")?),
    /// ])
    /// .ok_or("a pack declares a range")?;
    /// assert!(!disjoint.is_satisfiable());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn is_satisfiable(&self) -> bool {
        !self.together.is_empty()
    }

    /// Whether `installed` satisfies every pack's range, by `semver`'s own matching.
    #[must_use]
    pub fn admits(&self, installed: &PackVersion) -> bool {
        self.needs
            .iter()
            .all(|(_, req)| req.as_version_req().matches(installed.as_version()))
    }

    /// The refusal, located at the project file `project_file` alone, when `installed` does not
    /// satisfy every range; `None` when it does.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::{AirslRange, PackName, PackVersion, VersionRange};
    ///
    /// let range = AirslRange::new(vec![(PackName::new("core")?, VersionRange::new(">=0.1.4")?)])
    ///     .ok_or("a pack declares a range")?;
    /// let refusal = range
    ///     .installed_refusal(Path::new("agentstacks.toml"), &PackVersion::new("0.1.3")?)
    ///     .ok_or("0.1.3 is below the range")?;
    /// assert_eq!(
    ///     refusal.to_string(),
    ///     "agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4",
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn installed_refusal(
        &self,
        project_file: &Path,
        installed: &PackVersion,
    ) -> Option<Diagnostic> {
        if self.admits(installed) {
            return None;
        }
        Some(Diagnostic::new(
            Location::set_level(project_file, None),
            SetError::AirslInstalled {
                installed: installed.clone(),
                ranges: self.needs.iter().map(|(_, req)| req.clone()).collect(),
            },
        ))
    }

    /// The refusal, located at the project file `project_file` alone, when no version satisfies
    /// every range; `None` when some version does.
    pub(crate) fn disjoint_refusal(&self, project_file: &Path) -> Option<Diagnostic> {
        if self.is_satisfiable() {
            return None;
        }
        Some(Diagnostic::new(
            Location::set_level(project_file, None),
            SetError::AirslDisjoint {
                needs: self.needs.clone(),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::set::testing::{PROJECT_FILE, name};
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    fn range(needs: &[(&str, &str)]) -> Result<AirslRange, Box<dyn Error>> {
        let mut parsed = Vec::new();
        for (pack, written) in needs {
            parsed.push((name(pack)?, VersionRange::new(written)?));
        }
        Ok(AirslRange::new(parsed).ok_or("no pack declares a range")?)
    }

    fn version(text: &str) -> Result<PackVersion, Box<dyn Error>> {
        Ok(PackVersion::new(text)?)
    }

    #[test]
    fn ranges_intersect_per_cargo_semantics() -> TestResult {
        assert!(range(&[("a", ">=0.1.4"), ("b", "^0.1")])?.is_satisfiable());
        assert!(!range(&[("a", "~0.1.4"), ("b", ">=0.2")])?.is_satisfiable());
        assert!(range(&[("a", "*"), ("b", ">=0.1.4")])?.is_satisfiable());
        let any = range(&[("a", "*")])?;
        assert!(any.admits(&version("0.0.0")?));
        assert!(any.admits(&version("3.0.0")?));
        let between = range(&[("a", ">=0.1, <0.3")])?;
        assert!(between.admits(&version("0.2.3")?));
        assert!(!between.admits(&version("0.3.0")?));
        Ok(())
    }

    #[test]
    fn packs_are_kept_in_name_order() -> TestResult {
        let found = range(&[("sdlc", "<0.3"), ("core", ">=0.1.4")])?;
        let names: Vec<&str> = found
            .needs()
            .iter()
            .map(|(pack, _)| pack.as_str())
            .collect();
        assert_eq!(names, ["core", "sdlc"]);
        assert!(AirslRange::new(Vec::new()).is_none());
        Ok(())
    }

    #[test]
    fn disjoint_ranges_are_refused_naming_each_pack() -> TestResult {
        let refusal = range(&[("sdlc", "<0.2"), ("core", ">=0.2")])?
            .disjoint_refusal(Path::new(PROJECT_FILE))
            .ok_or("the ranges do not meet")?;
        assert_eq!(
            refusal.to_string(),
            "agentstacks.toml: airsl: core needs >=0.2, sdlc needs <0.2"
        );
        let refusal = range(&[("team-lint", ">=0.1"), ("sdlc", "<0.2"), ("core", ">=0.2")])?
            .disjoint_refusal(Path::new(PROJECT_FILE))
            .ok_or("the ranges do not meet")?;
        assert_eq!(
            refusal.to_string(),
            "agentstacks.toml: airsl: core needs >=0.2, sdlc needs <0.2, team-lint needs >=0.1"
        );
        assert!(
            range(&[("a", ">=0.1")])?
                .disjoint_refusal(Path::new(PROJECT_FILE))
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn installed_version_must_satisfy_every_range() -> TestResult {
        let refuse =
            |range: &AirslRange, installed: &str| -> Result<Option<String>, Box<dyn Error>> {
                Ok(range
                    .installed_refusal(Path::new(PROJECT_FILE), &version(installed)?)
                    .map(|refusal| refusal.to_string()))
            };
        let one = range(&[("core", ">=0.1.4")])?;
        assert_eq!(
            refuse(&one, "0.1.3")?.as_deref(),
            Some("agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4")
        );
        assert_eq!(refuse(&one, "0.1.4")?, None);
        let two = range(&[("core", ">=0.1.4"), ("sdlc", "<0.3")])?;
        assert_eq!(
            refuse(&two, "0.1.3")?.as_deref(),
            Some("agentstacks.toml: airsl 0.1.3 installed, packs need >=0.1.4, <0.3")
        );
        assert_eq!(
            refuse(&two, "0.3.0")?.as_deref(),
            Some("agentstacks.toml: airsl 0.3.0 installed, packs need >=0.1.4, <0.3")
        );
        Ok(())
    }
}
