#!/bin/sh
# Runs `dist plan` for each tag given and prints the announcement and releases (or the error).
for t in "$@"; do
  echo "=== $t"
  dist plan --tag="$t" --output-format=json 2>/dev/null | python3 -I -c '
import sys, json
s = sys.stdin.read()
try:
    d = json.loads(s)
    print("announce:", d.get("announcement_tag"), "releases:", [(r["app_name"], r["app_version"]) for r in d.get("releases", [])])
except Exception:
    print(s.strip()[:600])
'
done
