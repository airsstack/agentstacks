# git probe results (git 2.39.3 (Apple Git-146), /usr/bin/git, macOS Darwin 25.3.0), 2026-11-21

## Config isolation: deviation from the brief

The brief asked for `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1` on every command. The Claude Code
harness refused both env vars, verbatim:

```text
This session is isolated in the worktree /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/m2-m3-milestones, but this command sets GIT_CONFIG_GLOBAL, injecting git configuration whose effect on where git writes can't be verified. Refusing to run it — a worktree-isolated session's git operations must target its own worktree. Run the equivalent from /Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/m2-m3-milestones without the redirect.
```

(same text with `GIT_CONFIG_NOSYSTEM` for the second.) So every probe ran with the ambient config, recorded
in full in the Environment section below: `credential.helper=osxkeychain` and `init.defaultbranch=main`
(Apple's built-in CommandLineTools gitconfig), and global `user.name`, `user.email`,
`core.excludesfile`. None of these affect ls-remote/fetch/ls-tree/cat-file output on file:// or anonymous
https, except that `credential.helper=osxkeychain` is consulted for github.com https auth (see G3).
`/etc/gitconfig` does not exist. `GIT_TERMINAL_PROMPT=0` was exported for every probe command.
`-c key=value` on the command line was accepted by the harness and is used where the brief asked.

## Source repo (setup.sh, verbatim)

```bash
#!/bin/bash
# Builds the source repo.
set -eu
W=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git
# (harness refuses the global-config redirect; run with the user config, which holds only user.*/core.excludesfile)
export GIT_AUTHOR_NAME=probe GIT_AUTHOR_EMAIL=probe@example.invalid
export GIT_COMMITTER_NAME=probe GIT_COMMITTER_EMAIL=probe@example.invalid
export GIT_AUTHOR_DATE='2026-01-01T00:00:00Z' GIT_COMMITTER_DATE='2026-01-01T00:00:00Z'
rm -rf "$W/src"
git init -q -b main "$W/src"
cd "$W/src"
# commit "zero": an untagged commit that no ref points at once later commits land (non-tip)
printf 'zero\n' > zero.txt
git add zero.txt
git commit -q -m zero
mkdir -p packs/core/nested/deeper
printf 'name = "core"\nversion = "0.1.0"\n' > packs/core/pack.toml
printf 'a\r\nb\r\n' > packs/core/crlf.md
printf 'a\nb\n' > packs/core/lf.md
printf '* text=auto eol=crlf\n' > .gitattributes
ln -s pack.toml packs/core/link
printf '#!/bin/sh\necho hi\n' > packs/core/run.sh
chmod +x packs/core/run.sh
printf 'deep\n' > packs/core/nested/deeper/file.md
printf 'hidden\n' > packs/core/.hidden
printf 'a\r\nb\r\n' > packs/core/crlf-added.md
# ordering probes: siblings of dir "nested" that sort around "nested/"
printf 'x\n' > packs/core/nested.md
printf 'x\n' > packs/core/nested-x.md
printf 'x\n' > packs/core/nested0.md
printf 'x\n' > packs/core/Z.md
printf 'x\n' > "packs/core/sp ace.md"
printf 'x\n' > "packs/core/$(printf '\303\274').md"
git add -A
# crlf.md: store the CRLF bytes verbatim (no filters), bypassing text=auto normalisation
RAW=$(git hash-object -w --no-filters packs/core/crlf.md)
git update-index --add --cacheinfo 100644,$RAW,packs/core/crlf.md
git update-index --add --cacheinfo 160000,0123456789abcdef0123456789abcdef01234567,packs/core/sub
GIT_COMMITTER_DATE='2026-01-01T00:00:01Z' GIT_AUTHOR_DATE='2026-01-01T00:00:01Z' git commit -q -m one
GIT_COMMITTER_DATE='2026-01-01T00:00:01Z' git tag -a packs-v0.1.0 -m 'annotated 0.1.0'
printf 'two\n' > packs/core/two.md
git add packs/core/two.md
GIT_COMMITTER_DATE='2026-01-02T00:00:00Z' GIT_AUTHOR_DATE='2026-01-02T00:00:00Z' git commit -q -m two
git tag packs-v0.2.0
git tag packs-v0.9.0
git tag packs-v0.10.0
git tag other-v9.9.9
echo "setup done"
```

Note on CRLF: with `* text=auto eol=crlf`, a plain `git add` of a CRLF file NORMALISES it to LF in the
blob (`crlf-added.md` -> blob 422c2b7 == the LF blob of `lf.md`). To get a blob whose committed bytes really
are CRLF, `crlf.md` was stored with `git hash-object -w --no-filters` + `update-index --cacheinfo`
(blob c30dea8, `i/crlf` in `ls-files --eol`).

Commits: zero `a11c81c8fc825821c34b58fa42ce941ef328a9dc` (untagged, non-tip),
one `7eb036fe4d54e8c8cae19243e7e01e0fac4fac80` (annotated `packs-v0.1.0`, tag object `aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd`),
two `90e2a535db371753890122ed088c81bf3867be7a` (main; lightweight `packs-v0.2.0`, `packs-v0.9.0`, `packs-v0.10.0`, `other-v9.9.9`).

Each block below is: the exact command (`$ ...`, run via `eval` from the probe-git dir), its stdout, its
stderr, and its exit code, captured byte for byte (probes.sh / probes2.sh / probes3.sh in the same dir).


## Environment

```text
$ git --version
--- stdout ---
git version 2.39.3 (Apple Git-146)
--- stderr ---
[exit 0]
```

```text
$ command -v git
--- stdout ---
/usr/bin/git
--- stderr ---
[exit 0]
```

```text
$ git config --list --show-scope --show-origin
--- stdout ---
unknown	file:/Library/Developer/CommandLineTools/usr/share/git-core/gitconfig	credential.helper=osxkeychain
unknown	file:/Library/Developer/CommandLineTools/usr/share/git-core/gitconfig	init.defaultbranch=main
global	file:/Users/hiraq/.gitconfig	user.name=<redacted>
global	file:/Users/hiraq/.gitconfig	user.email=<redacted>
global	file:/Users/hiraq/.gitconfig	core.excludesfile=/Users/hiraq/.gitignore_global
--- stderr ---
[exit 0]
```

URL=file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src ; cache dir C=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache ; GIT_TERMINAL_PROMPT=0 exported for every command.

```text
$ git -C src log --format="%H %s %D"
--- stdout ---
90e2a535db371753890122ed088c81bf3867be7a two HEAD -> main, tag: packs-v0.9.0, tag: packs-v0.2.0, tag: packs-v0.10.0, tag: other-v9.9.9
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80 one tag: packs-v0.1.0
a11c81c8fc825821c34b58fa42ce941ef328a9dc zero 
--- stderr ---
[exit 0]
```

```text
$ git -C src rev-parse packs-v0.1.0 "packs-v0.1.0^{commit}" packs-v0.2.0
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
90e2a535db371753890122ed088c81bf3867be7a
--- stderr ---
[exit 0]
```


## G1 ls-remote

```text
$ git ls-remote --tags --refs -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 'refs/tags/packs-v*'
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd	refs/tags/packs-v0.1.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.10.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.2.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.9.0
--- stderr ---
[exit 0]
```

```text
$ git ls-remote --tags -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 'refs/tags/packs-v*'
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd	refs/tags/packs-v0.1.0
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80	refs/tags/packs-v0.1.0^{}
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.10.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.2.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.9.0
--- stderr ---
[exit 0]
```

```text
$ git ls-remote --tags --refs -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src'
--- stdout ---
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/other-v9.9.9
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd	refs/tags/packs-v0.1.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.10.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.2.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.9.0
--- stderr ---
[exit 0]
```

```text
$ git ls-remote --tags --refs -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 'packs-v*'
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd	refs/tags/packs-v0.1.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.10.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.2.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.9.0
--- stderr ---
[exit 0]
```

```text
$ git ls-remote --tags --refs --sort=v:refname -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 'refs/tags/packs-v*'
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd	refs/tags/packs-v0.1.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.2.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.9.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.10.0
--- stderr ---
[exit 0]
```

```text
$ git ls-remote --tags --refs -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 'refs/tags/packs-v0.1.0'
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd	refs/tags/packs-v0.1.0
--- stderr ---
[exit 0]
```

```text
$ git ls-remote --tags --refs -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 'refs/tags/packs-v9*'
--- stdout ---
--- stderr ---
[exit 0]
```

```text
$ git ls-remote --exit-code --tags --refs -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 'refs/tags/packs-v9*'
--- stdout ---
--- stderr ---
[exit 2]
```

```text
$ git ls-remote -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src'
--- stdout ---
90e2a535db371753890122ed088c81bf3867be7a	HEAD
90e2a535db371753890122ed088c81bf3867be7a	refs/heads/main
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/other-v9.9.9
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd	refs/tags/packs-v0.1.0
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80	refs/tags/packs-v0.1.0^{}
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.10.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.2.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.9.0
--- stderr ---
[exit 0]
```

```text
$ git ls-remote --tags --refs -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 'refs/tags/packs-v*' | od -c | head -4
--- stdout ---
0000000    a   e   c   9   2   b   4   5   9   6   a   e   b   d   6   f
0000020    b   c   0   d   2   a   b   9   2   5   3   2   b   b   e   3
0000040    c   6   e   0   b   d   c   d  \t   r   e   f   s   /   t   a
0000060    g   s   /   p   a   c   k   s   -   v   0   .   1   .   0  \n
--- stderr ---
[exit 0]
```


## G2 fetch tag into bare cache

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git'
--- stdout ---
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' refs/tags/packs-v0.1.0:refs/tags/packs-v0.1.0
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
 * [new tag]         packs-v0.1.0 -> packs-v0.1.0
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' rev-parse packs-v0.1.0
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' rev-parse 'packs-v0.1.0^{commit}'
--- stdout ---
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' rev-parse --verify --end-of-options 'refs/tags/packs-v0.1.0^{commit}'
--- stdout ---
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file -t packs-v0.1.0
--- stdout ---
tag
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file -p packs-v0.1.0
--- stdout ---
object 7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
type commit
tag packs-v0.1.0
tagger probe <probe@example.invalid> 1767225601 +0000

annotated 0.1.0
--- stderr ---
[exit 0]
```

```text
$ cat '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git/shallow'
--- stdout ---
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
--- stderr ---
[exit 0]
```

```text
$ cat '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git/FETCH_HEAD'
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd		tag 'packs-v0.1.0' of file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' for-each-ref
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd tag	refs/tags/packs-v0.1.0
--- stderr ---
[exit 0]
```

Re-fetch the same tag (idempotent?):

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' refs/tags/packs-v0.1.0:refs/tags/packs-v0.1.0
--- stdout ---
--- stderr ---
[exit 0]
```

Lightweight tag:

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' refs/tags/packs-v0.2.0:refs/tags/packs-v0.2.0
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
 * [new tag]         packs-v0.2.0 -> packs-v0.2.0
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' rev-parse packs-v0.2.0 'packs-v0.2.0^{commit}'
--- stdout ---
90e2a535db371753890122ed088c81bf3867be7a
90e2a535db371753890122ed088c81bf3867be7a
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file -t packs-v0.2.0
--- stdout ---
commit
--- stderr ---
[exit 0]
```

```text
$ cat '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git/shallow'
--- stdout ---
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
90e2a535db371753890122ed088c81bf3867be7a
--- stderr ---
[exit 0]
```

Variant: short tag name as refspec source, fresh cache:

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/y.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/y.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' packs-v0.1.0:refs/tags/packs-v0.1.0
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
 * [new tag]         packs-v0.1.0 -> packs-v0.1.0
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/y.git' rev-parse packs-v0.1.0 'packs-v0.1.0^{commit}'
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
--- stderr ---
[exit 0]
```

Variant: forced refspec (+), fresh cache:

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/z.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/z.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' +refs/tags/packs-v0.1.0:refs/tags/packs-v0.1.0
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
 * [new tag]         packs-v0.1.0 -> packs-v0.1.0
[exit 0]
```

Variant: no destination (FETCH_HEAD only):

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/w.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/w.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' refs/tags/packs-v0.1.0
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
 * tag               packs-v0.1.0 -> FETCH_HEAD
[exit 0]
```

```text
$ cat '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/w.git/FETCH_HEAD'
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd		tag 'packs-v0.1.0' of file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/w.git' for-each-ref
--- stdout ---
--- stderr ---
[exit 0]
```

Variant: without --no-tags (does it pull other tags?):

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/v.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/v.git' fetch --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' refs/tags/packs-v0.2.0:refs/tags/packs-v0.2.0
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
 * [new tag]         packs-v0.2.0  -> packs-v0.2.0
 * [new tag]         other-v9.9.9  -> other-v9.9.9
 * [new tag]         packs-v0.10.0 -> packs-v0.10.0
 * [new tag]         packs-v0.9.0  -> packs-v0.9.0
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/v.git' for-each-ref
--- stdout ---
90e2a535db371753890122ed088c81bf3867be7a commit	refs/tags/other-v9.9.9
90e2a535db371753890122ed088c81bf3867be7a commit	refs/tags/packs-v0.10.0
90e2a535db371753890122ed088c81bf3867be7a commit	refs/tags/packs-v0.2.0
90e2a535db371753890122ed088c81bf3867be7a commit	refs/tags/packs-v0.9.0
--- stderr ---
[exit 0]
```

Remote tag moved: copy src to src2, fetch packs-v0.2.0, re-point it in src2, re-fetch without and with +:

```text
$ cp -R src src2 && git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/m.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/m.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src2' refs/tags/packs-v0.2.0:refs/tags/packs-v0.2.0
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src2
 * [new tag]         packs-v0.2.0 -> packs-v0.2.0
[exit 0]
```

```text
$ git -C src2 tag -f packs-v0.2.0 HEAD~1
--- stdout ---
Updated tag 'packs-v0.2.0' (was 90e2a53)
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/m.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src2' refs/tags/packs-v0.2.0:refs/tags/packs-v0.2.0
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src2
 ! [rejected]        packs-v0.2.0 -> packs-v0.2.0  (would clobber existing tag)
[exit 1]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/m.git' rev-parse packs-v0.2.0
--- stdout ---
90e2a535db371753890122ed088c81bf3867be7a
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/m.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src2' +refs/tags/packs-v0.2.0:refs/tags/packs-v0.2.0
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src2
 t [tag update]      packs-v0.2.0 -> packs-v0.2.0
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/m.git' rev-parse packs-v0.2.0
--- stdout ---
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
--- stderr ---
[exit 0]
```


## G3 fetch by commit id

TIP=90e2a535db371753890122ed088c81bf3867be7a (main + lightweight tags) ; TAGGED=7eb036fe4d54e8c8cae19243e7e01e0fac4fac80 (peeled target of annotated packs-v0.1.0) ; NONTIP=a11c81c8fc825821c34b58fa42ce941ef328a9dc (no ref points at it)

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c1.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c1.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 90e2a535db371753890122ed088c81bf3867be7a
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
 * branch            90e2a535db371753890122ed088c81bf3867be7a -> FETCH_HEAD
[exit 0]
```

```text
$ cat '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c1.git/FETCH_HEAD'; git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c1.git' cat-file -t 90e2a535db371753890122ed088c81bf3867be7a
--- stdout ---
90e2a535db371753890122ed088c81bf3867be7a		'90e2a535db371753890122ed088c81bf3867be7a' of file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
commit
--- stderr ---
[exit 0]
```

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c2.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c2.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
 * branch            7eb036fe4d54e8c8cae19243e7e01e0fac4fac80 -> FETCH_HEAD
[exit 0]
```

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c3.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c3.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' a11c81c8fc825821c34b58fa42ce941ef328a9dc
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
 * branch            a11c81c8fc825821c34b58fa42ce941ef328a9dc -> FETCH_HEAD
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c3.git' cat-file -t a11c81c8fc825821c34b58fa42ce941ef328a9dc
--- stdout ---
commit
--- stderr ---
[exit 0]
```

Same three under protocol v0 (client -c protocol.version=0):

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/d1.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/d1.git' -c protocol.version=0 fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 90e2a535db371753890122ed088c81bf3867be7a
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src
 * branch            90e2a535db371753890122ed088c81bf3867be7a -> FETCH_HEAD
[exit 0]
```

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/d2.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/d2.git' -c protocol.version=0 fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
--- stdout ---
--- stderr ---
error: Server does not allow request for unadvertised object 7eb036fe4d54e8c8cae19243e7e01e0fac4fac80
[exit 1]
```

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/d3.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/d3.git' -c protocol.version=0 fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' a11c81c8fc825821c34b58fa42ce941ef328a9dc
--- stdout ---
--- stderr ---
error: Server does not allow request for unadvertised object a11c81c8fc825821c34b58fa42ce941ef328a9dc
[exit 1]
```

v0 non-tip with server-side uploadpack.allowReachableSHA1InWant=true (set in src2's config, the server repo):

```text
$ git -C src2 config uploadpack.allowReachableSHA1InWant true && git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/d4.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/d4.git' -c protocol.version=0 fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src2' a11c81c8fc825821c34b58fa42ce941ef328a9dc
--- stdout ---
--- stderr ---
From file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src2
 * branch            a11c81c8fc825821c34b58fa42ce941ef328a9dc -> FETCH_HEAD
[exit 0]
```

```text
$ git -C src2 config --unset uploadpack.allowReachableSHA1InWant
--- stdout ---
--- stderr ---
[exit 0]
```

Commit id that does not exist on the server:

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c5.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c5.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 0123456789abcdef0123456789abcdef01234567
--- stdout ---
--- stderr ---
fatal: git upload-pack: not our ref 0123456789abcdef0123456789abcdef01234567
fatal: remote error: upload-pack: not our ref 0123456789abcdef0123456789abcdef01234567
[exit 128]
```

Abbreviated commit id:

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c6.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/c6.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' 90e2a535db37
--- stdout ---
--- stderr ---
fatal: couldn't find remote ref 90e2a535db37
[exit 128]
```


## G3 GitHub (network)

```text
$ git ls-remote -- 'https://github.com/airsstack/agentstacks' HEAD
--- stdout ---
--- stderr ---
fatal: could not read Username for 'https://github.com': terminal prompts disabled
[exit 128]
```

GHSHA=

Public repo for comparison (airsstack/airsl):

```text
$ git ls-remote --tags --refs -- 'https://github.com/airsstack/airsl' | head -5
--- stdout ---
31f1ca929d72436dcea9681df23e532c5073823d	refs/tags/airsl-cli-v0.1.0
9e07c1c69627afbd99ce325e31a25ee062f4d687	refs/tags/airsl-cli-v0.1.1
7a42dbbc027a91f4149ec532cb06c21018812c3b	refs/tags/airsl-cli-v0.1.2
c9faef9750f865a1ed272e2bc43baf65bcab95a9	refs/tags/airsl-cli-v0.1.3
1edd8895d2736d152ee0e7c798ace278832ebbf2	refs/tags/airsl-cli-v0.1.4
--- stderr ---
[exit 0]
```

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/gh/e.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/gh/e.git' fetch --no-tags --depth 2 -- 'https://github.com/airsstack/airsl' 1a926b3ca5c1596a07dcd2da2be5abd916f3bd6e && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/gh/e.git' rev-parse 1a926b3ca5c1596a07dcd2da2be5abd916f3bd6e~1
--- stdout ---
d17a1fb256289a691972760ac4be221ec1617bd1
--- stderr ---
From https://github.com/airsstack/airsl
 * branch            1a926b3ca5c1596a07dcd2da2be5abd916f3bd6e -> FETCH_HEAD
[exit 0]
```

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/gh/f.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/gh/f.git' fetch --no-tags --depth 1 -- 'https://github.com/airsstack/airsl' d17a1fb256289a691972760ac4be221ec1617bd1
--- stdout ---
--- stderr ---
From https://github.com/airsstack/airsl
 * branch            d17a1fb256289a691972760ac4be221ec1617bd1 -> FETCH_HEAD
[exit 0]
```


## G4 ls-tree

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree packs-v0.1.0 -- packs/core | tr '\0' '\n'
--- stdout ---
100644 blob 136c05e0d0290e2dc7eb89bceaa27b5f011e780a	packs/core/.hidden
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/Z.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c	packs/core/crlf-added.md
100644 blob c30dea8a3641ea99b125d04d599d843712292759	packs/core/crlf.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c	packs/core/lf.md
120000 blob 71fc9eaf3e36f372a2b419e27c40cd814aabe3d9	packs/core/link
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested-x.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested.md
100644 blob 4cdb2265d30204be5463b38174b2e8e717982405	packs/core/nested/deeper/file.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested0.md
100644 blob d0a85717332d884c11c371725ba542b28ef47106	packs/core/pack.toml
100755 blob 4163036efa65bd4a469e752267498f01ea36a55c	packs/core/run.sh
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/sp ace.md
160000 commit 0123456789abcdef0123456789abcdef01234567	packs/core/sub
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/ü.md
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree packs-v0.1.0 -- packs/core | od -c | head -8
--- stdout ---
0000000    1   0   0   6   4   4       b   l   o   b       1   3   6   c
0000020    0   5   e   0   d   0   2   9   0   e   2   d   c   7   e   b
0000040    8   9   b   c   e   a   a   2   7   b   5   f   0   1   1   e
0000060    7   8   0   a  \t   p   a   c   k   s   /   c   o   r   e   /
0000100    .   h   i   d   d   e   n  \0   1   0   0   6   4   4       b
0000120    l   o   b       5   8   7   b   e   6   b   4   c   3   f   9
0000140    3   f   9   3   c   4   8   9   c   0   1   1   1   b   b   a
0000160    5   5   9   6   1   4   7   a   2   6   c   b  \t   p   a   c
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z -l --full-tree packs-v0.1.0 -- packs/core | tr '\0' '\n'
--- stdout ---
100644 blob 136c05e0d0290e2dc7eb89bceaa27b5f011e780a       7	packs/core/.hidden
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb       2	packs/core/Z.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c       4	packs/core/crlf-added.md
100644 blob c30dea8a3641ea99b125d04d599d843712292759       6	packs/core/crlf.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c       4	packs/core/lf.md
120000 blob 71fc9eaf3e36f372a2b419e27c40cd814aabe3d9       9	packs/core/link
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb       2	packs/core/nested-x.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb       2	packs/core/nested.md
100644 blob 4cdb2265d30204be5463b38174b2e8e717982405       5	packs/core/nested/deeper/file.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb       2	packs/core/nested0.md
100644 blob d0a85717332d884c11c371725ba542b28ef47106      32	packs/core/pack.toml
100755 blob 4163036efa65bd4a469e752267498f01ea36a55c      18	packs/core/run.sh
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb       2	packs/core/sp ace.md
160000 commit 0123456789abcdef0123456789abcdef01234567       -	packs/core/sub
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb       2	packs/core/ü.md
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r --full-tree packs-v0.1.0 -- packs/core
--- stdout ---
100644 blob 136c05e0d0290e2dc7eb89bceaa27b5f011e780a	packs/core/.hidden
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/Z.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c	packs/core/crlf-added.md
100644 blob c30dea8a3641ea99b125d04d599d843712292759	packs/core/crlf.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c	packs/core/lf.md
120000 blob 71fc9eaf3e36f372a2b419e27c40cd814aabe3d9	packs/core/link
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested-x.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested.md
100644 blob 4cdb2265d30204be5463b38174b2e8e717982405	packs/core/nested/deeper/file.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested0.md
100644 blob d0a85717332d884c11c371725ba542b28ef47106	packs/core/pack.toml
100755 blob 4163036efa65bd4a469e752267498f01ea36a55c	packs/core/run.sh
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/sp ace.md
160000 commit 0123456789abcdef0123456789abcdef01234567	packs/core/sub
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	"packs/core/\303\274.md"
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree packs-v0.1.0 -- packs/core/ | tr '\0' '\n'
--- stdout ---
100644 blob 136c05e0d0290e2dc7eb89bceaa27b5f011e780a	packs/core/.hidden
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/Z.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c	packs/core/crlf-added.md
100644 blob c30dea8a3641ea99b125d04d599d843712292759	packs/core/crlf.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c	packs/core/lf.md
120000 blob 71fc9eaf3e36f372a2b419e27c40cd814aabe3d9	packs/core/link
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested-x.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested.md
100644 blob 4cdb2265d30204be5463b38174b2e8e717982405	packs/core/nested/deeper/file.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested0.md
100644 blob d0a85717332d884c11c371725ba542b28ef47106	packs/core/pack.toml
100755 blob 4163036efa65bd4a469e752267498f01ea36a55c	packs/core/run.sh
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/sp ace.md
160000 commit 0123456789abcdef0123456789abcdef01234567	packs/core/sub
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/ü.md
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -z --full-tree packs-v0.1.0 -- packs/core | tr '\0' '\n'
--- stdout ---
040000 tree 3519f5a9cbcb8b44e70ffa08716fa5711a7c9ec9	packs/core
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -z --full-tree packs-v0.1.0 -- packs/core/ | tr '\0' '\n'
--- stdout ---
100644 blob 136c05e0d0290e2dc7eb89bceaa27b5f011e780a	packs/core/.hidden
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/Z.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c	packs/core/crlf-added.md
100644 blob c30dea8a3641ea99b125d04d599d843712292759	packs/core/crlf.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c	packs/core/lf.md
120000 blob 71fc9eaf3e36f372a2b419e27c40cd814aabe3d9	packs/core/link
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested-x.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested.md
040000 tree 9c09dc9dfc91c5b17e6b9b3f0b1fd48bd82946d5	packs/core/nested
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/nested0.md
100644 blob d0a85717332d884c11c371725ba542b28ef47106	packs/core/pack.toml
100755 blob 4163036efa65bd4a469e752267498f01ea36a55c	packs/core/run.sh
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/sp ace.md
160000 commit 0123456789abcdef0123456789abcdef01234567	packs/core/sub
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/ü.md
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree 'packs-v0.1.0^{tree}' -- packs/core | tr '\0' '\n' | head -3
--- stdout ---
100644 blob 136c05e0d0290e2dc7eb89bceaa27b5f011e780a	packs/core/.hidden
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb	packs/core/Z.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c	packs/core/crlf-added.md
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --format='%(objectmode) %(objecttype) %(objectname) %(objectsize)%x09%(path)' packs-v0.1.0 -- packs/core | tr '\0' '\n'
--- stdout ---
100644 blob 136c05e0d0290e2dc7eb89bceaa27b5f011e780a 7	packs/core/.hidden
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb 2	packs/core/Z.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c 4	packs/core/crlf-added.md
100644 blob c30dea8a3641ea99b125d04d599d843712292759 6	packs/core/crlf.md
100644 blob 422c2b7ab3b3c668038da977e4e93a5fc623169c 4	packs/core/lf.md
120000 blob 71fc9eaf3e36f372a2b419e27c40cd814aabe3d9 9	packs/core/link
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb 2	packs/core/nested-x.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb 2	packs/core/nested.md
100644 blob 4cdb2265d30204be5463b38174b2e8e717982405 5	packs/core/nested/deeper/file.md
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb 2	packs/core/nested0.md
100644 blob d0a85717332d884c11c371725ba542b28ef47106 32	packs/core/pack.toml
100755 blob 4163036efa65bd4a469e752267498f01ea36a55c 18	packs/core/run.sh
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb 2	packs/core/sp ace.md
160000 commit 0123456789abcdef0123456789abcdef01234567 -	packs/core/sub
100644 blob 587be6b4c3f93f93c489c0111bba5596147a26cb 2	"packs/core/\303\274.md"
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree packs-v0.1.0 -- packs/nope | od -c
--- stdout ---
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree packs-v0.1.0 -- packs/core/pack.toml | tr '\0' '\n'
--- stdout ---
100644 blob d0a85717332d884c11c371725ba542b28ef47106	packs/core/pack.toml
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree 0123456789abcdef0123456789abcdef01234567 -- packs/core
--- stdout ---
--- stderr ---
fatal: not a tree object
[exit 128]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree packs-v7.7.7 -- packs/core
--- stdout ---
--- stderr ---
fatal: Not a valid object name packs-v7.7.7
[exit 128]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree a11c81c8fc825821c34b58fa42ce941ef328a9dc -- packs/core
--- stdout ---
--- stderr ---
fatal: not a tree object
[exit 128]
```

Shallow boundary: the depth-1 cache holds only the tagged commit; its parent is absent:

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r 'packs-v0.1.0^{commit}~1'
--- stdout ---
--- stderr ---
fatal: Not a valid object name packs-v0.1.0^{commit}~1
[exit 128]
```


## G5 cat-file --batch

CRLF=c30dea8a3641ea99b125d04d599d843712292759 LF=422c2b7ab3b3c668038da977e4e93a5fc623169c ADDED=422c2b7ab3b3c668038da977e4e93a5fc623169c LINK=71fc9eaf3e36f372a2b419e27c40cd814aabe3d9

```text
$ printf '%s\n' c30dea8a3641ea99b125d04d599d843712292759 | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file --batch | od -c
--- stdout ---
0000000    c   3   0   d   e   a   8   a   3   6   4   1   e   a   9   9
0000020    b   1   2   5   d   0   4   d   5   9   9   d   8   4   3   7
0000040    1   2   2   9   2   7   5   9       b   l   o   b       6  \n
0000060    a  \r  \n   b  \r  \n  \n                                    
0000067
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n' c30dea8a3641ea99b125d04d599d843712292759 | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' -c core.autocrlf=true cat-file --batch | od -c
--- stdout ---
0000000    c   3   0   d   e   a   8   a   3   6   4   1   e   a   9   9
0000020    b   1   2   5   d   0   4   d   5   9   9   d   8   4   3   7
0000040    1   2   2   9   2   7   5   9       b   l   o   b       6  \n
0000060    a  \r  \n   b  \r  \n  \n                                    
0000067
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n' c30dea8a3641ea99b125d04d599d843712292759 | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' -c core.autocrlf=true -c core.eol=crlf cat-file --batch | od -c
--- stdout ---
0000000    c   3   0   d   e   a   8   a   3   6   4   1   e   a   9   9
0000020    b   1   2   5   d   0   4   d   5   9   9   d   8   4   3   7
0000040    1   2   2   9   2   7   5   9       b   l   o   b       6  \n
0000060    a  \r  \n   b  \r  \n  \n                                    
0000067
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n' 422c2b7ab3b3c668038da977e4e93a5fc623169c | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' -c core.autocrlf=true cat-file --batch | od -c
--- stdout ---
0000000    4   2   2   c   2   b   7   a   b   3   b   3   c   6   6   8
0000020    0   3   8   d   a   9   7   7   e   4   e   9   3   a   5   f
0000040    c   6   2   3   1   6   9   c       b   l   o   b       4  \n
0000060    a  \n   b  \n  \n                                            
0000065
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n' 422c2b7ab3b3c668038da977e4e93a5fc623169c | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file --batch | od -c
--- stdout ---
0000000    4   2   2   c   2   b   7   a   b   3   b   3   c   6   6   8
0000020    0   3   8   d   a   9   7   7   e   4   e   9   3   a   5   f
0000040    c   6   2   3   1   6   9   c       b   l   o   b       4  \n
0000060    a  \n   b  \n  \n                                            
0000065
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n' 71fc9eaf3e36f372a2b419e27c40cd814aabe3d9 | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file --batch | od -c
--- stdout ---
0000000    7   1   f   c   9   e   a   f   3   e   3   6   f   3   7   2
0000020    a   2   b   4   1   9   e   2   7   c   4   0   c   d   8   1
0000040    4   a   a   b   e   3   d   9       b   l   o   b       9  \n
0000060    p   a   c   k   .   t   o   m   l  \n                        
0000072
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n%s\n%s\n%s\n' 422c2b7ab3b3c668038da977e4e93a5fc623169c 0123456789abcdef0123456789abcdef01234567 c30dea8a3641ea99b125d04d599d843712292759 nonsense | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file --batch
--- stdout ---
422c2b7ab3b3c668038da977e4e93a5fc623169c blob 4
a
b

0123456789abcdef0123456789abcdef01234567 missing
c30dea8a3641ea99b125d04d599d843712292759 blob 6
a
b

nonsense missing
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n%s\n' 0123456789abcdef0123456789abcdef01234567 c30dea8a3641ea99b125d04d599d843712292759 | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file --batch | od -c
--- stdout ---
0000000    0   1   2   3   4   5   6   7   8   9   a   b   c   d   e   f
*
0000040    0   1   2   3   4   5   6   7       m   i   s   s   i   n   g
0000060   \n   c   3   0   d   e   a   8   a   3   6   4   1   e   a   9
0000100    9   b   1   2   5   d   0   4   d   5   9   9   d   8   4   3
0000120    7   1   2   2   9   2   7   5   9       b   l   o   b       6
0000140   \n   a  \r  \n   b  \r  \n  \n                                
0000150
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n' c30dea8a3641ea99b125d04d599d843712292759 0123456789abcdef0123456789abcdef01234567 | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file --batch-check
--- stdout ---
c30dea8a3641ea99b125d04d599d843712292759 blob 6
0123456789abcdef0123456789abcdef01234567 missing
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n' 'packs-v0.1.0:packs/core/crlf.md' | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file --batch | od -c
--- stdout ---
0000000    c   3   0   d   e   a   8   a   3   6   4   1   e   a   9   9
0000020    b   1   2   5   d   0   4   d   5   9   9   d   8   4   3   7
0000040    1   2   2   9   2   7   5   9       b   l   o   b       6  \n
0000060    a  \r  \n   b  \r  \n  \n                                    
0000067
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n' $(git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' rev-parse 'packs-v0.1.0^{tree}') | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file --batch-check
--- stdout ---
3dcf9dbe9e1b39c585a3724af8babe8fda09f2bd tree 110
--- stderr ---
[exit 0]
```

```text
$ printf '%s\n' packs-v0.1.0 | git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' cat-file --batch-check
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd tag 148
--- stderr ---
[exit 0]
```

Working-tree checkout for comparison (clone with -c core.autocrlf=true; the repo's .gitattributes says * text=auto eol=crlf):

```text
$ git -c core.autocrlf=true clone -q --branch packs-v0.1.0 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' clone-acrlf
--- stdout ---
--- stderr ---
Note: switching to '7eb036fe4d54e8c8cae19243e7e01e0fac4fac80'.

You are in 'detached HEAD' state. You can look around, make experimental
changes and commit them, and you can discard any commits you make in this
state without impacting any branches by switching back to a branch.

If you want to create a new branch to retain commits you create, you may
do so (now or later) by using -c with the switch command. Example:

  git switch -c <new-branch-name>

Or undo this operation with:

  git switch -

Turn off this advice by setting config variable advice.detachedHead to false

[exit 0]
```

```text
$ od -c clone-acrlf/packs/core/crlf.md; od -c clone-acrlf/packs/core/lf.md; od -c clone-acrlf/packs/core/crlf-added.md
--- stdout ---
0000000    a  \r  \n   b  \r  \n                                        
0000006
0000000    a  \r  \n   b  \r  \n                                        
0000006
0000000    a  \r  \n   b  \r  \n                                        
0000006
--- stderr ---
[exit 0]
```

```text
$ git -C clone-acrlf ls-files --eol -- packs/core/crlf.md packs/core/lf.md packs/core/crlf-added.md
--- stdout ---
i/lf    w/crlf  attr/text=auto eol=crlf	packs/core/crlf-added.md
i/crlf  w/crlf  attr/text=auto eol=crlf	packs/core/crlf.md
i/lf    w/crlf  attr/text=auto eol=crlf	packs/core/lf.md
--- stderr ---
[exit 0]
```

Contrast: cat-file --filters (applies checkout conversion; requires --path):

```text
$ git -C clone-acrlf cat-file --filters --path=packs/core/lf.md 422c2b7ab3b3c668038da977e4e93a5fc623169c | od -c
--- stdout ---
0000000    a  \r  \n   b  \r  \n                                        
0000006
--- stderr ---
[exit 0]
```


## G6 URL starting with -

```text
$ git ls-remote -- '-oops'
--- stdout ---
--- stderr ---
fatal: strange pathname '-oops' blocked
[exit 128]
```

```text
$ git ls-remote '-oops'
--- stdout ---
--- stderr ---
fatal: No remote configured to list refs from.
[exit 128]
```

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/u.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/u.git' fetch --no-tags --depth 1 -- '-oops' refs/tags/x:refs/tags/x
--- stdout ---
--- stderr ---
fatal: strange pathname '-oops' blocked
[exit 128]
```

```text
$ git ls-remote -- '--upload-pack=touch /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/PWNED'; ls '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/PWNED' 2>&1
--- stdout ---
ls: /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/PWNED: No such file or directory
--- stderr ---
fatal: strange pathname '--upload-pack=touch /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/PWNED' blocked
[exit 1]
```

```text
$ git ls-remote -- 'ssh://-oProxyCommand=false/x'
--- stdout ---
--- stderr ---
fatal: strange hostname '-oProxyCommand=false' blocked
[exit 128]
```


## G7 unreachable / missing

```text
$ git ls-remote --tags --refs -- 'file:///nope' 'refs/tags/packs-v*'
--- stdout ---
--- stderr ---
fatal: '/nope' does not appear to be a git repository
fatal: Could not read from remote repository.

Please make sure you have the correct access rights
and the repository exists.
[exit 128]
```

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/n.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/n.git' fetch --no-tags --depth 1 -- 'file:///nope' refs/tags/packs-v0.1.0:refs/tags/packs-v0.1.0
--- stdout ---
--- stderr ---
fatal: '/nope' does not appear to be a git repository
fatal: Could not read from remote repository.

Please make sure you have the correct access rights
and the repository exists.
[exit 128]
```

```text
$ git ls-remote -- '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/nope-plain-path'
--- stdout ---
--- stderr ---
fatal: '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/nope-plain-path' does not appear to be a git repository
fatal: Could not read from remote repository.

Please make sure you have the correct access rights
and the repository exists.
[exit 128]
```

```text
$ git ls-remote -- 'https://nonexistent.invalid/x.git'
--- stdout ---
--- stderr ---
fatal: unable to access 'https://nonexistent.invalid/x.git/': Could not resolve host: nonexistent.invalid
[exit 128]
```

```text
$ git ls-remote -- 'https://github.com/airsstack/does-not-exist-probe-xyz'
--- stdout ---
--- stderr ---
fatal: could not read Username for 'https://github.com': terminal prompts disabled
[exit 128]
```

```text
$ git init --bare -q '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/n2.git' && git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/n2.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' refs/tags/packs-v9.9.9:refs/tags/packs-v9.9.9
--- stdout ---
--- stderr ---
fatal: couldn't find remote ref refs/tags/packs-v9.9.9
[exit 128]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/n2.git' fetch --no-tags --depth 1 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' packs-v9.9.9
--- stdout ---
--- stderr ---
fatal: couldn't find remote ref packs-v9.9.9
[exit 128]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/n2.git' fetch --no-tags --depth 1 -- 'https://github.com/airsstack/agentstacks' refs/tags/packs-v9.9.9:refs/tags/packs-v9.9.9
--- stdout ---
--- stderr ---
fatal: could not read Username for 'https://github.com': terminal prompts disabled
[exit 128]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/n2.git' fetch --no-tags --depth 1 -- 'https://github.com/airsstack/airsl' refs/tags/packs-v9.9.9:refs/tags/packs-v9.9.9
--- stdout ---
--- stderr ---
fatal: couldn't find remote ref refs/tags/packs-v9.9.9
[exit 128]
```

```text
$ git ls-remote --exit-code --tags --refs -- 'https://github.com/airsstack/airsl' 'refs/tags/packs-v9.9.9'
--- stdout ---
--- stderr ---
[exit 2]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/nope-dir' fetch -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' x
--- stdout ---
--- stderr ---
fatal: cannot change to '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/nope-dir': No such file or directory
[exit 128]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' rev-parse --verify --end-of-options 'packs-v9.9.9^{commit}'
--- stdout ---
--- stderr ---
fatal: Needed a single revision
[exit 128]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' rev-parse --verify -q --end-of-options 'packs-v9.9.9^{commit}'
--- stdout ---
--- stderr ---
[exit 1]
```


## G9 determinism and order

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree packs-v0.1.0 -- packs/core > '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/lt1'; git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree packs-v0.1.0 -- packs/core > '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/lt2'; cmp '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/lt1' '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/lt2' && echo identical; shasum -a 256 '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/lt1' '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/lt2'
--- stdout ---
identical
a5491cc5e18034d9ec3a64ca7c37c12308d4708ee2fbf6b4654579d05a15c3b7  /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/lt1
a5491cc5e18034d9ec3a64ca7c37c12308d4708ee2fbf6b4654579d05a15c3b7  /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/lt2
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --full-tree --name-only packs-v0.1.0 -- packs/core > '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/names'; LC_ALL=C sort -z '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/names' > '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/names-sorted'; cmp '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/names' '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/names-sorted' && echo 'ls-tree order == LC_ALL=C byte sort'; tr '\0' '\n' < '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/names'
--- stdout ---
ls-tree order == LC_ALL=C byte sort
packs/core/.hidden
packs/core/Z.md
packs/core/crlf-added.md
packs/core/crlf.md
packs/core/lf.md
packs/core/link
packs/core/nested-x.md
packs/core/nested.md
packs/core/nested/deeper/file.md
packs/core/nested0.md
packs/core/pack.toml
packs/core/run.sh
packs/core/sp ace.md
packs/core/sub
packs/core/ü.md
--- stderr ---
[exit 0]
```

```text
$ git -C src ls-tree -r -z --full-tree --name-only HEAD > '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/all'; LC_ALL=C sort -z '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/all' | cmp - '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/out/all' && echo 'whole tree: ls-tree order == byte sort'
--- stdout ---
whole tree: ls-tree order == byte sort
--- stderr ---
[exit 0]
```

```text
$ git -C src ls-tree -z --name-only HEAD -- packs/core/ | tr '\0' '\n'
--- stdout ---
packs/core/.hidden
packs/core/Z.md
packs/core/crlf-added.md
packs/core/crlf.md
packs/core/lf.md
packs/core/link
packs/core/nested-x.md
packs/core/nested.md
packs/core/nested
packs/core/nested0.md
packs/core/pack.toml
packs/core/run.sh
packs/core/sp ace.md
packs/core/sub
packs/core/two.md
packs/core/ü.md
--- stderr ---
[exit 0]
```



## Follow-ups

G6 re-run: exit code of git alone (the earlier block reported the exit of the trailing ls):

```text
$ git ls-remote -- '--upload-pack=touch /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/PWNED'
--- stdout ---
--- stderr ---
fatal: strange pathname '--upload-pack=touch /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/PWNED' blocked
[exit 128]
```

```text
$ test -e '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/PWNED' && echo PWNED-exists || echo PWNED-absent
--- stdout ---
PWNED-absent
--- stderr ---
[exit 0]
```

G4: --format with -z still C-quotes %(path) in 2.39.3? od -c, default and with -c core.quotePath=false, vs default format:

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z --format='%(objectmode) %(objecttype) %(objectname)%x09%(path)' packs-v0.1.0 -- packs/core/ü.md | od -c
--- stdout ---
0000000    1   0   0   6   4   4       b   l   o   b       5   8   7   b
0000020    e   6   b   4   c   3   f   9   3   f   9   3   c   4   8   9
0000040    c   0   1   1   1   b   b   a   5   5   9   6   1   4   7   a
0000060    2   6   c   b  \t   p   a   c   k   s   /   c   o   r   e   /
0000100    ü  **   .   m   d  \0                                        
0000106
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' -c core.quotePath=false ls-tree -r -z --format='%(objectmode) %(objecttype) %(objectname)%x09%(path)' packs-v0.1.0 -- packs/core/ü.md | od -c
--- stdout ---
0000000    1   0   0   6   4   4       b   l   o   b       5   8   7   b
0000020    e   6   b   4   c   3   f   9   3   f   9   3   c   4   8   9
0000040    c   0   1   1   1   b   b   a   5   5   9   6   1   4   7   a
0000060    2   6   c   b  \t   p   a   c   k   s   /   c   o   r   e   /
0000100    ü  **   .   m   d  \0                                        
0000106
--- stderr ---
[exit 0]
```

```text
$ git -C '/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/cache/x.git' ls-tree -r -z packs-v0.1.0 -- packs/core/ü.md | od -c
--- stdout ---
0000000    1   0   0   6   4   4       b   l   o   b       5   8   7   b
0000020    e   6   b   4   c   3   f   9   3   f   9   3   c   4   8   9
0000040    c   0   1   1   1   b   b   a   5   5   9   6   1   4   7   a
0000060    2   6   c   b  \t   p   a   c   k   s   /   c   o   r   e   /
0000100    ü  **   .   m   d  \0                                        
0000106
--- stderr ---
[exit 0]
```

G1: ls-remote pattern is a tail match on the ref name (a branch named packs-v1 in src2):

```text
$ git -C src2 branch packs-v1
--- stdout ---
--- stderr ---
[exit 0]
```

```text
$ git ls-remote --refs -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src2' 'packs-v*'
--- stdout ---
90e2a535db371753890122ed088c81bf3867be7a	refs/heads/packs-v1
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd	refs/tags/packs-v0.1.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.10.0
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80	refs/tags/packs-v0.2.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.9.0
--- stderr ---
[exit 0]
```

```text
$ git ls-remote --tags --refs -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src2' 'packs-v*'
--- stdout ---
aec92b4596aebd6fbc0d2ab92532bbe3c6e0bdcd	refs/tags/packs-v0.1.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.10.0
7eb036fe4d54e8c8cae19243e7e01e0fac4fac80	refs/tags/packs-v0.2.0
90e2a535db371753890122ed088c81bf3867be7a	refs/tags/packs-v0.9.0
--- stderr ---
[exit 0]
```

G5: clone WITHOUT autocrlf (repo .gitattributes alone) for comparison:

```text
$ git -c advice.detachedHead=false clone -q --branch packs-v0.1.0 -- 'file:///private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/11b11407-be42-4105-93b0-46ffa5b55ffb/scratchpad/probe-git/src' clone-plain
--- stdout ---
--- stderr ---
[exit 0]
```

```text
$ od -c clone-plain/packs/core/lf.md; od -c clone-plain/packs/core/crlf.md
--- stdout ---
0000000    a  \r  \n   b  \r  \n                                        
0000006
0000000    a  \r  \n   b  \r  \n                                        
0000006
--- stderr ---
[exit 0]
```



## Follow-up: ls-tree --format path quoting (cwd = cache/x.git)

```text
$ git ls-tree -r -z --format='%(path)' packs-v0.1.0 -- packs/core/ü.md | od -c
--- stdout ---
0000000    p   a   c   k   s   /   c   o   r   e   /   ü  **   .   m   d
0000020   \0                                                            
0000021
--- stderr ---
[exit 0]
```

```text
$ git ls-tree -r -z --format='%(objectname)%x09%(path)' packs-v0.1.0 -- packs/core/ü.md | od -c
--- stdout ---
0000000    5   8   7   b   e   6   b   4   c   3   f   9   3   f   9   3
0000020    c   4   8   9   c   0   1   1   1   b   b   a   5   5   9   6
0000040    1   4   7   a   2   6   c   b  \t   "   p   a   c   k   s   /
0000060    c   o   r   e   /   \   3   0   3   \   2   7   4   .   m   d
0000100    "  \0                                                        
0000102
--- stderr ---
[exit 0]
```

```text
$ git ls-tree -r -z --format='%(objectsize)%x09%(path)' packs-v0.1.0 -- packs/core/ü.md | od -c
--- stdout ---
0000000    2  \t   "   p   a   c   k   s   /   c   o   r   e   /   \   3
0000020    0   3   \   2   7   4   .   m   d   "  \0                    
0000033
--- stderr ---
[exit 0]
```

```text
$ git ls-tree -r -z --format='%(objectsize:padded)%x09%(path)' packs-v0.1.0 -- packs/core/ü.md | od -c
--- stdout ---
0000000                            2  \t   "   p   a   c   k   s   /   c
0000020    o   r   e   /   \   3   0   3   \   2   7   4   .   m   d   "
0000040   \0                                                            
0000041
--- stderr ---
[exit 0]
```

```text
$ git ls-tree -r -z --format='%(objectmode) %(objecttype) %(objectname)%x09%(path)' packs-v0.1.0 -- packs/core/ü.md | od -c
--- stdout ---
0000000    1   0   0   6   4   4       b   l   o   b       5   8   7   b
0000020    e   6   b   4   c   3   f   9   3   f   9   3   c   4   8   9
0000040    c   0   1   1   1   b   b   a   5   5   9   6   1   4   7   a
0000060    2   6   c   b  \t   p   a   c   k   s   /   c   o   r   e   /
0000100    ü  **   .   m   d  \0                                        
0000106
--- stderr ---
[exit 0]
```

```text
$ git -c core.quotePath=false ls-tree -r -z --format='%(objectsize)%x09%(path)' packs-v0.1.0 -- packs/core/ü.md | od -c
--- stdout ---
0000000    2  \t   p   a   c   k   s   /   c   o   r   e   /   ü  **   .
0000020    m   d  \0                                                    
0000023
--- stderr ---
[exit 0]
```

```text
$ git ls-tree -r -z -l packs-v0.1.0 -- packs/core/ü.md | od -c
--- stdout ---
0000000    1   0   0   6   4   4       b   l   o   b       5   8   7   b
0000020    e   6   b   4   c   3   f   9   3   f   9   3   c   4   8   9
0000040    c   0   1   1   1   b   b   a   5   5   9   6   1   4   7   a
0000060    2   6   c   b                               2  \t   p   a   c
0000100    k   s   /   c   o   r   e   /   ü  **   .   m   d  \0        
0000116
--- stderr ---
[exit 0]
```

```text
$ git ls-tree -r -z -l packs-v0.1.0 -- 'packs/core/sp ace.md' | od -c
--- stdout ---
0000000    1   0   0   6   4   4       b   l   o   b       5   8   7   b
0000020    e   6   b   4   c   3   f   9   3   f   9   3   c   4   8   9
0000040    c   0   1   1   1   b   b   a   5   5   9   6   1   4   7   a
0000060    2   6   c   b                               2  \t   p   a   c
0000100    k   s   /   c   o   r   e   /   s   p       a   c   e   .   m
0000120    d  \0                                                        
0000122
--- stderr ---
[exit 0]
```



## G8 missing git binary (Rust probe, std only)

Toolchain: `cargo 1.99.0 (5f94df478 2026-08-27)`, `rustc 1.99.0 (b940084d7 2026-09-28)`, edition 2024, no dependencies.
Crate: `cargo new --edition 2024 --vcs none -q gitmissing` (target dir removed after the run).

```rust
use std::process::Command;

fn probe(program: &str) {
    match Command::new(program).arg("--version").output() {
        Ok(out) => println!(
            "{program}: ok status={:?} stdout={:?} stderr={:?}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        Err(e) => println!(
            "{program}: Display={e} | kind={:?} | raw_os_error={:?} | Debug={e:?}",
            e.kind(),
            e.raw_os_error()
        ),
    }
}

fn main() {
    probe("git-does-not-exist");
    probe("/nonexistent/dir/git");
    probe("/usr/bin/git");
}
```

```text
$ cargo run -q
git-does-not-exist: Display=No such file or directory (os error 2) | kind=NotFound | raw_os_error=Some(2) | Debug=Os { code: 2, kind: NotFound, message: "No such file or directory" }
/nonexistent/dir/git: Display=No such file or directory (os error 2) | kind=NotFound | raw_os_error=Some(2) | Debug=Os { code: 2, kind: NotFound, message: "No such file or directory" }
/usr/bin/git: ok status=ExitStatus(unix_wait_status(0)) stdout="git version 2.39.3 (Apple Git-146)\n" stderr=""
[exit 0]
```

Note: the io::Error Display does not name the program; a refusal that must name it has to add it.
