---
sidebar_position: 2
---

# Updating Goose

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { IconDownload } from "@site/src/components/icons/download";
import Link from "@docusaurus/Link";

:::info
To update Goose to the latest stable version, reinstall using the instructions below
:::

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    You can update Goose by simply running:

    ```sh
    goose update
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
        <div style={{ marginTop: '1rem' }}>
          1. To update Goose Desktop, click the button below:
            <div className="pill-button">
              <Link
                className="button button--primary button--lg"
                to="https://github.com/block/goose/releases/download/stable/Goose.zip"
              >
                <IconDownload />
                download goose desktop for macOS
              </Link>
            </div>
          2. Unzip the downloaded `Goose.zip` file.
          3. Run the executable file to launch the Goose Desktop application.
          4. Overwrite the existing Goose application with the new version.
          5. Run the executable file to launch the Goose desktop application.
        </div>
  </TabItem>
</Tabs>

All configuration settings will remain the same, with Goose updated to the latest version.
