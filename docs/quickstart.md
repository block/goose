# Goose in 5 minutes

## Quickstart guide

Goose is a developer agent that supercharges your software development by automating an array of coding tasks directly within your terminal or IDE. You can get it setup and running on your devices in only a few minutes.

### Installation

To install Goose, use `pipx`. First ensure [pipx][pipx] is installed:

``` sh
brew install pipx
pipx ensurepath
```

Then install Goose:

```sh
pipx install goose-ai
```

### Running Goose

#### Start a session

From your terminal, navigate to the directory you'd like to start from and run:

```sh
goose session start
```

#### Set up a provider
Goose works with your [preferred LLM][providers]. By default, it uses `openai` as the LLM provider. You'll be prompted to set an [API key][openai-key] if you haven't set one previously.

>[!TIP]
> **Billing:**
>
> You will need to have credits in your LLM Provider account to be able to successfully make requests.
>


#### Make Goose do the work for you
You will see the Goose prompt `G❯`:

```
G❯ type your instructions here exactly as you would speak to a developer.
```

e.g

```
G❯ Create a JavaScript project that fetches and displays weather for a user specified city using a public API
```

Now you are interacting with Goose in conversational sessions. Think of it like you're giving directions to a junior developer. The default toolkit allows Goose to take actions through shell commands and file edits. You can interrupt Goose with `CTRL+D` or `ESC+Enter` at any time to help redirect its efforts.

#### Exit the session

If you are looking to exit, use `CTRL+D`.

#### Resume a session

When you exit a session, it will save the history in `~/.config/goose/sessions` directory. You can then resume your last saved session later, using:

``` sh
goose session resume
```

To see more documentation on the available CLI commands, check out the [documentation][cli]. If you’d like to develop your own CLI commands for Goose, check out the [Contributing document][contributing].

### Running a Goose task

You can run Goose to do things just as a one off, such as tidying up, and then exiting:

```sh
goose run instructions.md
```

You can also use process substitution to provide instructions directly from the command line:

```sh
goose run <(echo "Create a new Python file that prints hello world")
```

This will run until completion as best it can. You can also pass `--resume-session` and it will re-use the first session it finds for context.

## Additional tips

You can place a `.goosehints` file in `~/.config/goose/.goosehints` for hints personal to you. Goose will automatically load these within your sessions.

### Next steps

Review the [Getting Started Guide][getting-started] to learn how to modify the Goose `profiles.yaml` file to add and remove functionality (toolkits) and provide additional context to get the most out of Goose.

**Want to move out of the terminal and into an IDE?**

We have some experimental IDE integrations for VSCode and JetBrains IDEs:
* https://github.com/square/goose-vscode
* https://github.com/Kvadratni/goose-intellij

**Goose as a Github Action**

There is also an experimental Github action to run Goose as part of your workflow (e.g., if you ask it to fix an issue):
https://github.com/marketplace/actions/goose-ai-developer-agent

**With Docker**

There is also a `Dockerfile` in the root of this project you can use if you want to run goose in a sandboxed fashion.



[pipx]: https://github.com/pypa/pipx?tab=readme-ov-file#install-pipx
[openai-key]: https://platform.openai.com/api-keys
[getting-started]: https://block.github.io/goose/guidance/getting-started.html
[providers]: https://block.github.io/goose/providers.html