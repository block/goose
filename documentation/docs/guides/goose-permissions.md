---
sidebar_position: 3
title: Goose Permissions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Managing Goose Modes

Goose’s permissions determine how much autonomy it has when modifying files, using extensions, and performing automated actions. By selecting a permission mode, you have full control over how Goose interacts with your development environment.

## Permission Modes

| Mode             | Description                                                                                             | Best For                                                                               |
| ---------------- | ------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| **Auto Mode**    | Goose can modify files, use extensions, and delete files **without requiring approval**.                | Users who want **full automation** and seamless integration into their workflow.       |
| **Approve Mode** | Goose **asks for confirmation** before all tools and extensions. | Users who want to **review and approve** any changes and extension use before they happen. |
| **Chat Mode**    | Goose **only engages in chat**, with no extension use or file modifications.                            | Users who prefer a **conversational AI experience** without automation.                |

:::warning
`Auto Mode` is applied by default.
:::

## Configuring Goose Mode

Here's how to configure:

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
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
    |  ○ Chat Mode
    |
    └  Set to Auto Mode - full file modification enabled
    ```

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">

    1. Click `...` in the upper right corner
    2. Click `Settings`
    3. Scroll down to `Others` section
    4. Under `Mode Selection`, choose the mode you'd like

    :::info
    If you choose `Approve` mode, you will see "Allow" and "Deny" buttons in your session windows during tool calls. Goose will only ask for permission before tool call for tools that it deems are 'write' tools, for example any 'text editor write', 'text editor edit', 'bash - rm, cp, mv' commands, as an example. Read write approval makes best effort attempt at classifying read or write tools- this is interpreted by your LLM provider. 
    :::

  </TabItem>
</Tabs>
