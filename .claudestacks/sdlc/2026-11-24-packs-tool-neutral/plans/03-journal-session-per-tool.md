---
status: approved
created: 2026-11-25
depends-on: [01]
built-on: 63f62977694c
---

# Journal Per-Tool Session Lookup Implementation Plan

**Goal:** The journal's capture and note find the session key and transcript through one native
file per tool.

**Architecture:** Two new native files, `packs/journal/native/{claude,opencode}/skills/journal-capture/session.md`,
hold each tool's session key, transcript lookup and clean-up. The neutral `capture` and `note`
skills read `<project_dir>/{{skill_dir journal/capture}}/session.md`, pass `cleanup` to their
agent, and relay the receipt when it arrives. The agents gain a `cleanup` input they run before the
receipt. `transcript-path.lua` is unchanged and is run only by the Claude file. No Rust `src/`
file changes. Depends on plan 01 for `packs.rs`'s both-tools `project()` and `init_both`.

**Tech Stack:** Markdown pack text, the agentstacks native-file mechanism, Rust integration tests,
airsl 0.1.5.

**Content authority:** spec §3 (flow), §5.1, §5.2, §5.3, §9 (journal README paragraph), D1, D5,
D6, E1-E13, the §4.2 rows for `journal/skills/{capture,note}` and `journal/agents/{capture,note}`.

---

## Files that change

```
packs/journal/native/claude/skills/journal-capture/session.md    — create: Claude id8, tpath via transcript-path.lua, empty cleanup
packs/journal/native/opencode/skills/journal-capture/session.md  — create: OpenCode id8, service check, export, cleanup
crates/agentstacks-cli/tests/packs.rs                            — modify: journal_session_lookup_is_per_tool
packs/journal/skills/capture/SKILL.md                            — modify: steps 1, 2, 5, 6, "If the agent is unavailable"
packs/journal/skills/note/SKILL.md                               — modify: steps 2, 3, 6, 7, "If the agent is unavailable"
packs/journal/agents/capture.md                                  — modify: description, inputs, step 1 wording, step 8
packs/journal/agents/note.md                                     — modify: inputs, step 8
packs/journal/references/note-schema.md                          — modify: lines 112-113, the session join sentence
packs/journal/README.md                                          — modify: OpenCode paragraph after the init block
```

## Order of work

### Task 1 — Add each tool's session file

**Files:**
- Create `packs/journal/native/claude/skills/journal-capture/session.md` — byte for byte
  `.claudestacks/sdlc/2026-11-24-packs-tool-neutral/probes/final/claude-session.md`.
- Create `packs/journal/native/opencode/skills/journal-capture/session.md` — byte for byte
  `.claudestacks/sdlc/2026-11-24-packs-tool-neutral/probes/final/opencode-session.md`.
- Test `crates/agentstacks-cli/tests/packs.rs` — one new test.

**Change:** Copy the two files (`cp`, no retyping). The Claude file keeps `id8` as the first eight
characters and runs `transcript-path.lua` with today's grants (D5, E13); the OpenCode file takes
the last eight characters in lower case (D5, E12), runs `opencode service status`, handles exit
127 (E3) and `stopped` (E4) with the two user messages in the file, exports to a fresh `mktemp`
file only when the service runs (D6, E5, E8), and removes it on a failed export (E6). Neither file
contains the text `journal-capture`, so gate 3 holds.

**Proof:** written first, seen failing, then passing — `journal_session_lookup_is_per_tool` — after
plan 01's `init_both` (skip without airsl first), `.claude/skills/journal-capture/session.md`
equals the Claude rendered text and `.opencode/skills/journal-capture/session.md` the OpenCode
rendered text. The expected texts are the two files above with only these tokens rendered:
`{{root}}` → `.claude/agentstacks/journal` in the Claude file, and `{{var tmpdir}}` →
`${TMPDIR:-/tmp}` in the OpenCode file (spec F2; the rendered Claude text is shown in
`probes/raw/a8-final-session-rendered.txt`). The test reads the expected texts from
`packs/journal/native/<tool>/skills/journal-capture/session.md` in the copied project and applies
those two replacements, so the text is not copied into the test. Because that test passes for any
content, the byte-for-byte copy is proven separately:
`cmp .claudestacks/sdlc/2026-11-24-packs-tool-neutral/probes/final/claude-session.md packs/journal/native/claude/skills/journal-capture/session.md`
and the same `cmp` for `opencode-session.md` against the opencode file, both exit 0.

**Done when:** `cargo test -p agentstacks-cli --test packs` and `cargo make lua` pass and
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`feat(repo): add a session file per tool to the journal pack`.

### Task 2 — Make capture and note read the session file

**Files:** Modify `packs/journal/skills/capture/SKILL.md`, `packs/journal/skills/note/SKILL.md`.

**Change:**
- `capture/SKILL.md`: steps 1, 2, 5 and 6 and the `## If the agent is unavailable` section become
  the first fenced block of spec §5.1, verbatim. Steps 3 and 4 and the worktree-guard paragraph
  are unchanged. Today's step 2 (the `transcript-path.lua` block, with its `HOME_ROOT` line) is
  removed: that lookup now lives in the Claude `session.md`.
- `note/SKILL.md`: steps 2, 3, 6 and 7 and the `## If the agent is unavailable` section become the
  second fenced block of spec §5.1, verbatim. Steps 1, 4 and 5 and the worktree-guard paragraph
  are unchanged; today's transcript-path block goes the same way.
- The resulting behaviour: an empty session id skips `session.md` (E1); a missing `session.md`
  gives the first eight characters and empty `tpath`/`cleanup` (E2); the receipt is relayed when
  it arrives (E7); an agent that cannot be spawned makes the skill run `cleanup` itself (E9).

**Proof:**
- `grep -nE "$GATE4"` (the pattern of plan 02) over both skills exits 1 after the edit (today it
  prints capture 31, 64-65 and note 31, 63-64).
- `grep -c 'transcript-path.lua'` over both skills prints 0.
- `grep -nF '{{skill_dir journal/capture}}/session.md'` finds one line in each skill, and
  `grep -nF 'if it is set, and stop'` one in each (the only fragment of the unavailable-agent
  sentence that sits on one line in both verbatim blocks).
- `cargo make lua` gates 1-3 pass.

**Done when:** `cargo make lua` and `cargo test -p agentstacks-cli --test packs` pass and
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`feat(repo): resolve the journal session through the tool's session file`.

### Task 3 — Teach the journal agents the cleanup input

**Files:** Modify `packs/journal/agents/capture.md`, `packs/journal/agents/note.md`,
`packs/journal/references/note-schema.md`.

**Change:**

`capture.md`:

| Line | New text |
|---|---|
| 4-5 (description) | only `session transcript JSONL` → `session transcript` (rewrap within the folded description) |
| 19 | ``- `session_id` — the full session id.`` |
| 20 | ``- `id8` — the eight-character session key the skill took from `session.md`.`` |
| 22-23 | ``- `tpath` — absolute path to the session transcript, in the tool's own format (resolved by the skill through `session.md`), or empty if none was found.`` |
| new line after 23 | ``- `cleanup` — a shell command that removes a temporary transcript, or empty.`` |
| 28 | `read it (the full transcript)` replaces `read it (the full transcript JSONL)` |
| 63 (step 8) | ``8. Run `cleanup` once if it is set, whether or not you could read the transcript. Then return a one-line receipt, e.g.`` (the receipt example line is unchanged) |

`note.md`:

| Line | New text |
|---|---|
| 21 | ``- `session_id`, `id8` — the session id and its eight-character session key.`` |
| 22 | ``- `tpath` — absolute path to the session transcript, or empty if none.`` |
| new line after 22 | ``- `cleanup` — a shell command that removes a temporary transcript, or empty.`` |
| 62 (step 8) | ``8. Run `cleanup` once if it is set, whether or not you could read the transcript. Then return a one-line receipt, e.g.`` (the rest of the step is unchanged) |

`note-schema.md:112-113`: the clause after `where` becomes ``where `<id8>` is the eight-character
session key the capture and note skills take from `session.md` (the same `<id8>` used in the
`session-<id8>` story stem).`` The following sentence is unchanged.

The agents' `tools:` frontmatter does not change.

**Proof:**
- `grep -rnE "$GATE4"` over `packs/journal/agents/` exits 1 after the edit.
- `grep -n 'Claude' packs/journal/agents/{capture,note}.md packs/journal/references/note-schema.md`
  prints nothing after the edit.
- `grep -nF 'Run `cleanup` once if it is set'` finds one line in each agent.

**Done when:** `cargo make lua` passes and `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green.
Commit `feat(repo): journal agents run the transcript cleanup`.

### Task 4 — Document the journal on OpenCode

**Files:** Modify `packs/journal/README.md` — after the `agentstacks init` fenced block in
`## Install` (line 21).

**Change:** Insert, after a blank line, the paragraph of spec §9:
"On OpenCode, pass `--tool opencode` (or both tools). `capture` and `note` read the session
through `opencode session export`, which needs `opencode` on `PATH` and OpenCode's background
service running; otherwise they write without the transcript and say why." — wrapped at the
README's width, words unchanged.

**Proof:** `grep -n 'opencode session export' packs/journal/README.md` exits 1 before and finds the
line after; `cargo make book-build` prints no `[ERROR]` (`ch02-03` includes this README).

**Done when:** `cargo make book-build` passes with no `[ERROR]` and
`AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` is green. Commit
`docs(repo): describe the journal on OpenCode`.

## Risks

- A model may still read the neutral prose as an instruction for the other tool (spec K1); no test
  proves model behaviour.
- If an extension renamed skill directories, `session.md` would land elsewhere and E2 would apply
  silently on that tool; `journal_session_lookup_is_per_tool` catches it (K3).
- The export file stays in the temp directory when an agent stops before step 8 (E10, accepted).
- Riskiest step: Task 2 replaces numbered steps in two skills; a stray old step (the transcript
  block) would leave a Claude-only command in neutral text, which the Task 2 greps and plan 04's
  gate catch.
- Not done here: the gate (plan 04), the `journal` version and changelog (plan 05).

## Proof

- `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` green.
- `cargo make book-build` with no `[ERROR]`.
