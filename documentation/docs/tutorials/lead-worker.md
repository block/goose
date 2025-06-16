---
description: Enable multi-modal functionality by pairing LLMs to complete your tasks
---

# Lead/Worker Multi-Model Setup

Goose supports a lead/worker model configuration that lets you pair two different AI models - one that's great at thinking and another that's fast at doing. This setup tackles a major pain point: premium models are powerful but expensive, while cheaper models are faster but can struggle with complex tasks. With lead/worker mode, you get the best of both worlds.

The lead/worker model is a smart hand-off system. The "lead" model (think: GPT-4 or Claude Opus) kicks things off, handling the early planning and big picture reasoning. Once the direction is set, Goose hands the task over to the "worker" model (like GPT-4o-mini or Claude Sonnet) to carry out the steps.

If things go sideways (e.g. the worker model gets confused or keeps making mistakes), Goose notices and automatically pulls the lead model back in to recover. Once things are back on track, the worker takes over again.

## Turn-Based System

A **turn** is one full interaction - your prompt and the model's response. Goose switches models based on turns:

- **Initial turns** (default: 3) go to the lead model
- **Subsequent turns** use the worker model
- **Fallback kicks in** if the worker model fails too many times in a row
- **Recovery** returns the session to the worker model once things stabilize


## Quick Example

You might configure Goose like this:

```bash
export GOOSE_LEAD_MODEL="gpt-4o"          # strong reasoning
export GOOSE_MODEL="gpt-4o-mini"          # fast execution
export GOOSE_PROVIDER="openai"
```

Goose will start with `gpt-4o` for the first three turns, then hand off to `gpt-4o-mini`. If the worker gets tripped up twice in a row, Goose temporarily switches back to the lead model for two fallback turns before trying the worker again.

## Configuration

:::tip
Ensure you have [added the LLMs to Goose](/docs/getting-started/providers)
:::

The only required setting is:

```bash
export GOOSE_LEAD_MODEL="gpt-4o"
```

That's it. Goose treats your regular `GOOSE_MODEL` as the worker model by default.

If you want more control, here are the optional knobs:

```bash
export GOOSE_LEAD_PROVIDER="anthropic"         # If different from the main provider
export GOOSE_LEAD_TURNS=5                      # Use lead model for first 5 turns
export GOOSE_LEAD_FAILURE_THRESHOLD=3          # Switch back to lead after 3 failures
export GOOSE_LEAD_FALLBACK_TURNS=2             # Use lead model for 2 turns before retrying worker
```

After making these configurations, the lead/worker models will be used in new CLI and Desktop sessions.

## What Counts as a Failure?

Goose is smart about detecting actual task failures, not just API errors. The fallback kicks in when the worker:

- Generates broken code (syntax errors, tool failures, missing files)
- Hits permission issues
- Gets corrected by the user ("that's wrong", "try again", etc.)

Meanwhile, technical hiccups like timeouts, auth issues, or service downtime don't trigger fallback mode. Goose just retries those quietly.

## Reasons to Use Lead/Worker

- **Lower your costs** by using cheaper models for routine execution
- **Speed things up** while still getting solid plans from more capable models
- **Mix and match providers** (e.g., Claude for reasoning, OpenAI for execution)
- **Handle long dev sessions** without worrying about model fatigue or performance

## Best Practices

If you're just getting started, the default settings will work fine. But here's how to tune things:

- Bump up `GOOSE_LEAD_TURNS` to 5–7 for heavier planning upfront
- Lower `GOOSE_LEAD_FAILURE_THRESHOLD` to 1 if you want Goose to correct issues quickly
- Choose a fast, lightweight worker model (Claude Haiku, GPT-4o-mini) for day-to-day tasks

For debugging, you can see model switching behavior by turning on this log:

```bash
export RUST_LOG=goose::providers::lead_worker=info
```

## Planning Mode Compatibility

Lead/worker mode also works alongside Goose's `/plan` command. You can even assign separate models for each:

```bash
export GOOSE_LEAD_MODEL="o1-preview"        # used automatically
export GOOSE_PLANNER_MODEL="gpt-4o"         # used when you explicitly call /plan
export GOOSE_MODEL="gpt-4o-mini"            # used for execution
```

---

The lead/worker model helps you work smarter with Goose. You get high quality reasoning when it matters and save time and money on execution. And with the fallback system in place, you don't have to babysit it. It just works.