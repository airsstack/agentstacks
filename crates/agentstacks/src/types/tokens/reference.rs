//! A reference to a skill, agent or skill directory: `[pack/]item`.
//!
//! The `skill`, `agent` and `skill_dir` tokens all point at the same `[pack/]item` shape. It is
//! read here once so the three kinds cannot read it differently.
//!
//! # Responsibilities
//!
//! - [`ItemRef`] holds an optional pack and an item name, both validated on construction.
//! - [`ItemRef::parse`] reads the `[pack/]item` text of a token target.
//!
//! # Non-responsibilities
//!
//! Whether the pack or the item exists belongs to the checks over a whole pack and a whole set of
//! packs.

use std::fmt;

use crate::types::name::ItemName;
use crate::types::name::PackName;
use crate::types::tokens::error::TokenError;

/// A `[pack/]item` reference; no pack means the pack the token sits in.
///
/// # Examples
///
/// ```
/// use agentstacks::types::ItemRef;
///
/// assert_eq!(ItemRef::parse("review")?.to_string(), "review");
/// assert_eq!(ItemRef::parse("core/review")?.to_string(), "core/review");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemRef {
    pack: Option<PackName>,
    item: ItemName,
}

impl ItemRef {
    /// Parses `[pack/]item`.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::InvalidTarget`] when `target` holds more than one `/`, or one `/`
    /// with an empty side. Otherwise returns [`TokenError::Name`] for the first side that is not a
    /// valid name, the pack side before the item side.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::ItemRef;
    ///
    /// assert!(ItemRef::parse("core/review").is_ok());
    /// let refused = ItemRef::parse("a/b/c").err().map(|error| error.to_string());
    /// assert_eq!(
    ///     refused.as_deref(),
    ///     Some("\"a/b/c\" is not a valid target (expected [pack/]name)"),
    /// );
    /// ```
    pub fn parse(target: &str) -> Result<Self, TokenError> {
        let invalid = || TokenError::InvalidTarget {
            target: target.to_owned(),
        };
        match target.split_once('/') {
            None => Ok(Self {
                pack: None,
                item: ItemName::new(target)?,
            }),
            Some((pack, item)) if !pack.is_empty() && !item.is_empty() && !item.contains('/') => {
                Ok(Self {
                    pack: Some(PackName::new(pack)?),
                    item: ItemName::new(item)?,
                })
            }
            Some(_) => Err(invalid()),
        }
    }

    /// The pack, when the reference names one.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::ItemRef;
    ///
    /// assert_eq!(ItemRef::parse("core/review")?.pack().map(|pack| pack.as_str()), Some("core"));
    /// assert!(ItemRef::parse("review")?.pack().is_none());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn pack(&self) -> Option<&PackName> {
        self.pack.as_ref()
    }

    /// The item name.
    ///
    /// # Examples
    ///
    /// ```
    /// use agentstacks::types::ItemRef;
    ///
    /// assert_eq!(ItemRef::parse("core/review")?.item().as_str(), "review");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub const fn item(&self) -> &ItemName {
        &self.item
    }
}

impl fmt::Display for ItemRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.pack {
            Some(pack) => write!(f, "{pack}/{}", self.item),
            None => write!(f, "{}", self.item),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_ref_parses_own_and_cross_pack() {
        let own = ItemRef::parse("discuss");
        assert_eq!(
            own.as_ref()
                .map(|r| (r.pack().map(ToString::to_string), r.item().to_string())),
            Ok((None, "discuss".to_owned()))
        );
        let cross = ItemRef::parse("sdlc/status");
        assert_eq!(
            cross
                .as_ref()
                .map(|r| (r.pack().map(ToString::to_string), r.item().to_string())),
            Ok((Some("sdlc".to_owned()), "status".to_owned()))
        );
        for text in ["discuss", "sdlc/status"] {
            assert_eq!(
                ItemRef::parse(text).map(|r| r.to_string()),
                Ok(text.to_owned())
            );
        }
    }

    #[test]
    fn item_ref_refuses_bad_shapes() {
        for target in ["a/b/c", "/x", "core/"] {
            assert_eq!(
                ItemRef::parse(target).map_err(|e| e.to_string()),
                Err(format!(
                    "\"{target}\" is not a valid target (expected [pack/]name)"
                ))
            );
        }
        assert_eq!(
            ItemRef::parse("Core/x").map_err(|e| e.to_string()),
            Err("\"Core\" is not a valid name".to_owned())
        );
    }
}
