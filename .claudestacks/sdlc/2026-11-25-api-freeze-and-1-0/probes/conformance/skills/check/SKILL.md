---
name: check
description: A conformance skill that uses every token kind
invocation: user
argument_hint: "[topic]"
---

Arguments: {{var arguments}}

Run {{skill check}} from {{skill_dir check}}. Ask {{agent checker}}. Files are under {{root}}.
Options: {{option label}}, {{option count}}, {{option flag}}.
Vars: {{var session_id}} {{var tmpdir}} {{var home}} {{var airsstack_home}} {{var project_dir}}.
A literal: \{{ not a token }}.
