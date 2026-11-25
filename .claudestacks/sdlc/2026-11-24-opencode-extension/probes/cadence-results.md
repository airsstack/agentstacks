# Probe results: OpenCode release cadence

```text
$ gh api repos/anomalyco/opencode/releases --jq '.[0:6][] | "\(.tag_name) \(.published_at)"'
v2.4.1 2026-11-23T15:57:40Z
v2.4.0 2026-11-23T09:22:21Z
v2.3.2 2026-11-18T01:35:12Z
v2.3.1 2026-11-18T00:17:57Z
v1.19.6 2026-11-17T07:21:32Z
process-daemon-v0.6.0 2026-11-16T23:36:04Z
```
