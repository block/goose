---
title: Google Docs Extension
description: Add Google Docs MCP Server as a Goose Extension — read, write, create, and manage Google Docs and Drive files directly from Goose.
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Google Docs MCP Extension](https://github.com/hmoses/goose-google-docs-extension) — built by [Harold Moses (@hmoses)](https://github.com/hmoses) — as a Goose extension, giving Goose full read and write access to your Google Docs and Google Drive.

Once set up, you can ask Goose to read documents, make edits, create new docs, find and replace text, apply formatting, list your files, share documents, and more — all in natural language.

## Tools Available (18)

| Tool | Description |
|------|-------------|
| `google_docs_auth_status` | Check Google authentication status |
| `google_docs_authenticate` | Trigger OAuth 2.0 browser login |
| `google_docs_read` | Read full text content of a Google Doc |
| `google_docs_get_metadata` | Get title, document ID, and revision info |
| `google_docs_create` | Create a new Google Doc with optional content |
| `google_docs_append_text` | Append text to the end of a document |
| `google_docs_replace_text` | Find and replace text across a document |
| `google_docs_insert_text` | Insert text at a specific character index |
| `google_docs_delete_range` | Delete a range of characters |
| `google_docs_apply_bold` | Apply bold formatting to a text range |
| `google_docs_set_heading` | Set heading style (H1–H6 or Normal Text) |
| `google_docs_batch_update` | Send raw Docs API batchUpdate for advanced edits |
| `google_docs_list` | List Google Docs in Drive (with optional search) |
| `google_docs_copy` | Duplicate a document with a new title |
| `google_docs_delete` | Move a document to trash |
| `google_docs_rename` | Rename a document |
| `google_docs_share` | Share a document with an email address |
| `google_docs_export` | Export a document as plain text or HTML |

## Prerequisites

- **Python 3.10+** — [Download](https://www.python.org/downloads/)
- **uv** (recommended) or pip — [Install uv](https://github.com/astral-sh/uv)
- **Goose** installed

## Configuration

### Step 1 — Clone and Install

```bash
git clone https://github.com/hmoses/goose-google-docs-extension
cd goose-google-docs-extension
chmod +x install.sh && ./install.sh
```

The installer:
- Creates a Python virtual environment in `.venv/`
- Installs all dependencies (`mcp`, `google-auth`, `google-api-python-client`)
- Registers the extension in `~/.config/goose/config.yaml`

### Step 2 — Google Cloud Setup (One-Time)

#### Create a Project

Go to [https://console.cloud.google.com/projectcreate](https://console.cloud.google.com/projectcreate) and create a new project.

#### Enable APIs

Enable both APIs in your project:

- [Google Docs API](https://console.cloud.google.com/apis/library/docs.googleapis.com)
- [Google Drive API](https://console.cloud.google.com/apis/library/drive.googleapis.com)

#### Create OAuth 2.0 Credentials

1. Go to [https://console.cloud.google.com/apis/credentials](https://console.cloud.google.com/apis/credentials)
2. Click **Create Credentials → OAuth client ID**
3. Configure the OAuth consent screen if prompted:
   - User type: **External**
   - Fill in App name and support email
4. Application type: **Desktop app**
5. Click **Create** and **Download JSON**
6. Move the file into place:

```bash
mv ~/Downloads/client_secret_*.json ~/.config/goose/google-docs-extension/credentials.json
```

#### Add Yourself as a Test User

If your OAuth consent screen is **External** and unverified:

1. Go to [https://console.cloud.google.com/auth/audience](https://console.cloud.google.com/auth/audience)
2. Under **Test users**, click **+ Add Users**
3. Add your Google account email
4. Click **Save**

### Step 3 — Register with Goose

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>

  The installer automatically registers the extension. After restarting Goose Desktop, you should see **Google Docs** in your Extensions list. Toggle it on if it isn't already enabled.

  To add manually via the Desktop UI:
  1. Click the sidebar button (top-left)
  2. Click **Extensions → Add custom extension**
  3. Fill in:
     - **Type**: `Standard IO`
     - **ID**: `google-docs`
     - **Name**: `Google Docs`
     - **Description**: `Read, write, create, and manage Google Docs and Drive files`
     - **Command**: `/path/to/goose-google-docs-extension/.venv/bin/python`
     - **Arguments**: `/path/to/goose-google-docs-extension/server.py`

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

  The installer registers the extension automatically. To verify:

  ```bash
  grep -A 8 "google-docs:" ~/.config/goose/config.yaml
  ```

  You should see:

  ```yaml
  google-docs:
    name: google-docs
    display_name: Google Docs
    description: "Read, write, create, and manage Google Docs and Drive files"
    cmd: /path/to/.venv/bin/python
    args:
      - /path/to/server.py
    enabled: true
    type: stdio
    timeout: 300
  ```

  To add manually via CLI:

  ```
  ┌   goose-configure
  │
  ◇  What would you like to configure?
  │  Add Extension
  │
  ◇  What type of extension would you like to add?
  │  Command-line Extension
  │
  ◇  What would you like to call this extension?
  │  Google Docs
  │
  ◇  What command should be run?
  │  /path/to/goose-google-docs-extension/.venv/bin/python /path/to/server.py
  │
  ◇  Please set the timeout for this tool (in secs):
  │  300
  │
  └  Added Google Docs extension
  ```

  </TabItem>
</Tabs>

### Step 4 — Authenticate

Restart Goose, then authenticate:

```
check google docs auth status
```

If not authenticated:

```
authenticate with google docs
```

A browser window will open. Log in with your Google account and grant permissions. Your token is saved and auto-refreshed — you only need to do this once.

## Example Usage

### goose Prompt

```
Read this Google Doc and summarize it:
https://docs.google.com/document/d/1Khb9bchiKxiveSEg3DN8d2TRL4ZQy0k9ibhbICSUAw4/edit
```

### goose Output

:::note Desktop

```
📄 **My Resume**

Harold Moses
Technical Writer II...

[full document text]

Here's a summary: The document is a professional resume for Harold Moses,
a Technical Writer with experience at Block and Sony Interactive Entertainment...
```

:::

### goose Prompt

```
Create a new Google Doc called "Q2 Planning Notes" with a brief intro paragraph
```

### goose Output

:::note Desktop

```
✅ Created document: Q2 Planning Notes
🆔 `1aBcDeFgHiJkLmNoPqRsTuVwXyZ`
🔗 https://docs.google.com/document/d/1aBcDeFgHiJkLmNoPqRsTuVwXyZ/edit
```

:::

### goose Prompt

```
In my strategy doc, find and replace all instances of "Q1" with "Q2":
https://docs.google.com/document/d/YOUR_DOC_ID/edit
```

### goose Output

:::note Desktop

```
✅ Replaced 7 occurrence(s) of 'Q1' with 'Q2'.
```

:::

### goose Prompt

```
List all my Google Docs
```

### goose Output

:::note Desktop

```
📁 Google Docs

- **Q2 Planning Notes**
  🆔 `1aBcDeFg...`
  🕒 2025-03-13T10:30:00.000Z
  🔗 https://docs.google.com/document/d/1aBcDeFg.../edit

- **My Resume**
  🆔 `1Khb9bch...`
  🕒 2025-03-13T09:15:00.000Z
  🔗 https://docs.google.com/document/d/1Khb9bch.../edit
```

:::

## Troubleshooting

**"credentials.json not found"**
Place your OAuth credentials at:
`~/.config/goose/google-docs-extension/credentials.json`

**"Access blocked: app not verified"**
Add your email as a test user at:
[https://console.cloud.google.com/auth/audience](https://console.cloud.google.com/auth/audience)

**"Token expired / invalid_grant"**
Delete `~/.config/goose/google-docs-extension/token.json` and re-authenticate.

**Extension not loading in Goose**
- Restart Goose after installation
- Check `~/.config/goose/config.yaml` for the `google-docs:` block
- Re-run `./install.sh` to re-register

## Security

- OAuth tokens stored locally at `~/.config/goose/google-docs-extension/token.json`
- Only requests `documents` and `drive` Google OAuth scopes
- No data is sent anywhere except Google's APIs
- Revoke access anytime at [https://myaccount.google.com/permissions](https://myaccount.google.com/permissions)

## Source

- **GitHub**: [https://github.com/hmoses/goose-google-docs-extension](https://github.com/hmoses/goose-google-docs-extension)
- **Author**: [Harold Moses (@hmoses)](https://github.com/hmoses)
- **License**: MIT
