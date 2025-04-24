---
sidebar_position: 3
title: Using Extensions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Extensions are add-ons that provide a way to extend the functionality of Goose by connecting with applications and tools you already use in your workflow. These extensions can be used to add new features, access data and resources, or integrate with other systems.

Extensions are based on the [Model Context Protocol (MCP)](https://github.com/modelcontextprotocol), so you can connect
Goose to a wide ecosystem of capabilities.

:::tip Tutorials
Check out the [step-by-step tutorials](/docs/category/tutorials) for adding and using several Goose Extensions
:::


## Built-in Extensions
Out of the box, Goose is installed with a few extensions but with only the `Developer` extension enabled by default.

Here are the built-in extensions:

1. **Developer**: provides a set of general development tools that are useful for software development.
2. **Computer Controller**: provides general computer control tools for webscraping, file caching, and automations.
3. **Memory**: teaches goose to remember your preferences as you use it
4. **JetBrains**: provides an integration for working with JetBrains IDEs.


#### Toggling Built-in Extensions

<Tabs groupId="interface">

  <TabItem value="cli" label="Goose CLI" default>
    
    If you know the exact name of the extension you'd like to add, run:

    ```sh
    goose mcp {name}
    ```

    To navigate through available extensions:

    1. Run the following command:
    ```sh
    goose configure
    ```
    2. Select `Add Extension` from the menu.
    3. Choose the type of extension you’d like to add:
        - `Built-In Extension`: Use an extension that comes pre-installed with Goose.
        - `Command-Line Extension`: Add a local command or script to run as an extension.
        - `Remote Extension`: Connect to a remote system via SSE (Server-Sent Events).
    4. Follow the prompts based on the type of extension you selected.

    **Example: Adding Built-in Extension**

    To select an option during configuration, hover over it and press Enter.

    ```
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Built-in Extension 
    │
    ◆  Which built-in extension would you like to enable?
    │  ○ Developer Tools 
    │  ○ Computer Controller (controls for webscraping, file caching, and automations)
    │  ○ Google Drive 
    │  ○ Memory 
    │  ● JetBrains 
    │        
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │ 
    └  Enabled jetbrains extension    
    ```
  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. Click `...` in the top right corner of the Goose Desktop.
  2. Select `Advanced Settings` from the menu.
  3. Under `Extensions`, you can toggle the built-in extensions on or off.
  </TabItem>
</Tabs>


:::info
All of Goose's built-in extensions are MCP servers in their own right. If you'd like
to use the MCP servers included with Goose with any other agent, you are free to do so.
:::


## Discovering Extensions

Goose provides a [central directory][extensions-directory] of extensions that you can install and use. 

You can also add any other [MCP Server](#mcp-servers) as a Goose extension, even if it's not listed in our directory.


## Adding Extensions

Extensions can be installed directly via the [extensions directory][extensions-directory], CLI, or UI.

### MCP Servers

You can install any MCP server as a Goose extension. 

:::tip MCP Server Directory
See available servers in the **[MCP Server Directory](https://www.pulsemcp.com/servers)**.
:::

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
  
  1. Run the following command: 

    ```sh
    goose configure
    ```

  2. Select `Add Extension` from the menu.

  3. Choose the type of extension you’d like to add:
      - `Built-In Extension`: Use an extension that comes pre-installed with Goose.
      - `Command-Line Extension`: Add a local command or script to run as an extension.
      - `Remote Extension`: Connect to a remote system via SSE (Server-Sent Events).

  4. Follow the prompts based on the type of extension you selected.

  #### Example of adding the [Knowledge Graph Memory MCP Server](https://github.com/modelcontextprotocol/servers/tree/main/src/memory):

<Tabs groupId="extensions">
   <TabItem value="node" label="Node">
  ```
 ┌   goose-configure 
 │
 ◇  What would you like to configure?
 │  Add Extension 
 │
 ◇  What type of extension would you like to add?
 │  Command-line Extension 
 │
 ◇  What would you like to call this extension?
 │  Knowledge Graph Memory
 │
 ◇  What command should be run?
 │  npx -y @modelcontextprotocol/server-memory
 │
 ◇  Please set the timeout for this tool (in secs):
 │  300
 │
 ◆  Would you like to add environment variables?
 │  No 
 │
 └  Added Knowledge Graph Memory extension
 ```

   </TabItem>
   <TabItem value="python" label="Python">

  ```
 ┌   goose-configure
 │
 ◇  What would you like to configure?
 │  Add Extension
 │
 ◇  What type of extension would you like to add?
 │  Command-line Extension
 │
 ◇  What would you like to call this extension?
 │  Wikipedia Reader
 │
 ◇  What command should be run?
 │  uvx mcp-wiki
 │
 ◇  Please set the timeout for this tool (in secs):
 │  300
 │
 ◆  Would you like to add environment variables?
 │  No
 │
 └  Added Wikipedia Reader extension
 ```

   </TabItem>
   <TabItem value="java" label="Java">

Note: Java and Kotlin extensions are only support on Linux and macOS

  ```
 ┌   goose-configure
 │
 ◇  What would you like to configure?
 │  Add Extension
 │
 ◇  What type of extension would you like to add?
 │  Command-line Extension
 │
 ◇  What would you like to call this extension?
 │  Spring Data Explorer
 │
 ◇  What command should be run?
 │  jbang -Dspring.profiles.active=dev org.example:spring-data-mcp:1.0.0
 │
 ◇  Please set the timeout for this tool (in secs):
 │  300
 │
 ◆  Would you like to add environment variables?
 │  Yes
 │
 ◇  Environment variable name:
 │  SPRING_DATASOURCE_URL
 │
 ◇  Environment variable value:
 │  jdbc:postgresql://localhost:5432/mydb
 │
 ◇  Add another environment variable?
 │  No
 │
 └  Added Spring Data Explorer extension
 ```

   </TabItem>
  </Tabs>

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
 
  1. Click `...` in the top right corner of the Goose Desktop.
  2. Select `Advanced Settings` from the menu.
  3. Under `Extensions`, click `Add custom extension`.
  4. On the `Add custom extension` modal, enter the necessary details
     - If adding an environment variable, click `Add` button to the right of the variable
     - The `Timeout` field lets you set how long Goose should wait for a tool call from this extension to complete
  5. Click `Add` button
  
  #### Example of adding the [Knowledge Graph Memory MCP Server](https://github.com/modelcontextprotocol/servers/tree/main/src/memory):
    * **Type**: `Standard IO`
    * **ID**: `kgm-mcp` (_set this to whatever you want_)
    * **Name**: `Knowledge Graph Memory` (_set this to whatever you want_)
    * **Description**: `maps and stores complex relationships between concepts` (_set this to whatever you want_)
    * **Command**: `npx -y @modelcontextprotocol/server-memory`
  </TabItem>
</Tabs>

### Config Entry
For advanced users, you can also directly edit the config file (`~/.config/goose/config.yaml`) to add, remove, or update an extension:

```yaml
extensions:
  fetch:
    name: GitHub
    cmd: npx
    args: [-y @modelcontextprotocol/server-github]
    enabled: true
    envs: { "GITHUB_PERSONAL_ACCESS_TOKEN": "<YOUR_TOKEN>" }
    type: stdio
    timeout: 300
```
    

## Enabling/Disabling Extensions

You can enable or disable installed extensions based on your workflow needs.

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    1. Run the following command to open up Goose's configurations:
    ```sh
    goose configure
    ```
    2. Select `Toggle Extensions` from the menu.
    3. A list of already installed extensions will populate.
    4. Press the `space bar` to toggle the extension. Solid means enabled. 

    **Example:**

    ```
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Toggle Extensions 
    │
    ◆  enable extensions: (use "space" to toggle and "enter" to submit)
    │  ◼ developer 
    │  ◻ fetch 
    └   
    ```
  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. Click the three dots in the top-right corner of the application.
  2. Select `Advanced Settings` from the menu, scroll down to the `Extensions` section.
  2. Use the toggle switch next to each extension to enable or disable it.

  </TabItem>
</Tabs>


## Removing Extensions

You can remove installed extensions. 

<Tabs groupId="interface">
<TabItem value="cli" label="Config file" default>
  :::info
  To remove an extension, you must [disable](#enablingdisabling-extensions) it first.
  :::

    1. Run the following command to open up Goose's configurations:
    ```sh
    goose configure
    ```
    2. Select `Remove` from the menu. Disabled extensions will be listed.
    3. Arrow down to the extension you want to remove.
    4. Press the `space bar` to select the extension. Solid means selected. 
    ```
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Remove Extension 
    │
    ◆  Select extensions to remove (note: you can only remove disabled extensions - use "space" to toggle and "enter" to submit)
    │  ◼ fetch 
    └  
    ```
    5. Press Enter to save
  </TabItem>
  <TabItem value="ui" label="Goose Desktop">

  1. Click `...` in the top right corner of the Goose Desktop.
  2. Select `Advanced Settings` from the menu.
  3. Under `Extensions`, find the extension you'd like to remove and click on the settings icon beside it.
  4. In the dialog that appears, click `Remove Extension`.

  </TabItem>
</Tabs>





## Starting Session with Extensions

You can start a tailored Goose session with specific extensions directly from the CLI. 

:::info Notes
* The extension will not be installed. It will only be enabled for the current session.
* There's no need to do this if you already have the extensions enabled.
:::

### Built-in Extensions

To enable a built-in extension while starting a session, run the following command:

```bash
goose session --with-builtin "{extension_id}"
```

For example, to enable the Developer and Computer Controller extensions and start a session, you'd run:

```bash
goose session --with-builtin "developer,computercontroller"
```

Or alternatively:

```bash
goose session --with-builtin developer --with-builtin computercontroller
```


### External Extensions

To enable an extension while starting a session, run the following command:

```bash
goose session --with-extension "{extension command}" --with-extension "{another extension command}"
```

For example, to start a session with the [Fetch extension](https://github.com/modelcontextprotocol/servers/tree/main/src/fetch), you'd run:

```bash
goose session --with-extension "uvx mcp-server-fetch"
```


#### Environment Variables

Some extensions require environment variables. You can include these in your command:

```bash
goose session --with-extension "VAR=value command arg1 arg2"
```

For example, to start a session with the [GitHub extension](https://github.com/modelcontextprotocol/servers/tree/main/src/github), you'd run:

```bash
goose session --with-extension "GITHUB_PERSONAL_ACCESS_TOKEN=<YOUR_TOKEN> npx -y @modelcontextprotocol/server-github"
```

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

### Remote Extensions over SSE

To enable a remote extension over SSE while starting a session, run the following command:

```bash
goose session --with-remote-extension "{extension URL}" --with-remote-extension "{another extension URL}"
```

For example, to start a session with a remote extension running on localhost on port 8080, you'd run:

```bash
goose session --with-remote-extension "http://localhost:8080/sse"
```

## Developing Extensions

Goose extensions are implemented with MCP, a standard protocol that allows AI models and agents to securely connect with local or remote resources. Learn how to build your own [extension as an MCP server](https://modelcontextprotocol.io/quickstart/server).

[extensions-directory]: https://block.github.io/goose/v1/extensions
