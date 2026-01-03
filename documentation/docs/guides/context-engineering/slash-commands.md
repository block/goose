---
sidebar_position: 4
title: Custom Slash Commands
sidebar_title: Slash Commands
description: "Create custom shortcuts to quickly apply reusable instructions in any goose chat session"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Terminal } from 'lucide-react';

Custom slash commands are shortcuts that let you instantly apply instructions in any goose chat session:

```
/daily-report
```

Custom slash commands save you from retyping common instructions by linking to your [recipes](/docs/guides/recipes). After creating a command, you can send it in a message to run the recipe.

## Create Slash Commands

Assign a custom command to a recipe.

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
   2. Click `Recipes` in the sidebar
   3. Find the recipe you want to use and click the <Terminal className="inline" size={16} /> button
   4. In the modal that pops up, type your custom command (without the leading `/`)
   5. Click `Save`
 
  The command appears in purple text under the recipe in your `Recipes` menu. For recipes that aren't in your Recipe Library, follow the `goose CLI` steps.

  </TabItem>
  <TabItem value="cli" label="goose CLI">

  Configure slash commands in your [configuration file](/docs/guides/config-files). List the command (without the leading `/`) along with the path to the recipe file on your computer:

```yaml title="~/.config/goose/config.yaml"
slash_commands:
  - command: "run-tests"
    recipe_path: "/path/to/recipe.yaml"
  - command: "daily-report"
    recipe_path: "/Users/me/.local/share/goose/recipes/report.yaml"
```

   </TabItem>
</Tabs>

## Use Slash Commands

In any chat session, type your custom command with a leading slash at the start of your message:

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>

```
/run-tests
```

:::tip Available Commands
Typing `/` in goose Desktop shows a popup menu with the available slash commands.
:::

  </TabItem>
  <TabItem value="cli" label="goose CLI">

```sh
Context: ●○○○○○○○○○ 5% (9695/200000 tokens)
( O)> /run-tests
```

  </TabItem>
</Tabs>

You can also pass one parameter after the command (if needed):

```
/deploy service-name
```

When you run a recipe using a slash command, the recipe's instructions and prompt fields are sent to your model and loaded into the conversation, but not displayed in chat. The model responds using the recipe's context and instructions just as if you opened it directly.

## Limitations

- Slash commands accept only one parameter. Any additional parameters in the recipe must have default values.
- Command names are case-insensitive (`/Bug` and `/bug` are treated as the same command).
- Command names must be unique and contain no spaces.
- You cannot use names that conflict with [built-in CLI slash commands](/docs/guides/goose-cli-commands#slash-commands) like `/recipe`, `/compact`, or `/help`.
- If the recipe file is missing or invalid, the command will be treated as regular text sent to the model.
