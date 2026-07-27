---
title: DialMCP Extension
description: Add DialMCP as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

DialMCP is a hosted remote MCP server that lets AI agents place real phone calls from your own SMS-verified number. Every call returns a transcript, a recording, and a structured outcome. US & Canada only.

## Quick Install

:::tip
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Install DialMCP extension](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.dialmcp.com%2Fmcp&id=dialmcp&name=DialMCP&description=Place%20real%20phone%20calls%20from%20your%20verified%20number%20with%20transcripts%20and%20recordings)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Use `goose configure` to add a `Remote Extension (Streamable HTTP)` with:

  **Endpoint URL**

  ```
  https://mcp.dialmcp.com/mcp
  ```
  </TabItem>
</Tabs>
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="dialmcp"
      extensionName="DialMCP"
      description="Place real phone calls from your verified number, with transcripts and recordings"
      type="streamable_http"
      url="https://mcp.dialmcp.com/mcp"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="dialmcp"
      type="streamable_http"
      url="https://mcp.dialmcp.com/mcp"
      description="Place real phone calls from your verified number, with transcripts and recordings"
    />
  </TabItem>
</Tabs>

## Example Usage

- "Call the restaurant at +1-415-555-0100 and ask if they have a table for 2 at 7pm tonight."
- "Phone my dentist's office and confirm my appointment on Thursday."
- "Call this vendor and get a status update on order 1842; summarize any commitments they make."

## Notes

- Auth is OAuth 2.1 (phone number + SMS). No API keys.
- Calls present your own verified caller ID.
- Scope: US & Canada, 8:00–21:00 local time; rate-limited.
- Website: https://dialmcp.com
- Connector / docs: https://github.com/SkillfulAgents/dialmcp-connector
