---
title: Dev.to Extension
description: Add Dev.to MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import { PanelLeft } from 'lucide-react';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/nkdksdxHxaQ" />  

This tutorial covers how to add the [Dev.to MCP Server](https://github.com/nickytonline/dev-to-mcp) as a Goose extension to enable access to the Dev.to public API. With this extension, Goose can fetch articles, tags, user info, comments, and more—without requiring authentication.

:::tip TLDR
**Endpoint URL**
```sh
http://localhost:3000/mcp
```
Make sure your server is running before adding the extension in Goose 
:::

## Configuration

:::info
Make sure you have Node.js and npm installed. You’ll also need to run `npm install` and `npm run build` once before using the server.
:::

1. First, clone and set up the `Dev.to MCP server`:
```bash
git clone https://github.com/nickytonline/dev-to-mcp.git
cd dev-to-mcp
npm install
npm run build # one-time setup
```
```bash
npm start
```
Your server will now be running at:
`http://localhost:3000/mcp`


<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
  2. Navigate to `Extensions`
  3. Click `Add Custom Extension`
  4. In `Extension Name`, enter: `dev.to`
  5. In the `Type` dropdown, select: `Streamable HTTP`
  6. (Optional) Add a Description 
  7. In `Endpoint URL`, enter:` http://localhost:3000/mcp`
  8. Set `Timeout` to `300` (default)
  9. Leave `Environment Variables` empty
  10. Leave `Request Headers` empty
  11. Click `Add Extension`

  </TabItem>
  <TabItem value="cli" label="Goose CLI" default>
      <CLIExtensionInstructions
        name="dev.to"
        type="http"
        url="http://localhost:3000/mcp"
        timeout={300}
      />
  </TabItem>
</Tabs>

## Example Usage

In this example, Goose uses the `Dev.to MCP` extension to fetch the 5 most recent JavaScript articles and instantly turn them into a personalized learning hub.

### Goose Prompt
```
Hey Goose, fetch the 5 most recent JavaScript articles from Dev.to
Create a simple HTML landing page that:

1. Lists each article title as a clickable link

2. Includes the author and a 1‑sentence summary under each title

3. Suggests a small project or code snippet I could build to apply what I learned from each article (if applicable)

4. Highlights which article I should read first if I only have time for one and explain why

The goal is to save me time by quickly showing what’s worth reading and help me learn faster by giving actionable practice ideas.
```

### Goose Output

:::note Desktop

I'll help you fetch the recent JavaScript articles from Dev.to and create a nice HTML learning page. Let me start by getting the articles.

`─── per_page state tag ──────────────────────────`

Now I'll create a beautiful HTML learning page with all the requested features:

`─── writing /Users/ebonyl/... ──────────────────────────`

Perfect! I've created a beautiful HTML landing page featuring the 5 most recent JavaScript articles from Dev.to. Here's what I've included:

🎯 Key Features
    - 📱 Responsive design with a modern gradient background and card-based layout
    - 🔗 Clickable article titles that open in new tabs
    - 👤 Author names and reading times for each article
    - 📝 One-sentence summaries explaining what each article covers
    - 🏷️ Tag display showing the relevant technologies
    - ⭐ Priority recommendation with visual highlighting
    - actionable projects to build to further learning
🎨 Design Elements
    - Gradient backgrounds and smooth hover effects
    - Clean typography using system fonts
    - Color-coded recommendation system
    - Responsive layout that works on all devices
    - Professional card-based design with shadows and animations
📖 Content Highlights

Top Recommendation: The Signal Clone tutorial (Part One) by Oluwabusayo Jacobs is highlighted as the must-read article because:

It's a comprehensive 29-minute deep dive
Covers real-world messaging app development
Includes authentication and real-time features
Uses React Native and Stream (highly transferable to web development)
Provides practical, hands-on learning experience

The landing page is saved as `javascript_articles_landing.html` and ready to open in your browser! Each article link will take you directly to the full Dev.to article for deeper reading.

![Dev.to Project Example](/img/dev.to-project.png)
:::
