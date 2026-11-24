#!/bin/sh
# Probe: the exact §6 jq program (extracted from spec.md) with claudestacks' runner table
# (extracted from its smoke workflow), over real dist plan JSON for both binary tags, run the way
# the targets job runs it: jq -c --argjson runners "$runners" "<program>" <<<"$PLAN".
set -u
S=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/a475ab1a-d9dd-4477-8e0d-dc1ff7315fd2/scratchpad
SPEC=/Users/hiraq/Projects/airsstack/agentstacks/.claude/worktrees/release-migration/.claudestacks/sdlc/2026-11-20-release-and-migration/spec.md
SMOKE=/Users/hiraq/Projects/airsstack/claudestacks/.github/workflows/claudevs-cli-smoke.yml
program=$(awk '$0 == "```text" {buf=""; on=1; next} on && $0 == "```" {if (buf ~ /^\. as \$all/) {printf "%s", buf; exit} on=0; next} on {buf = buf $0 "\n"}' "$SPEC")
runners=$(awk "/runners='\\{/{on=1; sub(/.*runners='/, \"\")} on {print} on && /^ *\\}'/{exit}" "$SMOKE" | sed "s/}'\$/}/")
echo '$ jq --version'; jq --version
echo '--- program (from spec.md §6)'; printf '%s\n' "$program"
echo '--- runners (from claudestacks smoke workflow)'; printf '%s\n' "$runners" | jq -c .
cd "$S/distprobe" || exit 1
for t in agentstacks-cli-v0.1.0 claudevs-cli-v0.1.2; do
  echo "--- dist plan --tag=$t | jq -c --argjson runners \"\$runners\" \"\$program\""
  PLAN=$(dist plan --tag="$t" --output-format=json 2>/dev/null)
  printf '%s' "$PLAN" | jq -c --argjson runners "$runners" "$program"
  echo "jq exit=$?"
done
