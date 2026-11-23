# Plugin enforce --explain with no path

Run 2026-11-23 in /Users/hiraq/Projects/airsstack/claudestacks/plugins with the installed airsl. stdout and stderr are shown apart. Output pasted verbatim.

~~~~text
$ airsl --version
airsl 0.1.4
$ airsl run --policy confined --allow-read / claudestacks/hooks/enforce.lua --explain
exit=0 (stdout above)
usage: enforce.lua --explain <path>
(stderr above)
$ airsl run --policy confined --allow-read / claudestacks/hooks/enforce.lua --explain ''
usage: enforce.lua --explain <path>
exit=0
~~~~
