---
title: WhatSync Extension
description: Add WhatSync as a goose Extension to search, read and send WhatsApp messages
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add [WhatSync](https://whatsync.app) as a goose extension so goose can search your WhatsApp chats, see who is waiting for a reply, and send messages when you ask it to.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fwhatsync.app%2Fapi%2Fmcp&id=whatsync&name=WhatSync&description=Search%2C%20read%20and%20send%20WhatsApp%20messages%20from%20a%20personal%20number%20or%20the%20WhatsApp%20Business%20API)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    Use `goose configure` to add a `Remote Extension (Streamable HTTP)` extension type with:

    **Endpoint URL**
    ```
    https://whatsync.app/api/mcp
    ```
  </TabItem>
</Tabs>
:::

## What is WhatSync?

WhatSync is a hosted WhatsApp MCP server. It works with a personal WhatsApp number, linked from WhatsApp's Linked devices screen the same way as WhatsApp Web, or with a number on the official WhatsApp Business API. You choose which chats and groups are saved and shared with your AI.

The server uses OAuth: the first time goose connects, your browser opens to sign in to WhatSync. If you have not linked WhatsApp yet, the tools reply with a link to the WhatSync dashboard where you can do that.

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="whatsync"
      extensionName="WhatSync"
      description="Search, read and send WhatsApp messages from a personal number or the WhatsApp Business API"
      type="http"
      url="https://whatsync.app/api/mcp"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="WhatSync"
      description="Search, read and send WhatsApp messages from a personal number or the WhatsApp Business API"
      type="http"
      url="https://whatsync.app/api/mcp"
    />
  </TabItem>
</Tabs>

## Example Usage

**See who is waiting on you**
```
Which WhatsApp chats are waiting for my reply?
```

**Catch up on a group**
```
Summarise what the Product Team group discussed this week.
```

**Find a file**
```
Find the invoice PDF Rahul sent me.
```

**Send a reply**
```
Reply to Neha that the report will be ready by 1 pm. Show me the message first.
```

Sending reaches real people. Ask goose to show you the recipient and the exact text before it sends.

## Resources

- Website: [whatsync.app](https://whatsync.app)
- MCP setup guide: [whatsync.app/whatsapp-mcp](https://whatsync.app/whatsapp-mcp)
- CLI and agent skill: [github.com/itsanishjain/whatsync-cli](https://github.com/itsanishjain/whatsync-cli)
