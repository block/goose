---
sidebar_position: 2
title: Updating Goose
sidebar_label: Updating Goose
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import MacDesktopInstallButtons from '@site/src/components/MacDesktopInstallButtons';
import WindowsDesktopInstallButtons from '@site/src/components/WindowsDesktopInstallButtons';
import LinuxDesktopInstallButtons from '@site/src/components/LinuxDesktopInstallButtons';

The Goose CLI and desktop apps are under active and continuous development. To get the newest features and fixes, you should periodically update your Goose client using the following instructions.

<Tabs>
  <TabItem value="mac" label="macOS" default>
    <Tabs groupId="interface">
      <TabItem value="ui" label="Goose Desktop" default>
        :::info
        To update Goose to the latest stable version, reinstall using the instructions below
        :::
        <div style={{ marginTop: '1rem' }}>
          1. <MacDesktopInstallButtons/>
          2. Unzip the downloaded zip file.
          3. Run the executable file to launch the Goose Desktop application.
          4. Overwrite the existing Goose application with the new version.
          5. Run the executable file to launch the Goose desktop application.
        </div>
      </TabItem>
      <TabItem value="cli" label="Goose CLI">
        You can update Goose by running:

        ```sh
        goose update
        ```

        Additional [options](/docs/guides/goose-cli-commands#update-options):
        
        ```sh
        # Update to latest canary (development) version
        goose update --canary

        # Update and reconfigure settings
        goose update --reconfigure
        ```

        Or you can run the [installation](/docs/getting-started/installation) script again:

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        To check your current Goose version, use the following command:

        ```sh
        goose --version
        ```
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="linux" label="Linux">
    <Tabs groupId="interface">
      <TabItem value="ui" label="Goose Desktop" default>
        :::info
        To update Goose to the latest stable version, reinstall using the instructions below
        :::
        <div style={{ marginTop: '1rem' }}>
          1. <LinuxDesktopInstallButtons/>
          
          **For Debian/Ubuntu-based distributions:**
          2. Download the DEB file
          3. Navigate to the directory where it is saved in a terminal
          4. Run `sudo dpkg -i (filename).deb`
          5. Launch Goose from the app menu

        </div>
      </TabItem>
      <TabItem value="cli" label="Goose CLI">
        You can update Goose by running:

        ```sh
        goose update
        ```

        Additional [options](/docs/guides/goose-cli-commands#update-options):
        
        ```sh
        # Update to latest canary (development) version
        goose update --canary

        # Update and reconfigure settings
        goose update --reconfigure
        ```

        Or you can run the [installation](/docs/getting-started/installation) script again:

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        To check your current Goose version, use the following command:

        ```sh
        goose --version
        ```
      </TabItem>
    </Tabs>
  </TabItem>

  <TabItem value="windows" label="Windows">
    <Tabs groupId="interface">
      <TabItem value="ui" label="Goose Desktop" default>
        :::info
        To update Goose to the latest stable version, reinstall using the instructions below
        :::
        <div style={{ marginTop: '1rem' }}>
          1. <WindowsDesktopInstallButtons/>
          2. Unzip the downloaded zip file.
          3. Run the executable file to launch the Goose Desktop application.
          4. Overwrite the existing Goose application with the new version.
          5. Run the executable file to launch the Goose Desktop application.
        </div>
      </TabItem>
      <TabItem value="cli" label="Goose CLI">
        There isn't native CLI support for Windows. You can run Goose CLI using WSL (Windows Subsystem for Linux).

        **If you haven't set up WSL yet:**

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

        **If you already have Goose CLI installed in WSL, you can update it:**

        ```sh
        goose update
        ```

        Additional [options](/docs/guides/goose-cli-commands#update-options):
        
        ```sh
        # Update to latest canary (development) version
        goose update --canary

        # Update and reconfigure settings
        goose update --reconfigure
        ```

        Or you can run the [installation](/docs/getting-started/installation) script again within WSL:

        ```sh
        curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
        ```

        To check your current Goose version within WSL, use:

        ```sh
        goose --version
        ```
      </TabItem>
    </Tabs>
  </TabItem>
</Tabs>