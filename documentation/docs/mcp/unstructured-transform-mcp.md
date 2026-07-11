---
title: Unstructured Transform Extension
description: Add Unstructured Transform as a goose Extension to turn documents into clean, LLM-ready Markdown
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add [Unstructured Transform](https://unstructured.io) as a goose extension to turn documents (PDF, DOCX, PPTX, XLSX, HTML, EML, images, and roughly 70 other formats) into clean, structured, LLM-ready output such as Markdown, Element JSON, HTML, or plain text. You will need goose [installed](/docs/getting-started/installation) with a model provider configured.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Fmcp.transform.unstructured.io&id=unstructured-transform&name=Unstructured%20Transform&description=Turn%20documents%20into%20clean%2C%20structured%2C%20LLM-ready%20Markdown%2C%20JSON%2C%20HTML%2C%20or%20text&timeout=300)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    Use `goose configure` to add a `Remote Extension (Streamable HTTP)` extension type with:

    **Endpoint URL**
    ```
    https://mcp.transform.unstructured.io
    ```
  </TabItem>
</Tabs>
:::

:::info OAUTH FLOW
An OAuth window will open in your browser when goose first connects to the extension (at the start of your next session or chat). Follow the prompts to sign in to Unstructured. Quick Install and the deeplink use OAuth only; to authenticate with an API key instead (for example, for headless or CI use), skip Quick Install and see [Authentication](#authentication) below.
:::

## What is Unstructured Transform?

[Unstructured Transform](https://unstructured.io) is a hosted service that turns raw documents into agent-ready data: clean, structured output that agents, LLMs, and RAG pipelines can act on directly instead of wrestling with file formats. It partitions files into typed elements, then optionally enriches, chunks, and embeds them, so the output arrives ready for retrieval and reasoning. The Transform MCP server is fully remote, so there is nothing to install or run locally.

The extension provides four tools that goose drives end to end: `request_file_upload_url`, `transform_files`, `check_transform_status`, and `get_transform_results`. Uploads and result downloads use pre-signed URLs, which goose handles with its built-in shell tools.

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="unstructured-transform"
      extensionName="Unstructured Transform"
      description="Turn documents into clean, structured, LLM-ready Markdown, JSON, HTML, or text"
      type="http"
      url="https://mcp.transform.unstructured.io"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="unstructured-transform"
      description="Turn documents into clean, structured, LLM-ready Markdown, JSON, HTML, or text"
      type="http"
      url="https://mcp.transform.unstructured.io"
      timeout={300}
      commandNote="OAuth authentication happens automatically in your browser when goose first connects to the extension"
    />
  </TabItem>
</Tabs>

## Authentication

Unstructured Transform supports two authentication methods:

- **OAuth (default)**: With no additional configuration, a browser window opens when goose first connects to the extension. Sign in to Unstructured once; goose stores the tokens in your system keychain and refreshes them automatically, so later sessions connect without the browser.
- **API key**: For headless or CI use, send an Unstructured API key as a bearer token instead. [Get an API key](https://transform.unstructured.io/get-started), then add a request header to the extension. In `~/.config/goose/config.yaml`:

  ```yaml
  extensions:
    unstructured-transform:
      enabled: true
      type: streamable_http
      name: unstructured-transform
      uri: https://mcp.transform.unstructured.io
      headers:
        Authorization: Bearer ${UNSTRUCTURED_API_KEY}
      env_keys:
        - UNSTRUCTURED_API_KEY
      timeout: 300
  ```

  The `env_keys` entry is required: goose substitutes `${UNSTRUCTURED_API_KEY}` only from keys it resolves through `env_keys`, which reads the `UNSTRUCTURED_API_KEY` environment variable (falling back to a secret stored in goose's keyring). Export the variable in the shell that runs goose, or put the literal key in the header value instead.

## Example Usage

Ask goose to transform a local document:

```
Use the unstructured-transform tools to transform ./quarterly-report.pdf into markdown. Upload the file bytes yourself with a plain HTTP PUT to the pre-signed upload URL, without an Authorization header on that request. Then give me the element count and a preview of the output.
```

goose requests a pre-signed upload URL, uploads the file with its shell tools, starts the transform job, polls until the job completes, and downloads the finished Markdown. goose shows the result inline by default; ask it to save the output to a file if you want it on disk. Transforms take from about 10 seconds to several minutes, depending on page count.

The upload and download URLs are pre-signed and reject requests that carry an `Authorization` header, which is why the example prompt calls it out.

Parsing requests have the following limits, which the server also reports back to goose through its tool responses:

- Each file must be 50 MB or less in size.
- Each request must have 10 files or fewer.
- Only 5 requests can be running at a time.
