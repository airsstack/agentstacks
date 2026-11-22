//! The closed words an operation refusal prints: Lua's type names and the names of the functions
//! an extension calls.
//!
//! # Responsibilities
//!
//! - [`LuaType`] is the type a Lua value has, as a Lua author reads it. It is a different closed
//!   set from [`TypeWord`](crate::types::TypeWord): a Lua refusal says `nil` and `table`, where a
//!   TOML or YAML refusal says `null` and `integer`, so the two do not share a type.
//! - [`ApiFunction`] names the function an extension called, as the author wrote it. It carries
//!   no payload and has no `delete`, which no function proposes.
//!
//! # Non-responsibilities
//!
//! The messages that use these words belong to [`OpError`](crate::types::op::error::OpError) and
//! the other refusal enums.

use std::fmt;

/// The type of a Lua value, as a refusal names it.
///
/// An integer and a float are both [`LuaType::Number`]; [`LuaType::List`] is how a JSON array is
/// named when it comes back from an extension's `values` handler.
///
/// # Examples
///
/// ```
/// use agentstacks::types::op::LuaType;
///
/// assert_eq!(LuaType::Nil.to_string(), "nil");
/// assert_eq!(LuaType::List.to_string(), "list");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuaType {
    /// `nil`.
    Nil,
    /// `boolean`.
    Boolean,
    /// `number`, whether an integer or a float.
    Number,
    /// `string`.
    String,
    /// `table`.
    Table,
    /// `function`.
    Function,
    /// `userdata`.
    Userdata,
    /// `thread`.
    Thread,
    /// `list`: a JSON array.
    List,
}

impl LuaType {
    /// The word as printed.
    const fn word(self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::String => "string",
            Self::Table => "table",
            Self::Function => "function",
            Self::Userdata => "userdata",
            Self::Thread => "thread",
            Self::List => "list",
        }
    }
}

impl fmt::Display for LuaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// A function an extension calls, as its author writes the name.
///
/// # Examples
///
/// ```
/// use agentstacks::types::op::ApiFunction;
///
/// assert_eq!(ApiFunction::RequireBinary.to_string(), "require_binary");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiFunction {
    /// `write`.
    Write,
    /// `copy`.
    Copy,
    /// `skip`.
    Skip,
    /// `require_binary`.
    RequireBinary,
    /// `read`.
    Read,
    /// `require`.
    Require,
}

impl ApiFunction {
    /// The name as printed.
    const fn name(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Copy => "copy",
            Self::Skip => "skip",
            Self::RequireBinary => "require_binary",
            Self::Read => "read",
            Self::Require => "require",
        }
    }
}

impl fmt::Display for ApiFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_type_displays_each_word() {
        let words = [
            (LuaType::Nil, "nil"),
            (LuaType::Boolean, "boolean"),
            (LuaType::Number, "number"),
            (LuaType::String, "string"),
            (LuaType::Table, "table"),
            (LuaType::Function, "function"),
            (LuaType::Userdata, "userdata"),
            (LuaType::Thread, "thread"),
            (LuaType::List, "list"),
        ];
        for (variant, word) in words {
            assert_eq!(variant.to_string(), word);
        }
    }

    #[test]
    fn api_function_displays_each_name() {
        let names = [
            (ApiFunction::Write, "write"),
            (ApiFunction::Copy, "copy"),
            (ApiFunction::Skip, "skip"),
            (ApiFunction::RequireBinary, "require_binary"),
            (ApiFunction::Read, "read"),
            (ApiFunction::Require, "require"),
        ];
        for (variant, name) in names {
            assert_eq!(variant.to_string(), name);
        }
    }
}
