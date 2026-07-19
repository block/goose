---
title: Latchshot Extension
description: Add Latchshot as a goose Extension for guarded public-webpage screenshots and PDFs
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add [Latchshot](https://github.com/BaiqingL/latchshot-mcp) as a goose extension to capture public webpages as PNG, JPEG, or PDF artifacts. The hosted browser accepts only public HTTP or HTTPS targets on ports 80 and 443, applies bounded render options, and counts only successful captures against quota.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?type=streamable_http&url=https%3A%2F%2Flatchshot.fly.dev%2Fmcp&id=latchshot&name=Latchshot&description=Capture%20public%20webpages%20as%20PNG%2C%20JPEG%2C%20or%20PDF%20with%20bounded%20options%20and%20network%20guardrails&header=Authorization%3DBearer%20YOUR_LATCHSHOT_API_KEY)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  Add a `Remote Extension (Streamable HTTP)` extension type with:

  **Endpoint URL**
  ```
  https://latchshot.fly.dev/mcp
  ```

  **Custom Request Header**
  ```
  Authorization: Bearer <YOUR_LATCHSHOT_API_KEY>
  ```
  </TabItem>
</Tabs>
:::

## Configuration

Get a [recurring Free-plan API key](https://latchshot.fly.dev/?intent=goose#trial), then configure the remote extension:

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="latchshot"
      extensionName="Latchshot"
      description="Capture public webpages as PNG, JPEG, or PDF with bounded options and network guardrails"
      type="http"
      url="https://latchshot.fly.dev/mcp"
      envVars={[
        { name: "Authorization", label: "Bearer YOUR_LATCHSHOT_API_KEY" }
      ]}
      apiKeyLink="https://latchshot.fly.dev/?intent=goose#trial"
      apiKeyLinkText="Latchshot API key"
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Latchshot"
      description="Capture public webpages as PNG, JPEG, or PDF with bounded options and network guardrails"
      type="http"
      url="https://latchshot.fly.dev/mcp"
      timeout={300}
      envVars={[
        { key: "Authorization", value: "Bearer ls_live_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx" }
      ]}
      infoNote={
        <>
          Get a <a href="https://latchshot.fly.dev/?intent=goose#trial" target="_blank" rel="noopener noreferrer">Latchshot API key</a> and paste it after <code>Bearer</code>. The recurring Free plan includes 100 successful renders per UTC calendar month and requires no credit card.
        </>
      }
    />
  </TabItem>
</Tabs>

Latchshot exposes two tools:

- `capture_page` returns an inline PNG or JPEG image, or an embedded PDF resource, plus render and quota diagnostics.
- `get_usage` reads the current plan, successful-render quota, reset time, and remaining allowance without consuming render quota.

## Example Usage

Ask goose to capture a public page and summarize the diagnostics:

### goose Prompt

```
Use Latchshot to capture https://example.com as an 800 by 450 PNG. Then report the byte count, render time, navigation state, fonts state, scripts state, and remaining quota.
```

### goose Output

```
The byte count for the captured page is 15155.
The render time was 450 ms.
The navigation state is "complete".
The fonts state is "original".
The scripts state is "active".
The remaining quota is 1.
```

This output was observed in a live goose 1.43.0 validation run. Artifact size and render time vary with the target page and selected options.

## Boundaries

- Targets must resolve entirely to public network addresses; credentials in target URLs and private, loopback, link-local, or metadata addresses are rejected.
- The tools do not expose payment or plan-upgrade actions.
- Failed direct captures do not consume quota, and automatic overages are disabled.
- Review the target site's terms and obtain permission before automating captures.
