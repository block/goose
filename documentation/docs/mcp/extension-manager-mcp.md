---
title: Extension Manager
description: Dynamically discover, enable, and disable extensions during goose sessions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PlatformExtensionNote } from '@site/src/components/PlatformExtensionNote';
import GooseBuiltinInstaller from '@site/src/components/GooseBuiltinInstaller';

You don't always need to manually manage extensions. The Extension Manager extension enables goose to dynamically discover, enable, and disable extensions during active sessions. Based on the task you give it, goose recognizes when it needs a specific extension, enables it when required, and suggests disabling unused extensions if the bloat is eating up your context window.

Simply describe your task, and goose will handle the extension management automatically.

## Configuration

<PlatformExtensionNote/>

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseBuiltinInstaller
    extensionName="Extension Manager"
    description="Dynamically discover, enable, and disable extensions during sessions"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">

  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to `Toggle Extensions`
  ```sh
  ┌   goose-configure 
  │
  ◇  What would you like to configure?
  │  Toggle Extensions 
  │
  ◆  Enable extensions: (use "space" to toggle and "enter" to submit)
  // highlight-start    
  │  ● extensionmanager
  // highlight-end  
  └  Extension settings updated successfully
  ```
  </TabItem>
</Tabs>

## Why Use Extension Manager?

goose can work with many extensions, but having too many enabled at once can:
- Overwhelm the LLM with too many tool choices
- Reduce the quality of tool selection
- Slow down response times
- Exceed recommended limits (5 extensions or 50 tools)

The Extension Manager solves this by letting goose:
- **Discover** what extensions are available
- **Enable** extensions only when needed for specific tasks
- **Disable** extensions when they're no longer required

This creates a more focused, efficient experience where goose has access to exactly the tools it needs, when it needs them.

:::tip Recommended Limits
For optimal performance, aim for **5 or fewer active extensions** with a total of **50 or fewer tools**. The Extension Manager helps you stay within these limits by enabling task-specific extensions only when needed.
:::

## Available Tools

The Extension Manager provides these tools:

| Tool | Description | Use Case |
|------|-------------|----------|
| `search_available_extensions` | Discover extensions that can be enabled or disabled | Finding the right extension for a task |
| `manage_extensions` | Enable or disable an extension by name | Loading/unloading extensions dynamically |
| `list_resources` | List resources from extensions (if supported) | Discovering available data sources |
| `read_resource` | Read specific resource content (if supported) | Accessing extension-provided data |

:::tip
The resource tools (`list_resources` and `read_resource`) are only available when at least one enabled extension supports resources.
:::

## Example Usage

Let's enable an extension when we need it. In this example, we'll enable the GitHub extension to work with repositories.

### goose Prompt

```
List all my GitHub repositories
```

### goose Output

:::note Desktop

```
I'll enable the GitHub extension for you so we can work with repositories.

MANAGE_EXTENSIONS
action: enable
extension_name: github

✅ The extension 'github' has been installed successfully

The GitHub extension is now active.

I'll list your GitHub repositories using the GitHub extension.

LIST_REPOSITORIES

Here are your repositories:

...

Would you like to work with any of these repositories?
```

:::