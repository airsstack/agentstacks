# guideline-python

The `guideline-python` pack: Python engineering rules and a Definition of Done in one skill, rendered
as `/guideline-python-guidelines`. The `core` pack's enforcement hook points the agent at it when it
reads or edits a `.py` or `.pyi` file or `pyproject.toml` in a project with a `pyproject.toml`, and
the `core` pack's `coder` and `reviewer` agents run its Definition of Done. The pack has no hooks and
no scripts.

## Install

```text
agentstacks init --tool claude --pack guideline-python
```

Add the `core` pack as well: its dispatcher is what points Claude at this skill, and its agents are
what run the Definition of Done. On its own the skill still loads on request.

## Documentation

The gate and the index of rules are in [`skills/guidelines/SKILL.md`](skills/guidelines/SKILL.md);
each rule is a file under [`skills/guidelines/references/`](skills/guidelines/references/).

## License

Apache-2.0. See [LICENSE](../../LICENSE).
