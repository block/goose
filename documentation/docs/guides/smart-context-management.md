---
title: Smart Context Management
sidebar_position: 5
sidebar_label: Smart Context Management
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Goose provides context management features to help you maintain productive sessions even when reaching model context limits. This guide explains how context management works and how to use it effectively.

## Understanding Context Management

Before diving into the features, let's understand the key concepts:

- **Context Length** is the amount of conversation history the AI model can consider
- **Context Limit** is the maximum number of tokens the model can process
- **Context Management** is how Goose handles conversations when approaching these limits

## Context Management Features

When a conversation reaches the context limit, Goose offers different ways to handle it:

| Feature | Description | Best For | Impact |
|---------|-------------|-----------|---------|
| **Summarization** | Condenses conversation while preserving key points | Long, complex conversations | Maintains most context |
| **Truncation** | Removes oldest messages to make room | Simple, linear conversations | Loses old context |
| **Clear** | Starts fresh while keeping session active | New direction in conversation | Loses all context |

## Using Context Management

Goose has features that help you continue working in the same session instead of starting over when hitting context limits. This is particularly useful for maintaining flow during complex tasks or long conversations.

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

## Benefits

- **Continue Working**: Stay in the same session without starting over
- **Preserve Context**: Keep important information and tool interactions
- **Maintain Flow**: Reduce disruptions from context limits

## Best Practices

- **Be Proactive**: Consider managing context before hitting limits on long tasks
- **Review Summaries**: Ensure important information is preserved accurately
- **Choose Wisely**: Pick the right management strategy for your task type