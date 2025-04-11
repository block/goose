---
title: PostgreSQL Extension
description: Add PostgreSQL MCP Server as a Goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';

The PostgreSQL MCP Server extension allows Goose to interact directly with your PostgreSQL databases, enabling database operations, querying, and schema management capabilities. This makes it easy to work with your databases through natural language interactions.

:::tip TLDR

**Command**
```sh
npx -y @modelcontextprotocol/server-postgres postgresql://localhost/mydb
```

It's worth noting that this MCP server only allows connecting to a single predefined database at this time, and the connection URL must be specified in the command.

**Environment Variables**
```
POSTGRES_URL: Your PostgreSQL connection URL

We're using `postgresql://localhost/mydb` as an example here to access a local database, but you can configure this for your own environment.
```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
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

  3. Name your extension
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
    │  PostgreSQL
    // highlight-end
    └ 
  ```

  4. Enter the command with your database connection URL
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
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
    │  Add Extension 
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

  6. Configure your PostgreSQL connection URL
  ```sh
    ┌   goose-configure 
    │
    ◇  What would you like to configure?
    │  Add Extension 
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
    ◆  Would you like to add environment variables?
    │  No 
    // highlight-end
    └  Added PostgreSQL extension
  ```  

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
  1. [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=@modelcontextprotocol/server-postgres&id=postgres&name=PostgreSQL&description=PostgreSQL%20database%20integration&env=POSTGRES_URL%3DYour%20PostgreSQL%20connection%20URL)
  2. Press `Yes` to confirm the installation
  3. Enter your PostgreSQL connection URL in the format: `postgresql://username:password@hostname:5432/database`
  4. Click `Save Configuration`
  5. Scroll to the top and click `Exit` from the upper left corner
  </TabItem>
</Tabs>

## Customizing Your Connection

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

## Example Usage

Let's see how to use Goose with the PostgreSQL extension to perform some common database operations.

### Listing Tables

#### Goose Prompt
```
Show me all tables in the database
```

#### Goose Output
```
I'll query the database to list all tables.

Tables in your database:
- users
- products
- orders
- inventory
- categories

Would you like to see the schema for any specific table?
```

### Querying Data

#### Goose Prompt
```
Show me the top 5 orders by value
```

#### Goose Output
```
I'll write and execute a query to find the highest value orders.

Query results:
| order_id | customer | total_value | order_date |
|----------|----------|-------------|------------|
| 1042     | ACME Inc | $5,280.00   | 2024-03-15 |
| 1067     | TechCorp | $4,150.00   | 2024-03-18 |
| 1039     | DataSys  | $3,900.00   | 2024-03-14 |
| 1055     | InfoTech | $3,675.00   | 2024-03-16 |
| 1071     | DevCo    | $3,450.00   | 2024-03-19 |

Would you like to see more details about any of these orders?
```

## Common Tasks

The PostgreSQL extension enables you to:
- Query and analyze data
- Manage database schema
- Create and modify tables
- Handle data import/export
- Monitor database performance
- Manage users and permissions

Just describe what you want to do in natural language, and Goose will help you accomplish it using the appropriate SQL commands and PostgreSQL features.