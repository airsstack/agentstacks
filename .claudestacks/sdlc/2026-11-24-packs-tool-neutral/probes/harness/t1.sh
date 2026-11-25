# usage: t1.sh <mode> <port>
SP=/private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad
D=$SP/tn/t1-$1
rm -rf "$D"; mkdir -p "$D"; cp -R "$SP/ptn/proj" "$D/proj"; cp "$SP/tn/scen1.json" "$D/scenario.json"
sh "$SP/tn/run.sh" "$D" "$2" "$1" "probe"
tail -1 "$D/out.txt"
cd "$D/proj"
for f in env.txt which.txt live.rc live-sa.rc; do echo "--- $f"; cat $f; done
echo "--- live.err"; head -c 400 live.err; echo
echo "--- live-sa.err"; head -c 400 live-sa.err; echo
wc -c live.json live-sa.json
