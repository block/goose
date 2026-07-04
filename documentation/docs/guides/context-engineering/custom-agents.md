---
title: Custom Agents
sidebar_position: 3
sidebar_label: Custom Agents
---

Custom agents are reusable gosling configurations for specific roles, behaviors, or areas of expertise. Each agent packages a name, description, optional model preference, and instructions so you can quickly ask gosling to work with a specialized role such as code reviewer, documentation writer, test planner, or release assistant.

Use custom agents when you want the same role or behavior across multiple sessions without retyping the same instructions.

## Create an Agent File

Agents are stored as Markdown files with YAML frontmatter. You can create or edit these files directly in your editor.

Global agents are available across gosling sessions:

```text
~/.agents/agents/
```

Project agents are available when gosling is working in that project:

```text
<project>/.agents/agents/
```

:::note Compatibility paths
gosling also discovers agents from `.gosling/agents/`, `.claude/agents/`, `~/.gosling/agents/`, `~/.claude/agents/`, gosling's platform-specific config agents directory, and project-local `.agents/agents/`. New shared agents should use `.agents/agents/` for project agents or `~/.agents/agents/` for global agents.
:::

Create the directory if it does not already exist, then add a Markdown file for your agent:

```markdown title="~/.agents/agents/code-reviewer.md"
---
name: code-reviewer
description: Reviews code for correctness, maintainability, and risk
model: gpt-5.5
---

You are a senior code reviewer. Review changes for correctness, maintainability, security, and test coverage. Be direct, prioritize issues by severity, and suggest concrete fixes.
```

The frontmatter supports:

| Field | Required | Description |
|---|---:|---|
| `name` | Yes | Name used to list, load, mention, or delegate to the agent. |
| `description` | No | Short summary shown when listing agents. |
| `model` | No | Preferred model for the agent. |

Only `name` is required in the frontmatter. `description` and `model` are optional. The Markdown body is the agent's instructions and should not be empty if you want the agent to appear as a usable source in chat.

## Use an Agent

After you create an agent file, start gosling from a working directory where the agent can be discovered. Project agents are discovered from the current working directory, while global agents are discovered from your home/config directories.

### List available agents

In a gosling chat session, ask gosling to list available sources:

```text
list available sources
```

This is a prompt to gosling, not a terminal command. When source loading is available, gosling lists discoverable agents alongside other sources.

You can invoke an agent by mentioning it by name:

```text
@code-reviewer review the current diff
```

Or ask gosling to use it:

```text
Use the code-reviewer agent to review this pull request.
```

In chat interfaces with a mention picker, type `@` and choose the agent by name, such as `code-reviewer`.

### Delegate work to an agent

Use a custom agent when you want an isolated specialist to perform a task and return the result to the current conversation.

```text
Delegate to code-reviewer: review the current diff and identify the highest-risk issues.
```

Or, if you are writing prompts that call the delegation tool directly, use the agent's `name` as the source:

```text
Use the code-reviewer agent to review this pull request.
```

The delegated agent runs in a separate session with the instructions from the agent file. It can use the model specified in the agent frontmatter. Some interfaces also let you override settings such as model, provider, temperature, or max turns when you delegate.

### Load an agent's instructions into the current conversation

Use an agent as loaded context when you want the current conversation to adopt the agent's instructions without creating a separate delegated session.

```text
Load the code-reviewer agent, then review this change.
```

Loading an agent adds its instructions to the current conversation context. Delegating to an agent runs it separately and returns a result.

## Example Agents

### Code reviewer

```markdown title="~/.agents/agents/code-reviewer.md"
---
name: code-reviewer
description: Reviews code for correctness, maintainability, and risk
model: gpt-5.5
---

You are a senior code reviewer. Review changes for correctness, maintainability, security, and test coverage.

Prioritize:
- bugs and correctness issues
- security or privacy risks
- missing tests
- unnecessary complexity
- unclear naming or structure

Be direct. Group findings by severity and suggest concrete fixes.
```

### Documentation writer

```markdown title="~/.agents/agents/docs-writer.md"
---
name: docs-writer
description: Writes clear developer documentation
---

You are a developer documentation writer. Explain features clearly, use practical examples, and avoid marketing language.

When writing docs:
- start with what the user can accomplish
- show the shortest working example
- explain important options after the example
- call out limitations and prerequisites
```

## When to Use Agents or Skills

| Use | Best fit |
|---|---|
| Change gosling's role, tone, or instructions for a task | Custom agent |
| Teach gosling a reusable workflow or domain-specific procedure it can load on demand | [Skill](/docs/guides/context-engineering/using-skills) |
| Delegate work to another isolated gosling instance | [Subagent](/docs/guides/context-engineering/subagents) |

Agents define who gosling should be for a task. Skills define what gosling should know or do.

### Can custom agents be scheduled to run?

No. Custom agents are reusable roles, not scheduled jobs. To run something on a schedule, use an external scheduler such as cron to invoke `gosling run`, and include the agent's instructions in the prompt if you want the run to behave like a custom agent.

### Do custom agents have workflows?

No. A custom agent defines who gosling should be for a task: its role, behavior, instructions, and optional model preference. It does not define a step-by-step workflow. Use a [skill](/docs/guides/context-engineering/using-skills) when you need a repeatable procedure gosling can load on demand.

### Can custom agents use skills?

Yes. A custom agent can use skills that are available in the session. Skills are still discovered and loaded through gosling's normal skill behavior, so the agent can use them when your request matches a skill or when you explicitly ask for one.

### Can custom agents use MCP servers?

Yes. Custom agents can use the MCP servers and extensions that are enabled in the current session. The agent file itself does not define a separate MCP server list.

### Can custom agents call subagents?

Yes, when delegation tools are available in the session. A custom agent can ask gosling to delegate work to a subagent just like the default gosling agent can. Delegated subagents run in isolated sessions and do not automatically inherit the full parent conversation.

### Can one custom agent call another custom agent?

Yes, through delegation. For example, one custom agent can delegate a task to another custom agent by name if that agent is discoverable. This is useful for one-off collaboration between specialized agents.

For repeatable chains, spell out the sequence in your instructions or a [skill](/docs/guides/context-engineering/using-skills), such as delegating first to a reviewer agent, then to a docs agent, then combining the results.
