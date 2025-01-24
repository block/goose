---
sidebar_position: 1
title: Using Extensions
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Using Extensions

:::info Supported Environments
Goose currently works only on **OSX** and **Linux** systems, and supports both **ARM** and **x86** architectures. If you'd like to request support for additional operating systems, please [open an issue on GitHub](https://github.com/block/goose/issues/new?template=Blank+issue) to let us know.
:::

Goose Extensions are add-ons that provide a way to extend the functionality of Goose by connecting with applications and tools you already use in your workflow. These extensions can be used to add new features, access data and resources, or integrate with other systems.

### Adding An Extension
When you install Goose, a few built-in extensions are included. In addition, you can add external extensions that were developed on the [Model Context Protocol (MCP)][mcp].

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    
    **To add an extension:**

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

    ```sh 
    What would you like to configure?
      Configure Providers
      Toggle Extensions
    > Add Extension


    What type of extension would you like to add?
    > Built-in Extension
      Command-line Extension
      Remote Extension

    Which Built-in extension would you like to enable?
      Developer Tools
      Non Developer
    > Jetbrains
    ```
  </TabItem>
  <TabItem value="ui" label="Goose UI">
    **Extensions can be installed directly from the [directory page][extensions-directory] to the Goose UI as shown below.** 
    
    ![Install Extension](../assets/guides/install-extension-ui.png)
  </TabItem>
</Tabs>

### Toggle Extensions

You can manage extensions by enabling or disabling them based on your workflow needs. Both, the CLI and UI, allow you to toggle extensions on or off as necessary.

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    **To enable or disable extensions that are already installed:** 

    1. Run the following command to open up Goose's configurations:
    ```sh
    goose configure
    ```
    2. Select `Toggle Extensions` from the menu.
    3. A list of already installed extensions will populate.
    4. Press the `space bar` to toggle the extension `enabled` or `disabled`. 

    **Example:**

    To select an option during configuration, hover over it and press Enter.
    ```sh
    What would you like to configure?
      Configure Providers
    > Toggle Extensions
      Add Extension

    Enable systems: (use "space" to toggle and "enter" to submit)
    [ ] Developer Tools 
    [X] JetBrains
    ```
  </TabItem>
  <TabItem value="ui" label="Goose UI">
  **To enable or disable extensions that are already installed:**

  1. Click the three dots in the top-right corner of the application.
  2. Select `Settings` from the menu, then click on the `Extensions` section.
  2. Use the toggle switch next to each extension to enable or disable it.

  ![Install Extension](../assets/guides/manage-extensions-ui.png)

  </TabItem>
</Tabs>

## Additional Resources

Visit the [Installation Guide][installation-guide] for detailed instructions on how to update your LLM provider.

[providers]: /docs/configuration/providers
[handling-rate-limits]: /docs/guides/handling-llm-rate-limits-with-goose
[mcp]: https://www.anthropic.com/news/model-context-protocol
[installation-guide]: /docs/installation/#update-a-provider
[extensions-directory]: https://silver-disco-nvm6v4e.pages.github.io/