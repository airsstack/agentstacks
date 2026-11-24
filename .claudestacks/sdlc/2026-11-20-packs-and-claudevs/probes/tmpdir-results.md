# ${TMPDIR:-/tmp} inside a double-quoted hook argument

Run 2026-11-23 with /bin/sh. Output pasted verbatim.

~~~~text
$ env -u TMPDIR sh -c 'printf "[%s]
" "${TMPDIR:-/tmp}"'
[/tmp]
$ env TMPDIR=/var/x sh -c 'printf "[%s]
" "${TMPDIR:-/tmp}"'
[/var/x]
$ env -u TMPDIR sh -c 'printf "[%s]
" "$TMPDIR"'   # the var value today
[]
~~~~
