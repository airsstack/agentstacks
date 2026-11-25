# usage: oc.sh <probe-dir> <opencode args...>  — runs the pinned opencode with that probe's isolated HOME
D=$1; shift
export HOME="$D/home" XDG_CONFIG_HOME="$D/home/.config" XDG_DATA_HOME="$D/home/.local/share" \
  XDG_CACHE_HOME="$D/home/.cache" XDG_STATE_HOME="$D/home/.local/state" \
  OPENCODE_DISABLE_MODELS_FETCH=1 OPENCODE_DISABLE_AUTOUPDATE=1
cd "$D/proj" 2>/dev/null || cd "$D"
"$D/bin/opencode" "$@"
echo "exit=$?"
