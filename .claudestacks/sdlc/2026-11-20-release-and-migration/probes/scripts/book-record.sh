#!/bin/sh
# Writes probes/book-results.md and probes/book-structure-results.txt from the book probes.
# Run from the worktree root. book-probe.sh starts from a clean scratch (it removes what an
# earlier run left), and book-probe6.sh section f reads book-probe.sh's output.
D=.claudestacks/sdlc/2026-11-20-release-and-migration/probes
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
{
  echo '# Book probes (`scripts/book-probe.sh`, `book-probe2.sh`, `book-probe3.sh`, `book-probe6.sh`, 2026-11-24)'
  echo
  echo 'Worktree HEAD `e4e1acd`, mdBook 0.4.52, mdbook-mermaid 0.16.2, airsl 0.1.5. Scratch paths shown as `<scratch>`.'
  echo
  echo '## book-probe.sh'; echo; echo '```text'
  sh "$D/scripts/book-probe.sh" 2>&1 | sed "s#$S#<scratch>#g"
  echo '```'; echo
  echo '## book-probe2.sh'; echo; echo '```text'
  sh "$D/scripts/book-probe2.sh" 2>&1
  echo '```'; echo
  echo '## book-probe3.sh'; echo; echo '```text'
  sh "$D/scripts/book-probe3.sh" 2>&1
  echo '```'; echo
  echo '## book-probe6.sh'; echo; echo '```text'
  sh "$D/scripts/book-probe6.sh" 2>&1 | sed "s#$S#<scratch>#g"
  echo '```'
} > "$D/book-results.md"
{
  echo '# Book structure facts (`scripts/book-probe4.sh`, 2026-11-24)'
  echo
  sh "$D/scripts/book-probe4.sh" 2>&1
} > "$D/book-structure-results.txt"
