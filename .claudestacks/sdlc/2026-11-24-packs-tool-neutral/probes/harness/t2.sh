# opencode started by absolute path, not on PATH: can the session's shell run `opencode`?
SP=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad
D=$SP/tn/t2
rm -rf "$D"; mkdir -p "$D"; cp -R "$SP/ptn/proj" "$D/proj"; cp "$SP/tn/scen2.json" "$D/scenario.json"
sh "$SP/tn/run-abs.sh" "$D" 47313 service "probe"
tail -1 "$D/out.txt"
cd "$D/proj"
for f in env.txt path.txt which.txt which.rc live.rc live.err; do echo "--- $f"; cat $f; done
wc -c live.json
