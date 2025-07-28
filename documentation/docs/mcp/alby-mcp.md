---
title: Alby Bitcoin Payments MCP

description: Connect Goose to your Bitcoin Lightning Wallet
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import { PanelLeft } from 'lucide-react';




This tutorial covers how to add the [Alby Bitcoin Payments MCP Server](https://github.com/getalby/mcp) as a Goose extension to interact with your lightning wallet, make and receive payments, list transactions, convert fiat amounts to sats, request invoices from lightning addresses, and interact with paid MCP tools (such as ones built with [PaidMCP](https://github.com/getAlby/paidmcp)).

:::info
You'll need a lightning wallet that supports [NWC](https://nwc.dev). If you don't have one yet, consider trying [Alby Hub](https://albyhub.com).
:::

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40getalby%2Fmcp&id=alby&name=Alby&description=Connect%20Goose%20to%20your%20Bitcoin%20Lightning%20Wallet&env=NWC_CONNECTION_STRING%3DNWC%20Connection%20Secret)
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  npx -y @getalby/mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  NWC_CONNECTION_STRING: nostr+walletconnect://...
  ```
:::

## Configuration

:::info
You'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`

**or** you can use the Alby-hosted MCP (see remote options below).
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    <Tabs>
      <TabItem value="local" label="Local" default>
        <GooseDesktopInstaller
          extensionId="alby"
          extensionName="Alby"
          description="Connect Goose to your Bitcoin Lightning Wallet"
          command="npx"
          args={["-y", "@getalby/mcp"]}
          envVars={[
            { name: "NWC_CONNECTION_STRING", label: "NWC Connection Secret" }
          ]}
          customStep3="Obtain a NWC Connection secret from your lightning wallet (nostr+walletconnect://...) and paste it in to the 'NWC Connection Secret' field"
        />
      </TabItem>
      <TabItem value="remote" label="Remote">
        1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40getalby%2Fmcp&id=alby&name=Alby&description=Connect%20Goose%20to%20your%20Bitcoin%20Lightning%20Wallet)
        2. Press `OK` to confirm the installation
        3. Change the type to "Streamable HTTP"
        4. Change the endpoint to `https://mcp.getalby.com/mcp`
        5. Add a request header with Header name = `Authorization` and Value:
```
Bearer nostr+walletconnect://...
```

        6. Press the `+Add` button to finish adding the request header
        7. Press `Add Extension`
        8. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
        9. Navigate to the chat
      </TabItem>
    </Tabs>
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    <Tabs>
      <TabItem value="local" label="Local" default>
        1. Run the `configure` command:
        ```sh
        goose configure
        ```

        2. Choose to add a `Command-line Extension`
        ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
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
          │  Add Extension (Connect to a new extension) 
          │
          ◇  What type of extension would you like to add?
          │  Command-line Extension 
          │
          // highlight-start
          ◆  What would you like to call this extension?
          │  Alby
          // highlight-end
          └ 
        ```

        4. Enter the command
        ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
          │
          ◇  What type of extension would you like to add?
          │  Command-line Extension 
          │
          ◇  What would you like to call this extension?
          │  Alby
          │
          // highlight-start
          ◆  What command should be run?
          │  npx -y @getalby/mcp
          // highlight-end
          └ 
        ```  

        5. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
          ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
          │
          ◇  What type of extension would you like to add?
          │  Command-line Extension 
          │
          ◇  What would you like to call this extension?
          │  Alby
          │
          ◇  What command should be run?
          │  npx -y @getalby/mcp
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
          │  Add Extension (Connect to a new extension) 
          │
          ◇  What type of extension would you like to add?
          │  Command-line Extension 
          │
          ◇  What would you like to call this extension?
          │  Alby
          │
          ◇  What command should be run?
          │  npx -y @getalby/mcp
          │
          ◇  Please set the timeout for this tool (in secs):
          │  300
          │
          // highlight-start
          ◇  Would you like to add a description?
          │  No
          // highlight-end
          │
          └ 
        ```
        
        7. Obtain a NWC connection secret from your lightning wallet and paste it in.

        ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
          │
          ◇  What type of extension would you like to add?
          │  Command-line Extension 
          │
          ◇  What would you like to call this extension?
          │  Alby
          │
          ◇  What command should be run?
          │  npx -y @getalby/mcp
          │     
          ◇  Please set the timeout for this tool (in secs):
          │  300
          │
          ◇  Would you like to add a description?
          │  No
          │    
          // highlight-start
          ◆  Would you like to add environment variables?
          │  Yes 
          │
          ◇  Environment variable name:
          │  NWC_CONNECTION_STRING
          │
          ◇  Environment variable value:
          │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
          │
          ◇  Add another environment variable?
          │  No 
          // highlight-end
          └  Added Alby extension
        ```
      </TabItem>
      <TabItem value="remote" label="Remote">
        8. Run the `configure` command:
        ```sh
        goose configure
        ```

        9. Choose to add a `Remote Extension`
        ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
          │
          ◆  What type of extension would you like to add?
          │  ○ Built-in Extension 
          │  ○ Command-line Extension (Run a local command or script)
          │  ○ Remote Extension (SSE)
          // highlight-start    
          │  ● Remote Extension (Streaming HTTP) 
          // highlight-end    
          └ 
        ```

        10. Give your extension a name
        ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
          │
          ◇  What type of extension would you like to add?
          │  Remote Extension 
          │
          // highlight-start
          ◆  What would you like to call this extension?
          │  Alby
          // highlight-end
          └ 
        ```

        11. Enter the Streaming HTTP endpoint URI

        ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
          │
          ◇  What type of extension would you like to add?
          │  Remote Extension 
          │
          ◇  What would you like to call this extension?
          │  Alby
          │
          // highlight-start
          ◆  What is the Streaming HTTP URI?
          │  https://mcp.getalby.com/mcp
          // highlight-end
          └ 
        ```

        12. Enter the number of seconds Goose should wait for actions to complete before timing out. Default is 300s
          ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
          │
          ◇  What type of extension would you like to add?
          │  Remote Extension
          │
          ◇  What would you like to call this extension?
          │  Alby
          │
          ◇  What is the Streaming HTTP endpoint URI?
          │  https://mcp.getalby.com/mcp
          │
          // highlight-start
          ◆  Please set the timeout for this tool (in secs):
          │  300
          // highlight-end
          │
          └ 
        ``` 

        13. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
          ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
          │
          ◇  What type of extension would you like to add?
          │  Remote Extension 
          │
          ◇  What would you like to call this extension?
          │  Alby
          │
          ◇  What is the Streaming HTTP endpoint URI?
          │  https://mcp.getalby.com/mcp
          │
          ◇  Please set the timeout for this tool (in secs):
          │  300
          │
          // highlight-start
          ◆  Would you like to add a description?
          │  No
          // highlight-end
          │
          └ 
        ```

        14. Add a custom header containing `Bearer YOUR_CONNECTIONSECRET`.
          ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
          │
          ◇  What type of extension would you like to add?
          │  Remote Extension 
          │
          ◇  What would you like to call this extension?
          │  Alby
          │
          ◇  What is the Streaming HTTP endpoint URI?
          │  https://mcp.getalby.com/mcp
          │
          ◇  Please set the timeout for this tool (in secs):
          │  300
          │
          ◇  Would you like to add a description?
          │  No
          │
          ◆  Would you like to add custom headers?
          // highlight-start
          │  ● Yes  / ○ No 
          // highlight-end
          │
          ◆  Header name:
          // highlight-start
          │  Authorization 
          // highlight-end
          │
          ◆  Header name:
          // highlight-start
          │  Bearer nostr+walletconnect://...
          // highlight-end
          │
          ◆  Add another header?
          │  ○ Yes  / ● No
          └
        ```
      </TabItem>  
    </Tabs>
  </TabItem>
</Tabs>

## Example Usage

:::info LLM
Claude Sonnet 3.7 was used for this task. A similarly capable model is recommended to ensure the tool is used correctly.
:::

:::tip Memory Extension
Use the built-in memory extension to save your contacts. e.g. "My friend Rene's lightning address is reneaaron@getalby.com. Please save it to your memory."
:::

### Goose Prompt

> _Pay Rene $1 USD, with message "Hi Rene, it's Roland. I'm making a Goose tutorial!"_

### Goose Output

:::note CLI

<details>
    <summary>Tool Calls</summary>

    ─── fiat_to_sats | alby ──────────────────────────

    fiat_amount: 1

    fiat_currency: USD



    ─── request_invoice | alby ──────────────────────────

    amount_in_sats: 935

    description: ...

    lightning_address: reneaaron@getalby.com



    ─── pay_invoice | alby ──────────────────────────

    invoice: ...



</details>



✅ **Payment Successful!**

I've successfully sent $1 USD (935 sats) to Rene at `reneaaron@getalby.com` with your message: "Hi Rene, it's Roland. I'm making a Goose tutorial!"

**Transaction Details:**
- Amount: 935 sats ($1 USD)
- Fees: 3 sats
- Total cost: 938 sats
- Message: "Hi Rene, it's Roland. I'm making a Goose tutorial!"
- Payment hash: `cd30642470d81c2e8eb04c9ac95e679c53778537aecdf5554b15288ac42fe129`

Rene will receive the message "Thanks, sats received!" as confirmation of the payment.
:::