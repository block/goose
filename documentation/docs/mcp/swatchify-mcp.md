---
title: Swatchify Extension
description: Add Swatchify MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [Swatchify MCP Server](https://github.com/james-see/swatchify) as a goose extension to extract dominant colors from images using k-means clustering and generate color palette PNGs.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=swatchify-mcp&id=swatchify&name=Swatchify&description=Extract%20dominant%20colors%20from%20images%20using%20k-means%20clustering)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  swatchify-mcp
  ```
  </TabItem>
</Tabs>
:::

## Configuration

:::info
You'll need [Go 1.21+](https://go.dev/doc/install) installed on your system to build the MCP server binary. No API key is required — swatchify processes local images entirely on-device.
:::

### Install the MCP server

```sh
go install github.com/james-see/swatchify/cmd/mcp@v0.4.0
```

This produces a binary named `mcp` in your `$GOPATH/bin`. Rename it so goose (and you) can find it by a distinct name:

```sh
mv "$(go env GOPATH)/bin/mcp" "$(go env GOPATH)/bin/swatchify-mcp"
```

Ensure `$GOPATH/bin` is on your `PATH`:

```sh
export PATH="$PATH:$(go env GOPATH)/bin"
```

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
      extensionId="swatchify"
      extensionName="Swatchify"
      description="Extract dominant colors from images using k-means clustering"
      type="stdio"
      command="swatchify-mcp"
      args={[]}
    />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Swatchify"
      description="Extract dominant colors from images using k-means clustering"
      type="stdio"
      command="swatchify-mcp"
      timeout={300}
    />
  </TabItem>
</Tabs>

## What You Can Do

Swatchify exposes two tools that turn any image into dominant-color data or a palette PNG. It supports JPG, PNG, WebP, GIF, BMP, and TIFF formats.

### Extract dominant colors

Pass an image file path and get the top dominant colors as hex codes with percentage dominance.

**Prompt:**

```
Extract the 5 dominant colors from ~/Photos/sunset.jpg
```

**Output:**

```
Extracted 5 colors from /Users/me/Photos/sunset.jpg:
  #E8A547 (32.3%) rgb(232,165,71)
  #C44820 (24.1%) rgb(196,72,32)
  #8B2D1A (18.7%) rgb(139,45,26)
  #F4D03F (14.5%) rgb(244,208,63)
  #2C1810 (10.4%) rgb(44,24,16)
```

### Generate a palette PNG

Create a horizontal strip of dominant colors as a new PNG file.

**Prompt:**

```
Generate a color palette PNG from ~/Photos/brand-logo.png and save it to ~/palette.png with 8 colors.
```

**Output:**

```
Generated palette with 8 colors from /Users/me/Photos/brand-logo.png
Saved to: /Users/me/palette.png
```

### Exclude white/black backgrounds

Filter out near-white or near-black colors (useful for logos on solid backgrounds).

**Prompt:**

```
Extract colors from ~/logo.png, excluding white and black background colors.
```