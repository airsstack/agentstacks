#!/bin/sh
# Book probe 6 (2026-11-24), answering the spec review. Run from the worktree root.
#  a. an unedited pack README included from book/src: what its relative links become
#  b. a broken chapter link and a bad anchor: does mdBook 0.4.52 report either?
#  c. repo-wide: every file outside .claudestacks/ naming a docs/ path (control: packs/)
#  d. link checkers in the org's other books (control: the mdbook pin they do have)
#  e. GitHub's default shell for run: (workflow-syntax page, saved to probes/)
#  f. the claudevs move check: each claudestacks e099300 chapter through the §7.6 map equals the moved file
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
W=$(pwd)
D=.claudestacks/sdlc/2026-11-20-release-and-migration/probes
O=/Users/hiraq/Projects/airsstack
CS=$O/claudestacks

echo "== a. unedited packs/guideline-rust/README.md included"
P="$S/p6a"; rm -rf "$P"; mkdir -p "$P/book/src" "$P/packs/guideline-rust"
cp "$W/packs/guideline-rust/README.md" "$P/packs/guideline-rust/"
printf '[book]\ntitle = "p"\nsrc = "src"\n[build]\ncreate-missing = false\n' > "$P/book/book.toml"
printf '# Summary\n- [gr](gr.md)\n' > "$P/book/src/SUMMARY.md"
printf '{{#include ../../packs/guideline-rust/README.md}}\n' > "$P/book/src/gr.md"
(cd "$P" && mdbook build book > out.txt 2>&1; echo "exit=$?"; grep -c -e ERROR -e WARN out.txt)
echo '$ README links (source) and the hrefs they became'
grep -o '\]([^)]*)' "$P/packs/guideline-rust/README.md"
grep -o 'href="[^"]*"' "$P/book/book/gr.html" | grep -e LICENSE -e skills

echo "== b. a broken chapter link and a bad anchor"
printf '# gr2\n\n[nope](nope.md) and [anchor](gr.md#no-such-anchor)\n' > "$P/book/src/gr2.md"
printf -- '- [gr2](gr2.md)\n' >> "$P/book/src/SUMMARY.md"
(cd "$P" && mdbook build book > out.txt 2>&1; echo "exit=$?"; echo "ERROR/WARN lines: $(grep -c -e ERROR -e WARN out.txt)")
grep -o 'href="nope[^"]*"\|href="gr.html#[^"]*"' "$P/book/book/gr2.html"

echo "== c. repo-wide docs/ paths, outside .claudestacks/"
echo '$ git grep -n "docs/" -- ":!.claudestacks" | grep -v -e "https\?://" (outside URLs dropped)'
git grep -n 'docs/' -- ':!.claudestacks' | grep -v -e 'https\?://'
echo '$ control: git grep -c "packs/" -- Makefile.toml'; git grep -c 'packs/' -- Makefile.toml

echo "== d. link checkers in the other books"
for p in airsl buildl claudestacks; do
  printf '%s: ' "$p"; grep -rlE 'linkcheck|lychee|link-check|markdown-link' "$O/$p/.github" "$O/$p/book/book.toml" "$O/$p/Makefile.toml" > /dev/null 2>&1; echo "grep exit=$?"
  printf '%s control (mdbook pin): ' "$p"; grep -c 'mdbook@0.4.52' "$O/$p/.github/workflows/pages.yml"
done

echo "== e. GitHub's default shell"
curl -sS -L 'https://docs.github.com/api/article/body?pathname=/en/actions/reference/workflows-and-actions/workflow-syntax' -o "$D/workflow-syntax.md"
grep -n 'bash -e {0}' "$D/workflow-syntax.md" | head -3

echo "== f. claudevs move check over the probe's moved files"
B="$S/bookprobe/book/src"; ok=0; bad=0
for f in $(git -C "$CS" ls-tree --name-only e099300 book/src/ | sed 's#book/src/##' | grep -E '^(ch0[5-9]-|appendix-01-claudevs-cli)'); do
  case "$f" in
    ch05-*) n="ch08-${f#ch05-}";; ch06-*) n="ch09-${f#ch06-}";; ch07-*) n="ch10-${f#ch07-}";;
    ch08-*) n="ch11-${f#ch08-}";; ch09-*) n="ch12-${f#ch09-}";; *) n="$f";;
  esac
  if git -C "$CS" show "e099300:book/src/$f" | sed -f "$S/cvmap.sed" | cmp -s - "$B/$n"; then ok=$((ok+1)); else echo "DIFFERS $f -> $n"; bad=$((bad+1)); fi
done
echo "identical=$ok differ=$bad"
