---
sidebar_position: 2
title: Quickstart
---
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';


# Goose in 5 minutes

:::info Supported Environments
Goose currently works only on **OSX** and **Linux** systems, and supports both **ARM** and **x86** architectures. If you'd like to request support for additional operating systems, please [open an issue on GitHub](https://github.com/block/goose/issues/new?template=Blank+issue) to let us know.
:::

## Quickstart

Goose is a developer AI agent that supercharges your software development by automating coding tasks. This Quickstart will guide you through getting started with Goose and covers using both the CLI and Desktop UI.


### Installation

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    #### Installing the Goose CLI
    To install Goose, run the following script on macOS or Linux. 

    ```sh
    curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | sh
    ```
    This script will fetch the latest version of Goose and set it up on your system.
  </TabItem>
  <TabItem value="ui" label="Goose UI">
    #### Installing the Goose Desktop Application
    To install Goose, click the **button** below:
      <Button 
        label=":arrow_down: Download Goose Desktop" 
        link="https://github.com/block/goose/releases/download/stable/Goose.zip" 
        variant="secondary" 
        size="lg" 
        outline 
      />
    <div style={{ marginTop: '1rem' }}>  
      1. Unzip the downloaded `Goose.zip` file.
      2. Run the executable file to launch the Goose desktop application.
    </div>  
  </TabItem>
</Tabs>

### Running Goose

#### Set up a provider
Goose works with [supported LLM providers][providers]. When you first run Goose, you'll be prompted to supply an API key from your preferred LLM provider.

The process will look similar to the example below:

<Tabs groupId="interface">
  <TabItem value="cli" label="Goose CLI" default>
    ![Set Up a Provider](./assets/guides/set-up-provider.png)
  </TabItem>
  <TabItem value="ui" label="Goose UI">
    ![Set Up a Provider UI](./assets/guides/set-up-provider-ui.png)
  </TabItem>
</Tabs>

:::info Billing
 You will need to have credits in your LLM Provider account (when necessary) to be able to successfully make requests. Some providers also have rate limits on API usage, which can affect your experience. Check out our [Handling Rate Limits][handling-rate-limits] guide to learn how to efficiently manage these limits while using Goose.
:::

#### Start a session
<Tabs groupId="interface">
    <TabItem value="cli" label="Goose CLI" default>
        From your terminal, navigate to the directory from which you'd like to start, and run:
        ```sh
        goose session 
        ```
    </TabItem>
    <TabItem value="ui" label="Goose UI">
        After choosing an LLM provider, you’ll see the session interface ready for use.
        
        Type your questions, tasks, or instructions directly into the input field, and Goose will immediately get to work. 

        ![Install Extension](./assets/guides/ui-session-interface.png)
    </TabItem>
</Tabs>

#### Make Goose do the work for you

You will see the Goose prompt `( O)>`. From here, you can interact with Goose in conversational sessions. Think of it as you're giving directions to a junior developer. 

```
( O)> type your instructions here exactly as you would speak to a developer.
```

Here's an example:

```
( O)> Create a JavaScript project that fetches and displays weather for a user specified city using a public API
```

You can interrupt Goose with `CTRL+C` while it is running to help redirect its efforts.

#### Exit the session

To end a session, use `CTRL+D` or enter `/exit`.

#### Resume a session

When you exit a session, it will save the history in the  `~/.config/goose/sessions` directory. You can later resume your last saved session by using:

``` sh
goose session --resume
```

Check out [Managing Goose sessions][managing-sessions] to learn more about working with sessions in Goose.


Be sure to check out the available [CLI commands][cli]. If you’d like to develop your own CLI commands for Goose, check out the [Contributing guide][contributing].


### Running a Goose task

As an alternative to the chat interface, you can also provide instructions to Goose via files. In this example, Goose will execute the commands that are specified in `instructions.md`:

```sh
goose run -t "Create a new Python file that prints hello world" instructions.md
```

You can also pass in a file full of instructions, or use process substitution to chain more complex commands:

```sh
goose run -t instructions.md
goose run -t <(echo "Create a new Python file that prints hello world")
```

This will run until completion as best it can. If you'd like to take the run and turn it into an interactive session,
you can use `goose session --resume` to pick up where it left off.

### Extending Goose Functionality

[Goose Extensions][extensions-guide] are add-ons built on the [Model Context Protocol(MCP)][MCP]. They enhance Goose's functionality by integrating with the applications and tools you already use in your workflow. Extensions can be used to add new features, access data, and integrate with other systems.

For more information on how to add or remove extensions, see [Managing Extensions][extensions-guide].

## Additional tips

You can provide Goose with a set of hints that it will automatically use in every session with you. To do so, create a file  named `.goosehints` and save it in `~/.config/goose/.goosehints`. For additional tips to enhance your experience, check out [Quick Tips][quick-tips].



[handling-rate-limits]: /docs/guides/handling-llm-rate-limits-with-goose
[openai-key]: https://platform.openai.com/api-keys
[getting-started]: /docs/category/getting-started
[providers]: /docs/configuration/providers
[managing-sessions]: /docs/guides/managing-goose-sessions
[contributing]: https://github.com/block/goose/blob/main/CONTRIBUTING.md
[quick-tips]: /docs/guides/tips
[extensions-guide]: /docs/configuration/managing-extensions
[cli]: /docs/guides/goose-cli-commands
[MCP]: https://www.anthropic.com/news/model-context-protocol
