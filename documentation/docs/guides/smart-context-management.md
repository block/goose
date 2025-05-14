---
title: Smart Context Management
sidebar_position: 22
sidebar_label: Smart Context Management
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { ScrollText } from 'lucide-react';

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

Goose Desktop exclusively uses summarization to manage context, preserving key information while reducing size.

<Tabs>
  <TabItem value="automatic" label="Automatic" default>

When you reach the context limit in Goose Desktop:

1. Goose will automatically start summarizing the conversation to make room.
2. You'll see a message that says **"Preparing summary..."**, followed by **"Session summarized."**
3. Once complete, you'll have the option to **"View or edit summary."**
4. You can then continue the session with the summarized context in place.

  </TabItem>
  <TabItem value="manual" label="Manual">

You can proactively summarize your conversation before reaching context limits:

1. Click the scroll text icon (<ScrollText className="inline" size={16} />) in the chat interface
2. Confirm the summarization in the modal
3. View or edit the generated summary if needed

  </TabItem>
</Tabs>

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

The CLI offers three context management options: summarize, truncate, or clear your session.

<Tabs>
  <TabItem value="automatic" label="Automatic" default>

When you hit the context limit, you'll see this prompt to choose a management option, allowing you to continue your session:

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

  </TabItem>
  <TabItem value="manual" label="Manual">

To proactively trigger summarization before reaching context limits, use the `/summarize` command:

```sh
( O)> /summarize
◇  Are you sure you want to summarize this conversation? This will condense the message history.
│  Yes 
│
Summarizing conversation...
Conversation has been summarized.
Key information has been preserved while reducing context length.
```

  </TabItem>
</Tabs>

  </TabItem>
</Tabs>