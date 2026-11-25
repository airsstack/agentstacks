# Probe results: pinning a Muse Code release for CI (C1) and OpenCode `.agents/skills` (C2)

Date: 2026-11-25. Host: macOS arm64 (Darwin 25.3.0). Every command below was run through a tiny
logger (`$P/scripts/run.sh <log> "<cmd>"`) that writes `$ <cmd>`, the raw combined stdout+stderr,
and `[exit N]`; each block is that log pasted byte for byte.

Paths: `$P` = `/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe`.
`$B` (the binary the launcher installed earlier this session) =
`/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1`.
Launcher source read: `probes/docs/launcher.sh` (`launcher_version="4"`).

~~~~sh
# $P/scripts/run.sh
#!/usr/bin/env bash
# usage: run.sh <logfile> <command string>; appends "$ cmd" + raw output + exit code
log="$1"; shift
{ printf '$ %s\n' "$*"; bash -c "$*" 2>&1; printf '[exit %s]\n\n' "$?"; } >>"$log"
~~~~

## C1.0 What the launcher does (source, `probes/docs/launcher.sh`)

- `launcher.sh:4-5`: `channel="${MUSE_CHANNEL:-muse-stable}"` / `channel_url="https://api.meta.ai/muse-code/channels/$channel"`; `launcher.sh:1095-1098` accepts only `muse-stable|muse-canary`.
- `launcher.sh:9`: `download_host="lookaside.facebook.com"`.
- `parse_channel_manifest` (`launcher.sh:334-359`) reads `channel` (must equal `$channel`), `version` (must match `version_pattern='^[0-9]+\.[0-9]+\.[0-9]+-R[0-9]+(\.[0-9]+)?$'`, `launcher.sh:18`), `manifest_url`, `urgency` (`none|encouraged|must_download`), `notification_text`; prints `version<TAB>manifest_url`.
- `parse_release_manifest` (`launcher.sh:361-394`) requires `version` == the channel version, `checksum_algorithm` == `sha256`, then `artifacts.<platform>.{url,checksum (64 hex),size (positive integer)}`.
- `detect_platform` (`launcher.sh:91-99`): `Darwin:arm64|Darwin:aarch64` → `aarch64_macos`, `Darwin:x86_64|Darwin:amd64` → `x86_macos`, `Linux:arm64|Linux:aarch64` → `aarch64_linux`, `Linux:x86_64|Linux:amd64` → `x86_linux`.
- `fetch_json_with_login` / `fetch_artifact_with_login` (`launcher.sh:770-806`): the first request is sent with `skip_download_auth=1` (no `Authorization` header); a bearer token is added and a device login started only when the answer is 401/403/404 (`is_auth_denial`, `launcher.sh:763-768`). `download_auth_header` only ever adds the header for `https://$download_host/` URLs (`launcher.sh:533-544`).
- `update_binary` (`launcher.sh:951-1033`): channel → release manifest → artifact into `$dir/.muse-update.XXXXXX/muse-bin`, checks `wc -c` == size and `sha256` == checksum, `chmod 0755`, moves it to `$dir/muse-bin-$version`, writes the channel JSON to `$dir/.muse-release-info.json` and the version to `$dir/.muse-version`.
- `main` (`launcher.sh:1093-1146`, the export at `1139-1144`): exports `MUSE_RELEASE_INFO` (the channel JSON) only when `.muse-release-info.json` matches `.muse-version`, else `unset MUSE_RELEASE_INFO`; then `exec "$binary" "$@"`. The download itself is the binary: no archive step anywhere in the launcher.

## C1.1 Channel manifests (stable and canary), no auth

~~~~text
$ curl -sS -w '\n[http %{http_code}]\n' https://api.meta.ai/muse-code/channels/muse-stable
{"channel":"muse-stable","version":"1.4.25-R9361.1","manifest_url":"https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=1.4.25-R9361.1&file=manifest.json","urgency":"none","notification_text":"","state":"public","min_version":null}
[http 200]
[exit 0]

$ curl -sS -w '\n[http %{http_code}]\n' https://api.meta.ai/muse-code/channels/muse-canary
{"channel":"muse-canary","version":"1.4.26-R9515.1","manifest_url":"https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=1.4.26-R9515.1&file=manifest.json","urgency":"none","notification_text":"","state":"canary","min_version":null}
[http 200]
[exit 0]

~~~~

Answer: both channel manifests are public (HTTP 200, no auth). The release-manifest URL is
version-specific: `https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=<VERSION>&file=manifest.json`,
where `<VERSION>` is the full `1.4.25-R9361.1` string (stable) and `1.4.26-R9515.1` (canary). The
`channel=muse` query value is the same for both channels; only `version=` differs. Both report
`min_version: null`.

## C1.2 Release manifests, no auth

The two `manifest_url` values from C1.1, fetched verbatim with plain `curl` (no header, no token):

~~~~text
$ curl -sS -w '\n[http %{http_code}]\n' 'https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=1.4.25-R9361.1&file=manifest.json'
{"version":"1.4.25-R9361.1","checksum_algorithm":"sha256","artifacts":{"x86_macos":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.25-R9361.1&file=muse-x86-macos","checksum":"fb02e24b14958ead92325550f4dfef914bb4c496b6c504588e90c1e56fb166b0","size":507643952},"aarch64_macos":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.25-R9361.1&file=muse-aarch64-macos","checksum":"d8faeec8b3a2507125d97f1a79ae18781d254e8cd376f26531587fb6e043c27d","size":478310160},"x86_linux":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.25-R9361.1&file=muse-x86-linux","checksum":"c1b728fb9732503c5e1869e3ad2f2c9bdb5b2638fb4cf398d6e2c46aa6d8f94a","size":468724056},"aarch64_linux":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.25-R9361.1&file=muse-aarch64-linux","checksum":"8fe089115ef137879fca46986d55edf18d5d3ab9cfdb6441c67e9d1807b4108a","size":415897832},"universal_macos_pkg":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.25-R9361.1&file=muse.pkg","checksum":"c08743f72832aea258ad8eef59566c91c0fc747cf2cf46d3b845ce74604d5765","size":376374836},"x86_windows":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.25-R9361.1&file=muse-x86-windows.exe","checksum":"eb03512b06982dd02481fcf5f090442824d92ff6070607afe5c7228596f3b089","size":413176056},"aarch64_windows":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.25-R9361.1&file=muse-aarch64-windows.exe","checksum":"8d325fc949a7e8c525e52526f2b84d5885b5c6c5b9cffea726cd4b8d0ae695ba","size":378382584}},"msp_schema_fingerprint":"sha256:6bf518067b065a503ea7d4f081a44fc056b532cf4b362a1894024863b25fb2aa"}
[http 200]
[exit 0]

$ curl -sS -w '\n[http %{http_code}]\n' 'https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=1.4.26-R9515.1&file=manifest.json'
{"version":"1.4.26-R9515.1","checksum_algorithm":"sha256","artifacts":{"x86_macos":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.26-R9515.1&file=muse-x86-macos","checksum":"a0aa2db2d77ffd122da9b67f15d5946c02de71fdcb61fc33ad31a25b7263659d","size":512575648},"aarch64_macos":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.26-R9515.1&file=muse-aarch64-macos","checksum":"4afff3a55998749e629da9ebc9d8cb80fc84b4019fd864dc7aebfc7667064424","size":481842512},"x86_linux":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.26-R9515.1&file=muse-x86-linux","checksum":"85f47f20c13472565b723d476ddb74bf04cca5ee92dce646d9ceb9187200b14f","size":471554392},"aarch64_linux":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.26-R9515.1&file=muse-aarch64-linux","checksum":"c4c4991947fee1f2fa72432174e5a51928f4db52a5d7dc63e5211cc6144d35ec","size":418912488},"universal_macos_pkg":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.26-R9515.1&file=muse.pkg","checksum":"d249eb11ccf65d0348e1beeb2e878838a9ec1ae2e356b956a3ded123f47fd002","size":379317204},"x86_windows":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.26-R9515.1&file=muse-x86-windows.exe","checksum":"db87fdd6a8a95843ac0a2ff77ff9890ae964c422946f6b0bfebd75a438376d29","size":415523064},"aarch64_windows":{"url":"https:\/\/lookaside.facebook.com\/lookaside\/muse\/download\/?channel=muse&version=1.4.26-R9515.1&file=muse-aarch64-windows.exe","checksum":"c084c5f79121c9986f674f9bb035ee7433fd978614fe5e05015a370dc7fb5ed6","size":380701432}},"msp_schema_fingerprint":"sha256:8f4061022347a35a74dd7280ee3871006f512750a0c9ddc4d762778e9deeca54"}
[http 200]
[exit 0]

~~~~

Answer: both release manifests are reachable without auth (HTTP 200). The artifact URL is
versioned and deterministic: `https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=<VERSION>&file=<FILE>`
with `<FILE>` = `muse-x86-linux`, `muse-aarch64-macos`, `muse-x86-macos`, `muse-aarch64-linux`,
`muse.pkg`, `muse-x86-windows.exe`, `muse-aarch64-windows.exe`. The 1.4.25-R9361.1 manifest (now one
version behind canary 1.4.26-R9515.1) is reachable. No version older than the current stable was
fetched: the saved changelog (`probes/docs/changelog.txt`) lists older versions only as `1.4.24`,
`1.4.23`, … without their `-R<build>` suffix, so no older manifest URL could be formed without
guessing; whether a manifest stays reachable after stable moves past it is unproven.

Pinned values for 1.4.25-R9361.1 (from the manifest above):

| platform | url `file=` | size | sha256 |
|---|---|---|---|
| `x86_linux` | `muse-x86-linux` | 468724056 | `c1b728fb9732503c5e1869e3ad2f2c9bdb5b2638fb4cf398d6e2c46aa6d8f94a` |
| `aarch64_macos` | `muse-aarch64-macos` | 478310160 | `d8faeec8b3a2507125d97f1a79ae18781d254e8cd376f26531587fb6e043c27d` |

## C1.3 `x86_linux` artifact: plain `curl -fsSL`, verify, inspect, delete

Run in its own empty directory `$P/dl-x86linux` (deleted afterwards). Response headers were filtered
to drop cookie/security/telemetry headers only.

~~~~text
$ curl -fsSL -D headers.txt -o muse-x86-linux -w '[http %{http_code}] [bytes %{size_download}] [redirects %{num_redirects}] [final %{url_effective}]\n' 'https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=1.4.25-R9361.1&file=muse-x86-linux'
[http 200] [bytes 468724056] [redirects 0] [final https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=1.4.25-R9361.1&file=muse-x86-linux]
[exit 0]

$ cat headers.txt | grep -iv -E '^(set-cookie|x-fb-|x-frame|cross-origin|report-to|nel|strict|content-security|permissions-policy|document-policy|alt-svc|x-xss|x-content)'
HTTP/2 200 
etag: -8708167905751218094
x-manifold-obj-blobsizebytes: 468724056
x-manifold-obj-canonicalpath: nodes/ca1e9bfc-30b9-43a1-91d0-8c9d14a69336/muse-x86-linux
x-manifold-obj-ctime: 1795288173
x-manifold-obj-entrytype: 1
x-manifold-obj-expiresat: 0
x-manifold-obj-isuserdata: 0
x-manifold-obj-propertiesjson: {}
x-manifold-obj-symlinktarget: 
accept-ranges: bytes
content-length: 468724056
date: Wed, 25 Nov 2026 09:30:15 GMT
cache-control: private, no-cache, no-store, must-revalidate
content-disposition: attachment;filename=muse-x86-linux
content-type: application/octet-stream
vary: Accept-Encoding

[exit 0]

$ wc -c < muse-x86-linux
 468724056
[exit 0]

$ shasum -a 256 muse-x86-linux
c1b728fb9732503c5e1869e3ad2f2c9bdb5b2638fb4cf398d6e2c46aa6d8f94a  muse-x86-linux
[exit 0]

$ file muse-x86-linux
muse-x86-linux: ELF 64-bit LSB executable, x86-64, version 1 (SYSV), statically linked, BuildID[sha1]=684d2278c5700ea13989bd13bd7fe6d6e4b7d175, stripped
[exit 0]

$ rm -f muse-x86-linux headers.txt && ls -la
total 0
drwxr-xr-x@ 2 hiraq  wheel   64 Nov 25 16:30 .
drwxr-xr-x@ 5 hiraq  wheel  160 Nov 25 16:30 ..
[exit 0]

~~~~

Answer: the `x86_linux` artifact downloads with plain `curl -fsSL` and no auth (HTTP 200, 0
redirects, `content-type: application/octet-stream`). Size 468724056 and sha256
`c1b728fb…d8f94a` both match the manifest. It is a raw executable, not an archive:
`ELF 64-bit LSB executable, x86-64, version 1 (SYSV), statically linked, … stripped`. Deleted.

## C1.4 `aarch64_macos` artifact: verify, compare with the launcher-installed binary, delete

Run in its own empty directory `$P/dl-armmac` (deleted afterwards).

~~~~text
$ curl -fsSL -o muse-aarch64-macos -w '[http %{http_code}] [bytes %{size_download}] [redirects %{num_redirects}] [content-type %{content_type}]\n' 'https://lookaside.facebook.com/lookaside/muse/download/?channel=muse&version=1.4.25-R9361.1&file=muse-aarch64-macos'
[http 200] [bytes 478310160] [redirects 0] [content-type application/octet-stream]
[exit 0]

$ wc -c < muse-aarch64-macos
 478310160
[exit 0]

$ shasum -a 256 muse-aarch64-macos /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1
d8faeec8b3a2507125d97f1a79ae18781d254e8cd376f26531587fb6e043c27d  muse-aarch64-macos
d8faeec8b3a2507125d97f1a79ae18781d254e8cd376f26531587fb6e043c27d  /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1
[exit 0]

$ file muse-aarch64-macos
muse-aarch64-macos: Mach-O 64-bit executable arm64
[exit 0]

$ cmp muse-aarch64-macos /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1 && echo identical
identical
[exit 0]

$ rm -f muse-aarch64-macos && ls -la
total 0
drwxr-xr-x@ 2 hiraq  wheel   64 Nov 25 16:31 .
drwxr-xr-x@ 6 hiraq  wheel  192 Nov 25 16:30 ..
[exit 0]

~~~~

Answer: the `aarch64_macos` artifact downloads with no auth (HTTP 200, 0 redirects). Size 478310160
and sha256 `d8faeec8…e043c27d` match the manifest, and it is byte-identical (`cmp`) to `$B`, the
binary the launcher installed. It is a raw `Mach-O 64-bit executable arm64`, not an archive (the
launcher's `chmod 0755` + rename is the whole install). Deleted.

## C1.5 Does the binary need the launcher at run time?

The installed binary was hard-linked alone into an empty directory (no `.muse-version`,
`.muse-release-info.json` or launcher beside it), run from another empty cwd with `env -i` (no
`MUSE_*` variable, only `HOME` set to an empty scratch dir and `PATH=/usr/bin:/bin`).

~~~~text
$ ln -f /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1 /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/lone/muse && ls -la /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/lone
total 934200
drwxr-xr-x@ 3 hiraq  wheel         96 Nov 25 16:31 .
drwxr-xr-x@ 9 hiraq  wheel        288 Nov 25 16:31 ..
-rwxr-xr-x@ 2 hiraq  wheel  478310160 Nov 25 16:16 muse
[exit 0]

$ env | grep -c '^MUSE_' || true
0
[exit 0]

$ cd /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/cwd && env -i HOME=/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/home PATH=/usr/bin:/bin /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/lone/muse --version
Muse Code 1.4.25 (1.4.25-R9361.1)
[exit 0]

$ find /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/home /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/cwd /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/lone -mindepth 1 | sed 's|/private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/||'
lone/muse
[exit 0]

$ strings -n 8 /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/lone/muse | grep -E 'MUSE_RELEASE_INFO|muse-release-info' | sort -u | head -20
[exit 0]

$ rm -f /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/lone/muse && rm -rf /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/lone /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/cwd /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/home /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/dl-x86linux /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/dl-armmac
[exit 0]

~~~~

Then a search of the binary for the env names it contains, and for the launcher's release-info
names (`grep -c` exits 1 when the count is 0):

~~~~text
$ LC_ALL=C grep -a -o -E 'MUSE_[A-Z_]{3,40}' /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1 | sort | uniq -c | sort -rn | head -40
   8 MUSE_EXPERIMENTAL_TUI_MSP_CLIENT
   8 MUSE_CUA_DEV_POLICY
   6 MUSE_CUA_DEV_SKIP_IDENTITY
   3 MUSE_WEB_SEARCH_MODE
   3 MUSE_ENABLE_WEB_TOOLS
   3 MUSE_ATTACH_BRIDGED
   2 MUSE_SESSIONS
   2 MUSE_SAFETY_IDENTIFIER
   2 MUSE_PROJECTS_HOME
   2 MUSE_PERMISSION_PROFILE_FAIL_OPEN
   2 MUSE_MODEL
   2 MUSE_FLAGS
   2 MUSE_ENABLE_SESSION_MCP
   2 MUSE_CURRENT_SESSION_LOG
   2 MUSE_CUA_RESPONSIBILITY_REEXEC
   2 MUSE_CUA_OBSERVE_DIAGNOSTICS
   1 MUSE_WWW_ROUTINGMUSE_DAEMON_TMUX
   1 MUSE_WWW_ROUTING
   1 MUSE_WORKSPACES_ROOT
   1 MUSE_WEB_SEARCH_MODEY
   1 MUSE_VERSION
   1 MUSE_TRANSPORT_TRACE
   1 MUSE_TOOL_USE_ID
   1 MUSE_THREAD_ID
   1 MUSE_SESSION_IDMUSE_THREAD_IDTRACEPARENTTRACE
   1 MUSE_SESSION_IDMUSE_THREAD_IDM
   1 MUSE_SESSION_IDMUSE_THREAD_ID
   1 MUSE_SESSION_IDMCP
   1 MUSE_SERVICEMUSE_EXPERIMENTAL_DEVELOPER_INSTR
   1 MUSE_SERVE_WORKER_THREADS
   1 MUSE_SERVE_OWNER
   1 MUSE_SERVE_COMMAND_ADMISSION_CAPACITYMUSE_SER
   1 MUSE_PLUGIN_ROOT
   1 MUSE_PLUGIN_ID
   1 MUSE_PLUGIN_DATA_DIRCLAUDE_PLUGIN_ROOTPLUGIN_
   1 MUSE_PARENT_SESSION_THREAD
   1 MUSE_PARENT_SESSION_IDMUSE_PARENT_SESSION_PRO
   1 MUSE_MODELMUSE_CONTRIBUTOR_MODEMUSE_PERMISSIO
   1 MUSE_LANE_BACKEND
   1 MUSE_HUMAN_CONFIRMATION
[exit 0]

$ LC_ALL=C grep -a -c -F RELEASE_INFO /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1; LC_ALL=C grep -a -c -F release-info /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/bin/muse-bin-1.4.25-R9361.1
0
0
[exit 1]

~~~~

Answer: no. Run directly, alone in a directory, from another cwd, with an empty environment and
an empty `HOME`, `--version` prints `Muse Code 1.4.25 (1.4.25-R9361.1)` and exits 0, and it wrote
nothing into the empty `HOME` or cwd. The strings `RELEASE_INFO` and `release-info` do not appear
in the binary at all (count 0), so `MUSE_RELEASE_INFO` is not read as a literal name by this build;
only `--version` was exercised, so a session run without the launcher is not proven by this probe.
For CI: download the manifest URL for the pinned version, check size + sha256, `chmod +x`, run.
No login, no launcher, no archive extraction.

## C2. Does OpenCode v2.4.1 read `.agents/skills/`?

Sources: the v2.4.1 docs and source saved by the previous chain under
`$O` = `/Users/hiraq/.herdr/worktrees/agentstacks/extension-musecode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes`
(`docs/`, `src/`; `src/agentstacks/` is a copy of this repo's book and is excluded where noted).
All greps ran with cwd `$O`. A runtime confirmation follows, using the previous chain's harness
(`$O/harness/run.sh`: isolated HOME/XDG, `opencode run --standalone --auto --format json`, mock
model) with the installed `opencode v2.4.1`; `opencode debug` has no skill-listing subcommand
(`agents`, `config`, `heap-dump`, `paths` only).

The probe projects were created in `$P/oc` and deleted after the outputs were recorded; the setup
script, extractor and outputs are below.

~~~~text
$ grep -rn -F '.agents/skills' docs src
docs/skills.mdx:47:| Global compatibility  | `~/.claude/skills`, `~/.agents/skills` |
docs/skills.mdx:49:| Project compatibility | `.claude/skills`, `.agents/skills`     |
docs/skills.mdx:178:3. `.agents/skills`, global first and then from the farthest ancestor toward the current directory
[exit 0]

$ grep -rn -F 'agents/skills' docs src
docs/skills.mdx:47:| Global compatibility  | `~/.claude/skills`, `~/.agents/skills` |
docs/skills.mdx:49:| Project compatibility | `.claude/skills`, `.agents/skills`     |
docs/skills.mdx:178:3. `.agents/skills`, global first and then from the farthest ancestor toward the current directory
[exit 0]

$ grep -rn -F '.agents' docs src | grep -v -F '.agents.' | grep -v '^src/agentstacks/'
docs/skills.mdx:47:| Global compatibility  | `~/.claude/skills`, `~/.agents/skills` |
docs/skills.mdx:49:| Project compatibility | `.claude/skills`, `.agents/skills`     |
docs/skills.mdx:178:3. `.agents/skills`, global first and then from the farthest ancestor toward the current directory
src/packages/core/src/config/plugin/agent.ts:75:        for (const [id, item] of Object.entries(document.info.agents ?? {})) {
src/packages/core/src/config/plugin/compatibility.ts:45:        const directories = [...roots.claude, ...roots.agents].map((root) => path.join(root, "skills"))
src/packages/core/src/config/plugin/compatibility.ts:84:const sourceDirectories = [".claude", ".agents"] as const
src/packages/core/src/config/plugin/compatibility.ts:92:    [...roots.claude, ...roots.agents].some((root) => FSUtil.contains(root, file))
src/packages/core/src/config/normalize.ts:157:    isRecord(input.agents)
src/packages/core/src/config/normalize.ts:158:      ? mapValues(input.agents, (agent) =>
src/packages/core/src/config/normalize.ts:163:      : input.agents,
src/packages/core/src/config/normalize.ts:176:    migratedSmallModel !== undefined || isRecord(input.agent) || isRecord(input.mode) || isRecord(input.agents),
src/packages/core/src/config/discovery.ts:80:  const globalAgentsDirectory = AbsolutePath.make(path.join(global.home, ".agents"))
src/packages/core/src/config/discovery.ts:93:  const candidates = [".claude", ".agents", ".opencode", ...names.toReversed()]
src/packages/core/src/config/discovery.ts:154:    direct: visible.filter((item) => ![".agents", ".claude", ".opencode"].includes(path.basename(item))).toReversed(),
src/packages/core/src/config/discovery.ts:175:        ...visible.filter((item) => path.basename(item) === ".agents").toReversed(),
src/packages/core/src/config.ts:64:  // false skips the system and global config dirs, ~/.claude, and ~/.agents;
src/packages/core/src/config.ts:452:            !isDeepStrictEqual(sources.agents, next.sources.agents)
src/packages/core/src/config.ts:539:            agents: Effect.filter(sources.agents, fs.isDir),
src/packages/core/src/session/prompt.ts:48:      agents: request.input.agents?.slice(),
src/packages/core/src/session/prompt.ts:84:      agents: input.agents,
src/packages/tui/src/component/prompt/index.tsx:772:      ...(prompt.agents ?? []).map((part, index) => ({
src/packages/tui/src/component/prompt/index.tsx:820:    if (data?.agent) return { type: "agent", index: (prompt.agents ??= []).push(copy(data.agent)) - 1 }
src/packages/tui/src/component/prompt/index.tsx:861:            const part = draft.prompt.agents?.[ref.index]
src/packages/tui/src/component/prompt/index.tsx:893:        if (!same(agents, draft.prompt.agents)) draft.prompt.agents = agents
src/packages/tui/src/component/prompt/index.tsx:1334:          agents: entry.agents,
src/packages/tui/src/component/prompt/index.tsx:1375:          agents: entry.agents,
src/packages/tui/src/component/prompt/index.tsx:1528:      (store.prompt.agents?.length ?? 0) > 0
src/packages/tui/src/component/prompt/autocomplete.tsx:244:      const agents = (draft.agents ??= [])
src/packages/tui/src/component/prompt/autocomplete.tsx:552:        ? [...skillOptions(), ...referenceAliasesValue, ...agentsValue]
[exit 0]

$ grep -rn -F '.claude/skills' docs src
docs/skills.mdx:47:| Global compatibility  | `~/.claude/skills`, `~/.agents/skills` |
docs/skills.mdx:49:| Project compatibility | `.claude/skills`, `.agents/skills`     |
docs/skills.mdx:177:2. `.claude/skills`, global first and then from the farthest ancestor toward the current directory
src/agentstacks/book/src/ch06-01-extensions.md:155:        dir = ".claude/skills/" .. p.name .. "-" .. s,
src/agentstacks/book/src/ch06-01-extensions.md:222:    "skills": [ { "id": "core/skill/status", "name": "status", "dir": ".claude/skills/core-status",
src/agentstacks/book/src/ch05-00-plan-and-apply.md:117:| first apply; `.claude/skills/core-status/SKILL.md` absent | `create` |
src/agentstacks/book/src/ch05-00-plan-and-apply.md:118:| user edited that file after apply; pack unchanged | refused `.claude/skills/core-status/SKILL.md: changed since the last apply` |
[exit 0]

$ grep -rn -F '.claude' docs src | grep -v '^src/agentstacks/'
docs/migrate-v1.mdx:556:V2 keeps the V1 `CLAUDE.md` fallback: a directory without `AGENTS.md` uses its `CLAUDE.md`, and `~/.claude/CLAUDE.md`
docs/migrate-v1.mdx:558:`OPENCODE_DISABLE_CLAUDE_CODE_PROMPT`, which in V1 skipped only `~/.claude/CLAUDE.md`. Unlike V1, which stopped at the
docs/instructions.mdx:45:`~/.config/opencode/AGENTS.md`, it reads `~/.claude/CLAUDE.md`. A directory's `AGENTS.md` always wins over its
docs/instructions.mdx:49:`.claude` skills. The server reads both when it starts. For the background service, persist them with
docs/skills.mdx:47:| Global compatibility  | `~/.claude/skills`, `~/.agents/skills` |
docs/skills.mdx:49:| Project compatibility | `.claude/skills`, `.agents/skills`     |
docs/skills.mdx:177:2. `.claude/skills`, global first and then from the farthest ancestor toward the current directory
docs/claude-agent.mdx:93:CLAUDE_CONFIG_DIR="$HOME/.claude-2" claude auth login
docs/claude-agent.mdx:99:$env:CLAUDE_CONFIG_DIR = "$HOME\.claude-2"; claude auth login
docs/claude-agent.mdx:113:          "settings": { "configDir": "~/.claude-2" },
docs/claude-agent.mdx:127:Sign in to each extra account in its own folder. Your existing login in `~/.claude` can stay as it is:
docs/claude-agent.mdx:130:CLAUDE_CONFIG_DIR="$HOME/.claude-1" claude auth login
docs/claude-agent.mdx:140:      "settings": { "configDir": ["~/.claude", "~/.claude-1"] },
src/packages/core/src/config/plugin/compatibility.ts:45:        const directories = [...roots.claude, ...roots.agents].map((root) => path.join(root, "skills"))
src/packages/core/src/config/plugin/compatibility.ts:84:const sourceDirectories = [".claude", ".agents"] as const
src/packages/core/src/config/plugin/compatibility.ts:92:    [...roots.claude, ...roots.agents].some((root) => FSUtil.contains(root, file))
src/packages/core/src/config/discovery.ts:81:  const globalClaudeDirectory = AbsolutePath.make(path.join(global.home, ".claude"))
src/packages/core/src/config/discovery.ts:93:  const candidates = [".claude", ".agents", ".opencode", ...names.toReversed()]
src/packages/core/src/config/discovery.ts:154:    direct: visible.filter((item) => ![".agents", ".claude", ".opencode"].includes(path.basename(item))).toReversed(),
src/packages/core/src/config/discovery.ts:164:      options?.claude === false
src/packages/core/src/config/discovery.ts:169:              ...visible.filter((item) => path.basename(item) === ".claude").toReversed(),
src/packages/core/src/config.ts:64:  // false skips the system and global config dirs, ~/.claude, and ~/.agents;
src/packages/core/src/config.ts:68:  // false skips ~/.claude and project .claude dirs.
src/packages/core/src/config.ts:451:            !isDeepStrictEqual(sources.claude, next.sources.claude) ||
src/packages/core/src/config.ts:538:            claude: Effect.filter(sources.claude, fs.isDir),
src/packages/cli/src/server-process.ts:127:            // Config reads .claude only for skills; CLAUDE.md is gated under instructions.
[exit 0]

$ sed -n 60,70p src/packages/core/src/config.ts
}

export const Options = Schema.Struct({
  project: Schema.optional(Schema.Boolean),
  // false skips the system and global config dirs, ~/.claude, and ~/.agents;
  // wellknown, file, and content entries still load.
  global: Schema.optional(Schema.Boolean),
  system: Schema.optional(Schema.Union([Schema.Boolean, Schema.String])),
  // false skips ~/.claude and project .claude dirs.
  claude: Schema.optional(Schema.Boolean),
  file: Schema.optional(Schema.String),
[exit 0]

$ sed -n 79,93p src/packages/core/src/config/discovery.ts
  const globalDirectory = AbsolutePath.make(global.config)
  const globalAgentsDirectory = AbsolutePath.make(path.join(global.home, ".agents"))
  const globalClaudeDirectory = AbsolutePath.make(path.join(global.home, ".claude"))
  const configDirectories = [...(system ? [system] : []), globalDirectory]
  const globalRoots = yield* Effect.forEach(
    [...configDirectories, globalClaudeDirectory, globalAgentsDirectory],
    (item) => fs.resolve(item),
    { concurrency: "unbounded" },
  )
  const current = yield* fs.resolve(location.directory)
  const directories =
    globalRoots.slice(0, configDirectories.length).includes(current) || options?.project === false
      ? []
      : yield* fs.up({ targets: ["."], start: location.directory }).pipe(Effect.orDie)
  const candidates = [".claude", ".agents", ".opencode", ...names.toReversed()]
[exit 0]

$ sed -n 163,178p src/packages/core/src/config/discovery.ts
    claude:
      options?.claude === false
        ? []
        : [
            ...new Set([
              ...(globalEnabled ? [globalClaudeDirectory] : []),
              ...visible.filter((item) => path.basename(item) === ".claude").toReversed(),
            ]),
          ],
    agents: [
      ...new Set([
        ...(globalEnabled ? [globalAgentsDirectory] : []),
        ...visible.filter((item) => path.basename(item) === ".agents").toReversed(),
      ]),
    ],
  } satisfies Sources
[exit 0]

$ sed -n 41,58p src/packages/core/src/config/plugin/compatibility.ts
    const refresh = Effect.fn("ConfigCompatibilityPlugin.refresh")(
      function* () {
        yield* FiberMap.clear(watches)
        const roots = yield* compatibility()
        const directories = [...roots.claude, ...roots.agents].map((root) => path.join(root, "skills"))
        const loaded = new Map<Skill.ID, Skill.Info>()
        for (const directory of directories) {
          const resolved = yield* fs.realPath(directory).pipe(Effect.orElseSucceed(() => undefined))
          if (!resolved) continue
          yield* watch(resolved, "directory")
          for (const filepath of yield* SkillScan.scan(resolved)) {
            const content = yield* fs.readFileStringSafe(filepath).pipe(Effect.orElseSucceed(() => undefined))
            if (!content) continue
            const parsed = SkillFile.parse(resolved, filepath, content)
            if (parsed._tag === "Parsed") loaded.set(parsed.skill.id, parsed.skill)
          }
        }
        skills.splice(0, skills.length, ...loaded.values())
[exit 0]

$ sed -n 77,80p src/packages/core/src/config/plugin/compatibility.ts
    yield* refresh()
    yield* ctx.skill.transform((editor) => {
      for (const skill of skills) editor.add(skill)
    })
[exit 0]

$ sed -n 128,129p src/packages/core/src/skill.ts
        add: (skill, files) => {
          editor.skills.set(skill.id, { ...skill })
[exit 0]

$ sed -n 40,50p docs/skills.mdx; sed -n 173,182p docs/skills.mdx
## Discovery

OpenCode automatically searches these locations:

| Scope                 | Sources                                |
| --------------------- | -------------------------------------- |
| Global                | `~/.config/opencode/skills`            |
| Global compatibility  | `~/.claude/skills`, `~/.agents/skills` |
| Project               | `.opencode/skills`                     |
| Project compatibility | `.claude/skills`, `.agents/skills`     |


Sources are registered from lower to higher precedence:

1. Built-in skills
2. `.claude/skills`, global first and then from the farthest ancestor toward the current directory
3. `.agents/skills`, global first and then from the farthest ancestor toward the current directory
4. `~/.config/opencode/skills`
5. Project `.opencode/skills`, from the project root toward the current directory
6. Explicit `skills` config entries, in config priority and array order

[exit 0]

$ ~/.opencode/bin/opencode --version
opencode v2.4.1
[exit 0]

$ cat /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/scripts/oc-setup.sh
#!/bin/sh
# C2 runtime probe setup. oc-a: dup in .opencode + .agents + .claude, plus agents-only.
# oc-b: dup in .agents + .claude only (no .opencode).
S=$1
mk() { mkdir -p "$S/$1/proj/$2/skills/$3"; printf -- "---\ndescription: %s from %s.\n---\n\n%s body from %s.\n" "$3" "$2" "$3" "$2" > "$S/$1/proj/$2/skills/$3/SKILL.md"; }
mk oc-a .opencode dup; mk oc-a .agents dup; mk oc-a .claude dup; mk oc-a .agents agents-only
mk oc-b .agents dup; mk oc-b .claude dup
for p in oc-a oc-b; do
  echo '[{"tool":"skill","args":{"id":"dup"}},{"tool":"skill","args":{"id":"agents-only"}},{"text":"ok"}]' > "$S/$p/scenario.json"
done
[exit 0]

$ cat /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/scripts/oc-x.js
// usage: node oc-x.js <probe-dir>
// Prints the <available_skills> block of the last tools-bearing request and every tool result.
const fs = require("fs");
const dir = process.argv[2];
const L = fs.readFileSync(dir + "/requests.jsonl", "utf8").trim().split("\n").map(JSON.parse);
const reqs = L.filter((r) => (r.body.tools || []).length > 0);
const last = reqs[reqs.length - 1].body;
const text = (c) => (typeof c === "string" ? c : JSON.stringify(c));
const sys = last.messages.filter((m) => m.role === "system").map((m) => text(m.content)).join("\n");
const a = sys.indexOf("<available_skills>");
const e = sys.indexOf("</available_skills>", a);
console.log("--- available_skills");
console.log(a >= 0 ? sys.slice(a, e + "</available_skills>".length) : "(none)");
for (const m of last.messages.filter((m) => m.role === "tool")) {
  console.log("--- tool result");
  console.log(text(m.content));
}
[exit 0]

$ cd /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc && find . -type f -path './oc-*/proj/*' | sort | while read f; do echo "## $f"; cat "$f"; done; cat oc-a/scenario.json
## ./oc-a/proj/.agents/skills/agents-only/SKILL.md
---
description: agents-only from .agents.
---

agents-only body from .agents.
## ./oc-a/proj/.agents/skills/dup/SKILL.md
---
description: dup from .agents.
---

dup body from .agents.
## ./oc-a/proj/.claude/skills/dup/SKILL.md
---
description: dup from .claude.
---

dup body from .claude.
## ./oc-a/proj/.opencode/skills/dup/SKILL.md
---
description: dup from .opencode.
---

dup body from .opencode.
## ./oc-b/proj/.agents/skills/dup/SKILL.md
---
description: dup from .agents.
---

dup body from .agents.
## ./oc-b/proj/.claude/skills/dup/SKILL.md
---
description: dup from .claude.
---

dup body from .claude.
[{"tool":"skill","args":{"id":"dup"}},{"tool":"skill","args":{"id":"agents-only"}},{"text":"ok"}]
[exit 0]

$ echo '(ran as:) sh /Users/hiraq/.herdr/worktrees/agentstacks/extension-musecode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-a 18761 "load dup" & sh /Users/hiraq/.herdr/worktrees/agentstacks/extension-musecode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-b 18762 "load dup" & wait'; tail -1 /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-a/out.txt; tail -1 /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-b/out.txt
(ran as:) sh /Users/hiraq/.herdr/worktrees/agentstacks/extension-musecode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-a 18761 "load dup" & sh /Users/hiraq/.herdr/worktrees/agentstacks/extension-musecode/.claudestacks/sdlc/2026-11-24-opencode-extension/probes/harness/run.sh /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-b 18762 "load dup" & wait
exit=0
exit=0
[exit 0]

$ node /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/scripts/oc-x.js /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-a
--- available_skills
<available_skills>
  <skill>
    <id>agents-only</id>
    <name>agents-only</name>
    <description>agents-only from .agents.</description>
  </skill>
  <skill>
    <id>dup</id>
    <name>dup</name>
    <description>dup from .opencode.</description>
  </skill>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
</available_skills>
--- tool result
<skill_content name="dup">
# Skill: dup

dup body from .opencode.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-a/proj/.opencode/skills/dup
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
--- tool result
<skill_content name="agents-only">
# Skill: agents-only

agents-only body from .agents.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-a/proj/.agents/skills/agents-only
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
[exit 0]

$ node /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/scripts/oc-x.js /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-b
--- available_skills
<available_skills>
  <skill>
    <id>dup</id>
    <name>dup</name>
    <description>dup from .agents.</description>
  </skill>
  <skill>
    <id>opencode</id>
    <name>OpenCode</name>
    <description>Use this skill for any question about OpenCode itself, including how OpenCode works, using or configuring it, migrating from V1 to V2, troubleshooting it, developing plugins or integrations, using the OpenCode SDK, clients, server, or API, and contributing to the OpenCode codebase. Also use it for OpenCode agents, commands, skills, tools, permissions, MCP servers, providers, models, themes, keybinds, formatters, the CLI, TUI, desktop app, and web app.</description>
  </skill>
  <skill>
    <id>report</id>
    <name>Report</name>
    <description>Use when the user wants to report an opencode issue or bug. Collect standard diagnostics, add user-specific reproduction context, and publish the issue with GitHub CLI.</description>
  </skill>
</available_skills>
--- tool result
<skill_content name="dup">
# Skill: dup

dup body from .agents.

Base directory for this skill: /private/tmp/claude-501/-Users-hiraq--herdr-worktrees-agentstacks-extension-musecode/08eae380-be82-415a-a612-6bf7f5a48a13/scratchpad/muse/relprobe/oc/oc-b/proj/.agents/skills/dup
Relative paths in this skill (e.g., scripts/, reference/) are relative to this base directory.
Note: file list is sampled.

<skill_files>
</skill_files>
</skill_content>
--- tool result
{"error":{"type":"tool.execution","message":"Unable to load skill agents-only"},"content":[]}
[exit 0]

~~~~

Answer: yes. OpenCode v2.4.1 reads project `.agents/skills/` and global `~/.agents/skills/` as
"compatibility" skill sources (`docs/skills.mdx:47,49`; `src/packages/core/src/config/discovery.ts:80,93,172-177`;
`src/packages/core/src/config/plugin/compatibility.ts:45,84`). The control grep finds `.claude` the
same way (`discovery.ts:81,93,163-171`, `compatibility.ts:45,84`). Precedence:
- `.opencode/skills/x` beats `.agents/skills/x`. The docs order (`docs/skills.mdx:173-180`, lower
  to higher): built-ins, `.claude/skills`, `.agents/skills`, `~/.config/opencode/skills`, project
  `.opencode/skills`, explicit config. The editor's `add` is a plain `Map.set`
  (`src/packages/core/src/skill.ts:128-129`), so the later registration wins. Runtime (oc-a): `dup`
  is listed once, with `dup from .opencode.`, and the `skill` call loads `dup body from .opencode.`
  from `.opencode/skills/dup`.
- `.agents/skills/x` beats `.claude/skills/x`. Inside the compatibility plugin the directories are
  `[...roots.claude, ...roots.agents]` and are loaded into one `Map` with `loaded.set(...)`
  (`compatibility.ts:45-55`), so `.agents` overwrites `.claude`. Runtime (oc-b): `dup from .agents.`
  wins.
- A skill only in `.agents/skills/` is listed and loadable (oc-a `agents-only`, base directory
  `.../proj/.agents/skills/agents-only`).
- Switches: `config.ts:68` says `claude: false` "skips ~/.claude and project .claude dirs" only; no
  separate switch for `.agents` appears in the saved `Options` (`config.ts:62-70`). `global: false` skips `~/.agents` (`config.ts:64`).
