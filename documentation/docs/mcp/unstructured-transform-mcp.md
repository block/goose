---
title: Unstructured Transform Extension
description: Add Unstructured Transform as a goose Extension to turn documents into clean, LLM-ready Markdown
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add [Unstructured Transform](https://unstructured.io) as a goose extension to turn documents (PDF, DOCX, PPTX, XLSX, HTML, EML, images, and roughly 70 other formats) into clean, structured, LLM-ready output such as Markdown, Element JSON, HTML, or plain text.

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
An OAuth window will open in your browser the first time you use the Transform tools. Follow the prompts to sign in to Unstructured. To authenticate with an API key instead (for example, for headless or CI use), see [Authentication](#authentication) below.
:::

## What is Unstructured Transform?

[Unstructured Transform](https://unstructured.io) is a hosted service that parses documents into structured output ready for LLMs, RAG pipelines, and agents. It partitions files into typed elements, then optionally enriches, chunks, and embeds them. The Transform MCP server is fully remote, so there is nothing to install or run locally.

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
      infoNote="OAuth authentication will happen automatically in your browser when you first use the Transform tools"
    />
  </TabItem>
</Tabs>

## Authentication

Unstructured Transform supports two authentication methods:

- **OAuth (default)**: With no additional configuration, a browser window opens the first time you use the Transform tools. Sign in to Unstructured and goose stores and refreshes the tokens for you.
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
      timeout: 300
  ```

  The `${UNSTRUCTURED_API_KEY}` reference is substituted from the environment at connection time, so the key stays out of the config file. Export the variable in any shell that runs goose.

## Example Usage

Ask goose to transform a local document:

```
Use the unstructured-transform tools to transform ./quarterly-report.pdf into markdown and give me the element count and a preview of the output.
```

goose requests a pre-signed upload URL, uploads the file with its shell tools, starts the transform job, polls until the job completes, and downloads the finished Markdown. Transforms take from about 10 seconds to several minutes, depending on page count.

Parsing requests have the following limits, which the server also reports back to goose through its tool responses:

- Each file must be 50 MB or less in size.
- Each request must have 10 files or fewer.
- Only 5 requests can be running at a time.
