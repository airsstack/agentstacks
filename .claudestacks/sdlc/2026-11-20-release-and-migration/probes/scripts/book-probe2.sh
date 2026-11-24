#!/bin/sh
# Book probe 2 (2026-11-24): what mdBook 0.4.52 does with an {{#include}} whose file is missing,
# and GitHub's path-filter meaning of `*` (saved workflow-syntax doc).
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
D=.claudestacks/sdlc/2026-11-20-release-and-migration/probes
P="$S/incprobe"; rm -rf "$P"; mkdir -p "$P/book/src"
printf '[book]\ntitle = "p"\nsrc = "src"\n[build]\ncreate-missing = false\n' > "$P/book/book.toml"
printf '# Summary\n- [A](a.md)\n' > "$P/book/src/SUMMARY.md"
printf '# A\n\n{{#include ../../packs/gone/README.md}}\n' > "$P/book/src/a.md"
cd "$P" || exit 1
echo '$ mdbook build book   # a.md includes ../../packs/gone/README.md, which does not exist'
mdbook build book > out.txt 2>&1; echo "exit=$?"; sed "s#$S#<scratch>#g" out.txt
echo '$ grep -c "{{#include" book/book/a.html'; grep -c '{{#include' book/book/a.html
cd - > /dev/null || exit 1
echo '$ grep -n -e "\*" workflow-syntax-filter.md | head (saved GitHub doc)'
grep -n -e "Matches zero or more characters" -e "does not match the" "$D/workflow-syntax-filter.md" | head
