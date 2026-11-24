#!/bin/sh
# Book design probes (2026-11-24), mdBook 0.4.52 + mdbook-mermaid 0.16.2.
#  a. the proposed book: docs/ files moved and renamed, claudestacks' claudevs chapters moved,
#     renamed and their URLs rewritten, pack READMEs included, placeholders for new chapters;
#     build, then check every internal link (file + anchor)
#  b. the same build with a missing chapter (create-missing = false)
#  c. leftovers: claudestacks mentions in moved chapters, rewritten example URLs exist here
#  d. "A First Project": official names via insteadOf onto a bare copy of HEAD tagged packs-v0.1.0
#  e. what reads docs/ (code, tests, Makefile, CI), with a control
#  f. pack text gates over the new packs/CHANGELOG.md text
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
W=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/release-migration
CS=/Users/hiraq/Projects/airsstack/claudestacks
mdbook --version; mdbook-mermaid --version

echo "== a. proposed book"
rm -rf "$S/official.git" "$S/gitconfig" "$S/firstproj" "$S/firstproj2"
P="$S/bookprobe"; rm -rf "$P"; mkdir -p "$P/book/src" "$P/packs"
for p in core sdlc journal guideline-rust guideline-python; do
  mkdir -p "$P/packs/$p"
  # the proposed README edit: every relative link becomes https://github.com/airsstack/agentstacks/{blob|tree}/main/<path>
  sed -E -e 's#\]\(\.\./\.\./LICENSE\)#](https://github.com/airsstack/agentstacks/blob/main/LICENSE)#' \
         -e "s#\\]\\(((skills|references)/([^)]*/)?)\\)#](https://github.com/airsstack/agentstacks/tree/main/packs/$p/\\1)#" \
         -e "s#\\]\\(((skills|references)/[^)]*\\.md)\\)#](https://github.com/airsstack/agentstacks/blob/main/packs/$p/\\1)#" \
         "$W/packs/$p/README.md" > "$P/packs/$p/README.md"
done
echo '$ the five pack READMEs links after the edit'; grep -on '\]([^)]*)' "$P"/packs/*/README.md | sed "s#$P/##"
cp "$CS/book/book.toml" "$P/book/book.toml"
sed -i '' 's/claudestacks/agentstacks/g' "$P/book/book.toml"
echo '$ cat book/book.toml'; cat "$P/book/book.toml"
(cd "$P" && mdbook-mermaid install book > /dev/null 2>&1)
# docs/ -> chapters
DOCMAP="cli:ch04-00-cli plan-and-apply:ch05-00-plan-and-apply authoring:ch06-00-authoring extensions:ch06-01-extensions hooks:ch06-02-hooks architecture:ch07-00-architecture"
sedf="$S/docmap.sed"; : > "$sedf"
for m in $DOCMAP; do echo "s#](${m%%:*}\\.md#](${m#*:}.md#g" >> "$sedf"; done
echo 's/(#missing-values)/(#values)/' >> "$sedf"
# each moved page's title becomes its SUMMARY entry (line 1 only)
cat >> "$sedf" <<'EOF'
1s/^# CLI$/# The Command Line/
1s/^# Plan, apply, doctor and update$/# Plan, Apply and the Lock/
1s/^# Build an agentstacks tool extension$/# Writing a Tool Extension/
1s/^# Writing a tool extension$/# The Extension API/
1s/^# Architecture$/# Under the Hood/
EOF
echo '$ cat docmap.sed'; cat "$sedf"
for m in $DOCMAP; do sed -f "$sedf" "$W/docs/${m%%:*}.md" > "$P/book/src/${m#*:}.md"; done
# claudevs chapters -> ch08..ch12
cvsed="$S/cvmap.sed"; : > "$cvsed"
for f in $(cd "$CS/book/src" && ls ch05-*.md ch06-*.md ch07-*.md ch08-*.md ch09-*.md); do
  # one rule per file, never chained: ch05->ch08, ch06->ch09, ch07->ch10, ch08->ch11, ch09->ch12
  case "$f" in
    ch05-*) n="ch08-${f#ch05-}";; ch06-*) n="ch09-${f#ch06-}";; ch07-*) n="ch10-${f#ch07-}";;
    ch08-*) n="ch11-${f#ch08-}";; ch09-*) n="ch12-${f#ch09-}";;
  esac
  echo "s#$f#@@$n#g" >> "$cvsed"
done
cat >> "$cvsed" <<'EOF'
s#https://airsstack.github.io/claudestacks/install.sh#https://airsstack.github.io/agentstacks/claudevs.sh#g
s#https://github.com/airsstack/claudestacks/#https://github.com/airsstack/agentstacks/#g
s#^claudevs 0\.1\.1$#claudevs 0.1.2#
s#@@ch#ch#g
EOF
echo '$ cat cvmap.sed'; cat "$cvsed"
for f in $(cd "$CS/book/src" && ls ch05-*.md ch06-*.md ch07-*.md ch08-*.md ch09-*.md) appendix-01-claudevs-cli.md; do
  case "$f" in
    ch05-*) n="ch08-${f#ch05-}";; ch06-*) n="ch09-${f#ch06-}";; ch07-*) n="ch10-${f#ch07-}";;
    ch08-*) n="ch11-${f#ch08-}";; ch09-*) n="ch12-${f#ch09-}";; *) n="$f";;
  esac
  sed -f "$cvsed" "$CS/book/src/$f" > "$P/book/src/$n"
done
# placeholders for the new chapters, and pack includes
for f in introduction ch01-00-getting-started ch01-01-installation ch01-02-first-project ch02-00-packs ch03-00-migration appendix-00; do
  printf '# %s\n\nplaceholder\n' "$f" > "$P/book/src/$f.md"
done
i=1; for p in core sdlc journal guideline-rust guideline-python; do
  printf '{{#include ../../packs/%s/README.md}}\n' "$p" > "$P/book/src/ch02-0$i-$p.md"; i=$((i+1))
done
cvsum=$(sed -n '/^# claudevs/,/^# The Plugin Suite/p' "$CS/book/src/SUMMARY.md" | sed '$d' | sed -f "$cvsed")
cat > "$P/book/src/SUMMARY.md" <<EOF
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

$cvsum

# Appendix

- [Appendix](appendix-00.md)
  - [A. claudevs CLI Reference](appendix-01-claudevs-cli.md)
EOF
echo '$ cat book/src/SUMMARY.md'; cat "$P/book/src/SUMMARY.md"
echo '$ ls book/src | wc -l'; ls "$P/book/src" | wc -l
echo '$ mdbook build book'; (cd "$P" && mdbook build book > "$S/mdb.out" 2>&1; echo "exit=$?"); grep -c ERROR "$S/mdb.out"
O="$P/book/book"
bad=0; n=0
# every href that is not an outside URL: its file (or directory) must exist, and its anchor too
for h in $(cd "$O" && ls *.html | grep -v -e '^print.html$' -e '^toc.html$' -e '^404.html$'); do
  for l in $(grep -o 'href="[^"]*"' "$O/$h" | sed 's/href="//;s/"$//' | sort -u); do
    case "$l" in http:*|https:*|mailto:*|//*) continue;; esac
    n=$((n+1)); file=${l%%#*}; anc=""; case "$l" in *"#"*) anc=${l#*#};; esac
    [ -z "$file" ] && file=$h
    if [ ! -e "$O/$file" ]; then echo "BROKEN $h -> $l (no file)"; bad=$((bad+1));
    elif [ -n "$anc" ] && ! grep -q "id=\"$anc\"" "$O/$file"; then echo "BROKEN $h -> $l (no anchor)"; bad=$((bad+1)); fi
  done
done
echo "relative hrefs checked=$n broken=$bad"
echo '$ the moved pages titles'; for f in ch04-00-cli ch05-00-plan-and-apply ch06-00-authoring ch06-01-extensions ch06-02-hooks ch07-00-architecture; do head -1 "$P/book/src/$f.md"; done
echo '$ grep -c class="mermaid" over chapters (total)'; cat "$O"/ch*.html | grep -o 'class="mermaid"' | wc -l
echo '$ relative links left in included pack chapters'
for i in 1 2 3 4 5; do grep -o 'href="[^"]*"' "$O"/ch02-0$i-*.html | grep -v -e 'href="http' -e 'href="#' -e '\.html' -e '\.css' -e '\.js' -e 'favicon' -e 'href="/agentstacks/' | sort -u; done
echo "== b. missing chapter"
printf -- '- [Gone](gone.md)\n' >> "$P/book/src/SUMMARY.md"
(cd "$P" && mdbook build book 2>&1 | grep ERROR | head -2; mdbook build book > /dev/null 2>&1; echo "exit=$?")

echo "== c. leftovers in moved claudevs chapters"
echo '$ grep -c -i claudestacks (moved claudevs chapters, total)'; cat "$P"/book/src/ch08-*.md "$P"/book/src/ch09-*.md "$P"/book/src/ch10-*.md "$P"/book/src/ch11-*.md "$P"/book/src/ch12-*.md "$P/book/src/appendix-01-claudevs-cli.md" | grep -c -i claudestacks
echo '$ every rewritten agentstacks example URL exists in this tree'
for u in $(grep -oh 'https://github.com/airsstack/agentstacks/[^)# ]*' "$P"/book/src/ch*.md "$P/book/src/appendix-01-claudevs-cli.md" | sort -u); do
  path=$(echo "$u" | sed -E 's#https://github.com/airsstack/agentstacks/(tree|blob)/main/##')
  [ -e "$W/$path" ] && echo "ok $path" || echo "MISSING $path"
done
echo '$ anchor #without-a-claude-binary: heading in crates/claudevs/examples/README.md'
grep -n -i 'without a .*claude.* binary' "$W/crates/claudevs/examples/README.md"
echo '$ the version line in the moved installation chapter'; grep -n 'claudevs 0\.1' "$P/book/src/ch08-01-installation.md"

echo "== d. A First Project (official names, packs-v0.1.0 on a bare copy of HEAD)"
B="$W/target/debug/agentstacks"; [ -x "$B" ] || (cd "$W" && cargo build -q -p agentstacks-cli)
rm -rf "$S/official.git"; git clone -q --bare "$W" "$S/official.git"; git -C "$S/official.git" tag -a packs-v0.1.0 -m packs-v0.1.0 HEAD
printf '[url "file://%s/official.git"]\n\tinsteadOf = https://github.com/airsstack/agentstacks\n' "$S" > "$S/gitconfig"
D="$S/firstproj"; rm -rf "$D"; mkdir -p "$D"; cd "$D" || exit 1
git init -q .
echo '$ airsl --version'; airsl --version
for c in "init --tool claude --pack core --pack sdlc --pack guideline-rust" "apply --yes" "plan" "doctor" "update"; do
  echo "\$ agentstacks $c"
  GIT_CONFIG_GLOBAL="$S/gitconfig" "$B" $c < /dev/null > out.txt 2>&1; echo "exit=$?"
  echo "-- first 3 lines"; head -3 out.txt; echo "-- last 4 lines"; tail -4 out.txt
done
echo '$ cat agentstacks.toml'; cat agentstacks.toml
echo '$ ls -a'; ls -a
echo '$ agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust --yes   # a fresh project, one step'
D2="$S/firstproj2"; rm -rf "$D2"; mkdir -p "$D2"; cd "$D2" || exit 1; git init -q .
GIT_CONFIG_GLOBAL="$S/gitconfig" "$B" init --tool claude --pack core --pack sdlc --pack guideline-rust --yes < /dev/null > out.txt 2>&1; echo "exit=$?"; tail -3 out.txt

echo "== e. what reads docs/"
cd "$W" || exit 1
echo '$ grep -rn "docs/" Makefile.toml .github deny.toml Cargo.toml crates/*/Cargo.toml'
grep -rn 'docs/' Makefile.toml .github deny.toml Cargo.toml crates/*/Cargo.toml 2>/dev/null
echo "(exit $?)"
echo '$ control: the same grep finds packs/ in Makefile.toml'; grep -c 'packs/' Makefile.toml
echo '$ mentions of docs paths, book, raw URLs inside docs/*.md'
grep -n -e 'docs/' -e 'book' -e 'raw\.githubusercontent' docs/*.md

echo "== f. pack text gates over the new packs/CHANGELOG.md text"
G="$S/gate"; rm -rf "$G"; mkdir -p "$G/packs"
cat > "$G/packs/CHANGELOG.md" <<'EOF'
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

Coming from the claudestacks plugins? See
[Moving from the claudestacks Plugins](https://airsstack.github.io/agentstacks/ch03-00-migration.html).
EOF
cd "$G" || exit 1
grep -rnE 'CLAUDE_PLUGIN_(ROOT|OPTION)|user_config|claudestacks(-[a-z]+)*:[a-z]' packs/ --exclude=README.md --exclude='*_test.lua'; echo "gate1 grep exit=$?"
grep -rnE '(^|[ `"(])/(snapshot-(load|save)|journal-[a-z]+)\b|subagent_type: [a-z]|Namespaced as|`(coder|explorer|reviewer|task-briefer|chain-reader|artifact-reviewer|journal-[a-z]+|context-handoff|enforce-doctor|orchestrate|process-guidelines|snapshot-load|snapshot-save|statusline)`' packs/ --include='*.md' --exclude=README.md --exclude=doc-comment-discipline.md --exclude=docstring-discipline.md; echo "gate2 grep exit=$?"
