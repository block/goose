---
title: BTCPayServer Extension
description: Add BTCPayServer MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [BTCPayServer MCP Server](https://github.com/Abhijay007/btcpayserver-mcp) as a Goose extension to enable comprehensive Bitcoin payment processing, store management, user administration, webhook handling and more with full BTCPayServer API coverage.

BTCPayServer MCP provides a streamlined interface for interacting with BTCPayServer's complete API ecosystem, allowing you to manage payments, stores, users, and system operations through natural language commands.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=btcpayserver-mcp&id=btcpayserver-mcp&name=BTCPayServer&description=BTCPayServer%20integration%2C%20providing%20tools%20for%20payment%20processing%2C%20store%20management%2C%20user%20administration%2C%20webhook%20handling%20and%20more&env=BTCPAY_BASE_URL%3Dhttps%3A%2F%2Fyour-btcpay-instance.com&env=BTCPAY_API_KEY%3Dyour_api_key_here)
  2. Click **OK** to confirm the installation
  3. Obtain your `BTCPAY_BASE_URL`, `BTCPAY_API_KEY`, and `BTCPAY_STORE_ID` from [BTCPayServer](https://btcpayserver.org/) and paste them as environment variables
  4. Click **Add Extension**
  5. Click the sidebar button in the top-left to open the sidebar
  6. Navigate to the chat
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  npx -y btcpayserver-mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variables**
  ```
  BTCPAY_BASE_URL: https://your-btcpay-instance.com
  BTCPAY_API_KEY: your_api_key_here
  BTCPAY_STORE_ID: your_default_store_id (optional)
  ```
:::

## Prerequisites

Before you begin, ensure you have:

- A running [BTCPayServer](https://btcpayserver.org/) instance
- Admin or appropriate user access to your BTCPayServer
- A BTCPayServer API key with required permissions

## Configuration

### Step 1: Set Up BTCPayServer API Key

1. **Log into your BTCPayServer instance**
   - Navigate to your BTCPayServer dashboard

2. **Generate an API Key**
   - Go to **Account** → **Manage Account** → **API Keys**
   - Click **Generate Key**

3. **Configure Permissions**
   Select the required permissions based on your use case:
   - **Store management**: `btcpay.store.canmodifystoresettings`
   - **Payment requests**: `btcpay.store.cancreateinvoice`
   - **User management**: `btcpay.user.canmodifyprofile`
   - **Webhooks**: `btcpay.store.webhooks.canmodifywebhooks`
   - **Full access**: Select all permissions for complete functionality

4. **Copy the API Key**
   - Save the generated API key securely
   - Note your BTCPayServer instance URL (e.g., `https://btcpay.example.com`)

### Step 2: Add BTCPayServer MCP to Goose

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="btcpayserver-mcp"
      extensionName="BTCPayServer"
      description="BTCPayServer integration, providing tools for payment processing, store management, user administration, webhook handling and more"
      command="npx"
      args={["-y", "btcpayserver-mcp"]}
      cliCommand="npx -y btcpayserver-mcp"
      timeout={300}
      envVars={[
        { key: "BTCPAY_BASE_URL", value: "https://your-btcpay-instance.com" },
        { key: "BTCPAY_API_KEY", value: "••••••••••••••••" },
        { key: "BTCPAY_STORE_ID", value: "your_default_store_id (optional)" }
      ]}
      apiKeyLink="https://btcpayserver.org/"
      apiKeyLinkText="Get your BTCPayServer credentials"
      
      infoNote={
        <>
          Get your API key from your BTCPayServer instance under{" "}
          <strong>Account → Manage Account → API Keys</strong>. Visit{" "}
          <a href="https://btcpayserver.org/" target="_blank" rel="noopener noreferrer">
            BTCPayServer.org
          </a>{" "}
          to learn more about setting up your instance.
        </>
      }
      note="Note that you'll need Node.js installed on your system to run this command, as it uses npx."
    />
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
      │  Add Extension (Connect to a new extension) 
      │
      ◆  What type of extension would you like to add?
      │  ○ Built-in Extension 
      // highlight-start    
      │  ● Command-line Extension (Run a local command or script)
      // highlight-end    
      │  ○ Remote Extension (SSE) 
      │  ○ Remote Extension (Streaming HTTP) 
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
      │  btcpayserver-mcp
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
      │  btcpayserver-mcp
      │
      // highlight-start
      ◆  What command should be run?
      │  npx -y btcpayserver-mcp
      // highlight-end
      └ 
    ```  

    5. Enter the timeout (default 300s)
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
      │  btcpayserver-mcp
      │
      ◇  What command should be run?
      │  npx -y btcpayserver-mcp
      │
      // highlight-start
      ◆  Please set the timeout for this tool (in secs):
      │  300
      // highlight-end
      └ 
    ```  

    6. Add a description (optional)
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
      │  btcpayserver-mcp
      │
      ◇  What command should be run?
      │  npx -y btcpayserver-mcp
      │
      ◆  Please set the timeout for this tool (in secs):
      │  300
      │
      // highlight-start
      ◇  Would you like to add a description?
      │  No
      // highlight-end
      └ 
    ```

    7. Add environment variables
     ```sh
      ┌   goose-configure 
      │
      // highlight-start
      ◆  Would you like to add environment variables?
      │  Yes
      │
      ◇  Environment variable name:
      │  BTCPAY_BASE_URL
      │
      ◇  Environment variable value:
      │  https://your-btcpay-instance.com
      │
      ◇  Add another environment variable?
      │  Yes
      │
      ◇  Environment variable name:
      │  BTCPAY_API_KEY
      │
      ◇  Environment variable value:
      │  ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
      │
      ◇  Add another environment variable?
      │  Yes
      │
      ◇  Environment variable name:
      │  BTCPAY_STORE_ID
      │
      ◇  Environment variable value:
      │  your_default_store_id
      │
      ◇  Add another environment variable?
      │  No
      // highlight-end
      └  Added btcpayserver-mcp extension
     ```
  </TabItem>
</Tabs>




## Available Tools

The BTCPayServer MCP Server provides three core tools for comprehensive API interaction:

| Tool | Description | Primary Use |
|------|-------------|-------------|
| `get_service_info` | Discover methods available for a service | Exploration and discovery |
| `get_method_info` | Get detailed parameter requirements | Request preparation |
| `btcpay_request` | Execute API calls to BTCPayServer | Performing operations |

## Available Services

BTCPayServer MCP provides access to BTCPayServer's complete API ecosystem including:

- **Payment Services**: Invoices, payment requests, Lightning Network operations
- **Store Management**: Store configuration, payment methods, payouts, user management
- **User & Access Management**: User accounts, API keys, authorization
- **Integration & Automation**: Webhooks, notifications, apps, pull payments
- **System**: Server information and status

Use `get_service_info` to discover all available services and their methods.

## Usage Pattern

For optimal interaction with the BTCPayServer API through MCP, follow this three-step pattern:

### 1. Discover
Use `get_service_info` to explore available methods:
```
get_service_info(serviceName: "invoices")
```

### 2. Understand
Use `get_method_info` to learn parameter requirements:
```
get_method_info(serviceName: "invoices", methodName: "create")
```

### 3. Execute
Use `btcpay_request` to perform the operation:
```
btcpay_request(serviceName: "invoices", methodName: "create", parameters: {storeId: "your-store-id", amount: "10.00", currency: "USD"})
```

## Example Usage

### Creating and Managing Invoices

#### Goose Prompt
```
Create a new Bitcoin invoice for $50 USD for a "Website Development Service" and then check its status. Also show me all recent invoices from the last 7 days.
```

#### Expected Goose Output
```
I'll help you create a Bitcoin invoice for $50 USD and then check its status and recent invoices.

First, let me create the invoice for your website development service:

[Goose creates the invoice using btcpay_request]

✅ **Invoice Created Successfully!**

**Invoice Details:**
- Invoice ID: `inv_abc123def456`
- Amount: $50.00 USD (≈ 0.00125 BTC)
- Description: Website Development Service
- Status: Unpaid
- Payment URL: `https://your-btcpay.com/i/inv_abc123def456`
- Expiry: 15 minutes from now

Now let me check the invoice status:

[Goose checks invoice status]

**Current Status:** Unpaid (Waiting for payment)

Now, let me fetch your recent invoices from the last 7 days:

[Goose retrieves recent invoices]

**Recent Invoices (Last 7 Days):**
1. **Today** - $50.00 USD - Website Development Service (Unpaid)
2. **2 days ago** - $25.00 USD - Consultation Fee (Paid)
3. **5 days ago** - $100.00 USD - E-commerce Setup (Paid)
4. **6 days ago** - $15.00 USD - Logo Design (Expired)

**Summary:**
- Total invoices: 4
- Paid: 2 ($125.00 USD)
- Unpaid: 1 ($50.00 USD)
- Expired: 1 ($15.00 USD)

The new invoice is ready for payment. Share the payment URL with your client to complete the transaction.
```

### Store Management and Configuration

#### Goose Prompt
```
Help me set up a new store called "Tech Gadgets Shop" with Bitcoin and Lightning payment methods enabled. Also configure email notifications and set up a webhook for payment confirmations.
```

#### Expected Goose Output
```
I'll help you set up your new "Tech Gadgets Shop" store with Bitcoin and Lightning payments, email notifications, and webhook configuration.

Let me start by creating the store:

[Goose creates the store using stores service]

✅ **Store Created Successfully!**

**Store Details:**
- Store ID: `store_tech789`
- Name: Tech Gadgets Shop
- Default Currency: USD
- Status: Active

Now let me configure the payment methods:

[Goose configures Bitcoin and Lightning payment methods]

✅ **Payment Methods Configured:**
- **Bitcoin (On-chain)**: Enabled with 1 confirmation requirement
- **Lightning Network**: Enabled with instant settlements
- **Payment Timeout**: 15 minutes for invoices

Setting up email notifications:

[Goose configures email settings]

✅ **Email Notifications Configured:**
- **SMTP Server**: Configured
- **Invoice Notifications**: Enabled
- **Payment Confirmations**: Enabled
- **Daily Reports**: Enabled

Finally, setting up the webhook for payment confirmations:

[Goose creates webhook]

✅ **Webhook Configuration Complete:**
- **Webhook URL**: `https://your-app.com/webhook/btcpay`
- **Events**: Invoice settled, Payment received
- **Security**: Webhook secret configured
- **Status**: Active

**Next Steps:**
1. Test the payment flow with a small test invoice
2. Configure your application to handle webhook events
3. Set up rate exchange providers if needed
4. Review security settings and permissions

Your "Tech Gadgets Shop" is now ready to accept Bitcoin and Lightning payments with full automation!
```

## Troubleshooting

### Common Issues

**Authentication Errors:**
- Verify API token has correct permissions
- Check if token hasn't expired
- Ensure token is properly set in environment variables
- Confirm BTCPayServer instance URL is correct

**Connection Issues:**
- Verify network connectivity to BTCPayServer instance
- Check firewall settings and DNS resolution
- Ensure BTCPayServer is running and accessible
- Test API connectivity with curl or browser

**Permission Errors:**
- Review API key permissions in BTCPayServer dashboard
- Ensure user has access to the specific store
- Check if operation requires admin privileges
- Verify store ID is correct and accessible

**Rate Limiting:**
- Monitor API usage in BTCPayServer logs
- Implement exponential backoff for retries
- Consider upgrading BTCPayServer hosting for higher limits
- Cache frequently accessed data to reduce API calls


## Getting Help

If you encounter issues or need assistance:

1. **Check BTCPayServer Documentation**
   - [BTCPayServer Docs](https://docs.btcpayserver.org/)
   - [API Reference](https://docs.btcpayserver.org/API/Greenfield/v1/)

2. **Review Repository Resources**
   - [BTCPayServer MCP Repository](https://github.com/Abhijay007/btcpayserver-mcp)
   - [Issue Tracker](https://github.com/Abhijay007/btcpayserver-mcp/issues)

3. **Community Support**
   - [Goose Discord](https://discord.gg/block-opensource)

4. **Contributing**
   - Report bugs and feature requests
   - Submit pull requests for improvements
   - Help with documentation and examples

:::note Development Status
This MCP server is in **beta**. Some methods might not work properly. Please help improve the project by reporting issues and contributing pull requests to keep it updated with the latest BTCPayServer API version.
:::

## Next Steps

With BTCPayServer MCP enabled in Goose, you can:

- **Automate payment processing** with natural language commands
- **Manage multiple stores** and their configurations
- **Monitor transactions** and generate reports
- **Set up complex payment workflows** with webhooks
- **Integrate Bitcoin payments** into any business process

Start by exploring the available services with `get_service_info` to discover all the capabilities available to you through this powerful integration.
