//! The refusals [`HookEvent`](crate::types::HookEvent) and [`Policy`](crate::types::Policy)
//! return when their parsers reject a value.
//!
//! These errors sit together because each is only the refusal wording of one closed vocabulary in
//! this group; the vocabularies themselves stay in their own files.
//!
//! # Responsibilities
//!
//! - [`InvalidHookEvent`]: a hook `event` outside the neutral vocabulary.
//! - [`InvalidPolicy`]: a `grants.policy` that is `trusted`, which no pack hook may use, or that is
//!   no policy at all.
//!
//! # Non-responsibilities
//!
//! Tool-specific events belong to `native/<tool>/hooks.toml`, where the event is an opaque string,
//! and turning a policy into airsl flags belongs to the extension host. The tests assert the exact
//! `Display` text of every variant.

/// An `event` that is none of the six neutral events.
///
/// # Examples
///
/// ```
/// use agentstacks::types::HookEvent;
///
/// let refused = HookEvent::parse("tool.pre").err();
/// assert_eq!(
///     refused.as_ref().map(ToString::to_string).as_deref(),
///     Some("\"tool.pre\" unknown")
/// );
/// assert_eq!(refused.as_ref().map(|error| error.value()), Some("tool.pre"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("\"{0}\" unknown")]
pub struct InvalidHookEvent(String);

impl InvalidHookEvent {
    /// An error for the refused `value`.
    pub(crate) fn new(value: &str) -> Self {
        Self(value.to_owned())
    }

    /// The refused value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

/// A value that is not a [`Policy`](crate::types::Policy).
///
/// # Examples
///
/// ```
/// use agentstacks::types::Policy;
///
/// let refused = Policy::parse("trusted").err().ok_or("expected a refusal")?;
/// assert_eq!(refused.to_string(), "\"trusted\" is not allowed for pack hooks");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidPolicy {
    /// The grant policy `trusted`, which a pack hook may not use.
    #[error("\"{text}\" is not allowed for pack hooks")]
    NotAllowed {
        /// The refused text.
        text: String,
    },
    /// A grant policy outside the two a pack hook may use.
    #[error("\"{text}\" (expected confined, pure)")]
    Unknown {
        /// The refused text.
        text: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_hook_event_display_quotes_the_value() {
        let error = InvalidHookEvent::new("tool.pre");
        assert_eq!(error.to_string(), "\"tool.pre\" unknown");
        assert_eq!(error.value(), "tool.pre");
    }

    #[test]
    fn invalid_policy_displays_each_message() {
        let cases = [
            (
                InvalidPolicy::NotAllowed {
                    text: "trusted".to_owned(),
                },
                "\"trusted\" is not allowed for pack hooks",
            ),
            (
                InvalidPolicy::Unknown {
                    text: "foo".to_owned(),
                },
                "\"foo\" (expected confined, pure)",
            ),
        ];
        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }
}
