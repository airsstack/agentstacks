# Claude Code documentation proof (docs-results)

Installed Claude Code: 2.1.330. Fetched: 2026-11-22 11:27 UTC (2026-11-22). Every quote below is copied by script (awk on the saved file), not retyped; format is `<file>:<line>: <text>`.

## Fetch commands

```sh
D=.claudestacks/sdlc/2026-11-20-cli-and-claude-extension/probes/docs
for p in skills sub-agents hooks settings slash-commands hooks-guide tools-reference settings-reference env-vars model-config; do
  curl -sS -L "https://code.claude.com/docs/en/$p.md" -o "$D/$p.md"; done   # all returned HTTP 200 from code.claude.com
```

`slash-commands.md` is byte-identical to `skills.md` (`cmp` reports no difference): the slash-commands page now serves the skills page.

## Byte sizes (wc -c)

```text
  194163 env-vars.md
   66685 hooks-guide.md
  294018 hooks.md
  134720 model-config.md
  460908 settings-reference.md
   45760 settings.md
  119789 skills.md
  119789 slash-commands.md
  115225 sub-agents.md
   97974 tools-reference.md
 1649031 total
```

## Skills

## S1 project skill location, invocation, same-name precedence

    skills.md:164: | Project | `.claude/skills/<skill-name>/SKILL.md` | Sessions in this repository. For a project's cloud threads, check [which threads load it](https://code.claude.com/docs/en/claude-projects.md#what-threads-pick-up-from-your-repositories). Commit it so your team gets it too |
    skills.md:181: Claude Code loads project skills from `.claude/skills/` in the directory where you start it and in every parent directory up to the repository root, so starting in `packages/frontend/` still picks up skills defined at the root. When you [move the session with `/cd`](https://code.claude.com/docs/en/permissions#move-the-session-to-another-directory) on v2.1.246 or later, Claude Code adds the new directory's project skills.
    skills.md:445: The command you type to invoke a skill comes from where the skill file lives and, for skill directories and plugin skills, from the frontmatter `name` field. In a personal or project skill directory, `name` sets the command that the `/` menu shows and that you type, unless another command already uses that name. The directory name also invokes the skill. In a plugin skill, `name` sets the last segment of the command and the plugin prefix stays in place.
    skills.md:449: | Skill location | Command name source | Example |
    skills.md:450: | :- | :- | :- |
    skills.md:451: | Skill directory under `~/.claude/skills/` or `.claude/skills/` | Frontmatter `name` or the directory name | `.claude/skills/deploy-staging/SKILL.md` → `/deploy-staging`, or `/deploy` with `name: deploy` |
    skills.md:455: | File in a subdirectory of `.claude/commands/` | Subdirectory path relative to `commands/` with each `/` replaced by `:`, then the file name without extension | `.claude/commands/frontend/component.md` → `/frontend:component` |
    skills.md:209: ### Resolve skills that share a name
    skills.md:211: When two skills share a directory or file name, where each one came from decides which one `/name` runs. For a name set by the frontmatter `name` field, see [How a skill gets its command name](#how-a-skill-gets-its-command-name).
    skills.md:213: | Same name in | Which one runs |
    skills.md:214: | :- | :- |
    skills.md:215: | Two of enterprise, personal, and project | Enterprise over personal, and personal over project. With `deploy` in both `~/.claude/skills/` and the project's `.claude/skills/`, `/deploy` runs the personal one |
    skills.md:216: | Any of those locations and a [bundled skill](#bundled-skills) | Your skill replaces the bundled command, but not its aliases. A project `code-review` skill replaces `/code-review`, and the bundled alias `/review` never runs your skill |
    skills.md:217: | Any of those locations and a [built-in command](https://code.claude.com/docs/en/commands.md) | In a local terminal session, your skill replaces the built-in command, but not its aliases. A project `usage` skill replaces `/usage`, and the built-in alias `/cost` still runs the built-in command |
    skills.md:218: | A skill and a file in `.claude/commands/` | The skill |
    skills.md:219: | A project-root skill and a nested skill | Both load. See [monorepos and subdirectories](#discovery-from-parent-and-nested-directories) |
    skills.md:220: | An enterprise, personal, or project skill and a skill in an [added directory outside the project](#skills-from-additional-directories) | The enterprise, personal, or project skill, while the added directory's skill runs under a prefixed name. See [run a same-named skill from an added directory](#run-a-same-named-skill-from-an-added-directory) |
    skills.md:221: | Two [added directories](#skills-from-additional-directories) outside the project | The skill from the directory added first, in `--add-dir` or `/add-dir` order. The other runs under a [prefixed name](#run-a-same-named-skill-from-an-added-directory) |
    skills.md:222: | A plugin skill and a skill at any of the locations above | Both load, because plugin skills are namespaced as `/plugin-name:skill-name` |


## S2 SKILL.md frontmatter keys (complete table)

Table spans skills.md:401-422 (`name`, `description`, `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`, `disallowed-tools`, `model`, `effort`, `context`, `agent`, `background`, `hooks`, `paths`, `shell`, `metadata`, `license`, `compatibility`). Command files in `.claude/commands/` accept all but `name` and `paths` (line 176).

    skills.md:401: | Field | Required | Description |
    skills.md:402: | :- | :- | :- |
    skills.md:403: | `name` | No | Command name shown in the `/` menu. Defaults to the directory name. See [How a skill gets its command name](#how-a-skill-gets-its-command-name) for how the field interacts with the name you type to invoke the skill. |
    skills.md:404: | `description` | Recommended | What the skill does and when to use it. Claude uses this to decide when to apply the skill. If omitted, uses the first non-empty line of the markdown content. Put the key use case first: the combined `description` and `when_to_use` text is truncated at 1,536 characters in the skill listing to reduce context usage. Angle brackets in the text [show as `&lt;` and `&gt;`](#angle-brackets-in-a-skill-description). |
    skills.md:405: | `when_to_use` | No | Additional context for when Claude should invoke the skill, such as trigger phrases or example requests. Appended to `description` in the skill listing and counts toward the 1,536-character cap. |
    skills.md:406: | `argument-hint` | No | Hint shown during autocomplete to indicate expected arguments. Example: `[issue-number]` or `[filename] [format]`. |
    skills.md:407: | `arguments` | No | Named positional arguments for [`$name` substitution](#available-string-substitutions) in the skill content. Accepts a space-separated string or a YAML list. Names map to argument positions in order. |
    skills.md:408: | `disable-model-invocation` | No | Set to `true` to prevent Claude from automatically loading this skill. Use for workflows you want to trigger manually with `/name`. Also prevents the skill from being [preloaded into subagents](https://code.claude.com/docs/en/sub-agents.md#preload-skills-into-subagents) and from running when a [session-scoped scheduled task](https://code.claude.com/docs/en/scheduled-tasks.md) fires with the skill as its prompt. Default: `false`. |
    skills.md:409: | `user-invocable` | No | Set to `false` when only Claude should invoke the skill: Claude Code hides it from the `/` menu and doesn't run it when you type `/name`. Use for background knowledge users shouldn't invoke directly. Default: `true`. |
    skills.md:410: | `allowed-tools` | No | Tools Claude can use without asking permission during the turn that invokes this skill. The grant clears when you send your next message. Accepts a space- or comma-separated string, or a YAML list. See [Pre-approve tools for a skill](#pre-approve-tools-for-a-skill). |
    skills.md:411: | `disallowed-tools` | No | Tools removed from Claude's available pool while this skill is active. Use for autonomous skills that should never call certain tools, such as `AskUserQuestion` for a background loop. Accepts a space- or comma-separated string, or a YAML list. The restriction clears when you send your next message. Like deny rules, the field can't remove [`EndConversation`](https://code.claude.com/docs/en/tools-reference#endconversation-tool-behavior) while any other tool remains. |
    skills.md:412: | `model` | No | Model to use when this skill is active. The override applies for the rest of the current turn and isn't saved to settings. The session model resumes when you send your next prompt. Accepts the same values as [`/model`](https://code.claude.com/docs/en/model-config), or `inherit` to keep the active model. A value excluded by your organization's [`availableModels`](https://code.claude.com/docs/en/model-config#restrict-model-selection) allowlist isn't used, and the session keeps its current model. In [auto mode](https://code.claude.com/docs/en/permission-modes.md#eliminate-prompts-with-auto-mode), and in [plan mode while the classifier reviews commands](https://code.claude.com/docs/en/permission-modes.md#analyze-before-you-edit-with-plan-mode), a model that auto mode doesn't support also isn't used, and the session keeps its current model. With `context: fork`, the value sets the [forked subagent's model](#run-skills-in-a-subagent) instead, and an excluded value follows the [same rules as a subagent model override](https://code.claude.com/docs/en/model-config.md#restrict-model-selection). |
    skills.md:413: | `effort` | No | [Effort level](https://code.claude.com/docs/en/model-config.md#adjust-effort-level) for the rest of the current turn. Overrides the session effort level, but not the [`CLAUDE_CODE_EFFORT_LEVEL`](https://code.claude.com/docs/en/env-vars#variables) environment variable. With `context: fork`, the value sets the forked subagent's effort level instead. When you omit it, the level comes from the [effort resolution order](https://code.claude.com/docs/en/model-config.md#adjust-effort-level). For a fork on another model, see [Effort level of subagents and teammates](https://code.claude.com/docs/en/model-config.md#effort-level-of-subagents-and-teammates). Options: `low`, `medium`, `high`, `xhigh`, `max`; available levels depend on the model. |
    skills.md:414: | `context` | No | Set to `fork` to run in a forked subagent context. See [Run skills in a subagent](#run-skills-in-a-subagent). |
    skills.md:415: | `agent` | No | Which subagent type to use when `context: fork` is set. |
    skills.md:416: | `background` | No | Only applies with `context: fork`. Set to `false` to wait for the forked subagent's result in the turn that invoked the skill, instead of [running it in the background](#run-skills-in-a-subagent). Default: `true`. |
    skills.md:417: | `hooks` | No | Hooks that Claude Code registers when the skill is invoked and keeps running for the rest of the session. See [Hooks in skills and agents](https://code.claude.com/docs/en/hooks.md#hooks-in-skills-and-agents) for the configuration format, the `once` option, and the [limits on skill hooks](https://code.claude.com/docs/en/hooks.md#limits-on-skill-hooks). |
    skills.md:418: | `paths` | No | Glob patterns for the files that activate this skill. Accepts a comma-separated string or a YAML list. Patterns are relative to Claude's current working directory. In the main conversation, they stay relative to the [primary working directory](https://code.claude.com/docs/en/permissions.md#working-directories) after a `cd` into one of its subdirectories in the same Git worktree. Patterns match only files inside the directory they're relative to. Uses the same glob format as [path-specific rules](https://code.claude.com/docs/en/memory.md#path-specific-rules). |
    skills.md:419: | `shell` | No | Shell to use for `` !`command` `` and ` ```! ` blocks in this skill. Accepts `bash` (default) or `powershell`. Setting `powershell` runs inline shell commands via PowerShell when the [PowerShell tool](https://code.claude.com/docs/en/tools-reference.md#powershell-tool) is enabled: it's on by default on Windows without Git Bash, on by default with Git Bash for claude.ai and Console accounts, and needs `CLAUDE_CODE_USE_POWERSHELL_TOOL=1` in Amazon Bedrock, Google Cloud's Agent Platform, and Microsoft Foundry sessions and on macOS, Linux, and WSL. Set it to `0` to turn the tool off. |
    skills.md:420: | `metadata` | No | Free-form YAML map for your own key-value data, such as entitlement or catalog fields, read by your own tooling from `SKILL.md`. Claude Code doesn't act on its contents, and drops a value that isn't a map. Don't reuse frontmatter field names such as `paths` as keys. |
    skills.md:421: | `license` | No | License covering the skill. Part of the [Agent Skills](https://agentskills.io) spec; see [Using skill frontmatter outside Claude Code](#using-skill-frontmatter-outside-claude-code). Claude Code accepts the field but doesn't act on it. |
    skills.md:422: | `compatibility` | No | Environment requirements for the skill, such as intended products or system prerequisites, as defined by the [Agent Skills](https://agentskills.io) spec; see [Using skill frontmatter outside Claude Code](#using-skill-frontmatter-outside-claude-code). Accepts a string of up to 500 characters. Claude Code accepts the field but doesn't act on it. |
    skills.md:176: * **Command files**: a Markdown file in `.claude/commands/` is the older format and still works. It supports the same [frontmatter](#frontmatter-reference) except `name` and `paths`. To find the name you type to invoke it, see [How a skill gets its command name](#how-a-skill-gets-its-command-name). Prefer a skill for new work, since skills also support [supporting files](#add-supporting-files).


## S3 string substitutions

    skills.md:487: | Variable | Description |
    skills.md:488: | :- | :- |
    skills.md:489: | `$ARGUMENTS` | All arguments passed when invoking the skill. When no placeholder receives an argument, Claude Code appends them as `ARGUMENTS: <value>`. See [Pass arguments to skills](#pass-arguments-to-skills). |
    skills.md:490: | `$ARGUMENTS[N]` | Access a specific argument by 0-based index, such as `$ARGUMENTS[0]` for the first argument. |
    skills.md:491: | `$N` | Shorthand for `$ARGUMENTS[N]`, such as `$0` for the first argument or `$1` for the second. |
    skills.md:492: | `$name` | Named argument declared in the [`arguments`](#frontmatter-reference) frontmatter list. Names map to positions in order, so with `arguments: [issue, branch]` the placeholder `$issue` expands to the first argument and `$branch` to the second. |
    skills.md:493: | `${CLAUDE_SESSION_ID}` | The current session ID. Useful for logging, creating session-specific files, or correlating skill output with sessions. |
    skills.md:494: | `${CLAUDE_EFFORT}` | The current effort level: `low`, `medium`, `high`, `xhigh`, or `max`. Use this to adapt skill instructions to the active effort setting. |
    skills.md:495: | `${CLAUDE_SKILL_DIR}` | The directory containing the skill's `SKILL.md` file. For plugin skills, this is the skill's subdirectory within the plugin, not the plugin root. Use this in bash injection commands to reference scripts or files bundled with the skill, regardless of the current working directory. |
    skills.md:496: | `${CLAUDE_PROJECT_DIR}` | The project root directory. This is the same path [hooks](https://code.claude.com/docs/en/hooks.md#reference-scripts-by-path) and MCP servers receive as `CLAUDE_PROJECT_DIR`. Use this to reference project-local scripts or files, such as `${CLAUDE_PROJECT_DIR}/.claude/hooks/helper.sh`, independent of where the skill is installed. |
    skills.md:497: | `${CLAUDE_PLUGIN_ROOT}` | The plugin's installation directory. Substituted only in plugin skills. Use this to reference scripts or files bundled anywhere in the plugin, including resources shared between the plugin's skills. See [plugin environment variables](https://code.claude.com/docs/en/plugins/manifest-reference.md#environment-variables). |
    skills.md:498: | `${CLAUDE_PLUGIN_DATA}` | The plugin's [persistent data directory](https://code.claude.com/docs/en/plugins/components.md#path-variables-and-persistent-data), which survives plugin updates. Substituted only in plugin skills. Use this to reference installed dependencies, generated files, or caches that must outlive an update. |
    skills.md:500: Claude Code substitutes `${CLAUDE_SKILL_DIR}` and `${CLAUDE_PROJECT_DIR}` in two places: the skill's markdown content, and Bash rules in the [`allowed-tools`](#frontmatter-reference) frontmatter. In a plugin skill, Claude Code substitutes `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}` in the same two places. Using the same variable in both places lets a skill run a bundled script without a permission prompt. The following skill shows the pattern:
    skills.md:516: An indexed placeholder with no corresponding argument, such as `$2` when only one argument was passed, stays in the content unchanged. A named placeholder from the [`arguments`](#frontmatter-reference) frontmatter with no matching argument expands to an empty string.
    skills.md:318: * In a terminal or VS Code session, Claude Code doesn't run [`!` commands](#inject-dynamic-context), doesn't attach the files that `@` references name, and doesn't substitute the `${CLAUDE_PROJECT_DIR}` and `${CLAUDE_SESSION_ID}` placeholders. The `@` references and both placeholders reach Claude as literal text. A `!` command line reaches Claude as literal text too, or as the `disableSkillShellExecution` placeholder when that setting is on. This handling requires Claude Code v2.1.228 or later.

## Subagents

## A1 location, frontmatter keys, model, effort, tools

    sub-agents.md:155: Store subagent files in different locations depending on scope. When multiple subagents share the same name, Claude Code uses the one from the higher-priority location.
    sub-agents.md:157: | Location | Scope | Priority | How to create |
    sub-agents.md:158: | :- | :- | :- | :- |
    sub-agents.md:159: | Managed settings | Organization-wide | 1 (highest) | Deployed via [managed settings](https://code.claude.com/docs/en/settings.md) |
    sub-agents.md:160: | `--agents` CLI flag | Current session | 2 | Pass JSON when launching Claude Code |
    sub-agents.md:161: | `.claude/agents/` | Current project | 3 | Ask Claude, or create the file manually |
    sub-agents.md:162: | `~/.claude/agents/` | All your projects | 4 | Ask Claude, or create the file manually |
    sub-agents.md:163: | Plugin's `agents/` directory | Where plugin is enabled | 5 (lowest) | Installed with [plugins](https://code.claude.com/docs/en/plugins/overview.md) |
    sub-agents.md:165: **Project subagents** (`.claude/agents/`) are ideal for subagents specific to a codebase. Check them into version control so your team can use and improve them collaboratively.
    sub-agents.md:167: Project subagents are discovered by walking up from the current working directory, so every `.claude/agents/` between there and the repository root is scanned. When more than one of these nested directories defines the same `name`, Claude Code uses the definition closest to the working directory.
    sub-agents.md:289: Configure a subagent with YAML [frontmatter](https://code.claude.com/docs/en/glossary.md#frontmatter) between `---` markers at the top of its file, and write its system prompt as Markdown after the closing `---`. Only `name` and `description` are required.
    sub-agents.md:296: | Field | Required | Description |
    sub-agents.md:297: | :- | :- | :- |
    sub-agents.md:298: | `name` | Yes | Unique identifier of at most 256 characters, such as `code-reviewer` or `reviewer-v2`. [Hooks](https://code.claude.com/docs/en/hooks.md#subagentstart) receive this value as `agent_type`. The filename doesn't have to match. Names can't contain `:`, which is reserved for [plugin-scoped identifiers](https://code.claude.com/docs/en/plugins/overview.md) such as `my-plugin:reviewer` |
    sub-agents.md:299: | `description` | Yes | When Claude should delegate to this subagent |
    sub-agents.md:300: | `tools` | No | [Tools](#available-tools) the subagent can use, as a comma-separated string such as `Read, Grep, Bash` or a YAML list. Inherits every tool available to subagents if omitted. If no entry in the list resolves to a tool, the subagent usually [fails to launch](https://code.claude.com/docs/en/errors.md#agent-would-be-spawned-with-zero-tools) with an error naming the entries. To preload Skills into context, use the `skills` field rather than listing `Skill` here |
    sub-agents.md:301: | `disallowedTools` | No | Tools to deny, removed from inherited or specified list. Same format as `tools`. An entry with a specifier, such as `Bash(git push *)`, still [removes the whole tool](#restrict-which-tools-a-subagent-can-use) |
    sub-agents.md:302: | `model` | No | [Model](#choose-a-model) to use: `sonnet`, `opus`, `haiku`, `fable`, a full model ID such as `claude-opus-5-5`, or `inherit`. When you omit it, Claude Code picks the model in the [subagent model order](#choose-a-model) |
    sub-agents.md:303: | `permissionMode` | No | [Permission mode](#permission-modes): `default`, `acceptEdits`, `auto`, `dontAsk`, `bypassPermissions`, `plan`, or `manual` as an alias for `default`. Ignored for [plugin subagents](#choose-the-subagent-scope) |
    sub-agents.md:304: | `maxTurns` | No | Maximum number of agentic turns before the subagent stops. When the subagent reaches the limit, Claude Code returns its output marked as partial, and Claude can [resume it](#resume-subagents) to continue. The partial marking requires Claude Code v2.1.246 or later |
    sub-agents.md:305: | `autoCompactWindow` | No | Lower the [auto-compact window](#subagent-auto-compact-window) for this subagent to a token count of `100000` or more. Requires Claude Code v2.1.296 or later |
    sub-agents.md:306: | `skills` | No | [Skills](https://code.claude.com/docs/en/skills.md) to preload into the subagent's context at startup. The full skill content is injected, not only the description. Subagents can still invoke unlisted project, user, and plugin skills through the Skill tool |
    sub-agents.md:307: | `mcpServers` | No | [MCP servers](https://code.claude.com/docs/en/mcp.md) available to this subagent. Each entry is either a server name referencing an already-configured server (e.g., `"slack"`) or an inline definition with the server name as key and a full [MCP server config](https://code.claude.com/docs/en/mcp.md#installing-mcp-servers) as value. Ignored for [plugin subagents](#choose-the-subagent-scope) |
    sub-agents.md:308: | `hooks` | No | [Lifecycle hooks](#define-hooks-for-subagents) scoped to this subagent. Ignored for [plugin subagents](#choose-the-subagent-scope) |
    sub-agents.md:309: | `memory` | No | [Persistent memory scope](#enable-persistent-memory): `user`, `project`, or `local`. Enables cross-session learning |
    sub-agents.md:310: | `background` | No | Set to `true` to keep this subagent in the background even when Claude asks to wait for it. By default, custom subagents Claude spawns from the main conversation already [start in the background](#run-subagents-in-foreground-or-background) |
    sub-agents.md:311: | `omitClaudeMd` | No | Set to `true` to start this subagent without the user, project, and local CLAUDE.md files in its [initial context](#what-loads-at-startup); [managed policy files](https://code.claude.com/docs/en/memory.md#how-claude-md-files-load) still load, except for [managed subagents](#choose-the-subagent-scope). Use it for subagents that take everything they need from the delegation prompt. Ignored when the agent runs as the main session agent via `--agent` or the `agent` setting. Requires Claude Code v2.1.271 or later |
    sub-agents.md:312: | `effort` | No | Effort level when this subagent is active. Overrides the session effort level, but not the [`CLAUDE_CODE_EFFORT_LEVEL`](https://code.claude.com/docs/en/env-vars#variables) environment variable. Options: `low`, `medium`, `high`, `xhigh`, `max`; available levels depend on the model. See [Choose an effort level](#choose-an-effort-level) |
    sub-agents.md:313: | `isolation` | No | Set to `worktree` to run the subagent in a temporary [git worktree](https://code.claude.com/docs/en/worktrees.md), giving it an isolated copy of the repository branched by default from your [default branch](https://code.claude.com/docs/en/worktrees.md#choose-the-base-branch) rather than the parent session's `HEAD`. The worktree is automatically cleaned up if the subagent makes no changes |
    sub-agents.md:314: | `color` | No | Display color for the subagent in the task list and transcript. Accepts `red`, `blue`, `green`, `yellow`, `purple`, `orange`, `pink`, or `cyan` |
    sub-agents.md:315: | `initialPrompt` | No | Auto-submitted as the first user turn when this agent runs as the main session agent (via `--agent` or the `agent` setting). [Commands](https://code.claude.com/docs/en/commands.md) and [skills](https://code.claude.com/docs/en/skills.md) are processed. Prepended to any user-provided prompt. Ignored for [plugin subagents](#choose-the-subagent-scope) |
    sub-agents.md:316: | `experimental` | No | Map of experimental options. Set its `cacheTtl` key to `5m` or `1h` to choose the [prompt cache lifetime](https://code.claude.com/docs/en/prompt-caching.md#choose-the-ttl-yourself) for this subagent's requests, at the frontmatter's place in the [cache lifetime precedence](https://code.claude.com/docs/en/prompt-caching.md#choose-the-ttl-yourself). Claude Code ignores any other value, ignores `1h` while your Claude subscription is using usage credits, and reads the field only from subagent files. Requires Claude Code v2.1.248 or later |
    sub-agents.md:347: ### Choose a model
    sub-agents.md:349: By default, a subagent you define runs on the same model as the conversation that starts it. To choose its model yourself, set the [`model`](#supported-frontmatter-fields) field in its definition to one of these values:
    sub-agents.md:351: * **Model alias**: `sonnet`, `opus`, `haiku`, or `fable`
    sub-agents.md:352: * **Full model ID**: a value the [`--model`](https://code.claude.com/docs/en/cli-reference#cli-flags) flag accepts, such as `claude-opus-5-5` or `claude-sonnet-5`
    sub-agents.md:353: * **`inherit`**: the main conversation's model, or the spawning subagent's model for a [nested subagent](#which-model-a-nested-subagent-runs-on)
    sub-agents.md:355: When more than one source sets a model, Claude Code uses the first in this order:
    sub-agents.md:357: 1. The per-invocation `model` parameter, which Claude can pass when it starts the subagent. On an Agent tool call, the parameter takes a model alias, not a full model ID or `inherit`.

Tool names (built-in tool table, tools-reference.md:23-75; selected rows) and the Glob/Grep default note:

    tools-reference.md:26: | `Bash` | Executes shell commands in your environment. See [Bash tool behavior](#bash-tool-behavior) | Yes |
    tools-reference.md:30: | `Edit` | Makes targeted edits to specific files. See [Edit tool behavior](#edit-tool-behavior) | Yes |
    tools-reference.md:36: | `Glob` | Finds files based on pattern matching. Absent by default on macOS, Linux, and WSL. See [Glob tool behavior](#glob-tool-behavior) | No |
    tools-reference.md:37: | `Grep` | Searches for patterns in file contents. Absent by default on macOS, Linux, and WSL. See [Grep tool behavior](#grep-tool-behavior) | No |
    tools-reference.md:48: | `Read` | Reads the contents of files. See [Read tool behavior](#read-tool-behavior) | No |
    tools-reference.md:59: | `Skill` | Executes a [skill](https://code.claude.com/docs/en/skills.md#control-who-invokes-a-skill) within the main conversation. Can prompt for skills that set `allowed-tools`, `disallowed-tools`, or `hooks` | Some skills |
    tools-reference.md:72: | `WebFetch` | Fetches content from a specified URL. See [WebFetch tool behavior](#webfetch-tool-behavior) | Yes |
    tools-reference.md:73: | `WebSearch` | Performs web searches. See [WebSearch tool behavior](#websearch-tool-behavior) | Yes |
    tools-reference.md:75: | `Write` | Creates or overwrites files. See [Write tool behavior](#write-tool-behavior) | Yes |
    tools-reference.md:324: The Glob tool finds files by name pattern. On Windows, it's part of the default tool set. On macOS, Linux, and WSL, Claude Code leaves Glob and [Grep](#grep-tool-behavior) out of the default tool set, and Claude searches with `find` and `grep` through the Bash tool instead. In Claude's shell those two commands run embedded versions of `bfs` and `ugrep`, and the searches reach your hooks and permission rules as `Bash` calls.
    tools-reference.md:446: ## NotebookEdit tool behavior
    tools-reference.md:448: NotebookEdit modifies a Jupyter notebook one cell at a time, targeting cells by their `cell_id`. It doesn't perform string replacement across the notebook the way [Edit](#edit-tool-behavior) does on plain files.

Model values: sub-agents.md:302 and :349-351. Effort values: sub-agents.md:312.

## Hooks

## H1 settings.json hooks schema and handler fields

    hooks.md:225: Hooks are defined in JSON settings files. The configuration has three levels of nesting:
    hooks.md:227: 1. Choose a [hook event](#hook-events) to respond to, like `PreToolUse` or `Stop`
    hooks.md:228: 2. Add a [matcher group](#matcher-patterns) to filter when it fires, like "for the Bash tool"
    hooks.md:229: 3. Define one or more [hook handlers](#hook-handler-fields) to run when matched
    hooks.md:321: ```json theme={null}
    hooks.md:322: {
    hooks.md:323:   "hooks": {
    hooks.md:324:     "PostToolUse": [
    hooks.md:325:       {
    hooks.md:326:         "matcher": "Edit|Write",
    hooks.md:327:         "hooks": [
    hooks.md:328:           {
    hooks.md:329:             "type": "command",
    hooks.md:330:             "command": "/path/to/lint-check.sh"
    hooks.md:331:           }
    hooks.md:332:         ]
    hooks.md:333:       }
    hooks.md:334:     ]
    hooks.md:335:   }
    hooks.md:336: }
    hooks.md:337: ```
    hooks.md:401: Each object in the inner `hooks` array is a hook handler: the shell command, HTTP endpoint, MCP tool, LLM prompt, or agent that runs when the matcher matches. There are five types:
    hooks.md:403: * **[Command hooks](#command-hook-fields)** (`type: "command"`): run a shell command. Your script receives the event's [JSON input](#hook-input-and-output) on stdin and communicates results back through exit codes and stdout.
    hooks.md:404: * **[HTTP hooks](#http-hook-fields)** (`type: "http"`): send the event's JSON input as an HTTP POST request to a URL. The endpoint communicates results back through the response body using the same [JSON output format](#json-output) as command hooks.
    hooks.md:405: * **[MCP tool hooks](#mcp-tool-hook-fields)** (`type: "mcp_tool"`): call a tool on a configured [MCP server](https://code.claude.com/docs/en/mcp.md). The tool's text output is treated like command-hook stdout.
    hooks.md:406: * **[Prompt hooks](#prompt-and-agent-hook-fields)** (`type: "prompt"`): send a prompt to a Claude model for single-turn evaluation. The model returns its decision as JSON. See [Prompt-based hooks](#prompt-based-hooks).
    hooks.md:407: * **[Agent hooks](#prompt-and-agent-hook-fields)** (`type: "agent"`): spawn a subagent that can use tools like Read, Grep, and Glob to verify conditions before returning a decision. Agent hooks are experimental and may change. See [Agent-based hooks](#agent-based-hooks).
    hooks.md:417: #### Common fields
    hooks.md:419: These fields apply to all hook types:
    hooks.md:421: | Field | Required | Description |
    hooks.md:422: | :- | :- | :- |
    hooks.md:423: | `type` | yes | `"command"`, `"http"`, `"mcp_tool"`, `"prompt"`, or `"agent"` |
    hooks.md:424: | `if` | no | [Permission rule syntax](https://code.claude.com/docs/en/permissions.md#permission-rule-syntax) to filter when this hook runs, such as `"Bash(git *)"` or `"Edit(*.ts)"`. The hook command only runs if the tool call matches the pattern. See the [Bash matching table](#bash-if-matching) for how Bash patterns evaluate against subcommands, `$()`, and backticks. Only evaluated on tool events: `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, and `PermissionDenied`. On other events, a hook with `if` set never runs |
    hooks.md:425: | `timeout` | no | Seconds before canceling. Claude Code doesn't enforce it on a command hook you run with [`async: true`](#run-hooks-in-the-background). Defaults: 600 for `command`, `http`, and `mcp_tool`; 30 for `prompt`; 60 for `agent`. Claude Code lowers the `command`, `http`, and `mcp_tool` default to 30 on [`UserPromptSubmit`](#userpromptsubmit), [`PreModelSwitch`](#premodelswitch), and [`PostModelSwitch`](#postmodelswitch), and to 10 on [`MessageDisplay`](#messagedisplay). [`SessionEnd`](#sessionend) hooks share a 1.5-second budget; if your settings set a longer per-hook `timeout`, Claude Code raises the budget to match, up to 60 seconds |
    hooks.md:426: | `statusMessage` | no | Custom spinner message displayed while the hook runs |
    hooks.md:427: | `once` | no | If `true`, Claude Code removes the hook after its first successful run. A run that fails, blocks with exit code 2, or times out leaves the hook in place, so it runs again on the next matching event. Only honored for hooks declared in [skill frontmatter](#hooks-in-skills-and-agents); ignored in settings files and agent frontmatter |
    hooks.md:447: #### Command hook fields
    hooks.md:449: In addition to the [common fields](#common-fields), command hooks accept these fields:
    hooks.md:451: | Field | Required | Description |
    hooks.md:452: | :- | :- | :- |
    hooks.md:453: | `command` | yes | Shell command to execute. With `args`, the executable to spawn directly. See [Exec form and shell form](#exec-form-and-shell-form) |
    hooks.md:454: | `args` | no | Argument list. When present, `command` is resolved as an executable and spawned directly with `args` as the argument vector, with no shell involved. See [Exec form and shell form](#exec-form-and-shell-form) |
    hooks.md:455: | `async` | no | If `true`, runs in the background without blocking. See [Run hooks in the background](#run-hooks-in-the-background) |
    hooks.md:456: | `asyncRewake` | no | If `true`, runs in the background and wakes Claude on exit code 2. The hook's stderr, or stdout if stderr is empty, is shown to Claude as a [system reminder](https://code.claude.com/docs/en/glossary.md#system-reminder) so it can react to a long-running background failure |
    hooks.md:457: | `shell` | no | Shell to use for this hook. Accepts `"bash"` or `"powershell"`. Defaults to `"bash"`, or to `"powershell"` on Windows when Git Bash isn't installed. Setting `"powershell"` runs the command via PowerShell on Windows. Does not require `CLAUDE_CODE_USE_POWERSHELL_TOOL` since hooks spawn PowerShell directly. Ignored when `args` is set |
    hooks.md:458: | `onFailure` | no | Whether the action that triggered the hook goes ahead when the hook fails: `"continue"` or `"block"`. See [Block the action when a hook fails](#block-the-action-when-a-hook-fails) for `"block"` and for leaving the field unset. Requires Claude Code v2.1.295 or later |
    hooks.md:3720: Before v2.1.268, `CLAUDE_CODE_SESSIONEND_HOOKS_TIMEOUT_MS` raised only the overall budget, and a hook without its own `timeout` was still canceled after 1.5 seconds.
    hooks.md:3722: ### Elicitation
    hooks.md:3724: Runs when an MCP server requests user input mid-task. By default, Claude Code shows an interactive dialog for the user to respond. Hooks can intercept this request and respond programmatically, skipping the dialog entirely.
    hooks.md:3726: For a complete hook with its settings entry and script, see [Answer a form request from a script](#answer-a-form-request-from-a-script).


## H2 matcher semantics and per-event matcher field

    hooks.md:277: The `matcher` field filters when hooks fire. How a matcher is evaluated depends on the characters it contains:
    hooks.md:279: | Matcher value | Evaluated as | Example |
    hooks.md:280: | :- | :- | :- |
    hooks.md:281: | `"*"`, `""`, or omitted | Match all | fires on every occurrence of the event |
    hooks.md:282: | Only letters, digits, `_`, `-`, spaces, `,`, and `\|` | Exact string, or list of exact strings separated by `\|` or `,` with optional surrounding whitespace | `Bash` matches the Bash tool, and on `PreToolUse` and `PermissionRequest` also [Monitor commands](#match-monitor-commands); `Edit\|Write` and `Edit, Write` each match either tool exactly; `code-reviewer` matches only that agent type |
    hooks.md:283: | Contains any other character | JavaScript regular expression, unanchored | `^Notebook` matches any tool whose name starts with `Notebook`; `mcp__memory__.*` matches every tool from the `memory` server |
    hooks.md:285: A matcher on the regular-expression path is tested with JavaScript's `RegExp.prototype.test`, which succeeds on a match anywhere in the value. `Edit.*` matches both `Edit` and `NotebookEdit`; wrap the pattern in `^` and `$`, as in `^Edit$`, when you need a whole-string match.
    hooks.md:287: `FileChanged` and `StopFailure` use a narrower exact-match set of letters, digits, `_`, and `|` only. A hyphen, space, or comma in a matcher for those two events keeps it on the regular-expression path, and only `|` separates alternatives. Every other event with matcher support in the table that follows accepts `|` or `,`.
    hooks.md:289: The `FileChanged` event doesn't follow these rules when building its watch list. See [FileChanged](#filechanged).
    hooks.md:291: Each event type matches on a different field:
    hooks.md:293: | Event | What the matcher filters | Example matcher values |
    hooks.md:294: | :- | :- | :- |
    hooks.md:295: | `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, `PermissionDenied` | tool name | `Bash`, `Edit\|Write`, `mcp__.*` |
    hooks.md:296: | `SessionStart` | how the session started | `startup`, `resume`, `clear`, `compact`, `fork` |
    hooks.md:297: | `Setup` | which CLI flag triggered setup | `init`, `maintenance` |
    hooks.md:298: | `SessionEnd` | why the session ended | `clear`, `resume`, `logout`, `prompt_input_exit`, `other` |
    hooks.md:299: | `Notification` | notification type | `permission_prompt`, `idle_prompt`, `auth_success`, `auth_storage_failure`, `elicitation_dialog`, `elicitation_url_dialog`, `elicitation_complete`, `elicitation_response`, `agent_needs_input`, `agent_completed`, `quota_auto_resume_fired`, `quota_auto_resume_stale`, `quota_auto_resume_disabled`, `plugin_notification` |
    hooks.md:300: | `SubagentStart` | agent type | `general-purpose`, `Explore`, `Plan`, custom agent names, or plugin-scoped names like `^my-plugin:reviewer$` |
    hooks.md:301: | `PreCompact`, `PostCompact` | what triggered compaction | `manual`, `auto` |
    hooks.md:302: | `PreModelSwitch`, `PostModelSwitch` | canonical name of the model the session switches to, as described under [PreModelSwitch](#premodelswitch) | `claude-opus-5`, `claude-opus-4-6\|claude-opus-5`, `.*opus.*` |
    hooks.md:303: | `SubagentStop` | agent type | same values as `SubagentStart` |
    hooks.md:304: | `ConfigChange` | configuration source | `user_settings`, `project_settings`, `local_settings`, `policy_settings`, `skills` |
    hooks.md:305: | `CwdChanged` | no matcher support | always fires on every occurrence |
    hooks.md:306: | `DirectoryAdded` | how the directory was added | `slash_command`, `register_repo_root` |
    hooks.md:307: | `FileChanged` | literal filenames to watch (see [FileChanged](#filechanged)) | `.envrc\|.env` |
    hooks.md:308: | `StopFailure` | error type | `rate_limit`, `authentication_failed`, `oauth_org_not_allowed`, `account_on_hold`, `billing_error`, `invalid_request`, `model_not_found`, `server_error`, `max_output_tokens`, `cloud_credential_error`, `unknown` |
    hooks.md:309: | `InstructionsLoaded` | load reason | `session_start`, `nested_traversal`, `path_glob_match`, `include`, `compact` |
    hooks.md:310: | `UserPromptExpansion` | command name | your skill or command names |
    hooks.md:311: | `Elicitation` | MCP server name | your configured MCP server names |
    hooks.md:312: | `ElicitationResult` | MCP server name | same values as `Elicitation` |
    hooks.md:313: | `UserPromptSubmit`, `PostToolBatch`, `Stop`, `TeammateIdle`, `TaskCreated`, `TaskCompleted`, `WorktreeCreate`, `WorktreeRemove`, `MessageDisplay` | no matcher support | always fires on every occurrence |
    hooks.md:317: For most events, Claude Code evaluates the matcher against a field from the [JSON input](#hook-input-and-output) it sends to your hook on stdin. For tool events, that field is `tool_name`. For `PreModelSwitch` and `PostModelSwitch`, Claude Code evaluates the matcher against the canonical name it derives from `to_model`, as described under [PreModelSwitch](#premodelswitch). Each [hook event](#hook-events) section lists the full set of matcher values and the input schema for that event.
    hooks.md:339: If you add a `matcher` field to an event without matcher support, it is silently ignored.
    hooks.md:2659: Runs when Claude spawns a subagent with the Agent tool, when Claude [resumes a subagent](https://code.claude.com/docs/en/sub-agents.md#resume-subagents), and each time an in-process [agent team](https://code.claude.com/docs/en/agent-teams.md) teammate handles a new message. Supports matchers to filter by agent type name. For built-in agents, this is the agent name like `general-purpose`, `Explore`, or `Plan`. For [custom subagents](https://code.claude.com/docs/en/sub-agents.md), this is the `name` field from the agent's frontmatter, not the filename.
    hooks.md:2661: For subagents shipped by a [plugin](https://code.claude.com/docs/en/plugins/overview.md), the agent type is the plugin-scoped identifier such as `my-plugin:reviewer`, not the bare frontmatter name. The colon places a plugin-scoped name on the regular-expression path, so anchor the matcher with `^` and `$` for an exact match: `^my-plugin:reviewer$`.
    hooks.md:2701: Runs when a Claude Code subagent has finished responding. Matches on agent type, same values as SubagentStart.
    hooks.md:2713: A `matcher` that names agent types doesn't match an empty `agent_type`. A hook whose matcher is omitted, `""`, or `"*"`, or is a regular expression that matches an empty string, runs for events with an empty `agent_type` too.


## H3 CLAUDE_PROJECT_DIR, working directory, shell

    hooks.md:413: Handlers run in the current directory with Claude Code's environment. If the current directory no longer exists, for example a worktree or temp directory that another shell deleted mid-session, Claude Code runs command hooks from the first of these that still exists: the directory the session started in, the project root, your home directory, or the system temp directory. Claude Code records a warning naming the fallback directory in the [debug log](#debug-hooks). For a worktree session that you start from the desktop app, see [What worktrees share with the main checkout](https://code.claude.com/docs/en/worktrees.md#what-worktrees-share-with-the-main-checkout).
    hooks.md:466: **Shell form** runs when `args` is absent. The `command` string is passed to a shell: `sh -c` on macOS and Linux, Git Bash on Windows, or PowerShell when Git Bash isn't installed. Set the `shell` field to choose explicitly. The shell tokenizes the string, expands variables, and interprets pipes, `&&`, redirects, and globs.
    hooks.md:491: Both forms support the same [path placeholders](#reference-scripts-by-path), and both export them as the environment variables `CLAUDE_PROJECT_DIR`, `CLAUDE_PLUGIN_ROOT`, and `CLAUDE_PLUGIN_DATA` on the spawned process, so a script can read `process.env.CLAUDE_PLUGIN_ROOT` regardless of how it was launched.
    hooks.md:610: Use these placeholders to reference hook scripts relative to the project or plugin root, regardless of the working directory when the hook runs:
    hooks.md:612: * `${CLAUDE_PROJECT_DIR}`: the project root where the session started. Claude Code also sets this variable in the environment of [stdio MCP servers](https://code.claude.com/docs/en/mcp.md#option-3-add-a-local-stdio-server) and plugin LSP servers.
    hooks.md:618: > **Worktrees are different.** If Claude enters a [worktree](https://code.claude.com/docs/en/worktrees.md) during the session, Claude Code keeps `${CLAUDE_PROJECT_DIR}` where it was and passes the worktree path to your hooks a different way:
    hooks.md:619: >
    hooks.md:620: > * **`${CLAUDE_PROJECT_DIR}` stays put**: it still points at the project root where the session started, so a command such as `${CLAUDE_PROJECT_DIR}/.claude/hooks/check-style.sh` still runs the script in the main checkout.
    hooks.md:621: > * **`cwd` follows Claude**: the `cwd` field in the hook's [input JSON](#common-input-fields) is the worktree root after Claude enters a worktree, and the new directory after Claude runs `cd`. Read it when a hook needs to know which directory Claude is working in.
    hooks.md:625: Prefer [exec form](#exec-form-and-shell-form) for any hook that references a path placeholder. In shell form, wrap each placeholder in double quotes.
    hooks.md:775: A hook process inherits the parent environment, apart from the `OTEL_*` exporter variables that Claude Code [removes from every subprocess it spawns](https://code.claude.com/docs/en/monitoring-usage.md#administrator-configuration) and, when [`CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`](https://code.claude.com/docs/en/env-vars#variables) is set to `1`, the variables it strips. In a session that [gets the HIPAA configuration](https://code.claude.com/docs/en/hipaa-setup.md#check-how-developers-sign-in-and-connect), Claude Code also [removes Anthropic credentials](https://code.claude.com/docs/en/hipaa-setup.md#anthropic-credentials-in-commands-hooks-and-mcp-servers) from the hook's environment.

`CLAUDE_PROJECT_DIR` is not listed in env-vars.md: `grep -n CLAUDE_PROJECT_DIR env-vars.md` returns nothing (control: `grep -n CLAUDE_CODE_EFFORT_LEVEL env-vars.md` finds lines). The hooks page is the authority.

    control grep count (env-vars.md, CLAUDE_CODE_EFFORT_LEVEL): 2


## H4 hook input (stdin JSON)

Common fields:

    hooks.md:743: Command hooks receive JSON data via stdin and communicate results through exit codes, stdout, and stderr. HTTP hooks receive the same JSON as the POST request body and communicate results through the HTTP response body. This section covers fields and behavior common to all events. Each event's section under [Hook events](#hook-events) includes its specific input schema and decision control options.
    hooks.md:753: | Field | Description |
    hooks.md:754: | :- | :- |
    hooks.md:755: | `session_id` | Current session identifier |
    hooks.md:756: | `prompt_id` | UUID identifying the user prompt currently being processed. Matches the [`prompt.id` attribute on OpenTelemetry events](https://code.claude.com/docs/en/monitoring-usage#event-correlation-attributes), so you can correlate hook output with telemetry for a single prompt. Absent until the first user input |
    hooks.md:757: | `transcript_path` | Path to conversation JSON. The transcript file is written asynchronously and may lag the in-memory conversation, so it may not yet include the current turn's most recent messages when a hook fires. Hooks that need the final assistant text of the current turn should use `last_assistant_message` on [Stop](#stop) and [SubagentStop](#subagentstop) instead of reading the transcript |
    hooks.md:758: | `cwd` | Current working directory when the hook is invoked |
    hooks.md:759: | `scratchpad_dir` | Path to the session's [scratchpad directory](https://code.claude.com/docs/en/claude-directory.md#session-scratchpad-directory), where Claude keeps temporary working files. Absent when the session has no scratchpad or the temp directory is unavailable. Requires Claude Code v2.1.257 or later |
    hooks.md:760: | `permission_mode` | Current [permission mode](https://code.claude.com/docs/en/permissions.md#permission-modes): `"default"`, `"plan"`, `"acceptEdits"`, `"auto"`, `"dontAsk"`, or `"bypassPermissions"`. The mode labeled **Manual** arrives as `"default"`, never as `"manual"`, so scripts that match `"default"` keep working. Not all events receive this field. Check the JSON example in each [hook event](#hook-events) section |
    hooks.md:761: | `effort` | Object whose `level` is the [effort level](https://code.claude.com/docs/en/model-config.md#adjust-effort-level) setting when the hook runs, including a skill's, subagent's, or `ultrathink` level: `"low"`, `"medium"`, `"high"`, `"xhigh"`, or `"max"`. Present for `PreToolUse`, `PostToolUse`, `Stop`, `SubagentStop`, and other events with a tool-use context, on models with effort. If the model doesn't support your level, `level` is the level that ran. A level you changed can show here before it [starts](https://code.claude.com/docs/en/model-config.md#when-a-new-effort-level-starts). `$CLAUDE_EFFORT` holds the same level. |
    hooks.md:762: | `hook_event_name` | Name of the event that fired |
    hooks.md:764: `agent_id` and `agent_type` tell your script which agent a hook fired in, such as a subagent, an [in-process teammate](https://code.claude.com/docs/en/agent-teams.md#choose-a-display-mode), or the agent you chose with `--agent`:
    hooks.md:766: | Field | Description |
    hooks.md:767: | :- | :- |
    hooks.md:768: | `agent_id` | Unique identifier for the subagent or in-process teammate the hook fires in. |
    hooks.md:769: | `agent_type` | Agent name (for example, `"Explore"` or `"security-reviewer"`). Present when the session uses `--agent` or the hook fires inside a subagent. For subagents, the subagent's type takes precedence over the session's `--agent` value. See [SubagentStart](#subagentstart) for the values custom and plugin subagents report and how to write a matcher against a plugin-scoped name. |
    hooks.md:777: For example, a `PreToolUse` hook for a Bash command receives this on stdin:
    hooks.md:779: ```json theme={null}
    hooks.md:780: {
    hooks.md:781:   "session_id": "abc123",
    hooks.md:782:   "prompt_id": "550e8400-e29b-41d4-a716-446655440000",
    hooks.md:783:   "transcript_path": "/home/user/.claude/projects/.../transcript.jsonl",
    hooks.md:784:   "cwd": "/home/user/my-project",
    hooks.md:785:   "scratchpad_dir": "/tmp/claude-1000/-home-user-my-project/abc123/scratchpad",
    hooks.md:786:   "permission_mode": "default",
    hooks.md:787:   "hook_event_name": "PreToolUse",
    hooks.md:788:   "tool_name": "Bash",
    hooks.md:789:   "tool_input": {
    hooks.md:790:     "command": "npm test",
    hooks.md:791:     "description": "Run test suite",
    hooks.md:792:     "timeout": 120000,
    hooks.md:793:     "run_in_background": false
    hooks.md:794:   },
    hooks.md:795:   "tool_use_id": "toolu_01ABC123..."
    hooks.md:796: }
    hooks.md:797: ```
    hooks.md:799: The `tool_name`, `tool_input`, and `tool_use_id` fields are event-specific. Each [hook event](#hook-events) section documents the additional fields for that event.

SessionStart:

    hooks.md:1289: SessionStart runs on every session, so keep these hooks fast. Only `type: "command"` and `type: "mcp_tool"` hooks are supported. See [MCP tool hook fields](#mcp-tool-hook-fields) for when `mcp_tool` hooks run.
    hooks.md:1291: The matcher value corresponds to how the session was initiated:
    hooks.md:1293: | Matcher | When it fires |
    hooks.md:1294: | :- | :- |
    hooks.md:1295: | `startup` | New session |
    hooks.md:1296: | `resume` | `--resume`, `--continue`, or `/resume` |
    hooks.md:1297: | `clear` | `/clear` |
    hooks.md:1298: | `compact` | Auto or manual compaction |
    hooks.md:1299: | `fork` | A new session forked from an existing one: `--fork-session` with `--resume` or `--continue`, the `/fork` background copy, `/branch`, or a conversation you [move to the background](https://code.claude.com/docs/en/agent-view.md#from-inside-a-session) |
    hooks.md:1311: In addition to the [common input fields](#common-input-fields), SessionStart hooks receive `source` and optionally `model`, `agent_type`, and `session_title`:
    hooks.md:1313: | Field | Description |
    hooks.md:1314: | :- | :- |
    hooks.md:1315: | `source` | How the session started: `"startup"` for new sessions, `"resume"` for resumed sessions, `"clear"` after `/clear`, `"compact"` after compaction, or `"fork"` for a new session forked from an existing one |
    hooks.md:1316: | `model` | The active model identifier. It can be omitted, for example after `/clear` or when a session is restored through conversation recovery, so check for the field before reading it |
    hooks.md:1317: | `agent_type` | The agent name, present when you start Claude Code with `claude --agent <name>` |
    hooks.md:1318: | `session_title` | The session's custom title, present when one is set, for example with `--name`, `/rename`, a hook's `sessionTitle` output, or the Agent SDK's `renameSession()`. A hook that emits `sessionTitle` can check this field first to avoid overwriting an existing custom title |

SessionEnd:

    hooks.md:3682: The `reason` field in the hook input indicates why the session ended:
    hooks.md:3684: | Reason | Description |
    hooks.md:3685: | :- | :- |
    hooks.md:3686: | `clear` | Session cleared with `/clear` command |
    hooks.md:3687: | `resume` | Session switched via interactive `/resume` |
    hooks.md:3688: | `logout` | User logged out |
    hooks.md:3689: | `prompt_input_exit` | User exited while prompt input was visible |
    hooks.md:3690: | `other` | Other exit reasons |
    hooks.md:3691: | `bypass_permissions_disabled` | Removed in v2.1.234; Claude Code doesn't send it. Drop it from your `SessionEnd` matchers |
    hooks.md:3695: In addition to the [common input fields](#common-input-fields), SessionEnd hooks receive a `reason` field indicating why the session ended. See the [reason table](#sessionend) above for all values.
    hooks.md:3697: ```json theme={null}
    hooks.md:3698: {
    hooks.md:3699:   "session_id": "abc123",
    hooks.md:3700:   "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
    hooks.md:3701:   "cwd": "/Users/...",
    hooks.md:3702:   "hook_event_name": "SessionEnd",
    hooks.md:3703:   "reason": "other"
    hooks.md:3704: }
    hooks.md:3705: ```

UserPromptSubmit:

    hooks.md:1548: In addition to the [common input fields](#common-input-fields), UserPromptSubmit hooks receive the `prompt` field containing the submitted text. Pasted content that collapsed to a `[Pasted text #N]` placeholder arrives expanded in place. In sessions where Claude Code [marks pasted text for Claude](https://code.claude.com/docs/en/terminal-config.md#how-claude-treats-pasted-text), that expanded content sits between a `<pasted_content id="…">` line and a `</pasted_content id="…">` line, so account for those lines if your hook parses the prompt.
    hooks.md:1552: ```json theme={null}
    hooks.md:1553: {
    hooks.md:1554:   "session_id": "abc123",
    hooks.md:1555:   "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
    hooks.md:1556:   "cwd": "/Users/...",
    hooks.md:1557:   "permission_mode": "default",
    hooks.md:1558:   "hook_event_name": "UserPromptSubmit",
    hooks.md:1559:   "prompt": "Write a function to calculate the factorial of a number"
    hooks.md:1560: }
    hooks.md:1561: ```
    hooks.md:1563: When the prompt is a [skill](https://code.claude.com/docs/en/skills.md) or custom command you typed, such as `/deploy staging`, the hook receives that line as you typed it rather than the expanded skill content. If the hook blocks the prompt, the command doesn't run, so shell commands the skill [injects with `` !`<command>` ``](https://code.claude.com/docs/en/skills#inject-dynamic-context) don't run either. To match on the command's name or arguments instead, use [UserPromptExpansion](#userpromptexpansion).

PreToolUse / PostToolUse (tool_name, tool_input, file_path):

    hooks.md:1808: In addition to the [common input fields](#common-input-fields), PreToolUse hooks receive `tool_name`, `tool_input`, and `tool_use_id`.
    hooks.md:1812: For the file tools `Write`, `Edit`, and `Read`, `tool_input.file_path` is always absolute:
    hooks.md:1814: * Claude Code expands `~` and relative paths before hooks run, so a hook that matches on paths can't be bypassed via `~` or a relative spelling of the same path
    hooks.md:1888: | Field | Type | Example | Description |
    hooks.md:1889: | :- | :- | :- | :- |
    hooks.md:1890: | `file_path` | string | `"/path/to/file.txt"` | Absolute path to the file to write |
    hooks.md:1891: | `content` | string | `"file content"` | Content to write to the file |
    hooks.md:1897: | Field | Type | Example | Description |
    hooks.md:1898: | :- | :- | :- | :- |
    hooks.md:1899: | `file_path` | string | `"/path/to/file.txt"` | Absolute path to the file to edit |
    hooks.md:1900: | `old_string` | string | `"original text"` | Text to find and replace |
    hooks.md:1901: | `new_string` | string | `"replacement text"` | Replacement text |
    hooks.md:1902: | `replace_all` | boolean | `false` | Whether to replace all occurrences |
    hooks.md:1908: | Field | Type | Example | Description |
    hooks.md:1909: | :- | :- | :- | :- |
    hooks.md:1910: | `file_path` | string | `"/path/to/file.txt"` | Absolute path to the file to read |
    hooks.md:1911: | `offset` | number | `10` | Optional line number to start reading from |
    hooks.md:1912: | `limit` | number | `50` | Optional number of lines to read |
    hooks.md:2297: The input includes both `tool_input`, the arguments sent to the tool, and `tool_response`, the result it returned. The exact schema for both depends on the tool. For a failed call, `tool_response` is the error text as a string. For an MCP tool, the input also carries the [`mcp_server`](#pretooluse-input) object.
    hooks.md:2299: ```json theme={null}
    hooks.md:2300: {
    hooks.md:2301:   "session_id": "abc123",
    hooks.md:2302:   "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
    hooks.md:2303:   "cwd": "/Users/...",
    hooks.md:2304:   "permission_mode": "default",
    hooks.md:2305:   "hook_event_name": "PostToolUse",
    hooks.md:2306:   "tool_name": "Write",
    hooks.md:2307:   "tool_input": {
    hooks.md:2308:     "file_path": "/path/to/file.txt",
    hooks.md:2309:     "content": "file content"
    hooks.md:2310:   },
    hooks.md:2311:   "tool_response": {
    hooks.md:2312:     "filePath": "/path/to/file.txt",
    hooks.md:2313:     "type": "create"
    hooks.md:2314:   },
    hooks.md:2315:   "tool_use_id": "toolu_01ABC123...",
    hooks.md:2316:   "duration_ms": 12
    hooks.md:2317: }
    hooks.md:2318: ```

SubagentStart / SubagentStop:

    hooks.md:2669: In addition to the [common input fields](#common-input-fields), SubagentStart hooks receive `agent_id` with the unique identifier for the subagent and `agent_type` with the agent name that the matcher filters on.
    hooks.md:2671: ```json theme={null}
    hooks.md:2672: {
    hooks.md:2673:   "session_id": "abc123",
    hooks.md:2674:   "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
    hooks.md:2675:   "cwd": "/Users/...",
    hooks.md:2676:   "hook_event_name": "SubagentStart",
    hooks.md:2677:   "agent_id": "agent-abc123",
    hooks.md:2678:   "agent_type": "Explore"
    hooks.md:2679: }
    hooks.md:2680: ```
    hooks.md:2709: In addition to the [common input fields](#common-input-fields), SubagentStop hooks receive `stop_hook_active`, `agent_id`, `agent_type`, `agent_transcript_path`, and `last_assistant_message`. The `transcript_path` is the main session's transcript, while `agent_transcript_path` is the subagent's own transcript stored in a nested `subagents/` folder. The `last_assistant_message` field contains the text content of the subagent's final response.
    hooks.md:2711: Claude Code runs internal agents for some of its own features, such as [prompt suggestions](https://code.claude.com/docs/en/interactive-mode.md#prompt-suggestions) and [`/btw` side questions](https://code.claude.com/docs/en/interactive-mode#side-questions-with-%2Fbtw), and SubagentStop fires when one of those finishes too. For those events, `agent_type` is the agent name the session itself runs as, such as one set with [`--agent`](https://code.claude.com/docs/en/cli-reference#cli-flags) or the [`agent` setting](https://code.claude.com/docs/en/settings-reference#agent), and an empty string when the session runs without one.
    hooks.md:2715: On Claude Code v2.1.271 or later, a subagent that runs with the [`SubagentHandback`](https://code.claude.com/docs/en/tools-reference) tool delivers its report through that tool before it stops. The `last_assistant_message` field then holds the subagent's closing text, if any, which is not the delivered report. The report is that call's `message` input, which a `PreToolUse` or `PostToolUse` hook matched on `SubagentHandback` receives as `tool_input.message`.
    hooks.md:2719: ```json theme={null}
    hooks.md:2720: {
    hooks.md:2721:   "session_id": "abc123",
    hooks.md:2722:   "transcript_path": "~/.claude/projects/.../abc123.jsonl",
    hooks.md:2723:   "cwd": "/Users/...",
    hooks.md:2724:   "permission_mode": "default",
    hooks.md:2725:   "hook_event_name": "SubagentStop",
    hooks.md:2726:   "stop_hook_active": false,
    hooks.md:2727:   "agent_id": "def456",
    hooks.md:2728:   "agent_type": "Explore",
    hooks.md:2729:   "agent_transcript_path": "~/.claude/projects/.../abc123/subagents/agent-def456.jsonl",
    hooks.md:2730:   "last_assistant_message": "Analysis complete. Found 3 potential issues...",
    hooks.md:2731:   "background_tasks": [],
    hooks.md:2732:   "session_crons": []
    hooks.md:2733: }
    hooks.md:2734: ```
    hooks.md:255: Hooks from settings files, managed policy settings, and plugins also run inside [subagents](https://code.claude.com/docs/en/sub-agents.md). When a subagent calls a tool, tool events such as `PreToolUse` and `PostToolUse` fire the same configured hooks as in the main conversation, and the input carries the `agent_id` and `agent_type` [common input fields](#common-input-fields) that identify the subagent.

Stop:

    hooks.md:2853: Runs when the main Claude Code agent has finished responding. Does not run if
    hooks.md:2854: the stoppage occurred due to a user interrupt. API errors fire
    hooks.md:2855: [StopFailure](#stopfailure) instead.
    hooks.md:2863: In addition to the [common input fields](#common-input-fields), Stop hooks receive `stop_hook_active`, `last_assistant_message`, `background_tasks`, and `session_crons`. The `stop_hook_active` field is `true` when Claude Code is already continuing as a result of a stop hook. Check this value or process the transcript to avoid blocking on a condition that will never resolve.
    hooks.md:2865: Claude Code applies an 8-consecutive-continuation cap: after stop hooks have continued the turn eight times in a row, Claude Code overrides the next block and ends the turn. The count of consecutive continuations resets each time Claude calls a tool. To raise the cap, set [`CLAUDE_CODE_STOP_HOOK_BLOCK_CAP`](https://code.claude.com/docs/en/env-vars).
    hooks.md:2867: The `last_assistant_message` field contains the text content of Claude's final response, so hooks can access it without parsing the transcript file. For hooks that act on the just-completed turn, such as read-aloud or notification hooks, use this field rather than reading `transcript_path`: the transcript file isn't guaranteed to include the final message at Stop time on all versions.


## H5 hook output: exit codes, JSON fields, additionalContext, decisions

Exit codes:

    hooks.md:805: * **Success**: your hook exits 0. Claude Code applies any [JSON output](#json-output) fields your hook printed, and the action goes ahead unless those fields block or deny it. [Hooks that start a background process](#hooks-that-start-a-background-process) covers the exception.
    hooks.md:806: * **Blocking error**: your hook exits 2. On [events that can block](#exit-code-2-behavior-per-event), Claude Code stops the action.
    hooks.md:807: * **Non-blocking error**: your hook exits with any other code, or fails in some other way, such as not starting or printing invalid JSON. The action goes ahead, except for hooks [whose failure blocks the action](#block-the-action-when-a-hook-fails), for example a hook with `onFailure` set to `"block"`. On events such as `PreToolUse`, you see a `<hook name> hook error` notice in the transcript.
    hooks.md:809: What your hook prints to stdout can change the outcome. For example, when a `PreToolUse` hook exits 1 but prints JSON that passes validation, that JSON can still decide what happens. To find your hook's outcome on an event such as `PreToolUse`, match what it printed to stdout in the first column with its exit code along the top:
    hooks.md:811: | Stdout | Exit 0 | Exit 2 | Any other exit code |
    hooks.md:812: | :- | :- | :- | :- |
    hooks.md:813: | JSON object that passes [schema validation](#json-output) | Success. The fields apply | Blocking error. Claude Code still reads the fields, but a `PreToolUse` call stays blocked even if you return `"allow"` or `continue: false`. For other events, see [Exit code 2](#exit-code-2) | Success. Claude Code ignores the exit code, and the fields alone decide. For hooks [whose failure blocks the action](#block-the-action-when-a-hook-fails), the run counts as a failure instead |
    hooks.md:814: | JSON that [can't be parsed](#exit-code-0) or fails schema validation | Non-blocking error. The notice carries the parse or validation message | Blocking error. Your stderr is the reason | Non-blocking error. The notice carries the parse or validation message |
    hooks.md:815: | [Plain text](#exit-code-0), or nothing | Success | Blocking error. Your stderr is the reason | Non-blocking error |
    hooks.md:821: * **`Stop`, `SubagentStop`, `TaskCompleted`, and a plugin's `UserPromptSubmit` hook**: when your hook exits 2 with nothing on stdout and its stderr says a file is missing, such as `No such file or directory`, Claude Code treats the run as a non-blocking error.
    hooks.md:829: Exit 0 means success, and is the intended exit code when you print JSON for structured control. When your hook exits 0 but prints JSON that can't be parsed or fails [schema validation](#json-output), the action goes ahead unless the hook is one [whose failure blocks the action](#block-the-action-when-a-hook-fails). Instead of a success with your fields applied, the run is a [non-blocking error](#exit-code-output).
    hooks.md:831: If a `<hook name> hook error` notice carries a parse or validation message, use it to fix your output. For a hook that isn't [async](#how-async-hooks-execute), your stdout counts as [JSON output](#json-output) only when you print one JSON object, on one line or several, with nothing around it but whitespace. Claude Code reads any other stdout as plain text or a parse failure:
    hooks.md:833: * **Output that doesn't start with `{`, or that starts with `{` and doesn't end with `}`**: plain text. A JSON array and a quoted JSON string are plain text by this rule.
    hooks.md:834: * **Two or more lines that each parse as JSON on their own, the first starting with `{` and the last ending with `}`**: plain text when no line is a JSON output object that sets a field. When one line is, the output is a parse failure, so print that object with nothing else on stdout.
    hooks.md:835: * **Anything else that starts with `{` and ends with `}` but isn't valid JSON**: a parse failure.
    hooks.md:837: On these events, your plain-text stdout reaches Claude or the compaction summary:
    hooks.md:839: * **`UserPromptSubmit`, `UserPromptExpansion`, `SessionStart`, and `PostModelSwitch`**: Claude Code adds plain-text stdout as context that Claude can see and act on. It doesn't add stdout it failed to parse.
    hooks.md:840: * **`PreCompact`**: your stdout becomes [instructions for the compaction summary](#precompact-output)
    hooks.md:842: Claude never sees stderr from a hook that exits 0. To read it yourself on events such as `PreToolUse`, enable [debug logging](#debug-hooks). To surface a warning to Claude from a `PostToolUse` or `PostToolUseFailure` hook, exit 2 instead so [Claude sees the stderr](#exit-code-2-behavior-per-event) even though the tool already ran.
    hooks.md:846: Exit with code 2 to block the action. On [events that can block](#exit-code-2-behavior-per-event), Claude Code stops the action: a `PreToolUse` hook blocks the tool call, for example, and a `UserPromptSubmit` hook rejects the prompt.
    hooks.md:848: The message that comes with the block is your hook's stderr. If your hook also printed JSON that makes a blocking decision, Claude Code uses that decision's reason instead.
    hooks.md:852: * **JSON that sets `continue: false`**: on events such as `Stop` and `SubagentStop`, the field takes precedence over exit code 2, so Claude or the subagent ends its turn. To keep Claude or the subagent working, leave the field out. On `PreToolUse`, the tool call stays blocked with or without the field.
    hooks.md:900: > Exit code 1 without valid JSON output is a non-blocking error unless the hook is one [whose failure blocks the action](#block-the-action-when-a-hook-fails), even though 1 is the conventional Unix failure code. If your hook is meant to enforce a policy, use `exit 2`.
    hooks.md:1034: | Hook event | Can block? | What happens on exit 2 |
    hooks.md:1035: | :- | :- | :- |
    hooks.md:1036: | `PreToolUse` | Yes | Blocks the tool call |
    hooks.md:1037: | `PermissionRequest` | No | Exit code 2 isn't honored for this event and the permission flow proceeds unchanged. Deny through the [`decision` object](#permissionrequest-decision-control) instead |
    hooks.md:1038: | `UserPromptSubmit` | Yes | Blocks the prompt. See [What a blocked prompt leaves behind](#what-a-blocked-prompt-leaves-behind) |
    hooks.md:1039: | `UserPromptExpansion` | Yes | Blocks the expansion |
    hooks.md:1040: | `Stop` | Yes | Prevents Claude from stopping, continues the conversation |
    hooks.md:1041: | `SubagentStop` | Yes | Prevents the subagent from stopping |
    hooks.md:1042: | `TeammateIdle` | Yes | Prevents the teammate from going idle, so it continues working |
    hooks.md:1043: | `TaskCreated` | Yes | Rolls back the task creation |
    hooks.md:1044: | `TaskCompleted` | Yes | Prevents the task from being marked as completed |
    hooks.md:1045: | `ConfigChange` | Yes | Blocks the configuration change from taking effect (except `policy_settings`) |
    hooks.md:1046: | `StopFailure` | No | Output and exit code are ignored, except `terminalSequence` |
    hooks.md:1047: | `PostToolUse` | No | Shows stderr to Claude; the tool already ran |
    hooks.md:1048: | `PostToolUseFailure` | No | Shows stderr to Claude; the tool already failed |
    hooks.md:1049: | `PostToolBatch` | Yes | Stops the agentic loop before the next model call |
    hooks.md:1050: | `PermissionDenied` | No | Exit code and stderr are ignored because the denial already occurred. Use JSON `hookSpecificOutput.retry: true` to tell the model it may retry; Claude Code ignores `retry: true` for [no-verdict denials](#permissiondenied-decision-control) |
    hooks.md:1051: | `Notification` | No | Exit code and stderr are ignored |
    hooks.md:1052: | `SubagentStart` | No | Shows stderr to user only |
    hooks.md:1053: | `SessionStart` | No | Shows stderr to user only |
    hooks.md:1054: | `Setup` | No | Exit code and stderr are ignored |
    hooks.md:1055: | `SessionEnd` | No | Shows stderr to user only |
    hooks.md:1056: | `CwdChanged` | No | Shows stderr to user only |
    hooks.md:1057: | `DirectoryAdded` | No | Stderr goes to the debug log; the directory is already added |
    hooks.md:1058: | `FileChanged` | No | Shows stderr to user only |
    hooks.md:1059: | `PreCompact` | Yes | Blocks compaction |
    hooks.md:1060: | `PostCompact` | No | Shows stderr to user only |
    hooks.md:1061: | `PreModelSwitch` | Yes | Blocks the model switch and shows stderr to the user |
    hooks.md:1062: | `PostModelSwitch` | No | Shows stderr to user only; the model already switched |
    hooks.md:1063: | `Elicitation` | Yes | Declines the request, and no dialog appears |
    hooks.md:1064: | `ElicitationResult` | Yes | Blocks the response (action becomes decline) |
    hooks.md:1065: | `WorktreeCreate` | Yes | Any non-zero exit code causes worktree creation to fail |
    hooks.md:1066: | `WorktreeRemove` | Yes | Any non-zero exit code causes worktree removal to fail if the directory still exists afterward. See [WorktreeRemove](#worktreeremove) for what happens to the directory |
    hooks.md:1067: | `InstructionsLoaded` | No | Exit code is ignored |
    hooks.md:1068: | `MessageDisplay` | No | The original text is displayed |
    hooks.md:1070: For `SessionStart`, `SubagentStart`, and `PostModelSwitch`, Claude Code renders the exit code 2 stderr in the transcript as a `<hook name> hook error` notice, the same way it renders a [non-blocking error](#exit-code-output). Claude doesn't see it, and the session or subagent proceeds. For `SubagentStart`, the notice appears in the subagent's own transcript, not in the parent conversation.

JSON output universal fields:

    hooks.md:1093: Print nothing but the JSON object to stdout. For a hook that isn't [async](#how-async-hooks-execute), other text there, such as a line your shell profile echoes at startup, keeps Claude Code from reading the object as JSON, and [Hook JSON has no effect](https://code.claude.com/docs/en/hooks-guide.md#hook-json-has-no-effect) shows how to find and silence that text.
    hooks.md:1095: A hook's `additionalContext`, `systemMessage`, and `initialUserMessage` strings, and its plain stdout, are capped at 10,000 characters:
    hooks.md:1101: The JSON object supports three kinds of fields:
    hooks.md:1103: * **Universal fields** like `continue` are listed in the table below. Every event accepts them, but some events discard them or deliver `systemMessage` somewhere other than the transcript. Each event's section says so. `terminalSequence` works on those events too, with the exceptions listed under [Emit terminal notifications](#emit-terminal-notifications).
    hooks.md:1104: * **Top-level `decision` and `reason`** are used by some events to block or provide feedback.
    hooks.md:1105: * **`hookSpecificOutput`** is a nested object for events that need richer control. It requires a `hookEventName` field set to the event name.
    hooks.md:1107: | Field | Default | Description |
    hooks.md:1108: | :- | :- | :- |
    hooks.md:1109: | `continue` | `true` | If `false`, Claude stops processing entirely after the hook runs. It doesn't block a `PreToolUse` tool call, so to do that, also return `permissionDecision: "deny"` |
    hooks.md:1110: | `stopReason` | none | Message shown to the user when `continue` is `false`. It stays in the conversation, so Claude sees it if the conversation continues |
    hooks.md:1111: | `suppressOutput` | `false` | Has no effect |
    hooks.md:1112: | `systemMessage` | none | Warning message shown to the user. In [Agent SDK](https://code.claude.com/docs/en/agent-sdk/overview.md) and [`--output-format stream-json`](https://code.claude.com/docs/en/headless) output, it can arrive as an [`SDKInformationalMessage`](https://code.claude.com/docs/en/agent-sdk/typescript#sdkinformationalmessage) |
    hooks.md:1113: | `terminalSequence` | none | A terminal escape sequence for Claude Code to emit on your behalf, such as a desktop notification, window title, or bell. Restricted to OSC `0`/`1`/`2`/`9`/`99`/`777` and BEL. If the value contains anything outside the allowlist, the field is ignored. Use this instead of writing to `/dev/tty`, which is unavailable to hooks |

additionalContext (which events) and decision control table:

    hooks.md:1166: The `additionalContext` field passes a string from your hook into Claude's context window. Claude Code wraps the string in a [system reminder](https://code.claude.com/docs/en/glossary.md#system-reminder) and inserts it into the conversation at the point where the hook fired. Claude reads the reminder on the next model request, but it doesn't appear as a chat message in the interface.
    hooks.md:1179: Where the reminder appears depends on the event:
    hooks.md:1181: * [SessionStart](#sessionstart) and [SubagentStart](#subagentstart): at the start of the conversation, before the first prompt
    hooks.md:1182: * [UserPromptSubmit](#userpromptsubmit) and [UserPromptExpansion](#userpromptexpansion): alongside the submitted prompt
    hooks.md:1183: * [PreToolUse](#pretooluse), [PostToolUse](#posttooluse), [PostToolUseFailure](#posttoolusefailure), and [PostToolBatch](#posttoolbatch): next to the tool result
    hooks.md:1184: * [Stop](#stop) and [SubagentStop](#subagentstop): at the end of the turn. The conversation continues so Claude can act on the feedback. See [Stop decision control](#stop-decision-control)
    hooks.md:1185: * [PostModelSwitch](#postmodelswitch): with the next request after the switch. See [PostModelSwitch decision control](#postmodelswitch-decision-control) for timing
    hooks.md:1187: When several hooks return `additionalContext` for the same event, Claude receives all of the values.
    hooks.md:1207: Not every event supports blocking or controlling behavior through JSON. The events that do each use a different set of fields to express that decision. Use this table as a quick reference before writing a hook:
    hooks.md:1209: | Events | Decision pattern | Key fields |
    hooks.md:1210: | :- | :- | :- |
    hooks.md:1211: | UserPromptSubmit, UserPromptExpansion, PostToolUse, PostToolUseFailure, PostToolBatch, Stop, SubagentStop, ConfigChange, PreCompact | Top-level `decision` | `decision: "block"`, `reason`. Stop and SubagentStop also accept `hookSpecificOutput.additionalContext` for [non-error feedback that continues the conversation](#stop-decision-control) |
    hooks.md:1212: | TeammateIdle, TaskCompleted | Exit code or top-level `decision` | Exit code 2 or `decision: "block"` keeps the teammate working or the task open. With `continue: false`, the teammate goes idle. [Claude Code ignores `continue: false` on a `TaskUpdate` call](#taskcompleted-decision-control) |
    hooks.md:1213: | TaskCreated | Exit code or top-level `decision` | Exit code 2 or `decision: "block"` [cancels the task](#taskcreated-decision-control) and returns the message to Claude. `continue: false` is ignored |
    hooks.md:1214: | PreToolUse | `hookSpecificOutput` | `permissionDecision` (allow/deny/ask/defer), `permissionDecisionReason` |
    hooks.md:1215: | PreModelSwitch | `hookSpecificOutput` or top-level `decision` | `permissionDecision` (allow/deny/ask), `permissionDecisionReason`. `decision: "block"` also [cancels the switch](#premodelswitch-decision-control) |
    hooks.md:1216: | PermissionRequest | `hookSpecificOutput` | `decision.behavior` (allow/deny) |
    hooks.md:1217: | PermissionDenied | `hookSpecificOutput` | `retry: true` tells the model it may retry the denied tool call; Claude Code ignores it for [no-verdict denials](#permissiondenied-decision-control) |
    hooks.md:1218: | WorktreeCreate | path return | Command hook prints path on stdout; HTTP hook returns `hookSpecificOutput.worktreePath`. Hook failure or missing path fails creation |
    hooks.md:1219: | WorktreeRemove | Exit code | Any non-zero exit code makes the removal fail if the directory still exists afterward. JSON output is discarded |
    hooks.md:1220: | Elicitation, ElicitationResult | `hookSpecificOutput` or top-level `decision` | `action` (accept/decline/cancel), `content` (form field values). `decision: "block"` also [declines](#other-ways-to-decline-an-elicitation) |
    hooks.md:1221: | MessageDisplay | `hookSpecificOutput` | `displayContent` replaces the displayed text on screen. Display-only: the transcript and what Claude sees keep the original |
    hooks.md:1222: | SessionStart, SubagentStart, PostModelSwitch | Context only | `hookSpecificOutput.additionalContext` adds context for Claude. SessionStart also accepts [`initialUserMessage`, `watchPaths`, `sessionTitle`, and `reloadSkills`](#sessionstart-decision-control). No blocking or decision control |
    hooks.md:1223: | Setup, Notification, SessionEnd, PostCompact, InstructionsLoaded, StopFailure, CwdChanged, DirectoryAdded, FileChanged | None | No decision control. Used for side effects like logging or cleanup |
    hooks.md:1238: > The only value for `decision` is `"block"`. To allow the action to proceed, omit `decision` from your JSON, or exit 0 without any JSON at all:

PreToolUse permissionDecision:

    hooks.md:2011: `PreToolUse` hooks can control whether a tool call proceeds. Unlike other hooks that use a top-level `decision` field, PreToolUse returns its decision inside a `hookSpecificOutput` object. This gives it richer control: four outcomes (allow, deny, ask, or defer) plus the ability to modify tool input before execution. To allow or rewrite a Monitor command, see [Match Monitor commands](#match-monitor-commands).
    hooks.md:2013: | Field | Description |
    hooks.md:2014: | :- | :- |
    hooks.md:2015: | `permissionDecision` | `"allow"` skips the permission prompt, except for [actions no mode auto-approves](https://code.claude.com/docs/en/permission-modes.md#actions-no-mode-auto-approves) and [reads from network paths](https://code.claude.com/docs/en/permissions.md#network-paths). It doesn't skip approval for calls to [tools an Agent SDK app marks `always_ask`](https://code.claude.com/docs/en/agent-sdk/custom-tools#require-approval-for-specific-tools) either, unless the [permission mode approves the call](https://code.claude.com/docs/en/agent-sdk/typescript.md#permission-modes-that-can-approve-an-always_ask-call). `AskUserQuestion` and `ExitPlanMode` need [`updatedInput` paired with `"allow"`](#allow-with-updatedinput). For plan mode, see [Hook approvals in plan mode](https://code.claude.com/docs/en/permissions.md#hook-approvals-in-plan-mode). `"deny"` prevents the tool call. `"ask"` prompts the user to confirm. `"defer"` ends the turn so the tool can be resumed later, and requires [non-interactive mode](#defer-a-tool-call-for-later) with the `-p` flag. [Deny and ask rules](https://code.claude.com/docs/en/permissions.md#manage-permissions) are still evaluated regardless of what the hook returns |
    hooks.md:2016: | `permissionDecisionReason` | For `"ask"`, shown to the user in the permission prompt. When Claude Code [denies the call](https://code.claude.com/docs/en/headless.md#turn-off-permission-prompts-in-unattended-runs) in a `-p` run where no one can answer that prompt, Claude reads the reason in the tool result instead. For `"deny"`, shown to Claude. For `"allow"` and `"defer"`, written to the [debug log](#debug-hooks) only |
    hooks.md:2017: | `updatedInput` | Modifies the tool's input parameters before execution. Replaces the entire input object, so include unchanged fields alongside modified ones. Claude Code evaluates permission rules and a Bash command's [auto-background eligibility](https://code.claude.com/docs/en/tools-reference.md#foreground-commands-that-move-to-the-background) against the input your hook returns, not the input Claude sent. Combine with `"allow"` to auto-approve, or `"ask"` to show the modified input to the user. For `"defer"`, ignored. With no `permissionDecision`, [auto mode can deny the changed call](#pretooluse-updatedinput-in-auto-mode) |
    hooks.md:2018: | `additionalContext` | String added to Claude's context alongside the tool result. Ignored when `permissionDecision` is `"defer"`. See [Add context for Claude](#add-context-for-claude) |
    hooks.md:2020: When multiple PreToolUse hooks return different decisions, precedence is `deny` > `defer` > `ask` > `allow`.
    hooks.md:2022: A hook that blocks by exiting 2 routes the same way as `"deny"`: Claude sees the stderr message as the denial reason.
    hooks.md:2026: ```json theme={null}
    hooks.md:2027: {
    hooks.md:2028:   "hookSpecificOutput": {
    hooks.md:2029:     "hookEventName": "PreToolUse",
    hooks.md:2030:     "permissionDecision": "allow",
    hooks.md:2031:     "permissionDecisionReason": "My reason here",
    hooks.md:2032:     "updatedInput": {
    hooks.md:2033:       "field_to_modify": "new value"
    hooks.md:2034:     },
    hooks.md:2035:     "additionalContext": "Current environment: production. Proceed with caution."
    hooks.md:2036:   }
    hooks.md:2037: }
    hooks.md:2038: ```
    hooks.md:2042: > PreToolUse previously used top-level `decision` and `reason` fields, but these are deprecated for this event. Use `hookSpecificOutput.permissionDecision` and `hookSpecificOutput.permissionDecisionReason` instead. The deprecated values `"approve"` and `"block"` map to `"allow"` and `"deny"` respectively. Other events like PostToolUse and Stop continue to use top-level `decision` and `reason` as their current format.

PostToolUse decision:

    hooks.md:2328: | Field | Description |
    hooks.md:2329: | :- | :- |
    hooks.md:2330: | `decision` | `"block"` adds the `reason` next to the tool result. Claude still sees the original output; to replace it, use `updatedToolOutput` |
    hooks.md:2331: | `reason` | Explanation shown to Claude when `decision` is `"block"` |
    hooks.md:2332: | `additionalContext` | String added to Claude's context alongside the tool result. See [Add context for Claude](#add-context-for-claude) |

Stop / SubagentStop block:

    hooks.md:2927: `Stop` and `SubagentStop` hooks can control whether Claude continues. In addition to the [JSON output fields](#json-output) available to all hooks, your hook script can return these event-specific fields:
    hooks.md:2929: | Field | Description |
    hooks.md:2930: | :- | :- |
    hooks.md:2931: | `decision` | `"block"` prevents Claude from stopping. Omit to allow Claude to stop |
    hooks.md:2932: | `reason` | Required when `decision` is `"block"`. Tells Claude why it should continue |
    hooks.md:2933: | `hookSpecificOutput.additionalContext` | Non-error feedback for Claude. The conversation continues so Claude can act on it, but unlike `decision: "block"` it is shown in the transcript as hook feedback rather than a hook error |
    hooks.md:2935: A hook that blocks by exiting 2 routes the same way as `reason`: Claude receives the stderr message as the explanation for why it should continue.
    hooks.md:2937: ```json theme={null}
    hooks.md:2938: {
    hooks.md:2939:   "decision": "block",
    hooks.md:2940:   "reason": "Must be provided when Claude is blocked from stopping"
    hooks.md:2941: }
    hooks.md:2942: ```
    hooks.md:2944: Use `additionalContext` when the hook is working as designed and giving Claude guidance, such as "run the test suite before finishing". It keeps the conversation going through the same loop protections as `decision: "block"`, namely the `stop_hook_active` input and the 8-consecutive-continuation cap, but the transcript labels it `Stop hook feedback` and no hook error notification is shown:
    hooks.md:2736: SubagentStop hooks use the same decision control format as [Stop hooks](#stop-decision-control), including `hookSpecificOutput.additionalContext` with `hookEventName` set to `"SubagentStop"`, for non-error feedback that keeps the subagent running. Returning `decision: "block"` with a `reason` keeps the subagent running and delivers `reason` to the subagent as its next instruction. To inject context into the parent session after a subagent returns, use a [`PostToolUse`](#posttooluse) hook on the `Agent` tool instead.

UserPromptSubmit block and plain stdout as context:

    hooks.md:1569: There are two ways to add context to the conversation on exit code 0:
    hooks.md:1571: * **Plain text stdout**: Claude Code adds stdout it [treats as plain text](#exit-code-0) to Claude's context
    hooks.md:1572: * **JSON with `additionalContext`**: use the JSON format below for more control. The `additionalContext` field is added as context
    hooks.md:1574: Neither channel produces a visible transcript entry. Plain stdout and the `additionalContext` value are each injected as a system reminder that starts with the hook's name; Claude reads both. To confirm delivery, check the [debug log](#debug-hooks).
    hooks.md:1576: To block a prompt, return a JSON object with `decision` set to `"block"`:
    hooks.md:1578: | Field | Description |
    hooks.md:1579: | :- | :- |
    hooks.md:1580: | `decision` | `"block"` rejects the prompt. Omit to allow the prompt to proceed |
    hooks.md:1581: | `reason` | Shown to the user when `decision` is `"block"`. Not added to context, except when the hook blocks [Code Mode output](https://code.claude.com/docs/en/code-mode.md#code-output-withheld-by-a-hook) before v2.1.328 |
    hooks.md:1582: | `additionalContext` | String added to Claude's context alongside the submitted prompt. See [Add context for Claude](#add-context-for-claude) |
    hooks.md:1583: | `sessionTitle` | Sets the session title. Use to name sessions automatically based on the prompt content |
    hooks.md:1584: | `suppressOriginalPrompt` | If `true` when the hook blocks the prompt, leaves the prompt text out of the block message. See [What a blocked prompt leaves behind](#what-a-blocked-prompt-leaves-behind) |
    hooks.md:1586: If your hook blocks by exiting 2, its stderr text is treated the same way as `reason`.

SessionStart output and plain stdout as context:

    hooks.md:1350: A SessionStart hook can add context for Claude, supply the first user message, set the session title, watch files, and reload skills. Return the field for each one, in addition to the [JSON output fields](#json-output) available to all hooks:
    hooks.md:1352: | Field | Description |
    hooks.md:1353: | :- | :- |
    hooks.md:1354: | `additionalContext` | String added to Claude's context at the start of the conversation, before the first prompt. See [Add context for Claude](#add-context-for-claude) for how the text is delivered and what to put in it |
    hooks.md:1355: | `initialUserMessage` | String used as the first user message of the session, in [non-interactive mode](https://code.claude.com/docs/en/headless.md) with the `-p` flag. It becomes the first turn even if you pass no prompt. A prompt you do pass follows as the next turn |
    hooks.md:1356: | `sessionTitle` | Sets the session title, with the same effect as `/rename`. Applies when `source` is `"startup"`, `"resume"`, or `"fork"` |
    hooks.md:1357: | `watchPaths` | Array of absolute paths to watch for [FileChanged](#filechanged) events during this session |
    hooks.md:1358: | `reloadSkills` | Boolean. When `true`, Claude Code re-scans the [skill](https://code.claude.com/docs/en/skills.md) and command directories after the SessionStart hooks complete. See [Reload skills that a hook installs](#reload-skills-that-a-hook-installs) |
    hooks.md:1372: A hook that only adds context can print it without building JSON, because Claude Code adds a SessionStart hook's [plain-text stdout](#exit-code-0) to Claude's context.


## H6 combining hooks across settings files; trust

    hooks.md:241: | Location | Scope | Shareable |
    hooks.md:242: | :- | :- | :- |
    hooks.md:243: | `~/.claude/settings.json` | All your projects | No, local to your machine |
    hooks.md:244: | `.claude/settings.json` | Single project | Yes, can be committed to the repo |
    hooks.md:245: | `.claude/settings.local.json` | Single project | No, gitignored when Claude Code saves a setting to it |
    hooks.md:246: | Managed policy settings | Organization-wide | Yes, admin-controlled |
    hooks.md:247: | [Plugin](https://code.claude.com/docs/en/plugins/overview.md) `hooks/hooks.json` | When plugin is enabled | Yes, bundled with the plugin |
    hooks.md:266: Hook entries merge across settings levels rather than replacing each other: user, project, and local settings add their own hooks without removing managed ones, and the [`disableAllHooks`](#disable-or-remove-hooks) setting can't disable managed hooks from outside managed settings.
    hooks.md:411: If you define the same handler in more than one settings file, it runs once. A plugin's or skill's copy of the same handler stays separate.
    hooks.md:735: To temporarily disable all hooks without removing them, set `"disableAllHooks": true` in your settings file. Claude Code reads the value left after [settings precedence](https://code.claude.com/docs/en/settings.md#settings-precedence) applies, so a `"disableAllHooks": false` in a project's `.claude/settings.json` overrides a `true` in your user settings. To turn hooks off for one run whatever the project's settings say, pass `--settings '{"disableAllHooks": true}'`, which takes precedence over project and local settings. There is no way to disable an individual hook while keeping it in the configuration.
    hooks.md:739: Direct edits to hooks in settings files are normally picked up automatically by the file watcher.
    hooks.md:4300: Claude Code checks workspace trust before it runs any hook from a settings file. What counts as trusted depends on the session type:
    hooks.md:4302: * **Interactive session**: Claude Code holds back hooks from every settings file, including your own `~/.claude/settings.json`, until you accept the [workspace trust dialog](https://code.claude.com/docs/en/permissions.md#project-allow-rules-and-workspace-trust) for the folder, or for a parent directory whose trust extends to it
    hooks.md:4303: * **`-p` or SDK session**: Claude Code never shows the dialog and treats the folder as trusted, so hooks committed in a repository's `.claude/settings.json` run in a folder you've never trusted
    hooks.md:4305: Before you script `claude -p` over a repository you didn't write, review its `.claude/` settings files, start with [`--bare`](https://code.claude.com/docs/en/headless#start-faster-with-bare-mode), or [turn hooks off for that run](#disable-or-remove-hooks) with `--settings '{"disableAllHooks": true}'`. Frontmatter hooks in a project subagent follow a stricter rule than settings-file hooks. [What runs before you trust a folder](https://code.claude.com/docs/en/permissions.md#what-runs-before-you-trust-a-folder) lists each kind of repository content by session type.
    hooks.md:3724: Runs when an MCP server requests user input mid-task. By default, Claude Code shows an interactive dialog for the user to respond. Hooks can intercept this request and respond programmatically, skipping the dialog entirely.


## H7 hook event list

The lifecycle table (hooks.md:31-65) lists 33 events: SessionStart, Setup, UserPromptSubmit, UserPromptExpansion, PreToolUse, PermissionRequest, PermissionDenied, PostToolUse, PostToolUseFailure, PostToolBatch, Notification, MessageDisplay, SubagentStart, SubagentStop, TaskCreated, TaskCompleted, Stop, StopFailure, TeammateIdle, InstructionsLoaded, ConfigChange, CwdChanged, DirectoryAdded, FileChanged, WorktreeCreate, WorktreeRemove, PreCompact, PostCompact, PreModelSwitch, PostModelSwitch, Elicitation, ElicitationResult, SessionEnd.

    hooks.md:21: Events fall into three cadences:
    hooks.md:23: * per session: `SessionStart` and `SessionEnd`
    hooks.md:24: * per turn: `UserPromptSubmit`, `Stop`, and `StopFailure`
    hooks.md:25: * on every tool call inside the agentic loop: `PreToolUse` and `PostToolUse`, except [`EndConversation`](https://code.claude.com/docs/en/tools-reference#endconversation-tool-behavior) calls, which skip both
    hooks.md:31: | Event | When it fires |
    hooks.md:32: | :- | :- |
    hooks.md:33: | `SessionStart` | When a session begins or resumes |
    hooks.md:34: | `Setup` | When you start Claude Code with `--init-only`, or with `--init` or `--maintenance` in `-p` mode. For one-time preparation in CI or scripts |
    hooks.md:35: | `UserPromptSubmit` | When a prompt is submitted, before Claude processes it. Also fires on [turns Claude Code starts on its own](https://code.claude.com/docs/en/hooks.md#userpromptsubmit) |
    hooks.md:36: | `UserPromptExpansion` | When a user-typed command expands into a prompt, before it reaches Claude. Can block the expansion |
    hooks.md:37: | `PreToolUse` | Before a tool call executes. Can block it |
    hooks.md:38: | `PermissionRequest` | When a tool call needs a permission decision |
    hooks.md:39: | `PermissionDenied` | When auto mode denies a tool call, including denials without a classifier verdict. Use JSON `hookSpecificOutput.retry: true` to tell the model it may retry the denied tool call. Claude Code ignores `retry` when the classifier produced no verdict |
    hooks.md:40: | `PostToolUse` | After a tool call succeeds |
    hooks.md:41: | `PostToolUseFailure` | After a tool call fails |
    hooks.md:42: | `PostToolBatch` | After a full batch of parallel tool calls resolves, before the next model call |
    hooks.md:43: | `Notification` | When Claude Code sends a notification |
    hooks.md:44: | `MessageDisplay` | While assistant message text is displayed |
    hooks.md:45: | `SubagentStart` | When a subagent is spawned |
    hooks.md:46: | `SubagentStop` | When a subagent finishes |
    hooks.md:47: | `TaskCreated` | When a task is being created via `TaskCreate` |
    hooks.md:48: | `TaskCompleted` | When a task is being marked as completed |
    hooks.md:49: | `Stop` | When Claude finishes responding |
    hooks.md:50: | `StopFailure` | When the turn ends due to an API error |
    hooks.md:51: | `TeammateIdle` | When an [agent team](https://code.claude.com/docs/en/agent-teams.md) teammate is about to go idle |
    hooks.md:52: | `InstructionsLoaded` | When a CLAUDE.md or `.claude/rules/*.md` file is loaded into context. Fires at session start and when files are lazily loaded during a session |
    hooks.md:53: | `ConfigChange` | When a configuration file changes during a session |
    hooks.md:54: | `CwdChanged` | When the working directory changes, for example when Claude executes a `cd` command. Useful for reactive environment management with tools like direnv |
    hooks.md:55: | `DirectoryAdded` | When a working directory is added mid-session via `/add-dir` or the SDK `register_repo_root` control request |
    hooks.md:56: | `FileChanged` | When a watched file changes on disk. The `matcher` field specifies which filenames to watch |
    hooks.md:57: | `WorktreeCreate` | When a worktree is being created via `--worktree`, `isolation: "worktree"`, or for a background session. Replaces default git behavior |
    hooks.md:58: | `WorktreeRemove` | When a worktree that a `WorktreeCreate` hook created is being removed |
    hooks.md:59: | `PreCompact` | Before context compaction |
    hooks.md:60: | `PostCompact` | After context compaction completes |
    hooks.md:61: | `PreModelSwitch` | Before Claude Code applies a model switch that you or a client requested. Can block the switch |
    hooks.md:62: | `PostModelSwitch` | After the session's model changes, including changes Claude Code makes on its own, such as restoring the model when you resume a session |
    hooks.md:63: | `Elicitation` | When an MCP server requests user input during a tool call |
    hooks.md:64: | `ElicitationResult` | After a user responds to an MCP elicitation, before the response is sent back to the server |
    hooks.md:65: | `SessionEnd` | When a session terminates |

## Settings

## T1 settings files, precedence, committing .claude/settings.json

    settings.md:25: | Scope | File | Who it affects | Use it for |
    settings.md:26: | :- | :- | :- | :- |
    settings.md:27: | User | `~/.claude/settings.json` | You, in every project on this machine | Personal preferences: theme, editor mode, default model, your own permission rules |
    settings.md:28: | Shared project | `.claude/settings.json` | Everyone working in the folder that contains it. In a git repository, commit it so teammates get it | Team permissions, hooks, plugins, and the environment variables the project needs |
    settings.md:29: | Project local | `.claude/settings.local.json` | You, in this one project only. Claude Code keeps it out of git when it creates the file; if you create it by hand, add it to `.gitignore` yourself | Personal overrides for one project, and testing before you share |
    settings.md:30: | Managed | `managed-settings.json` and other [managed sources](https://code.claude.com/docs/en/managed-settings.md#delivery-mechanisms) | Everyone your organization deploys it to; [Settings precedence](#settings-precedence) says what can override it | Security policy and compliance requirements |
    settings.md:69: Commit `.claude/settings.json` so everyone who clones the repository gets the same permissions, hooks, and plugin settings. Each teammate can still override it for themselves in their own `.claude/settings.local.json`, so personal exceptions don't need a commit. For a complete team file, see [a team's shared settings](https://code.claude.com/docs/en/settings-example.md#a-teams-shared-settings).
    settings.md:75: To change a setting for yourself in one project without changing it for your teammates, save it in `.claude/settings.local.json` inside the project. Claude Code applies that file over the committed `.claude/settings.json`, so if your team's file sets `"model": "claude-sonnet-5"` and you want Opus, put `"model": "claude-opus-5-5"` in your local file and only your sessions change.
    settings.md:80: * **You don't need to gitignore it yourself, unless you created it by hand.** The first time Claude Code writes the file in a git repository that doesn't already ignore it, it adds `**/.claude/settings.local.json` to your global git excludes file, so the file stays out of your commits in every repository. That file is `core.excludesFile` when your global git config sets it to an absolute or `~`-prefixed path; otherwise it's `$XDG_CONFIG_HOME/git/ignore`, or `~/.config/git/ignore` when `XDG_CONFIG_HOME` is unset. If you created the file by hand and Claude Code hasn't written to it yet, add it to `.gitignore` yourself.
    settings.md:81: * **Its allow rules don't wait for trust while the file stays untracked.** Because the file is yours and not the repository's, Claude Code applies its `allow` rules without the [workspace trust](https://code.claude.com/docs/en/permissions.md#project-allow-rules-and-workspace-trust) step it requires for the committed file. If the file is tracked by git, the trust step applies to it too; see [When your local settings file needs trust](https://code.claude.com/docs/en/permissions.md#when-your-local-settings-file-needs-trust).
    settings.md:94: Claude Code reads the shared `.claude/settings.json` from the session's [primary working directory](https://code.claude.com/docs/en/permissions.md#working-directories), so to use a file committed at the repository root, start Claude Code there. After you [move the session with `/cd`](https://code.claude.com/docs/en/permissions#move-the-session-to-another-directory), Claude Code reads both project files from the new directory instead, placing the local file by the same rules. Reading them from the directory you moved to requires Claude Code v2.1.246 or later. For a worktree session that you start from the desktop app, see [What worktrees share with the main checkout](https://code.claude.com/docs/en/worktrees.md#what-worktrees-share-with-the-main-checkout).
    settings.md:208: ## Settings precedence
    settings.md:210: When the same key appears in more than one place, Claude Code uses the value from the highest level that sets it. The stack below shows the levels, highest on top; a key at a higher level overrides the same key anywhere below it.
    settings.md:212: Settings precedence, highest first: managed settings, command line, project local, shared project, user. A key set at a higher level overrides the same key set lower down.
    settings.md:214: In order, highest precedence first:
    settings.md:216: 1. **Managed settings**: settings your organization deploys, by a `managed-settings.json` file, an MDM policy, or [server-managed settings](https://code.claude.com/docs/en/server-managed-settings.md) from the claude.ai console. Nothing in your own settings files or `--settings` overrides a managed key, and a flag such as `--model` picks only from the models your organization allows. A managed [`model`](https://code.claude.com/docs/en/settings-reference#model) is a starting default, not a lock; the locks are [`availableModels`](https://code.claude.com/docs/en/settings-reference#availablemodels) and [`deniedModels`](https://code.claude.com/docs/en/settings-reference#deniedmodels). When your organization delivers more than one managed source, the rules for [precedence within the managed tier](https://code.claude.com/docs/en/managed-settings.md#precedence-within-the-managed-tier) say what Claude Code reads from each.
    settings.md:217: 2. **Command line**: JSON you pass with `--settings <file-or-json>` when you start `claude`, for that session only; see [Change a setting for one session](#change-a-setting-for-one-session). A key you set there overrides the same key in your project and user settings files, and a key you leave out keeps its value from those files. Other flags, such as `--model`, set one thing for the session and aren't part of this stack; a key's entry on the [settings reference](https://code.claude.com/docs/en/settings-reference.md) says which flags override it.
    settings.md:218: 3. **Project local settings** (`.claude/settings.local.json`): your personal settings for this project.
    settings.md:219: 4. **Shared project settings** (`.claude/settings.json`): settings your team checks into source control.
    settings.md:220: 5. **User settings** (`~/.claude/settings.json`): your personal settings for every project.
    settings.md:226: ### Lists merge instead of overriding
    settings.md:228: When you set the same list key, such as `permissions.allow`, in more than one file, Claude Code combines the lists instead of picking one, so each file can add entries without removing another file's. Four keys that hold model lists or per-model entries follow their own rules:

