---
title: Vercel Extension
description: Add Vercel MCP Server as a goose Extension for managing your deployments
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/-Y-werFKtTw" />

This tutorial covers how to add the [Vercel MCP Server](https://vercel.com/docs/mcp/vercel-mcp) as a goose extension to manage your Vercel projects and deployment status directly from goose.

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run installation commands, which use `npx`.
:::

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?url=https%3A%2F%2Fmcp.vercel.com&type=streamable_http&id=vercel&name=Vercel&description=Access%20deployments%2C%20manage%20projects%2C%20and%20more%20with%20Vercel%E2%80%99s%20official%20MCP%20server)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    ```sh
    npx mcp-remote https://mcp.vercel.com
    ```
  </TabItem>
</Tabs>
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="Vercel"
      extensionName="Vercel"
      description="Access Vercel deployments and manage projects"
      type="http"
      url="https://mcp.vercel.com"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Vercel"
      description="Access Vercel deployments and manage projects"
      type="http"
      url="https://mcp.vercel.com"
      timeout={300}
    />
  </TabItem>
</Tabs>

## Example Usage

### goose Prompt
```
Get a list of my Vercel projects via the Vercel MCP server, and build me a table showing when I last deployed the project and other details I should know about their status.
```

The first time you use the Vercel MCP server, you will be prompted to authenticate with Vercel via OAuth.

### goose Output
```
I'll help you get your Vercel projects and create a table with deployment details. Let me first get your teams to identify which team to use for listing projects.

> List Teams

> teamId: team_p3hPvXImXmdkbZ36FoqhoyYo


Now let me get the deployment information for each project to see the last deployment dates and status:

> projectId, teamId

> projectId, teamId

> projectId, teamId

Now let me get more details about each project to provide a comprehensive table:

> projectId, teamId

> projectId, teamId

> projectId, teamId

Based on the data from your Vercel projects, here's a comprehensive table showing your project status and deployment information:

Vercel Projects Overview
```
Project Name|Framework|Status|Last Deployment|Custom Domain|GitHub Repo|Notes
---|---|---|---|---|---|---
project1|Next.js|🟡 Building|Sep 8, 2025 (Today)|None|about-me|Currently building - newest project
project2|Next.js|✅ Ready|May 12, 2025|example.com|starter-blog|Notes for site 2
project3|Static|✅ Ready|Oct 25, 2024|None|sample.com|Notes for Site 3

```
Key Details & Status Summary

🟡 project1 (Currently Building)
Created: Today (Sep 8, 2025)
Status: Currently building (deployment in progress)
Last Commit: "first" (Sep 8, 2025)
Node Version: 22.x
Issue: This is your newest project and the deployment is still in progress

✅ project2 (Production Ready)
Created: Aug 21, 2024
Status: Production ready and live
Custom Domain: example.com (with www redirect)
Last Update: "grammar fix" (May 12, 2025)
Node Version: 20.x
Activity: Most active project with 12 deployments
Note: One failed deployment on Mar 12 but recovered successfully

✅ project3 (Production Ready)
Created: Oct 25, 2024
Status: Production ready
Last Update: "updated public to remove localhost" (Oct 25, 2024)
Node Version: 20.x
Activity: Single successful deployment

Recommendations
- Monitor the building project: Check on project1 as it's currently building
- Portfolio maintenance: Your project2 is well-maintained and actively used
- Domain setup: Consider adding a custom domain to your other projects if needed
- Node versions: You have mixed Node versions (20.x and 22.x) - consider standardizing
- All projects are connected to GitHub repositories and have automatic deployments configured.
```