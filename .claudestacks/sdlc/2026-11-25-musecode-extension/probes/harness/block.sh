#!/bin/bash
cat >/dev/null; printf '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"BLOCKED_BY_HOOK"}}\n'
