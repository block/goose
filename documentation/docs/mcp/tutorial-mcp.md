---
title: Tutorial Extension
description: Learn how to use Goose's built-in Tutorial extension for guided learning
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

The Tutorial extension is a built-in feature of Goose that provides interactive, step-by-step guidance for learning various aspects of Goose and its capabilities. It's designed to help users get comfortable with Goose's features through hands-on practice.

The Tutorial extension serves as an interactive learning tool that:
- Provides structured, step-by-step tutorials
- Allows hands-on practice with Goose features
- Offers immediate feedback and guidance

## Configuration

1. Ensure the Tutorial extension is enabled:

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="Tutorial"
  />
  </TabItem>
  <TabItem value="cli" label="Goose CLI">

```sh
goose configure
```

2. Choose to add a `Built-in Extension`
```sh
┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension (Connect to a new extension) 
│
◆  What type of extension would you like to add?
// highlight-start  
│  ● Built-in Extension (Use an extension that comes with Goose)
// highlight-end  
│  ○ Command-line Extension 
│  ○ Remote Extension (SSE) 
│  ○ Remote Extension (Streaming HTTP) 
└  
```

3. Select the `Tutorial` extension
```sh
┌   goose-configure 
│
◇  What would you like to configure?
│  Add Extension (Connect to a new extension) 
│
◇  What type of extension would you like to add?
│  Built-in Extension 
│
◆  Which built-in extension would you like to enable?
│  ○ Developer Tools 
│  ○ Computer Controller 
// highlight-start  
│  ● Tutorial
// highlight-end
│  ○ Memory 
|  ○ JetBrains
└ 
```

4. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
```sh
   ┌   goose-configure 
   │
   ◇  What would you like to configure?
   │  Add Extension (Connect to a new extension) 
   │
   ◇  What type of extension would you like to add?
   │  Built-in Extension 
   │
   ◇  Which built-in extension would you like to enable?
   │  Tutorial
   │
   // highlight-start
   ◆  Please set the timeout for this tool (in secs):
   │  300
   // highlight-end
   └  Enabled Tutorials extension
``` 
</TabItem>
</Tabs>

## Available Tutorials

Currently, the following tutorials are available:

- **build-mcp-extension**: Learn how to build an extension using the Model Context Protocol (MCP)
- **first-game**: Create your first game with Goose

More tutorials are being added regularly to cover additional features and use cases.

## Using the Tutorial Extension

### Starting a Tutorial

There are two main ways to interact with tutorials:

1. **Ask Goose directly**: You can simply ask about a topic you're interested in, and Goose will suggest relevant tutorials.

```
( O)> I'd like to learn how to build an extension for Goose
```

2. **Request a specific tutorial**: You can directly request a tutorial by name.

```
( O)> Can you walk me through the first-game tutorial?
```

### During the Tutorial

While going through a tutorial:

1. Goose will guide you step-by-step through the process
2. You'll receive clear instructions before any actions are taken
3. You can ask questions at any time for clarification
4. You can take breaks and resume later

### Best Practices

- **Take your time**: Don't rush through the tutorials. Make sure you understand each step before moving on.
- **Ask questions**: If something isn't clear, ask Goose to explain it further.
- **Experiment**: Feel free to try variations of what you're learning.
- **Take notes**: Document any interesting findings or techniques you learn.

## Example Usage

### Starting a Tutorial Session

Here's an example of how to start learning about building extensions:

### Goose Prompt
```
I'd like to learn how to build an extension for Goose
```

### Goose Output
```
I'll help you learn about building extensions! The "build-mcp-extension" tutorial would be perfect for this. Let me load it and guide you through it.

─── load_tutorial | tutorial ──────────────────────────
name: build-mcp-extension



Before we dive into building your extension, I need to understand a few things to better guide you:

1. Which SDK would you prefer to use? We have three options:
   - Python (beginner-friendly, great for prototyping)
   - TypeScript (great for web-based tools)
   - Kotlin (robust, type-safe, good for complex applications)

2. What type of extension would you like to build? For example:
   - A tool that performs specific actions
   - A resource that provides data to the LLM
   - Or do you have a specific idea in mind?

Let me know your preferences, and I'll help you get started with the appropriate setup and implementation!

For reference, here's what we'll cover:
1. Setting up your development environment
2. Creating the basic MCP server
3. Implementing your specific functionality
4. Testing and debugging

Which SDK would you like to use to get started?
```

## Next Steps

After completing tutorials, you can:
- Apply learned concepts to your own projects
- Contribute to the Goose community by sharing your experiences
- Suggest new tutorial topics that would be helpful for others

### Need Further Help? 
If you have questions, run into issues, or just need to brainstorm ideas join the [Discord Community][discord]!

[discord]: https://discord.gg/block-opensource