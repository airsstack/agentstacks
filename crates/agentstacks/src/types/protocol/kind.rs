//! Where a hook was declared.
//!
//! # Responsibilities
//!
//! - [`HookKind`] names the two places a Lua hook is declared and the argument the runtime reads
//!   to tell them apart.

/// Where a Lua hook was declared, which decides the event the runtime hands it.
///
/// # Examples
///
/// ```
/// use agentstacks::types::protocol::HookKind;
///
/// assert_eq!(HookKind::Neutral.as_arg(), "neutral");
/// assert_eq!(HookKind::Native.as_arg(), "native");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookKind {
    /// A `[[hooks]]` entry in `pack.toml`; the hook gets a tool-neutral event.
    Neutral,
    /// An entry in `native/<tool>/hooks.toml`; the hook gets the tool's raw payload.
    Native,
}

impl HookKind {
    /// The command-line argument that carries this kind.
    #[must_use]
    pub const fn as_arg(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Native => "native",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_kind_args() {
        assert_eq!(HookKind::Neutral.as_arg(), "neutral");
        assert_eq!(HookKind::Native.as_arg(), "native");
    }
}
