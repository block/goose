---
sidebar_position: 2
title: Quickstart
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import Link from "@docusaurus/Link";
import { IconDownload } from "@site/src/components/icons/download";


# Goose in 5 minutes

:::info Supported Environments
Goose currently works only on **macOS** and **Linux** systems, and supports both **ARM** and **x86** architectures. If you'd like to request support for additional operating systems, please [open an issue on GitHub](https://github.com/block/goose/issues/new?template=Blank+issue).
:::

Goose is a developer AI agent that supercharges your software development by automating coding tasks. This Quickstart will guide you through getting started with Goose and covers using both the CLI and Desktop UI.


## Install Goose

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    #### Installing the Goose CLI
    To install Goose, run the following script on macOS or Linux. 

    ```sh
    curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | sh
    ```
    This script will fetch the latest version of Goose and set it up on your system.
  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
    #### Installing the Goose Desktop Application
    To install Goose, click the **button** below:
    <div className="pill-button">
      <Link
        className="button button--primary button--lg"
        to="https://github.com/block/goose/releases/download/stable/Goose.zip"
      >
        <IconDownload />
        download goose desktop
      </Link>
    </div>
    <div style={{ marginTop: '1rem' }}>  
      1. Unzip the downloaded `Goose.zip` file.
      2. Run the executable file to launch the Goose desktop application.
    </div>  
  </TabItem>
</Tabs>

## Configure Provider

Goose works with [supported LLM providers][providers]. When you first run Goose, you'll be prompted to supply an API key from your preferred LLM provider.

The process will look similar to the example below:

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    ![Set Up a Provider](./assets/guides/set-up-provider.png)
  </TabItem>
  <TabItem value="ui" label="Goose Desktop">
    ![Set Up a Provider UI](./assets/guides/set-up-provider-ui.png)
  </TabItem>
</Tabs>

:::info Billing
 You will need to have credits in your LLM Provider account (when necessary) to be able to successfully make requests. Some providers also have rate limits on API usage, which can affect your experience. Check out our [Handling Rate Limits][handling-rate-limits] guide to learn how to efficiently manage these limits while using Goose.
:::

## Start Session
<Tabs groupId="interface">
    <TabItem value="cli" label="Goose CLI" default>
        From your terminal, navigate to the directory from which you'd like to start, and run:
        ```sh
        goose session 
        ```
    </TabItem>
    <TabItem value="ui" label="Goose Desktop">
        After choosing an LLM provider, you’ll see the session interface ready for use.
        
        Type your questions, tasks, or instructions directly into the input field, and Goose will immediately get to work. 

        ![Install Extension](./assets/guides/ui-session-interface.png)
    </TabItem>
</Tabs>

## Write Prompt

You will see the Goose prompt `( O)>`. From here, you can interact with Goose in conversational sessions. Think of it as you're giving directions to a junior developer. 

```
( O)> type your instructions here exactly as you would speak to a developer.
```

Here's an example:

```
Create a JavaScript project that fetches and displays weather for a user-specified city using a public API
```

You can interrupt Goose with `CTRL+C` while it is running to help redirect its efforts.

## Next Steps

* Install [Extensions][extensions-guide] to enhance Goose's functionality.
* Provide Goose with a [set of hints](/docs/guides/using-goosehints) to use within your sessions.




[handling-rate-limits]: /docs/guides/handling-llm-rate-limits-with-goose
[openai-key]: https://platform.openai.com/api-keys
[getting-started]: /docs/category/getting-started
[providers]: /docs/getting-started/providers
[managing-sessions]: /docs/guides/managing-goose-sessions
[contributing]: https://github.com/block/goose/blob/main/CONTRIBUTING.md
[quick-tips]: /docs/guides/tips
[extensions-guide]: /docs/getting-started/using-extensions
[cli]: /docs/guides/goose-cli-commands
[MCP]: https://www.anthropic.com/news/model-context-protocol
