---
title: Spraay x402 Extension
description: Add Spraay x402 MCP Server as a goose Extension for batch crypto payments, onchain data, and AI model access
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

This tutorial covers how to add the [Spraay x402 MCP Server](https://github.com/plagtech/spraay-x402-mcp) as a goose extension for batch cryptocurrency payments, onchain data, and AI model access on Base.

With the Spraay extension, goose can send ETH and ERC-20 tokens to up to **200 recipients in a single transaction**, query live token prices, check wallet balances, resolve ENS names, get swap quotes, and chat with 200+ AI models — all pay-per-call via the [x402 protocol](https://x402.org).

## Supported Tools

The Spraay x402 MCP Server provides the following tools:

| Tool | Cost | Description |
|------|------|-------------|
| `spraay_chat` | $0.005 | AI chat via 200+ models (GPT-4, Claude, Llama, Gemini) |
| `spraay_models` | $0.001 | List available AI models with pricing |
| `spraay_batch_execute` | $0.01 | Batch USDC payments to multiple recipients |
| `spraay_batch_estimate` | $0.001 | Estimate gas for batch payments |
| `spraay_swap_quote` | $0.002 | Uniswap V3 swap quotes on Base |
| `spraay_tokens` | $0.001 | List supported tokens on Base |
| `spraay_prices` | $0.002 | Live onchain token prices (Uniswap V3) |
| `spraay_balances` | $0.002 | ETH + ERC-20 balances for any address |
| `spraay_resolve` | $0.001 | Resolve ENS names and Basenames to addresses |

:::info
AI agents pay USDC per request via the x402 protocol. No API keys or accounts needed — just a wallet with USDC on Base.
:::

## Setup

### Prerequisites

- Git and Node.js installed
- A wallet private key with USDC on Base (for x402 micropayments)

### Install the MCP Server

```bash
git clone https://github.com/plagtech/spraay-x402-mcp.git
cd spraay-x402-mcp
npm install
npm run build
```

### Configuration

<Tabs>
<TabItem value="ui" label="goose Desktop">

1. Click the **Extensions** icon in the sidebar
2. Click **Add custom extension**
3. Select **Command-line Extension**
4. Fill in the following:
   - **Name**: `spraay`
   - **Command**: `node /absolute/path/to/spraay-x402-mcp/dist/index.js`
   - **Timeout**: `300`
5. Add environment variable:
   - **Name**: `EVM_PRIVATE_KEY`
   - **Value**: Your wallet private key (with USDC on Base)

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
│   node /absolute/path/to/spraay-x402-mcp/dist/index.js
│
│   ◇  Please set the timeout for this tool (in secs):
│   300
│
│   ◇  Would you like to add environment variables?
│   Yes
│
│   ◇  Environment variable name:
│   EVM_PRIVATE_KEY
│
│   ◇  Environment variable value:
│   ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
│
│   └  Added spraay extension
```

</TabItem>
</Tabs>

## Example Usage

### Check Token Prices

```
What's the current price of ETH on Base?
```

goose will use the `spraay_prices` tool to fetch live onchain prices from Uniswap V3.

### Check Wallet Balances

```
Check the USDC balance of vitalik.eth
```

goose will use `spraay_resolve` to resolve the ENS name, then `spraay_balances` to fetch balances.

### Get a Swap Quote

```
Get me a swap quote for 100 USDC to WETH on Base
```

goose will use `spraay_swap_quote` to fetch a live quote from Uniswap V3.

### Batch Send USDC

```
Send 10 USDC each to these wallets:
0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18
0x53d284357ec70cE289D6D64134DfAc8E511c8a3D
0xFBb1b73C4f0BDa4f67dcA266ce6Ef42f520fBB98
```

goose will use `spraay_batch_execute` to process all payments in a single transaction.

### Chat with AI Models

```
Using spraay, ask GPT-4 to explain what x402 is
```

goose will use `spraay_chat` to query from 200+ available AI models.

## How It Works

1. You ask goose → goose calls a Spraay tool (e.g. `spraay_prices`)
2. MCP server sends request to `gateway.spraay.app`
3. Gateway returns HTTP 402 + payment requirements
4. x402 client auto-signs USDC payment on Base
5. Gateway verifies payment and returns data

Each call costs $0.001–$0.01 in USDC. No API keys, no accounts.

## Resources

- [Spraay App](https://spraay.app) — Multi-chain batch payments (10 chains)
- [Spraay x402 Gateway](https://gateway.spraay.app) — AI agent payment gateway
- [GitHub](https://github.com/plagtech/spraay-x402-mcp) — MCP server source code
- [x402 Protocol](https://x402.org) — The payment standard
- [Smart Contract on BaseScan](https://basescan.org/address/0x1646452F98E36A3c9Cfc3eDD8868221E207B5eEC)
