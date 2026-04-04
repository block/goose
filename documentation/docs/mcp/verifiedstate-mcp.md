---
title: VerifiedState Extension
description: Add VerifiedState MCP Server as a goose Extension for verified agent memory with cryptographic receipts
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [VerifiedState MCP Server](https://verifiedstate.ai) as a goose extension. VerifiedState provides decision trace infrastructure — every assertion your goose agent makes gets a cryptographic verification receipt.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="cli" label="goose CLI" default>
  Use `goose configure` to add a `Command-line Extension (stdio)` extension type with:

  **Command**
  ```
  npx @verifiedstate/mcp-server
  ```

  **Environment Variables**
  ```
  VERIFIEDSTATE_API_KEY=vs_live_your_key
  VERIFIEDSTATE_NAMESPACE_ID=your_namespace_id
  ```
  </TabItem>
</Tabs>
:::

## What VerifiedState adds to goose

goose is an autonomous agent making consequential decisions — architectural choices, tool selections, code changes. VerifiedState adds a signed audit trail to every decision:

- **Verified memory** — every fact goose stores gets a cryptographic receipt
- **Point-in-time queries** — reconstruct what goose believed at any moment
- **Conflict detection** — detect when new assertions contradict verified facts
- **Audit export** — full decision trace bundle for compliance review

## Configuration

<Tabs groupId="interface">
  <TabItem value="cli" label="goose CLI" default>

    <CLIExtensionInstructions
      name="VerifiedState"
      description="Decision trace infrastructure for AI agents. Cryptographic receipts on every assertion."
      type="stdio"
      command="npx @verifiedstate/mcp-server"
      envVars={[
        { key: "VERIFIEDSTATE_API_KEY", value: "your-api-key" },
        { key: "VERIFIEDSTATE_NAMESPACE_ID", value: "your-namespace-id" }
      ]}
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
