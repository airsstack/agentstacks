#!/bin/sh
# Probe: find the newest stable `<app>-vX.Y.Z` release of a repository from the GitHub releases
# API with POSIX sh tools only (curl, grep, sed, sort, tail), without relying on the API's order.
# Run against claudestacks (claudevs-cli releases mixed with airsclaude tags) and airsl.
set -u
newest() { # repo app
  curl --proto '=https' --tlsv1.2 -sSfL --max-time 30 "https://api.github.com/repos/$1/releases?per_page=100" |
    grep -o "\"tag_name\": *\"$2-v[0-9]*\.[0-9]*\.[0-9]*\"" |
    sed "s/.*\"$2-v\([0-9.]*\)\"/\1/" |
    sort -t. -k1,1n -k2,2n -k3,3n |
    tail -n 1
}
echo "\$ newest airsstack/claudestacks claudevs-cli"; newest airsstack/claudestacks claudevs-cli; echo "exit=$?"
echo "\$ newest airsstack/claudestacks airsclaude"; newest airsstack/claudestacks airsclaude; echo "exit=$?"
echo "\$ newest airsstack/airsl airsl-cli"; newest airsstack/airsl airsl-cli; echo "exit=$?"
echo "\$ newest airsstack/agentstacks agentstacks-cli   # no releases yet"; v=$(newest airsstack/agentstacks agentstacks-cli); echo "exit=$? value='$v'"
echo "\$ printf '0.1.9\n0.1.10\n0.2.0\n0.1.2\n' | sort -t. -k1,1n -k2,2n -k3,3n | tail -n 1"
printf '0.1.9\n0.1.10\n0.2.0\n0.1.2\n' | sort -t. -k1,1n -k2,2n -k3,3n | tail -n 1
echo "$ gh release list -R airsstack/claudestacks"; gh release list -R airsstack/claudestacks < /dev/null
