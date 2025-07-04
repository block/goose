---
title: Alby MCP

description: Connect Goose to your Bitcoin Lightning Wallet
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';




This tutorial covers how to add the [Alby MCP Server](https://github.com/getalby/mcp) as a Goose extension to interact with your lightning wallet, make and receive payments, list transactions, convert fiat amounts to sats, request invoices from lightning addresses, and interact with paid MCP tools (such as ones built with [LMCP](https://github.com/getAlby/lmcp)).

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
        1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40getalby%2Fmcp&id=alby&name=Alby&description=Connect%20Goose%20to%20your%20Bitcoin%20Lightning%20Wallet&env=NWC_CONNECTION_STRING%3DNWC%20Connection%20Secret)
        2. Press `Yes` to confirm the installation
        3. Obtain a NWC Connection secret from your lightning wallet (`nostr+walletconnect://...`) and paste it in to the "NWC Connection Secret" field
        4. Click `Add Extension`
        5. Scroll to the top and click `Exit` from the upper left corner
      </TabItem>
      <TabItem value="remote" label="Remote">
        1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40getalby%2Fmcp&id=alby&name=Alby&description=Connect%20Goose%20to%20your%20Bitcoin%20Lightning%20Wallet)
        2. Press `Yes` to confirm the installation
        3. Change the type to "Server-Sent Events (SSE)"
        4. Change the endpoint. To get your endpoint URL, open browser devtools (right click -> inspect) and enter this in the console, with your own NWC connection secret set:
            ```js
            "https://mcp.getalby.com/sse?nwc=" + encodeURIComponent("nostr+walletconnect://...");
            ```
        5. Copy the value, and paste it into the endpoint field.
        6. Click `Add Extension`
        7. Scroll to the top and click `Exit` from the upper left corner
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
        1. Run the `configure` command:
        ```sh
        goose configure
        ```

        2. Choose to add a `Remote Extension`
        ```sh
          ┌   goose-configure 
          │
          ◇  What would you like to configure?
          │  Add Extension (Connect to a new extension) 
          │
          ◆  What type of extension would you like to add?
          │  ○ Built-in Extension 
          │  ○ Command-line Extension (Run a local command or script)
          // highlight-start    
          │  ● Remote Extension 
          // highlight-end    
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
          │  Remote Extension 
          │
          // highlight-start
          ◆  What would you like to call this extension?
          │  Alby
          // highlight-end
          └ 
        ```

        4. Enter the SSE endpoint URI

        :::info SSE Endpoint URI
        _To get the endpoint URL, open browser devtools (right click -> inspect) and enter this in the console, with your own NWC connection secret set:_
          ```js
          "https://mcp.getalby.com/sse?nwc=" + encodeURIComponent("nostr+walletconnect://...");
          ```
        Copy the value that was outputted to the console.
        :::

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
          ◆  What is the SSE endpoint URI?
          │  https://mcp.getalby.com/sse?nwc=YOUR_ENCODED_CONNECTION_SECRET
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
          │  Remote Extension
          │
          ◇  What would you like to call this extension?
          │  Alby
          │
          ◇  What is the SSE endpoint URI?
          │  https://mcp.getalby.com/sse?nwc=YOUR_ENCODED_CONNECTION_SECRET
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
          │  Remote Extension 
          │
          ◇  What would you like to call this extension?
          │  Alby
          │
          ◇  What is the SSE endpoint URI?
          │  https://mcp.getalby.com/sse?nwc=YOUR_ENCODED_CONNECTION_SECRET
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