#!/bin/sh
# Installs the agentstacks command-line tool, and airsl if it is not installed.
#
#   curl -LsSf https://airsstack.github.io/agentstacks/install.sh | sh
#   curl -LsSf https://airsstack.github.io/agentstacks/install.sh | AGENTSTACKS_VERSION=0.1.0 sh
#
# This is a pointer, not an installer. It finds a release of agentstacks, fetches
# the installer dist generated for it and runs that with the arguments given here,
# so the short URL never goes stale and what it installs is exactly what that
# release published. Every variable that installer reads (AGENTSTACKS_CLI_INSTALL_DIR,
# ...) passes through untouched. The packs' hooks run under the airsl binary and `plan`
# refuses without one, so when no airsl is installed the script then runs airsl's
# own pointer; an airsl that is already there is never replaced or upgraded.
#
# It is sh, not Lua, because it runs before airsl exists.
#
# Published to GitHub Pages from install/ by .github/workflows/pages.yml: a
# change merged here is live for everyone who runs the command above.

set -eu

repo="https://github.com/airsstack/agentstacks"
api="https://api.github.com/repos/airsstack/agentstacks/releases"
app="agentstacks-cli"
name="agentstacks"
airsl_pointer="https://airsstack.github.io/airsl/install.sh"

say() {
    printf 'agentstacks-install: %s\n' "$1" >&2
}

fail() {
    say "error: $1"
    exit 1
}

# curl first, wget otherwise: the same order dist's installer uses, so a machine
# that can run one can run the other. Both write to stdout and the shell writes
# the file, which also keeps a snap-confined curl out of the temp directory.
download() {
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSfL "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$1"
    else
        fail "neither curl nor wget is installed"
    fi
}

# agentstacks needs airsl, so make sure one is installed. It looks on PATH and in
# $HOME/.local/bin, where dist's installers put binaries and which the running
# shell may not have on PATH yet.
ensure_airsl() {
    if airsl_path=$(command -v airsl 2>/dev/null); then
        say "airsl found at $airsl_path; leaving it as it is"
        return 0
    fi
    if [ -x "$HOME/.local/bin/airsl" ]; then
        say "airsl found at $HOME/.local/bin/airsl; leaving it as it is"
        return 0
    fi

    say "agentstacks needs airsl, which is not installed; installing the latest airsl with $airsl_pointer"
    if ! download "$airsl_pointer" >"$tmp/airsl-install.sh" || ! sh "$tmp/airsl-install.sh"; then
        fail "agentstacks is installed, but installing airsl failed; install it with: curl -LsSf $airsl_pointer | sh"
    fi
}

main() {
    # Nothing else runs first, so a machine with no downloader sees only this message.
    if ! command -v curl >/dev/null 2>&1 && ! command -v wget >/dev/null 2>&1; then
        fail "neither curl nor wget is installed"
    fi

    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT

    version="${AGENTSTACKS_VERSION:-latest}"
    version="${version#v}"

    case "$version" in
        latest)
            say "finding the latest $name release"
            if ! download "$api?per_page=100" >"$tmp/releases.json"; then
                fail "could not list the releases at $api (GitHub allows 60 unauthenticated requests an hour); set AGENTSTACKS_VERSION to a version to skip this lookup"
            fi
            # The newest stable release, whatever order the API lists them in:
            # a pre-release such as 0.2.0-rc.1 never matches the pattern.
            version=$(grep -o "\"tag_name\": *\"$app-v[0-9]*\.[0-9]*\.[0-9]*\"" "$tmp/releases.json" |
                sed "s/.*\"$app-v\([0-9.]*\)\"/\1/" |
                sort -t. -k1,1n -k2,2n -k3,3n |
                tail -n 1)
            if [ -z "$version" ]; then
                fail "found no $name release at $repo/releases"
            fi
            ;;
        *[!0-9A-Za-z.+-]* | "")
            fail "AGENTSTACKS_VERSION='${AGENTSTACKS_VERSION:-}' is not a version (expected MAJOR.MINOR.PATCH or latest)"
            ;;
        *) ;;
    esac

    say "fetching the installer for $name $version"
    if ! download "$repo/releases/download/$app-v$version/$app-installer.sh" >"$tmp/$app-installer.sh"; then
        fail "could not download the installer for $name $version; is $version a released version? See $repo/releases"
    fi

    status=0
    sh "$tmp/$app-installer.sh" "$@" || status=$?
    if [ "$status" -ne 0 ]; then
        exit "$status"
    fi

    ensure_airsl
}

# Nothing runs until the last line arrives, so a download cut off halfway
# through `curl | sh` defines functions and exits instead of running half a script.
main "$@"
