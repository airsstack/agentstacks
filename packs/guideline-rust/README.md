# guideline-rust

The `guideline-rust` pack: Rust engineering rules and a Definition of Done in one skill, rendered as
`/guideline-rust-guidelines`. The `core` pack's enforcement hook points the agent at it when it reads
or edits a Rust file or `Cargo.toml` in a project with a `Cargo.toml`, and the `core` pack's `coder`
and `reviewer` agents run its Definition of Done. The pack has no hooks and no scripts.

## Install

```text
agentstacks init --tool claude --pack guideline-rust
```

Add the `core` pack as well: its dispatcher is what points the agent at this skill, and its agents
are what run the Definition of Done. On its own the skill still loads on request.

## Documentation

The gate and the index of rules are in [`skills/guidelines/SKILL.md`](https://github.com/airsstack/agentstacks/blob/main/packs/guideline-rust/skills/guidelines/SKILL.md);
each rule is a file under [`skills/guidelines/references/`](https://github.com/airsstack/agentstacks/tree/main/packs/guideline-rust/skills/guidelines/references/).

## License

Apache-2.0. See [LICENSE](https://github.com/airsstack/agentstacks/blob/main/LICENSE).
