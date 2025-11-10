---
sidebar_position: 1
title: Quickstart
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import Link from "@docusaurus/Link";
import { IconDownload } from "@site/src/components/icons/download";
import { RateLimits } from '@site/src/components/RateLimits';
import { ModelSelectionTip } from '@site/src/components/ModelSelectionTip';
import YouTubeShortEmbed from '@site/src/components/YouTubeShortEmbed';
import MacDesktopInstallButtons from '@site/src/components/MacDesktopInstallButtons';
import WindowsDesktopInstallButtons from '@site/src/components/WindowsDesktopInstallButtons';
import LinuxDesktopInstallButtons from '@site/src/components/LinuxDesktopInstallButtons';
import { PanelLeft } from 'lucide-react';

# goose in 5 minutes

goose is an extensible open source AI agent that enhances your software development by automating coding tasks. 

This quick tutorial will guide you through:

- ✅ Installing goose
- ✅ Configuring your LLM
- ✅ Building a small app
- ✅ Adding an MCP server

Let's begin 🚀

## Install goose

<Tabs>
  <TabItem value="mac" label="macOS" default>
    Choose to install the Desktop and/or CLI version of goose:

    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        <MacDesktopInstallButtons/>
        <div style={{ marginTop: '1rem' }}>
          1. Unzip the downloaded zip file.
          2. Run the executable file to launch the goose Desktop application.
        </div>
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        Run the following command to install goose:

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="linux" label="Linux">
    Choose to install the Desktop and/or CLI version of goose:

    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        <LinuxDesktopInstallButtons/>
        <div style={{ marginTop: '1rem' }}>
          **For Debian/Ubuntu-based distributions:**
          1. Download the DEB file
          2. Navigate to the directory where it is saved in a terminal
          3. Run `sudo dpkg -i (filename).deb`
          4. Launch goose from the app menu

        </div>
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        Run the following command to install the goose CLI on Linux:

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```   
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="windows" label="Windows">
    Choose to install the Desktop and/or CLI version of goose:

    <Tabs groupId="interface">
      <TabItem value="ui" label="goose Desktop" default>
        <WindowsDesktopInstallButtons/>
        <div style={{ marginTop: '1rem' }}>
          1. Unzip the downloaded zip file.
          2. Run the executable file to launch the goose Desktop application.
        </div>
      </TabItem>
      <TabItem value="cli" label="goose CLI">
        
        Run the following command in **Git Bash**, **MSYS2**, or **PowerShell** to install the goose CLI natively on Windows:

        ```bash
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```
        
        Learn about prerequisites in the [installation guide](/docs/getting-started/installation).

        :::info PATH Warning And Keyring
        If you see a PATH warning after installation, you'll need to add Goose to your PATH before running `goose configure`. See the [Windows CLI installation instructions](/docs/getting-started/installation) for detailed steps.

        If prompted during configuration, choose to not store to keyring. If you encounter keyring errors, see the [Windows setup instructions](/docs/getting-started/installation#set-llm-provider) for more information.
        :::

      </TabItem>
    </Tabs>
  </TabItem>
</Tabs>

## Configure Provider

Goose works with [supported LLM providers](/docs/getting-started/providers) that give Goose the AI intelligence it needs to understand your requests. On first use, you'll be prompted to configure your preferred provider.

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>

    On the welcome screen, choose `Automatic setup with Tetrate Agent Router`.
    
    Goose will open a browser for you to authenticate.
      
    :::info Free Credits Offer
    You'll receive $10 in free credits the first time you automatically authenticate with Tetrate through Goose. This offer is available to both new and existing Tetrate users and is valid through October 2, 2025.
    :::

    Tetrate provides access to multiple AI models with built-in rate limiting and automatic failover. If you prefer a different provider, choose automatic setup with OpenRouter or manually configure a provider.
    
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    
    On the welcome screen, choose `Tetrate Agent Router Service Login`. Use the up and down arrow keys to navigate the options, then press `Enter` to select. 
    
    Goose will open a browser for you to authenticate.
      
    :::info Free Credits Offer
    You'll receive $10 in free credits the first time you automatically authenticate with Tetrate through Goose. This offer is available to both new and existing Tetrate users and is valid through October 2, 2025.
    :::

    Tetrate provides access to multiple AI models with built-in rate limiting and automatic failover. If you prefer a different provider, choose automatic setup with OpenRouter or manually configure a provider.

  </TabItem>
</Tabs>

## Start Session
Sessions are single, continuous conversations between you and Goose. Let's start one.

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
        After choosing an LLM provider, click the `Home` button in the sidebar.

        Type your questions, tasks, or instructions directly into the input field, and Goose will immediately get to work.
    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        1. Make an empty directory (e.g. `goose-demo`) and navigate to that directory from the terminal.
        2. To start a new session, run:
        ```sh
        goose session
        ```

        :::tip Goose Web
        CLI users can also start a session in [Goose Web](/docs/guides/goose-cli-commands#web), a web-based chat interface:
        ```sh
        goose web --open
        ```
        :::

    </TabItem>
</Tabs>

## Write Prompt

From the prompt, you can interact with Goose by typing your instructions exactly as you would speak to a developer.

Let's ask Goose to make a tic-tac-toe game!

```
create an interactive browser-based tic-tac-toe game in javascript where a player competes against a bot
```

Goose will create a plan and then get right to work on implementing it. Once done, your directory should contain a JavaScript file as well as an HTML page for playing.


## Enable an Extension

While you're able to manually navigate to your working directory and open the HTML file in a browser, wouldn't it be better if Goose did that for you? Let's give Goose the ability to open a web browser by enabling the [`Computer Controller` extension](/docs/mcp/computer-controller-mcp).

<Tabs groupId="interface">

    <TabItem value="ui" label="Goose Desktop" default>
        1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar.
        2. Click `Extensions` in the sidebar menu.
        3. Toggle the `Computer Controller` extension to enable it. This extension enables webscraping, file caching, and automations.
        4. Return to your session to continue.
        5. Now that Goose has browser capabilities, let's ask it to launch your game in a browser:
    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        1. End the current session by entering `Ctrl+C` so that you can return to the terminal's command prompt.
        2. Run the configuration command
        ```sh
        goose configure
        ```
        3. Choose `Add Extension` > `Built-in Extension` > `Computer Controller`, and set the timeout to 300s. This extension enables webscraping, file caching, and automations.
        ```
        ┌   goose-configure
        │
        ◇  What would you like to configure?
        │  Add Extension
        │
        ◇  What type of extension would you like to add?
        │  Built-in Extension
        │
        ◇  Which built-in extension would you like to enable?
        │  Computer Controller
        │
        ◇  Please set the timeout for this tool (in secs):
        │  300
        │
        └  Enabled computercontroller extension
        ```
        4. Now that Goose has browser capabilities, let's resume your last session:
        ```sh
         goose session -r
        ```
        5. Ask Goose to launch your game in a browser:
    </TabItem>
</Tabs>

```
open the tic-tac-toe game in a browser
```

Go ahead and play your game, I know you want to 😂 ... good luck!


## Next Steps
Congrats, you've successfully used Goose to develop a web app! 🎉

Here are some ideas for next steps:
* Continue your session with Goose and improve your game (styling, functionality, etc).
* Browse other available [extensions](/extensions) and install more to enhance Goose's functionality even further.
* Provide Goose with a [set of hints](/docs/guides/using-goosehints) to use within your sessions.

