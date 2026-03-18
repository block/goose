---
title: "Spraay x402 扩展"
description: "介绍如何把 Spraay x402 MCP Server 作为 goose 扩展，用于批量加密支付、链上数据和模型访问。"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Spraay x402 扩展

本教程介绍如何把 [Spraay x402 MCP Server](https://github.com/plagtech/spraay-x402-mcp) 作为 goose 扩展接入，用于 Base 链上的批量加密货币支付、链上数据查询，以及 AI 模型访问。

通过 Spraay 扩展，goose 可以在**一笔交易里最多向 200 个收款地址发送 ETH 或 ERC-20 token**，也可以查询实时 token 价格、检查钱包余额、解析 ENS 名称、获取 swap quote，并通过 [x402 协议](https://x402.org) 以按调用付费方式访问 200 多种 AI 模型。

## 支持的工具

Spraay x402 MCP Server 提供以下工具：

| Tool | 成本 | 说明 |
|---|---|---|
| `spraay_chat` | $0.005 | 通过 200+ 模型进行 AI 聊天（GPT-4、Claude、Llama、Gemini 等） |
| `spraay_models` | $0.001 | 列出可用 AI 模型及其价格 |
| `spraay_batch_execute` | $0.01 | 向多个收款方批量执行 USDC 支付 |
| `spraay_batch_estimate` | $0.001 | 估算批量支付的 gas |
| `spraay_swap_quote` | $0.002 | 获取 Base 上的 Uniswap V3 swap quote |
| `spraay_tokens` | $0.001 | 列出 Base 上支持的 token |
| `spraay_prices` | $0.002 | 获取链上 token 实时价格（Uniswap V3） |
| `spraay_balances` | $0.002 | 查询任意地址的 ETH 和 ERC-20 余额 |
| `spraay_resolve` | $0.001 | 把 ENS 或 Basenames 解析为地址 |

:::info
AI agent 会通过 x402 协议按请求支付 USDC。你不需要单独申请 API key，也不需要注册账号，只要在 Base 上有一个持有 USDC 的钱包即可。
:::

## 安装与配置

### 前置条件

- 已安装 Git 和 Node.js
- 准备一个在 Base 链上持有 USDC 的钱包私钥（用于 x402 micropayment）

### 安装 MCP Server

```bash
git clone https://github.com/plagtech/spraay-x402-mcp.git
cd spraay-x402-mcp
npm install
npm run build
```

### 在 goose 中配置

<Tabs>
<TabItem value="ui" label="goose Desktop">

1. 点击侧边栏中的 **Extensions** 图标
2. 点击 **Add custom extension**
3. 选择 **Command-line Extension**
4. 填写以下内容：
   - **Name**：`spraay`
   - **Command**：`node /absolute/path/to/spraay-x402-mcp/dist/index.js`
   - **Timeout**：`300`
5. 添加环境变量：
   - **Name**：`EVM_PRIVATE_KEY`
   - **Value**：你的钱包私钥（该钱包应在 Base 上持有 USDC）

  </TabItem>
  <TabItem value="cli" label="goose CLI">

```
goose configure
```

然后依次选择 **Add Extension** → **Command-line Extension**，并按下面的方式填写：

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

## 使用示例

### 查询 Token 价格

```text
What's the current price of ETH on Base?
```

goose 会调用 `spraay_prices`，从 Uniswap V3 获取链上实时价格。

### 查询钱包余额

```text
Check the USDC balance of vitalik.eth
```

goose 会先用 `spraay_resolve` 解析 ENS 名称，再通过 `spraay_balances` 查询余额。

### 获取 Swap Quote

```text
Get me a swap quote for 100 USDC to WETH on Base
```

goose 会调用 `spraay_swap_quote` 获取 Uniswap V3 的实时报价。

### 批量发送 USDC

```text
Send 10 USDC each to these wallets:
0x742d35Cc6634C0532925a3b844Bc9e7595f2bD18
0x53d284357ec70cE289D6D64134DfAc8E511c8a3D
0xFBb1b73C4f0BDa4f67dcA266ce6Ef42f520fBB98
```

goose 会调用 `spraay_batch_execute`，把所有付款合并进一笔交易里执行。

### 调用 AI 模型

```text
Using spraay, ask GPT-4 to explain what x402 is
```

goose 会使用 `spraay_chat`，从 200 多个可用模型中发起调用。

## 背后发生了什么

1. 你向 goose 发出请求
2. goose 调用对应的 Spraay 工具，例如 `spraay_prices`
3. MCP server 把请求发送到 `gateway.spraay.app`
4. Gateway 返回 HTTP 402 与支付要求
5. x402 client 自动在 Base 上签署对应 USDC 支付
6. Gateway 验证付款后，返回真正的数据结果

每次调用的成本大约在 `$0.001–$0.01` USDC 之间。不需要 API key，也不需要单独注册账号。

## 资源

- [Spraay App](https://spraay.app)：支持 10 条链的批量支付应用
- [Spraay x402 Gateway](https://gateway.spraay.app)：AI agent 支付网关
- [GitHub](https://github.com/plagtech/spraay-x402-mcp)：MCP server 源码
- [x402 Protocol](https://x402.org)：支付协议标准
- [BaseScan 上的智能合约](https://basescan.org/address/0x1646452F98E36A3c9Cfc3eDD8868221E207B5eEC)
