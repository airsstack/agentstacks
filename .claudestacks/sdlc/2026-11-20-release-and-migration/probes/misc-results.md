# Misc: crates.io categories, sh -n, claudevs-cli metadata (`misc-probe.sh`, 2026-11-24)

```text
$ crates.io categories
command-line-utilities: command-line-utilities
development-tools: development-tools
development-tools::testing: development-tools::testing
config: config
$ sh -n on a good and a broken script
good exit=0
<scratch>/bad.sh: line 4: syntax error near unexpected token `}'
<scratch>/bad.sh: line 4: `}'
bad exit=2
$ sh -n on the existing pointer scripts
airsl install.sh exit=0
claudestacks install.sh exit=0
$ claudevs-cli on crates.io: categories, keywords
['development-tools::testing', 'command-line-utilities'] ['claude', 'claude-code', 'cli', 'plugin', 'testing']
```
