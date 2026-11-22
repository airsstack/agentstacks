//! Loads the pack fixtures through the public API over the real disk.
//!
//! Each valid fixture shows one feature; each invalid fixture is valid except for the one refusal
//! its name says, and the test asserts that refusal's exact text.

use std::error::Error;
use std::path::PathBuf;

use agentstacks::fs::StdFileTree;
use agentstacks::types::Pack;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/packs")
        .join(name)
}

fn refusals(name: &str) -> Vec<String> {
    Pack::load(&StdFileTree::new(fixture(name)))
        .err()
        .unwrap_or_default()
        .into_sorted()
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn load_accepts_each_feature_fixture() -> TestResult {
    for name in [
        "valid-hooks",
        "valid-native-hooks",
        "valid-options",
        "valid-native-files",
        "valid-render-exclude",
        "valid-tokens",
        "valid-shared-name",
    ] {
        let loaded = Pack::load(&StdFileTree::new(fixture(name)));
        if let Err(found) = loaded {
            let shown: Vec<String> = found
                .into_sorted()
                .iter()
                .map(ToString::to_string)
                .collect();
            return Err(format!("{name}: {}", shown.join("; ")).into());
        }
    }
    Ok(())
}

#[test]
fn load_refuses_each_fixture() {
    // (fixture, the location and message the refusal reads, after the fixture directory)
    let cases = [
        ("no-pack-toml", ": not a pack (no pack.toml)"),
        (
            "reserved-commands",
            "/commands: reserved, not supported in api 0",
        ),
        ("reserved-rules", "/rules: reserved, not supported in api 0"),
        ("skill-missing-skill-md", "/skills/a: missing SKILL.md"),
        (
            "skills-stray-file",
            "/skills/README.md: only skill directories belong in skills/",
        ),
        (
            "skill-bad-dir-name",
            "/skills/My_Skill: \"My_Skill\" is not a valid name",
        ),
        (
            "agents-stray-file",
            "/agents/notes.txt: agents/ holds only <name>.md files",
        ),
        (
            "agents-subdirectory",
            "/agents/x: agents/ holds only <name>.md files",
        ),
        (
            "agent-bad-stem",
            "/agents/Coder.md: \"Coder\" is not a valid name",
        ),
        (
            "skill-name-mismatch",
            "/skills/a/SKILL.md: name \"b\" does not match \"a\"",
        ),
        (
            "agent-name-mismatch",
            "/agents/a.md: name \"b\" does not match \"a\"",
        ),
        (
            "native-stray-file",
            "/native/README.md: only tool directories belong in native/",
        ),
        (
            "native-bad-tool",
            "/native/Claude: \"Claude\" is not a valid name",
        ),
        ("nothing-to-install", ": pack declares nothing to install"),
        (
            "nothing-but-excluded-native-hooks",
            ": pack declares nothing to install",
        ),
        (
            "lua-without-airsl",
            "/pack.toml: requires.airsl: required (the pack contains .lua files)",
        ),
        (
            "render-missing",
            "/pack.toml: render[0]: \"enforcment.json\" does not exist",
        ),
        (
            "render-excluded",
            "/pack.toml: render[0]: \"x.json\" is also excluded",
        ),
        (
            "exclude-protected-pack-toml",
            "/pack.toml: exclude[0]: \"pack.toml\" excludes pack.toml",
        ),
        (
            "exclude-protected-skill-md",
            "/pack.toml: exclude[0]: \"**/*.md\" excludes skills/discuss/SKILL.md",
        ),
        (
            "exclude-protected-agent",
            "/pack.toml: exclude[0]: \"agents/*\" excludes agents/coder.md",
        ),
        (
            "exclude-protected-run",
            "/pack.toml: exclude[0]: \"hooks/**\" excludes hooks/enforce.lua",
        ),
        (
            "render-not-utf8",
            "/data.json: not valid UTF-8, cannot render",
        ),
        ("pack-toml-not-utf8", "/pack.toml: not valid UTF-8"),
        ("skill-md-not-utf8", "/skills/a/SKILL.md: not valid UTF-8"),
        ("agent-md-not-utf8", "/agents/b.md: not valid UTF-8"),
        (
            "native-hooks-not-utf8",
            "/native/claude/hooks.toml: not valid UTF-8",
        ),
        (
            "arguments-outside-skill",
            "/agents/b.md:6: var \"arguments\" is only valid in SKILL.md",
        ),
        (
            "neutral-references-native",
            "/skills/a/SKILL.md:6: neutral content must not reference native/",
        ),
        (
            "own-skill-dir-is-an-agent",
            "/skills/a/SKILL.md:6: pack \"core\" has no skill \"coder\"",
        ),
        (
            "own-agent-missing",
            "/skills/a/SKILL.md:6: pack \"core\" has no agent \"codr\"",
        ),
        (
            "hook-run-missing",
            "/pack.toml: hooks[1].run: \"hooks/missing.lua\" does not exist",
        ),
        (
            "native-hook-run-missing",
            "/native/claude/hooks.toml: hooks[0].run: \"hooks/missing.lua\" does not exist",
        ),
        (
            "hook-agents-unknown",
            "/pack.toml: hooks[3].agents[1]: pack \"core\" has no agent \"task-briefer\"",
        ),
    ];
    for (name, rest) in cases {
        let expected = format!("{}{rest}", fixture(name).display());
        assert_eq!(refusals(name), [expected], "fixture {name}");
    }
}

#[test]
fn load_reports_errors_from_every_file_in_one_run() {
    let root = fixture("many-errors").display().to_string();
    assert_eq!(
        refusals("many-errors"),
        [
            format!("{root}/agents/b.md:6: unknown token kind \"skil\""),
            format!("{root}/pack.toml: pack.description: empty"),
            format!("{root}/skills/a/SKILL.md: unknown key \"disable-model-invocation\""),
        ]
    );
}

/// (fixture, the location and message the refusal reads, after the fixture directory): one per
/// refusal row of the format spec that no other table covers.
const SPEC_ROW_CASES: &[(&str, &str)] = &[
    (
        "token-unknown-kind",
        "/skills/a/SKILL.md:6: unknown token kind \"skil\"",
    ),
    (
        "token-unterminated",
        "/skills/a/SKILL.md:6: unterminated token",
    ),
    ("token-empty", "/skills/a/SKILL.md:6: empty token"),
    (
        "token-needs-target",
        "/skills/a/SKILL.md:6: token \"skill\" needs a target",
    ),
    (
        "token-takes-one-target",
        "/skills/a/SKILL.md:6: token \"root\" takes one target",
    ),
    (
        "token-invalid-pack-name",
        "/skills/a/SKILL.md:6: \"Core\" is not a valid name",
    ),
    (
        "token-invalid-target",
        "/skills/a/SKILL.md:6: \"a/b/c\" is not a valid target (expected [pack/]name)",
    ),
    (
        "token-root-takes-pack-name",
        "/skills/a/SKILL.md:6: token \"root\" takes a pack name, not \"core/x\"",
    ),
    (
        "token-unknown-var",
        "/skills/a/SKILL.md:6: unknown var \"session\" (known: session_id, arguments, tmpdir, home, airsstack_home, project_dir)",
    ),
    (
        "token-option-bad-name",
        "/skills/a/SKILL.md:6: \"a-b\" is not a valid option name",
    ),
    (
        "token-option-other-pack",
        "/skills/a/SKILL.md:6: option tokens name this pack's options only",
    ),
    (
        "own-skill-missing",
        "/skills/a/SKILL.md:6: pack \"core\" has no skill \"status\"",
    ),
    (
        "unknown-key-in-hook",
        "/pack.toml: hooks[0].grant: unknown key",
    ),
    (
        "pack-name-invalid",
        "/pack.toml: pack.name: \"Core\" is not a valid name",
    ),
    ("pack-name-missing", "/pack.toml: pack.name: required"),
    (
        "pack-version-not-semver",
        "/pack.toml: pack.version: \"1.2\" is not semver",
    ),
    (
        "pack-api-unsupported",
        "/pack.toml: core: api 1 not supported (this agentstacks speaks 0)",
    ),
    (
        "pack-description-empty",
        "/pack.toml: pack.description: empty",
    ),
    (
        "requires-packs-twice",
        "/pack.toml: requires.packs[1]: \"sdlc\" listed twice",
    ),
    (
        "requires-packs-itself",
        "/pack.toml: requires.packs[0]: a pack cannot require itself",
    ),
    (
        "requires-airsl-not-a-range",
        "/pack.toml: requires.airsl: \"nope\" is not a version range",
    ),
    (
        "requires-binaries-twice",
        "/pack.toml: requires.binaries[1]: \"git\" listed twice",
    ),
    (
        "render-absolute",
        "/pack.toml: render[0]: \"/x\" must be relative",
    ),
    (
        "render-outside",
        "/pack.toml: render[0]: \"../x\" must stay inside the pack",
    ),
    (
        "render-pack-toml",
        "/pack.toml: render[0]: \"pack.toml\" is never installed",
    ),
    (
        "render-native-hooks",
        "/pack.toml: render[0]: \"native/claude/hooks.toml\" is never installed",
    ),
    (
        "exclude-invalid-glob",
        "/pack.toml: exclude[0]: \"[\" is not a valid glob",
    ),
    (
        "exclude-absolute",
        "/pack.toml: exclude[0]: \"/tmp/**\" must be relative",
    ),
    (
        "option-name-invalid",
        "/pack.toml: options.archive-keep: \"archive-keep\" is not a valid option name",
    ),
    (
        "option-type-invalid",
        "/pack.toml: options.keep.type: \"number\" (expected boolean, integer, string)",
    ),
    (
        "option-default-mismatch",
        "/pack.toml: options.archive_keep.default: expects integer, got string",
    ),
    (
        "option-description-empty",
        "/pack.toml: options.archive_keep.description: empty",
    ),
    (
        "frontmatter-missing",
        "/skills/a/SKILL.md: missing frontmatter",
    ),
    (
        "frontmatter-syntax-error",
        "/skills/a/SKILL.md:3: illegal placement of ':' indicator",
    ),
    (
        "frontmatter-duplicate-key",
        "/skills/a/SKILL.md:3: duplicate mapping key: name, set DuplicateKeyPolicy in Options if acceptable",
    ),
    (
        "frontmatter-not-a-mapping",
        "/skills/a/SKILL.md: frontmatter: expected a mapping, got null",
    ),
    ("skill-name-required", "/skills/a/SKILL.md: name: required"),
    (
        "skill-description-required",
        "/skills/a/SKILL.md: description: required",
    ),
    (
        "skill-description-empty",
        "/skills/a/SKILL.md: description: empty",
    ),
    (
        "skill-name-not-literal",
        "/skills/a/SKILL.md: name must be a literal",
    ),
    (
        "skill-unknown-key",
        "/skills/a/SKILL.md: unknown key \"disable-model-invocation\"",
    ),
    (
        "skill-invocation-invalid",
        "/skills/a/SKILL.md: invocation: \"manual\" (expected user, any)",
    ),
    (
        "skill-argument-hint-boolean",
        "/skills/a/SKILL.md: argument_hint: expected a string, got boolean",
    ),
    (
        "skill-native-bad-tool",
        "/skills/a/SKILL.md: native.Claude: \"Claude\" is not a valid tool name",
    ),
    (
        "agent-model-invalid",
        "/agents/b.md: model: \"sonnet\" (expected fast, balanced, deep)",
    ),
    (
        "agent-effort-invalid",
        "/agents/b.md: effort: \"max\" (expected low, medium, high)",
    ),
    (
        "agent-tools-unknown",
        "/agents/b.md: tools[0]: unknown \"bash\"",
    ),
    (
        "agent-tools-twice",
        "/agents/b.md: tools[1]: \"read\" listed twice",
    ),
    (
        "hook-kind-invalid",
        "/pack.toml: hooks[0].kind: \"sh\" (expected message, lua)",
    ),
    (
        "hook-event-unknown",
        "/pack.toml: hooks[0].event: \"tool.pre\" unknown",
    ),
    (
        "hook-match-not-allowed",
        "/pack.toml: hooks[0].match: not allowed on session.end",
    ),
    (
        "hook-match-invalid-value",
        "/pack.toml: hooks[0].match[0]: \"bash\" not valid for tool.before",
    ),
    (
        "hook-match-empty",
        "/pack.toml: hooks[0].match: empty list never fires",
    ),
    (
        "hook-match-twice",
        "/pack.toml: hooks[0].match[1]: \"read\" listed twice",
    ),
    (
        "hook-agents-not-agent-stop",
        "/pack.toml: hooks[0].agents: only allowed on agent.stop",
    ),
    (
        "hook-message-text-missing",
        "/pack.toml: hooks[0].text: required",
    ),
    (
        "hook-message-text-empty",
        "/pack.toml: hooks[0].text: empty",
    ),
    (
        "hook-message-run",
        "/pack.toml: hooks[0].run: not allowed on a message hook",
    ),
    (
        "hook-message-args",
        "/pack.toml: hooks[0].args: not allowed on a message hook",
    ),
    (
        "hook-message-grants",
        "/pack.toml: hooks[0].grants: not allowed on a message hook",
    ),
    (
        "hook-message-may-block",
        "/pack.toml: hooks[0].may_block: message hooks cannot block",
    ),
    (
        "hook-lua-text",
        "/pack.toml: hooks[0].text: not allowed on a lua hook",
    ),
    (
        "hook-lua-run-required",
        "/pack.toml: hooks[0].run: required",
    ),
    (
        "hook-lua-run-not-lua",
        "/pack.toml: hooks[0].run: \"x.sh\" is not a .lua file",
    ),
    (
        "hook-lua-run-form",
        "/pack.toml: hooks[0].run: must be pack-relative, {{root}}/… or {{root <pack>}}/…",
    ),
    (
        "hook-policy-trusted",
        "/pack.toml: hooks[0].grants.policy: \"trusted\" is not allowed for pack hooks",
    ),
    (
        "hook-grants-read-relative",
        "/pack.toml: hooks[0].grants.read[0]: \"data\" must be absolute or start with a token",
    ),
    (
        "hook-grants-env-invalid",
        "/pack.toml: hooks[0].grants.env[0]: \"home\" is not an environment variable name",
    ),
    (
        "hook-grants-exec-not-required",
        "/pack.toml: hooks[0].grants.exec[0]: \"git\" is not in requires.binaries",
    ),
    (
        "hook-may-block-not-bool",
        "/pack.toml: hooks[0].may_block: expected true or false",
    ),
    (
        "native-hooks-agents-key",
        "/native/claude/hooks.toml: hooks[0].agents: unknown key",
    ),
    (
        "render-path-empty",
        "/pack.toml: render[0]: \"\" must not be empty",
    ),
    (
        "render-path-dot",
        "/pack.toml: render[0]: \".\" must name a file inside the pack",
    ),
    (
        "render-path-backslash",
        "/pack.toml: render[0]: \"a\\b\" must use / as the separator",
    ),
    (
        "render-path-dot-segment",
        "/pack.toml: render[0]: \"./x\" must not have a . segment",
    ),
    (
        "render-path-trailing-slash",
        "/pack.toml: render[0]: \"dir/\" must not end with /",
    ),
    (
        "render-path-empty-segment",
        "/pack.toml: render[0]: \"a//b\" must not have an empty segment",
    ),
    (
        "render-is-code",
        "/pack.toml: render[0]: \"x.lua\" is code; code is installed verbatim",
    ),
    (
        "hook-grants-policy-unknown",
        "/pack.toml: hooks[0].grants.policy: \"foo\" (expected confined)",
    ),
    (
        "hook-grants-write-relative",
        "/pack.toml: hooks[0].grants.write[0]: \"data\" must be absolute or start with a token",
    ),
    (
        "hook-grants-env-twice",
        "/pack.toml: hooks[0].grants.env[1]: \"HOME\" listed twice",
    ),
    (
        "hook-grants-exec-twice",
        "/pack.toml: hooks[0].grants.exec[1]: \"airsl\" listed twice",
    ),
    (
        "hook-match-not-string",
        "/pack.toml: hooks[0].match[0]: expected string, got integer",
    ),
    (
        "hook-agents-empty",
        "/pack.toml: hooks[0].agents: empty list never fires",
    ),
    (
        "hook-agents-bad-name",
        "/pack.toml: hooks[0].agents[0]: \"Coder\" is not a valid name",
    ),
    (
        "agent-tools-not-array",
        "/agents/b.md: tools: expected array, got string",
    ),
    (
        "agent-tools-item-not-string",
        "/agents/b.md: tools[0]: expected string, got integer",
    ),
    (
        "skill-integer-beyond-i64",
        "/skills/a/SKILL.md:4: integer beyond i64",
    ),
];

#[test]
fn load_refuses_each_spec_row_fixture() {
    let mut mismatches = Vec::new();
    for (name, rest) in SPEC_ROW_CASES {
        let expected = format!("{}{rest}", fixture(name).display());
        let found = refusals(name);
        if found != [expected.clone()] {
            mismatches.push(format!("{name}\n   want {expected}\n   have {found:?}"));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
