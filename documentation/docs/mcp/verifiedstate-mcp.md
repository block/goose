---
title: VerifiedState Extension
description: Add VerifiedState MCP Server as a goose Extension for verified agent memory with cryptographic receipts
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [VerifiedState MCP Server](https://verifiedstate.ai) as a goose extension. VerifiedState provides decision trace infrastructure — every assertion your goose agent makes gets a cryptographic verification receipt.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40verifiedstate%2Fmcp-server&id=verifiedstate&name=VerifiedState&description=Verified%20agent%20memory%20with%20cryptographic%20receipts&env=VERIFIEDSTATE_API_KEY%3DYour%20API%20Key&env=VERIFIEDSTATE_NAMESPACE_ID%3DYour%20Namespace%20ID)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @verifiedstate/mcp-server
  ```
  </TabItem>
</Tabs>
  **Environment Variables**
  ```
  VERIFIEDSTATE_API_KEY: <YOUR_API_KEY>
  VERIFIEDSTATE_NAMESPACE_ID: <YOUR_NAMESPACE_ID>
  ```
:::

## What VerifiedState adds to goose

goose is an autonomous agent making consequential decisions — architectural choices, tool selections, code changes. VerifiedState adds a signed audit trail to every decision:

- **Verified memory** — every fact goose stores gets a cryptographic receipt
- **Point-in-time queries** — reconstruct what goose believed at any moment
- **Conflict detection** — detect when new assertions contradict verified facts
- **Audit export** — full decision trace bundle for compliance review

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="verifiedstate"
    extensionName="VerifiedState"
    description="Verified agent memory with cryptographic receipts"
    command="npx"
    args={["-y", "@verifiedstate/mcp-server"]}
    envVars={[
      { name: "VERIFIEDSTATE_API_KEY", label: "Your VerifiedState API Key" },
      { name: "VERIFIEDSTATE_NAMESPACE_ID", label: "Your Namespace ID" }
    ]}
    apiKeyLink="https://verifiedstate.ai/keys"
    apiKeyLinkText="VerifiedState API Key"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="verifiedstate"
      description="Verified agent memory with cryptographic receipts"
      type="stdio"
      command="npx -y @verifiedstate/mcp-server"
      timeout={300}
      envVars={[
        { key: "VERIFIEDSTATE_API_KEY", value: "<Your VerifiedState API Key>" },
        { key: "VERIFIEDSTATE_NAMESPACE_ID", value: "<Your Namespace ID>" }
      ]}
      infoNote={
        <>
          Get your free API key at <a href="https://verifiedstate.ai/keys" target="_blank" rel="noopener noreferrer">verifiedstate.ai/keys</a>. Free tier includes 25,000 assertions/month.
        </>
      }
    />
    </TabItem>
</Tabs>

## Available Tools

| Tool | Description |
|------|-------------|
| `memory_ingest` | Store content and create an artifact with normalized spans |
| `memory_query` | Six-channel retrieval: semantic, lexical, temporal, graph, conflict, exact |
| `memory_verify` | Run the verification ladder and produce a signed receipt |
| `memory_health` | Get memory health metrics for the namespace |

## Getting an API Key

Get a free API key at [verifiedstate.ai/keys](https://verifiedstate.ai/keys). Free tier includes 25,000 assertions per month with no credit card required.

## Example Usage

Once connected, goose can use VerifiedState tools naturally:

```
Store this decision: "Chose PostgreSQL over MySQL for the user service because of jsonb support and pgvector for embeddings"
```

goose will call `memory_ingest`, extract assertions, and verify them with signed receipts.

```
What did I decide about the database for the user service?
```

goose will call `memory_query` to retrieve the verified assertion with its receipt.

## Links

- [VerifiedState Documentation](https://verifiedstate.ai/docs)
- [API Reference](https://verifiedstate.ai/docs)
- [Whitepaper](https://verifiedstate.ai/whitepaper)
- [GitHub](https://github.com/verifiedstate/verified-memory)
