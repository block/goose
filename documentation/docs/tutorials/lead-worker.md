---
description: Enable multi-model functionality by pairing LLMs to complete your tasks
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Lead/Worker Multi-Model Setup

goose supports a lead/worker model configuration that lets you pair two different AI models - one that's great at thinking and another that's fast at doing. This setup tackles a major pain point: premium models are powerful but expensive, while cheaper models are faster but can struggle with complex tasks. With lead/worker mode, you get the best of both worlds.

<details>
  <summary>Lead/Worker Mode Walkthrough</summary>
  <iframe
    class="aspect-ratio"
    src="https://youtube.com/embed/ZyhUTsChFUw"
    title="Lead/Worker Mode Setup and Settings Explained"
    frameBorder="0"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
    allowFullScreen
  ></iframe>
</details>

The lead/worker model is a smart hand-off system. The "lead" model (think: GPT-4 or Claude Opus) kicks things off, handling the early planning and big picture reasoning. Once the direction is set, goose hands the task over to the "worker" model (like GPT-4o-mini or Claude Sonnet) to carry out the steps.

If things go sideways (e.g. the worker model gets confused or keeps making mistakes), goose notices and automatically pulls the lead model back in to recover. Once things are back on track, the worker takes over again.

## Turn-Based System

A **turn** is one full interaction - your prompt and the model's response. goose switches models based on turns:

- **Initial turns** (default: 3) go to the lead model
- **Subsequent turns** use the worker model
- **Fallback kicks in** if the worker model fails too many times in a row
- **Recovery** returns the session to the worker model once things stabilize


## Quick Example

You might configure goose like this:

```bash
export GOOSE_LEAD_MODEL="gpt-4o"          # strong reasoning
export GOOSE_MODEL="gpt-4o-mini"          # fast execution
export GOOSE_PROVIDER="openai"
```

goose will start with `gpt-4o` for the first three turns, then hand off to `gpt-4o-mini`. If the worker gets tripped up twice in a row, goose temporarily switches back to the lead model for two fallback turns before trying the worker again.

## Configuration

:::tip
Ensure you have [added the LLMs to goose](/docs/getting-started/providers)
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   1. Click the model name at the bottom of the goose Desktop window
   2. Click **Lead/Worker Settings**
   3. Check the box to **Enable lead/worker mode**
   4. Select your **Lead Model** and **Worker Model** from the dropdown menus
   5. (Optional) Change the default number of **initial lead turns**, the **failure threshold** before switching back to the leavd model, or the number of **fallback turns** to use the lead model during fallback
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    The only required configuration is setting the `GOOSE_LEAD_MODEL` [environment variable](/docs/guides/environment-variables#leadworker-model-configuration):

    ```bash
    export GOOSE_LEAD_MODEL="gpt-4o"
    ```
    That's it. goose treats your regular `GOOSE_MODEL` as the worker model by default.

    For more control, you can also set these optional environment variables:

    ```bash
    export GOOSE_LEAD_PROVIDER="anthropic"         # If different from the main provider
    export GOOSE_LEAD_TURNS=5                      # Use lead model for first 5 turns
    export GOOSE_LEAD_FAILURE_THRESHOLD=3          # Switch back to lead after 3 failures
    export GOOSE_LEAD_FALLBACK_TURNS=2             # Use lead model for 2 turns before retrying worker
    ```
    After making these configurations, the lead/worker models will be used in new CLI and Desktop sessions.
    </TabItem>
</Tabs>  

## What Counts as a Failure?

goose is smart about detecting actual task failures, not just API errors. The fallback kicks in when the worker:

- Generates broken code (syntax errors, tool failures, missing files)
- Hits permission issues
- Gets corrected by the user ("that's wrong", "try again", etc.)

Technical hiccups like timeouts, authentication issues, or service downtime don't trigger fallback mode. goose retries those quietly.

## Reasons to Use Lead/Worker

- **Lower your costs** by using cheaper models for routine execution
- **Speed things up** while still getting solid plans from more capable models
- **Mix and match providers** (e.g., Claude for reasoning, OpenAI for execution)
- **Handle long dev sessions** without worrying about model fatigue or performance

## Best Practices

If you're just getting started, the default settings will work fine. But here's how to tune things:

- Bump up `GOOSE_LEAD_TURNS` to 5–7 for heavier planning upfront
- Lower `GOOSE_LEAD_FAILURE_THRESHOLD` to 1 if you want goose to correct issues quickly
- Choose a fast, lightweight worker model (Claude Haiku, GPT-4o-mini) for day-to-day tasks

For debugging, you can see model switching behavior by turning on this log:

```bash
export RUST_LOG=goose::providers::lead_worker=info
```

## Planning Mode Compatibility
The lead/worker model is an automatic alternative to the [goose CLI's `/plan` command](/docs/guides/creating-plans). You can assign separate models to use as the lead/worker and planning models. For example:

```bash
export GOOSE_PROVIDER="openai"
export GOOSE_MODEL="gpt-4o-mini"        # the main conversational model

export GOOSE_LEAD_MODEL="o1-preview"    # the lead model used automatically
export GOOSE_PLANNER_MODEL="gpt-4o"     # the model used when you explicitly call /plan
```

Use **planning mode** when you want a dedicated reasoning model to generate comprehensive strategies that you can review and approve before execution. Use the **lead/worker model** for iterative development work where you want smart automation without interruption - like implementing features, debugging issues, or exploratory coding. Your workflow can combine both: use `/plan` to strategize major decisions, then let the lead/worker models handle the tactical implementation with automatic optimization. 
