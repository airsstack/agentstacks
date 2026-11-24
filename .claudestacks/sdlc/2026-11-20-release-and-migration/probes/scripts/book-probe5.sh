#!/bin/sh
# Book probe 5 (2026-11-24): RUNNER_TEMP in GitHub's variables reference (saved to probes/),
# and book.toml after `mdbook-mermaid install book` compared with the file it was given.
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
D=.claudestacks/sdlc/2026-11-20-release-and-migration/probes
curl -sS -L 'https://docs.github.com/api/article/body?pathname=/en/actions/reference/workflows-and-actions/variables' -o "$D/variables.md"
echo '$ grep -n RUNNER_TEMP probes/variables.md'
grep -n 'RUNNER_TEMP' "$D/variables.md" | head -3
echo '$ book.toml given (claudestacks with the name swapped) vs after mdbook-mermaid install'
sed 's/claudestacks/agentstacks/g' /Users/hiraq/Projects/airsstack/claudestacks/book/book.toml > "$S/given.toml"
diff "$S/given.toml" "$S/bookprobe/book/book.toml"; echo "diff exit=$?"
