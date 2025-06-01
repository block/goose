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

<Tabs>
  <TabItem value="ui" label="Goose Desktop" default>
   :::warning
   You cannot create a recipe from an existing recipe session - the "Make Agent from this session" option will be disabled.
   :::

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
   :::warning
   You cannot create a recipe from an existing recipe session - the `/recipe` command will not work.
   :::

   ### Create a Recipe File

   Recipe files can be either JSON (.json) or YAML (.yaml) files. While in a [session](/docs/guides/managing-goose-sessions#start-session), run this command to generate a recipe.yaml file in your current directory:

   ```sh
   /recipe
   ```

   If you want to specify a different name, you can provide it as an argument:

   ```sh
   /recipe my-custom-recipe.yaml
   ```

   <details>
   <summary>Recipe file structure</summary>

   ```yaml
   # Required fields
   version: 1.0.0
   title: $title
   description: $description
   instructions: $instructions    # Define the model's behavior

   # Optional fields
   prompt: $prompt               # Initial message to start with
   extensions:                   # Tools the recipe needs
   - $extensions
   activities:                   # Example prompts to display in the Desktop app
   - $activities
   ```
   </details>

   ### Edit Recipe File

   You can edit any field to add customizations.

   #### Key Recipe Components
   A recipe needs at least one of these core components:

   - **Instructions**: Define the agent's behavior and capabilities
      - Acts as the agent's mission statement
      - Makes the agent ready for any relevant task
      - Required if no prompt is provided

   - **Prompt** (Optional): Starts the conversation automatically
      - Without a prompt, the agent waits for user input
      - Useful for specific, immediate tasks
      - Required if no instructions are provided

   - **Activities**: Example tasks that appear as clickable bubbles
      - Help users understand what the recipe can do
      - Make it easy to get started

   ### Optional Parameters

   You may add parameters to a recipe, which will require users to fill in data when running the recipe. Parameters can be added to any part of the recipe (instructions, prompt, activities, etc).

   To use parameters:
   1. Add template variables using `{{ variable_name }}` syntax in your recipe content
   2. Define each parameter in the `parameters` section of your YAML file

   <details>
   <summary>Example recipe with parameters</summary>

   ```yaml
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

   ### Validate Recipe

   [Exit the session](/docs/guides/managing-goose-sessions#exit-session) and run:

   ```sh
   goose recipe validate recipe.yaml
   ```

Validation ensures that:
   - All required fields are present
   - Parameters are properly formatted
   - Referenced extensions exist and are valid
   - The YAML/JSON syntax is correct

   ### Share Your Recipe

   Now that your recipe is created, you can share it with CLI users by directly sending them the recipe file or converting it to a shareable deep link for Desktop users:

   ```sh
   goose recipe deeplink recipe.yaml
   ```

   </TabItem> 
</Tabs>


## Use Recipe

<Tabs>
  <TabItem value="ui" label="Goose Desktop" default>

   There are two ways to use a recipe in Goose Desktop:

   1. **Direct Link**
      - Click a recipe link shared with you
      - The recipe will automatically open in Goose Desktop

   2. **Manual URL Entry**
      - Copy a recipe URL
      - Paste it into your browser's address bar
      - You will see a prompt to "Open Goose"
      - Goose Desktop will open with the recipe
  </TabItem>

  <TabItem value="cli" label="Goose CLI">

   ### Configure Recipe Location

  Recipes can be stored locally on your device or in a GitHub repository. Configure your recipe repository using either the `goose configure` command or config file:

   <Tabs>
     <TabItem value="configure" label="Using goose configure" default>

       Run the configure command:
       ```sh
       goose configure
       ```

       You'll see the following prompts:

       ```sh
       ┌  goose-configure 
       │
       ◆  What would you like to configure?
       │  ○ Configure Providers 
       │  ○ Add Extension 
       │  ○ Toggle Extensions 
       │  ○ Remove Extension 
       // highlight-start
       │  ● Goose Settings (Set the Goose Mode, Tool Output, Tool Permissions, Experiment, Goose recipe github repo and more)
       // highlight-end
       │
       ◇  What would you like to configure?
       │  Goose Settings 
       │
       ◆  What setting would you like to configure?
       │  ○ Goose Mode 
       │  ○ Tool Permission 
       │  ○ Tool Output 
       │  ○ Toggle Experiment 
       // highlight-start
       │  ● Goose recipe github repo (Goose will pull recipes from this repo if not found locally.)
       // highlight-end
       └  
       ┌  goose-configure 
       │
       ◇  What would you like to configure?
       │  Goose Settings 
       │
       ◇  What setting would you like to configure?
       │  Goose recipe github repo 
       │
       ◆  Enter your Goose Recipe Github repo (owner/repo): eg: my_org/goose-recipes
       // highlight-start
       │  squareup/goose-recipes (default)
       // highlight-end
       └  
       ```

     </TabItem>

     <TabItem value="config" label="Using config file">

       Add to your config file:
       ```yaml title="~/.config/goose/config.yaml"
       GOOSE_RECIPE_GITHUB_REPO: "owner/repo"
       ```

     </TabItem>
   </Tabs>

   ### Run a Recipe

   <Tabs>
     <TabItem value="local" label="Local Recipe" default>

       **Basic Usage** - Run once and exit:
       ```sh
       # Using recipe file in current directory
       goose run --recipe recipe.yaml

       # Using full path
       goose run --recipe ./recipes/my-recipe.yaml
       ```

       **Interactive Mode** - Start an interactive session:
       ```sh
       goose run --recipe recipe.yaml --interactive
       ```
       When running interactively, you'll be prompted for any required parameters:
       ```sh
       ◆ Enter value for required parameter 'language':
       │ Python
       │
       ◆ Enter value for required parameter 'style_guide':
       │ PEP8
       ```

       **With Parameters** - Supply values directly:
       ```sh
       # All parameters provided - runs without prompts
       goose run --recipe recipe.yaml \
         --params language=Python \
         --params style=PEP8

       # Missing required parameters - will fail in non-interactive mode
       goose run --recipe recipe.yaml \
         --params language=Python

       # Using default values - recipe.yaml contains: style_guide: "PEP8"
       goose run --recipe recipe.yaml \
         --params language=Python
       # style_guide will use "PEP8" from recipe defaults
       ```

     </TabItem>

     <TabItem value="github" label="GitHub Recipe">

       Once you've configured your GitHub repository, you can run recipes by name:

       **Basic Usage**:
       ```sh
       # This will look for <recipe-name>/recipe.yaml (or .json) in your configured repo
       goose run --recipe recipe-name
       ```

       For example, if your repository structure is:
       ```
       my-repo/
       ├── code-review/
       │   └── recipe.yaml
       └── setup-project/
           └── recipe.yaml
       ```

       **Simple Run** - Execute recipe and exit:
       ```sh
       goose run --recipe code-review
       ```

       **Interactive Mode** - With parameter prompts:
       ```sh
       goose run --recipe code-review --interactive
       ```
       The interactive mode will prompt for required values:
       ```sh
       ◆ Enter value for required parameter 'project_name':
       │ MyProject
       │
       ◆ Enter value for required parameter 'language':
       │ Python
       ```

       **With Parameters** - Supply values directly:
       ```sh
       # All parameters provided
       goose run --recipe code-review \
         --params project_name=MyProject \
         --params language=Python \
         --params test_coverage=80

       # Using default values - recipe defines default test_coverage: 90
       goose run --recipe code-review \
         --params project_name=MyProject \
         --params language=Python
       # test_coverage will use 90 from recipe defaults
       ```

       :::tip Repository Structure
       - Each recipe should be in its own directory
       - Directory name matches the recipe name you use in commands
       - Recipe file can be either recipe.yaml or recipe.json
       :::

     </TabItem>
   </Tabs>

   </TabItem>
</Tabs>

:::note Privacy & Isolation
- Each person gets their own private session
- No data is shared between users
- Your session won't affect the original recipe creator's session
:::

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