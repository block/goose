# Goose Extension Allowlist

This document describes the extension allowlist feature in goose-server, which provides a security mechanism for controlling which commands can be executed by extensions.

## Overview

The allowlist feature enables administrators to restrict which commands can be executed by Stdio extensions in Goose. This is an important security measure that prevents potentially malicious extensions from executing unauthorized commands on the system.

When enabled, the server will only allow execution of commands that match entries in the allowlist. Commands that are not in the allowlist will be rejected with an error message.

## How It Works

1. The allowlist is fetched from a URL specified by the `GOOSE_ALLOWLIST` environment variable.
2. The allowlist is a YAML file that contains a list of allowed extension commands.
3. The allowlist is fetched once when first needed and cached for the lifetime of the server.
4. When a Stdio extension is registered, the command is checked against the allowlist.
5. If the command is not in the allowlist, the extension registration is rejected.

## Configuration

### Setting the Allowlist URL

Set the `GOOSE_ALLOWLIST` environment variable to the URL of your allowlist YAML file:

```bash
export GOOSE_ALLOWLIST=https://example.com/goose-allowlist.yaml
```

If this environment variable is not set, no allowlist restrictions will be applied (all commands will be allowed).

### Allowlist File Format

The allowlist file should be a YAML file with the following structure:

```yaml
extensions:
  - id: extension-id-1
    command: command-name-1
  - id: extension-id-2
    command: command-name-2
  # ... more extensions
```

Example:

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

When a Stdio extension attempts to register with a command, the system:

1. Extracts the base command name (the last part of the path)
   - For example, `/Users/username/bin/mcp thing-here` becomes `mcp thing-here`
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

## Troubleshooting

If extensions are being rejected unexpectedly:

1. Check if the `GOOSE_ALLOWLIST` environment variable is set correctly.
2. Verify that the allowlist file is accessible from the server.
3. Ensure the allowlist file is properly formatted YAML.
4. Check server logs for any errors related to fetching or parsing the allowlist.
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
./goosed
```

3. When extensions are registered, only those with commands matching the allowlist will be accepted.