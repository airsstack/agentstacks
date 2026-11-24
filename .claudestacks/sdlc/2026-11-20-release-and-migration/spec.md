---
status: approved
created: 2026-11-24
built-on: 726d0575c25a
---

# Spec: First release and migration

This spec ships agentstacks to users who have no Rust toolchain. It covers the first release of the
four crates on crates.io, prebuilt `agentstacks` and `claudevs` binaries through dist, the
`packs-v0.1.0` tag that makes `init --tool claude --pack <p>` resolve official names, installers
that also install `airsl` when it is missing, a migration guide from the claudestacks marketplace
plugins, the README and org-doc status changes, and the archive of claudestacks after all of that
is out. No Rust module is added, moved or changed. The work is versions, release configuration,
installer scripts and docs.

Probe outputs live in `probes/` next to this file, and the scripts that produced them in
`probes/scripts/`. Every probe ran on 2026-11-24 against worktree HEAD `0fe1f61`, cargo 1.94.1,
dist (`cargo-dist`) 0.33.1 and jq 1.7.1.

## 1. Premises (proven)

| # | Fact | Proof |
|---|---|---|
| P1 | `airsstack/agentstacks` is public: an anonymous `git ls-remote --tags` succeeds and lists no tags | `probes/init-results.md` §3 (`exit=0`, empty) |
| P2 | Today `init --tool claude --pack core` refuses with `--pack "core": no packs-v release tag in https://github.com/airsstack/agentstacks` and `--tool "claude": …`, exit 1 | `probes/init-results.md:7-9` |
| P3 | With a `packs-v0.1.0` tag on the official repository, `init --tool claude --pack core --pack guideline-rust` writes `[tools.claude]`/`[packs.*]` with `git = "https://github.com/airsstack/agentstacks"`, `tag = "packs-v0.1.0"` and the `subdir`, and `plan` exits 0. Proven by redirecting the official URL to a bare copy of HEAD with `insteadOf` | `probes/init-results.md` §2 |
| P4 | `plan` with no `airsl` on `PATH` prints `agentstacks.toml: airsl not found on PATH (needed by core)` and exits 1; with an airsl that reports 0.1.4 it prints `agentstacks.toml: airsl 0.1.4 installed, packs need >=0.1.5` and exits 1 | `probes/airsl-results.md:6-11` |
| P5 | `core`, `journal` and `sdlc` require `airsl = ">=0.1.5"`; `guideline-rust` and `guideline-python` do not | `probes/docs-excerpts.md` (last block) |
| P6 | claudevs never runs the `airsl` binary: there is no `"airsl"` in `crates/claudevs{,-cli}/src` (grep exit 1), while the same grep finds `const AIRSL: &str = "airsl";` in `crates/agentstacks/src/process/binary.rs:26` | `probes/airsl-results.md:18-27` |
| P7 | crates.io has no `agentstacks` or `agentstacks-cli`; it has `claudevs` 0.1.0 and `claudevs-cli` 0.1.0, 0.1.1 | `probes/publish-results.md` (crates.io state) |
| P8 | `cargo publish --workspace --dry-run --allow-dirty` with agentstacks 0.1.0, agentstacks-cli 0.1.0, claudevs 0.1.1, claudevs-cli 0.1.2 and `publish = true` packages all four crates in dependency order and exits 0, with no warning except `aborting upload due to dry run` | `probes/publish-results.md`, `probes/spec-examples-results.md` §2 (both filtered to the lines named in their headers) |
| P9 | dist's plan step fails on a `packs-v0.1.0` tag: `unexpected character 'p' while parsing major version number` | `probes/dist-lockstep-results.md:68` |
| P10 | dist's default tag trigger is `'**[0-9]+.[0-9]+.[0-9]+*'`, which matches `packs-v0.1.0`. In GitHub's filter syntax `**` matches any characters, `+` one or more of the preceding character, `[]` one character of a range | `probes/dist-lockstep-results.md:78`; `probes/workflow-syntax-filter.md:6-9` |
| P11 | With the trigger edited to `agentstacks-cli-v…` and `claudevs-cli-v…`, `dist plan --tag=agentstacks-cli-v0.1.0` exits 255 (`help: run 'dist init' to update the file ('allow-dirty' in Cargo.toml to ignore out of date contents)`). With `allow-dirty = ["ci"]` it exits 0 and releases only `agentstacks-cli` 0.1.0 with `agentstacks-cli-installer.sh`. `claudevs-cli-v0.1.2` releases only `claudevs-cli` 0.1.2. `dist generate` then keeps the edited trigger | `probes/dist-results.md:36-64`; `probes/spec-examples-results.md` §1 (the §4 texts) |
| P12 | With per-binary versions, an untagged `dist plan` fails under `pr-run-mode = "upload"` (`There are too many unrelated apps in your workspace to coherently Announce!`, exit 255) and passes under `"plan"` (exit 0) | `probes/dist-pr-results.md:7-20` |
| P13 | Under `pr-run-mode = "plan"` the build job runs only when publishing or in upload mode. `custom-smoke` needs `build-local-artifacts`, and GitHub skips a job whose needed job was skipped. So a pull request runs only the plan job. `host` needs `custom-smoke` to succeed or be skipped | `probes/dist-pr-results.md` (plan-mode section: generated line 96 `if:`, `custom-smoke` `needs`, line 239 `host` `if:`) and `probes/dist-pr-results.md:68-72` (GitHub's `needs` rule) |
| P14 | The six targets build on `macos-15` (aarch64-apple-darwin), `macos-15-intel`, `ubuntu-22.04` and `ubuntu-22.04-arm`, and each release has a `<app>-installer.sh` | `probes/dist-lockstep-results.md` (runner lines); `probes/dist-results.md:51,54` |
| P15 | GitHub's latest release is the most recent non-prerelease, non-draft release of the whole repository, and new releases default to latest | `probes/rest-releases.md:196-197,379` |
| P16 | GitHub creates no push event for tags when more than three tags are pushed at once | `probes/webhook-events-tags.md:15` |
| P17 | Unauthenticated GitHub REST requests are limited to 60 an hour | `probes/rate-limits.md:17` |
| P18 | This POSIX sh pipeline finds the newest stable `<app>-vX.Y.Z` release from `GET /repos/{o}/{r}/releases?per_page=100` whatever the API's order: `0.1.1` for claudestacks `claudevs-cli` (mixed with airsclaude tags), `0.1.5` for airsl `airsl-cli`, empty for agentstacks today; the sort orders `0.1.10` below `0.2.0` | `probes/latest-results.md` (script lines 6-12, output lines 30-45) |
| P19 | Archiving makes a repository read-only for all users. Archived repositories keep serving the Pages site they already had (`microsoft/autogen`: archived, `build_type` workflow, 200). claudestacks' Pages is workflow-built | `probes/archiving.md:3,17`; `probes/pages-results.md` |
| P20 | `https://airsstack.github.io/airsl/install.sh` and `…/claudestacks/install.sh` serve 200; `…/agentstacks/install.sh` serves 404 and agentstacks has no Pages site (API 404) | `probes/pages-results.md` |
| P21 | airsl's Pages deploys from an Actions artifact and needs the repository's Pages source set to "GitHub Actions" | `/Users/hiraq/Projects/airsstack/airsl/.github/workflows/pages.yml:6-9` |
| P22 | airsl's and claudestacks' pointer scripts are POSIX sh with `say`/`fail`/`download` helpers and `main "$@"` as the last line; `sh -n` passes both | `/Users/hiraq/Projects/airsstack/airsl/install/install.sh`; `/Users/hiraq/Projects/airsstack/claudestacks/install/install.sh:44-74`; `probes/misc-results.md` |
| P23 | `sh -n` reads commands without executing them (POSIX). The macOS `sh -n` exits 2 on a script with an unclosed `if` and 0 on a good one | `probes/docs-excerpts.md` (POSIX block); `probes/misc-results.md` |
| P24 | crates.io keywords: at most 5, ASCII, at most 20 characters, starting alphanumeric, only letters, numbers, `_`, `-`, `+`. The category slugs `command-line-utilities`, `development-tools` and `development-tools::testing` exist | `probes/docs-excerpts.md`; `probes/misc-results.md` |
| P25 | The agentstacks smoke steps need no tool on `PATH` (no git, no airsl). With `PATH=/nonexistent`, `init --tool ext/claude --pack packs/guideline-rust --yes` exits 0 and prints `applied: 11 written, 0 deleted`; `plan` prints `0 to write, 0 to delete, 11 unchanged, 0 skipped` and exits 0; `doctor` prints `healthy` and exits 0 | `probes/smoke-results.md`, script `probes/scripts/smoke2-probe.sh` |
| P26 | `claudevs --version` prints `claudevs <version>`, and `agentstacks --version` prints `agentstacks <version>` (`claudevs 0.1.1`, `agentstacks 0.0.0` today). The claudevs smoke fixtures from claudestacks exist under `crates/claudevs/tests/fixtures/` | `probes/smoke-results.md`; `probes/init-results.md` (`--version`); `probes/paths-results.md` (fixtures) |
| P27 | Claude Code: `claude plugin uninstall <plugin>@<marketplace> --scope <scope>` uninstalls one plugin (uninstall defaults to user scope); `claude plugin marketplace remove <name>` uninstalls every plugin installed from that marketplace and removes their `enabledPlugins` entries | `probes/discover-plugins.md:328-338,380-382` |
| P28 | The claudestacks marketplace is named `claudestacks` and lists `claudestacks`, `claudestacks-sdlc`, `claudestacks-guideline-rust`, `claudestacks-guideline-python`, `claudestacks-journal`, `claudestacks-cmux` | `/Users/hiraq/Projects/airsstack/claudestacks/.claude-plugin/marketplace.json` |
| P29 | The sdlc pack uses the same `.claudestacks/sdlc/` paths as the plugin (identical path strings in both skill sets). The journal vault root is the plugin's (`$AIRSSTACK_HOME/journal`, default `~/.airsstack/journal`), and `lib/vault.lua` without comment lines is identical to the plugin's. The snapshot store is `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/` in both | `packs/journal/scripts/lib/vault.lua:127-133`; `probes/paths-results.md`; `probes/round2-results.md` §e |
| P30 | The pack text gates: `Makefile.toml:173` greps every file under `packs/` except `README.md` and `*_test.lua` for plugin syntax and plugin-qualified names; `Makefile.toml:181` greps every `*.md` under `packs/` except `README.md` and two guideline files for unprefixed skill, agent or command names | `Makefile.toml:171-196` |
| P31 | The plugin-to-pack renames and user-visible behaviour changes are listed in the packs spec | `.claudestacks/sdlc/2026-11-20-packs-and-claudevs/spec.md` §6.1 and §9 |
| P32 | `init --tool claude --pack core --pack sdlc --pack guideline-rust` (official names, `packs-v0.1.0`) writes the project file and `plan` exits 0 (`76 to write, 0 to delete, 0 unchanged, 1 skipped`). Without `--pack core` it refuses: `agentstacks.toml: packs.sdlc: sdlc requires pack "core", which agentstacks.toml does not list (add it: --pack core)` | `probes/round2-results.md` §a |
| P33 | airsl's pointer, under `/bin/sh` and `/bin/dash`: `''` and `latest` take the latest path; `v0.1.5` and `0.1.5` give `fetching the installer for airsl 0.1.5`; `0.1 5`, `a/b` and `v` give `AIRSL_VERSION='<value>' is not a version (expected MAJOR.MINOR.PATCH or latest)`; `0.2.0-rc.1` is accepted as given. A failed download prints one error. With `PATH=/nonexistent` it dies on `mktemp: command not found` (exit 127) before any helper runs | `probes/round2-results.md` §b |
| P34 | The dist plan manifest has `.releases[]` with `app_name`, `app_version` and `artifacts` (a list of artifact names), and `.artifacts[<name>]` with `kind` (`executable-zip` for archives) and `target_triples`. The §6 jq program over it yields six `{archive, target, app, bin, version}` rows per binary tag, `bin` being `agentstacks` or `claudevs`, `alpine` true only on the musl rows, merged with claudestacks' runner table | `probes/round2-results.md` §c (raw keys); `probes/jq-results.md` (the exact §6 program) |
| P35 | dist's generated shell installer installs to `$HOME/.local/bin` (help text and the default branch), unless `AGENTSTACKS_CLI_INSTALL_DIR` (or `CARGO_DIST_FORCE_INSTALL_DIR`) is set | `probes/round2-results.md` §d |
| P36 | `cargo publish` uploads one package, then polls until it appears in the index; that poll may time out, which does not affect the upload | `probes/docs-excerpts.md` (cargo publish block) |
| P37 | GitHub's "List releases" section states no order (no `sort`, `order` or `created_at` in it), while the latest-release section does say `sorted by the created_at attribute` | `probes/round2-results.md` §f |

## 2. Versions and crate metadata

| Crate | Version | Why |
|---|---|---|
| `agentstacks` | 0.1.0 | first release (P7) |
| `agentstacks-cli` | 0.1.0 | first release (P7) |
| `claudevs` | 0.1.1 | crates.io has 0.1.0 (P7) |
| `claudevs-cli` | 0.1.2 | crates.io has 0.1.1 (P7) |

Root `Cargo.toml`:

- `[workspace.package] publish = true` (was `false`).
- `[workspace.dependencies]` `agentstacks = { path = "crates/agentstacks", version = "0.1.0" }` and
  `claudevs = { path = "crates/claudevs", version = "0.1.1" }`.

Member manifests:

| Crate | Adds |
|---|---|
| `agentstacks` | `readme = "README.md"`, `keywords = ["agents", "claude-code", "cli", "lua", "setup"]`, `categories = ["development-tools"]` |
| `agentstacks-cli` | `readme = "README.md"`, `keywords = ["agents", "claude-code", "cli", "lua", "setup"]`, `categories = ["command-line-utilities", "development-tools"]` |
| `claudevs`, `claudevs-cli` | version only; their `readme`, `keywords` and `categories` stay |

The keywords and categories meet the crates.io rules (P24). The publish dry run accepts this exact
metadata (`probes/spec-examples-results.md` §2).

New files. Every link in them is an absolute `https://github.com/airsstack/agentstacks/…` URL,
because crates.io renders the README away from the repository:

- `crates/agentstacks/README.md`: what the library is, that it backs the `agentstacks` CLI, that
  its API is unstable during `0.x`, and a link to `docs/architecture.md`.
- `crates/agentstacks-cli/README.md`: install with the §5.3 one-liner, or with
  `cargo install agentstacks-cli` plus the note "this route does not install airsl, which `plan`
  needs (≥0.1.5)"; usage (`init --tool claude --pack core`, `plan`, `apply`, `doctor`, `update`);
  links to the repository README and `docs/migration.md`.

`crates/claudevs-cli/README.md:12` becomes
`curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh   # prebuilt binary, no Rust needed`.
The book links in both claudevs READMEs point to the claudestacks book. It keeps being served
after the archive (P19), so they stay.

## 3. CHANGELOGs

Every file follows Keep a Changelog 1.1.0 with the header shape of
`crates/claudevs/CHANGELOG.md:1-5`. Every new entry is dated `Unreleased`. The author replaces
`Unreleased` with the release day in the release pull request (§9 step 1).

| File | Entry |
|---|---|
| `crates/agentstacks/CHANGELOG.md` (create) | `## [0.1.0] - Unreleased`: initial release, minimum supported Rust 1.94; Added: the pack, extension, project and lock formats; the plan, apply, doctor and update flows; the extension host and the hook runtime |
| `crates/agentstacks-cli/CHANGELOG.md` (create) | `## [0.1.0] - Unreleased`: initial release; Added: `init`, `plan`, `apply`, `doctor`, `update`; official names `--tool claude` and `--pack core\|sdlc\|journal\|guideline-rust\|guideline-python`, resolved to the newest `packs-v*` tag; prebuilt archives for the six targets of §4 and the installer `curl -LsSf https://airsstack.github.io/agentstacks/install.sh \| sh`, which also installs airsl when it is missing |
| `crates/claudevs/CHANGELOG.md` | `## [0.1.1] - Unreleased`: Changed: the source moved to https://github.com/airsstack/agentstacks; behaviour is unchanged. (The packs spec §2 is the source of "unchanged"; the entry carries no citation) |
| `crates/claudevs-cli/CHANGELOG.md` | `## [0.1.2] - Unreleased`, built on `claudevs` 0.1.1: Changed: built and released from https://github.com/airsstack/agentstacks; the prebuilt installer is `curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh \| sh` |
| `packs/CHANGELOG.md` (create) | exactly the text below |

`packs/CHANGELOG.md`, exactly:

```markdown
# Changelog

All notable changes to the official packs and tool extensions are recorded here. They are released
together under `packs-v*` tags, which `agentstacks init` and `agentstacks update` resolve. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and each pack and extension
follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [packs-v0.1.0] - Unreleased

First release.

### Added

- Packs `core`, `sdlc`, `journal`, `guideline-rust` and `guideline-python`, each at 0.1.0.
- The `claude` tool extension at 0.1.0, on extension API `api = 0`.

Coming from the claudestacks plugins? See [docs/migration.md](../docs/migration.md).
```

Text rule for later entries: name no skill, agent, command or plugin, only pack and extension names,
versions and docs. The pack text gates scan this file (P30), and `cargo make dod` runs them. Both
gates pass the text above (`probes/spec-examples-results.md` §3).

## 4. Release workflow (dist)

### 4.1 `dist-workspace.toml` (create)

Keys exactly as below. The comments say why each non-default key exists.

```toml
[workspace]
members = ["cargo:."]

# Config for 'dist'
[dist]
# The preferred dist version to use in CI (Cargo.toml SemVer syntax)
cargo-dist-version = "0.33.1"
# CI backends to support
ci = "github"
# Shell only: agentstacks and claudevs run on Unix, not Windows.
installers = ["shell"]
# macOS and Linux on both architectures, plus a musl build of each Linux one.
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "aarch64-unknown-linux-musl", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"]
# Not CARGO_HOME: the prebuilt binaries exist for engineers who have no Rust toolchain.
install-path = "~/.local/bin"
hosting = "github"
install-updater = false
# An attestation ties each archive's digest to this repository, commit and workflow.
github-attestations = true
# `host` waits for this workflow, so an archive that fails its smoke test is never published.
global-artifacts-jobs = ["./smoke"]
# agentstacks-cli and claudevs-cli carry their own versions. An untagged plan that builds
# ("upload") refuses two apps with different versions, so pull requests run `dist plan` only.
pr-run-mode = "plan"
# release.yml's tag trigger is edited by hand (see the comment there). Without this, dist
# reports the edit as out of date and fails its plan step.
allow-dirty = ["ci"]

# GitHub retired every macOS 14 image on 2026-11-02, and dist's default runner for
# aarch64-apple-darwin is still macos-14. Keep this entry whenever this file is rewritten.
[dist.github-custom-runners]
aarch64-apple-darwin = "macos-15"
```

### 4.2 `.github/workflows/release.yml` (create)

To produce it: write §4.1 without the `allow-dirty` line, run `dist generate` (dist 0.33.1), edit
the trigger, then add the `allow-dirty` line. The only hand edit replaces the `on:` block with the
block below, comment included:

```yaml
# Edited by hand, kept by allow-dirty = ["ci"] in dist-workspace.toml: only agentstacks-cli and
# claudevs-cli tags start a release. Library tags (agentstacks-v*, claudevs-v*) and packs-v* tags
# do not; dist's plan step fails on a packs-v* tag. To regenerate this file, remove allow-dirty,
# run `dist generate`, then reapply this edit and allow-dirty.
on:
  pull_request:
  push:
    tags:
      - 'agentstacks-cli-v[0-9]+.[0-9]+.[0-9]+*'
      - 'claudevs-cli-v[0-9]+.[0-9]+.[0-9]+*'
```

Why: the default trigger matches `packs-v0.1.0` (P10), and dist fails on that tag (P9). With the
edit and `allow-dirty`, each binary tag plans exactly one app, and an untagged plan passes (P11,
P12; the two texts above are what `probes/spec-examples-results.md` §1 ran).

### 4.3 Tags

| Tag | Marks | Starts a release |
|---|---|---|
| `agentstacks-v<version>` | the `agentstacks` crate on crates.io | no |
| `agentstacks-cli-v<version>` | the `agentstacks` binary release | yes |
| `claudevs-v<version>` | the `claudevs` crate on crates.io | no |
| `claudevs-cli-v<version>` | the `claudevs` binary release | yes |
| `packs-v<version>` | the official packs and extensions that `init`/`update` resolve | no |

- Every tag is annotated (`git tag -a <tag> -m <tag>`) and pushed alone (`git push origin <tag>`),
  because GitHub creates no event when more than three tags are pushed at once (P16).
- The first release tags the merged release commit with all five, in the order of §9.
- Later, each crate and the packs release on their own. A new `packs-vX.Y.Z` needs no binary
  release; `update` picks the newest stable `packs-v*` tag (`docs/plan-and-apply.md:336`).

## 5. Installers

### 5.1 Hosting: `.github/workflows/pages.yml` (create)

The shape of `/Users/hiraq/Projects/airsstack/airsl/.github/workflows/pages.yml`, minus the book.
Triggers are a push to `main` touching `install/**` or the workflow, and `workflow_dispatch`. The
build job checks out, copies `install/install.sh` and `install/claudevs.sh` into a `site/`
directory and uploads it with `actions/upload-pages-artifact@v5` (`path: site`). The deploy job is
airsl's (`actions/deploy-pages@v5`, `pages: write`, `id-token: write`, environment `github-pages`),
with the same `concurrency` and `permissions`. The scripts are served at
`https://airsstack.github.io/agentstacks/install.sh` and `…/claudevs.sh`. The repository's Pages
source must be set to "GitHub Actions" first (P21), an author step (§9).

### 5.2 Shared shape

Both scripts are POSIX sh, the shape of airsl's pointer (P22): a header comment with the usage
lines, `set -eu`, `say` (prints `<p>: <text>` to stderr), `fail` (`say "error: <text>"`, exit 1),
`download` (curl with `--proto '=https' --tlsv1.2 -sSfL`, else `wget -qO-`), a `main`, and
`main "$@"` as the last line. Why sh and not Lua: they run before airsl exists, as airsl's and
claudestacks' pointers do (P22). `main` runs in this order:

```text
1. no curl and no wget (command -v, a builtin) → M1. Nothing else runs before this check, so M1 is
   the only message (airsl's script would die on mktemp first, P33).
2. tmp = mktemp -d; trap removes it on exit.
3. version = ${<VAR>:-latest}, one leading "v" stripped.
4. version = latest:
     say M3
     download https://api.github.com/repos/airsstack/agentstacks/releases?per_page=100 > "$tmp/releases.json"
       (a plain redirect in the current shell, never inside $( ); fails → M4)
     version = the P18 grep | sed | sort | tail over "$tmp/releases.json"; empty → M5
   contains a char outside [0-9A-Za-z.+-], or empty after the strip → M2
   anything else → used as given (a pre-release such as 0.2.0-rc.1 is allowed, never chosen as latest)
5. url = https://github.com/airsstack/agentstacks/releases/download/<app>-v<version>/<app>-installer.sh
   say M6; download url > "$tmp/<app>-installer.sh"; fails → M7
6. sh "$tmp/<app>-installer.sh" "$@"; a non-zero exit ends the script with that status.
7. (install.sh only) the airsl step, §5.3.
```

The rules of steps 3-4 are airsl's `case` (P33). `releases/latest` is not used: it is repo-wide,
so after a claudevs-only release it would hold no agentstacks installer (P15).

### 5.3 `install/install.sh` (create)

Prefix `agentstacks-install`, `<VAR>` = `AGENTSTACKS_VERSION`, `<app>` = `agentstacks-cli`,
`<name>` = `agentstacks`. Usage lines in the header:

```text
curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh
curl -LsSf https://airsstack.github.io/agentstacks/install.sh | AGENTSTACKS_VERSION=0.1.0 sh
```

The airsl step, because agentstacks needs airsl (P4):

```text
command -v airsl succeeds                   → say M8 with that path; done
else "$HOME/.local/bin/airsl" is executable → say M8 with that path; done
else → say M9; download https://airsstack.github.io/airsl/install.sh > "$tmp/airsl-install.sh"
       and run it with sh (download or run fails → M10)
```

`$HOME/.local/bin` is checked by path because that is where dist's installers put binaries by
default (P35; airsl's `dist-workspace.toml` sets the same `install-path`), and the running shell
may not have it on `PATH` yet. An airsl that is found is never replaced or upgraded, even when it
is too old; `plan` then names the version it needs (P4). Arguments go only to the agentstacks
installer. Environment variables reach both installers unchanged (for example
`AGENTSTACKS_CLI_INSTALL_DIR`, `AIRSL_VERSION`).

### 5.4 `install/claudevs.sh` (create)

Prefix `claudevs-install`, `<VAR>` = `CLAUDEVS_VERSION`, `<app>` = `claudevs-cli`,
`<name>` = `claudevs`. No airsl step, because claudevs never runs the airsl binary (P6). Usage lines:

```text
curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh
curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | CLAUDEVS_VERSION=0.1.2 sh
```

### 5.5 Messages (exact; `<p>` is the script's prefix, `<name>`/`<VAR>`/`<app>` per script)

| # | When | Text (stderr) | Exit |
|---|---|---|---|
| M1 | neither curl nor wget | `<p>: error: neither curl nor wget is installed` | 1 |
| M2 | bad version | `<p>: error: <VAR>='<raw value>' is not a version (expected MAJOR.MINOR.PATCH or latest)` | 1 |
| M3 | latest lookup starts | `<p>: finding the latest <name> release` | — |
| M4 | API download failed | `<p>: error: could not list the releases at https://api.github.com/repos/airsstack/agentstacks/releases (GitHub allows 60 unauthenticated requests an hour); set <VAR> to a version to skip this lookup` | 1 |
| M5 | no stable `<app>-v*` release | `<p>: error: found no <name> release at https://github.com/airsstack/agentstacks/releases` | 1 |
| M6 | fetching | `<p>: fetching the installer for <name> <version>` | — |
| M7 | installer download failed | `<p>: error: could not download the installer for <name> <version>; is <version> a released version? See https://github.com/airsstack/agentstacks/releases` | 1 |
| M8 | airsl present (install.sh only) | `agentstacks-install: airsl found at <path>; leaving it as it is` | — |
| M9 | airsl missing (install.sh only) | `agentstacks-install: agentstacks needs airsl, which is not installed; installing the latest airsl with https://airsstack.github.io/airsl/install.sh` | — |
| M10 | airsl install failed (install.sh only) | `agentstacks-install: error: agentstacks is installed, but installing airsl failed; install it with: curl -LsSf https://airsstack.github.io/airsl/install.sh \| sh` | 1 |

The dist installer's own failures print dist's text, and the script exits with its status.

## 6. Smoke workflow: `.github/workflows/smoke.yml` (create)

Written by hand, with `on: workflow_call` taking an input `plan` (string), and
`permissions: contents: read`. The shape and the runner table come from
`/Users/hiraq/Projects/airsstack/claudestacks/.github/workflows/claudevs-cli-smoke.yml`, with the
header comment adapted to two apps.

**targets job.** It builds the matrix with this jq program over the plan (`jq -c --argjson runners
"$runners"`), proven on both binary tags (P34):

```text
. as $all
| [.releases[] as $r
   | $r.artifacts[]
   | select($all.artifacts[.].kind == "executable-zip")
   | {archive: ., target: $all.artifacts[.].target_triples[0],
      app: $r.app_name, bin: ($r.app_name | sub("-cli$"; "")), version: $r.app_version,
      alpine: false}
     + ($runners[$all.artifacts[.].target_triples[0]] // {})]
```

`$runners` is claudestacks' six-row table. The empty-matrix and missing-runner errors are
claudestacks' (`the plan lists no archives to smoke-test`, `no smoke runner for: <targets>`). The
version comes per archive, never from `.releases[0]`.

**smoke job** (per archive, `fail-fast: false`, bash, env `ARCHIVE`, `BIN_NAME` = matrix `bin`,
`VERSION` = matrix `version`): checkout, download `artifacts-build-local-*`, check the checksum,
unpack, then find the file named `$BIN_NAME`. If it is missing: `no $BIN_NAME binary inside $ARCHIVE`.
musl archives run inside `alpine:3.20` through a wrapper, as in claudestacks. The binary's path
(or the wrapper's) goes into `$GITHUB_ENV` as `BIN`; claudestacks calls this variable `CLAUDEVS`,
and every copied step uses `"$BIN"`. Then:

| Step | When | Check |
|---|---|---|
| version | always | `"$BIN" --version` equals `$BIN_NAME $VERSION`, else `$BIN_NAME --version printed '<got>', expected '$BIN_NAME $VERSION'` (P26) |
| claudevs doctor, wiring fixtures, test-stage fixtures | `BIN_NAME == claudevs` | claudestacks' three steps with `$CLAUDEVS` → `$BIN`, nothing else changed, including `if: !matrix.alpine` on the test stage (the fixtures exist here, P26) |
| agentstacks project | `BIN_NAME == agentstacks` | In `.smoke/project` (inside the workspace, so the Alpine mount sees it): copy `extensions/claude` to `ext/claude` and `packs/guideline-rust` to `packs/guideline-rust`. Run `init --tool ext/claude --pack packs/guideline-rust --yes` (exit 0, a line matching `^applied: [0-9]+ written, 0 deleted$`), then `plan` (exit 0, a line matching `^0 to write, 0 to delete, [0-9]+ unchanged, 0 skipped$`), then `doctor` (exit 0, a line equal to `healthy`). On a miss: `agentstacks <command> exited <code>, expected 0` or `agentstacks <command> did not print '<expected>'` |

`guideline-rust` is used because it needs no airsl (P5), and these commands need no tool on `PATH`,
not even git (P25). So the Alpine archives run every agentstacks step. The counts are regexes, so
a change to the pack's file count does not break smoke.

A pull request does not run this workflow, because `pr-run-mode = "plan"` builds nothing (P13). It
runs on each binary tag, and `host` publishes only after it succeeds (P13). Its first run is
therefore on the first binary tag, which §9 pushes before anything irreversible happens.

## 7. CI and docs in this repository

### 7.1 `.github/workflows/ci.yml` (modify)

Add a job `installers` (`runs-on: ubuntu-latest`, `timeout-minutes: 5`) that checks out the
repository and runs, for each of `install/install.sh` and `install/claudevs.sh`:

| Check | Expect |
|---|---|
| `sh -n <script>` | exit 0 (P23) |
| `env -i PATH=/nonexistent /bin/sh <script>` | exit 1, and stderr is exactly the M1 line. This proves §5.2 step 1 comes first. `command -v` and `printf` are shell builtins, so no external tool is needed to reach M1 |
| `env -i PATH=/usr/bin:/bin HOME="$RUNNER_TEMP" <VAR>='a/b' /bin/sh <script>`, with `curl` and `wget` present on the runner | exit 1, and stderr is exactly the M2 line with `'a/b'` |

All three are offline. Nothing else in `ci.yml` changes.

### 7.2 `README.md` (modify)

- Status block becomes: `> **Status: pre-release (0.1.0).** The CLI and the Claude Code extension
  are released, with the five official packs (`packs-v0.1.0`). OpenCode is not here yet. The
  extension API is `api = 0` and will change. Coming from the claudestacks plugins? See
  [docs/migration.md](docs/migration.md).`
- A new `## Install` section before `## Usage`: the install.sh one-liner, the line that it also
  installs airsl when it is missing, and `cargo install agentstacks-cli` with the §2 airsl note.
- `## Usage` leads with the official-names block, which works now:
  `agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust` (P32: `sdlc`
  needs `core`), then `plan`, `apply`, `doctor`, `update`. The "Works today, with … local paths"
  block becomes "Local extensions and packs work too", with the same commands. The
  `--tool opencode` example and the "Planned, once the official packs are released" heading go.
- Tools table: Claude Code → `Released (`extensions/claude`, `packs-v0.1.0`)`.
- Packs: one line saying they are released as `packs-v0.1.0`.
- Layout tree: add `install/` (installer scripts, served on Pages) and `RELEASING.md`.
- Roadmap: `M1  claude extension (api = 0) ✓ released (0.1.0); the claudestacks plugins are packs`.

### 7.3 `docs/migration.md` (create)

For a user of the claudestacks marketplace plugins. Sections, in order:

1. **What maps to what**: `claudestacks` → `core`, `claudestacks-sdlc` → `sdlc`,
   `claudestacks-journal` → `journal`, `claudestacks-guideline-rust` → `guideline-rust`,
   `claudestacks-guideline-python` → `guideline-python`, `claudestacks-cmux` → not converted, keep
   the plugin if you use it (P28; packs spec §9 last row).
2. **Install agentstacks**: the install.sh one-liner (installs airsl when missing).
3. **Remove the plugins**, so their skills and hooks don't run twice. Per plugin:
   `claude plugin uninstall <plugin>@claudestacks --scope <scope>`, with the scope you installed it
   at (default user). Or remove the whole marketplace with `claude plugin marketplace remove
   claudestacks`, which uninstalls every plugin from it, cmux included (P27).
4. **Set up each project**: `agentstacks init --tool claude --pack core --pack sdlc --pack journal
   --pack guideline-rust` (pick the packs you used; `sdlc` requires `core`, P32), review the plan,
   answer the apply prompt, then commit `agentstacks.toml`, `agentstacks.lock` and `.claude/`.
   Packs install per project, never machine-wide.
5. **What changed**: packs spec §9's rows, rewritten for users (every row except the last, which is
   step 1's cmux line). The command and agent renames come first, as an old → new table
   (`/claudestacks:x` → `/core-x`, … from §9 row 1 and §6.1). Then the rest: airsl on `PATH`
   ≥0.1.5, reported by `plan` and `doctor`; `style_reinject` in `agentstacks.toml`
   `[packs.core.options]`; `/core-statusline`; the enforce hook finds guideline packs installed in
   the same project; `sdlc` requires `core`; the enforce pointer names `/guideline-rust-guidelines`;
   the enforce doctor's changes; the SessionEnd reminder shown as a skip by `plan`; the
   SubagentHandback deny text.
6. **What stays**: `.claudestacks/sdlc/` chain files are read and written at the same paths. The
   journal vault stays at `$AIRSSTACK_HOME/journal` (default `~/.airsstack/journal`), and the
   snapshot store at `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/` (P29).
7. **claudevs users**: `curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh`;
   `cargo install claudevs-cli` is unchanged; the old claudestacks installer URL forwards here once
   §8.2 lands.

The doc cites no `.claudestacks/` planning path for its content. `docs/README.md` gains a row for
`migration.md`.

### 7.4 `RELEASING.md` (create)

The author's checklist: §9 as numbered steps with its commands and recovery lines, and the tag
table of §4.3. It says that `cargo publish` is irreversible (a version can be yanked, never
replaced) and that each tag is pushed alone. It also says to revisit the installers' release
lookup when the repository passes 100 releases (§10).

### 7.5 `CLAUDE.md` (modify)

- Status heading: `## Status: core, CLI commands, claude extension, packs and claudevs built; first release configured`.
- The layout block gains `dist-workspace.toml`, `install/` and `RELEASING.md`.
- In Commands, the line `When dist is added, set aarch64-apple-darwin = "macos-15" … smoke
  workflow.` becomes: `dist-workspace.toml sets aarch64-apple-darwin = "macos-15" under
  [dist.github-custom-runners], and smoke.yml uses the same runner. Releases follow RELEASING.md.`
- The CI sentence names the `installers` job too.
- The "Archiving claudestacks … is `2026-11-20-release-and-migration` work" sentence becomes
  `claudestacks is archived after the first release (RELEASING.md); never delete it: releases,
  install.sh and the old marketplace still point there.`

## 8. Outside this repository

### 8.1 Org docs (files in `/Users/hiraq/Projects/airsstack`, a local repo, and its `.github` repo)

A plan edits these files in place; they are plain files outside this worktree, with no branch to
make. They state that claudestacks is archived, so the author commits them only after the archive
(§9 step 11).

`.github/profile/README.md`:

- Line 25: `SDLC["agentstacks<br/>agent tool setup: packs + extensions"]`. Line 46: `style SDLC
  fill:#4a3f2a,stroke:#c9a35c` (amber = pre-release; the key is at line 57).
- Projects table: insert above the claudestacks row
  `| [**agentstacks**](https://github.com/airsstack/agentstacks) | Run the factory | A CLI that sets up agentic coding tools in a project. **Packs** hold tool-neutral content (skills, agents, commands, hooks, rules); **tool extensions** in Lua, sandboxed by airsl, render them in each tool's own format, as a plan you review before it is applied. Claude Code first, OpenCode next. Hosts [**claudevs**](https://crates.io/crates/claudevs), a deterministic test engine for Claude Code setups. | Pre-release |`.
  The claudestacks row's text after its link becomes `Archived. Its Claude Code plugins became the agentstacks packs and claudevs moved to agentstacks; it still hosts airsclaude until airsdk.`
  and its status becomes `Archived`.
- The airsl row: `Powers buildl and the claudestacks hooks.` → `Powers buildl and the agentstacks hooks.`
- "Where we are heading": `Nothing below has moved yet.` → `agentstacks has moved; airsdk has not yet.`
  The `claudestacks` → **agentstacks** row stays as the record of the move.
- Line 103: ``Every change moves through `claudestacks-sdlc`:`` → ``Every change moves through the agentstacks `sdlc` pack:``.
- Line 111: `the agentstacks and airsdk regrouping` → `the airsdk regrouping`.

`README.md` (org root):

- Line 44 (map): `│ claudestacks  sdlc + plugins    │` → `│ agentstacks   tool setup: packs │`
  (the cell keeps its 33 characters).
- Line 55: `Planned regrouping, three repos with one job each (nothing has moved yet):` →
  `Regrouping into three repos with one job each (agentstacks has moved; airsstack and airsdk have not):`.
- Lines 92-94 become exactly:

  ```text
  ├── agentstacks/      agent tool setup: packs + extensions, claudevs   (pre-release)
  ├── airsstack/        controller CLI          ┐ planned: fresh private repos,
  └── airsdk/           provider SDKs           ┘ no code yet
  ```
- Projects table, the agentstacks row: name `[**agentstacks**](https://github.com/airsstack/agentstacks) (from claudestacks)`,
  status `Pre-release`, Docs `[README](https://github.com/airsstack/agentstacks#readme)`, text
  unchanged except `claudevs moves here.` → `Hosts claudevs.`.
- The claudestacks row: text `Archived: its plugins became the agentstacks packs and claudevs moved to agentstacks; it still hosts airsclaude until airsdk.`,
  status `Archived`, Docs unchanged.
- The airsl row: `Powers buildl and the claudestacks hooks.` → `Powers buildl and the agentstacks hooks.`
- Line 116: `claudestacks hooks ► airsl` → `agentstacks hooks ─► airsl` (same width).
- Line 156: ``Every change moves through `claudestacks-sdlc`:`` → ``Every change moves through the agentstacks `sdlc` pack:``.

`CLAUDE.md` (org root). Its map states agentstacks' status ("no code yet"), which is the same
status fact the intent's org-docs outcome covers:

- Line 24: `claudestacks/   airsclaude SDK, claudevs plugin test engine, Claude Code plugin suite` →
  `claudestacks/   archived; still hosts the airsclaude SDK until airsdk`.
- Lines 26-28 become exactly:

  ```text
  agentstacks/    agent tool setup, packs + extensions, claudevs   pre-release (0.1.0)
  airsstack/      controller CLI (planned, kubectl-style)  ┐ fresh private repos, created
  airsdk/         provider SDKs (AI first, cloud later)    ┘ 2026-11-20, no code yet
  ```

### 8.2 claudestacks (applied by the author, or a session opened in the claudestacks repository, on a branch)

This work is not in this chain's plans: it lands in another public repository through its own
pull request.

- `README.md`: a banner first, under the title: `> **Archived.** The Claude Code plugins are now
  agentstacks packs and claudevs lives in agentstacks: https://github.com/airsstack/agentstacks.
  Migrating: https://github.com/airsstack/agentstacks/blob/main/docs/migration.md. airsclaude stays
  here until it moves to airsdk.`
- `install/install.sh` becomes a forwarder, with the same prefix (`claudevs-install`) and helpers:
  - `CLAUDEVS_VERSION` unset, empty or `latest`: say `claudevs moved to https://github.com/airsstack/agentstacks; running https://airsstack.github.io/agentstacks/claudevs.sh`,
    download that script to the temp directory and run it with `sh <file> "$@"`. The environment,
    `CLAUDEVS_VERSION` included, passes through.
  - an explicit version: the current claudestacks download (`claudevs-cli-v<version>` from
    claudestacks releases, which has 0.1.0 and 0.1.1). If that download fails, forward as above.
  - a bad version: today's text, unchanged:
    `CLAUDEVS_VERSION='<value>' is not a version (expected something like 0.1.0)`
    (`/Users/hiraq/Projects/airsstack/claudestacks/install/install.sh:54`).
  - the forwarded script fails to download:
    `claudevs-install: error: could not download https://airsstack.github.io/agentstacks/claudevs.sh`, exit 1.
- Check before merging: `sh -n install/install.sh`. After its Pages deploy: §9 step 9's run.
- It is merged and its Pages deploy has finished **before** the archive. An archived repository is
  read-only (P19), so nothing can redeploy its Pages afterwards.

### 8.3 Archive

At §9 step 10: Settings → Archive, or `gh repo archive airsstack/claudestacks`. Never delete it.
Its releases, the marketplace and the Pages site stay reachable (P19). airsclaude is frozen until
airsdk; a fix needs an unarchive, a release and a re-archive (decided with the author 2026-11-24).

## 9. Release runbook (the author's steps, in order)

Nothing here is a plan task; the plans deliver the files. RELEASING.md holds this list. Binaries
release before crates.io, because dist builds from the repository and every step before step 6 can
be undone.

1. Release pull request (this chain's work) green: `cargo make dod`, `cargo make claudevs-check`,
   `cargo deny check`, the `installers` job, and the release workflow's plan job. In the pull
   request, replace every `Unreleased` in the five CHANGELOGs with the day you will run steps 3-6.
   Merge.
2. Settings → Pages → Source: GitHub Actions. Run `pages.yml` (workflow_dispatch). Check that
   `https://airsstack.github.io/agentstacks/install.sh` and `…/claudevs.sh` serve 200.
3. On the merged commit, push `packs-v0.1.0`.
4. Push `agentstacks-cli-v0.1.0` and wait for its release workflow to finish green; then the same
   for `claudevs-cli-v0.1.2`. **If one fails**, nothing was published (`host` waits for smoke,
   P13). Delete any GitHub Release it left, then delete the tag (`git tag -d <tag>`,
   `git push origin :refs/tags/<tag>`), fix it in a pull request, and tag the same version on the
   new merged commit. The version is still free, because crates.io comes later.
5. Verify, in one subshell with a throwaway home and a minimal `PATH`:

   ```sh
   (
     export HOME="$(mktemp -d)" PATH=/usr/bin:/bin
     curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh    # prints M9: no airsl here
     curl -LsSf https://airsstack.github.io/agentstacks/claudevs.sh | sh
     export PATH="$HOME/.local/bin:$PATH"
     agentstacks --version    # agentstacks 0.1.0
     claudevs --version       # claudevs 0.1.2
     airsl --version          # airsl 0.1.5 or newer
     cd "$(mktemp -d)" && agentstacks init --tool claude --pack core --yes \
       && grep 'tag = "packs-v0.1.0"' agentstacks.toml && agentstacks doctor    # healthy
   )
   ```

   A failure here is fixed like step 4's: nothing is on crates.io yet.
6. Publish to crates.io, one crate at a time, from a clean, up-to-date `main`:
   `cargo publish --workspace --dry-run`, then ✋ `cargo publish -p agentstacks`,
   `cargo publish -p agentstacks-cli`, `cargo publish -p claudevs`, `cargo publish -p claudevs-cli`.
   Each command polls until its crate is in the index. On a timeout, confirm the version on
   crates.io before the next command (P36). **If a publish fails**, the crates before it stay
   published, and you continue from the failed crate once the cause is fixed. A fix that changes
   that crate's code needs a new version: bump it, and for a `-cli` crate release that version
   again through step 4.
7. Push `agentstacks-v0.1.0` and `claudevs-v0.1.1`, each alone.
8. claudestacks: apply §8.2 on a branch, merge, and wait for its Pages deploy.
9. Check the forwarder: `( export HOME="$(mktemp -d)" PATH=/usr/bin:/bin; curl -LsSf https://airsstack.github.io/claudestacks/install.sh | sh && "$HOME/.local/bin/claudevs" --version )`
   prints `claudevs 0.1.2`.
10. Archive claudestacks (§8.3).
11. Commit the org docs (§8.1) in the org root and push the `.github` repository.
12. `/claudestacks-sdlc:milestone foundation/01` to roll the milestone up to done.

## 10. Edge cases (settled)

| Case | Behaviour |
|---|---|
| `<VAR>` empty, or `latest` | latest lookup (`${VAR:-latest}`, as airsl does, P33) |
| `<VAR>=v0.1.0` | one leading `v` stripped → `0.1.0` (P33) |
| `<VAR>` with a space, `/`, or only `v` | M2 (P33) |
| `<VAR>` is not a released version | M7 |
| Pre-release tags (`…-v0.2.0-rc.1`) | never picked as latest (the grep wants plain `X.Y.Z`, P18); installable by naming them (P33) |
| No curl and no wget | M1 only (§5.2 step 1; checked by the `installers` job) |
| API rate limit hit, or the API unreachable | M4, which names `<VAR>` as the way around it (P17) |
| No release of that app yet | M5 |
| More than 100 releases | only the first page is read, and the API states no order (P37), so the newest could be missed. Accepted at today's scale; RELEASING.md says to revisit at 100 releases |
| airsl on `PATH`, or at `~/.local/bin/airsl` | left alone (M8), even if older than 0.1.5; `plan` names the version it needs (P4) |
| airsl only in a directory off `PATH` other than `~/.local/bin` | not found; airsl's installer puts a second copy in its default directory (M9) |
| `AGENTSTACKS_CLI_INSTALL_DIR` set | agentstacks goes there (P35). The airsl check is unchanged, and airsl's installer uses its own default unless `AIRSL_CLI_INSTALL_DIR` is set too |
| Running install.sh again | agentstacks reinstalled by dist's installer; airsl found (M8) |
| agentstacks installer fails | the script exits with its status; airsl is not attempted |
| `cargo install agentstacks-cli` | no airsl. README and the crate README say so (§2) |
| A binary tag whose version differs from its manifest | dist's plan fails (`The provided announcement tag (agentstacks-cli-v0.1.9) claims we're releasing agentstacks-cli 0.1.9, but that …`, exit 255; `probes/spec-examples-results.md` §1) and nothing is published. Recover as in §9 step 4 |
| Release workflow or smoke fails on a binary tag | nothing is published (`host` needs smoke, P13); §9 step 4's recovery, before crates.io |
| `cargo publish` fails or times out partway | §9 step 6: published crates stay; continue from the failed crate |
| `packs-v*`, `agentstacks-v*`, `claudevs-v*` pushed | no release workflow run (§4.2 trigger, P10) |
| More than 3 tags in one push | no events (P16), so the runbook pushes one at a time |
| Pull request | the release workflow runs plan only and passes (P12, P13). Builds and smoke run on the tag |
| Windows | no installer or archive (§4.1). Not claimed to build |
| claudestacks installer with an explicit old version | the claudestacks release (0.1.0, 0.1.1); newer versions forward to agentstacks (§8.2) |
| Plugins and packs both active | duplicate skills and hooks. The migration doc removes the plugins first (§7.3 step 3) |
| README example without `--pack core` | refused, because `sdlc` requires `core` (P32). Every example in §7.2 and §7.3 lists `core` with `sdlc` |

## 11. Architecture (approved by the author on 2026-11-24)

No Rust module is added, moved or changed. `docs/architecture.md` and the crate layering are
untouched. The rust-guidelines references were read for this spec. The rules applied come from
`workspace.md`: members inherit `[workspace.package]`; publishable members carry a `description`
and their own `README.md`; workspace dependencies carry `version` and `path`; publish dependency
first. `modularity.md`, `mod-rs-export-only.md`, `static-dispatch.md` and `unit-test-mandate.md`
have nothing to apply, since no source file changes.

```text
agentstacks/  (this repo)
├─ Cargo.toml                    publish = true; workspace dep versions (§2)
├─ crates/agentstacks{,-cli}/    + README.md + CHANGELOG.md; readme/keywords/categories (§2, §3)
├─ crates/claudevs{,-cli}/       version bump + CHANGELOG entry; claudevs-cli README install line
├─ packs/CHANGELOG.md            packs-v releases (§3)
├─ dist-workspace.toml           §4.1
├─ .github/workflows/
│  ├─ release.yml               dist-generated + the trigger edit (§4.2)
│  ├─ smoke.yml                 §6
│  ├─ pages.yml                 §5.1
│  └─ ci.yml                    + installers job (§7.1)
├─ install/install.sh, claudevs.sh    §5
├─ docs/migration.md (+ docs/README.md row)   §7.3
├─ RELEASING.md                  §7.4, §9
└─ README.md, CLAUDE.md          §7.2, §7.5

outside: org root docs (§8.1: edited by a plan, committed by the author after the archive) ·
         claudestacks README + install.sh forwarder (§8.2, author) · archive (§8.3, author)
```

Dependency direction is unchanged: `agentstacks-cli → agentstacks`, `claudevs-cli → claudevs`,
and both libraries → `airsl` from crates.io.

## 12. Testing

| What | How |
|---|---|
| Workspace still green | `cargo make dod` (includes `cargo make lua`, which runs the pack text gates over `packs/CHANGELOG.md`), `cargo make claudevs-check`, `cargo deny check` |
| Manifests publishable | `cargo publish --workspace --dry-run` exits 0 (P8; rerun at §9 step 6) |
| dist config | the release workflow's plan job on the pull request (P12) |
| Archives | smoke.yml on every binary tag, before publishing (§6) |
| Installer scripts | the `installers` job (§7.1: parse, M1 first, M2); end to end at §9 step 5 |

The design-time probes of this spec's exact texts (the §4.1 file and §4.2 trigger with `dist plan`,
the §2 metadata with the publish dry run, and `packs/CHANGELOG.md` with both pack text gates) are
in `probes/spec-examples-results.md`.

## 13. Non-goals

- Windows archives or a PowerShell installer.
- Installing, pinning or upgrading airsl beyond §5.3; that belongs to `airsstack tool install`.
- An automated release command or a `/release` skill; the author runs §9.
- The book (`book/`) and putting it on Pages.
- Migration tooling beyond the documented steps (milestone non-goal).
- Moving airsclaude or openrouter-rs (airsdk).
- Changing `TAG_PREFIX` or any Rust code.
- Converting `claudestacks-cmux`.
