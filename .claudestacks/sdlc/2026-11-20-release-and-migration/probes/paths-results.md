# Paths: claudevs fixtures, sdlc chain paths, journal vault root (2026-11-24)

```text
$ ls crates/claudevs/tests/fixtures | grep -E "^(exec-args|dead-script|escape|bad-matcher|hooks-array|minimal|project-branch|matcher-routing)-plugin$"
bad-matcher-plugin
dead-script-plugin
escape-plugin
exec-args-plugin
hooks-array-plugin
matcher-routing-plugin
minimal-plugin
project-branch-plugin
$ grep -rhno "\.claudestacks/sdlc[a-z/]*" packs/sdlc/skills/*/SKILL.md | sort -u
10:.claudestacks/sdlc/
10:.claudestacks/sdlc/adrs/
11:.claudestacks/sdlc/prds/
12:.claudestacks/sdlc/rfcs/
14:.claudestacks/sdlc/
15:.claudestacks/sdlc/
15:.claudestacks/sdlc/roadmaps/
18:.claudestacks/sdlc/
20:.claudestacks/sdlc/roadmaps/
22:.claudestacks/sdlc/roadmaps/
23:.claudestacks/sdlc/
3:.claudestacks/sdlc/
3:.claudestacks/sdlc/roadmaps/
50:.claudestacks/sdlc/
73:.claudestacks/sdlc/roadmaps/
87:.claudestacks/sdlc/adrs/
$ (same grep over the claudestacks-sdlc plugin skills)
10:.claudestacks/sdlc/
10:.claudestacks/sdlc/adrs/
11:.claudestacks/sdlc/prds/
12:.claudestacks/sdlc/rfcs/
14:.claudestacks/sdlc/
15:.claudestacks/sdlc/
15:.claudestacks/sdlc/roadmaps/
18:.claudestacks/sdlc/
20:.claudestacks/sdlc/roadmaps/
22:.claudestacks/sdlc/roadmaps/
23:.claudestacks/sdlc/
3:.claudestacks/sdlc/
3:.claudestacks/sdlc/roadmaps/
50:.claudestacks/sdlc/
73:.claudestacks/sdlc/roadmaps/
87:.claudestacks/sdlc/adrs/
$ diff <vault root function in pack> <the same in plugin>
identical:
-- The vault root: `$AIRSSTACK_HOME/journal`, defaulting to `~/.airsstack/journal`.
function M.root()
  local home = airsstack.env.get("AIRSSTACK_HOME")
  if not home or home == "" then
    home = path.join(airsstack.env.get("HOME") or "", ".airsstack")
  end
  return path.join(home, "journal")
end
```
