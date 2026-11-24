#!/bin/sh
# Book probe 4 (2026-11-24): the structure facts the book move relies on. Run from the worktree root.
set -u
W=$(pwd)
CS=/Users/hiraq/Projects/airsstack/claudestacks
echo '$ git -C claudestacks rev-parse --short HEAD'; git -C "$CS" rev-parse --short HEAD
echo '== docs/ paths named in README.md and CLAUDE.md'
grep -n 'docs/' README.md CLAUDE.md
echo '== crates/claudevs/docs: files, and who links them (then a control grep for a known path)'
find crates/claudevs/docs -type f | sort
grep -rn 'claudevs/docs' --include='*.md' --include='*.rs' --include='*.toml' --include='*.yml' . --exclude-dir=.claudestacks --exclude-dir=target; echo "(grep exit $?)"
grep -rln 'claudevs/examples' --include='*.md' crates | head -2
echo '== claudestacks book: the claudevs part (SUMMARY) and its files'
sed -n '/^# claudevs/,/^# The Plugin Suite/p' "$CS/book/src/SUMMARY.md"
(cd "$CS/book/src" && ls ch05-*.md ch06-*.md ch07-*.md ch08-*.md ch09-*.md appendix-01-claudevs-cli.md | wc -l)
echo '== links from those chapters to chapter files outside the set (expect none), then a control'
(cd "$CS/book/src" && grep -on '](\(ch[0-9][0-9]-[^)#]*\|appendix-[^)#]*\|introduction[^)#]*\)' ch05-*.md ch06-*.md ch07-*.md ch08-*.md ch09-*.md appendix-01-claudevs-cli.md | grep -v -e '](ch0[5-9]-' -e '](appendix-01-'); echo "(grep exit $?)"
(cd "$CS/book/src" && grep -c '](ch0[5-9]-' ch05-00-claudevs-getting-started.md)
echo '== every claudestacks mention in those chapters'
(cd "$CS/book/src" && grep -n -i claudestacks ch05-*.md ch06-*.md ch07-*.md ch08-*.md ch09-*.md appendix-01-claudevs-cli.md | grep -c -v -e 'https://github.com/airsstack/claudestacks/' -e 'https://airsstack.github.io/claudestacks/install.sh'; true)
(cd "$CS/book/src" && grep -n -i claudestacks ch05-*.md ch06-*.md ch07-*.md ch08-*.md ch09-*.md appendix-01-claudevs-cli.md | wc -l)
echo '== claudestacks book lines the pointer goes next to'
grep -n 'Start with \[claudevs Getting Started\]' "$CS/book/src/introduction.md"
sed -n 1p "$CS/book/src/ch05-00-claudevs-getting-started.md"
echo '== claudevs README book links'
grep -n 'airsstack.github.io/claudestacks' crates/claudevs/README.md crates/claudevs-cli/README.md
echo '== pack READMEs: excluded from installs; their links today'
grep -n 'README' packs/*/pack.toml
grep -on '\]([^)]*)' packs/*/README.md
echo '== org README Projects table, Docs column'
grep -n '^| Project\|^|---' /Users/hiraq/Projects/airsstack/README.md | head -3
sed -n 104,108p /Users/hiraq/Projects/airsstack/README.md | cut -c1-60
echo '== README.md sections to edit'
sed -n 84,87p README.md; sed -n 103,121p README.md; sed -n 133,153p README.md
