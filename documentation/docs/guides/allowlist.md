---
sidebar_position: 90
title: goose Extension Allowlist
sidebar_label: Extension Allowlist
---

goose is an extensible framework that, by default, allows you to install any MCP server. However, you may want stricter controls on which MCP servers can be installed as extensions (e.g. in a corporate setting). 

This guide explains how you can create an **allowlist** of safe extensions that work with goose Desktop and CLI. An allowlist lets administrators control which MCP servers can be installed as goose extensions. When enabled, goose will only install extensions that are on the list, and will block installation of any others.

## How It Works

1. The allowlist is a YAML file that contains a list of allowed extension commands.
2. goose fetches the allowlist from a URL specified by the `GOOSE_ALLOWLIST` environment variable.
3. The allowlist is fetched when first needed and is cached. It is refetched on every restart of goose.
4. When a user attempts to install an extension, goose checks the MCP server's installation command against the allowlist.
5. If the command is not in the allowlist, the extension installation is rejected.

## Configuration

### 1. Create and Deploy Allowlist

The allowlist must be a YAML file with the following structure:

```yaml
extensions:
  - id: extension-id-1
    command: command-name-1
  - id: extension-id-2
    command: command-name-2
  # ... more extensions
```

#### Example

In this example, only the Slack, GitHub, and Jira extensions can be installed: 

```yaml
extensions:
  - id: slack
    command: uvx mcp_slack
  - id: github
    command: uvx mcp_github
  - id: jira
    command: uvx mcp_jira
```


After creating the allowlist, you must deploy it to a URL.


### 2. Set Environment Variable

Create an environment variable called `GOOSE_ALLOWLIST` and set the value to the URL of your YAML file:

```bash
export GOOSE_ALLOWLIST=https://example.com/goose-allowlist.yaml
```

You can also add this export to your shell configuration file (On a Mac, it's your `~/.bashrc` or `~/.zshrc` file). 

:::info
If this environment variable is not set, no allowlist restrictions are applied. With no restrictions, all extensions can be installed.
:::


## Best Practices

To effectively use the allowlist with exact matching:

1. **Specify commands**: Define the exact command string that you want to allow.
2. **Include full paths**: If you want to allow a command only from a specific path, include the full path in the allowlist.
3. **Audit regularly**: Review your allowlist frequently to ensure it only contains the commands you intend to allow.
4. **Use HTTPS**: Use an HTTPS URL for your allowlist to prevent man-in-the-middle attacks.
5. **Restrict edit access**: Ensure that only authorized users can edit the allowlist.
6. **Validate entries**: Carefully review the allowlist to ensure only trusted commands are included.
7. **Monitor installations**: Watch for rejected commands during extension installation, which might indicate attempted abuse.


## Troubleshooting

If extensions are being rejected unexpectedly:

1. Check if the `GOOSE_ALLOWLIST` environment variable is set correctly.
2. Verify that the allowlist file is accessible from the server.
3. Ensure the allowlist file is properly formatted YAML.
4. Check [server logs](/docs/guides/logs) for any errors related to fetching or parsing the allowlist.
5. Verify that the command in the extension installations exactly matches what's in the allowlist.