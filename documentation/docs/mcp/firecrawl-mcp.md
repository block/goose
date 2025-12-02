---
title: Firecrawl Extension
description: Add Firecrawl MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

<YouTubeShortEmbed videoUrl="https://youtube.com/embed/_rKBFLREVcM" /> 


This tutorial will get you started with the [Firecrawl MCP Server](https://github.com/firecrawl/firecrawl-mcp-server) as a goose extension to enable powerful web scraping, crawling, and search capabilities for AI agents.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=firecrawl-mcp&id=firecrawl&name=Firecrawl&description=Web%20scraping%20and%20crawling%20capabilities&env=FIRECRAWL_API_KEY%3DYour%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y firecrawl-mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  FIRECRAWL_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="firecrawl"
    extensionName="Firecrawl"
    description="Web scraping and crawling capabilities"
    command="npx"
    args={["-y", "firecrawl-mcp"]}
    envVars={[{ name: "FIRECRAWL_API_KEY", label: "Your Firecrawl API Key" }]}
    apiKeyLink="https://firecrawl.dev/app/api-keys"
    apiKeyLinkText="Firecrawl API Key"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="firecrawl"
      description="Web scraping and crawling capabilities"
      type="stdio"
      command="npx -y firecrawl-mcp"
      timeout={300}
      envVars={[
        { key: "FIRECRAWL_API_KEY", value: "<Your Firecrawl API Key>" }
      ]}
      infoNote={
        <>
          Obtain your <a href="https://firecrawl.dev/app/api-keys" target="_blank" rel="noopener noreferrer">Firecrawl API Key</a> and paste it in.
        </>
      }
    />
    </TabItem>
</Tabs>

## Example Usage

The Firecrawl MCP server provides comprehensive web scraping and crawling capabilities, including:

- **Single Page Scraping**: Extract clean, structured content from individual web pages
- **Batch Processing**: Scrape multiple URLs efficiently with batch operations
- **Site Crawling**: Crawl entire websites or specific sections with customizable depth
- **Search Integration**: Search across crawled content with advanced filtering

### goose Prompt

```
I'm building a multiplayer game and trying to decide between Photon and Unity Netcode. Can you scrape their official documentation and recent comparison articles to help me understand the pros and cons of each platform, including performance, pricing, ease of use, and scalability?
```

### goose Output

```
I'll help you research Photon and Unity Netcode by scraping their official documentation and finding recent comparison articles. Let me start by gathering information from both platforms' official sites and then look for comparison articles.

──────────────  Firecrawl ─────────────────
Firecrawl Scrape url, formats, onlyMainContent

Firecrawl Scrape url, formats, onlyMainContent

Firecrawl Search query, limit, sources

Firecrawl Scrape url, formats, onlyMainContent

Firecrawl Scrape url, formats, onlyMainContent

Firecrawl Search query, limit, sources

Firecrawl Scrape url, formats, onlyMainContent

Firecrawl Search query, limit, sources

Firecrawl Scrape url, formats, onlyMainContent


Based on my research of the official documentation and recent discussions, I'll provide you with a comprehensive comparison of Photon and Unity Netcode for multiplayer game development.


Photon vs Unity Netcode: Comprehensive Comparison

Overview

Photon is a mature, third-party multiplayer networking solution with multiple products (PUN, Fusion, Quantum) that has been powering games for over a decade. Unity Netcode refers to Unity's first-party networking solutions, primarily Netcode for GameObjects and Netcode for Entities.


Performance Comparison

Photon Fusion

Bandwidth Usage: 6x more efficient than Unity's MLAPI/Mirror
Player Count: Supports thousands of networked objects across hundreds of client connections
Features: Tick-based simulation, client-side prediction, lag compensation, snapshot interpolation
Optimization: Advanced replication algorithms (full consistency vs eventual consistency)

Unity Netcode

Netcode for GameObjects: Designed for casual co-op games (smaller scale)
Netcode for Entities: Built for competitive action games with ECS architecture
Performance: Generally less optimized than Photon Fusion for bandwidth usage
Features: Client/server authoritative models, prediction, interpolation
Winner: Photon Fusion for raw performance and bandwidth efficiency


Pricing Comparison

Photon Pricing (2024)
  Free Tier:
  - 20 CCU for development only
  - 100 CCU for launched games (≈40k MAU)

  Paid Plans:
  - 500 CCU: $95/month (≈200k MAU)
  - 1,000 CCU: $195/month (≈400k MAU)
  - 2,000 CCU: $395/month (≈800k MAU)
  - Premium Cloud: $580/month minimum (auto-scaling up to 50k CCU)

  One-time Options:
  - PUN Plus: $95 one-time for 100 CCU (12 months)

Unity Gaming Services Pricing
  Free Tier:
  - Relay: First 50 average monthly CCU free
  - Lobby: 10 GiB/month bandwidth free
  - Multiplay: $800 credit for 6 months
  
  Paid Plans:
  - Relay: $0.16 per additional average CCU

  Multiplay Hosting:
  - CPU: $0.038/hour per core
  - RAM: $0.0051/GiB/hour
  - Network: $0.14/GiB

  No free tier after credits expire

Winner: Photon for small to medium games; Unity potentially cheaper for very large scale with dedicated servers


Ease of Use

Photon

Pros:
- Mature documentation and tutorials
- Large community and extensive examples
- Works out-of-the-box with minimal setup
- Cross-platform support (including WebGL without server setup)
Cons:
- Third-party dependency
- Learning curve for advanced features

Unity Netcode

Pros:
- First-party Unity integration
- Consistent with Unity's development patterns
- Direct support from Unity
Cons:
- Newer solution with evolving documentation
- Requires more setup for WebGL (needs Relay service)
- Steeper learning curve for Netcode for Entities

Winner: Photon for immediate productivity and WebGL deployment


Scalability

Photon
- Global Infrastructure: 15+ regions worldwide
- DDoS Protection: Built-in for higher tiers
- Auto-scaling: Available in Premium Cloud plans
- Maximum: Up to 50k CCU (Premium), unlimited (Enterprise)

Unity Netcode
- Infrastructure: Leverages Google Cloud
- Scaling: Manual server management required
- Distributed Authority: New topology for better scaling
- Maximum: Depends on your server infrastructure investment

Winner: Photon for managed scaling; Unity for custom infrastructure control


Platform Support

Photon
- Platforms: All major platforms including WebGL, mobile, console, VR/AR
- WebGL: Works seamlessly without dedicated servers
- Cross-play: Full support across all platforms

Unity Netcode
- Platforms: All Unity-supported platforms
- WebGL: Requires Unity Relay service for multiplayer
- Cross-play: Supported but requires additional setup

Winner: Tie - both support all major platforms


Key Considerations for Your Decision

Choose Photon if:
- You want to deploy quickly, especially to WebGL
- You need proven, battle-tested networking
- You prefer managed infrastructure
- Your game will have moderate CCU (under 10k)
- You want extensive documentation and community support

Choose Unity Netcode if:
- You want first-party Unity integration
- You plan to scale to very high CCU with custom servers
- You need tight control over server infrastructure
- You're building competitive games requiring ECS performance
- You want to avoid third-party dependencies

Recent Community Insights

From the Unity forums discussion, developers note:
- Photon Fusion offers 100 CCU free for one app, combinable with subscriptions
- Unity Multiplay has no free tier but provides $800 credit for 6 months
- WebGL deployment is simpler with Photon as it handles the server backend
- Migration between frameworks requires touching every networking script

Recommendation

For most indie and small-to-medium studios, Photon offers better value and ease of use, especially for WebGL deployment. For larger studios with dedicated infrastructure teams or those requiring maximum performance at scale, Unity Netcode with custom server management might be more cost-effective long-term.

The choice ultimately depends on your specific needs: immediate deployment ease (Photon) vs. long-term infrastructure control (Unity).
```
