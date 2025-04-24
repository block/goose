---
sidebar_position: 19
title: Goose Extension Allowlist
sidebar_label: Goose Extension Allowlist
---

Goose is an extensible framework that, by default, allows you to install any MCP server. However when using Goose in a corporate setting, you may want stricter controls on which MCP servers can be installed as extensions. This guide explains how your organization can create an approved list of safe extensions that work with both the desktop app and command line version of Goose. This keeps your system secure by making sure only trusted extensions can run commands. The approved list is known as an **allowlist**.

An allowlist lets administrators control which commands can run in Goose's Stdio extensions. This security feature stops harmful extensions from running unapproved commands on your computer. When turned on, the Goose server agent only allows commands that are on the list. Any command not on this list are blocked, and a Goose user see an error message.


## How It Works

1. Goose fetches the allowlist from a URL specified by the `GOOSE_ALLOWLIST` environment variable.
2. The allowlist is a YAML file that contains a list of allowed extension commands.
3. The allowlist is fetched once when first needed and cached for the lifetime of the Goose server.
4. When a Stdio extension is [added](/docs/getting-started/using-extensions#adding-extensions), Goose checks the command against the allowlist.
5. If the command is not in the allowlist, the extension registration is rejected.

## Configuration

### Setting the Allowlist URL

Set the `GOOSE_ALLOWLIST` environment variable to the URL of your allowlist YAML file:

```bash
export GOOSE_ALLOWLIST=https://example.com/goose-allowlist.yaml
```

You can also add this export to your shell configuration file (On a Mac, it's your `~/.bashrc` or `~/.zshrc` file). 

:::info
If this environment variable is not set, no allowlist restrictions are applied. With no restrictions, all commands are allowed.
:::

### Allowlist File Format

The allowlist file needs to be a YAML file with the following structure:

```yaml
extensions:
  - id: extension-id-1
    command: command-name-1
  - id: extension-id-2
    command: command-name-2
  # ... more extensions
```

#### Example
In this example, only the Slack, Github, and Jira extenstions can be enabled in a Goose client: 
```yaml
extensions:
  - id: slack
    command: uvx mcp_slack
  - id: github
    command: uvx mcp_github
  - id: jira
    command: uvx mcp_jira
```

### Command Matching

When a Stdio extension attempts to register with a command, the Goose server agent does the following:

1. Extracts the base command name (the last part of the path)
   - For example, `/Users/username/bin/mcp slack` becomes `mcp slack`
2. Checks if this base command **exactly matches** any of the command strings in the allowlist
3. Allows the extension if there's a match, rejects it otherwise

### Special Cases

There are a few special cases in the command matching logic:

1. **goosed commands**: Any command that is either exactly "goosed" or ends with "/goosed" is always allowed, regardless of the allowlist. This ensures that the Goose server itself can always be executed.

2. **No allowlist**: If no allowlist is configured (the `GOOSE_ALLOWLIST` environment variable is not set), all commands are allowed.

3. **Empty allowlist**: If the allowlist is empty (contains no entries), all commands are allowed.

### Best Practices for Defining Allowlist Entries

To effectively use the allowlist with exact matching:

1. **Be specific**: Define the exact command string that you want to allow.
2. **Include full paths if needed**: If you want to allow a command only from a specific path, include the full path in the allowlist.
3. **Regular auditing**: Regularly audit your allowlist to ensure it only contains the commands you intend to allow.

## Security Considerations

1. **HTTPS**: Always use HTTPS URLs for your allowlist to prevent man-in-the-middle attacks.
2. **Access Control**: Ensure the allowlist URL is only accessible to authorized users.
3. **Validation**: The allowlist file should be carefully reviewed to ensure only trusted commands are included.
4. **Monitoring**: Monitor extension registrations for any rejected commands, which might indicate attempted abuse.

To implement monitoring for rejected commands, use your centralized logging infrastructure and configure Goose clients to send their logs to your logging system.

## Troubleshooting

If extensions are being rejected unexpectedly:

1. Check if the `GOOSE_ALLOWLIST` environment variable is set correctly.
2. Verify that the allowlist file is accessible from the server.
3. Ensure the allowlist file is properly formatted YAML.
4. Check [server logs](/docs/guides/logs) for any errors related to fetching or parsing the allowlist.
5. Verify that the command in the extension registration exactly matches what's in the allowlist.

## Example Usage

1. Create and host an allowlist file:

```yaml
# allowlist.yaml
extensions:
  - id: slack
    command: uvx mcp_slack
  - id: github
    command: uvx mcp_github
```

2. Start goose-server with the allowlist URL:

```bash
export GOOSE_ALLOWLIST=https://secure-server.example.com/allowlist.yaml
/Applications/Goose.app/Contents/MacOS/Goosed agent
```

3. When extensions are registered, only those with commands matching the allowlist are accepted.