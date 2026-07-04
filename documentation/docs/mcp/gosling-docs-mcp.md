---
title: gosling Docs Extension

description: Add gosling Docs MCP Server as a gosling Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GoslingDesktopInstaller from '@site/src/components/GoslingDesktopInstaller';


This tutorial covers how to add the [gosling Docs MCP Server](https://github.com/idosal/git-mcp) as a gosling extension to enable gosling to answer questions about itself.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="gosling Desktop" default>
  [Launch the installer](gosling://extension?cmd=npx&arg=mcp-remote&arg=https%3A%2F%2Frepo-makeover.gitmcp.io%2Fgosling%2F&id=gosling-docs&name=gosling%20Docs&description=gitmcp%20for%20gosling%20documentation)
  </TabItem>
  <TabItem value="cli" label="gosling CLI">
  **Command**
  ```sh
  npx mcp-remote https://repo-makeover.gitmcp.io/gosling/
  ```
  </TabItem>
</Tabs>
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="gosling Desktop" default>
    <GoslingDesktopInstaller
      extensionId="gosling-docs"
      extensionName="gosling Docs"
      description="GitMCP for gosling documentation"
      command="npx"
      args={["mcp-remote", "https://repo-makeover.gitmcp.io/gosling/"]}
      cliCommand="npx mcp-remote https://repo-makeover.gitmcp.io/gosling/"
      timeout={300}
      note="Note that you'll need Node.js installed on your system to run this command, as it uses npx."
    />
 </TabItem>
  <TabItem value="cli" label="gosling CLI">
      <CLIExtensionInstructions
        name="gosling Docs"
        description="GitMCP for gosling documentation"
        command="npx mcp-remote https://repo-makeover.gitmcp.io/gosling/"
        timeout={300}
      />
  </TabItem>
</Tabs>


## Example Usage
GitMCP allows you to convert any Git repository into a gosling extension. The gosling Docs MCP Server is a great example of this, as it provides a way for gosling to answer questions about itself.

Let's ask gosling to research its own documentation and write a setup guide for us! It will use the gosling Docs MCP Server to find the relevant information.


### gosling Prompt

```
hey gosling, my development team is new to gosling and I want to help them get set up with extensions. Can you research the gosling documentation to understand:

1. How extensions are configured (config file, CLI, and Desktop)
2. Which built-in extensions are enabled by default
3. How to add an external MCP server as an extension

Then write a concise setup guide as a reusable skill (a SKILL.md file) that teaches gosling to walk a new teammate through enabling the extensions we use.
```


### gosling Output

:::note Desktop

I researched the gosling documentation using the gosling Docs extension and wrote a reusable setup guide as a skill. Here's a summary of what I found and created:

#### 📋 Documentation Research

- **Configuring extensions**: Extensions are managed in `~/.config/gosling/config.yaml`, via `gosling configure` in the CLI, or from the Extensions page in gosling Desktop.
- **Built-in platform extensions**: Extension Manager, Summon, and Todo are enabled by default. Others like Chat Recall and Code Mode can be toggled on as needed.
- **External MCP servers**: Any MCP server can be added as an extension by providing the command (stdio) or URL (http) plus a name and timeout.

#### 🛠️ Generated Skill

I saved the guide as `.agents/skills/team-extension-setup/SKILL.md` so gosling can load it whenever a teammate asks for help getting set up.

```markdown
---
name: team-extension-setup
description: Walk a new teammate through enabling our standard gosling extensions
---

# Team Extension Setup

Guide the user through this checklist, one step at a time:

1. Verify gosling is installed and run `gosling configure`
2. Confirm the default platform extensions are enabled (Extension Manager, Summon, Todo)
3. Enable the Developer extension for file and shell access
4. Add our team MCP servers as extensions:
   - GitHub MCP server (requires GITHUB_TOKEN)
   - gosling Docs: `npx mcp-remote https://repo-makeover.gitmcp.io/gosling/`
5. Restart the session and ask gosling to list enabled extensions to verify

If a step fails, check `~/.config/gosling/config.yaml` for typos and confirm
required environment variables are set before retrying.
```

Your teammates can now say "use the team-extension-setup skill" in any session and gosling will walk them through the whole setup!
:::
