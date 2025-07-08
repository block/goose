---
title: Money Extension 
description: Add MCP Money Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

This tutorial covers how to add the [MCP Money Server](https://github.com/pablof7z/mcp-money) as a Goose extension to provide access to Bitcoin Lightning Network and Cashu ecash functionality directly within your AI assistant workflow.

:::tip TLDR

**Command**
```sh
npx mcp-money
```

:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  1. [Launch the installer](goose://extension?cmd=npx&arg=mcp-money&id=money&name=Money&description=Bitcoin%20Lightning%20and%20Cashu%20ecash%20wallet%20with%20Nostr%20zaps%20support)
  2. Press `Yes` to confirm the installation
  3. Click `Save Configuration`
  4. Scroll to the top and click `Exit` from the upper left corner
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◆  What type of extension would you like to add?
    │  ○ Built-in Extension 
    // highlight-start    
    │  ● Command-line Extension (Run a local command or script)
    // highlight-end    
    │  ○ Remote Extension 
    └ 
  ```

  3. Give your extension a name
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    // highlight-start
    ◆  What would you like to call this extension?
    │  Money
    // highlight-end
    └ 
  ```

  4. Enter the command
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Money
    │
    // highlight-start
    ◆  What command should be run?
    │  npx mcp-money
    // highlight-end
    └ 
  ```  

  5. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
    ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Money
    │
    ◇  What command should be run?
    │  npx mcp-money
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    │
    └ 
  ``` 

  6. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
    ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Money
    │
    ◇  What command should be run?
    │  npx mcp-money
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◇  Would you like to add a description?
    │  Yes
    │
    ◆  Please enter a description:
    │  Bitcoin Lightning and Cashu ecash wallet with Nostr zaps support
    // highlight-end
    │
    └ 
  ```
  
  7. Choose No when asked to add environment variables (optional - you can add your Nostr private key later)

   ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
    │
    ◇  What type of extension would you like to add?
    │  Command-line Extension 
    │
    ◇  What would you like to call this extension?
    │  Money
    │
    ◇  What command should be run?
    │  npx mcp-money
    │     
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    ◇  Would you like to add a description?
    │  Yes
    │
    ◇  Please enter a description:
    │  Bitcoin Lightning and Cashu ecash wallet with Nostr zaps support
    │    
    // highlight-start
    ◆  Would you like to add environment variables?
    │  No
    │   
    // highlight-end
    └  Added Money extension
  ```  

  </TabItem>
</Tabs>

## Features

The Money extension provides the following capabilities:

- **Cashu Wallet Integration**: Full support for Cashu ecash mints and tokens
- **NIP-60 Wallet Interface**: Uses the standardized Nostr wallet interface to store and manage proofs
- **Lightning Network**: Create deposit invoices and pay Lightning invoices
- **Nostr Zaps**: Send zaps to users using npub or NIP-05 identifiers
- **Multi-mint Support**: Manage multiple Cashu mints simultaneously
- **Balance Management**: Check total balance and per-mint breakdowns

## Authentication

The wallet supports multiple ways to provide your Nostr private key for zapping functionality:

1. **Environment variable**: Set `NSEC=nsec1...` in your environment
2. **Command line parameter**: Use `--nsec nsec1...` when running manually
3. **Auto-generate**: The extension will create a new key if none is provided
4. **Config file**: Keys are automatically saved to `.wallet.json` for persistence

:::warning Security Note
Your private keys are stored locally in `.wallet.json`. Keep this file secure and never share it.
:::

## Example Usage

### Goose Prompt

> _Goose, what's my current Bitcoin balance? If I have any funds, send a 100 sat zap to jack@block.xyz with the message "Great post!"_

:::info Demo
You can see a real example of mcp-money in action [in this tweet](https://njump.me/nevent1qvzqqqqqqypzp75cf0tahv5z7plpdeaws7ex52nmnwgtwfr2g3m37r844evqrr6jqyxhwumn8ghj7e3h0ghxjme0qyd8wumn8ghj7urewfsk66ty9enxjct5dfskvtnrdakj7qghwaehxw309aex2mrp0yh8qunfd4skctnwv46z7qpqh4le94u3rjvg6lgyajekrvf8xaq2ef2e3q9nf0znk562gm5a0hkq790893) demonstrating the extension being used with Goose.
:::

### Goose Output

:::note CLI

<details>
    <summary>Tool Calls</summary>

    ─── get_balance | money ──────────────────────────

</details>

I'll check your current Bitcoin balance first.

Your current total balance is **2,547 satoshis** across your configured mints.

Let me break this down by mint:
- mint.coinos.io: 1,200 sats
- mint.lnvoltz.com: 847 sats  
- mint.chorus.community: 500 sats

Since you have sufficient funds, I'll now send a 100 satoshi zap to jack@block.xyz with your message.

<details>
    <summary>Tool Calls</summary>

    ─── zap | money ──────────────────────────

    recipient: jack@block.xyz
    amount: 100
    comment: Great post!

</details>

✅ Successfully sent 100 sat zap to jack@block.xyz with message "Great post!"

Your new balance is **2,447 satoshis**.

:::

### Advanced Usage Examples

#### Deposit Funds

> _Create a Lightning invoice for 10,000 sats to deposit funds to my wallet._

#### Pay Lightning Invoice

> _Pay this Lightning invoice: lnbc10000n1..._

#### Add New Mint

> _Add the mint at https://mint.example.com to my wallet._

#### Check Mint Balances

> _Show me my balance breakdown across all mints._

## Default Configuration

The Money extension comes pre-configured with:

### Default Relays
- `wss://relay.damus.io`
- `wss://nos.lol`
- `wss://relay.nostr.band`
- `wss://nostr.mutinywallet.com`

### Default Mints
- `https://mint.coinos.io`
- `https://mint.lnvoltz.com`
- `https://mint.chorus.community`

## Available Tools

When the Money extension is active, the following tools become available to Goose:

- `get_balance`: Get the total wallet balance
- `get_mint_balances`: Get balance breakdown per mint
- `deposit`: Create a deposit invoice for specified amount and mint
- `pay`: Pay a Lightning invoice
- `zap`: Send a zap to a user via npub or NIP-05 identifier
- `add_mint`: Add a new mint to the wallet

## Security Considerations

- Private keys are stored locally in `.wallet.json`
- Mint information is cached for performance
- All Nostr communications use standard NIP protocols
- Lightning payments require explicit confirmation
- Only interact with trusted Cashu mints
- Regular backup of your wallet configuration is recommended

## Troubleshooting

**Issue**: Extension fails to start
- Ensure Node.js is installed and accessible
- Check internet connectivity for accessing mints and relays

**Issue**: Zaps fail to send
- Verify your Nostr private key is properly configured
- Check that the recipient's npub is valid
- Ensure sufficient satoshi balance

**Issue**: Lightning payments fail
- Verify the Lightning invoice is valid and not expired
- Check that your connected mints support Lightning
- Ensure sufficient balance across your mints