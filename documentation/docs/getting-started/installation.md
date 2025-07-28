---
sidebar_position: 1
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import RateLimits from '@site/src/components/RateLimits';
import MacDesktopInstallButtons from '@site/src/components/MacDesktopInstallButtons';
import WindowsDesktopInstallButtons from '@site/src/components/WindowsDesktopInstallButtons';
import LinuxDesktopInstallButtons from '@site/src/components/LinuxDesktopInstallButtons';
import { PanelLeft } from 'lucide-react';

# Install Goose

<Tabs>
  <TabItem value="mac" label="macOS" default>
    Choose to install Goose on CLI and/or Desktop:

    <Tabs groupId="interface">
      <TabItem value="ui" label="Goose Desktop" default>
        Install Goose directly from the browser or with [Homebrew](https://brew.sh/).
        
        <h3 style={{ marginTop: '1rem' }}>Option 1: Install via Download</h3>
        <MacDesktopInstallButtons/>

        <div style={{ marginTop: '1rem' }}>
          1. Unzip the downloaded zip file.
          2. Run the executable file to launch the Goose Desktop application.

          :::tip Updating Goose
          It's best to keep Goose updated by periodically running the installation steps again.
          :::
        </div>
        <h3>Option 2: Install via Homebrew</h3>
        Homebrew downloads the [same app](https://github.com/Homebrew/homebrew-cask/blob/master/Casks/b/block-goose.rb) but can take care of updates too. 
        ```bash
          brew install --cask block-goose
        ```
        ---
        <div style={{ marginTop: '1rem' }}>
          :::note Permissions
          If you're on an Apple Mac M3 and the Goose Desktop app shows no window on launch, check and update the following:

          Ensure the `~/.config` directory has read and write access.

          Goose needs this access to create the log directory and file. Once permissions are granted, the app should load correctly. For steps on how to do this, refer to the  [Troubleshooting Guide](/docs/troubleshooting.md#macos-permission-issues)
          :::
        </div>
      </TabItem>
      <TabItem value="cli" label="Goose CLI">
        Install Goose directly from the browser or with [Homebrew](https://brew.sh/).

        <h3 style={{ marginTop: '1rem' }}>Option 1: Install via Download script</h3>
        Run the following command to install the latest version of Goose on macOS:

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```
        This script will fetch the latest version of Goose and set it up on your system.

        If you'd like to install without interactive configuration, disable `CONFIGURE`:

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        :::tip Updating Goose
        It's best to keep Goose updated. To update Goose, run:
        ```sh
        goose update
        ```
        :::

        <h3>Option 2: Install via Homebrew</h3>
        Homebrew downloads the [a precompiled CLI tool](https://github.com/Homebrew/homebrew-core/blob/master/Formula/b/block-goose-cli.rb) and can take care of updates.
        ```bash
        brew install block-goose-cli
        ```
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="linux" label="Linux" default>
    Choose to install Goose on CLI and/or Desktop:

    <Tabs groupId="interface">
      <TabItem value="ui" label="Goose Desktop" default>
        Install Goose Desktop directly from the browser.
        
        <h3 style={{ marginTop: '1rem' }}>Install via Download</h3>
        <LinuxDesktopInstallButtons/>

        <div style={{ marginTop: '1rem' }}>
          **For Debian/Ubuntu-based distributions:**
          1. Download the DEB file
          2. Navigate to the directory where it is saved in a terminal
          3. Run `sudo dpkg -i (filename).deb`
          4. Launch Goose from the app menu
          
          :::tip Updating Goose
          It's best to keep Goose updated by periodically running the installation steps again.
          :::
        </div>
      </TabItem>
      <TabItem value="cli" label="Goose CLI">
        Run the following command to install the Goose CLI on Linux:

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```
        This script will fetch the latest version of Goose and set it up on your system.

        If you'd like to install without interactive configuration, disable `CONFIGURE`:

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        :::tip Updating Goose
        It's best to keep Goose updated. To update Goose, run:
        ```sh
        goose update
        ```
        :::
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="windows" label="Windows">
    Choose to install Goose on CLI and/or Desktop:

    <Tabs groupId="interface">
      <TabItem value="ui" label="Goose Desktop" default>
        Install Goose Desktop directly from the browser.
        
        <h3 style={{ marginTop: '1rem' }}>Install via Download</h3>
        <WindowsDesktopInstallButtons/>

        <div style={{ marginTop: '1rem' }}>
          1. Unzip the downloaded zip file.
          2. Run the executable file to launch the Goose Desktop application.

          :::tip Updating Goose
          It's best to keep Goose updated by periodically running the installation steps again.
          :::
        </div>
      </TabItem>
      <TabItem value="cli" label="Goose CLI">
        Run the following command in **Git Bash**, **MSYS2**, or **PowerShell** to install the Goose CLI natively on Windows:

        ```bash
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
        ```
        This script will fetch the latest version of Goose and set it up on your system.

        If you'd like to install without interactive configuration, disable `CONFIGURE`:

        ```bash
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        :::note Prerequisites
        - **Git Bash** (recommended): Comes with [Git for Windows](https://git-scm.com/download/win)
        - **MSYS2**: Available from [msys2.org](https://www.msys2.org/)
        - **PowerShell**: Available on Windows 10/11 by default
        
        The script requires `curl` and `unzip` to be available in your environment.
        :::

        <details>
        <summary>Install via Windows Subsystem for Linux (WSL)</summary>

          We recommend running the Goose CLI natively on Windows, but you can use WSL if you prefer a Linux-like environment.

          1. Open [PowerShell](https://learn.microsoft.com/en-us/powershell/scripting/install/installing-powershell-on-windows) as Administrator and install WSL and the default Ubuntu distribution:

          ```bash
          wsl --install
          ```

          2. If prompted, restart your computer to complete the WSL installation. Once restarted, or if WSL is already installed, launch your Ubuntu shell by running:

          ```bash
          wsl -d Ubuntu
          ```

          3. Run the Goose installation script:
          ```bash
          curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash
          ```
          :::tip
            If you encounter any issues on download, you might need to install `bzip2` to extract the downloaded file:

            ```bash
            sudo apt update && sudo apt install bzip2 -y
            ```
          :::

          If you'd like to install without interactive configuration, disable `CONFIGURE`:

          ```sh
          curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
          ```  

        </details>
      </TabItem>
    </Tabs>
  </TabItem>
</Tabs>

## Set LLM Provider
Goose works with a set of [supported LLM providers][providers], and you'll need an API key to get started. When you use Goose for the first time, you'll be prompted to select a provider and enter your API key.

<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
    Upon installing, the Provider screen will appear. Here is where you can choose your LLM Provider.

    ![Set Up a Provider UI](../assets/guides/set-up-provider-ui.png)

    Once selecting your provider, you'll be prompted to enter an API key if applicable. Do so, and click `Submit`.
  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    Upon installing, Goose will automatically enter its configuration screen. Here is where you can set up your LLM provider.

    :::tip Windows Users
    When using the native Windows CLI, choose to not store to keyring when prompted during initial configuration.
    :::

    Example:

    ```
    ┌   goose-configure
    │
    ◇ What would you like to configure?
    │ Configure Providers
    │
    ◇ Which model provider should we use?
    │ OpenAI
    │
    ◇ Provider openai requires OPENAI_API_KEY, please enter a value
    │▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇ Enter a model from that provider:
    │ gpt-4o
    │
    ◇  Welcome aboard! You're all set to start using this agent—let's achieve great things together!
    │
    └  Configuration saved successfully
  ```

  :::info Windows Users
  On initial run, you may encounter errors about keyrings when setting your API Keys. Set the needed environment variables manually, e.g.:

  **For Native Windows CLI (Git Bash/MSYS2):**
  ```bash
  export OPENAI_API_KEY={your_api_key}
  ```

  **For WSL:**
  ```bash
  export OPENAI_API_KEY={your_api_key}
  ```

  Run `goose configure` again and proceed through the prompts. When you reach the step for entering the API key, Goose will detect that the key is already set as an environment variable and display a message like:

  ```
  ● OPENAI_API_KEY is set via environment variable
  ```

  **To make the changes persist across sessions:**

  **For Native Windows CLI (Git Bash):**
  Add the goose path and export commands to your `~/.bashrc` or `~/.bash_profile` file:
  ```bash
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
  echo 'export OPENAI_API_KEY=your_api_key' >> ~/.bashrc
  source ~/.bashrc
  ```

  **For WSL:**
  ```bash
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
  echo 'export OPENAI_API_KEY=your_api_key' >> ~/.bashrc
  source ~/.bashrc
  ```
  :::
  </TabItem>
</Tabs>

## Update Provider
<Tabs groupId="interface">
  <TabItem value="ui" label="Goose Desktop" default>
  **To update your LLM provider and API key:**

    1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar.
    2. Click the `Settings` button on the sidebar.
    3. Click the `Models` tab.
    4. Click `Configure Providers`
    5. Choose your provider
    6. Click `Configure`, enter your API key, and click `Submit`.

  </TabItem>
  <TabItem value="cli" label="Goose CLI">
    **To update your LLM provider and API key:**
    1. Run the following command:
    ```sh
    goose configure
    ```
    2. Select `Configure Providers` from the menu.
    3. Follow the prompts to choose your LLM provider and enter or update your API key.

    **Example:**

    To select an option during configuration, use the up and down arrows to highlight your choice then press Enter.

    ```
    ┌   goose-configure
    │
    ◇ What would you like to configure?
    │ Configure Providers
    │
    ◇ Which model provider should we use?
    │ Google Gemini
    │
    ◇ Provider Google Gemini requires GOOGLE_API_KEY, please enter a value
    │▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪
    │
    ◇ Enter a model from that provider:
    │ gemini-2.0-flash-exp
    │
    ◇  Hello there! You're all set to use me, so please ask away!
    │
    └  Configuration saved successfully
    ```
  </TabItem>
</Tabs>

<RateLimits />

## Running Goose

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
        Starting a session in the Goose Desktop is straightforward. After choosing your provider, you'll see the session interface ready for use.

        Type your questions, tasks, or instructions directly into the input field, and Goose will get to work immediately.
    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        From your terminal, navigate to the directory you'd like to start from and run:
        ```sh
        goose session
        ```
    </TabItem>
</Tabs>

## Shared Configuration Settings

The Goose CLI and Desktop UI share all core configurations, including LLM provider settings, model selection, and extension configurations. When you install or configure extensions in either interface, the settings are stored in a central location at `~/.config/goose/config.yaml`, making them available to both the Desktop application and CLI. This makes it convenient to switch between interfaces while maintaining consistent settings.

:::note
While core configurations are shared between interfaces, extensions have flexibility in how they store authentication credentials. Some extensions may use the shared config file while others implement their own storage methods.
::: 

<Tabs groupId="interface">
    <TabItem value="ui" label="Goose Desktop" default>
        Navigate to shared configurations through:
        1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar.
        2. Click the `Settings` button on the sidebar.
    </TabItem>
    <TabItem value="cli" label="Goose CLI">
        Use the following command to manage shared configurations:
        ```sh
        goose configure
        ```
    </TabItem>
</Tabs>

## Additional Resources

You can also configure Extensions to extend Goose's functionality, including adding new ones or toggling them on and off. For detailed instructions, visit the [Using Extensions Guide][using-extensions].

[using-extensions]: /docs/getting-started/using-extensions
[providers]: /docs/getting-started/providers
[handling-rate-limits]: /docs/guides/handling-llm-rate-limits-with-goose
[mcp]: https://www.anthropic.com/news/model-context-protocol