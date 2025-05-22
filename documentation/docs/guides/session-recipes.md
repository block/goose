---
sidebar_position: 5
title: Create a Recipe from Your Session
sidebar_label: Shareable Recipes
description: "Share a Goose session setup (including tools, goals, and instructions) as a reusable recipe that others can launch with a single click"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

Sometimes you finish a task in Goose and realize, "Hey, this setup could be useful again." Maybe you have curated a great combination of tools, defined a clear goal, and want to preserve that flow. Or maybe you're trying to help someone else replicate what you just did without walking them through it step by step. 

You can turn your current Goose session into a reusable recipe that includes the tools, goals, and setup you're using right now and package it into a new Agent that others (or future you) can launch with a single click.

## Create Recipe

:::tip Heads Up
You'll need to provide both instructions and activities for your Recipe.

- **Instructions** provide the purpose. These get sent directly to the model and define how it behaves. Think of this as its internal mission statement. Make it clear, action-oriented, and scoped to the task at hand.

- **Activities** are specific, example prompts that appear as clickable bubbles on a fresh session. They help others understand how to use the Recipe.
:::

<Tabs>
  <TabItem value="ui" label="Goose Desktop" default>

   1. While in the session you want to save as a recipe, click the menu icon **⋮** in the top right corner  
   2. Select **Make Agent from this session**  
   3. In the dialog that appears:
      - Name the recipe
      - Provide a description
      - Some **activities** will be automatically generated. Add or remove as needed.
      - A set of **instructions** will also be automatically generated. Review and edit as needed. 
   4. Copy the Recipe URL and use it however you like (e.g., share it with teammates, drop it in documentation, or keep it for yourself)

  </TabItem>

  <TabItem value="cli" label="Goose CLI">

   While in a session, run the following command:

   ```sh
   /recipe
   ```

   This will generate a `recipe.yaml` file in your current directory.

   Alternatively, you can provide a custom filename:

   ```sh
   /recipe my-custom-recipe.yaml
   ```

   <details>
   <summary>recipe.yaml</summary>
   
   ```yaml
   # Required fields
   version: 1.0.0
   title: $title
   description: $description
   instructions: $instructions # instructions to be added to the system prompt

   # Optional fields
   prompt: $prompt             # if set, the initial prompt for the run/session
   extensions:
   - $extensions
   context:
   - $context
   activities:
   - $activities
   author:
   contact: $contact
   metadata: $metadata
   ```

   </details>

   You can then edit the recipe file to include the following key information:

   - `instructions`: Add or modify the system instructions
   - `prompt`: Add the initial message or question to start a Goose session with
   - `activities`: List the activities that can be performed


   #### Recipe Parameters
   
   You may add parameters to a recipe, which will require uses to fill in data when running the recipe. Parameters can be added to any part of the recipe (instructions, prompt, activities, etc).

   To add parameters, edit your recipe file to include template variables using `{{ variable_name }}` syntax and define each of them in your yaml using `parameters`.

   <details>
   <summary>Example recipe with parameters</summary>
      
   ```yaml title="code-review.yaml"
   version: 1.0.0
   title: "{{ project_name }} Code Review" # Wrap the value in quotes if it starts with template syntax to avoid YAML parsing errors
   description: Automated code review for {{ project_name }} with {{ language }} focus
   instructions: |
      You are a code reviewer specialized in {{ language }} development.
      Apply the following standards:
      - Complexity threshold: {{ complexity_threshold }}
      - Required test coverage: {{ test_coverage }}%
      - Style guide: {{ style_guide }}
   activities:
   - "Review {{ language }} code for complexity"
   - "Check test coverage against {{ test_coverage }}% requirement"
   - "Verify {{ style_guide }} compliance"
   parameters:
   - key: project_name
     input_type: string
     requirement: required # could be required, optional or user_prompt
     description: name of the project
   - key: language
     input_type: string
     requirement: required
     description: language of the code
   - key: complexity_threshold
     input_type: number
     requirement: optional
     default: 20 # default is required for optional parameters
     description: a threshold that defines the maximum allowed complexity
   - key: test_coverage
     input_type: number
     requirement: optional
     default: 80
     description: the minimum test coverage threshold in percentage
   - key: style_guide
     input_type: string
     description: style guide name
     requirement: user_prompt
     # If style_guide param value is not specified in the command, user will be prompted to provide a value, even in non-interactive mode
   ```

   </details>

   When someone runs a recipe that contains template parameters, they will need to provide the parameters:

   ```sh
   goose run --recipe code-review.yaml \
  --params project_name=MyApp \
  --params language=Python \
  --params complexity_threshold=15 \
  --params test_coverage=80 \
  --params style_guide=PEP8
  ```

   #### Validate the recipe
   
   [Exit the session](/docs/guides/managing-goose-sessions/#exit-session) and run:

   ```sh
   goose recipe validate recipe.yaml
   ```

   #### Share the recipe

   - To share with **CLI users**, send them the recipe yaml file
   - To share with **Desktop users**, run the following command to create a deep link:

   ```sh
   goose recipe deeplink recipe.yaml
   ```

   </TabItem> 
</Tabs>


## Use Recipe

<Tabs>
  <TabItem value="ui" label="Goose Desktop" default>

   To use a shared recipe, simply click the recipe link, or paste in a browser address bar. This will open Goose Desktop and start a new session with:

   - The recipe's defined instructions  
   - Suggested activities as clickable bubbles  
   - The same extensions and project context (if applicable)

   Each person using the recipe gets their own private session, so no data is shared between users, and nothing links back to your original session.

  </TabItem>

  <TabItem value="cli" label="Goose CLI">

   You can start a session with a recipe file in the following ways:

   - Run the recipe once and exit:

   ```sh
   goose run --recipe recipe.yaml
   ```

   - Run the recipe and enter interactive mode:

   ```sh
   goose run --recipe recipe.yaml --interactive
   ```

   - Run the recipe with parameters:

   ```sh
   goose run --recipe recipe.yaml --interactive --params language=Spanish --params style=formal --params name=Alice
   ```

   - Explain the recipe with description and parameters

   ```sh
   goose run --recipe recipe.yaml --explain
   ```

   #### Discover recipes
   When using recipe-related CLI commands, there are a few ways to specify which recipe to use:
   ##### Options 1: Provide the full file path
   Use the exact path to the recipe file:
      
   ```sh
   goose run --recipe ~/my_recipe.yaml
   goose recipe validate ~/my_recipe.yaml
   goose recipe deeplink ~/my_recipe.yaml
   ```
   ##### Option 2: Use the recipe name
   If your recipe is named my_recipe, you can simply use the name:

   ```sh
   goose run --recipe my_recipe
   goose recipe validate my_recipe
   goose recipe deeplink my_recipe
   ```
   When you use the recipe name, Goose will search for the file in the following order:
   1. Local search:
      Goose will search for `my_recipe.yaml` or `my_recipe.json` in the current working directory
      
   2. Remote search (GitHub):
      - If the `GOOSE_RECIPE_GITHUB_REPO` environment variable is set or configured in the `Goose Settings` via `goose configure`, Goose will search the specified GitHub repo. (eg: my_org/goose-recipes).
      - Goose will look for `my_recipe/recipe.yaml` or `my_recipe/recipe.json` within that GitHub repository.
   </TabItem> 
</Tabs>


## What's Included

A Recipe captures:

- AI instructions (goal/purpose)  
- Suggested activities (examples for the user to click)  
- Enabled extensions and their configurations  
- Project folder or file context  
- Initial setup (but not full conversation history)


To protect your privacy and system integrity, Goose excludes:

- Global and local memory  
- API keys and personal credentials  
- System-level Goose settings  


This means others may need to supply their own credentials or memory context if the Recipe depends on those elements.


## Example Use Cases

- 🔧 Share a debugging workflow with your team  
- 📦 Save a repeatable project setup  
- 📚 Onboard someone into a task without overwhelming them  


## Tips for Great Recipes

If you're sharing recipes with others, here are some tips:

- Be specific and clear in the instructions, so users know what the recipe is meant to do.
- Keep the activity list focused. Remove anything that's too specific or out of scope.
- Test the link yourself before sharing to make sure everything loads as expected.
- Mention any setup steps that users might need to complete (e.g., obtaining an API key).

## Troubleshooting

- You can't create a Recipe from an existing Recipe session. The menu option will be disabled  
- Make sure you're using the latest version of Goose if something isn't working  
- Remember that credentials, memory, and certain local setups won't carry over  
