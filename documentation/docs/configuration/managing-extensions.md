---
sidebar_position: 2
title: Managing Goose Extensions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Goose Extensions are add-ons that provide are a way to extend the functionality of Goose. They also provide a way to connect Goose with applications and tools you already use in your workflow. These extensions can be used to add new features, automate tasks, or integrate with other systems. Extensions are based on the [Model Context Protocol (MCP)](https://github.com/modelcontextprotocol) so you can connect
goose to a wide ecosystem of capabilities.

## Managing Extensions in Goose

You can add extensions to goose through its configuration system. 

```
goose configure
```

After the first time setup, configure will let you choose to add extensions. Head to "Add Extension" to see your
options. You can also always edit the config file directly, which is stored in `~/.config/goose/config.yaml`. 

### Built-in Extensions
Goose starts out of the box with it's Developer extension enabled, giving it a shell to run commands and a way to edit files
on your filesystem.

Goose has some other builtin extensions. Run `goose configure` and head to "Add Extension" > "Built-in Extension" to see the options.

:::tip

All of the goose builtin extensions are MCP servers in their own right. If you'd like
to use the MCP servers included with goose with any other agent, you are free to do so!
You can run them with
```
goose mcp {name}
```

:::

### MCP Servers

You can run any MCP server as a goose extension. Head to `goose configure` > "Add Extension" > "Command-line Extension". 
That will let you enter a command and any environment variables needed. For example, to connect to the [Fetch Server](https://github.com/modelcontextprotocol/servers/tree/main/src/fetch), enter `uvx mcp-server-fetch` as the command.

You can also edit the resulting config entry directly, which should look like this:

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


### Discovering Extensions

Goose comes with a [central directory](https://silver-disco-nvm6v4e.pages.github.io/) of extensions that you can install and use. You can install extensions from the Goose CLI or from the Goose GUI. The page will give you a test command to try out extensions, and then
if you want to keep them, you can add through `goose configure`. 

You can test out an extension for a single session with

```
goose session --with-extension "command to run"
```


## Starting a Goose Session with Extensions

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
Goose extensions are implemented with the Model Context Protocol (MCP) - a system that allows AI models and agents to securely connect with local or remote resources using standard protocols. Learn how to build your own [extension as an MCP server](https://modelcontextprotocol.io/quickstart/server).
