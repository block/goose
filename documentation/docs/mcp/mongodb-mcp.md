---
title: MongoDB Extension
description: Add MongoDB MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

The [MongoDB MCP Server](https://github.com/mongodb-js/mongodb-mcp-server) extension allows Goose to interact directly with your MongoDB databases, enabling comprehensive database operations including querying, document manipulation, collection management, and database administration. This makes it easy to work with your MongoDB databases through natural language interactions.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=mongodb-mcp-server&arg=--connection-string&arg=mongodb://localhost:27017&id=mongodb&name=MongoDB&description=MongoDB%20database%20integration)
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
  **Command**
  ```sh
  npx -y mongodb-mcp-server --connection-string mongodb://localhost:27017
  ```
  </TabItem>
</Tabs>
:::

## Customizing Your Connection

The MongoDB MCP server connects to a single MongoDB database instance using a connection string. The connection string must be specified using the `--connection-string` flag. We're using `mongodb://localhost:27017` as an example here to access a local MongoDB instance, but you can configure this for your own environment.

The MongoDB connection string follows this format:
```
mongodb://username:password@hostname:27017/database
```

Where:
- `username`: Your MongoDB user (optional for local development)
- `password`: Your MongoDB password (optional for local development)
- `hostname`: The host where MongoDB is running (e.g., localhost, IP address, or domain)
- `27017`: The default MongoDB port (change if using a different port)
- `database`: The name of your database (optional, will connect to default)

Examples:
- Local database: `mongodb://localhost:27017`
- Local with credentials: `mongodb://myuser:mypass@localhost:27017/mydb`
- MongoDB Atlas: `mongodb+srv://user:pass@cluster.mongodb.net/database`

:::caution
Never commit connection strings with credentials to version control! Use environment variables or secure configuration management. For MongoDB Atlas, ensure your IP address is whitelisted and use strong passwords.
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`. You'll also need a running MongoDB instance or access to MongoDB Atlas.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="mongodb"
      extensionName="MongoDB"
      description="MongoDB database integration"
      command="npx"
      args={["-y", "mongodb-mcp-server", "--connection-string", "mongodb://localhost:27017"]}
    />
    
    :::info Configure Your Connection String
    If needed, [update the extension](/docs/getting-started/using-extensions#updating-extension-properties) to match to your [MongoDB environment](#customizing-your-connection). For example, change the connection string in the `command` property to use the `mongodb://username:password@hostname:27017/database` format.
    :::

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
    │  ○ Remote Extension 
    └ 
  ```

  3. Name your extension
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
    │  MongoDB
    // highlight-end
    └ 
  ```

  4. Enter the command with the database connection string that matches your [MongoDB environment](#customizing-your-connection)
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What would you like to call this extension?
    │  MongoDB
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y mongodb-mcp-server --connection-string mongodb://localhost:27017
    // highlight-end
    └ 
  ```  

  5. Set the timeout (default 300s is usually sufficient)
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What would you like to call this extension?
    │  MongoDB
    │
    ◇  What command should be run?
    │  npx -y mongodb-mcp-server --connection-string mongodb://localhost:27017
    │
    // highlight-start
    ◆  Please set the timeout for this tool (in secs):
    │  300
    // highlight-end
    └ 
  ```

  6. Choose to add a description. If you select "Yes" here, you will be prompted to enter a description for the extension.
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What would you like to call this extension?
    │  MongoDB
    │
    ◇  What command should be run?
    │  npx -y mongodb-mcp-server --connection-string mongodb://localhost:27017
    │
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    // highlight-start
    ◆  Would you like to add a description?
    │  No
    // highlight-end
    └ 
  ```

  7. Complete the configuration
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What would you like to call this extension?
    │  MongoDB
    │
    ◇  What command should be run?
    │  npx -y mongodb-mcp-server --connection-string mongodb://localhost:27017
    │     
    ◇  Please set the timeout for this tool (in secs):
    │  300
    │
    ◇  Would you like to add a description?
    │  No
    │    
    // highlight-start
    ◆  Would you like to add environment variables?
    │  No 
    // highlight-end
    └  Added MongoDB extension
  ```  

  </TabItem>
</Tabs>

## Available Operations

The MongoDB extension provides comprehensive database management capabilities through natural language interactions. You can perform a wide range of operations including:

### Query Operations
- **Find documents** with complex filters and sorting
- **Aggregate data** using MongoDB's powerful aggregation pipeline
- **Count documents** in collections with optional filtering

### Document Management
- **Insert** single or multiple documents
- **Update** one or many documents with specified criteria
- **Delete** specific documents or bulk deletions

### Collection & Database Administration
- **Create indexes** to optimize query performance
- **Rename collections** for better organization
- **Drop collections** when no longer needed
- **Drop entire databases** (use with caution!)

### Schema & Analytics
- **Analyze collection schemas** to understand document structure
- **List databases and collections** for exploration
- **View collection indexes** and their configurations
- **Check database statistics** and collection storage sizes

Just describe what you want to accomplish in natural language, and Goose will help you execute the appropriate MongoDB operations safely and efficiently.

## Example Usage

#### Goose Prompt
```
Update all users in the premium tier who haven't logged in for more than 90 days - set their status to 'inactive' and add a note about when this change was made.
```

#### Goose Output
```
I've successfully updated inactive premium users using updateMany:

db.users.updateMany(
  {
    subscriptionTier: "premium",
    lastLogin: { $lt: new Date(Date.now() - 90 * 24 * 60 * 60 * 1000) }
  },
  {
    $set: {
      status: "inactive",
      inactiveNote: "Auto-updated to inactive on 2025-05-25 due to 90+ days of inactivity",
      lastModified: new Date()
    }
  }
)

Operation Result:
{
  acknowledged: true,
  matchedCount: 23,
  modifiedCount: 23,
  upsertedId: null,
  upsertedCount: 0
}

Updated 23 premium tier users who hadn't logged in since February 24, 2025. Each document now has:
• status: "inactive"
• inactiveNote: explains the automatic status change
• lastModified: timestamp of this operation

Consider sending re-engagement emails to these users or reviewing their subscription status.
```
