---
sidebar_position: 1
title: Automatic Multi-Model Switching
sidebar_label: Automatic Model Switching
---

The AutoPilot feature enables intelligent, context-aware switching between different models. You simply work naturally with goose, and AutoPilot chooses the right model based on conversation content, complexity, tool usage patterns, and other triggers.

:::warning Experimental Feature
AutoPilot is an experimental feature. Behavior and configuration may change in future releases.
:::

## How AutoPilot Works

After you configure which models to use for different roles, AutoPilot handles the rest. During your sessions, it automatically switches to the most appropriate model for your current task&mdash;whether you need specialized coding help, complex reasoning, or just want a second opinion.

**For example:**
- When you ask to "debug this error," AutoPilot switches to a model optimized for debugging
- When you request "analyze the performance implications," it switches to a model better suited for complex reasoning  
- When you're doing repetitive coding tasks, it uses a cost-effective model, but escalates to a more powerful one when it encounters failures

Switching happens automatically based on:
- The terminology used in your requests ("debug", "analyze", "implement")
- How complex the task appears to be
- Whether previous attempts have failed and need a different approach
- How much autonomous work has been happening without your input

When AutoPilot switches to a specialized model, it stays with that model for a configured number of <abbr title="A turn is one complete prompt-response interaction between goose and the LLM" style={{ textUnderlineOffset: "3px" }}>turns</abbr> before evaluating whether to switch back to the base model or to a different specialized model based on the new context.

:::info
You can use `goose session --debug` in goose CLI to see when AutoPilot switches models. Note that each switch applies the provider's rate limits and pricing.
::: 

## Configuration

Add the `x-advanced-models` section to your [`config.yaml`](/docs/guides/config-files) file and map your model preferences to [predefined](#predefined-roles) or custom roles. 

The `provider`, `model` and `role` parameters are required.

```yaml
# Base provider and model (always available)
GOOSE_PROVIDER: "anthropic"
GOOSE_MODEL: "claude-sonnet-4-20250514"

# AutoPilot models
x-advanced-models:
- provider: openai
  model: o1-preview
  role: deep-thinker
- provider: openai
  model: gpt-4o
  role: debugger
- provider: anthropic
  model: claude-opus-4-20250805
  role: reviewer
```

**Migrate From Lead/Worker Model**

This example shows how you can reproduce [lead model](/docs/tutorials/lead-worker) behavior using `x-advanced-models`.

```yaml
# Before: Defined lead model using environment variables
# GOOSE_LEAD_PROVIDER=openai
# GOOSE_LEAD_MODEL=o1-preview

# After: AutoPilot equivalent
GOOSE_PROVIDER: "anthropic"
GOOSE_MODEL: "claude-sonnet-4-20250514"  # Base is used as the worker model

x-advanced-models:
- provider: openai
  model: o1-preview
  role: lead  # Use the predefined lead role (or define a custom role)
```

### Predefined Roles

AutoPilot includes a set of predefined roles defined in [`premade_roles.yaml`](https://github.com/block/goose/blob/main/crates/goose/src/agents/model_selector/premade_roles.yaml) that goose is aware of by default. Examples include:

- **deep-thinker**: Activates for complex reasoning tasks
- **debugger**: Switches in for error resolution
- **reviewer**: Monitors after extensive tool usage
- **coder**: Handles code implementation tasks
- **mathematician**: Processes mathematical computations

### Custom Roles

You can create custom roles with specific triggers by defining them in your `config.yaml` file:

```yaml
x-advanced-models:
- provider: openai
  model: gpt-4o
  role: custom-debugger
  rules:
    triggers:
      keywords: ["bug", "broken", "failing", "crash"]
      consecutive_failures: 1
    active_turns: 5
    priority: 15
```

<details>
<summary>Custom Role Configuration Fields</summary>

**Rule Configuration:**
| Parameter | Description | Values |
|-----------|-------------|---------|
| `triggers` | Conditions that activate the role | Object (see parameters below) |
| `active_turns` | Number of turns the rule stays active once triggered | Integer (default: 5) |
| `priority` | Selection priority when multiple roles match | Integer (higher wins, default: 0) |

**Trigger Parameters:**

| Parameter | Description | Values |
|-----------|-------------|---------|
| `keywords` | Words that activate the role | Array of strings |
| `match_type` | How to match keywords | "any", "all" |
| `complexity_threshold` | Minimum complexity level | "low", "medium", "high" |
| `consecutive_failures` | Failures in sequence | Integer |
| `first_turn` | Trigger on conversation start | Boolean |
| `source` | Message source filter | "human", "machine", "any" |

The previous table includes several common rule trigger parameters. For the complete list, see the `TriggerRules` struct in [`autopilot.rs`](https://github.com/block/goose/blob/main/crates/goose/src/agents/model_selector/autopilot.rs).

</details>
