#!/bin/sh
# Book probe 3 (2026-11-24): the spec §7.7 build step, run under bash -e (GitHub's default shell
# for `run:`), over three books: clean, a missing {{#include}} target, a missing chapter.
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
cat > "$S/buildstep.sh" <<'EOF'
mdbook build book > "$RUNNER_TEMP/mdbook.log" 2>&1 || { cat "$RUNNER_TEMP/mdbook.log"; exit 1; }
cat "$RUNNER_TEMP/mdbook.log"
if grep -q '\[ERROR\]' "$RUNNER_TEMP/mdbook.log"; then
  echo "mdbook build book logged an error; see the [ERROR] lines above" >&2
  exit 1
fi
EOF
echo '$ cat buildstep.sh'; cat "$S/buildstep.sh"
for k in clean include chapter; do
  P="$S/step-$k"; rm -rf "$P"; mkdir -p "$P/book/src" "$P/packs/core" "$P/tmp"
  printf '[book]\ntitle = "p"\nsrc = "src"\n[build]\ncreate-missing = false\n' > "$P/book/book.toml"
  printf '# core\n' > "$P/packs/core/README.md"
  printf '# Summary\n- [A](a.md)\n' > "$P/book/src/SUMMARY.md"
  printf '{{#include ../../packs/core/README.md}}\n' > "$P/book/src/a.md"
  [ "$k" = include ] && printf '{{#include ../../packs/gone/README.md}}\n' > "$P/book/src/a.md"
  [ "$k" = chapter ] && printf -- '- [B](b.md)\n' >> "$P/book/src/SUMMARY.md"
  echo "== $k"
  (cd "$P" && RUNNER_TEMP="$P/tmp" bash -e "$S/buildstep.sh" 2>&1 | grep -v INFO | sed "s#$S#<scratch>#g"; echo "exit=${PIPESTATUS:-?}")
  (cd "$P" && RUNNER_TEMP="$P/tmp" bash -e "$S/buildstep.sh" > /dev/null 2>&1; echo "status=$?")
done
