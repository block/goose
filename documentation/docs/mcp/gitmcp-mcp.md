---
title: GitMCP Extension
description: Add Git MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';


<YouTubeShortEmbed videoUrl="https://www.youtube.com/embed/6aV8pinnUS8" /> 

This tutorial covers how to add the [Git MCP Server](https://github.com/idosal/git-mcp) as a goose extension to give LLMs live access to GitHub repos, enabling smart documentation search, code exploration, and accurate project insights.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=npx&arg=mcp-remote&arg=https%3A%2F%2Fgitmcp.io%2Fdocs&id=gitmcp&name=GitMCP&description=Remote%20MCP%20server%20from%20gitmcp.io)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  npx -y mcp-remote https://gitmcp.io/docs

  ```
  </TabItem>
</Tabs>
:::

## Configuration

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
    <GooseDesktopInstaller
        extensionId="Git-mcp"
        extensionName="Git mcp"
        description="Git MCP server for up-to-date docs and project insights"
        command="npx"
        args={["-y", "mcp-remote" , "https://gitmcp.io/docs"]}
        cliCommand="npx -y mcp-remote https://gitmcp.io/docs"
        timeout={300}
        note="Note that you'll need Node.js installed on your system to run this command, as it uses npx."
    />
 </TabItem>
  <TabItem value="cli" label="goose CLI">
      <CLIExtensionInstructions
        name="Git MCP"
        description="Git MCP server for up-to-date docs and project insights"
        command="npx -y mcp-remote https://gitmcp.io/docs"
        timeout={300}
      />
  </TabItem>
</Tabs>

## Example Usage

In this example, goose uses GitMCP to pull real-time documentation from the `openai/whisper` GitHub repository, explore how the speech-to-text model works, and surface accurate setup instructions and command-line usage—all directly from the source.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   1. Open a new session in goose Desktop
  </TabItem>
  <TabItem value="cli" label="goose CLI">

  1. Open a terminal and start a new goose session:

  ```sh
  goose session
  ```

  </TabItem>
</Tabs>

### goose Prompt
```
  Can you help me understand how the OpenAI Whisper speech-to-text model works?

  Use the GitHub repository `openai/whisper` to find accurate information.

  Please:
  - Summarize what the model does and its key features
  - Show an example of how to run it from the command line
  - Explain what files or models are needed to get started
  - Include any useful code snippets or setup instructions you find in the repo

  Give me the response in a Markdown guide format.
```
### goose Output

:::note Desktop

I'll help you understand OpenAI Whisper by exploring their GitHub repository. Let me fetch the documentation and relevant information.

`─── owner | repo ──────────────────────────`

Now let me search for some specific setup and usage information:

`─── owner | query ──────── repo ────────────`

Let me search for more technical details and examples:

`─── owner | query ──────── repo ────────────`

Perfect! Now I have all the information I need from the OpenAI Whisper repository. Let me create a comprehensive Markdown guide for you.

OpenAI Whisper Speech-to-Text Model Guide

Overview
Whisper is a general-purpose speech recognition model developed by OpenAI. It's a powerful, multitasking neural network that can perform:

- Multilingual speech recognition (99+ languages)
- Speech translation (convert foreign speech to English text)
- Language identification (detect what language is being spoken)
- Voice activity detection (identify when speech is present)

Key Features:

- ✅ Robust Performance: Trained on 680,000 hours of diverse, multilingual audio data
- ✅ Zero-shot Transfer: Works on many languages without fine-tuning
- ✅ Multiple Model Sizes: From tiny (39M params) to large (1.55B params)
- ✅ Easy to Use: Simple command-line interface and Python API
- ✅ Open Source: MIT licensed with freely available model weights

Architecture
Whisper uses a Transformer sequence-to-sequence model that processes audio in 30-second chunks. It converts audio to log-Mel spectrograms and uses special tokens to specify tasks (transcribe, translate, language ID, etc.).

Command-Line Usage
Basic Transcription

## Getting Started Checklist

1. **✅ Install Python 3.8-3.11**
2. **✅ Install FFmpeg** for your operating system
3. **✅ Install Whisper**: `pip install -U openai-whisper`
4. **✅ Test installation**: `whisper --help`
5. **✅ Try basic transcription**: `whisper your_audio_file.mp3`

### First Run

On your first run, Whisper will automatically download the model weights:
- `tiny`: ~39 MB download
- `base`: ~142 MB download  
- `small`: ~461 MB download
- `medium`: ~1.42 GB download
- `large`: ~2.87 GB download
- `turbo`: ~1.5 GB download

Models are cached locally, so subsequent runs are much faster.


## Tips & Best Practices

### 🎯 **Model Selection**
- **English-only**: Use `.en` models (`base.en`, `small.en`, etc.)
- **Fast transcription**: Use `tiny` or `base` models
- **High accuracy**: Use `medium` or `large` models
- **Translation tasks**: Use `medium` or `large` (NOT `turbo`)

### 🎵 **Audio Quality**
- Clean, clear audio produces better results
- Whisper handles various audio formats (MP3, WAV, FLAC, M4A, etc.)
- Background noise may affect accuracy

### 🌍 **Language Support**
- Supports 99+ languages
- Some languages perform better than others
- Check the [language performance breakdown](https://github.com/openai/whisper#available-models-and-languages) in the repository

### 💾 **Resource Management**
- Larger models require more VRAM/RAM
- Consider your hardware limitations when choosing models
- Use smaller models for real-time applications

Whisper is released under the **MIT License**, making it free to use for both personal and commercial projects.

:::