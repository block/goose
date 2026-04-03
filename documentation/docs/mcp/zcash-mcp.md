---
title: Zcash
description: Shielded transactions, memo decoding, and on-chain attestation for AI agents
---

# Zcash MCP Server

Zcash tools for AI agents. Shielded operations, structured memo decoding, lifecycle attestation via ZAP1, and chain queries. Privacy by default.

## Quick Install

```
npx -y @frontiercompute/zcash-mcp
```

Or configure in `~/.config/goose/config.yaml`:

```yaml
extensions:
  zcash-mcp:
    name: Zcash
    type: stdio
    cmd: npx
    args: ["-y", "@frontiercompute/zcash-mcp"]
    enabled: true
    envs:
      ZAP1_API_URL: https://pay.frontiercompute.io
```

## Tools

| Tool | Description |
|------|-------------|
| get_block_height | Current Zcash chain height |
| lookup_transaction | Transaction details by txid |
| get_balance | Wallet lifecycle and attestation status |
| send_shielded | Generate shielded payment URI (ZIP 321) |
| decode_memo | Decode ZAP1, ZIP 302, or plain text memos |
| attest_event | Create a ZAP1 lifecycle attestation on Zcash |
| verify_proof | Verify a ZAP1 Merkle proof |
| get_stats | Protocol stats (leaves, anchors, event types) |
| get_anchors | Anchor history with txids and block heights |
| get_events | Recent attestation events |
| get_agent_status | Agent attestation summary |
| get_agent_bond | Agent bond and policy compliance status |

## Example Usage

Decode a Zcash shielded memo:
```
decode this memo: ZAP1:09:024e36515ea30efc15a0a7962dd8f677455938079430b9eab174f46a4328a07a
```

Verify an attestation proof:
```
verify this proof: ddbe05cc63697e1ce83c210ab4500cf1d2d4921eb863dbb40f10ec3e42c7b28c
```

Check protocol stats:
```
what are the current ZAP1 stats?
```

## Configuration

| Variable | Description | Required |
|----------|-------------|----------|
| ZEBRA_RPC_URL | Zcash Zebra node endpoint | No (defaults to public endpoint) |
| ZAP1_API_URL | ZAP1 attestation API | No (defaults to pay.frontiercompute.io) |
| ZAP1_API_KEY | API key for write operations (attest_event) | No (read tools work without it) |

## Links

- [Source](https://github.com/Frontier-Compute/zcash-mcp)
- [npm](https://www.npmjs.com/package/@frontiercompute/zcash-mcp)
- [ZAP1 Protocol](https://frontiercompute.io/protocol.html)
- [ZIP 1243 Draft](https://github.com/zcash/zips/pull/1243)
