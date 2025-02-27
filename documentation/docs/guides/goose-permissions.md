---
sidebar_position: 3
title: Goose Permissions
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Managing Goose Permissions

Goose’s **permissions** determine how much autonomy it has when modifying files, using extensions, and performing automated actions. By selecting a permission mode, you have full control over how Goose interacts with your development environment.

## Permission Modes

| Mode        | Description | Best For |
|------------|------------|---------|
| **Auto Mode** | Goose can modify files, use extensions, and delete files **without requiring approval**. | Users who want **full automation** and seamless integration into their workflow. |
| **Approve Mode** | Goose **asks for confirmation** before modifying, creating, deleting files and before using extensions. | Users who want to **review and approve** changes and extension use before they happen. |
| **Chat Mode** | Goose **only engages in chat**, with no extension use or file modifications. | Users who prefer a **conversational AI experience** without automation. |

:::warning
    `Auto Mode` is applied by default unless you specify otherwise.
:::

## Configuring Goose Permissions
Here's how to configure your chosen goose permissions:

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    1. Run the following command: 

    ```sh
    goose configure
    ```

    2. Select `Goose Settings` from the menu and press Enter.

    ```sh
   ┌   goose-configure 
   │
   ◆  What would you like to configure?
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
   4. Choose the Goose mode you would like to configure.

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
    :::info 🚀 Goose Modes in Desktop – Coming Soon!
        Currently, Goose Modes can only be configured via the CLI. 
        
        By default, Goose Desktop operates in **Auto Mode**, allowing full automation.   
        
        A future update will bring **Goose Mode selection** to the Desktop app. Stay tuned!
    :::
  </TabItem>
</Tabs>