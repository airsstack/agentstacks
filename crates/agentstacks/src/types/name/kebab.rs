//! The one rule behind pack, tool, item and option names.
//!
//! The pack, tool, item and option names follow one character rule. It is written once here, in
//! a private file, so the four name types cannot drift apart and none of them repeats the check.
//!
//! # Responsibilities
//!
//! - [`check_kebab_name`] accepts 1 to 64 bytes that start with `a`-`z` and continue with `a`-`z`,
//!   `0`-`9` or `-`, and refuses everything else with [`InvalidName`].
//! - [`is_name_with_separator`] is the same rule with the separator byte as a parameter, so
//!   `kebab-case` and `snake_case` names share one definition.
//!
//! # Non-responsibilities
//!
//! The name types that use the rule own their own constructors and accessors.

use crate::types::name::InvalidName;

/// The longest a name may be, in bytes.
const MAX_LEN: usize = 64;

/// Checks `value` against `^[a-z][a-z0-9-]{0,63}$`.
///
/// # Errors
///
/// Returns [`InvalidName`] holding `value` when it breaks the rule.
pub(super) fn check_kebab_name(value: &str) -> Result<(), InvalidName> {
    if is_name_with_separator(value, b'-') {
        Ok(())
    } else {
        Err(InvalidName::new(value))
    }
}

/// Whether `value` is 1 to 64 bytes that start with `a`-`z` and continue with `a`-`z`, `0`-`9` or
/// `separator`.
pub(super) fn is_name_with_separator(value: &str, separator: u8) -> bool {
    let bytes = value.as_bytes();
    let starts_well = bytes.first().is_some_and(u8::is_ascii_lowercase);
    let continues_well = bytes
        .iter()
        .skip(1)
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == separator);
    starts_well && continues_well && bytes.len() <= MAX_LEN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separator_rule_takes_the_separator_as_a_parameter() {
        assert!(is_name_with_separator("archive_keep", b'_'));
        assert!(!is_name_with_separator("archive-keep", b'_'));
        assert!(is_name_with_separator("archive-keep", b'-'));
        assert!(!is_name_with_separator("archive_keep", b'-'));
    }

    #[test]
    fn kebab_rule_accepts_lowercase_digits_and_hyphens_up_to_64_bytes() {
        for value in ["core", "guideline-rust", &"a".repeat(64)] {
            assert_eq!(check_kebab_name(value), Ok(()), "{value}");
        }
    }

    #[test]
    fn kebab_rule_refuses_bad_start_bad_character_overlong_and_empty() {
        for value in ["Core", "9lives", "-x", &"a".repeat(65), "archive_keep", ""] {
            assert_eq!(
                check_kebab_name(value),
                Err(InvalidName::new(value)),
                "{value}"
            );
        }
    }
}
