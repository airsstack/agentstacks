# The §6 smoke matrix jq program, exactly as in spec.md (`scripts/jq-probe.sh`, 2026-11-24)

```text
$ jq --version
jq-1.7.1-apple
--- program (from spec.md §6)
. as $all
| [.releases[] as $r
   | $r.artifacts[]
   | select($all.artifacts[.].kind == "executable-zip")
   | {archive: ., target: $all.artifacts[.].target_triples[0],
      app: $r.app_name, bin: ($r.app_name | sub("-cli$"; "")), version: $r.app_version,
      alpine: false}
     + ($runners[$all.artifacts[.].target_triples[0]] // {})]
--- runners (from claudestacks smoke workflow)
{"aarch64-apple-darwin":{"runner":"macos-15"},"x86_64-apple-darwin":{"runner":"macos-15-intel"},"x86_64-unknown-linux-gnu":{"runner":"ubuntu-22.04"},"aarch64-unknown-linux-gnu":{"runner":"ubuntu-22.04-arm"},"x86_64-unknown-linux-musl":{"runner":"ubuntu-22.04","alpine":true},"aarch64-unknown-linux-musl":{"runner":"ubuntu-22.04-arm","alpine":true}}
--- dist plan --tag=agentstacks-cli-v0.1.0 | jq -c --argjson runners "$runners" "$program"
[{"archive":"agentstacks-cli-aarch64-apple-darwin.tar.xz","target":"aarch64-apple-darwin","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0","alpine":false,"runner":"macos-15"},{"archive":"agentstacks-cli-aarch64-unknown-linux-gnu.tar.xz","target":"aarch64-unknown-linux-gnu","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0","alpine":false,"runner":"ubuntu-22.04-arm"},{"archive":"agentstacks-cli-aarch64-unknown-linux-musl.tar.xz","target":"aarch64-unknown-linux-musl","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0","alpine":true,"runner":"ubuntu-22.04-arm"},{"archive":"agentstacks-cli-x86_64-apple-darwin.tar.xz","target":"x86_64-apple-darwin","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0","alpine":false,"runner":"macos-15-intel"},{"archive":"agentstacks-cli-x86_64-unknown-linux-gnu.tar.xz","target":"x86_64-unknown-linux-gnu","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0","alpine":false,"runner":"ubuntu-22.04"},{"archive":"agentstacks-cli-x86_64-unknown-linux-musl.tar.xz","target":"x86_64-unknown-linux-musl","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0","alpine":true,"runner":"ubuntu-22.04"}]
jq exit=0
--- dist plan --tag=claudevs-cli-v0.1.2 | jq -c --argjson runners "$runners" "$program"
[{"archive":"claudevs-cli-aarch64-apple-darwin.tar.xz","target":"aarch64-apple-darwin","app":"claudevs-cli","bin":"claudevs","version":"0.1.2","alpine":false,"runner":"macos-15"},{"archive":"claudevs-cli-aarch64-unknown-linux-gnu.tar.xz","target":"aarch64-unknown-linux-gnu","app":"claudevs-cli","bin":"claudevs","version":"0.1.2","alpine":false,"runner":"ubuntu-22.04-arm"},{"archive":"claudevs-cli-aarch64-unknown-linux-musl.tar.xz","target":"aarch64-unknown-linux-musl","app":"claudevs-cli","bin":"claudevs","version":"0.1.2","alpine":true,"runner":"ubuntu-22.04-arm"},{"archive":"claudevs-cli-x86_64-apple-darwin.tar.xz","target":"x86_64-apple-darwin","app":"claudevs-cli","bin":"claudevs","version":"0.1.2","alpine":false,"runner":"macos-15-intel"},{"archive":"claudevs-cli-x86_64-unknown-linux-gnu.tar.xz","target":"x86_64-unknown-linux-gnu","app":"claudevs-cli","bin":"claudevs","version":"0.1.2","alpine":false,"runner":"ubuntu-22.04"},{"archive":"claudevs-cli-x86_64-unknown-linux-musl.tar.xz","target":"x86_64-unknown-linux-musl","app":"claudevs-cli","bin":"claudevs","version":"0.1.2","alpine":true,"runner":"ubuntu-22.04"}]
jq exit=0
```
