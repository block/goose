---
sidebar_position: 3
title: Goose Permission Modes
sidebar_label: Goose Permissions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft } from 'lucide-react';

Goose’s permissions determine how much autonomy it has when modifying files, using extensions, and performing automated actions. By selecting a permission mode, you have full control over how Goose interacts with your development environment.

<details>
  <summary>Permission Modes Video Walkthrough</summary>
  <iframe
  class="aspect-ratio"
  src="https://www.youtube.com/embed/bMVFFnPS_Uk"
  title="Goose Permission Modes Explained"
  frameBorder="0"
  allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
  allowFullScreen
  ></iframe>
</details>

## Permission Modes

| Mode               | Description                                                                                           | Best For                                                                                   |
|--------------------|-------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------|
| **Completely Autonomous**           | Goose can modify files, use extensions, and delete files **without requiring approval**.              | Users who want **full automation** and seamless integration into their workflow.           |
| **Manual Approval**| Goose **asks for confirmation** before using any tools or extensions.                                 | Users who want to **review and approve** every change and tool usage.                      |
| **Smart Approval** | Goose uses a risk-based approach to **automatically approve low-risk actions** and **flag others** for approval. | Users who want a **balanced mix of autonomy and oversight** based on the action’s impact. |
| **Chat Only**      | Goose **only engages in chat**, with no extension use or file modifications.                          | Users who prefer a **conversational AI experience** without automation.                    |
       |

:::warning
`Autonomous Mode` is applied by default.
:::

## Configuring Goose Mode

Here's how to configure:

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>

    You can change modes before or during a session and it will take effect immediately.

     <Tabs groupId="method">
      <TabItem value="session" label="In Session" default>

      Click the Goose Mode option from the bottom menu. 
      </TabItem>
      <TabItem value="settings" label="From Settings">
        1. Click the <PanelLeft className="inline" size={16} /> button on the top-left to open the sidebar.
        2. Click the `Settings` button on the sidebar.
        3. Click `Chat`.
        4. Under `Mode`, choose the mode you'd like.
      </TabItem>
    </Tabs>   
  </TabItem>
  <TabItem value="cli" label="Goose CLI">

    <Tabs groupId="method">
      <TabItem value="session" label="In Session" default>
        To change modes mid-session, use the `/mode` command.

        * Autonomous: `/mode auto`
        * Approve: `/mode approve`
        * Chat: `/mode chat`     
      </TabItem>
      <TabItem value="settings" label="From Settings">
        1. Run the following command:

        ```sh
        goose configure
        ```

        2. Select `Goose Settings` from the menu and press Enter.

        ```sh
        ┌ goose-configure
        │
        ◆ What would you like to configure?
        | ○ Configure Providers
        | ○ Add Extension
        | ○ Toggle Extensions
        | ○ Remove Extension
        // highlight-start
        | ● Goose Settings (Set the Goose Mode, Tool Output, Experiment and more)
        // highlight-end
        └
        ```

        3. Choose `Goose Mode` from the menu and press Enter.

        ```sh
        ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  Goose Settings
        │
        ◆  What setting would you like to configure?
        // highlight-start
        │  ● Goose Mode (Configure Goose mode)
        // highlight-end
        |  ○ Tool Output
        └
        ```

        4.  Choose the Goose mode you would like to configure.

        ```sh
        ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  Goose Settings
        │
        ◇  What setting would you like to configure?
        │  Goose Mode
        │
        ◆  Which Goose mode would you like to configure?
        // highlight-start
        │  ● Auto Mode
        // highlight-end
        |  ○ Approve Mode
        |  ○ Smart Approve Mode    
        |  ○ Chat Mode
        |
        └  Set to Auto Mode - full file modification enabled
        ```     
      </TabItem>
    </Tabs>
  </TabItem>
</Tabs>

  :::info
  If you choose `Approve` mode, you will see "Allow" and "Deny" buttons in your session windows during tool calls. 
  Goose will only ask for permission for tools that it deems are 'write' tools, e.g. any 'text editor write', 'text editor edit', 'bash - rm, cp, mv' commands. 
  
  Read/write approval makes best effort attempt at classifying read or write tools. This is interpreted by your LLM provider. 
  :::
