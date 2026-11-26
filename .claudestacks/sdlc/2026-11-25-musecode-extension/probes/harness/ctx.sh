#!/bin/bash
cat >/dev/null; printf '{"hookSpecificOutput":{"hookEventName":"%s","additionalContext":"CTX_%s"}}\n' "$1" "$1"
