#!/bin/sh
# Prints the HTTP status of each URL given.
for u in "$@"; do
  printf '%s -> ' "$u"
  curl -sSL -o /dev/null -w '%{http_code}\n' "$u"
done
