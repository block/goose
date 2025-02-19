# Updating Goose

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';


This guide explains how to keep your Goose installation up to date with the latest features and improvements.

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    You can update Goose by running the [installation](/docs/getting-started/installation) script again:

    ```sh
    curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash
    ```

    To check your current Goose version, use the following command:

    ```sh
    goose --version
    ```

  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
    To update Goose Desktop:
    - Download the latest version of Goose from the [releases page](https://github.com/block/goose/releases/download/stable/Goose.zip)
    - Unzip the downloaded `Goose.zip` file.
    - Overwrite the existing Goose application with the new version.
    - Run the executable file to launch the Goose desktop application.
    
  </TabItem>
</Tabs>

All configuration settings will remain the same, with Goose updated to the latest version.

