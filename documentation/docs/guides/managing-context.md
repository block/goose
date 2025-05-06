---
title: Managing Context Length
sidebar_position: 5
sidebar_label: Managing Context
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Goose provides context management features in the CLI to help you maintain productive sessions even when reaching model context limits. This guide explains how context management works and how to use it effectively.

## Understanding Context Management

Before diving into the features, let's understand the key concepts:

- **Context Length** is the amount of conversation history the AI model can consider
- **Context Limit** is the maximum number of tokens the model can process
- **Context Management** is how Goose handles conversations when approaching these limits

:::info
Context management in the CLI helps you continue working in the same session instead of starting over when hitting context limits.
:::

## Context Management Features

When a conversation reaches the context limit, Goose offers different ways to handle it:

| Feature | Description | Best For | Impact |
|---------|-------------|-----------|---------|
| **Summarization** | Condenses conversation while preserving key points | Long, complex conversations | Maintains most context |
| **Truncation** | Removes oldest messages to make room | Simple, linear conversations | Loses old context |
| **Clear** | Starts fresh while keeping session active | New direction in conversation | Loses all context |

## Using Context Management

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>

When you reach the context limit in Goose Desktop:

1. You'll see a notification that the context limit has been reached
2. You'll need to start a new session to continue your conversation

:::tip
You can use the Previous Sessions feature to reference information from your earlier session.
:::

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

When you reach the context limit in the CLI, you'll see a prompt like this:

```sh
◇  The model's context length is maxed out. You will need to reduce the # msgs. Do you want to?
│  ○ Clear Session   
│  ○ Truncate Message
// highlight-start
│  ● Summarize Session
// highlight-end

final_summary: [A summary of your conversation will appear here]

Context maxed out
--------------------------------------------------
Goose summarized messages for you.
```

After choosing an option and the context is managed, you can continue your conversation in the same session.

  </TabItem>
</Tabs>

## Benefits of Smart Context Management

### 1. Workflow Continuity
- Continue working in the same session
- Maintain conversation momentum
- Avoid starting over

### 2. Context Preservation
- Keep important information accessible
- Preserve tool interactions and their results
- Access full history when needed

### 3. Improved Productivity
- Reduce disruptions from context limits
- Maintain focus on your task
- Seamless conversation flow

## Best Practices

:::tip
Consider summarizing proactively before hitting context limits if you're working on a long task.
:::

Here are some tips for effective context management:

1. **Regular Checkpoints**
   - Save important conclusions or decisions
   - Break complex tasks into smaller segments
   - Use summarization at natural break points

2. **Summary Management**
   - Review generated summaries for accuracy
   - Edit summaries to highlight key points
   - Include critical context for your current task

3. **Context Awareness**
   - Monitor conversation length
   - Keep track of important information
   - Consider task complexity when choosing management strategy


:::warning
While context management helps maintain longer sessions, it's still good practice to periodically start fresh sessions for complex tasks.
:::