# Configuring Goose

## Profiles

If you need to customize goose, one way is via editing: `~/.config/goose/profiles.yaml`.

By default, it looks like this:

```yaml
default:
  provider: open-ai
  processor: gpt-4o
  accelerator: gpt-4o-mini
  moderator: passive
  toolkits:
    - name: developer
      requires: {}
```

If you run `goose session start` without the `--profile` flag it will use the `default` profile automatically.

### Fields

#### provider

`provider` specifies the chosen LLM provider by the user. You can set up multiple profiles with different providers. Goose will use the provider specified in the profile to interact with the LLM. Here is the list of [supported LLM providers][providers]


#### processor

This is the model used for the main Goose loop and main tools -- it should be be capable of complex, multi-step tasks such as writing code and executing commands. Example: `gpt-4o`. You should choose the model based the provider you configured.

#### accelerator

Small model for fast, lightweight tasks. Example: `gpt-4o-mini`. You should choose the model based the provider you configured.

#### moderator

Rules designed to control or manage the output of the model. Moderators that currently are supported by Goose:

- `passive`: does not actively intervene in every response
- `truncate`: truncates the first contexts when the contexts exceed the max token size
- `synopsis`: instead of truncating, it uses LLMs to summarize and condense context dynamically, keeping relevant information while staying under the token limit.

> **Important:** `synopsis` only works when the `synopsis` toolkit is enabled. Be sure to update your [`profile.yml` configurations](https://block.github.io/goose/guidance/getting-started.html#configuring-goose-with-the-profilesyaml-file) to enable both.


#### toolkits
These are modular add-ons that enhance the functionality of Goose. Each toolkit provides specific capabilities or integrations that can be tailored to meet particular needs or use cases e.g `browser`, `developer`, `screen` etc. 

To list available toolkits, use the following command:

```
  goose toolkit list
```


## Adding a toolkit
To make a toolkit available to Goose, add it to your project's pyproject.toml. For example in the Goose pyproject.toml file:
```
[project.entry-points."goose.toolkit"]
developer = "goose.toolkit.developer:Developer"
github = "goose.toolkit.github:Github"
# Add a line like this - the key becomes the name used in profiles
my-new-toolkit = "goose.toolkit.my_toolkits:MyNewToolkit"  # this is the path to the class that implements the toolkit
```

Then to set up a profile that uses it, add something to `~/.config/goose/profiles.yaml`:
```yaml
my-profile:
  provider: openai
  processor: gpt-4o
  accelerator: gpt-4o-mini
  moderator: passive
  toolkits:  # new toolkit gets added here
    - developer
    - my-new-toolkit
```

And now you can run Goose with this new profile to use the new toolkit!

```sh
goose session start --profile my-profile
```

Or, if you're developing a new toolkit and want to test it:
```sh
uv run goose session start --profile my-profile
```


## Tuning Goose to your repo

Goose ships with the ability to read in the contents of a file named `.goosehints` from your repo. If you find yourself repeating the same information across sessions to Goose, this file is the right place to add this information.

This file will be read into the Goose system prompt if it is present in the current working directory.

Check out the [guide on using .goosehints][using-goosehints] for more tips.

> [!NOTE]
> `.goosehints` follows [jinja templating rules][jinja-guide] in case you want to leverage templating to insert file contents or variables.


[providers]: https://block.github.io/goose/plugins/providers.html
[jinja-guide]: https://jinja.palletsprojects.com/en/3.1.x/
[using-goosehints]: https://block.github.com/goose/guidance/using-goosehints.html
