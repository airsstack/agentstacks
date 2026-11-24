# Round-2 probes (`scripts/round2-probe.sh`, 2026-11-24)

Sections a-f match the script's `==` headers. Scratch paths are shown as <scratch>.

```text
== a. README example (official URL -> bare copy of HEAD tagged packs-v0.1.0, via insteadOf)
$ agentstacks init --tool claude --pack core --pack sdlc --pack guideline-rust < /dev/null
exit=1
76 to write, 0 to delete, 0 unchanged, 1 skipped
stdin is not a terminal; pass --yes to apply
agentstacks.toml written; nothing applied (run apply)
$ agentstacks plan | tail -1
exit=0
76 to write, 0 to delete, 0 unchanged, 1 skipped
$ agentstacks init --tool claude --pack sdlc --pack guideline-rust   # without core, fresh dir
exit=1
agentstacks.toml: packs.sdlc: sdlc requires pack "core", which agentstacks.toml does not list (add it: --pack core)
agentstacks.toml written; fix the above and run apply
== b. airsl's pointer script (/Users/hiraq/Projects/airsstack/airsl/install/install.sh), fake curl that always fails (exit 22)
$ AIRSL_VERSION='' PATH=<fakecurl>:/usr/bin:/bin /bin/sh install.sh
airsl-install: fetching the installer for the latest release
airsl-install: error: could not download https://github.com/airsstack/airsl/releases/latest/download/airsl-cli-installer.sh
$ AIRSL_VERSION='latest' PATH=<fakecurl>:/usr/bin:/bin /bin/sh install.sh
airsl-install: fetching the installer for the latest release
airsl-install: error: could not download https://github.com/airsstack/airsl/releases/latest/download/airsl-cli-installer.sh
$ AIRSL_VERSION='v0.1.5' PATH=<fakecurl>:/usr/bin:/bin /bin/sh install.sh
airsl-install: fetching the installer for airsl 0.1.5
airsl-install: error: could not download the installer for airsl 0.1.5; is 0.1.5 a released version? See https://github.com/airsstack/airsl/releases
$ AIRSL_VERSION='0.1.5' PATH=<fakecurl>:/usr/bin:/bin /bin/sh install.sh
airsl-install: fetching the installer for airsl 0.1.5
airsl-install: error: could not download the installer for airsl 0.1.5; is 0.1.5 a released version? See https://github.com/airsstack/airsl/releases
$ AIRSL_VERSION='0.1 5' PATH=<fakecurl>:/usr/bin:/bin /bin/sh install.sh
airsl-install: error: AIRSL_VERSION='0.1 5' is not a version (expected MAJOR.MINOR.PATCH or latest)
$ AIRSL_VERSION='a/b' PATH=<fakecurl>:/usr/bin:/bin /bin/sh install.sh
airsl-install: error: AIRSL_VERSION='a/b' is not a version (expected MAJOR.MINOR.PATCH or latest)
$ AIRSL_VERSION='v' PATH=<fakecurl>:/usr/bin:/bin /bin/sh install.sh
airsl-install: error: AIRSL_VERSION='v' is not a version (expected MAJOR.MINOR.PATCH or latest)
$ AIRSL_VERSION='0.2.0-rc.1' PATH=<fakecurl>:/usr/bin:/bin /bin/sh install.sh
airsl-install: fetching the installer for airsl 0.2.0-rc.1
airsl-install: error: could not download the installer for airsl 0.2.0-rc.1; is 0.2.0-rc.1 a released version? See https://github.com/airsstack/airsl/releases
$ PATH=/nonexistent /bin/sh install.sh   # no curl, no wget
/Users/hiraq/Projects/airsstack/airsl/install/install.sh: line 62: mktemp: command not found
exit=127
$ AIRSL_VERSION='' PATH=<fakecurl>:/usr/bin:/bin /bin/dash install.sh
airsl-install: fetching the installer for the latest release
airsl-install: error: could not download https://github.com/airsstack/airsl/releases/latest/download/airsl-cli-installer.sh
$ AIRSL_VERSION='latest' PATH=<fakecurl>:/usr/bin:/bin /bin/dash install.sh
airsl-install: fetching the installer for the latest release
airsl-install: error: could not download https://github.com/airsstack/airsl/releases/latest/download/airsl-cli-installer.sh
$ AIRSL_VERSION='v0.1.5' PATH=<fakecurl>:/usr/bin:/bin /bin/dash install.sh
airsl-install: fetching the installer for airsl 0.1.5
airsl-install: error: could not download the installer for airsl 0.1.5; is 0.1.5 a released version? See https://github.com/airsstack/airsl/releases
$ AIRSL_VERSION='0.1.5' PATH=<fakecurl>:/usr/bin:/bin /bin/dash install.sh
airsl-install: fetching the installer for airsl 0.1.5
airsl-install: error: could not download the installer for airsl 0.1.5; is 0.1.5 a released version? See https://github.com/airsstack/airsl/releases
$ AIRSL_VERSION='0.1 5' PATH=<fakecurl>:/usr/bin:/bin /bin/dash install.sh
airsl-install: error: AIRSL_VERSION='0.1 5' is not a version (expected MAJOR.MINOR.PATCH or latest)
$ AIRSL_VERSION='a/b' PATH=<fakecurl>:/usr/bin:/bin /bin/dash install.sh
airsl-install: error: AIRSL_VERSION='a/b' is not a version (expected MAJOR.MINOR.PATCH or latest)
$ AIRSL_VERSION='v' PATH=<fakecurl>:/usr/bin:/bin /bin/dash install.sh
airsl-install: error: AIRSL_VERSION='v' is not a version (expected MAJOR.MINOR.PATCH or latest)
$ AIRSL_VERSION='0.2.0-rc.1' PATH=<fakecurl>:/usr/bin:/bin /bin/dash install.sh
airsl-install: fetching the installer for airsl 0.2.0-rc.1
airsl-install: error: could not download the installer for airsl 0.2.0-rc.1; is 0.2.0-rc.1 a released version? See https://github.com/airsstack/airsl/releases
$ PATH=/nonexistent /bin/dash install.sh   # no curl, no wget
/Users/hiraq/Projects/airsstack/airsl/install/install.sh: 62: mktemp: not found
exit=127
== c. smoke matrix jq program on dist plan JSON
$ cat matrix.jq
[.releases[] as $r
 | $r.artifacts[]
 | select($all.artifacts[.].kind == "executable-zip")
 | {archive: ., target: $all.artifacts[.].target_triples[0],
    app: $r.app_name, bin: ($r.app_name | sub("-cli$"; "")), version: $r.app_version}]
$ dist plan --tag=agentstacks-cli-v0.1.0 --output-format=json | jq -c '. as $all | <matrix.jq>'
[{"archive":"agentstacks-cli-aarch64-apple-darwin.tar.xz","target":"aarch64-apple-darwin","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0"},{"archive":"agentstacks-cli-aarch64-unknown-linux-gnu.tar.xz","target":"aarch64-unknown-linux-gnu","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0"},{"archive":"agentstacks-cli-aarch64-unknown-linux-musl.tar.xz","target":"aarch64-unknown-linux-musl","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0"},{"archive":"agentstacks-cli-x86_64-apple-darwin.tar.xz","target":"x86_64-apple-darwin","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0"},{"archive":"agentstacks-cli-x86_64-unknown-linux-gnu.tar.xz","target":"x86_64-unknown-linux-gnu","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0"},{"archive":"agentstacks-cli-x86_64-unknown-linux-musl.tar.xz","target":"x86_64-unknown-linux-musl","app":"agentstacks-cli","bin":"agentstacks","version":"0.1.0"}]
$ dist plan --tag=claudevs-cli-v0.1.2 --output-format=json | jq -c '. as $all | <matrix.jq>'
[{"archive":"claudevs-cli-aarch64-apple-darwin.tar.xz","target":"aarch64-apple-darwin","app":"claudevs-cli","bin":"claudevs","version":"0.1.2"},{"archive":"claudevs-cli-aarch64-unknown-linux-gnu.tar.xz","target":"aarch64-unknown-linux-gnu","app":"claudevs-cli","bin":"claudevs","version":"0.1.2"},{"archive":"claudevs-cli-aarch64-unknown-linux-musl.tar.xz","target":"aarch64-unknown-linux-musl","app":"claudevs-cli","bin":"claudevs","version":"0.1.2"},{"archive":"claudevs-cli-x86_64-apple-darwin.tar.xz","target":"x86_64-apple-darwin","app":"claudevs-cli","bin":"claudevs","version":"0.1.2"},{"archive":"claudevs-cli-x86_64-unknown-linux-gnu.tar.xz","target":"x86_64-unknown-linux-gnu","app":"claudevs-cli","bin":"claudevs","version":"0.1.2"},{"archive":"claudevs-cli-x86_64-unknown-linux-musl.tar.xz","target":"x86_64-unknown-linux-musl","app":"claudevs-cli","bin":"claudevs","version":"0.1.2"}]
$ jq keys of .releases[0] and of one executable-zip artifact (tag agentstacks-cli-v0.1.0)
["app_name","app_version","artifacts","display","display_name","env","hosting"]
["source.tar.gz","source.tar.gz.sha256","agentstacks-cli-installer.sh","sha256.sum","agentstacks-cli-aarch64-apple-darwin.tar.xz","agentstacks-cli-aarch64-apple-darwin.tar.xz.sha256","agentstacks-cli-aarch64-unknown-linux-gnu.tar.xz","agentstacks-cli-aarch64-unknown-linux-gnu.tar.xz.sha256","agentstacks-cli-aarch64-unknown-linux-musl.tar.xz","agentstacks-cli-aarch64-unknown-linux-musl.tar.xz.sha256","agentstacks-cli-x86_64-apple-darwin.tar.xz","agentstacks-cli-x86_64-apple-darwin.tar.xz.sha256","agentstacks-cli-x86_64-unknown-linux-gnu.tar.xz","agentstacks-cli-x86_64-unknown-linux-gnu.tar.xz.sha256","agentstacks-cli-x86_64-unknown-linux-musl.tar.xz","agentstacks-cli-x86_64-unknown-linux-musl.tar.xz.sha256"]
{"name":"agentstacks-cli-x86_64-unknown-linux-musl.tar.xz","kind":"executable-zip","target_triples":["x86_64-unknown-linux-musl"],"checksum":"agentstacks-cli-x86_64-unknown-linux-musl.tar.xz.sha256"}
== d. dist build --artifacts=global: the generated shell installer's install dir
exit=0
agentstacks-cli-installer.sh
129:    \$HOME/.local/bin
696:    if [ -n "${AGENTSTACKS_CLI_INSTALL_DIR:-}" ]; then
697:        _force_install_dir="$AGENTSTACKS_CLI_INSTALL_DIR"
699:    elif [ -n "${CARGO_DIST_FORCE_INSTALL_DIR:-}" ]; then
700:        _force_install_dir="$CARGO_DIST_FORCE_INSTALL_DIR"
703:        _force_install_dir="$UNMANAGED_INSTALL"
724:                _install_dir="$_force_install_dir/bin"
725:                _lib_install_dir="$_force_install_dir/lib"
726:                _receipt_install_dir="$_force_install_dir"
732:                _install_dir="$_force_install_dir/bin"
733:                _lib_install_dir="$_force_install_dir/bin"
734:                _receipt_install_dir="$_force_install_dir"
== e. snapshot store path; vault.lua minus comment lines
$ grep -n "snapshots/" packs/core/skills/snapshot-save/SKILL.md (and the plugin)
36:${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/
91:Created on first use: make `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/` and an empty
36:${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/
94:Created on first use: make `${AIRSSTACK_HOME:-~/.airsstack}/snapshots/<project-key>/` and an empty
$ diff <pack vault.lua without comment lines> <plugin vault.lua without comment lines>
diff exit=0
== f. GitHub REST docs: order of List releases
$ sed -n 11,40p rest-releases.md | grep -n -i -E "sort|order|created_at"
grep exit=1
$ grep -n -i "sorted by" rest-releases.md   # control
379:The latest release is the most recent non-prerelease, non-draft release, sorted by the created_at attribute. The created_at attribute is the date of the commit used for the release, and not the date when the release was drafted or published.
```
