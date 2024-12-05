# Installation

To install Goose, use `pipx` on macOS, Linux, or Windows. 

First, ensure [pipx][pipx] is installed:

```sh
brew install pipx
pipx ensurepath
```

Then install Goose:

```sh
pipx install goose-ai
```

[pipx]: https://github.com/pypa/pipx?tab=readme-ov-file#install-pipx

### Configuration

#### Set up a provider
Goose works with a set of [supported LLM providers][providers] that you can obtain an API key from if you don't already have one. You'll be prompted to set an API key if you haven't set one previously when you run Goose.

>[!TIP]
> **Billing:**
>
> You will need to have credits in your LLM Provider account (when necessary) to be able to successfully make requests.
>

#### Profiles

After installation, you can configure Goose anytime by editing your profile file located at `~/.config/goose/profiles.yaml`. You can set multiple profile configurations, use different LLM providers, and enable toolkits that customize Goose's functionality as well:

```yaml
default:
  provider: openai
  processor: gpt-4o
  accelerator: gpt-4o-mini
  moderator: passive
  toolkits:
    - name: developer
      requires: {}
```


## Running Goose

You can run `goose` from the command line using:

```sh
goose session start
```


## Additional Resources

Visit the [Configuration Guide][configuration-guide] for detailed instructions on configuring Goose.

[configuration-guide]: https://block.github.io/goose/configuration.html
[providers]: https://block.github.io/goose/plugins/providers.html