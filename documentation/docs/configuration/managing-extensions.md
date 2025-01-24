---
sidebar_position: 2
title: Managing Goose Extensions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Extensions are add-ons that provide a way to extend the functionality of Goose. They also provide a way to connect Goose with applications and tools you already use in your workflow. These extensions can be used to add new features, automate tasks, or integrate with other systems. 

Extensions are based on the [Model Context Protocol (MCP)](https://github.com/modelcontextprotocol), so you can connect
Goose to a wide ecosystem of capabilities.


## Adding Extensions

You can add extensions to Goose through the CLI or the UI.

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
  
  1. After the initial Goose setup, You can add extensions through the configuration system. 

    ```
    goose configure
    ```

  2. Choose `Add Extension` to see the options. 
  
  You can also edit the config file directly, which is stored in `~/.config/goose/config.yaml`. 

  </TabItem>
  <TabItem value="ui" label="Goose UI">
  1. Locate the menu (...) in the top right corner of the Goose UI.
  2. Select `Settings` from the menu.
  3. Under `Extensions`, you can add (+) a new extension manually, 
  4. Or [`Browse Extensions`][extensions] to find curated extensions.
  5. Click 'Install' on extension you'd like to add and it installs right in the Goose app.
  </TabItem>
</Tabs>

## Removing Extensions

You can remove extensions installed on Goose 

<Tabs groupId="interface">
<TabItem value="cli" label="Goose CLI" default>
    At the moment, you can remove extensions by editing the config file directly, which is stored in `~/.config/goose/config.yaml`.
  </TabItem>
  <TabItem value="ui" label="Goose UI">

  1. Locate the menu (...) in the top right corner of the Goose UI.
  2. Select `Settings` from the menu.
  3. Under `Extensions`, find the extension you'd like to remove and click on the settings icon beside it.
  4. In the dialog that appears, click `Remove Extension`.

  </TabItem>
</Tabs>

## Built-in Extensions
Out of the box, Goose is installed with a few extensions out of the box but with only the `Developer` extension enabled by default.

Here are the default extensions:

1. **Developer**: The `Developer` extension provides a set of general development tools that are useful for software development.
2. **Non-Developer**: The `Non-Developer` extension provides general computer control tools that don't require you to be a developer or engineer.
3. **Memory**: The `Memory` extension teaches goose to remember your preferences as you use it
4. **JetBrains**: The `JetBrains` extension provides an integration for working with JetBrains IDEs.
5. **Google Drive**: The `Google Drive` extension provides an integration for working with Google Drive for file management and access.


#### Toggling Built-in Extensions

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    1. Run Goose configuration:
    
    ```bash
      goose configure
    ```
    2. Choose `Add Extension`
    3. Choose `Built-in Extension`

    Alternatively, you can enable a built-in extension by specifying its name in this command:

    ```
    goose mcp {name}
    ```

  </TabItem>
  <TabItem value="ui" label="Goose UI">
  1. Locate the menu (...) in the top right corner of the Goose UI.
  2. Select `Settings` from the menu.
  3. Under `Extensions`, you can toggle the built-in extensions on or off.
  </TabItem>
</Tabs>


:::tip
All of Goose's built-in extensions are MCP servers in their own right. If you'd like
to use the MCP servers included with Goose with any other agent, you are free to do so.
:::

## MCP Servers

You can run any MCP server as a Goose extension. 

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>

    1. Run `goose configure`
    2. Choose `Add Extension`
    3. Choose `Command-line Extension`

    You'll then be prompted to enter a command and any environment variables needed. For example, to connect to the [Fetch Server](https://github.com/modelcontextprotocol/servers/tree/main/src/fetch), enter `uvx mcp-server-fetch` as the command.

    You can also edit the resulting config entry directly, which would look like this:

    ```yaml
    extensions:
      fetch:
        name: fetch
        cmd: uvx
        args: [mcp-server-fetch]
        enabled: true
        envs: {}
        type: stdio
    ```


  </TabItem>
  <TabItem value="ui" label="Goose UI">

  1. Locate the menu (...) in the top right corner of the Goose UI.
  2. Select `Settings` from the menu.
  3. Under `Extensions`, you can add a MCP server as an extension manually by clicking on the (+) button to the right.
  4. In the dialog that appears, enter the details of the MCP server including any environment variables needed.
  </TabItem>
</Tabs>


## Discovering Extensions

Goose comes with a [central directory][extensions] of extensions that you can install and use. You can install extensions from the Goose CLI or from the Goose GUI. The page will give you a test command to try out extensions, and if you want to keep them, you can add through `goose configure`. 

You can test out an extension for a single session with

```sh
goose session --with-extension "command to run"
```


## Starting a Session with Extensions

You can start a tailored goose session with specific extensions directly from the CLI. To do this, run the following command:

```bash
goose session --with-extension "{extension command}"
```

:::note
You may need to set necessary environment variables for the extension to work correctly.
```bash
goose session --with-extension "VAR=value command arg1 arg2"
```
:::

## Developing Extensions
Goose extensions are implemented with MCP - a system that allows AI models and agents to securely connect with local or remote resources using standard protocols. Learn how to build your own [extension as an MCP server](https://modelcontextprotocol.io/quickstart/server).


[extensions]: https://block.github.io/goose/v1/extensions