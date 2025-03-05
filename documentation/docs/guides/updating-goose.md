---
sidebar_position: 2
---

# Updating Goose

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import DesktopInstallButtons from '@site/src/components/DesktopInstallButtons';

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
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
  <TabItem value="ui" label="Goose Desktop">
        :::info
        To update Goose to the latest stable version, reinstall using the instructions below
        :::
        <div style={{ marginTop: '1rem' }}>
          1. <DesktopInstallButtons/>
          2. Unzip the downloaded zip file.
          3. Run the executable file to launch the Goose Desktop application.
          4. Overwrite the existing Goose application with the new version.
          5. Run the executable file to launch the Goose desktop application.
        </div>
  </TabItem>
</Tabs>