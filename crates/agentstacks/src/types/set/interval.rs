//! The release versions one version range admits, as an interval.
//!
//! It is separate from [`AirslRange`](crate::types::AirslRange) because `semver` can test one
//! version against a range but cannot say whether several ranges overlap, which needs the
//! intervals.
//!
//! # Responsibilities
//!
//! - `VersionInterval` is a half-open interval of release versions: its lower bound is
//!   included and its upper bound is excluded, or absent for an interval that never ends. It is
//!   built from a `semver::Comparator` for each of the eight operators semver has
//!   (`=`, `>`, `>=`, `<`, `<=`, `~`, `^` and the wildcard), with major-only and major.minor
//!   forms, and from a whole `semver::VersionReq` as the intersection of its comparators. A range
//!   with no comparators (`*`) is the interval that never ends.
//! - Two intervals intersect, and an interval reports whether it is empty.
//! - Intervals cover release versions only. A pre-release is judged by `semver`'s own matching,
//!   where the installed version is compared with the ranges. A bound that would pass the largest
//!   `u64` carries into the next part, and an interval whose end would pass every version has no
//!   end.
//!
//! # Non-responsibilities
//!
//! Which ranges a pack may declare is guaranteed by
//! [`VersionRange`](crate::types::VersionRange), and which packs' ranges are intersected belongs to
//! [`AirslRange`](crate::types::AirslRange).

use semver::{Comparator, Op, VersionReq};

/// A version's `(major, minor, patch)`, ordered the way versions are.
type Triple = (u64, u64, u64);

/// The release versions from `lower` up to, not including, `upper`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VersionInterval {
    lower: Triple,
    upper: Option<Triple>,
}

impl VersionInterval {
    /// Every release version.
    pub(super) const UNBOUNDED: Self = Self {
        lower: (0, 0, 0),
        upper: None,
    };

    /// No version at all.
    const EMPTY: Self = Self {
        lower: (0, 0, 0),
        upper: Some((0, 0, 0)),
    };

    /// The release versions every comparator of `req` admits.
    pub(super) fn of_req(req: &VersionReq) -> Self {
        req.comparators
            .iter()
            .map(Self::of_comparator)
            .fold(Self::UNBOUNDED, |both, next| both.intersect(&next))
    }

    /// The release versions one comparator admits.
    ///
    /// `Op` is `#[non_exhaustive]`, so a match over it needs a last arm. A range held by a
    /// [`VersionRange`](crate::types::VersionRange) never reaches it, because that type refuses any
    /// operator but the eight handled here when it is built. Should one arrive anyway, the arm
    /// admits nothing, so an operator this code does not understand is refused instead of
    /// accepted.
    fn of_comparator(comparator: &Comparator) -> Self {
        let floor = (
            comparator.major,
            comparator.minor.unwrap_or(0),
            comparator.patch.unwrap_or(0),
        );
        let next = next_at_written_precision(comparator);
        match comparator.op {
            Op::Exact | Op::Wildcard => Self::between(floor, next),
            Op::Greater => next.map_or(Self::EMPTY, |lower| Self::between(lower, None)),
            Op::GreaterEq => Self::between(floor, None),
            Op::Less => Self::between((0, 0, 0), Some(floor)),
            Op::LessEq => Self::between((0, 0, 0), next),
            Op::Tilde => Self::between(floor, tilde_end(comparator)),
            Op::Caret => Self::between(floor, caret_end(comparator)),
            _ => Self::EMPTY,
        }
    }

    const fn between(lower: Triple, upper: Option<Triple>) -> Self {
        Self { lower, upper }
    }

    /// The versions both intervals admit.
    pub(super) fn intersect(&self, other: &Self) -> Self {
        let upper = match (self.upper, other.upper) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (bound, None) | (None, bound) => bound,
        };
        Self {
            lower: self.lower.max(other.lower),
            upper,
        }
    }

    /// Whether no version is admitted.
    pub(super) fn is_empty(&self) -> bool {
        self.upper.is_some_and(|upper| upper <= self.lower)
    }
}

/// The first version after every version that matches `comparator` as far as it is written: the
/// next patch for `1.2.3`, the next minor for `1.2`, the next major for `1`. `None` when no
/// version follows.
const fn next_at_written_precision(comparator: &Comparator) -> Option<Triple> {
    let major = comparator.major;
    match (comparator.minor, comparator.patch) {
        (Some(minor), Some(patch)) => match patch.checked_add(1) {
            Some(next) => Some((major, minor, next)),
            None => after_minor(major, minor),
        },
        (Some(minor), None) => after_minor(major, minor),
        (None, _) => after_major(major),
    }
}

/// The first version after every `major.minor.*`.
const fn after_minor(major: u64, minor: u64) -> Option<Triple> {
    match minor.checked_add(1) {
        Some(next) => Some((major, next, 0)),
        None => after_major(major),
    }
}

/// The first version after every `major.*.*`.
const fn after_major(major: u64) -> Option<Triple> {
    match major.checked_add(1) {
        Some(next) => Some((next, 0, 0)),
        None => None,
    }
}

/// The end of a tilde range: the next minor when a minor is written, else the next major.
const fn tilde_end(comparator: &Comparator) -> Option<Triple> {
    match comparator.minor {
        Some(minor) => after_minor(comparator.major, minor),
        None => after_major(comparator.major),
    }
}

/// The end of a caret range: the leftmost non-zero part is the one that may not change.
const fn caret_end(comparator: &Comparator) -> Option<Triple> {
    let major = comparator.major;
    if major > 0 {
        return after_major(major);
    }
    match (comparator.minor, comparator.patch) {
        (None, _) => Some((1, 0, 0)),
        (Some(0), Some(patch)) => match patch.checked_add(1) {
            Some(next) => Some((0, 0, next)),
            None => after_minor(0, 0),
        },
        (Some(minor), _) => after_minor(0, minor),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use semver::Version;
    use std::error::Error;

    type TestResult = Result<(), Box<dyn Error>>;

    const PROBES: [Triple; 18] = [
        (0, 0, 0),
        (0, 0, 3),
        (0, 0, 4),
        (0, 1, 0),
        (0, 1, 3),
        (0, 1, 4),
        (0, 1, 9),
        (0, 2, 0),
        (0, 2, 3),
        (0, 3, 0),
        (1, 0, 0),
        (1, 1, 9),
        (1, 2, 0),
        (1, 2, 3),
        (1, 2, 4),
        (1, 3, 0),
        (2, 0, 0),
        (3, 0, 0),
    ];

    fn admits(interval: &VersionInterval, version: Triple) -> bool {
        version >= interval.lower && interval.upper.is_none_or(|upper| version < upper)
    }

    fn interval(range: &str) -> Result<VersionInterval, Box<dyn Error>> {
        Ok(VersionInterval::of_req(&VersionReq::parse(range)?))
    }

    #[test]
    fn interval_matches_semver_for_every_operator() -> TestResult {
        let rows = [
            "=1.2.3",
            "=1.2",
            "=1",
            ">1.2.3",
            ">1.2",
            ">1",
            ">=0.1.4",
            ">=0.1",
            "<0.2",
            "<1.2.3",
            "<1",
            "<=1.2.3",
            "<=1.2",
            "<=1",
            "~0.1.4",
            "~1.2",
            "~1",
            "^1.2.3",
            "^1.2",
            "^1",
            "^0.2.3",
            "^0.0.3",
            "^0.0",
            "^0",
            "^0.0.0",
            "0.1",
            "1.2.*",
            "1.*",
            "*",
            ">=0.1, <0.3",
        ];
        for row in rows {
            let req = VersionReq::parse(row)?;
            let interval = VersionInterval::of_req(&req);
            for (major, minor, patch) in PROBES {
                let version = Version::new(major, minor, patch);
                assert_eq!(
                    admits(&interval, (major, minor, patch)),
                    req.matches(&version),
                    "{row} against {version}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn interval_intersection_reports_emptiness() -> TestResult {
        let both = interval(">=0.1.4")?.intersect(&interval("^0.1")?);
        assert!(!both.is_empty());
        assert!(admits(&both, (0, 1, 4)));
        assert!(!admits(&both, (0, 2, 0)));
        assert!(interval(">=0.2")?.intersect(&interval("<0.2")?).is_empty());
        assert!(!VersionInterval::UNBOUNDED.is_empty());
        Ok(())
    }

    #[test]
    fn bounds_past_the_largest_number_carry_or_disappear() -> TestResult {
        let max = u64::MAX;
        assert!(interval(&format!(">{max}.{max}.{max}"))?.is_empty());
        let at_most = interval(&format!("<={max}.{max}.{max}"))?;
        assert!(!at_most.is_empty());
        assert_eq!(at_most.upper, None);
        let caret = interval(&format!("^{max}"))?;
        assert_eq!(caret.upper, None);
        assert!(admits(&caret, (max, 5, 5)));
        let carried = interval(&format!(">{max}.1.{max}"))?;
        assert_eq!(carried.lower, (max, 2, 0));
        Ok(())
    }
}
