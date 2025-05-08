---
title: Smart Context Management
sidebar_position: 22
sidebar_label: Smart Context Management
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

When working with [Large Language Models (LLMs)](/docs/getting-started/providers), there are limits to how much conversation history they can process at once. Goose provides smart context management features to help you maintain productive sessions even when reaching these limits. Here are the key concepts:

- **Context Length**: The amount of conversation history the LLM can consider
- **Context Limit**: The maximum number of tokens the model can process
- **Context Management**: How Goose handles conversations approaching these limits

## Smart Context Management Features

When a conversation reaches the context limit, Goose offers different ways to handle it:

| Feature | Description | Best For | Impact |
|---------|-------------|-----------|---------|
| **Summarization** | Condenses conversation while preserving key points | Long, complex conversations | Maintains most context |
| **Truncation** | Removes oldest messages to make room | Simple, linear conversations | Loses old context |
| **Clear** | Starts fresh while keeping session active | New direction in conversation | Loses all context |

## Using Smart Context Management

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>

When you reach the context limit in Goose Desktop:

1. You'll see a notification that the context limit has been reached
2. You'll need to start a new session to continue your conversation

:::tip
You can access previous context by:
- Referencing information from your previous sessions
- Using the [Memory extension](/docs/tutorials/memory-mcp) to maintain context across sessions and reference information from previous conversations
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