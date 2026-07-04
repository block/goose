---
title: hushvert Extension
description: Add hushvert MCP Server as a goose Extension to convert files across formats
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [hushvert MCP Server](https://github.com/hushvert/mcp) as a goose extension to convert files across formats: office documents to PDF, PDF to Word, document interchange (Markdown/HTML/EPUB/LaTeX), and audio/video transcodes, over the hushvert hosted API.

:::tip Quick Install

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=-y&arg=%40hushvert%2Fmcp&id=hushvert&name=hushvert&description=Convert%20files%20across%20formats%20over%20the%20hushvert%20hosted%20API&env=HUSHVERT_API_KEY%3Dhushvert%20API%20Key)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y @hushvert/mcp
  ```
  </TabItem>
</Tabs>
  **Environment Variable**
  ```
  HUSHVERT_API_KEY: <YOUR_API_KEY>
  ```
:::

## Configuration

:::info
Note that you'll need [Node.js](https://nodejs.org/) installed on your system to run this command, as it uses `npx`.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="hushvert"
    extensionName="hushvert"
    description="Convert files across formats over the hushvert hosted API"
    command="npx"
    args={["-y", "@hushvert/mcp"]}
    envVars={[
      { name: "HUSHVERT_API_KEY", label: "hushvert API Key" }
    ]}
    apiKeyLink="https://hushvert.com/developers/keys"
    apiKeyLinkText="HUSHVERT_API_KEY"
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="hushvert"
      description="Convert files across formats over the hushvert hosted API"
      command="npx -y @hushvert/mcp"
      envVars={[
        { key: "HUSHVERT_API_KEY", value: "▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪" }
      ]}
      infoNote={
        <>
          Get a free <a href="https://hushvert.com/developers/keys" target="_blank" rel="noopener noreferrer">HUSHVERT_API_KEY</a> (an <code>hv_live_</code> key) and paste it in.
        </>
      }
    />

  </TabItem>
</Tabs>

## Example Usage

hushvert handles the conversions a browser cannot do on its own, so goose can turn a source file into another format in a single tool call. The four tools are `convert_file`, `convert_poll` (finish a long video job), `list_formats`, and `check_usage`.

### goose Prompt

```
Convert report.docx in this folder to PDF.
```

### goose Output

```
I'll convert that document to PDF with the hushvert extension.

─── convert_file | hushvert ──────────────────────────
input_path: report.docx
to: pdf

Done. The converted file is here:

- output_path: report.pdf
- pair: docx-to-pdf
- status: done

report.docx has been converted to report.pdf in the same folder.
```
