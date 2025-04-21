---
title: Managing Tool Permissions
sidebar_position: 4
sidebar_label: Tool Permissions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Tool permissions provide fine-grained control over how Goose uses different tools within extensions. This guide will help you understand and configure these permissions effectively.

## Understanding Tools and Extensions

Before diving into permissions, let's clarify the key components:

- **Extensions** are packages that add functionality to Goose (like Developer, Google Drive, etc.)
- **Tools** are specific functions within each extension that Goose can use

For example, the Developer extension includes multiple tools like:

- Text editor tool for file editing
- Shell tool for running commands
- Screen capture tool for taking screenshots
:::warning Performance Optimization
Goose performs best with fewer than 25 total tools enabled across all extensions. Consider enabling only the extensions you need for your current task.
:::

## Permission Levels

Each tool can be set to one of three permission levels:

| Permission Level | Description | Best For | Examples |
|-----------------|-------------|-----------|----------|
| **Always Allow** | Tool runs without requiring approval | Safe, read-only operations | • File reading<br></br>• Directory listing<br></br>• Information retrieval |
| **Ask Before** | Requires confirmation | State-changing operations | • File writing/editing<br></br>• System commands<br></br>• Resource creation |
| **Never Allow** | Tool cannot be used | Sensitive operations | • Credential access<br></br>• System-critical files<br></br>• Resource deletion |

:::info
Tool permissions work alongside [Goose Permission Modes](/docs/guides/goose-permissions). The mode sets default behavior, while tool permissions let you override specific tools.
:::

## Configuring Tool Permissions

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>

    1. Run the configure command:
    ```sh
    goose configure
    ```

    2. Select `Goose Settings` from the menu
    ```sh
    ┌ goose-configure
    │
    ◆ What would you like to configure?
    | ○ Configure Providers
    | ○ Add Extension
    | ○ Toggle Extensions
    | ○ Remove Extension
    // highlight-start
    | ● Goose Settings
    // highlight-end
    └
    ```

    3. Choose `Tool Permission`
    ```sh
    ┌   goose-configure
    │
    ◇  What would you like to configure?
    │  Goose Settings
    │
    ◆  What setting would you like to configure?
    │  ○ Goose Mode
    // highlight-start
    │  ● Tool Permission
    // highlight-end
    |  ○ Tool Output
    └
    ```

    4. Select an extension and configure permissions for its tools:
    ```sh
    ┌   goose-configure
    │
    ◇  What setting would you like to configure?
    │  Tool Permission 
    │
    ◇  Choose an extension to configure tools
    │  developer 
    │
    ◇  Choose a tool to update permission
    │  developer__image_processor 
    │
    ◆  Set permission level for tool developer__image_processor, current permission level: Not Set
    │  ○ Always Allow 
     // highlight-start
    │  ● Ask Before (Prompt before executing this tool)
    // highlight-end
    │  ○ Never Allow 
    └
    ```
  </TabItem>
  <TabItem value="ui" label="Goose Desktop">

    You can configure tool permissions through either Manual or Smart Approval modes:

     <Tabs>
      <TabItem value="manual" label="Manual Approval" default>
        1. Click `...` in the upper right corner
        2. Click `Advanced Settings`
        3. Under `Mode Selection`, choose `Manual Approval`
        4. Click on an extension name
        5. Use the dropdown next to each tool to set its permission level
      </TabItem>
      <TabItem value="smart" label="Smart Approval">
      :::tip
        In Smart Approval mode, Goose will automatically detect and allow read-only operations while requiring approval for state-changing actions.
      :::
        1. Click `...` in the upper right corner
        2. Click `Advanced Settings`
        3. Under `Mode Selection`, choose `Smart Approval`
        4. Click on an extension name
        5. Use the dropdown next to each tool to set its permission level
      </TabItem>
    </Tabs>   
  </TabItem>
</Tabs>

## Benefits of Permission Management

:::tip
Review and update your tool permissions as your tasks change. You can modify permissions at any time during a session.
:::

There are several reasons to configure tool permissions:

1. **Performance Optimization**
   - Keep total enabled tools under 25 for best performance
   - Disable tools you don't need for your current task
   - Reduce context window usage and improve response quality
   - Prevent tool decision paralysis

2. **Security Control**
   - Restrict access to sensitive operations
   - Prevent accidental file modifications
   - Control system resource usage

3. **Task Focus**
   - Enable only tools needed for current task
   - Help Goose make better tool choices
   - Reduce noise in responses

## Example Permission Configuration

### Task-Based Configuration

Configure permissions based on your current task:

```
Development Task:
✓ File reading → Always Allow
✓ Code editing → Ask Before
✓ Test running → Always Allow
✗ System commands → Ask Before

Documentation Task:
✓ File reading → Always Allow
✓ Markdown editing → Always Allow
✗ Code editing → Never Allow
✗ System commands → Never Allow
```
