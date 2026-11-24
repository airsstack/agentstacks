## Filter pattern cheat sheet

You can use special characters in path, branch, tag, and workflow name filters.

* `*`: Matches zero or more characters, but does not match the `/` character. For example, `Octo*` matches `Octocat`.
* `**`: Matches zero or more of any character.
* `?`: Matches zero or one of the preceding character.
* `+`: Matches one or more of the preceding character.
* `[]` Matches one alphanumeric character listed in the brackets or included in ranges. Ranges can only include `a-z`, `A-Z`, and `0-9`. For example, the range`[0-9a-z]` matches any digit or lowercase letter. For example, `[CB]at` matches `Cat` or `Bat` and `[1-2]00` matches `100` and `200`.
* `!`: At the start of a pattern makes it negate previous positive patterns. It has no special meaning if not the first character.

The characters `*`, `[`, and `!` are special characters in YAML. If you start a pattern with `*`, `[`, or `!`, you must enclose the pattern in quotes. Also, if you use a [flow sequence](https://yaml.org/spec/1.2.2/#flow-sequences) with a pattern containing `[` and/or `]`, the pattern must be enclosed in quotes.

```yaml
# Valid
paths:
  - '**/README.md'
