# Using Toolkits
[Toolkits](https://block.github.io/goose/plugins/plugins.html) in Goose are add-ons that expand its capabilities, offering tools and prompts for specific tasks. They make it easier to interact with external systems and handle complex operations. In this guide, we'll cover how to use toolkits included in `goose` and those available through the `goose-plugins` repository.

!!! important
    Before using toolkits, ensure Goose is installed and properly set up. If you haven’t installed Goose yet, follow the [Goose Installation Guide](https://block.github.io/goose/installation.html).    

## Listing Available Toolkits

To list available toolkits, use the following command within the Goose repo:

```
goose toolkit list
```

This will display a list of all toolkits available in your environment.

The output should look similar to the following: 

```yaml
Available toolkits:
 - browser: A toolkit for interacting with web browsers using Selenium.
 - github: Provides an additional prompt on how to interact with Github
 - jira: Provides an additional prompt on how to interact with Jira
 - reasoner: Deep thinking toolkit for reasoning through problems and solutions
 - repo_context: Provides context about the current repository
 - screen: Provides an instructions on when and how to work with screenshots
 - synopsis: Provides shell and file operation tools using OperatingSystem.
 - codesearch: Provides a way of searching through internal company code.
 - glean: Provides Goose with access to Glean, our AI search vendor.
 - java: Provides guidance on how to work in Java codebases
 - migrate-prefect: Enabled Goose to automate the Prefect 2 migration
```

The list above is limited. For a complete list, refer to the [Goose Available Toolkits Guide](https://block.github.io/goose/plugins/available-toolkits.html).

## Adding Toolkits to a Profile

To use a Toolkit within your Goose sessions, you'll need to add them to the `profiles.yaml` file, which can be found in your `User` directory at `~/.config/goose/profiles.yaml`. Here's how to add multiple toolkits:

```yaml title="profiles.yaml"
default:
  provider: openai
  processor: gpt-4o
  accelerator: gpt-4o-mini
  moderator: synopsis
  toolkits:
    - name: synopsis
      requires: {}
    - name: my_toolkit
      requires: {}
```

!!! important
    You always want to have the `synopsis` Toolkit, formerly known as the `developer` Toolkit enabled. It is essential for Goose to be able to create files for you, if this is removed it will greatly limit Goose's functionality. 


Additionally, use the `requires` field to specify dependencies between toolkits, and any necessary configurations. If there are no requirements, simply add an empty set of braces: `{}`. 

## Starting a Goose Session with Toolkits

Once your [profile](https://block.github.io/goose/guidance/getting-started.html#configuring-goose-with-the-profilesyaml-file) is set up, you can start a Goose session with the specified profile. You can configure multiple profiles in your profiles.yaml file to suit different needs. For example, you might have separate `default` and `work` profiles:

```yaml title="profiles.yaml"
default:
  provider: openai
  processor: gpt-4o
  accelerator: gpt-4o-mini
  moderator: synopsis
  toolkits:
  - name: synopsis
    requires: {}
  - name: screen
    requires: {}
work:
  provider: block
  processor: gpt-4o
  accelerator: gpt-4o-mini
  moderator: synopsis
  toolkits:
  - name: synopsis
    requires: {}
  - name: jira
    requires: {}
```

In the example above:
- The `default` profile uses the `synopsis`, and `screen` toolkits.
- The `work` profile is configured with the `synopsis` and `jira` toolkits.

To start a session with the `work` profile, run: 

```bash
goose session start --profile work
```
If your `profile.yaml` file contains only one profile, you can simply run: 

```bash
goose session start
```

## Using Toolkits from Goose Plugins

Goose comes with a default set of toolkits for essential functionality. For more specialized tasks and integrations, the `goose-plugins` repository provides optional toolkits that you can add as needed. This allows you to customize Goose to match your specific workflows without overloading the core installation.

To access these additional toolkits, follow these steps:

### Install `goose-plugins`:

Run the following command to install the `goose-plugins` package:

```bash
 pipx install goose-ai --preinstall goose-plugins
```

### List available toolkits:

Within the `goose-plugins` repo, you may need to [install `uv`](https://docs.astral.sh/uv/getting-started/installation/) first. 

```bash
uv run goose toolkit list
```

### Update the `profiles.yaml` File:
Add the desired Toolkit from the `goose-plugins` repository to your profile. For example:

```yaml title="profiles.yaml"
default:
  provider: openai
  processor: gpt-4o
  accelerator: gpt-4o-mini
  moderator: synopsis
  toolkits:
    - name: synopsis                                                                                                             
      requires: {}   
    - name: jira
      requires: {}
```

### Start the Goose Session:

```bash
goose session start --profile default
```




