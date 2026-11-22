//! Every refusal of an operation, a call into the extension API, or an extension's handlers.
//!
//! The enum's `Display` is the text that follows the location. The location is never part of the
//! error: a refusal is a [`Diagnostic`](crate::types::Diagnostic), a [`Location`] paired with the
//! [`FormatError`](crate::types::FormatError) that carries one of these.
//!
//! # Responsibilities
//!
//! - [`OpError`] has one variant per message. The variants fall in groups by the location they
//!   are shown at: a call in an extension file (`<tool>/<file>:<line>`), the extension entry
//!   (`<tool>/<entry>`, with a `values` key path for some), and `agentstacks.toml`.
//! - A call refusal starts with the name of the function the extension called.
//!
//! # Non-responsibilities
//!
//! Deciding which refusal applies belongs to the code that validates the calls, the handlers'
//! return values and the proposed operations. The `required` and `unknown key` refusals of a
//! `values` key are [`FieldError`](crate::types::FieldError), whose location already names the
//! key.

use std::fmt::Display;

use crate::types::name::{BinaryName, InvalidName, PackName, ToolName};
use crate::types::op::{ApiFunction, LuaType, PathShape};
use crate::types::report::{Location, Quoted};

/// The suffix a path refusal adds after its field: `: empty` for an empty path, a space and the
/// shape's own words for every other shape.
fn path_suffix(shape: &PathShape) -> String {
    match shape {
        PathShape::Empty => ": empty".to_owned(),
        other => format!(" {other}"),
    }
}

/// `items` displayed and joined with `", "`.
fn join_names<T: Display>(items: &[T]) -> String {
    items
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// A refusal of an operation or of an extension's handlers.
///
/// Compare displayed text rather than values: `std::io::Error` does not implement `PartialEq`.
///
/// # Examples
///
/// ```
/// use agentstacks::types::op::OpError;
///
/// assert_eq!(OpError::NoPlanHandler.to_string(), "registers no plan handler");
/// ```
#[derive(Debug, thiserror::Error)]
pub enum OpError {
    /// A function was called with something other than the one table (or string) it takes.
    #[error("{function}: expected {expected}, got {got}")]
    ArgumentType {
        /// The function called.
        function: ApiFunction,
        /// The type the function takes.
        expected: LuaType,
        /// The type it was given.
        got: LuaType,
    },
    /// A required field is missing.
    #[error("{function}: {field}: required")]
    Required {
        /// The function called.
        function: ApiFunction,
        /// The field, as written.
        field: String,
    },
    /// A field has the wrong type.
    #[error("{function}: {field}: expected {expected}, got {got}")]
    FieldType {
        /// The function called.
        function: ApiFunction,
        /// The field, as written, with an index for a list entry (`covers[2]`).
        field: String,
        /// The type the field takes.
        expected: LuaType,
        /// The type it was given.
        got: LuaType,
    },
    /// A field that must be a list is not.
    #[error("{function}: {field}: expected a list")]
    NotList {
        /// The function called.
        function: ApiFunction,
        /// The field, as written.
        field: String,
    },
    /// A table key is not a string.
    #[error("{function}: keys must be strings, got {got}")]
    KeyType {
        /// The function called.
        function: ApiFunction,
        /// The type the key has.
        got: LuaType,
    },
    /// A field the function does not take.
    #[error("{function}: {key}: unknown key")]
    UnknownKey {
        /// The function called.
        function: ApiFunction,
        /// The key, as written.
        key: String,
    },
    /// A string field is not UTF-8.
    #[error("{function}: {field}: not valid UTF-8")]
    NotUtf8 {
        /// The function called.
        function: ApiFunction,
        /// The field, as written.
        field: String,
    },
    /// A string field is empty.
    #[error("{function}: {field}: empty")]
    Empty {
        /// The function called.
        function: ApiFunction,
        /// The field, as written.
        field: String,
    },
    /// A path field has a refused shape. An empty path reads `<field>: empty`; every other shape
    /// reads `<field> <shape>`.
    #[error("{function}: {field}{}", path_suffix(.shape))]
    OpPath {
        /// The function called.
        function: ApiFunction,
        /// The field, as written.
        field: String,
        /// Why the path is refused.
        shape: PathShape,
    },
    /// A path that agentstacks writes itself.
    #[error("{function}: path {path} is managed by agentstacks")]
    PathManaged {
        /// The function called.
        function: ApiFunction,
        /// The path, as written.
        path: Quoted,
    },
    /// A path an earlier call in the same extension already proposed.
    #[error("{function}: path {path} is already proposed at {earlier}")]
    ProposedAt {
        /// The function called.
        function: ApiFunction,
        /// The path, as written.
        path: Quoted,
        /// Where the earlier call is.
        earlier: Location,
    },
    /// A path another tool already proposed.
    #[error("{function}: path {path} is already proposed by {tool}")]
    ProposedByTool {
        /// The function called.
        function: ApiFunction,
        /// The path, as written.
        path: Quoted,
        /// The tool that proposed it.
        tool: ToolName,
    },
    /// A path the core writes for a pack's root file.
    #[error("{function}: path {path} is already proposed by core (root file of {pack})")]
    ProposedByCore {
        /// The function called.
        function: ApiFunction,
        /// The path, as written.
        path: Quoted,
        /// The pack whose root file it is.
        pack: PackName,
    },
    /// A path the core writes for the hook runtime of a tool.
    #[error("{function}: path {path} is already proposed by core (runtime.lua of {tool})")]
    ProposedByCoreRuntime {
        /// The function called.
        function: ApiFunction,
        /// The path, as written.
        path: Quoted,
        /// The tool whose runtime it is.
        tool: ToolName,
    },
    /// An id that is not in this run's payload.
    #[error("{function}: {field} {id} is not in this plan")]
    NotInPlan {
        /// The function called.
        function: ApiFunction,
        /// The field, as written, with an index for a list entry (`covers[1]`).
        field: String,
        /// The id, as written.
        id: Quoted,
    },
    /// A `copy` of an id that is not a file.
    #[error("copy: file {id} is not a file")]
    NotAFile {
        /// The id, as written.
        id: Quoted,
    },
    /// A `write` whose `covers` names a skill instead of its files.
    #[error("write: {field}: {id} is a skill; list its files")]
    CoversSkill {
        /// The `covers` entry, as written (`covers[<i>]`).
        field: String,
        /// The id, as written.
        id: Quoted,
    },
    /// A `require_binary` name that is not a binary name.
    #[error("require_binary: name: {0}")]
    InvalidBinaryName(InvalidName),
    /// A `read` of a path the extension did not declare in `reads`.
    #[error("read: {path} is not in extension.reads")]
    NotInReads {
        /// The path, as written.
        path: Quoted,
    },
    /// A function called outside the `plan` handler.
    #[error("{function}: only allowed inside the plan handler")]
    OutsidePlanHandler {
        /// The function called.
        function: ApiFunction,
    },
    /// A `read` whose path resolves outside the project.
    #[error("read: {path} resolves outside the project")]
    ReadOutside {
        /// The path, as written.
        path: Quoted,
    },
    /// A `skip` of an item an earlier call already covers.
    #[error("skip: item {item} is already covered at {earlier}")]
    SkipAfterCover {
        /// The item, as written.
        item: Quoted,
        /// Where the covering call is.
        earlier: Location,
    },
    /// A call that covers an item an earlier `skip` skipped.
    #[error("{function}: covers {id}, which {earlier} skipped")]
    CoverAfterSkip {
        /// The function called.
        function: ApiFunction,
        /// The id covered (the `file` of a `copy`).
        id: Quoted,
        /// Where the `skip` is.
        earlier: Location,
    },
    /// A module name written with `/`; the message offers the dotted form.
    #[error("require: invalid module name {name} (use dots: {dotted})")]
    ModuleNameDotted {
        /// The name, as written.
        name: Quoted,
        /// The same name with dots.
        dotted: String,
    },
    /// A module name that is not a valid module name.
    #[error("require: invalid module name {name}")]
    ModuleName {
        /// The name, as written.
        name: Quoted,
    },
    /// A module the extension does not hold.
    #[error("require: module {module} not found in extension {tool}")]
    ModuleNotFound {
        /// The module, as written.
        module: Quoted,
        /// The extension's tool.
        tool: ToolName,
    },
    /// A `require` of the entry file.
    #[error("require: {name} is the entry file")]
    EntryModule {
        /// The name, as written.
        name: Quoted,
    },
    /// A module that requires itself, directly or through others.
    #[error("require: module {module} requires itself ({})", .chain.join(" \u{2192} "))]
    ModuleCycle {
        /// The module the call closes the cycle on.
        module: Quoted,
        /// The modules in the order they were required, ending with `module`.
        chain: Vec<String>,
    },
    /// A binary the extension requires that is not on `PATH`.
    #[error("require_binary: \"{name}\" not found on PATH")]
    BinaryNotFound {
        /// The binary.
        name: BinaryName,
    },
    /// An item the extension proposed nothing for.
    #[error("{id} got no operation (copy it, write it, or skip it with a reason)")]
    Uncovered {
        /// The item id.
        id: String,
    },
    /// A `lua` hook is in the run but the extension did not write the translator.
    #[error("lua hooks need {root}/translate.lua (write it)")]
    TranslateMissing {
        /// The hook runtime root.
        root: String,
    },
    /// The extension registered no `values` handler.
    #[error("registers no values handler")]
    NoValuesHandler,
    /// The extension registered no `plan` handler.
    #[error("registers no plan handler")]
    NoPlanHandler,
    /// A failure airsl reported, in its own first line.
    #[error("{first_line}")]
    Lua {
        /// The first line airsl printed.
        first_line: String,
    },
    /// A `values` entry has the wrong type.
    #[error("expected {expected}, got {got}")]
    ValuesType {
        /// The type the entry takes.
        expected: LuaType,
        /// The type it has.
        got: LuaType,
    },
    /// A `values` path has a refused shape. The key is part of the text, so this is located at
    /// the extension entry alone.
    #[error("{key}: {shape}")]
    ValuesPath {
        /// The key path, as written (`values.root`).
        key: String,
        /// Why the path is refused.
        shape: PathShape,
    },
    /// A `values.skills` key that is not a skill in this run.
    #[error("not a skill in this run")]
    NotASkill,
    /// A `values.agents` key that is not an agent in this run.
    #[error("not an agent in this run")]
    NotAnAgent,
    /// A `values.vars` key that is not a var in this run.
    #[error("not a var in this run")]
    NotAVar,
    /// A pack requires a binary that is not on `PATH`.
    #[error("binary \"{name}\" not found on PATH")]
    PackBinaryNotFound {
        /// The binary.
        name: BinaryName,
    },
    /// A file the core writes that a tool already proposed.
    #[error("path {path} is already proposed by {tool}")]
    CoreProposedByTool {
        /// The path, as written.
        path: Quoted,
        /// The tool that proposed it.
        tool: ToolName,
    },
    /// `airsl` is needed and not on `PATH`.
    #[error("airsl not found on PATH (needed by {})", join_names(.packs))]
    AirslNotFound {
        /// The packs that need it.
        packs: Vec<PackName>,
    },
    /// `airsl --version` printed text that is not `airsl <version>`.
    #[error("airsl --version printed \"{text}\", expected \"airsl <version>\"")]
    AirslVersionText {
        /// What it printed, trimmed.
        text: String,
    },
    /// `airsl` could not be started.
    #[error("airsl: {0}")]
    AirslSpawn(std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::io::{Error, ErrorKind};
    use std::num::NonZeroUsize;

    use super::*;
    use crate::types::name::{BinaryName, PackName, ToolName};
    use crate::types::report::{
        Diagnostic, FieldError, FormatError, Location, MustBeRelative, OutsideProject,
    };
    use crate::types::tree::utf8_text;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const ADAPTER: &str = "claude/adapter.lua";

    fn line(file: &str, n: usize) -> Location {
        Location::at_line(file, NonZeroUsize::MIN.saturating_add(n - 1))
    }

    fn at_adapter(n: usize) -> Location {
        line(ADAPTER, n)
    }

    fn key(file: &str, key_path: &str) -> Location {
        Location::at_key(file, key_path)
    }

    fn file(path: &str) -> Location {
        Location::file(path)
    }

    fn q(text: &str) -> Quoted {
        Quoted::new(text)
    }

    fn show(location: Location, error: impl Into<FormatError>) -> String {
        Diagnostic::new(location, error).to_string()
    }

    fn shown_at(n: usize, error: OpError) -> String {
        show(at_adapter(n), error)
    }

    #[test]
    fn op_error_uncovered_item() {
        let item = OpError::Uncovered {
            id: "core/skill/status/references/x.md".to_owned(),
        };
        assert_eq!(
            show(file(ADAPTER), item),
            "claude/adapter.lua: core/skill/status/references/x.md got no operation (copy it, write it, or skip it with a reason)"
        );
        let agent = OpError::Uncovered {
            id: "core/agent/coder".to_owned(),
        };
        assert_eq!(
            show(file(ADAPTER), agent),
            "claude/adapter.lua: core/agent/coder got no operation (copy it, write it, or skip it with a reason)"
        );
    }

    #[test]
    fn op_error_skip_after_cover() {
        let error = OpError::SkipAfterCover {
            item: q("core/skill/status"),
            earlier: line("adapter.lua", 9),
        };
        assert_eq!(
            show(line(ADAPTER, 14), error),
            "claude/adapter.lua:14: skip: item \"core/skill/status\" is already covered at adapter.lua:9"
        );
    }

    #[test]
    fn op_error_cover_after_skip() {
        let write = OpError::CoverAfterSkip {
            function: ApiFunction::Write,
            id: q("core/agent/coder"),
            earlier: line("adapter.lua", 9),
        };
        assert_eq!(
            show(line(ADAPTER, 14), write),
            "claude/adapter.lua:14: write: covers \"core/agent/coder\", which adapter.lua:9 skipped"
        );
        let copy = OpError::CoverAfterSkip {
            function: ApiFunction::Copy,
            id: q("core/skill/status/SKILL.md"),
            earlier: line("adapter.lua", 9),
        };
        assert_eq!(
            show(line(ADAPTER, 14), copy),
            "claude/adapter.lua:14: copy: covers \"core/skill/status/SKILL.md\", which adapter.lua:9 skipped"
        );
    }

    #[test]
    fn op_error_translate_missing() {
        let error = OpError::TranslateMissing {
            root: ".claude/agentstacks".to_owned(),
        };
        assert_eq!(
            show(file(ADAPTER), error),
            "claude/adapter.lua: lua hooks need .claude/agentstacks/translate.lua (write it)"
        );
    }

    #[test]
    fn op_error_argument_type() {
        let write = OpError::ArgumentType {
            function: ApiFunction::Write,
            expected: LuaType::Table,
            got: LuaType::String,
        };
        assert_eq!(
            shown_at(12, write),
            "claude/adapter.lua:12: write: expected table, got string"
        );
        let read = OpError::ArgumentType {
            function: ApiFunction::Read,
            expected: LuaType::String,
            got: LuaType::Number,
        };
        assert_eq!(
            shown_at(9, read),
            "claude/adapter.lua:9: read: expected string, got number"
        );
        let require = OpError::ArgumentType {
            function: ApiFunction::Require,
            expected: LuaType::String,
            got: LuaType::Number,
        };
        assert_eq!(
            shown_at(1, require),
            "claude/adapter.lua:1: require: expected string, got number"
        );
    }

    #[test]
    fn op_error_required_field() {
        let cases = [
            (ApiFunction::Write, "path"),
            (ApiFunction::Write, "content"),
            (ApiFunction::Copy, "file"),
            (ApiFunction::Skip, "item"),
            (ApiFunction::Skip, "reason"),
            (ApiFunction::RequireBinary, "name"),
        ];
        for (function, field) in cases {
            let error = OpError::Required {
                function,
                field: field.to_owned(),
            };
            assert_eq!(
                shown_at(12, error),
                format!("claude/adapter.lua:12: {function}: {field}: required")
            );
        }
        let error = OpError::Required {
            function: ApiFunction::Skip,
            field: "reason".to_owned(),
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: skip: reason: required"
        );
    }

    #[test]
    fn op_error_field_type() {
        let cases = [
            (
                "path",
                LuaType::String,
                LuaType::Number,
                "path: expected string, got number",
            ),
            (
                "covers",
                LuaType::Table,
                LuaType::String,
                "covers: expected table, got string",
            ),
            (
                "covers[2]",
                LuaType::String,
                LuaType::Boolean,
                "covers[2]: expected string, got boolean",
            ),
        ];
        for (field, expected, got, text) in cases {
            let error = OpError::FieldType {
                function: ApiFunction::Write,
                field: field.to_owned(),
                expected,
                got,
            };
            assert_eq!(
                shown_at(12, error),
                format!("claude/adapter.lua:12: write: {text}")
            );
        }
    }

    #[test]
    fn op_error_field_not_a_list() {
        let error = OpError::NotList {
            function: ApiFunction::Write,
            field: "covers".to_owned(),
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: write: covers: expected a list"
        );
    }

    #[test]
    fn op_error_keys_must_be_strings() {
        let error = OpError::KeyType {
            function: ApiFunction::Write,
            got: LuaType::Number,
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: write: keys must be strings, got number"
        );
    }

    #[test]
    fn op_error_unknown_key() {
        let error = OpError::UnknownKey {
            function: ApiFunction::Write,
            key: "foo".to_owned(),
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: write: foo: unknown key"
        );
    }

    #[test]
    fn op_error_field_not_utf8() {
        let path = OpError::NotUtf8 {
            function: ApiFunction::Write,
            field: "path".to_owned(),
        };
        assert_eq!(
            shown_at(12, path),
            "claude/adapter.lua:12: write: path: not valid UTF-8"
        );
        let reason = OpError::NotUtf8 {
            function: ApiFunction::Skip,
            field: "reason".to_owned(),
        };
        assert_eq!(
            shown_at(12, reason),
            "claude/adapter.lua:12: skip: reason: not valid UTF-8"
        );
    }

    fn op_path(shape: PathShape) -> OpError {
        OpError::OpPath {
            function: ApiFunction::Write,
            field: "path".to_owned(),
            shape,
        }
    }

    #[test]
    fn op_error_op_path_empty() {
        assert_eq!(
            shown_at(12, op_path(PathShape::Empty)),
            "claude/adapter.lua:12: write: path: empty"
        );
    }

    #[test]
    fn op_error_op_path_must_be_relative() {
        let shape = PathShape::MustBeRelative(MustBeRelative::new("/x"));
        assert_eq!(
            shown_at(12, op_path(shape)),
            "claude/adapter.lua:12: write: path \"/x\" must be relative"
        );
        let shape = PathShape::MustBeRelative(MustBeRelative::new("/etc/x"));
        assert_eq!(
            shown_at(12, op_path(shape)),
            "claude/adapter.lua:12: write: path \"/etc/x\" must be relative"
        );
    }

    #[test]
    fn op_error_op_path_outside_project() {
        let shape = PathShape::OutsideProject(OutsideProject::new("../x"));
        assert_eq!(
            shown_at(12, op_path(shape)),
            "claude/adapter.lua:12: write: path \"../x\" must stay inside the project"
        );
    }

    #[test]
    fn op_error_op_path_separator() {
        let shape = PathShape::Separator(q("a\\b"));
        assert_eq!(
            shown_at(12, op_path(shape)),
            "claude/adapter.lua:12: write: path \"a\\b\" must use / as the separator"
        );
    }

    #[test]
    fn op_error_path_managed() {
        for path in ["agentstacks.lock", "agentstacks.toml"] {
            let error = OpError::PathManaged {
                function: ApiFunction::Write,
                path: q(path),
            };
            assert_eq!(
                shown_at(12, error),
                format!("claude/adapter.lua:12: write: path \"{path}\" is managed by agentstacks")
            );
        }
    }

    #[test]
    fn op_error_path_proposed_at() {
        let error = OpError::ProposedAt {
            function: ApiFunction::Write,
            path: q(".claude/x"),
            earlier: line("adapter.lua", 9),
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: write: path \".claude/x\" is already proposed at adapter.lua:9"
        );
    }

    #[test]
    fn op_error_path_proposed_by_tool() -> TestResult {
        let error = OpError::ProposedByTool {
            function: ApiFunction::Write,
            path: q(".claude/x"),
            tool: ToolName::new("opencode")?,
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: write: path \".claude/x\" is already proposed by opencode"
        );
        Ok(())
    }

    #[test]
    fn op_error_path_proposed_by_core() -> TestResult {
        let error = OpError::ProposedByCore {
            function: ApiFunction::Write,
            path: q(".claude/x"),
            pack: PackName::new("core")?,
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: write: path \".claude/x\" is already proposed by core (root file of core)"
        );
        Ok(())
    }

    #[test]
    fn op_error_path_proposed_by_core_runtime() -> TestResult {
        let error = OpError::ProposedByCoreRuntime {
            function: ApiFunction::Write,
            path: q(".claude/agentstacks/runtime.lua"),
            tool: ToolName::new("claude")?,
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: write: path \".claude/agentstacks/runtime.lua\" is already proposed by core (runtime.lua of claude)"
        );
        Ok(())
    }

    #[test]
    fn op_error_core_write_proposed_by_tool() -> TestResult {
        let error = OpError::CoreProposedByTool {
            path: q(".claude/x"),
            tool: ToolName::new("claude")?,
        };
        assert_eq!(
            show(key("agentstacks.toml", "tools.opencode"), error),
            "agentstacks.toml: tools.opencode: path \".claude/x\" is already proposed by claude"
        );
        Ok(())
    }

    #[test]
    fn op_error_id_not_in_plan() {
        let cases = [
            (ApiFunction::Copy, "file", "copy: file"),
            (ApiFunction::Skip, "item", "skip: item"),
            (ApiFunction::Write, "covers[1]", "write: covers[1]"),
        ];
        for (function, field, head) in cases {
            let error = OpError::NotInPlan {
                function,
                field: field.to_owned(),
                id: q("core/skill/x/SKILL.md"),
            };
            assert_eq!(
                shown_at(12, error),
                format!(
                    "claude/adapter.lua:12: {head} \"core/skill/x/SKILL.md\" is not in this plan"
                )
            );
        }
    }

    #[test]
    fn op_error_copy_needs_a_file() {
        let error = OpError::NotAFile {
            id: q("core/agent/coder"),
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: copy: file \"core/agent/coder\" is not a file"
        );
    }

    #[test]
    fn op_error_covers_names_a_skill() {
        let error = OpError::CoversSkill {
            field: "covers[1]".to_owned(),
            id: q("core/skill/status"),
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: write: covers[1]: \"core/skill/status\" is a skill; list its files"
        );
    }

    #[test]
    fn op_error_reason_empty() {
        let error = OpError::Empty {
            function: ApiFunction::Skip,
            field: "reason".to_owned(),
        };
        assert_eq!(
            shown_at(12, error),
            "claude/adapter.lua:12: skip: reason: empty"
        );
    }

    #[test]
    fn op_error_binary_name_invalid() -> TestResult {
        let refused = BinaryName::new("Git")
            .err()
            .ok_or("Git is not a valid binary name")?;
        assert_eq!(
            shown_at(12, OpError::InvalidBinaryName(refused)),
            "claude/adapter.lua:12: require_binary: name: \"Git\" is not a valid name"
        );
        Ok(())
    }

    #[test]
    fn op_error_read_not_in_reads() {
        let error = OpError::NotInReads { path: q(".env") };
        assert_eq!(
            shown_at(9, error),
            "claude/adapter.lua:9: read: \".env\" is not in extension.reads"
        );
    }

    #[test]
    fn op_error_outside_plan_handler() {
        for function in [ApiFunction::Write, ApiFunction::Read] {
            let error = OpError::OutsidePlanHandler { function };
            assert_eq!(
                shown_at(3, error),
                format!("claude/adapter.lua:3: {function}: only allowed inside the plan handler")
            );
        }
    }

    #[test]
    fn op_error_read_resolves_outside() {
        let error = OpError::ReadOutside {
            path: q(".claude/settings.json"),
        };
        assert_eq!(
            shown_at(9, error),
            "claude/adapter.lua:9: read: \".claude/settings.json\" resolves outside the project"
        );
    }

    #[test]
    fn op_error_no_values_handler() {
        assert_eq!(
            show(file(ADAPTER), OpError::NoValuesHandler),
            "claude/adapter.lua: registers no values handler"
        );
    }

    #[test]
    fn op_error_no_plan_handler() {
        assert_eq!(
            show(file(ADAPTER), OpError::NoPlanHandler),
            "claude/adapter.lua: registers no plan handler"
        );
    }

    #[test]
    fn op_error_values_type() {
        let cases = [
            (
                "values",
                LuaType::Table,
                LuaType::Nil,
                "values: expected table, got nil",
            ),
            (
                "values.agents",
                LuaType::Table,
                LuaType::List,
                "values.agents: expected table, got list",
            ),
            (
                "values.skills.\"core/status\".dir",
                LuaType::String,
                LuaType::Number,
                "values.skills.\"core/status\".dir: expected string, got number",
            ),
        ];
        for (key_path, expected, got, text) in cases {
            assert_eq!(
                show(
                    key(ADAPTER, key_path),
                    OpError::ValuesType { expected, got }
                ),
                format!("claude/adapter.lua: {text}")
            );
        }
    }

    #[test]
    fn values_required_is_the_field_shape() {
        for key_path in [
            "values.root",
            "values.skills.\"core/status\".dir",
            "values.skills.\"core/status\".invoke",
        ] {
            assert_eq!(
                show(key(ADAPTER, key_path), FieldError::Required),
                format!("claude/adapter.lua: {key_path}: required")
            );
        }
    }

    fn values_path(shape: PathShape) -> OpError {
        OpError::ValuesPath {
            key: "values.root".to_owned(),
            shape,
        }
    }

    #[test]
    fn op_error_values_path_empty() {
        assert_eq!(
            show(file(ADAPTER), values_path(PathShape::Empty)),
            "claude/adapter.lua: values.root: empty"
        );
    }

    #[test]
    fn values_unknown_key_is_the_field_shape() {
        for key_path in ["values.foo", "values.skills.\"core/status\".icon"] {
            assert_eq!(
                show(key(ADAPTER, key_path), FieldError::UnknownKey),
                format!("claude/adapter.lua: {key_path}: unknown key")
            );
        }
    }

    #[test]
    fn op_error_values_path_must_be_relative() {
        let shape = PathShape::MustBeRelative(MustBeRelative::new("/x"));
        assert_eq!(
            show(file(ADAPTER), values_path(shape)),
            "claude/adapter.lua: values.root: \"/x\" must be relative"
        );
    }

    #[test]
    fn op_error_values_path_outside_project() {
        let shape = PathShape::OutsideProject(OutsideProject::new("../x"));
        assert_eq!(
            show(file(ADAPTER), values_path(shape)),
            "claude/adapter.lua: values.root: \"../x\" must stay inside the project"
        );
    }

    #[test]
    fn op_error_values_path_separator() {
        let shape = PathShape::Separator(q("a\\b"));
        assert_eq!(
            show(file(ADAPTER), values_path(shape)),
            "claude/adapter.lua: values.root: \"a\\b\" must use / as the separator"
        );
    }

    #[test]
    fn op_error_not_a_skill_in_run() {
        assert_eq!(
            show(key(ADAPTER, "values.skills.\"core/x\""), OpError::NotASkill),
            "claude/adapter.lua: values.skills.\"core/x\": not a skill in this run"
        );
    }

    #[test]
    fn op_error_not_an_agent_in_run() {
        assert_eq!(
            show(
                key(ADAPTER, "values.agents.\"core/x\""),
                OpError::NotAnAgent
            ),
            "claude/adapter.lua: values.agents.\"core/x\": not an agent in this run"
        );
    }

    #[test]
    fn op_error_not_a_var_in_run() {
        assert_eq!(
            show(key(ADAPTER, "values.vars.home"), OpError::NotAVar),
            "claude/adapter.lua: values.vars.home: not a var in this run"
        );
    }

    #[test]
    fn op_error_module_name_with_slash() {
        let error = OpError::ModuleNameDotted {
            name: q("lib/x"),
            dotted: "lib.x".to_owned(),
        };
        assert_eq!(
            shown_at(1, error),
            "claude/adapter.lua:1: require: invalid module name \"lib/x\" (use dots: lib.x)"
        );
    }

    #[test]
    fn op_error_module_name_without_slash() {
        let error = OpError::ModuleName { name: q("x y") };
        assert_eq!(
            shown_at(1, error),
            "claude/adapter.lua:1: require: invalid module name \"x y\""
        );
    }

    #[test]
    fn op_error_module_not_found() -> TestResult {
        let error = OpError::ModuleNotFound {
            module: q("lib.x"),
            tool: ToolName::new("claude")?,
        };
        assert_eq!(
            shown_at(1, error),
            "claude/adapter.lua:1: require: module \"lib.x\" not found in extension claude"
        );
        Ok(())
    }

    #[test]
    fn op_error_entry_module() {
        let error = OpError::EntryModule { name: q("adapter") };
        assert_eq!(
            shown_at(1, error),
            "claude/adapter.lua:1: require: \"adapter\" is the entry file"
        );
    }

    #[test]
    fn op_error_module_cycle() {
        let error = OpError::ModuleCycle {
            module: q("lib.a"),
            chain: vec!["lib.a".to_owned(), "lib.b".to_owned(), "lib.a".to_owned()],
        };
        assert_eq!(
            show(line("claude/lib/b.lua", 1), error),
            "claude/lib/b.lua:1: require: module \"lib.a\" requires itself (lib.a \u{2192} lib.b \u{2192} lib.a)"
        );
    }

    #[test]
    fn op_error_lua_failure_keeps_airsl_text() {
        let syntax = OpError::Lua {
            first_line: "lua error in adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>".to_owned(),
        };
        assert_eq!(
            show(file(ADAPTER), syntax),
            "claude/adapter.lua: lua error in adapter.lua: syntax error: adapter.lua:1: <name> or '...' expected near <eof>"
        );
        let runtime = OpError::Lua {
            first_line: "lua error in plan: adapter.lua:3: path must be relative".to_owned(),
        };
        assert_eq!(
            show(file(ADAPTER), runtime),
            "claude/adapter.lua: lua error in plan: adapter.lua:3: path must be relative"
        );
    }

    #[test]
    fn op_error_limit_breach_keeps_airsl_text() {
        let error = OpError::Lua {
            first_line: "script `plan` exceeded its instruction limit of 10000000".to_owned(),
        };
        assert_eq!(
            show(file(ADAPTER), error),
            "claude/adapter.lua: script `plan` exceeded its instruction limit of 10000000"
        );
    }

    #[test]
    fn op_error_unknown_event_keeps_airsl_text() {
        let error = OpError::Lua {
            first_line: "lua error in adapter.lua: event `nope` is not one this host dispatches; declared events: plan, values".to_owned(),
        };
        assert_eq!(
            show(file(ADAPTER), error),
            "claude/adapter.lua: lua error in adapter.lua: event `nope` is not one this host dispatches; declared events: plan, values"
        );
    }

    #[test]
    fn extension_file_not_utf8_is_the_shared_text_refusal() -> TestResult {
        let refused = utf8_text(vec![0x66, 0xFF])
            .err()
            .ok_or("the bytes are not UTF-8")?;
        assert_eq!(
            show(file("claude/lib/x.lua"), refused),
            "claude/lib/x.lua: not valid UTF-8"
        );
        Ok(())
    }

    #[test]
    fn op_error_pack_binary_not_found() -> TestResult {
        let error = OpError::PackBinaryNotFound {
            name: BinaryName::new("rg")?,
        };
        assert_eq!(
            show(key("agentstacks.toml", "packs.core"), error),
            "agentstacks.toml: packs.core: binary \"rg\" not found on PATH"
        );
        Ok(())
    }

    #[test]
    fn op_error_extension_binary_not_found() -> TestResult {
        let error = OpError::BinaryNotFound {
            name: BinaryName::new("node")?,
        };
        assert_eq!(
            shown_at(7, error),
            "claude/adapter.lua:7: require_binary: \"node\" not found on PATH"
        );
        Ok(())
    }

    #[test]
    fn op_error_airsl_not_found() -> TestResult {
        let error = OpError::AirslNotFound {
            packs: vec![PackName::new("core")?, PackName::new("sdlc")?],
        };
        assert_eq!(
            show(file("agentstacks.toml"), error),
            "agentstacks.toml: airsl not found on PATH (needed by core, sdlc)"
        );
        Ok(())
    }

    #[test]
    fn op_error_airsl_version_unparsed() {
        let error = OpError::AirslVersionText {
            text: "airsl-dev".to_owned(),
        };
        assert_eq!(
            show(file("agentstacks.toml"), error),
            "agentstacks.toml: airsl --version printed \"airsl-dev\", expected \"airsl <version>\""
        );
    }

    #[test]
    fn op_error_airsl_spawn() {
        let error =
            OpError::AirslSpawn(Error::new(ErrorKind::PermissionDenied, "permission denied"));
        assert_eq!(
            show(file("agentstacks.toml"), error),
            "agentstacks.toml: airsl: permission denied"
        );
    }
}
