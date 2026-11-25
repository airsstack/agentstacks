#!/bin/sh
# The shell probes behind api-refusals-output.txt (P7), edges-output.txt (P8),
# conformance-output.txt (P15) and airsl-test-output.txt (P5). Run from the repository root at
# HEAD 33e3925 after `cargo build -p agentstacks-cli`; airsl 0.1.5 on PATH.
set -u
P=$PWD/.claudestacks/sdlc/2026-11-25-api-freeze-and-1-0/probes; B=$PWD/target/debug/agentstacks
run() { (cd "$T" && "$B" "$@") > "$T/.o" 2> "$T/.e" && rc=0 || rc=$?; echo "stdout:"; sed "s|$T|<tmp>|g" "$T/.o"; echo "stderr:"; sed "s|$T|<tmp>|g" "$T/.e"; echo "exit=$rc"; }
project() { T=$(mktemp -d); mkdir -p "$T/ext" "$T/packs"; cp -R extensions/claude "$T/ext/claude"; cp -R crates/agentstacks-cli/tests/fixtures/project/packs/demo "$T/packs/demo"
  printf 'api = 0\n\n[tools.claude]\npath = "ext/claude"\n\n[packs.demo]\npath = "packs/demo"\n' > "$T/agentstacks.toml"; }

# P7: api refusals
{ echo "## P7"; project
  echo "### baseline apply --yes"; run apply --yes
  echo "### agentstacks.toml api = 1"; sed -i '' 's/^api = 0/api = 1/' "$T/agentstacks.toml"; run plan; sed -i '' 's/^api = 1/api = 0/' "$T/agentstacks.toml"
  echo "### agentstacks.lock api = 1"; sed -i '' 's/^api = 0/api = 1/' "$T/agentstacks.lock"; run plan; sed -i '' 's/^api = 1/api = 0/' "$T/agentstacks.lock"
  echo "### pack.toml api = 1"; sed -i '' 's/^api = 0/api = 1/' "$T/packs/demo/pack.toml"; run plan; sed -i '' 's/^api = 1/api = 0/' "$T/packs/demo/pack.toml"
  echo "### extension.toml api = 1"; sed -i '' -E 's/^api( *)= 0/api\1= 1/' "$T/ext/claude/extension.toml"; run plan
  echo "### update"; sed -i '' -E 's/^api( *)= 1/api\1= 0/' "$T/ext/claude/extension.toml"; run update --yes
  echo "### doctor"; run doctor; rm -rf "$T"; } > "$P/api-refusals-output.txt" 2>&1

# P8: edges
{ echo "## P8"; T=$(mktemp -d); echo "### plan with no agentstacks.toml"; run plan; rm -rf "$T"; project
  for v in '""' '"   "' '1' '"nope"'; do sed -i '' -E "s/^airsl = .*/airsl = $v/" "$T/packs/demo/pack.toml"; echo "### requires.airsl = $v"; run plan; done
  sed -i '' -E 's/^airsl = .*/airsl = ">=0.1.4"/' "$T/packs/demo/pack.toml"
  sed -i '' -E 's/^\[requires\]$/[requires]\nagentstacks = ">=1.0"/' "$T/packs/demo/pack.toml"; echo "### requires.agentstacks"; run plan
  cp crates/agentstacks-cli/tests/fixtures/project/packs/demo/pack.toml "$T/packs/demo/pack.toml"
  printf '\n[requires]\nagentstacks = ">=1.0"\n' >> "$T/ext/claude/extension.toml"; echo "### extension [requires]"; run plan; rm -rf "$T"
  echo "## versions"; grep -H '^version' packs/*/pack.toml extensions/*/extension.toml crates/agentstacks*/Cargo.toml; } > "$P/edges-output.txt" 2>&1

# P15: conformance pack against both official extensions
{ echo "## P15"; for tool in claude opencode; do T=$(mktemp -d); mkdir -p "$T/ext" "$T/packs"; cp -R "extensions/$tool" "$T/ext/$tool"; cp -R "$P/conformance" "$T/packs/conformance"
  echo "### $tool init"; run init --tool "./ext/$tool" --pack ./packs/conformance --yes; echo "### $tool plan"; run plan; rm -rf "$T"; done; } > "$P/conformance-output.txt" 2>&1

# P5: airsl test
T=$(mktemp -d); mkdir -p "$T/pass" "$T/fail" "$T/empty"
printf 'return { a = function() assert(1 == 1) end }\n' > "$T/pass/a_test.lua"
printf 'return { good = function() end, bad = function() assert(false, "boom") end }\n' > "$T/fail/b_test.lua"
{ echo "## P5 $(airsl --version)"; for d in pass fail empty missing; do echo "### $d"; airsl test "$T/$d" > "$T/o" 2> "$T/e" && rc=0 || rc=$?; echo "stdout:"; sed "s|$T|<tmp>|g" "$T/o"; echo "stderr:"; sed "s|$T|<tmp>|g" "$T/e"; echo "exit=$rc"; done; } > "$P/airsl-test-output.txt"; rm -rf "$T"
