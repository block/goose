---
title: Google Docs Extension
description: Add the Google Docs MCP Server as a goose Extension to read, write, create, and manage Google Docs
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Google Docs MCP Server](https://github.com/hmoses/goose-google-docs-extension) as a goose extension, giving goose full read and write access to your Google Docs and Google Drive.

> Built by [Harold Moses (@hmoses)](https://github.com/hmoses)

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

  <GooseDesktopInstaller
    extensionId="google-docs"
    extensionName="Google Docs"
    description="Read, write, create, and manage Google Docs and Drive files via OAuth 2.0"
    type="stdio"
    command="uvx"
    args={["--from", "git+https://github.com/hmoses/goose-google-docs-extension", "server"]}
    timeout={300}
  />

  </TabItem>
  <TabItem value="cli" label="goose CLI">

  <CLIExtensionInstructions
    name="Google Docs"
    description="Read, write, create, and manage Google Docs and Drive files"
    type="stdio"
    command="uvx --from git+https://github.com/hmoses/goose-google-docs-extension server"
    timeout={300}
  />

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

This extension requires a one-time Google OAuth setup. Follow the [setup guide on GitHub](https://github.com/hmoses/goose-google-docs-extension#setup) to:

1. Create a Google Cloud project and enable the Docs and Drive APIs
2. Create OAuth 2.0 credentials (Desktop app type) and download the JSON file
3. Place the credentials file at `~/.config/goose/google-docs-extension/credentials.json`

Once credentials are in place, restart goose and authenticate:

> *"Check my Google Docs auth status"*

If not yet authenticated:

> *"Authenticate with Google Docs"*

A browser window will open — log in with your Google account and grant permissions. Your token is saved automatically and refreshed when it expires.

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
Place your OAuth credentials file at `~/.config/goose/google-docs-extension/credentials.json`. See the [setup guide](https://github.com/hmoses/goose-google-docs-extension#setup) for full instructions.

**"Access blocked: app not verified"**
Add your email as a test user at [https://console.cloud.google.com/auth/audience](https://console.cloud.google.com/auth/audience).

**"Token expired"**
Delete `~/.config/goose/google-docs-extension/token.json` and re-authenticate by asking goose to *"Authenticate with Google Docs"*.
