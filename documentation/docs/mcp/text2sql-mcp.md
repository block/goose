---
title: text2sql Extension
description: Add text2sql MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [text2sql MCP Server](https://github.com/cpenniman12/text2sql-mcp) as a goose extension to ask any SQL database questions in natural language. The agent explores the schema, writes SQL, executes it against the real database, and self-corrects on errors — no RAG layer, no schema descriptions, no embeddings required. Just a connection string and a frontier model.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=uvx&arg=text2sql-mcp&id=text2sql&name=text2sql&description=Ask%20your%20SQL%20database%20questions%20in%20natural%20language&env=TEXT2SQL_DATABASE_URL%3DSQLAlchemy%20connection%20string&env=ANTHROPIC_API_KEY%3DAnthropic%20API%20key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  uvx text2sql-mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variables**
  ```
  TEXT2SQL_DATABASE_URL: <SQLALCHEMY_CONNECTION_STRING>
  ANTHROPIC_API_KEY: <YOUR_ANTHROPIC_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this command, as it uses `uvx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

    <GooseDesktopInstaller
      extensionId="text2sql"
      extensionName="text2sql"
      description="Ask your SQL database questions in natural language"
      type="stdio"
      command="uvx"
      args={["text2sql-mcp"]}
      timeout={300}
      envVars={[
        { name: "TEXT2SQL_DATABASE_URL", label: "SQLAlchemy connection string (e.g. sqlite:///mydb.db)" },
        { name: "ANTHROPIC_API_KEY", label: "Anthropic API key" }
      ]}
      apiKeyLink="https://console.anthropic.com/settings/keys"
      apiKeyLinkText="Anthropic API Key"
    />

  </TabItem>
  <TabItem value="cli" label="goose CLI">

    <CLIExtensionInstructions
      name="text2sql"
      description="Ask your SQL database questions in natural language"
      type="stdio"
      command="uvx text2sql-mcp"
      timeout={300}
      envVars={[
        { key: "TEXT2SQL_DATABASE_URL", value: "sqlite:///mydb.db" },
        { key: "ANTHROPIC_API_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      infoNote={
        <>
          <code>TEXT2SQL_DATABASE_URL</code> is any SQLAlchemy URL — SQLite, Postgres, MySQL, Snowflake, BigQuery, etc. Get your Anthropic API key from{" "}
          <a href="https://console.anthropic.com/settings/keys" target="_blank" rel="noopener noreferrer">
            console.anthropic.com
          </a>.
        </>
      }
    />

  </TabItem>
</Tabs>

:::info
text2sql defaults to `anthropic:claude-sonnet-4-6`. To use OpenAI instead, set `TEXT2SQL_MODEL=openai:gpt-4.1` and provide `OPENAI_API_KEY`. See the [text2sql-framework README](https://github.com/cpenniman12/text2sql-framework) for the full list of supported options, including custom business-rule instructions and example libraries.
:::

## Tools

text2sql exposes a single tool:

- **`query`** — ask the database a natural-language question. Returns the SQL the agent ran, the resulting rows, the row count, and the number of exploratory SQL calls the agent made along the way.

## Example Usage

Point text2sql at any SQL database — here we use a SQLite copy of the [Chinook](https://github.com/lerocha/chinook-database) sample database — and ask a question that requires the agent to figure out the schema on its own.

### goose Prompt

```
What were the top 5 customers by total invoice amount last year?
```

### goose Output

```
I'll ask the text2sql agent to figure this out. Behind the scenes it explored
the schema, found the `customers` and `invoices` tables, joined them, and
filtered by the previous calendar year.

SQL:

  SELECT
    c.FirstName || ' ' || c.LastName AS customer,
    ROUND(SUM(i.Total), 2)            AS total_spent
  FROM invoices i
  JOIN customers c ON c.CustomerId = i.CustomerId
  WHERE strftime('%Y', i.InvoiceDate) = strftime('%Y', date('now', '-1 year'))
  GROUP BY c.CustomerId
  ORDER BY total_spent DESC
  LIMIT 5;

Top 5 customers:

  Helena Holý           49.62
  Richard Cunningham    47.62
  Luis Rojas            46.62
  Ladislav Kovács       45.62
  Hugh O'Reilly         45.62

The agent made 6 exploratory SQL calls before writing the final query —
listing tables, inspecting the `customers` and `invoices` schemas,
checking the date format, and verifying the join works.
```
