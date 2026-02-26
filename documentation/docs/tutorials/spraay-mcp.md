---
title: Spraay Batch Payments Extension
description: Add Spraay MCP Server as a goose Extension for batch cryptocurrency payments
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

This tutorial covers how to add the [Spraay MCP Server](https://github.com/AIdleMinerTycoon/spraay-mcp-server) as a goose extension for batch cryptocurrency payments on Base.

With the Spraay extension, goose can send ETH and ERC-20 tokens to up to **200 recipients in a single transaction**, saving approximately 80% on gas fees compared to individual transfers.

## Supported Tools

The Spraay MCP Server provides the following tools:

| Tool | Description |
|------|-------------|
| `batch_send_eth` | Send equal amounts of ETH to multiple recipients |
| `batch_send_token` | Send equal amounts of any ERC-20 token to multiple recipients |
| `batch_send_eth_variable` | Send different ETH amounts to each recipient |
| `batch_send_token_variable` | Send different token amounts to each recipient |

:::info
The Spraay protocol charges a 0.3% fee per transaction. Maximum 200 recipients per batch.
:::

## Setup

### Prerequisites

- A wallet private key with ETH on Base for gas
- Node.js installed (for npx/Smithery)

### Configuration

<Tabs>
<TabItem value="ui" label="goose Desktop">

1. Click the **Extensions** icon in the sidebar
2. Click **Add custom extension**
3. Select **Command-line Extension**
4. Fill in the following:
   - **Name**: `spraay`
   - **Command**: `npx -y @smithery/cli@latest run @AIdleMinerTycoon/spraay-mcp-server --key YOUR_SMITHERY_KEY`
   - **Timeout**: `300`
5. Add environment variable:
   - **Name**: `SPRAAY_PRIVATE_KEY`
   - **Value**: Your wallet private key

</TabItem>
<TabItem value="cli" label="goose CLI">

```
goose configure
```

Choose **Add Extension** → **Command-line Extension** and configure:

```
┌   goose-configure
│
│   ◇  What would you like to call this extension?
│   spraay
│
│   ◇  What command should be run?
│   npx -y @smithery/cli@latest run @AIdleMinerTycoon/spraay-mcp-server --key YOUR_SMITHERY_KEY
│
│   ◇  Please set the timeout for this tool (in secs):
│   300
│
│   ◇  Would you like to add environment variables?
│   Yes
│
│   ◇  Environment variable name:
│   SPRAAY_PRIVATE_KEY
│
│   ◇  Environment variable value:
│   ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
│
│   └  Added spraay extension
```

</TabItem>
</Tabs>

## Example Usage

### Pay Team Members Equal ETH

```
Send 0.01 ETH to each of these wallets:
0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18
0x53d284357ec70cE289D6D64134DfAc8E511c8a3D
0xFBb1b73C4f0BDa4f67dcA266ce6Ef42f520fBB98
```

goose will use the `batch_send_eth` tool to process all three payments in a single transaction.

### Distribute USDC to Contributors

```
Distribute 50 USDC to each of these addresses using token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913:
0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18
0x53d284357ec70cE289D6D64134DfAc8E511c8a3D
```

goose will automatically handle the ERC-20 token approval and batch send.

### Variable Amounts

```
Send different ETH amounts: 0.05 to 0x742d...bD18 and 0.1 to 0x53d2...8a3D
```

goose selects `batch_send_eth_variable` when amounts differ per recipient.

## Common Token Addresses (Base)

| Token | Address |
|-------|---------|
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` |
| USDT | `0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2` |
| DAI | `0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb` |
| WETH | `0x4200000000000000000000000000000000000006` |

## Resources

- [Spraay Dapp](https://spraay-base-dapp.vercel.app)
- [Spraay MCP Server on Smithery](https://smithery.ai/server/@AIdleMinerTycoon/spraay-mcp-server)
- [Smart Contract on BaseScan](https://basescan.org/address/0x1646452F98E36A3c9Cfc3eDD8868221E207B5eEC)
- [GitHub](https://github.com/AIdleMinerTycoon/spraay-mcp-server)
