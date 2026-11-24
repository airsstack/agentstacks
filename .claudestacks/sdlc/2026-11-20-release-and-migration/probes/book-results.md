# Book probes (`scripts/book-probe.sh`, `book-probe2.sh`, `book-probe3.sh`, `book-probe6.sh`, 2026-11-24)

Worktree HEAD `e4e1acd`, mdBook 0.4.52, mdbook-mermaid 0.16.2, airsl 0.1.5. Scratch paths shown as `<scratch>`.

## book-probe.sh

```text
mdbook v0.4.52
mdbook-mermaid 0.16.2
== a. proposed book
$ the five pack READMEs links after the edit
packs/core/README.md:27:](https://github.com/juliusbrussee/caveman)
packs/core/README.md:33:](https://github.com/airsstack/agentstacks/blob/main/LICENSE)
packs/guideline-python/README.md:20:](https://github.com/airsstack/agentstacks/blob/main/packs/guideline-python/skills/guidelines/SKILL.md)
packs/guideline-python/README.md:21:](https://github.com/airsstack/agentstacks/tree/main/packs/guideline-python/skills/guidelines/references/)
packs/guideline-python/README.md:25:](https://github.com/airsstack/agentstacks/blob/main/LICENSE)
packs/guideline-rust/README.md:19:](https://github.com/airsstack/agentstacks/blob/main/packs/guideline-rust/skills/guidelines/SKILL.md)
packs/guideline-rust/README.md:20:](https://github.com/airsstack/agentstacks/tree/main/packs/guideline-rust/skills/guidelines/references/)
packs/guideline-rust/README.md:24:](https://github.com/airsstack/agentstacks/blob/main/LICENSE)
packs/journal/README.md:25:](https://github.com/airsstack/agentstacks/blob/main/packs/journal/references/note-schema.md)
packs/journal/README.md:26:](https://github.com/airsstack/agentstacks/tree/main/packs/journal/skills/)
packs/sdlc/README.md:6:](https://claude.com/blog/the-ai-native-sdlc-playbook)
packs/sdlc/README.md:22:](https://github.com/airsstack/agentstacks/blob/main/packs/sdlc/references/artifact-chain.md)
packs/sdlc/README.md:23:](https://github.com/airsstack/agentstacks/blob/main/packs/sdlc/references/templates.md)
packs/sdlc/README.md:28:](https://claude.com/blog/the-ai-native-sdlc-playbook)
packs/sdlc/README.md:30:](https://github.com/obra/superpowers)
packs/sdlc/README.md:34:](https://github.com/airsstack/agentstacks/blob/main/LICENSE)
$ cat book/book.toml
[book]
title = "The agentstacks Book"
authors = ["airsstack"]
language = "en"
src = "src"

# A chapter named in SUMMARY.md with no file behind it fails the build
# (`Chapter file not found`) instead of being created empty.
[build]
create-missing = false

# Turns ```mermaid blocks into diagrams. mermaid.min.js and mermaid-init.js
# beside this file are written by `mdbook-mermaid install book`.
[preprocessor.mermaid]
command = "mdbook-mermaid"

[output.html]
# Pages serves this repository as a project site under /agentstacks/, so the 404
# page needs the prefix to load its own stylesheets.
site-url = "/agentstacks/"
git-repository-url = "https://github.com/airsstack/agentstacks"
edit-url-template = "https://github.com/airsstack/agentstacks/edit/main/book/{path}"
additional-js = ["mermaid.min.js", "mermaid-init.js"]
$ cat docmap.sed
s#](cli\.md#](ch04-00-cli.md#g
s#](plan-and-apply\.md#](ch05-00-plan-and-apply.md#g
s#](authoring\.md#](ch06-00-authoring.md#g
s#](extensions\.md#](ch06-01-extensions.md#g
s#](hooks\.md#](ch06-02-hooks.md#g
s#](architecture\.md#](ch07-00-architecture.md#g
s/(#missing-values)/(#values)/
1s/^# CLI$/# The Command Line/
1s/^# Plan, apply, doctor and update$/# Plan, Apply and the Lock/
1s/^# Build an agentstacks tool extension$/# Writing a Tool Extension/
1s/^# Writing a tool extension$/# The Extension API/
1s/^# Architecture$/# Under the Hood/
$ cat cvmap.sed
s#ch05-00-claudevs-getting-started.md#@@ch08-00-claudevs-getting-started.md#g
s#ch05-01-installation.md#@@ch08-01-installation.md#g
s#ch05-02-first-plugin.md#@@ch08-02-first-plugin.md#g
s#ch05-03-first-case.md#@@ch08-03-first-case.md#g
s#ch05-04-running-the-gate.md#@@ch08-04-running-the-gate.md#g
s#ch06-00-writing-cases.md#@@ch09-00-writing-cases.md#g
s#ch06-01-hook-decisions.md#@@ch09-01-hook-decisions.md#g
s#ch06-02-scripts-and-flows.md#@@ch09-02-scripts-and-flows.md#g
s#ch06-03-cases-in-lua.md#@@ch09-03-cases-in-lua.md#g
s#ch06-04-native-suites.md#@@ch09-04-native-suites.md#g
s#ch06-05-selecting-and-skips.md#@@ch09-05-selecting-and-skips.md#g
s#ch07-00-check-gate.md#@@ch10-00-check-gate.md#g
s#ch07-01-stages.md#@@ch10-01-stages.md#g
s#ch07-02-wiring.md#@@ch10-02-wiring.md#g
s#ch07-03-ci.md#@@ch10-03-ci.md#g
s#ch08-00-embedding.md#@@ch11-00-embedding.md#g
s#ch08-01-first-suite.md#@@ch11-01-first-suite.md#g
s#ch08-02-recipes.md#@@ch11-02-recipes.md#g
s#ch09-00-claudevs-under-the-hood.md#@@ch12-00-claudevs-under-the-hood.md#g
s#ch09-01-design.md#@@ch12-01-design.md#g
s#ch09-02-engine.md#@@ch12-02-engine.md#g
s#https://airsstack.github.io/claudestacks/install.sh#https://airsstack.github.io/agentstacks/claudevs.sh#g
s#https://github.com/airsstack/claudestacks/#https://github.com/airsstack/agentstacks/#g
s#^claudevs 0\.1\.1$#claudevs 0.1.2#
s#@@ch#ch#g
$ cat book/src/SUMMARY.md
# Summary

[Introduction](introduction.md)

# agentstacks

- [Getting Started](ch01-00-getting-started.md)
  - [Installation](ch01-01-installation.md)
  - [A First Project](ch01-02-first-project.md)
- [The Packs](ch02-00-packs.md)
  - [core](ch02-01-core.md)
  - [sdlc](ch02-02-sdlc.md)
  - [journal](ch02-03-journal.md)
  - [guideline-rust](ch02-04-guideline-rust.md)
  - [guideline-python](ch02-05-guideline-python.md)
- [Moving from the claudestacks Plugins](ch03-00-migration.md)
- [The Command Line](ch04-00-cli.md)
- [Plan, Apply and the Lock](ch05-00-plan-and-apply.md)
- [Writing a Tool Extension](ch06-00-authoring.md)
  - [The Extension API](ch06-01-extensions.md)
  - [Hooks](ch06-02-hooks.md)
- [Under the Hood](ch07-00-architecture.md)

# claudevs

- [Getting Started](ch08-00-claudevs-getting-started.md)
  - [Installation](ch08-01-installation.md)
  - [A First Plugin](ch08-02-first-plugin.md)
  - [A First Case](ch08-03-first-case.md)
  - [Running the Gate](ch08-04-running-the-gate.md)
- [Writing Cases](ch09-00-writing-cases.md)
  - [Hook Decisions](ch09-01-hook-decisions.md)
  - [Scripts and Flows](ch09-02-scripts-and-flows.md)
  - [Cases in Lua](ch09-03-cases-in-lua.md)
  - [Native Suites](ch09-04-native-suites.md)
  - [Selecting Cases and Reading Skips](ch09-05-selecting-and-skips.md)
- [The check Gate](ch10-00-check-gate.md)
  - [Stages](ch10-01-stages.md)
  - [Wiring](ch10-02-wiring.md)
  - [Running in CI](ch10-03-ci.md)
- [Embedding](ch11-00-embedding.md)
  - [Your First Suite](ch11-01-first-suite.md)
  - [Recipes](ch11-02-recipes.md)
- [Under the Hood](ch12-00-claudevs-under-the-hood.md)
  - [Why It Works This Way](ch12-01-design.md)
  - [How the Engine Fits Together](ch12-02-engine.md)

# Appendix

- [Appendix](appendix-00.md)
  - [A. claudevs CLI Reference](appendix-01-claudevs-cli.md)
$ ls book/src | wc -l
      41
$ mdbook build book
exit=0
0
relative hrefs checked=843 broken=0
$ the moved pages titles
# The Command Line
# Plan, Apply and the Lock
# Writing a Tool Extension
# The Extension API
# Hooks
# Under the Hood
$ grep -c class="mermaid" over chapters (total)
       5
$ relative links left in included pack chapters
== b. missing chapter
2026-11-24 13:18:22 [ERROR] (mdbook::utils): Error: Chapter file not found, gone.md
2026-11-24 13:18:22 [ERROR] (mdbook::utils): 	Caused By: No such file or directory (os error 2)
exit=101
== c. leftovers in moved claudevs chapters
$ grep -c -i claudestacks (moved claudevs chapters, total)
0
$ every rewritten agentstacks example URL exists in this tree
ok crates/claudevs/examples/README.md
ok crates/claudevs/examples
ok crates/claudevs/examples/01_hook_decision
ok crates/claudevs/examples/02_hook_decision_broken
ok crates/claudevs/examples/03_session_context
ok crates/claudevs/examples/04_script_and_flow
ok crates/claudevs/examples/05_lua_cases
ok crates/claudevs/examples/06_native_suite
ok crates/claudevs/examples/07_wiring_broken
ok crates/claudevs/examples/08_installed_broken
ok crates/claudevs/examples/09_doctor_gaps
$ anchor #without-a-claude-binary: heading in crates/claudevs/examples/README.md
29:## Without a `claude` binary
$ the version line in the moved installation chapter
41:claudevs 0.1.2
== d. A First Project (official names, packs-v0.1.0 on a bare copy of HEAD)
$ airsl --version
airsl 0.1.5
$ agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust
exit=1
-- first 3 lines
agentstacks plan

  create   .claude/agents/core-coder.md                                                   core
-- last 4 lines

76 to write, 0 to delete, 0 unchanged, 1 skipped
stdin is not a terminal; pass --yes to apply
agentstacks.toml written; nothing applied (run apply)
$ agentstacks apply --yes
exit=0
-- first 3 lines
agentstacks plan

  create   .claude/agents/core-coder.md                                                   core
-- last 4 lines
  airsl    0.1.5 (new pin)

76 to write, 0 to delete, 0 unchanged, 1 skipped
applied: 76 written, 0 deleted
$ agentstacks plan
exit=0
-- first 3 lines
agentstacks plan

  skip     core/hook/3: claude shows no hook output to the model at session end
-- last 4 lines

  airsl    0.1.5 (pinned)

0 to write, 0 to delete, 76 unchanged, 1 skipped
$ agentstacks doctor
exit=0
-- first 3 lines
healthy
-- last 4 lines
healthy
$ agentstacks update
exit=0
-- first 3 lines
  note     agentstacks.toml: packs.core: no newer tag than packs-v0.1.0
  note     agentstacks.toml: packs.guideline-rust: no newer tag than packs-v0.1.0
  note     agentstacks.toml: packs.sdlc: no newer tag than packs-v0.1.0
-- last 4 lines
  note     agentstacks.toml: packs.guideline-rust: no newer tag than packs-v0.1.0
  note     agentstacks.toml: packs.sdlc: no newer tag than packs-v0.1.0
  note     agentstacks.toml: tools.claude: no newer tag than packs-v0.1.0
nothing to update
$ cat agentstacks.toml
api = 0

[tools.claude]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "extensions/claude"

[packs.core]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "packs/core"

[packs.guideline-rust]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "packs/guideline-rust"

[packs.sdlc]
git = "https://github.com/airsstack/agentstacks"
tag = "packs-v0.1.0"
subdir = "packs/sdlc"
$ ls -a
.
..
.claude
.git
agentstacks.lock
agentstacks.toml
out.txt
$ agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust --yes   # a fresh project, one step
exit=0

76 to write, 0 to delete, 0 unchanged, 1 skipped
applied: 76 written, 0 deleted
== e. what reads docs/
$ grep -rn "docs/" Makefile.toml .github deny.toml Cargo.toml crates/*/Cargo.toml
crates/claudevs/Cargo.toml:13:exclude    = ["docs/"]
(exit 0)
$ control: the same grep finds packs/ in Makefile.toml
12
$ mentions of docs paths, book, raw URLs inside docs/*.md
docs/authoring.md:7:> https://raw.githubusercontent.com/airsstack/agentstacks/main/docs/authoring.md
docs/authoring.md:18:| [extensions.md](https://raw.githubusercontent.com/airsstack/agentstacks/main/docs/extensions.md) | the `api = 0` Lua surface: manifest, events, operations, coverage, validation, every refusal |
docs/authoring.md:19:| [hooks.md](https://raw.githubusercontent.com/airsstack/agentstacks/main/docs/hooks.md) | the hook command line, the shared runtime, the neutral event and result, the translator contract |
docs/authoring.md:20:| [adapter.lua](https://raw.githubusercontent.com/airsstack/agentstacks/main/extensions/claude/adapter.lua) | the official Claude Code extension, a complete worked example |
docs/authoring.md:21:| [demo pack](https://raw.githubusercontent.com/airsstack/agentstacks/main/crates/agentstacks-cli/tests/fixtures/project/packs/demo/pack.toml) | a small pack (one skill, one agent, one lua hook, one message hook) to test with |
docs/README.md:3:Working documentation for contributors. It lives here until the mdBook under `book/` exists, and
== f. pack text gates over the new packs/CHANGELOG.md text
gate1 grep exit=1
gate2 grep exit=1
```

## book-probe2.sh

```text
$ mdbook build book   # a.md includes ../../packs/gone/README.md, which does not exist
exit=0
2026-11-24 13:18:27 [INFO] (mdbook::book): Book building has started
2026-11-24 13:18:27 [ERROR] (mdbook::preprocess::links): Error updating "{{#include ../../packs/gone/README.md}}", Could not read file for link {{#include ../../packs/gone/README.md}} (<scratch>/incprobe/book/src/../../packs/gone/README.md)
2026-11-24 13:18:27 [WARN] (mdbook::preprocess::links): Caused By: No such file or directory (os error 2)
2026-11-24 13:18:27 [INFO] (mdbook::book): Running the html backend
$ grep -c "{{#include" book/book/a.html
1
$ grep -n -e "\*" workflow-syntax-filter.md | head (saved GitHub doc)
5:* `*`: Matches zero or more characters, but does not match the `/` character. For example, `Octo*` matches `Octocat`.
```

## book-probe3.sh

```text
$ cat buildstep.sh
mdbook build book > "$RUNNER_TEMP/mdbook.log" 2>&1 || { cat "$RUNNER_TEMP/mdbook.log"; exit 1; }
cat "$RUNNER_TEMP/mdbook.log"
if grep -q '\[ERROR\]' "$RUNNER_TEMP/mdbook.log"; then
  echo "mdbook build book logged an error; see the [ERROR] lines above" >&2
  exit 1
fi
== clean
exit=0
status=0
== include
2026-11-24 13:18:27 [ERROR] (mdbook::preprocess::links): Error updating "{{#include ../../packs/gone/README.md}}", Could not read file for link {{#include ../../packs/gone/README.md}} (<scratch>/step-include/book/src/../../packs/gone/README.md)
2026-11-24 13:18:27 [WARN] (mdbook::preprocess::links): Caused By: No such file or directory (os error 2)
mdbook build book logged an error; see the [ERROR] lines above
exit=1
status=1
== chapter
2026-11-24 13:18:27 [ERROR] (mdbook::utils): Error: Chapter file not found, b.md
2026-11-24 13:18:27 [ERROR] (mdbook::utils): 	Caused By: No such file or directory (os error 2)
exit=1
status=1
```

## book-probe6.sh

```text
== a. unedited packs/guideline-rust/README.md included
exit=0
0
$ README links (source) and the hrefs they became
](skills/guidelines/SKILL.md)
](skills/guidelines/references/)
](../../LICENSE)
href="skills/guidelines/SKILL.html"
href="skills/guidelines/references/"
href="../../LICENSE"
== b. a broken chapter link and a bad anchor
exit=0
ERROR/WARN lines: 0
href="nope.html"
href="gr.html#no-such-anchor"
== c. repo-wide docs/ paths, outside .claudestacks/
$ git grep -n "docs/" -- ":!.claudestacks" | grep -v -e "https\?://" (outside URLs dropped)
CLAUDE.md:18:docs/                     contributor docs; docs/architecture.md is the layering contract
CLAUDE.md:73:**[`docs/architecture.md`](docs/architecture.md) is the contract for `crates/agentstacks`. Read it
CLAUDE.md:95:[`docs/authoring.md`](docs/authoring.md) step by step. It is written for an agent to follow,
CLAUDE.md:96:links the API contract (`docs/extensions.md`, `docs/hooks.md`) and uses `extensions/claude/` as
CLAUDE.md:103:crates/agentstacks/      types · port · app · fs · process · host (see docs/architecture.md)
CLAUDE.md:108:docs/                    contributor docs until the book exists
README.md:86:| Your own | Write a tool extension against the public API: [`docs/authoring.md`](docs/authoring.md) |
README.md:115:[`docs/authoring.md`](docs/authoring.md), or its raw link:
README.md:144:├─ docs/                 contributor docs, until the book exists                     built
README.md:150:Contributor docs live in [`docs/`](docs/README.md) until the book exists. Start with
README.md:151:[`docs/architecture.md`](docs/architecture.md): the layers, the dependency rule, where logic lives,
crates/agentstacks/src/types/pack/install/set.rs:227:                "docs/x.md",
crates/agentstacks/src/types/pack/install/set.rs:234:            "exclude = [\"**/*_test.lua\", \"docs/**\"]",
crates/agentstacks/src/types/pack/install/set.rs:241:        assert!(!set.is_rendered(&tp("docs/x.md")));
crates/agentstacks/src/types/pack/load.rs:635:        let tree = with_manifest("exclude = [\"docs/**\"]", "").file("docs/x.md", b"{{bad}}\n");
crates/agentstacks/src/types/pack/load.rs:637:        assert!(!names(pack.rendered().iter()).contains(&"docs/x.md"));
crates/agentstacks/src/types/pack/load.rs:1045:            .file("docs/SKILL.md", b"{{var arguments}}\n");
crates/agentstacks/src/types/pack/load.rs:1049:                "{ROOT}/docs/SKILL.md:1: var \"arguments\" is only valid in SKILL.md"
crates/agentstacks/src/types/pack/local.rs:209:        let other_skill_md = format!("{DIR}/docs/SKILL.md");
crates/claudevs/Cargo.toml:13:exclude    = ["docs/"]
crates/claudevs/src/contract/site.rs:14:/// release notes and `docs/` tree are not — nothing loads them, and a path
crates/claudevs/src/contract/site.rs:124:            "docs/design.md",
packs/core/agents/coder.md:22:If your task references a spec or plan (e.g. under `docs/specs/` or `docs/plans/`), read the named section before you start.
packs/core/agents/reviewer.md:67:Did we build the RIGHT thing? Read the spec/plan named in your brief (e.g. under `docs/specs/`, `docs/plans/`) and judge the delivery against it:
packs/core/hooks/globs_test.lua:33:    assert(matches("docs/**/*.md", "docs/b.md") == true)
packs/core/hooks/globs_test.lua:34:    assert(matches("docs/**/*.md", "docs/a/b.md") == true)
packs/core/scripts/handoff_report_test.lua:267:    assert(report.under_handoff_root("/repo/docs/spec.md", nil) == false)
packs/guideline-python/skills/guidelines/references/docstring-discipline.md:208:grep -rnE '\.claudestacks|\.airsstack|docs/(specs|plans)|(Phase|Task|Step) [0-9]|subagent|implementer|brainstorm|the (plan|spec)\b|(Claude|Opus|Sonnet|Haiku)|(PR|issue) #[0-9]+' \
packs/guideline-rust/skills/guidelines/references/workspace.md:54:- Top-level stays scannable (workspace root, tooling dirs, `docs/`, `crates/`).
packs/guideline-rust/skills/guidelines/references/workspace.md:101:- **Globbing in `members` without an `exclude`** — `members = ["*"]` will pick up `docs/`, tooling dirs, etc. Use `crates/*` instead.
$ control: git grep -c "packs/" -- Makefile.toml
Makefile.toml:12
== d. link checkers in the other books
airsl: grep exit=1
airsl control (mdbook pin): 1
buildl: grep exit=1
buildl control (mdbook pin): 1
claudestacks: grep exit=1
claudestacks control (mdbook pin): 1
== e. GitHub's default shell
910:| Linux / macOS      | unspecified       | The default shell on non-Windows platforms. Note that this runs a different command to when `bash` is specified explicitly. If `bash` is not found in the path, this is treated as `sh`.                                                          | `bash -e {0}`                                   |
2029:| Linux / macOS      | unspecified       | The default shell on non-Windows platforms. Note that this runs a different command to when `bash` is specified explicitly. If `bash` is not found in the path, this is treated as `sh`.                                                          | `bash -e {0}`                                   |
2394:| Linux / macOS      | unspecified       | The default shell on non-Windows platforms. Note that this runs a different command to when `bash` is specified explicitly. If `bash` is not found in the path, this is treated as `sh`.                                                          | `bash -e {0}`                                   |
== f. claudevs move check over the probe's moved files
identical=22 differ=0
```
