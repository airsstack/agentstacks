//! The domain types and the logic over them: names, tokens, frontmatter, manifests, hooks,
//! diagnostics and the tree walk.
//!
//! Every other module builds on one shared set of types and rules, so they live in one place.
//!
//! # Responsibilities
//!
//! - [`api`]: the one `api` number this agentstacks speaks, which every file reader checks.
//! - [`report`]: where and what of every refusal, and the sorted collection a run reports.
//! - [`name`]: names that parse on construction (pack, tool, item, option, binary, environment).
//! - [`tokens`]: the token vocabulary, scanning text into located tokens, rendering values.
//! - [`format`](mod@format): generic TOML field access by key path, and an own YAML value type.
//! - [`frontmatter`]: skill and agent frontmatter, from the YAML block to every checked field.
//! - [`lock`]: the lockfile `agentstacks.lock`, parsed strictly, checked as a whole, written back.
//! - [`manifest`]: a pack's `pack.toml`: identity, requirements, render and exclude lists, options.
//! - [`hooks`]: neutral and tool-specific hooks, with their runs and grants.
//! - [`extension`]: a tool extension directory read into one checked extension.
//! - [`pack`]: a pack directory read into one checked pack, with every rule that needs the files.
//! - [`set`]: the checks over a whole project, ending in the [`ValidSet`] later stages accept.
//! - [`project`]: the project file `agentstacks.toml` and the source values it is built from.
//! - [`shorthand`]: classifying an `init` argument as an official name or a path.
//! - [`op`]: an operation a tool extension proposes, and the validation of it.
//! - [`json`]: editing the text of a JSON document at one key path.
//! - [`plan`]: the step for each path, decided from the operation, the lock and the disk state.
//! - [`resolve`]: reconciling the lock with `agentstacks.toml`, and choosing tags.
//! - [`protocol`]: the hook command line and the text of the hook runtime.
//! - [`tree`]: walking a directory tree under the layout rules, and hashing files and trees.
//!
//! # Non-responsibilities
//!
//! It touches nothing outside the process except through the read-only tree port that
//! `crate::port` declares; reading the real disk belongs to `crate::fs`.
//!
//! This file is an export-only module with no executable logic, so it carries no inline tests.

pub mod api;
pub mod extension;
pub mod format;
pub mod frontmatter;
pub mod hooks;
pub mod json;
pub mod lock;
pub mod manifest;
pub mod name;
pub mod op;
pub mod pack;
pub mod plan;
pub mod project;
pub mod protocol;
pub mod report;
pub mod resolve;
pub mod set;
pub mod shorthand;
pub mod tokens;
pub mod tree;

#[doc(inline)]
pub use api::API;
#[doc(inline)]
pub use extension::EntryPath;
#[doc(inline)]
pub use extension::Extension;
#[doc(inline)]
pub use extension::ExtensionError;
#[doc(inline)]
pub use extension::ExtensionManifest;
#[doc(inline)]
pub use format::YamlValue;
#[doc(inline)]
pub use frontmatter::AgentMeta;
#[doc(inline)]
pub use frontmatter::AgentTool;
#[doc(inline)]
pub use frontmatter::BodyStart;
#[doc(inline)]
pub use frontmatter::Effort;
#[doc(inline)]
pub use frontmatter::Frontmatter;
#[doc(inline)]
pub use frontmatter::FrontmatterError;
#[doc(inline)]
pub use frontmatter::Invocation;
#[doc(inline)]
pub use frontmatter::ModelTier;
#[doc(inline)]
pub use frontmatter::SkillMeta;
#[doc(inline)]
pub use hooks::Grants;
#[doc(inline)]
pub use hooks::Hook;
#[doc(inline)]
pub use hooks::HookAction;
#[doc(inline)]
pub use hooks::HookEvent;
#[doc(inline)]
pub use hooks::HookRun;
#[doc(inline)]
pub use hooks::InvalidHookEvent;
#[doc(inline)]
pub use hooks::InvalidMatchValue;
#[doc(inline)]
pub use hooks::InvalidPolicy;
#[doc(inline)]
pub use hooks::LuaHook;
#[doc(inline)]
pub use hooks::MatchValue;
#[doc(inline)]
pub use hooks::MessageHook;
#[doc(inline)]
pub use hooks::NativeHook;
#[doc(inline)]
pub use hooks::NativeHooks;
#[doc(inline)]
pub use hooks::Policy;
#[doc(inline)]
pub use lock::EntryKey;
#[doc(inline)]
pub use lock::FileEntry;
#[doc(inline)]
pub use lock::LockContents;
#[doc(inline)]
pub use lock::LockError;
#[doc(inline)]
pub use lock::LockedEntry;
#[doc(inline)]
pub use lock::LockedSource;
#[doc(inline)]
pub use lock::Lockfile;
#[doc(inline)]
pub use lock::Owner;
#[doc(inline)]
pub use lock::check_airsl_presence;
#[doc(inline)]
pub use lock::write_lock;
#[doc(inline)]
pub use manifest::AirslReq;
#[doc(inline)]
pub use manifest::ExcludeGlob;
#[doc(inline)]
pub use manifest::InvalidAirslReq;
#[doc(inline)]
pub use manifest::InvalidExcludeGlob;
#[doc(inline)]
pub use manifest::InvalidOptionType;
#[doc(inline)]
pub use manifest::InvalidPackPath;
#[doc(inline)]
pub use manifest::InvalidVersion;
#[doc(inline)]
pub use manifest::ManifestError;
#[doc(inline)]
pub use manifest::OptionDecl;
#[doc(inline)]
pub use manifest::OptionType;
#[doc(inline)]
pub use manifest::OptionValue;
#[doc(inline)]
pub use manifest::PackManifest;
#[doc(inline)]
pub use manifest::PackPath;
#[doc(inline)]
pub use manifest::PackVersion;
#[doc(inline)]
pub use manifest::Requires;
#[doc(inline)]
pub use name::BinaryName;
#[doc(inline)]
pub use name::EnvName;
#[doc(inline)]
pub use name::InvalidEnvName;
#[doc(inline)]
pub use name::InvalidName;
#[doc(inline)]
pub use name::InvalidOptionName;
#[doc(inline)]
pub use name::ItemName;
#[doc(inline)]
pub use name::OptionName;
#[doc(inline)]
pub use name::PackName;
#[doc(inline)]
pub use name::ToolName;
#[doc(inline)]
pub use pack::ItemKind;
#[doc(inline)]
pub use pack::NativeDir;
#[doc(inline)]
pub use pack::Pack;
#[doc(inline)]
pub use pack::PackAgent;
#[doc(inline)]
pub use pack::PackError;
#[doc(inline)]
pub use pack::PackSkill;
#[doc(inline)]
pub use project::CommitId;
#[doc(inline)]
pub use project::GitTag;
#[doc(inline)]
pub use project::GitUrl;
#[doc(inline)]
pub use project::InvalidCommitId;
#[doc(inline)]
pub use project::InvalidGitTag;
#[doc(inline)]
pub use project::InvalidGitUrl;
#[doc(inline)]
pub use project::InvalidLocalPath;
#[doc(inline)]
pub use project::InvalidSubPath;
#[doc(inline)]
pub use project::LocalPath;
#[doc(inline)]
pub use project::PackEntry;
#[doc(inline)]
pub use project::Pin;
#[doc(inline)]
pub use project::ProjectError;
#[doc(inline)]
pub use project::ProjectFile;
#[doc(inline)]
pub use project::Source;
#[doc(inline)]
pub use project::SourceError;
#[doc(inline)]
pub use project::SubPath;
#[doc(inline)]
pub use report::Diagnostic;
#[doc(inline)]
pub use report::Diagnostics;
#[doc(inline)]
pub use report::DoesNotExist;
#[doc(inline)]
pub use report::FieldError;
#[doc(inline)]
pub use report::FormatError;
#[doc(inline)]
pub use report::ListedTwice;
#[doc(inline)]
pub use report::Location;
#[doc(inline)]
pub use report::MustBeRelative;
#[doc(inline)]
pub use report::NotLuaFile;
#[doc(inline)]
pub use report::OutsideProject;
#[doc(inline)]
pub use report::ParserMessage;
#[doc(inline)]
pub use report::Quoted;
#[doc(inline)]
pub use report::TypeWord;
#[doc(inline)]
pub use report::UnsupportedApi;
#[doc(inline)]
pub use set::AirslRange;
#[doc(inline)]
pub use set::SetError;
#[doc(inline)]
pub use set::ValidSet;
#[doc(inline)]
pub use set::check as check_pack_set;
#[doc(inline)]
pub use shorthand::Argument;
#[doc(inline)]
pub use shorthand::Flag;
#[doc(inline)]
pub use shorthand::ShorthandError;
#[doc(inline)]
pub use shorthand::classify as classify_argument;
#[doc(inline)]
pub use tokens::ItemRef;
#[doc(inline)]
pub use tokens::LocatedToken;
#[doc(inline)]
pub use tokens::TextOrigin;
#[doc(inline)]
pub use tokens::Token;
#[doc(inline)]
pub use tokens::TokenError;
#[doc(inline)]
pub use tokens::TokenKind;
#[doc(inline)]
pub use tokens::TokenTarget;
#[doc(inline)]
pub use tokens::TokenValues;
#[doc(inline)]
pub use tokens::Var;
#[doc(inline)]
pub use tokens::render;
#[doc(inline)]
pub use tokens::scan;
#[doc(inline)]
pub use tree::ContentHash;
#[doc(inline)]
pub use tree::ContentHashError;
#[doc(inline)]
pub use tree::TreePath;
#[doc(inline)]
pub use tree::WalkError;
#[doc(inline)]
pub use tree::Walked;
#[doc(inline)]
pub use tree::tree_hash;
#[doc(inline)]
pub use tree::walk;
