---
title: SongID Extension
description: Add SongID MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';

This tutorial covers how to add the [SongID MCP Server](https://github.com/james-see/songid) as a goose extension to identify songs from audio files or microphone input using open-source Chromaprint fingerprinting and AcoustID lookup.

:::tip Quick Install
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=songid-mcp&id=songid&name=SongID&description=Identify%20songs%20from%20audio%20files%20or%20microphone%20input)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  songid-mcp
  ```
  </TabItem>
</Tabs>
:::

## Configuration

:::info
You'll need [chromaprint](https://github.com/acoustid/chromaprint) (fpcalc) installed for audio fingerprinting, and [ffmpeg](https://ffmpeg.org) for microphone capture. No API key is required — songid uses a built-in default AcoustID key that works out of the box.
:::

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

### Install dependencies

```sh
brew install chromaprint ffmpeg    # macOS
# apt install libchromaprint-tools ffmpeg    # Linux (Debian/Ubuntu)
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

## Add the extension to goose

<CLIExtensionInstructions command="songid-mcp" />

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