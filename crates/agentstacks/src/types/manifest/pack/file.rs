//! A pack's `pack.toml`, parsed and checked.
//!
//! It is the one file that sees the whole `pack.toml`, so the checks that cross tables (a pack
//! cannot require itself, hooks may only exec the binaries `[requires]` lists) have one owner; each
//! table's own rules stay in the type that holds it.
//!
//! # Responsibilities
//!
//! - [`PackManifest::parse`] reads the whole file and returns the manifest, or every refusal of
//!   the file at once: a field's refusal never hides the others.
//! - A syntax error stops only this file and is its only diagnostic; empty text is an empty table,
//!   so each missing required field is refused by name.
//! - It reads `[pack]` (name, version, api, description), `[requires]`, `render`, `exclude`,
//!   `[options]` and `[[hooks]]`.
//!
//! # Non-responsibilities
//!
//! Rules that need the pack's files (a `render` file existing, an `exclude` protecting
//! `SKILL.md`, a `requires.airsl` being present when the pack holds Lua) belong to the pack
//! loader, and rules that need the whole set of packs belong to the set checks.

use std::path::Path;

use crate::types::api::API;
use crate::types::format::TomlFields;
use crate::types::format::parse_toml_document;
use crate::types::hooks::Hook;
use crate::types::manifest::ExcludeGlob;
use crate::types::manifest::OptionDecl;
use crate::types::manifest::PackPath;
use crate::types::manifest::PackVersion;
use crate::types::manifest::error::ManifestError;
use crate::types::manifest::requires::Requires;
use crate::types::name::BinaryName;
use crate::types::name::OptionName;
use crate::types::name::PackName;
use crate::types::report::Diagnostics;
use crate::types::tokens::LocatedToken;

/// A parsed `pack.toml`.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use agentstacks::types::PackManifest;
///
/// let text = r#"
/// [pack]
/// name = "core"
/// version = "1.0.0"
/// api = 0
/// description = "Shared skills"
/// "#;
/// let manifest = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
///     .map_err(|found| format!("{found:?}"))?;
/// assert_eq!(manifest.name().as_str(), "core");
/// assert_eq!(manifest.version().to_string(), "1.0.0");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct PackManifest {
    name: PackName,
    version: PackVersion,
    description: String,
    requires: Requires,
    render: Vec<PackPath>,
    exclude: Vec<ExcludeGlob>,
    options: Vec<(OptionName, OptionDecl)>,
    hooks: Vec<Hook>,
    tokens: Vec<LocatedToken>,
}

impl PackManifest {
    /// Parses `text`, the content of the `pack.toml` at `file`.
    ///
    /// # Errors
    ///
    /// Returns every refusal of the file when any field is refused; the [`Diagnostics`] hand them
    /// back sorted.
    ///
    /// # Examples
    ///
    /// A refusal is located at the key path of the entry it is about, here the first `render`
    /// entry:
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use agentstacks::types::PackManifest;
    ///
    /// let text = r#"
    /// render = ["/x"]
    ///
    /// [pack]
    /// name = "core"
    /// version = "1.0.0"
    /// api = 0
    /// description = "Shared skills"
    /// "#;
    /// let refused = PackManifest::parse(Path::new("packs/core/pack.toml"), text)
    ///     .err()
    ///     .ok_or("expected a refusal")?;
    /// let lines: Vec<String> = refused.into_sorted().iter().map(ToString::to_string).collect();
    /// assert_eq!(lines, ["packs/core/pack.toml: render[0]: \"/x\" must be relative"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn parse(file: &Path, text: &str) -> Result<Self, Diagnostics> {
        let table = match parse_toml_document(file, text) {
            Ok(table) => table,
            Err(syntax) => {
                let mut diagnostics = Diagnostics::new();
                diagnostics.push(syntax);
                return Err(diagnostics);
            }
        };
        let mut root = TomlFields::new(file, "", table);
        root.refuse_unknown_keys(&TOP_LEVEL_KEYS);
        let (own_name, identity) = root
            .table_or_empty("pack")
            .map_or((None, None), |mut pack| {
                let read = read_identity(&mut pack);
                root.absorb(pack);
                read
            });
        let requires = root.optional_table("requires").map(|mut table| {
            let requires = Requires::read(&mut table, own_name.as_ref());
            root.absorb(table);
            requires
        });
        let render = read_render(&mut root);
        let exclude = read_exclude(&mut root);
        let options = read_options(&mut root);
        let binaries = requires.as_ref().map_or(&[][..], Requires::binaries);
        let mut tokens = Vec::new();
        let hooks = read_hooks(&mut root, binaries, &mut tokens);
        let diagnostics = root.into_diagnostics();
        match identity {
            Some(identity) if diagnostics.is_empty() => Ok(Self {
                name: identity.name,
                version: identity.version,
                description: identity.description,
                requires: requires.unwrap_or_default(),
                render,
                exclude,
                options,
                hooks,
                tokens,
            }),
            _ => Err(diagnostics),
        }
    }

    /// The pack's name.
    #[must_use]
    pub const fn name(&self) -> &PackName {
        &self.name
    }

    /// The pack's version.
    #[must_use]
    pub const fn version(&self) -> &PackVersion {
        &self.version
    }

    /// The pack's description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// What the pack requires.
    #[must_use]
    pub const fn requires(&self) -> &Requires {
        &self.requires
    }

    /// The files the pack renders besides its Markdown.
    #[must_use]
    pub fn render(&self) -> &[PackPath] {
        &self.render
    }

    /// The globs the pack excludes from installation.
    #[must_use]
    pub fn exclude(&self) -> &[ExcludeGlob] {
        &self.exclude
    }

    /// The neutral hooks the pack declares, in file order.
    #[must_use]
    pub fn hooks(&self) -> &[Hook] {
        &self.hooks
    }

    /// Every token found in the hook fields (`text`, `args`, `run` and grant paths), located at
    /// its key path.
    #[must_use]
    pub fn tokens(&self) -> &[LocatedToken] {
        &self.tokens
    }

    /// The options the pack declares, in name order.
    #[must_use]
    pub fn options(&self) -> &[(OptionName, OptionDecl)] {
        &self.options
    }
}

/// The keys a `pack.toml` may hold at its top level.
const TOP_LEVEL_KEYS: [&str; 6] = ["pack", "requires", "render", "exclude", "options", "hooks"];

/// The required fields of `[pack]`, once every one of them is valid.
struct Identity {
    name: PackName,
    version: PackVersion,
    description: String,
}

/// Reads `[pack]`, refusing what is wrong.
///
/// Returns the pack's name when the name itself is valid, whatever the other fields are, and the
/// whole identity when every field is valid.
fn read_identity(pack: &mut TomlFields) -> (Option<PackName>, Option<Identity>) {
    let name = pack.required_string("name").and_then(|text| {
        PackName::new(&text)
            .map_err(|error| pack.refuse("name", ManifestError::from(error)))
            .ok()
    });
    let version = pack.required_string("version").and_then(|text| {
        PackVersion::new(&text)
            .map_err(|error| pack.refuse("version", ManifestError::from(error)))
            .ok()
    });
    let api = pack.required_integer("api");
    if let Some(api) = api.filter(|api| *api != API) {
        match &name {
            Some(pack_name) => pack.refuse_at_file(ManifestError::ApiUnsupported {
                pack: pack_name.clone(),
                api,
            }),
            None => pack.refuse("api", ManifestError::ApiUnsupportedUnnamed { api }),
        }
    }
    let description = pack.required_text("description");
    pack.refuse_unknown_keys(&["name", "version", "api", "description"]);
    let identity = match (name.clone(), version, api, description) {
        (Some(name), Some(version), Some(API), Some(description)) => Some(Identity {
            name,
            version,
            description,
        }),
        _ => None,
    };
    (name, identity)
}

/// Reads `render`: relative paths that stay inside the pack and are not code. A refused entry is
/// located at its own index, `render[<index>]`.
fn read_render(root: &mut TomlFields) -> Vec<PackPath> {
    let mut paths = Vec::new();
    for (index, text) in root.optional_string_list("render").unwrap_or_default() {
        let item_path = root.item_path("render", index);
        match PackPath::new(&text) {
            Err(error) => root.refuse_path(&item_path, ManifestError::from(error)),
            Ok(path) if path.has_extension("lua") || path.has_extension("sh") => {
                root.refuse_path(&item_path, ManifestError::RenderIsCode { path: text });
            }
            Ok(path) => paths.push(path),
        }
    }
    paths
}

/// Reads `[options]`: one declaration per option, keyed by a valid option name.
///
/// A bad name is refused but its declaration is still read, so every error of the entry shows.
fn read_options(root: &mut TomlFields) -> Vec<(OptionName, OptionDecl)> {
    let mut declared = Vec::new();
    let Some(mut options) = root.optional_table("options") else {
        return declared;
    };
    for (key, _) in options.entries() {
        let name = OptionName::new(&key)
            .map_err(|error| options.refuse(&key, ManifestError::from(error)))
            .ok();
        let decl = options.optional_table(&key).and_then(|mut entry| {
            let decl = OptionDecl::read(&mut entry);
            options.absorb(entry);
            decl
        });
        if let (Some(name), Some(decl)) = (name, decl) {
            declared.push((name, decl));
        }
    }
    root.absorb(options);
    declared
}

/// Reads the `[[hooks]]` items; every token found is appended to `tokens`.
fn read_hooks(
    root: &mut TomlFields,
    binaries: &[BinaryName],
    tokens: &mut Vec<LocatedToken>,
) -> Vec<Hook> {
    let mut hooks = Vec::new();
    for mut item in root.table_array("hooks") {
        let hook = Hook::read(&mut item, binaries, tokens);
        root.absorb(item);
        hooks.extend(hook);
    }
    hooks
}

/// Reads `exclude`: relative globs that compile. A refused entry is located at its own index,
/// `exclude[<index>]`.
fn read_exclude(root: &mut TomlFields) -> Vec<ExcludeGlob> {
    let mut globs = Vec::new();
    for (index, text) in root.optional_string_list("exclude").unwrap_or_default() {
        match ExcludeGlob::new(&text) {
            Ok(glob) => globs.push(glob),
            Err(error) => {
                root.refuse_path(
                    &root.item_path("exclude", index),
                    ManifestError::from(error),
                );
            }
        }
    }
    globs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OptionType;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const FILE: &str = "packs/core/pack.toml";

    const VALID: &str =
        "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 0\ndescription = \"d\"\n";

    /// The metadata part of a typical manifest. `render` and `exclude` come first because a key
    /// written after a `[table]` header belongs to that table.
    const METADATA_EXAMPLE: &str = r#"
render  = ["enforcement.json"]
exclude = ["**/*_test.lua"]

[pack]
name        = "sdlc"
version     = "0.4.2"
api         = 0
description = "a chain of steps"

[requires]
packs    = ["core"]
airsl    = ">=0.1.4"
binaries = ["git"]
"#;

    fn refusals(file: &str, text: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match PackManifest::parse(Path::new(file), text) {
            Ok(_) => Err(format!("expected refusals for {text:?}").into()),
            Err(diagnostics) => Ok(diagnostics
                .into_sorted()
                .iter()
                .map(ToString::to_string)
                .collect()),
        }
    }

    fn with_valid(extra: &str) -> String {
        format!("{extra}{VALID}")
    }

    #[test]
    fn pack_manifest_parses_the_metadata_example() -> TestResult {
        let manifest = PackManifest::parse(Path::new("packs/sdlc/pack.toml"), METADATA_EXAMPLE)
            .map_err(|d| {
                format!(
                    "{:?}",
                    d.into_sorted()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                )
            })?;
        assert_eq!(manifest.name().as_str(), "sdlc");
        assert_eq!(manifest.version().to_string(), "0.4.2");
        assert_eq!(manifest.description(), "a chain of steps");
        assert_eq!(manifest.requires().packs(), [PackName::new("core")?]);
        assert_eq!(
            manifest.requires().airsl().map(ToString::to_string),
            Some(">=0.1.4".to_owned())
        );
        assert_eq!(manifest.requires().binaries().len(), 1);
        assert_eq!(manifest.render(), [PackPath::new("enforcement.json")?]);
        assert_eq!(
            manifest
                .exclude()
                .iter()
                .map(ExcludeGlob::pattern)
                .collect::<Vec<_>>(),
            ["**/*_test.lua"]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_each_pack_field() -> TestResult {
        let cases = [
            (
                FILE,
                "[pack]\nname = \"Core\"\nversion = \"1.0.0\"\napi = 0\ndescription = \"d\"\n",
                "packs/core/pack.toml: pack.name: \"Core\" is not a valid name",
            ),
            (
                FILE,
                "[pack]\nname = \"core\"\nversion = \"1.2\"\napi = 0\ndescription = \"d\"\n",
                "packs/core/pack.toml: pack.version: \"1.2\" is not semver",
            ),
            (
                "packs/sdlc/pack.toml",
                "[pack]\nname = \"sdlc\"\nversion = \"1.0.0\"\napi = 1\ndescription = \"d\"\n",
                "packs/sdlc/pack.toml: sdlc: api 1 not supported (this agentstacks speaks 0)",
            ),
            (
                FILE,
                "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 0\ndescription = \"\"\n",
                "packs/core/pack.toml: pack.description: empty",
            ),
            (
                FILE,
                "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 0\ndescription = \"  \"\n",
                "packs/core/pack.toml: pack.description: empty",
            ),
            (
                FILE,
                "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 1979-05-27\ndescription = \"d\"\n",
                "packs/core/pack.toml: pack.api: expected integer, got datetime",
            ),
            (
                FILE,
                "[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 0.5\ndescription = \"d\"\n",
                "packs/core/pack.toml: pack.api: expected integer, got float",
            ),
        ];
        for (file, text, message) in cases {
            assert_eq!(refusals(file, text)?, [message], "{text}");
        }
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_api_without_a_valid_name() -> TestResult {
        let file = "packs/sdlc/pack.toml";
        let api = "packs/sdlc/pack.toml: pack.api: 1 not supported (this agentstacks speaks 0)";
        let cases = [
            (
                "[pack]\nversion = \"1.0.0\"\napi = 1\ndescription = \"d\"\n",
                "packs/sdlc/pack.toml: pack.name: required",
            ),
            (
                "[pack]\nname = \"Core\"\nversion = \"1.0.0\"\napi = 1\ndescription = \"d\"\n",
                "packs/sdlc/pack.toml: pack.name: \"Core\" is not a valid name",
            ),
            (
                "[pack]\nname = 1\nversion = \"1.0.0\"\napi = 1\ndescription = \"d\"\n",
                "packs/sdlc/pack.toml: pack.name: expected string, got integer",
            ),
        ];
        for (text, name_message) in cases {
            assert_eq!(refusals(file, text)?, [name_message, api], "{text}");
        }
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_an_empty_file_field_by_field() -> TestResult {
        assert_eq!(
            refusals(FILE, "")?,
            [
                "packs/core/pack.toml: pack.name: required",
                "packs/core/pack.toml: pack.version: required",
                "packs/core/pack.toml: pack.api: required",
                "packs/core/pack.toml: pack.description: required",
            ]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_render_entries() -> TestResult {
        let text = with_valid("render = [\"/x\", \"../x\", \"x.lua\", \"y.sh\"]\n");
        assert_eq!(
            refusals(FILE, &text)?,
            [
                "packs/core/pack.toml: render[0]: \"/x\" must be relative",
                "packs/core/pack.toml: render[1]: \"../x\" must stay inside the pack",
                "packs/core/pack.toml: render[2]: \"x.lua\" is code; code is installed verbatim",
                "packs/core/pack.toml: render[3]: \"y.sh\" is code; code is installed verbatim",
            ]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_exclude_entries() -> TestResult {
        let text = with_valid("exclude = [\"[\", \"/tmp/**\"]\n");
        assert_eq!(
            refusals(FILE, &text)?,
            [
                "packs/core/pack.toml: exclude[0]: \"[\" is not a valid glob",
                "packs/core/pack.toml: exclude[1]: \"/tmp/**\" must be relative",
            ]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_unknown_keys_at_each_level() -> TestResult {
        let text = "foo = 1\n[pack]\nname = \"core\"\nversion = \"1.0.0\"\napi = 0\ndescription = \"d\"\nfoo = 1\n";
        assert_eq!(
            refusals(FILE, text)?,
            [
                "packs/core/pack.toml: foo: unknown key",
                "packs/core/pack.toml: pack.foo: unknown key",
            ]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_reports_all_errors_at_once() -> TestResult {
        let text = "render = [\"/x\"]\n[pack]\nname = \"Core\"\nversion = \"1.2\"\napi = 0\ndescription = \"d\"\n";
        assert_eq!(
            refusals(FILE, text)?,
            [
                "packs/core/pack.toml: pack.name: \"Core\" is not a valid name",
                "packs/core/pack.toml: pack.version: \"1.2\" is not semver",
                "packs/core/pack.toml: render[0]: \"/x\" must be relative",
            ]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_requiring_itself_beside_another_bad_field() -> TestResult {
        let text = "[pack]\nname = \"sdlc\"\nversion = \"1.2\"\napi = 0\ndescription = \"d\"\n[requires]\npacks = [\"sdlc\"]\n";
        assert_eq!(
            refusals("packs/sdlc/pack.toml", text)?,
            [
                "packs/sdlc/pack.toml: pack.version: \"1.2\" is not semver",
                "packs/sdlc/pack.toml: requires.packs[0]: a pack cannot require itself",
            ]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_stops_at_a_syntax_error() -> TestResult {
        assert_eq!(
            refusals(FILE, "[pack]\nname = \"Core\"\na = [1,")?,
            ["packs/core/pack.toml:3: unclosed array, expected `]`"]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_reads_typical_options() -> TestResult {
        let text = format!(
            "{METADATA_EXAMPLE}\n[options.archive_keep]\ntype        = \"integer\"\ndefault     = 20\ndescription = \"Discussions kept per project besides the current one\"\n"
        );
        let manifest =
            PackManifest::parse(Path::new("packs/sdlc/pack.toml"), &text).map_err(|d| {
                format!(
                    "{:?}",
                    d.into_sorted()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                )
            })?;
        let names: Vec<&str> = manifest.options().iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["archive_keep"]);
        assert_eq!(
            manifest.options().first().map(|(_, d)| d.option_type()),
            Some(OptionType::Integer)
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_bad_option_names_and_shapes() -> TestResult {
        let cases = [
            (
                "[options.archive-keep]\ntype = \"integer\"\ndescription = \"d\"\n",
                "packs/core/pack.toml: options.archive-keep: \"archive-keep\" is not a valid option name",
            ),
            (
                "[options.x]\ndescription = \"d\"\n",
                "packs/core/pack.toml: options.x.type: required",
            ),
            (
                "options = 1\n",
                "packs/core/pack.toml: options: expected table, got integer",
            ),
            (
                "[options]\nkeep = 1\n",
                "packs/core/pack.toml: options.keep: expected table, got integer",
            ),
        ];
        for (extra, message) in cases {
            let text = if extra.starts_with("options") {
                with_valid(extra)
            } else {
                format!("{VALID}{extra}")
            };
            assert_eq!(refusals(FILE, &text)?, [message], "{extra}");
        }
        Ok(())
    }

    const TYPICAL_HOOKS: &str = r#"
[[hooks]]
kind  = "message"
event = "session.end"
text  = "Reminder: run {{skill snapshot-save}} to flush durable session learnings."

[[hooks]]
kind      = "lua"
event     = "tool.before"
match     = ["read", "edit", "write"]
run       = "{{root core}}/hooks/enforce.lua"
args      = ["--manifest", "{{root}}/enforcement.json"]
may_block = false
[hooks.grants]
policy = "confined"
read   = ["/"]
write  = ["{{var tmpdir}}"]
env    = ["HOME", "TMPDIR"]
exec   = []
"#;

    #[test]
    fn pack_manifest_parses_typical_hooks() -> TestResult {
        let text = format!("{METADATA_EXAMPLE}{TYPICAL_HOOKS}");
        let manifest =
            PackManifest::parse(Path::new("packs/sdlc/pack.toml"), &text).map_err(|d| {
                format!(
                    "{:?}",
                    d.into_sorted()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                )
            })?;
        assert_eq!(manifest.hooks().len(), 2);
        let located: Vec<String> = manifest
            .tokens()
            .iter()
            .map(|t| format!("{} {}", t.token(), t.location()))
            .collect();
        assert_eq!(
            located,
            [
                "{{skill snapshot-save}} packs/sdlc/pack.toml: hooks[0].text",
                "{{root core}} packs/sdlc/pack.toml: hooks[1].run",
                "{{root}} packs/sdlc/pack.toml: hooks[1].args[1]",
                "{{var tmpdir}} packs/sdlc/pack.toml: hooks[1].grants.write[0]",
            ]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_an_unknown_hook_key() -> TestResult {
        let hook = "[[hooks]]\nkind = \"message\"\nevent = \"session.end\"\ntext = \"x\"\n";
        let third = "[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"hooks/x.lua\"\ngrant = {}\n";
        let text = format!("{VALID}{hook}{hook}{third}");
        assert_eq!(
            refusals(FILE, &text)?,
            ["packs/core/pack.toml: hooks[2].grant: unknown key"]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_checks_exec_against_its_own_binaries() -> TestResult {
        let hook = "[[hooks]]\nkind = \"lua\"\nevent = \"session.end\"\nrun = \"hooks/x.lua\"\n[hooks.grants]\nexec = [\"git\"]\n";
        let declared = format!("{VALID}[requires]\nbinaries = [\"git\"]\n{hook}");
        let undeclared = format!("{VALID}{hook}");
        assert!(PackManifest::parse(Path::new(FILE), &declared).is_ok());
        assert_eq!(
            refusals(FILE, &undeclared)?,
            ["packs/core/pack.toml: hooks[0].grants.exec[0]: \"git\" is not in requires.binaries"]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_a_non_array_hooks_key() -> TestResult {
        assert_eq!(
            refusals(FILE, &with_valid("hooks = 1\n"))?,
            ["packs/core/pack.toml: hooks: expected array, got integer"]
        );
        Ok(())
    }

    #[test]
    fn pack_manifest_refuses_a_non_table_pack_key() -> TestResult {
        assert_eq!(
            refusals(FILE, "pack = 1")?,
            ["packs/core/pack.toml: pack: expected table, got integer"]
        );
        Ok(())
    }
}
