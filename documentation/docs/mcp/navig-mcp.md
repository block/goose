---
title: NAVIG Extension
description: Add NAVIG as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [NAVIG MCP Server](https://github.com/navig-run/core) as a goose extension to search your own sessions, chats and files, read an encrypted local vault, and reach your own hosts over SSH — all on the machine goose runs on.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=uvx&arg=--from&arg=navig&arg=navig&arg=mcp&arg=serve&arg=--transport&arg=stdio&id=navig&name=NAVIG&description=Local%20operator%20tools%20over%20your%20own%20machine)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  uvx --from navig navig mcp serve --transport stdio
  ```
  </TabItem>
</Tabs>
:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uvx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="navig"
      extensionName="NAVIG"
      description="Local operator tools over your own machine"
      type="stdio"
      command="uvx"
      args={["--from", "navig", "navig", "mcp", "serve", "--transport", "stdio"]}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="NAVIG"
      description="Local operator tools over your own machine"
      type="stdio"
      command="uvx --from navig navig mcp serve --transport stdio"
      timeout={300}
    />
  </TabItem>
</Tabs>

NAVIG needs no API key and no account: it exposes the machine it runs on, and everything it
reads or changes is gated by its own approval layer. State lives in `~/.navig`.

## Example Usage

**Recall** something from your own past work:

```
Search my NAVIG sessions for the docker compose change I made to the web stack last week
```

**Read from the vault** without pasting a secret into the chat:

```
List the keys in my NAVIG vault under infra/, do not print any secret values
```

**Reach a host** you have already registered:

```
Check disk usage on the staging host through NAVIG and tell me which volume is closest to full
```

**Inspect the machine goose is running on**:

```
Use NAVIG to list the running containers and their exposed ports
```

## Available tools

The server exposes 130 tools, all namespaced `navig_*`. The ones most useful from a coding
agent are `navig_list_hosts`, `navig_host_info`, `navig_search_wiki`, `navig_list_wiki_pages`,
`navig_read_wiki_page`, `navig_list_databases`, and the recall tools.

## Notes

- Local only: stdio transport, no hosted service, no telemetry.
- Alternative install if you prefer a persistent CLI: `pipx install navig`, then run
  `navig mcp serve --transport stdio`.
- Uninstall: `uvx` caches nothing you must clean up; for the CLI path, `pipx uninstall navig`.
