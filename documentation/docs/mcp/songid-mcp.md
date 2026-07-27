---
title: SongID Extension
description: Add SongID MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [SongID MCP Server](https://github.com/james-see/songid) as a goose extension to identify songs from audio files or microphone input using open-source Chromaprint fingerprinting and AcoustID lookup.

## Prerequisites

:::info
You'll need [chromaprint](https://github.com/acoustid/chromaprint) (fpcalc) installed for audio fingerprinting, and [ffmpeg](https://ffmpeg.org) for microphone capture. No API key is required — songid uses a built-in default AcoustID key that works out of the box.
:::

### Install dependencies

**macOS:**

```sh
brew install chromaprint ffmpeg
```

**Linux (Debian/Ubuntu):**

```sh
apt install libchromaprint-tools ffmpeg
```

### Install the MCP server

**Option 1: Homebrew (recommended on macOS)**

```sh
brew install james-see/tap/songid
```

This installs both `songid` (CLI) and `songid-mcp` (MCP server) binaries.

**Option 2: Go install**

```sh
go install github.com/james-see/songid/cmd/mcp@latest
```

This produces a binary named `mcp` in your `$GOPATH/bin`. Rename it so goose can find it:

```sh
mv $(go env GOPATH)/bin/mcp $(go env GOPATH)/bin/songid-mcp
```

### Verify installation

If you installed via Homebrew (Option 1), verify with the CLI:

```sh
songid doctor
```

If you installed via Go (Option 2), only `songid-mcp` is available. Verify dependencies are on your PATH:

```sh
fpcalc -version
ffmpeg -version
```

## Configure the extension

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop">
  Go to **Settings > Extensions** and add a new extension with:
  - **Name:** SongID
  - **Command:** `songid-mcp`
  - **Type:** stdio
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="songid"
      description="Identify songs from audio files or microphone input"
      command="songid-mcp"
    />
  </TabItem>
</Tabs>

## Tools

SongID provides 3 MCP tools:

| Tool | Description |
|------|-------------|
| `identify_from_microphone` | Record audio from the system microphone and identify the song |
| `identify_from_file` | Identify a song from an audio file path |
| `doctor` | Check if dependencies (fpcalc, ffmpeg, API key) are installed |

## How it works

1. **Capture** — ffmpeg records audio from the microphone (or reads from a file)
2. **Fingerprint** — Chromaprint (fpcalc) generates a compact audio fingerprint
3. **Lookup** — The fingerprint is sent to the AcoustID web service, which returns matching recordings
4. **Metadata** — Song title and artist come from MusicBrainz via AcoustID

All services used are free and open source. No paid subscriptions required.