---
title: Bitcoin Knowledge Base Extension
description: Add bkb-mcp as a goose Extension for Bitcoin and Lightning development research
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Bitcoin Knowledge Base (BKB) MCP Server](https://github.com/tnull/bitcoin-knowledge-base) as a goose extension. BKB is a search and reference tool that indexes the Bitcoin and Lightning development ecosystem — including BIPs, BOLTs, bLIPs, LUDs, NUTs, GitHub issues/PRs/commits, mailing lists, IRC logs, Delving Bitcoin, BitcoinTalk, and Optech newsletters.

:::tip Quick Install
1. Install: `cargo install bkb-mcp`
2. Configure: [Add BKB Extension](goose://extension?cmd=bkb-mcp&id=bkb-mcp&name=Bitcoin%20Knowledge%20Base&description=Search%20and%20reference%20tool%20for%20Bitcoin%20and%20Lightning%20development)
:::

## Installation

This extension requires the Rust toolchain. Check your version:

```bash
rustc --version
```

If you need to install or update Rust, visit [rustup.rs](https://rustup.rs/).

Install the BKB MCP server from crates.io:

```bash
cargo install bkb-mcp
```

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="bkb-mcp"
      extensionName="Bitcoin Knowledge Base"
      description="Search and reference tool for Bitcoin and Lightning development"
      type="stdio"
      command="bkb-mcp"
      args={[]}
      envVars={[
        { name: "BKB_API_URL", label: "BKB API URL" }
      ]}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Bitcoin Knowledge Base"
      description="Search and reference tool for Bitcoin and Lightning development"
      type="stdio"
      command="bkb-mcp"
      envVars={[
        { key: "BKB_API_URL", value: "https://bitcoinknowledge.dev" }
      ]}
    />
  </TabItem>
</Tabs>

:::info
Set the `BKB_API_URL` environment variable to `https://bitcoinknowledge.dev` to use the public hosted API. If you self-host the BKB server, point this to your local instance instead (e.g., `http://127.0.0.1:3000`).
:::

## Example Usage

### goose Prompt

> _What is Taproot and how was it discussed in the community?_

### goose Output

:::note CLI

<details>
    <summary>Tool Calls</summary>

    ─── bkb_search | bkb-mcp ──────────────────────────

    query: Taproot Bitcoin soft fork

    ─── bkb_lookup_bip | bkb-mcp ──────────────────────────

    number: 341

    ─── bkb_get_references | bkb-mcp ──────────────────────────

    document_id: bip-341

</details>

**Taproot** (activated November 2021) is a Bitcoin soft fork defined in BIPs 340–342 that introduced Schnorr signatures and Merkelized Alternative Script Trees (MAST).

**Key BIPs:**
- **BIP 340** — Schnorr Signatures for secp256k1
- **BIP 341** — Taproot: SegWit version 1 spending rules
- **BIP 342** — Validation of Taproot Scripts

**Community Discussion:**
The proposal was extensively discussed on the bitcoin-dev mailing list starting in 2018, with activation debates on Delving Bitcoin and IRC throughout 2020–2021. The Optech newsletter covered Taproot progress in issues #46, #48, #128, and the Taproot activation series.
:::

## Available Tools

| Tool | Description |
|------|-------------|
| `bkb_search` | Full-text search across all indexed Bitcoin and Lightning resources |
| `bkb_get_document` | Retrieve a specific document by ID |
| `bkb_get_references` | Get references and citations for a document |
| `bkb_lookup_bip` | Look up a Bitcoin Improvement Proposal by number |
| `bkb_lookup_bolt` | Look up a BOLT (Lightning Network specification) by number |
| `bkb_lookup_blip` | Look up a bLIP (Lightning Improvement Proposal) by number |
| `bkb_lookup_lud` | Look up a LUD (LNURL specification) by number |
| `bkb_lookup_nut` | Look up a NUT (Cashu protocol specification) by number |
| `bkb_timeline` | Get a timeline of events for a topic |
| `bkb_find_commit` | Find relevant commits across Bitcoin-related repositories |
