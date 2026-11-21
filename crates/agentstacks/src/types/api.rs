//! The `api` number this agentstacks speaks.
//!
//! # Responsibilities
//!
//! - [`API`] is the one place the supported `api` is written. Every reader that checks the `api`
//!   key of a file, and every refusal message that names the supported number, takes it from here,
//!   so moving to another api number changes one line.
//!
//! # Non-responsibilities
//!
//! Which files carry an `api` key, and what a refusal of it looks like, belong to the readers of
//! those files.

/// The only `api` this agentstacks speaks, the value of the `api` key in `pack.toml`,
/// `extension.toml`, `agentstacks.toml` and `agentstacks.lock`.
///
/// It is an `i64` because that is the type a TOML integer is read as.
///
/// # Examples
///
/// ```
/// use agentstacks::types::API;
///
/// assert_eq!(API, 0);
/// ```
pub const API: i64 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_is_the_unstable_zero() {
        assert_eq!(API, 0);
    }
}
