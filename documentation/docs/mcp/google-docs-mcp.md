---
title: Google Docs Extension
description: Add the Google Docs MCP Server as a goose Extension to read, write, create, and manage Google Docs
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Google Docs MCP Server](https://github.com/hmoses/goose-google-docs-extension) as a goose extension, giving goose full read and write access to your Google Docs and Google Drive.

> Built by [Harold Moses (@hmoses)](https://github.com/hmoses)

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=uvx&arg=--from&arg=git%2Bhttps%3A%2F%2Fgithub.com%2Fhmoses%2Fgoose-google-docs-extension&arg=server&id=google-docs&name=Google%20Docs&description=Read%2C%20write%2C%20create%2C%20and%20manage%20Google%20Docs%20and%20Drive%20files)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  uvx --from git+https://github.com/hmoses/goose-google-docs-extension server
  ```
  </TabItem>
</Tabs>
:::

## What You Can Do

With this extension, goose can:

| Capability | Tools |
|------------|-------|
| **Read** any Google Doc by URL or ID | `google_docs_read`, `google_docs_get_metadata`, `google_docs_export` |
| **Write & Edit** documents | `google_docs_append_text`, `google_docs_replace_text`, `google_docs_insert_text`, `google_docs_delete_range` |
| **Create** new Google Docs | `google_docs_create` |
| **Format** documents | `google_docs_apply_bold`, `google_docs_set_heading`, `google_docs_batch_update` |
| **Manage** files in Drive | `google_docs_list`, `google_docs_copy`, `google_docs_rename`, `google_docs_delete` |
| **Share** documents | `google_docs_share` |
| **Authenticate** with Google | `google_docs_auth_status`, `google_docs_authenticate` |

## Configuration

:::info Prerequisites
You'll need [uv](https://docs.astral.sh/uv/#installation) installed on your system to run this extension, as it uses `uvx`.

This extension also requires a **Google Cloud project** with OAuth 2.0 credentials. Follow the full setup guide in the [extension README](https://github.com/hmoses/goose-google-docs-extension#setup) to create your credentials and place them at `~/.config/goose/google-docs-extension/credentials.json`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="google-docs"
    extensionName="Google Docs"
    description="Read, write, create, and manage Google Docs and Drive files"
    command="uvx"
    args={["--from", "git+https://github.com/hmoses/goose-google-docs-extension", "server"]}
    timeout={300}
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  1. Run the `configure` command:
  ```sh
  goose configure
  ```

  2. Choose to add a `Command-line Extension`
  ```
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
    │  ○ Remote Extension (Streamable HTTP)
    └
  ```

  3. Give your extension a name
  ```
    ┌   goose-configure
    │
    ◆  What would you like to call this extension?
    │  Google Docs
    └
  ```

  4. Enter the command
  ```
    ┌   goose-configure
    │
    ◆  What command should be used to start the extension?
    │  uvx --from git+https://github.com/hmoses/goose-google-docs-extension server
    └
  ```

  5. Enter a timeout of 300
  ```
    ┌   goose-configure
    │
    ◆  Please set the timeout for this extension (default: 300)
    │  300
    └
  ```

  6. Choose No for environment variables
  ```
    ┌   goose-configure
    │
    ◆  Would you like to add environment variables?
    │  No
    └
  ```
  </TabItem>
</Tabs>

## Example Usage

Once authenticated, you can interact with your Google Docs using natural language.

### Read and Edit a Document

#### goose Prompt

> *Read this Google Doc and fix any typos: https://docs.google.com/document/d/YOUR_DOC_ID/edit*

#### goose Output

:::note Desktop

```
I'll read the document first, then fix any typos I find.

─── google_docs_read ────────────────────────────────
📄 My Document

This is a documnet with some typos in it...

I found 1 typo: "documnet" → "document". Fixing now.

─── google_docs_replace_text ─────────────────────────
find          documnet
replace_with  document

✅ Replaced 1 occurrence of 'documnet' with 'document'.

Done! The document has been updated.
```

:::

### Create a New Document

#### goose Prompt

> *Create a new Google Doc called "Q2 Planning Notes" with a brief intro paragraph*

#### goose Output

:::note Desktop

```
─── google_docs_create ────────────────────────────────
title      Q2 Planning Notes
body_text  Q2 Planning Notes\n\nThis document captures...

✅ Created document: Q2 Planning Notes
🆔 ID: `1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgVE2upms`
🔗 https://docs.google.com/document/d/1BxiMVs0XRA5.../edit

Your new document is ready!
```

:::

### List and Search Documents

#### goose Prompt

> *List all my Google Docs that have "resume" in the name*

#### goose Output

:::note Desktop

```
─── google_docs_list ────────────────────────────────
query   resume

📁 Google Docs

- **Harold Moses Resume**
  🆔 `1Khb9bchiKxiveSEg3DN8d2TRL4ZQy0k9ibhbICSUAw4`
  🕒 Modified: 2026-03-13
  🔗 https://docs.google.com/document/d/1Khb9bchi.../edit

- **Resume - Template**
  🆔 `2Abc1defGHijKLmnoPQrstUVwxYZ1234567890abcdef`
  🕒 Modified: 2026-02-28
  🔗 https://docs.google.com/document/d/2Abc1def.../edit
```

:::

## Troubleshooting

**"credentials.json not found"**
Follow the [setup guide](https://github.com/hmoses/goose-google-docs-extension#setup) to create OAuth credentials and place them at `~/.config/goose/google-docs-extension/credentials.json`.

**"Access blocked: app not verified"**
Add your email as a test user at [https://console.cloud.google.com/auth/audience](https://console.cloud.google.com/auth/audience).

**"Token expired"**
Ask goose: *"Authenticate with Google Docs"* to trigger a fresh OAuth flow.
