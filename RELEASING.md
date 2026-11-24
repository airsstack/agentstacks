# Releasing agentstacks

The maintainer's runbook for the first release. Binaries release before crates.io, because dist
builds from the repository and every step before step 6 can be undone. Once a crate is published,
nothing can be taken back (see [Notes](#notes)).

This file stays at the repository root, outside the book: the book is for users and contributors,
and this is the maintainer's runbook.

## Tags

| Tag | Marks | Starts a release |
|---|---|---|
| `agentstacks-v<version>` | the `agentstacks` crate on crates.io | no |
| `agentstacks-cli-v<version>` | the `agentstacks` binary release | yes |
| `claudevs-v<version>` | the `claudevs` crate on crates.io | no |
| `claudevs-cli-v<version>` | the `claudevs` binary release | yes |
| `packs-v<version>` | the official packs and extensions that `init`/`update` resolve | no |

- Every tag is annotated (`git tag -a <tag> -m <tag>`) and pushed alone (`git push origin <tag>`),
  because GitHub creates no event when more than three tags are pushed at once.
- The first release tags the merged release commit with all five, in the order of the steps below.
- Later, each crate and the packs release on their own. A new `packs-vX.Y.Z` needs no binary
  release; `update` picks the newest stable `packs-v*` tag
  ([plan and apply](book/src/ch05-00-plan-and-apply.md)).

## Steps

1. Release pull request green: `cargo make dod`, `cargo make claudevs-check`,
   `cargo deny check`, the `installers` and `book` jobs, and the release workflow's plan job. In the pull
   request, replace every `Unreleased` in the five CHANGELOGs with the day you will run steps 3-6.
   Merge.
2. Settings → Pages → Source: GitHub Actions. Run `pages.yml` (workflow_dispatch). Check that
   `https://airsstack.github.io/agentstacks/`, `…/ch03-00-migration.html`, `…/install.sh` and
   `…/claudevs.sh` serve 200. The crate READMEs published at step 6 link into the book, so it is
   up before then.
3. On the merged commit, push `packs-v0.1.0`.
4. Push `agentstacks-cli-v0.1.0` and wait for its release workflow to finish green; then the same
   for `claudevs-cli-v0.1.2`. **If one fails**, nothing was published (`host` waits for smoke).
   Delete any GitHub Release it left, then delete the tag (`git tag -d <tag>`,
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

   `# prints M9` means the installer printed
   `agentstacks-install: agentstacks needs airsl, which is not installed; installing the latest airsl with https://airsstack.github.io/airsl/install.sh`.

   A failure here is fixed like step 4's: nothing is on crates.io yet.
6. Publish to crates.io, one crate at a time, from a clean, up-to-date `main`:
   `cargo publish --workspace --dry-run`, then ✋ `cargo publish -p agentstacks`,
   `cargo publish -p agentstacks-cli`, `cargo publish -p claudevs`, `cargo publish -p claudevs-cli`.
   Each command polls until its crate is in the index. On a timeout, confirm the version on
   crates.io before the next command. **If a publish fails**, the crates before it stay
   published, and you continue from the failed crate once the cause is fixed. A fix that changes
   that crate's code needs a new version: bump it, and for a `-cli` crate release that version
   again through step 4.
7. Push `agentstacks-v0.1.0` and `claudevs-v0.1.1`, each alone.
8. claudestacks: apply the [claudestacks changes](#claudestacks-changes) (README banner, book
   pointer, forwarder) on a branch, merge, and wait for its Pages deploy.
9. Check the forwarder: `( export HOME="$(mktemp -d)" PATH=/usr/bin:/bin; curl -LsSf https://airsstack.github.io/claudestacks/install.sh | sh && "$HOME/.local/bin/claudevs" --version )`
   prints `claudevs 0.1.2`.
10. Archive claudestacks: Settings → Archive, or `gh repo archive airsstack/claudestacks`. Never
    delete it: its releases, the marketplace and the Pages site stay reachable. airsclaude is
    frozen until it moves to airsdk; a fix needs an unarchive, a release and a re-archive.
11. Commit the org docs in the org root and push the `.github` repository.
12. `/claudestacks-sdlc:milestone foundation/01` to roll the milestone up to done.

## claudestacks changes

Applied by the author, or by a session opened in the claudestacks repository, on a branch, through
its own pull request. claudevs 0.1.1 is the last claudestacks release.

- `README.md`: a banner first, under the title:

  ```text
  > **Archived.** The Claude Code plugins are now agentstacks packs and claudevs lives in agentstacks:
  https://github.com/airsstack/agentstacks. Migrating:
  https://airsstack.github.io/agentstacks/ch03-00-migration.html. airsclaude stays
  here until it moves to airsdk.
  ```

- The book's claudevs part points to its new home. This blockquote goes under the `# Getting
  Started` title of `book/src/ch05-00-claudevs-getting-started.md`, and above line 53
  (`Start with [claudevs Getting Started](…)`) of `book/src/introduction.md`, each followed by a
  blank line:

  ```text
  > **claudevs moved to agentstacks.** Its book is now
  > https://airsstack.github.io/agentstacks/ch08-00-claudevs-getting-started.html. The claudevs
  > chapters here describe claudevs 0.1.1 and are no longer updated.
  ```

  Check before merging: `mdbook build book` exits 0 in claudestacks.
- `install/install.sh` becomes a forwarder, with the same prefix (`claudevs-install`) and helpers:
  - `CLAUDEVS_VERSION` unset, empty or `latest`: say
    `claudevs moved to https://github.com/airsstack/agentstacks; running https://airsstack.github.io/agentstacks/claudevs.sh`,
    download that script to the temp directory and run it with `sh <file> "$@"`. The environment,
    `CLAUDEVS_VERSION` included, passes through.
  - An explicit version: the current claudestacks download (`claudevs-cli-v<version>` from
    claudestacks releases, which has 0.1.0 and 0.1.1). If that download fails, forward as above.
  - A bad version: today's text, unchanged:
    `CLAUDEVS_VERSION='<value>' is not a version (expected something like 0.1.0)`.
  - The forwarded script fails to download:
    `claudevs-install: error: could not download https://airsstack.github.io/agentstacks/claudevs.sh`,
    exit 1.
- Check before merging: `sh -n install/install.sh`. After its Pages deploy: step 9's run.
  claudestacks' `pages.yml` deploys on `book/**` and `install/**`, so the one merge republishes
  both.
- It is merged and its Pages deploy has finished **before** the archive. An archived repository is
  read-only, so nothing can redeploy its Pages afterwards.

## Notes

- `cargo publish` is irreversible: a version can be yanked, never replaced. Only step 6 and later
  are past the point of undoing.
- The installers' release lookup reads one page of 100 releases, and the GitHub API states no
  order for them, so the newest could be missed. Revisit the lookup when this repository passes 100
  releases.
- A relative link added to a pack README breaks in the book only, and no CI check catches it.
  Links in pack READMEs are absolute URLs, because the book includes those files.
