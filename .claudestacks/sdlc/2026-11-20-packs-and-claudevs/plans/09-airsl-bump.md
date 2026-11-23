---
status: approved
created: 2026-11-23
depends-on: [01]
built-on: 0f89fdb3ebb3
---

# airsl bump Implementation Plan

**Goal:** Build, test and run CI on airsl 0.1.5.

**Architecture:**
- **Precondition, outside this chain:** airsl and airsl-cli 0.1.5 are published (crates.io and the
  GitHub release `airsl-cli-v0.1.5`) with airsstack/airsl#17: `require` passes the module name to
  the required chunk (spec §3). If `cargo search airsl --limit 2` does not show `airsl = "0.1.5"`
  and `airsl-cli = "0.1.5"`, this plan does not start.
- The library, the CI binary and its hash move together (the CI comment at
  `.github/workflows/ci.yml`, step "Install the airsl binary").
- claudevs was proven on 0.1.4 (spec §1). `cargo make dod` re-proves it on 0.1.5; if it fails, this
  plan fixes claudevs before it lands (spec §3).

**Tech Stack:** Cargo, GitHub Actions, cargo-deny.

**Content authority:** spec §1 (airsl rows), §3.

---

## Files that change

```
Cargo.toml                  — modify: [workspace.dependencies] airsl = "0.1.5"
Cargo.lock                  — modify: airsl 0.1.5
deny.toml                   — modify: skip-tree entry names airsl@0.1.5 (:81)
.github/workflows/ci.yml    — modify: AIRSL_VERSION 0.1.5 and its AIRSL_SHA256
```

## Order of work

### Task 1 — Move the library, the binary pin and the hash to 0.1.5

**Files:**
- Modify `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.github/workflows/ci.yml`.

**Change:**
- `Cargo.toml:15`: `airsl = "0.1.5"`; `cargo update -p airsl` updates the lock.
- `deny.toml:81`: the `skip-tree` entry's `crate` becomes `"airsl@0.1.5"`, and its reason names
  the `sha2` and `toml` versions that `cargo tree -p airsl@0.1.5 --depth 1 -e normal` prints. If
  0.1.5 pins no older `sha2` or `toml` than agentstacks, the entry is removed, together with the
  `[bans]` comment that describes it (`deny.toml:69-70`). `cargo make deny` is green either way
  (spec §3).
- `ci.yml`: `AIRSL_VERSION: 0.1.5` and `AIRSL_SHA256` set to the output of
  `curl -sSfL https://github.com/airsstack/airsl/releases/download/airsl-cli-v0.1.5/airsl-cli-x86_64-unknown-linux-gnu.tar.xz | shasum -a 256`
  (the first field). The comment above the step is unchanged.
- The local `airsl` binary is 0.1.5 (`cargo install --locked airsl-cli@0.1.5`); `airsl --version`
  prints it.

**Proof:**
- The existing runtime tests (`crates/agentstacks/tests/runtime.rs`) and the claude contract tests
  pass on the 0.1.5 binary with `AGENTSTACKS_REQUIRE_AIRSL=1`.
- claudevs' unit, integration and doc tests pass on the 0.1.5 library.
- `cargo tree -i airsl` shows only 0.1.5.

**Done when:** `AGENTSTACKS_REQUIRE_AIRSL=1 cargo make dod` and `cargo make deny` are green, and the
pushed CI `dod` job passes its `sha256sum -c` step. Commit `build(repo): airsl 0.1.5`.

---

## Risks

- If 0.1.5 moved a dependency pin after all, `cargo make deny` reports the duplicate. Fix
  `deny.toml` to name what it now pins; never silence the check.
- If the hash is copied wrong, the CI install step fails at `sha256sum -c`, before any test runs.

## Proof

- `cargo make dod`, `cargo make deny` and CI green on 0.1.5.
