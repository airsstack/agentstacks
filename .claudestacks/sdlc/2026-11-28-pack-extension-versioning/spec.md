---
status: approved
created: 2026-11-28
built-on: 358a0a62197d
---

# Spec: Independent per-artifact versions for packs and extensions

This spec replaces the bundle version dial with eight independent streams —
one tag series per official pack and per official extension — while keeping
every published `packs-v*` tag resolving forever. It settles the scheme, the
manifest discipline, the behavior of `init`, `update` and `doctor`, the gates,
the changelogs, the book, and the release automation for the cutover and for
steady state.

## 1. Decision record

Three approaches were considered against GitHub issue #26, which opens:
"Packs and extensions currently carry **three version dials that disagree**,
and none of them tells a user what code they are running" (body fetched via
`gh issue view 26 --repo airsstack/agentstacks --json`, this session — the
three dials being the `packs-v*` tag, frozen per-pack `version` fields, and
lockstep per-extension `version` fields). *Tag-only* (drop manifest versions) breaks the frozen pack format
and lock format, so it needs `api = 2`. *Align-to-tag* (every manifest mirrors
the bundle tag) ships inside 1.x but preserves lockstep and robs versions of
per-artifact meaning. *Independent streams* keeps the format frozen, gives
each artifact a version that tracks its own content, and keeps the tag as the
distribution unit — one stream per artifact instead of one bundle. The author
chose independent streams on 2026-11-28, with frozen-not-deleted bundle tags,
warn-don't-rewrite migration, and advisory-only freshness in `doctor`.

## 2. Tag scheme

- Each official artifact owns one tag stream. Packs mint
  `packs-<name>-v<semver>` (`packs-sdlc-v1.1.0`); extensions mint
  `ext-<name>-v<semver>` (`ext-claude-v1.2.0`). *Proof of shape: the streams
  did not exist yet at spec time (negative: full `git ls-remote --tags
  origin` matches no `packs-<name>-v*` or `ext-<name>-v*` tag — control: the
  same listing shows exactly the six bundle tags `packs-v0.1.0` through
  `packs-v1.2.0`, and the stream-pattern grep
  (`refs/tags/(packs-[a-z]+-v|ext-[a-z]+-v)`) printed nothing).*
- Stream prefixes never collide with each other or with the retired bundle
  prefix. Run 1 (this session): `printf '%s\n' packs-v1.2.0 packs-core-v1.0.1
  packs-sdlc-v1.1.0 ext-claude-v1.2.0 | grep "^packs-v"` printed only
  `packs-v1.2.0`. Run 2: `printf '%s\n' packs-v1.2.0 packs-core-v1.0.1 |
  grep "^packs-core-v"` printed only `packs-core-v1.0.1`. Neither direction
  leaks.
- Bare `packs-v` (no artifact name) and bare `ext-v` are reserved permanently
  and never minted, so no future stream reads as the retired bundle scheme.
- The `packs-v*` bundle stream is frozen: no new bundle tags are minted, and
  every published bundle tag stays on the remote resolving forever. *Proof:
  the six bundle tags are pushed on origin (full `git ls-remote --tags
  origin`, this session); 30 Rust source files under `crates/`
  (`grep -rl "packs-v" --include="*.rs" crates/ | wc -l` printed 30, this
  session) and 9 book chapters (`grep -rln "packs-v" book/src`, this session)
  reference the prefix.*
- The `api`-major rule generalizes. Today `official_major` returns a major
  only for the official URL with exactly prefix `packs-v`
  (`crates/agentstacks/src/types/shorthand.rs:43,75-76`). After this change a
  closed prefix table (the eight official streams) maps to `API_MAJOR` the
  same way: official repo plus known stream prefix admits only that major.
  Unknown prefixes keep today's behavior (no major filter).
- Every tag is annotated and pushed alone. *Proof: the runbook requires solo
  pushes because GitHub fires no event past three tags at once
  (`RELEASING.md:22-23`).* The seed release therefore pushes eight times.

## 3. Manifest discipline

- A manifest version moves if and only if that artifact's shipped content
  changed. Worked example done right: the `packs-v1.1.1` release moved only
  `packs/core/pack.toml`, with the hunk `-version = "1.0.0"` / `+version =
  "1.0.1"` (*proof: `git show f3cdc7d -- packs/core/pack.toml`, this
  session*), for the handoff-path fix `c4c599f` in the same range (*proof:
  `git log --oneline packs-v1.1.0..packs-v1.1.1 -- packs/core` prints
  `f3cdc7d` and `c4c599f`, this session*). Control: the release commit
  touches exactly two files, so the other four packs sat still (*proof: `git
  show --stat f3cdc7d` lists only `packs/CHANGELOG.md` and
  `packs/core/pack.toml`, this session*). Anti-example: the `packs-v1.2.0`
  release moved all three `extension.toml` files `1.0.0` to `1.1.0` while its
  `--stat` totals 12 files — crate version bumps, two test files, three
  extension manifests, one changelog — with nothing else under `extensions/`
  (*proof: `git show --stat 0b41d85` and `git show 0b41d85 -- extensions/`,
  this session*).
- Shipped scope equals the installed set as computed by the existing
  pack/extension loaders — the same code `plan` uses — minus the release
  paperwork itself (`pack.toml` / `extension.toml`, `CHANGELOG.md`).
  Consequences follow without enumerating file kinds: a `README.md` typo or
  a `*_test.lua` change never forces a bump because the loaders exclude them;
  a `SKILL.md`, adapter, or hook change always does; editing `exclude`
  itself changes the installed set and therefore requires a bump. *Proof of
  the loader behavior: `exclude` means excluded from installation
  (`book/src/ch06-04-pack-authoring.md:33,69,82`), parsed as install-exclusion
  globs (`crates/agentstacks/src/types/manifest/pack/file.rs:188-190`), and
  a `SKILL.md` cannot be excluded (same chapter, refusal table).*
- Every bump carries a changelog entry in that artifact's own file (section
  6). A bump without an entry fails the gate the same way a bump without
  content does.
- Bump size (patch vs minor vs major) is suggested from Conventional Commits
  since the stream's last tag (`fix` suggests patch, `feat` suggests minor)
  and decided by the reviewer. The gate never polices size. Worked
  suggestion: commits `packs-v1.2.0..HEAD` under `packs/sdlc` are exactly one
  `feat` (`82d7561 feat(packs): rename sdlc chain root ...`, this session),
  so the suggestion for the seed release is minor — `sdlc` `1.0.0` to
  `1.1.0`. Empty or non-conventional history yields no suggestion and the
  maintainer picks; a stream's first tag takes its stipulated seed version
  (section 9).

## 4. Resolution behavior

- `init` and `ext add` resolve each requested official artifact under its own
  stream prefix: each pack name to the newest `packs-<name>-v*` tag of the
  `api` major this binary speaks, each tool name to the newest
  `ext-<name>-v*` tag.
  *Proof of the machinery reused: `newest_release` lists one prefix and picks
  the newest tag of the allowed major
  (`crates/agentstacks/src/app/newest.rs:40-50`); tag ranking ignores
  pre-releases and filters majors
  (`crates/agentstacks/src/types/resolve/tag.rs:92-108`); the binary never
  moves to a later `api` (`crates/agentstacks/src/types/shorthand.rs:53-56`).
  The per-stream generalization itself is new code: its checked examples
  arrive as plan-stage tests (FakeGit stream listings to newest, section
  11).*
- `update` keeps one entry per git source in today's shape
  (`crates/agentstacks/src/app/update.rs:10-13,61-103`) but lists each source under
  its own stream prefix. A source pinned to a legacy `packs-v*` tag is
  listed, reported as frozen, and pointed at its artifact stream — it is
  never rewritten automatically (author decision 2026-11-28). Prescribed
  note: `agentstacks.toml: packs.<name>: pinned to bundle tag packs-v<version>
  (frozen stream); <stream>-v<newest> is available — retag to migrate`. New
  note wordings are pinned by doctests the same way the existing ones are
  (*proof of the convention: `UpdateNote` doctests assert exact strings,
  `crates/agentstacks/src/types/resolve/update.rs:43-52`*).
- `doctor` keeps its drift half exactly as today — lock plus disk plus the
  airsl pin, strict, offline-capable
  (`crates/agentstacks/src/app/doctor.rs:40-75`) — and gains a read-only
  freshness half. For each locked git entry it splits the lock's recorded tag
  into prefix and version, lists that prefix, and reports a newer tag as an
  advisory note. *Proof of the split rule: the version starts at the leftmost
  ASCII digit whose remainder parses as semver, with `packs-v0.1.0` splitting
  to prefix `packs-v`, version `0.1.0`
  (`crates/agentstacks/src/types/resolve/tag.rs:45-60` with doctest at
  31-36); stream-tag splits arrive as plan-stage unit tests (section 11).*
  One rule, no new inputs: a legacy `packs-v` prefix takes the
  frozen-warning branch (the target stream is derived from the entry's kind
  plus name); any other prefix is listed for a newer tag, official or
  third-party alike.
- Freshness advisories never change the exit status: drift findings still
  exit 1, staleness and unreachable-remotes report at exit 0 (author decision
  2026-11-28). A missing network, missing git, or unlistable remote degrades
  to a `could not check <name>` note, never an error — `doctor` on an
  airplane behaves exactly as today. *Proof of today's contract: no network,
  no git, no extension (`crates/agentstacks/src/app/doctor.rs:1-14`); proof
  of the tolerated-failure pattern reused: git-missing degrades to a named
  list instead of failing (`crates/agentstacks/src/app/update.rs:89-97`).
  Offline and exit-0 behavior arrive as plan-stage tests (section 11).*
- `path` sources report nothing under freshness, matching `update` today.
  Entries carrying a `rev` pin instead of a tag report `pinned to rev <rev>,
  not updated`, matching `UpdateNote::PinnedRev` (*proof:
  `crates/agentstacks/src/types/resolve/update.rs:54-76`*). A tag with no parseable
  version reports `tag "<tag>" has no version to update from`, matching
  `UpdateNote::NoVersion`. A stream with no tags yet reports the existing
  no-release-tag shape as an advisory note. Exact note wording is prescribed
  here and carries no stability weight. *Proof: the stability chapter covers only location and
  key shapes of messages, never exact text
  (`book/src/ch06-06-stability.md`, "What is not covered").*
- A pinned tag missing upstream reports `pinned tag <tag> not found upstream`
  as advisory; resolution itself keeps today's failure at `plan`/`apply`
  time (*proof of the failure shape: `book/src/ch05-00-plan-and-apply.md:304`
  records the unresolvable-ref refusal*).
- The official-URL byte-equality rule wins over prefix appearance: a `.git`
  suffix or any other URL makes the source third-party and skips the major
  filter, even if its tags look official.
  *Proof: documented in `book/src/ch05-00-plan-and-apply.md` and coded at
  `crates/agentstacks/src/types/shorthand.rs:55-59`.*

## 5. Gates

- The PR gate fails a change that alters shipped scope without a manifest
  bump, a manifest bump without shipped-scope change, and either without a
  changelog entry. Scope and paperwork exclusions follow section 3. Both
  failing directions are enforced (author decision 2026-11-28: strict).
- The PR gate diffs the pull request against its merge-base with the target
  branch; the release gate diffs the release commit against the stream's last
  tag. Both exclude manifests and changelogs from the content comparison, or
  every release trivially "changes." Multi-artifact diffs are evaluated per
  artifact independently; a diff touching only paperwork requires no bump and
  passes. Worked verdicts (recorded runs, this session): commit `82d7561`
  changed 26 shipped files under `packs/sdlc` (`git diff --stat
  packs-v1.2.0..HEAD -- packs/sdlc`, 160 insertions, 66 deletions) while
  `packs/sdlc/pack.toml` still declares `1.0.0` with no changelog entry —
  FAIL under both directions of the rule. The seed release is exempt by
  definition: with no prior tag there is no base to diff against, so its
  versions are stipulated in section 9 and the gates go live at the seed
  commit.
- Release-side Lua (bump suggestion, manifest/tag agreement) ships as pure
  functions with `*_test.lua` suites, so the existing `airsl test` gate
  covers it. *Proof the precedent exists: the `lua` cargo-make task already
  runs `airsl check` and `airsl test` over `extensions/`, `packs/`, and the
  conformance assets (`Makefile.toml:112-118`), including an
  `--allow-exec git` pack-test invocation.*

## 6. Changelogs

- One `CHANGELOG.md` per official pack and per official extension, colocated
  with its artifact, in Keep a Changelog shape with artifact-version
  headings. The shared `packs/CHANGELOG.md` freezes with a top note pointing
  at the eight successors; its history is closed, not rewritten. *Proof the
  bundle file is the shared record today: per-release `packs-v*` headings
  with per-pack versions inside (`packs/CHANGELOG.md`, read in this
  session).*

## 7. Book and runbook

- One new book section maps the scheme: stream tag (distribution, per
  artifact), manifest version (declares the release), `agentstacks.toml` tag
  (what the user pins), `api` (compatibility), lock tag plus commit plus tree
  (installed proof). Gate-enforced equality of manifest and tag is the
  release-time invariant; where pre-gate historical data disagrees, the tag
  wins for resolution. This section is
  written first and pins the vocabulary the code and gates reuse: *stream*,
  *bundle tag*, *frozen*, *seed* (author decision 2026-11-28).
- Updated chapters: packs (no longer "released together as `packs-v*`"),
  command line (`update` per-stream listing, `doctor` freshness half and exit
  codes), plan-and-apply (prefix rules, frozen-stream warning, migration
  path), pack and extension authoring (version means "this artifact changed";
  bump-iff-changed; changelog required), stability (stream prefix format
  frozen for 1.x), first-project tutorial (stream tag examples), 0.x
  migration (bundle-to-stream path), and the introduction's release pointer.
  *Proof of current coverage: `grep -rln "packs-v" book/src` lists 9
  chapters (this session).*
- The stability promise extends to the stream prefix format
  (`packs-<name>-v`, `ext-<name>-v` frozen for 1.x); renaming a prefix later
  breaks every pin. `RELEASING.md` replaces its `<packs-version>` placeholder
  with per-artifact versions and records the solo-push discipline per stream.
- Code and chapters ship in the same change, per the book's own rule that a
  chapter describes what the code does (*proof: `CLAUDE.md:99-101`*).

## 8. Release automation

- A `release-artifact` workflow on `workflow_dispatch` takes `(artifact,
  version)`, re-verifies a clean tree with manifest equal to version and a
  changelog entry present, mints one annotated tag, pushes it alone, and
  smoke-verifies with a fresh `init` plus `update` in a throwaway home —
  mirroring the runbook's verification subshell (*proof: `RELEASING.md`,
  Later-releases step 4*). The workflow needs `contents: write` for the tag
  push; the existing release workflow and manual-dispatch `pages.yml` are the
  templates (*proof: `RELEASING.md:107` — the workflow starts on
  `agentstacks-cli-v*` and `claudevs-cli-v*` tags and `packs-v*` tags start
  none*).
- A local shell script, `scripts/release-artifact.sh`, wraps the dispatch:
  preflight (clean tree, latest `main`, manifest equals version, entry
  present, tag absent locally and on the remote), then `gh workflow run`
  plus `gh run watch`. Refusals, each named before anything is pushed: dirty
  tree, version mismatch, missing changelog entry, tag already present, airsl
  missing, dispatch failure. Smoke assertions mirror the runbook: a fresh
  `init` resolves the new tag, `update` discovers it, `doctor` is healthy.
  Direct `git tag -a` plus `git push origin <tag>` stays documented as the
  Actions-down fallback. Shell effects follow the repo's precedent
  (`install/install.sh`, `packs/sdlc/scripts/fingerprint.sh`,
  `packs/sdlc/scripts/status.sh` — all present and read in this session).
- The decision half of the release logic (bump suggestion, manifest/tag
  agreement) lives in Lua functions with `*_test.lua` suites under the
  existing `airsl test` gate. *Proof of capability: `airsl run --help`
  documents `--policy trusted` as "Every safe Lua library, unrestricted
  grants, no ceilings. First-party scripts only" and `airsl doctor` lists
  `proc` among its modules (runs recorded in this session, airsl 0.1.5).*
  The Lua-only invariant governs shipped agent content in the sandbox
  (extensions propose operations; the core validates and writes — project
  guidance `AGENTS.md`, read at session start), not maintainer tooling.
  Crates.io publishing stays manual. Automation and gates cover official
  artifacts only.

## 9. Seed release and rollout order

- One release commit: `sdlc` `1.0.0` to `1.1.0` for its unreleased chain-root
  content, every other manifest untouched, eight changelog seed entries.
  Stipulated seed baselines: `core` 1.0.1, `sdlc` 1.1.0, `journal` 1.0.0,
  `guideline-rust` 1.0.0, `guideline-python` 1.0.0, `claude`/`musecode`/
  `opencode` 1.1.0 (*proof: current manifest `grep '^version'` across all
  eight artifacts, this session*). *Proof the sdlc bump is owed: `git diff
  --stat packs-v1.2.0..HEAD -- packs/sdlc` shows 26 changed files under no
  tag (this session).* The seed versions and distributes #27's content; it
  does not re-do or duplicate #27's migration, which stays shipped
  separately per the intent's non-goals.
- Eight annotated stream tags minted at that commit, pushed one at a time.
  No `packs-v*` tag is minted or moved. The gates go live at the seed
  commit: nothing before it is judged, everything after it is.
- Tags ship before the `agentstacks`/`agentstacks-cli` `1.2.0` to `1.3.0`
  minor release (dual-scheme support, no format break, `api` stays 1; the
  release itself follows the existing crates.io rules, unchanged).
  Rationale: a new binary whose `init` resolves streams finds nothing until
  streams exist, while old binaries ignore unknown tags — the old binary only
  ever queries its hardcoded `packs-v` prefix
  (`crates/agentstacks/src/types/shorthand.rs:43`), so seed data leads and
  the reader follows.

## 10. Edge cases

- Empty or malformed manifest version: refused as today by the existing
  version parser, which accepts only valid semver and reports `"X" is not
  semver` (*proof:
  `crates/agentstacks/src/types/manifest/pack/version.rs:51-55`*). No new
  error shape.
- Unknown artifact names: the stream prefix table is closed (five packs,
  three extensions). A name outside it is refused as not-an-official-pack or
  not-an-official-tool, reusing the existing refusal shapes (*proof:
  `crates/agentstacks/src/app/init.rs:591` and
  `crates/agentstacks/src/app/ext/add.rs:456`*).
- Lookalike tags: matching anchors the full stream prefix at the tag start
  and requires the remainder to parse as semver
  (`crates/agentstacks/src/types/resolve/tag.rs:97-103`); pre-releases are
  ignored by the same rule. `packs-core-v1.0.1` never matches `packs-v`, and
  `packs-v1.2.0` never matches `packs-core-v` (proven by the section-2 run).
- A stream with no tags yet: the existing no-release-tag refusal
  (`no <prefix>1 release tag in <url>`, exercised in
  `crates/agentstacks/src/app/newest.rs:98-109`). This is the expected state
  of a stream before its seed tag.
- Mixed pins (some sources on bundle tags, some on streams): entries are
  per-source and independent, so each resolves, warns, or notes on its own.
- A pinned tag missing upstream (deleted or never pushed): freshness reports
  `pinned tag <tag> not found upstream` as advisory; resolution itself keeps
  today's failure at `plan`/`apply` time.
- Manifest-plus-changelog-only diffs with no version bump: no shipped-scope
  change, so no bump is required and the gate passes. (A bump present in such
  a diff is the bump-without-content failure of section 5.)
- Missing airsl for the Lua release script: the script refuses with a
  pointer to the installer instead of running degraded — release mechanics
  never guess. The pointer text is prescribed at plan time and pinned by a
  test, like every other new refusal in this spec.
- Third-party git sources: the split rule applies to their own tags with no
  major filter, exactly as `update` treats non-official sources today.
- Concurrent artifact releases: tags are independent and pushed one at a
  time, so two streams shipping from one commit cannot collide.

## 11. Testing

- Unit tests with the existing fakes (`FakeGit`, `FakeWorkspace`) are
  required for: the closed prefix table (eight streams, unknown names
  refused); per-stream newest selection including pre-release exclusion and
  the empty-stream NoTag advisory; mixed bundle/stream pins resolving
  independently; the frozen-stream note naming the right artifact stream;
  freshness advisories on staleness, missing tags, and unreachable remotes
  all leaving exit 0; offline `doctor` behaving exactly as today.
  *Proof the fakes exist for exactly this: `newest.rs` tests build tag
  listings through `FakeGit` (lines 64-94, read in this session).*
- Unit tests split every stream-tag shape the spec relies on
  (`packs-<name>-v`, `ext-<name>-v`, legacy `packs-v`) through the real
  split rule — the checked examples this spec could not run because the tags
  do not exist yet.
- Contract tests against real git cover prefix-filtered listing
  (*proof: existing cases at
  `crates/agentstacks/src/process/git/cli.rs:258-283`*).
- Helper scope tests assert excluded paths (`README.md`, `*_test.lua`,
  release paperwork) never force a bump, an `exclude` edit always does, and
  a `SKILL.md`, adapter, or hook change always does — with the scope computed
  by the loaders, not reimplemented.
- Lua release logic ships `*_test.lua` suites under the existing `airsl test`
  gate. Layering stays intact: no new adapter imports in `types`, `port` or
  `app`. *Proof of the enforced boundary: `crates/agentstacks/tests/
  isolation.rs` exists (verified in this session) per the project guidance
  (`AGENTS.md`, `CLAUDE.md`, read at session start).*
- The Definition of Done stays green throughout (`cargo make dod`), plus the
  named corpora the runbook requires: `claudevs-check`, `opencode-check`,
  `musecode-check`, the installers and book jobs (*proof: the command list
  in `CLAUDE.md`, read at session start*).

## 12. Non-goals

- The sdlc chain-root rename and its migration (shipped separately in #27).
- Migrating the SDLC board off `.claudestacks/sdlc/`.
- Binary, library, or claudevs versioning schemes.
- Rewriting history or deleting published tags.
- Policing bump size, or a strict mode that fails `doctor` on staleness.
- Any `api = 2` format change.
