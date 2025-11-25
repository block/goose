---
title: PostgreSQL Extension
description: Add PostgreSQL MCP Server as a goose Extension
unlisted: true
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/PZlYQ5IthYM" />

Server archived

The PostgreSQL MCP Server extension allows goose to interact directly with your PostgreSQL databases, enabling database operations, querying, and schema management capabilities. This makes it easy to work with your databases through natural language interactions.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=@modelcontextprotocol/server-postgres&arg=Your%20PostgreSQL%20connection%20URL&id=postgres&name=PostgreSQL&description=PostgreSQL%20database%20integration)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @modelcontextprotocol/server-postgres postgresql://localhost/mydb
  ```
  </TabItem>
</Tabs>
:::

## Customizing Your Connection

It's worth noting that this MCP server only allows connecting to a single predefined database at this time, and the connection URL must be specified in the command. We're using `postgresql://localhost/mydb` as an example here to access a local database, but you can configure this for your own environment.

The PostgreSQL connection URL follows this format:
```
postgresql://username:password@hostname:5432/database
```

Where:
- `username`: Your PostgreSQL user
- `password`: Your PostgreSQL password
- `hostname`: The host where PostgreSQL is running (e.g., localhost, IP address, or domain)
- `5432`: The default PostgreSQL port (change if using a different port)
- `database`: The name of your database

Examples:
- Local database: `postgresql://localhost/mydb`
- Local with credentials: `postgresql://myuser:mypass@localhost/mydb`
- Remote database: `postgresql://user:pass@db.example.com:5432/production`

:::caution
Never commit connection strings with credentials to version control! Use environment variables or secure configuration management.
:::



## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="postgres"
    extensionName="PostgreSQL"
    description="PostgreSQL database integration"
    command="npx"
    args={["-y", "@modelcontextprotocol/server-postgres", "Your PostgreSQL connection URL"]}
  />

  :::info
  Enter your PostgreSQL connection URL in the format: `postgresql://username:password@hostname:5432/database`
  :::

  </TabItem>
  <TabItem value="cli" label="goose CLI">
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
    │  PostgreSQL
    // highlight-end
    └ 
  ```

  4. Enter the command with your database connection URL
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What would you like to call this extension?
    │  PostgreSQL
    │
    // highlight-start
    ◆  What command should be run?
    │  npx -y @modelcontextprotocol/server-postgres postgresql://localhost/mydb
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
    │  PostgreSQL
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-postgres postgresql://localhost/mydb
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
    │  PostgreSQL
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-postgres postgresql://localhost/mydb
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

  7. Configure your PostgreSQL connection URL
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension (Connect to a new extension) 
    │
    ◇  What would you like to call this extension?
    │  PostgreSQL
    │
    ◇  What command should be run?
    │  npx -y @modelcontextprotocol/server-postgres postgresql://localhost/mydb
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
    └  Added PostgreSQL extension
  ```  

  </TabItem>
</Tabs>

## Example Usage

The PostgreSQL extension enables you to query and analyze data or give you information about the table structures. The extension is only capable of *read-only* operations, so you can use it to get information about your database, but not to modify it.

Just describe what you want to do in natural language, and goose will help you accomplish it using the appropriate SQL commands and PostgreSQL features.

#### goose Prompt
```
Search my database for any students whose current GPA is higher than the average in their current grade level. Tell me their name, grade, and how much they exceed their grade’s average GPA, and the average GPA.
```

#### goose Output
```
Based on the query results, here are the students who are performing above their grade level average:

Grade 11
  Emma Johnson
    GPA: 4.00
    Grade Average: 3.90
    Exceeds average by: 0.10 points

Grade 12
  John Smith
    GPA: 3.85
    Grade Average: 3.78
    Exceeds average by: 0.08 points

Both students are performing well above their respective grade level averages. Emma Johnson in Grade 11 shows the highest difference above the grade average, exceeding it by 0.10 points, while John Smith in Grade 12 exceeds his grade's average by 0.08 points.
```
