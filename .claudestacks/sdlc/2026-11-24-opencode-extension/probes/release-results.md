# Probe results: OpenCode v2.4.1 release assets

Run 2026-11-25. `SHA256SUMS` (fetched with `gh release download v2.4.1 -R anomalyco/opencode -p SHA256SUMS`) is saved beside this file as `opencode-v2.4.1-SHA256SUMS`.

```text
$ curl -sI -o /dev/null -w '%{http_code}\n' https://github.com/anomalyco/opencode/releases/download/v2.4.1/opencode-linux-x64.tar.gz
302
$ curl -sI -o /dev/null -w '%{http_code}\n' https://github.com/anomalyco/opencode/releases/download/v2.4.1/opencode-darwin-arm64.zip
302
$ grep -E "opencode-(linux-x64.tar.gz|darwin-arm64.zip)$" opencode-v2.4.1-SHA256SUMS
95a101eda95108478f1db43cff8fcbd4811f6c0e79af1e167301843c6a48d731  opencode-darwin-arm64.zip
9ddd18e3faf64eaeccf6a0fcbc96a5c7a14520ae5fd5b61e5be51d894f478b47  opencode-linux-x64.tar.gz
$ ~/.opencode/bin/opencode --version
opencode v2.4.1
```

## Archive contents (downloaded with `gh release download v2.4.1 -R anomalyco/opencode -p <asset>`, listed, not run)

```text
$ shasum -a 256 opencode-linux-x64.tar.gz opencode-darwin-arm64.zip
9ddd18e3faf64eaeccf6a0fcbc96a5c7a14520ae5fd5b61e5be51d894f478b47  opencode-linux-x64.tar.gz
95a101eda95108478f1db43cff8fcbd4811f6c0e79af1e167301843c6a48d731  opencode-darwin-arm64.zip
$ tar -tvzf opencode-linux-x64.tar.gz
-rwxr-xr-x  0 0      0   275163312 Jan  1  1970 opencode
$ unzip -Z -l opencode-darwin-arm64.zip
Archive:  /private/tmp/claude-501/-Users-hiraq-Projects-airsstack-agentstacks/2c5e6598-abe6-4df1-bfda-1cd3d720d32e/scratchpad/arch/opencode-darwin-arm64.zip
Zip file size: 115188658 bytes, number of entries: 1
-rwxr-xr-x  3.0 unx 240032800 b- 115188544 defN 80-Jan-01 00:00 opencode
1 file, 240032800 bytes uncompressed, 115188544 bytes compressed:  52.0%
```
