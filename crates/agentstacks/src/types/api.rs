//! The `api` number and the version this agentstacks runs as, and the decision every reader takes
//! on a file's `api`.
//!
//! # Responsibilities
//!
//! - [`API`] is the one place the supported `api` is written. Every reader that checks the `api`
//!   key of a file, and every refusal message that names the supported number, takes it from here,
//!   so moving to another api number changes one line.
//! - [`API_MAJOR`] is that number as the unsigned major a release tag carries.
//! - [`VERSION`] is the product version of the running agentstacks as a semver value, so a range a
//!   pack or extension requires can be tested against it.
//! - [`check`] is the one rule on a found `api`: it is the current one, the legacy `0` of the
//!   `0.x` releases, or one this agentstacks does not speak. [`ApiCheck`] is its answer.
//! - `read` applies [`check`] to the `api` key of a TOML file, refusing an unsupported one in the
//!   caller's own error type, so the lock and the project file share one reader.
//!
//! # Non-responsibilities
//!
//! Which files carry an `api` key, and what a refusal of it looks like, belong to the readers of
//! those files.

use crate::types::format::TomlFields;
use crate::types::report::FormatError;
use crate::types::report::UnsupportedApi;

/// The `api` this agentstacks speaks, the value of the `api` key in `pack.toml`, `extension.toml`,
/// `agentstacks.toml` and `agentstacks.lock`.
///
/// It is an `i64` because that is the type a TOML integer is read as.
///
/// # Examples
///
/// ```
/// use agentstacks::types::API;
///
/// assert_eq!(API, 1);
/// ```
pub const API: i64 = 1;

/// The major version a release of the official packs and extensions must have to speak [`API`].
///
/// It is [`API`] as an unsigned number; the build fails when [`API`] is negative.
///
/// # Examples
///
/// ```
/// use agentstacks::types::api::API_MAJOR;
///
/// assert_eq!(API_MAJOR, 1);
/// ```
pub const API_MAJOR: u64 = {
    assert!(API >= 0, "API is a version number and cannot be negative");
    API.unsigned_abs()
};

/// Reads one decimal component of the package version, `0` when it is not a number.
///
/// Cargo sets `CARGO_PKG_VERSION_MAJOR`, `CARGO_PKG_VERSION_MINOR` and `CARGO_PKG_VERSION_PATCH`
/// to numbers, so the `0` is never reached for them.
const fn component(text: &str) -> u64 {
    match u64::from_str_radix(text, 10) {
        Ok(number) => number,
        Err(_) => 0,
    }
}

/// The version of this agentstacks, the product version and not the [`API`] number, built from the
/// package version Cargo compiles the crate with.
///
/// # Examples
///
/// ```
/// use agentstacks::types::VERSION;
///
/// assert_eq!(VERSION.to_string(), env!("CARGO_PKG_VERSION"));
/// ```
pub const VERSION: semver::Version = semver::Version::new(
    component(env!("CARGO_PKG_VERSION_MAJOR")),
    component(env!("CARGO_PKG_VERSION_MINOR")),
    component(env!("CARGO_PKG_VERSION_PATCH")),
);

/// The `api` of the `0.x` releases, which a reader names as legacy.
///
/// # Examples
///
/// ```
/// use agentstacks::types::api::{LEGACY_API, check};
/// use agentstacks::types::ApiCheck;
///
/// assert_eq!(check(LEGACY_API), ApiCheck::Legacy);
/// ```
pub const LEGACY_API: i64 = 0;

/// What this agentstacks makes of the `api` a file asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiCheck {
    /// The `api` this agentstacks speaks, [`API`].
    Current,
    /// The `api` of the `0.x` releases.
    Legacy,
    /// Any other integer, which this agentstacks does not speak.
    Unsupported(i64),
}

/// Decides what `found`, the `api` a file asks for, is.
///
/// # Examples
///
/// ```
/// use agentstacks::types::api::check;
/// use agentstacks::types::{API, ApiCheck};
///
/// assert_eq!(check(API), ApiCheck::Current);
/// assert_eq!(check(0), ApiCheck::Legacy);
/// assert_eq!(check(2), ApiCheck::Unsupported(2));
/// ```
#[must_use]
pub const fn check(found: i64) -> ApiCheck {
    if found == API {
        ApiCheck::Current
    } else if found == LEGACY_API {
        ApiCheck::Legacy
    } else {
        ApiCheck::Unsupported(found)
    }
}

/// Reads the required `api` key of `root`, refusing an unsupported one at the file as `wrap` of
/// the refused number.
///
/// A missing or mistyped `api` is refused by the read itself. The answer is `None` for either
/// refusal, and the refusal is already recorded in `root`, so a caller that returns a value only
/// when `root` holds no refusal never uses a made-up answer.
pub(in crate::types) fn read<E: Into<FormatError>>(
    root: &mut TomlFields,
    wrap: impl FnOnce(UnsupportedApi) -> E,
) -> Option<ApiCheck> {
    match check(root.required_integer("api")?) {
        ApiCheck::Unsupported(found) => {
            root.refuse_at_file(wrap(UnsupportedApi::new(found)));
            None
        }
        read => Some(read),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::types::format::parse_toml_document;
    use crate::types::lock::LockError;

    #[test]
    fn version_is_the_package_version() {
        assert_eq!(VERSION.to_string(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn component_reads_a_number_and_falls_back_to_zero() {
        assert_eq!(component("12"), 12);
        assert_eq!(component("0"), 0);
        assert_eq!(component("1-rc"), 0);
        assert_eq!(component(""), 0);
    }

    #[test]
    fn api_is_one() {
        assert_eq!(API, 1);
    }

    #[test]
    fn api_major_is_the_api() {
        assert_eq!(API_MAJOR, 1);
    }

    #[test]
    fn current_is_the_api_this_agentstacks_speaks() {
        assert_eq!(check(1), ApiCheck::Current);
    }

    #[test]
    fn zero_is_legacy() {
        assert_eq!(check(LEGACY_API), ApiCheck::Legacy);
        assert_eq!(LEGACY_API, 0);
    }

    #[test]
    fn two_and_minus_one_are_unsupported() {
        assert_eq!(check(2), ApiCheck::Unsupported(2));
        assert_eq!(check(-1), ApiCheck::Unsupported(-1));
    }

    fn read_text(
        text: &str,
    ) -> Result<(Option<ApiCheck>, Vec<String>), Box<dyn std::error::Error>> {
        let file = Path::new("agentstacks.lock");
        let table = parse_toml_document(file, text).map_err(|found| found.to_string())?;
        let mut root = TomlFields::new(file, "", table);
        let read = read(&mut root, LockError::from);
        let shown = root
            .into_diagnostics()
            .into_sorted()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok((read, shown))
    }

    #[test]
    fn read_answers_current_and_legacy_without_a_refusal() -> Result<(), Box<dyn std::error::Error>>
    {
        assert_eq!(read_text("api = 1\n")?, (Some(ApiCheck::Current), vec![]));
        assert_eq!(read_text("api = 0\n")?, (Some(ApiCheck::Legacy), vec![]));
        Ok(())
    }

    #[test]
    fn read_refuses_an_unsupported_api_in_the_callers_error()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            read_text("api = 2\n")?,
            (
                None,
                vec![
                    "agentstacks.lock: api 2 not supported (this agentstacks speaks 1)".to_owned()
                ]
            )
        );
        Ok(())
    }

    #[test]
    fn read_answers_none_for_a_missing_or_mistyped_api() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            read_text("")?,
            (None, vec!["agentstacks.lock: api: required".to_owned()])
        );
        assert_eq!(
            read_text("api = \"1\"\n")?,
            (
                None,
                vec!["agentstacks.lock: api: expected integer, got string".to_owned()]
            )
        );
        Ok(())
    }
}
