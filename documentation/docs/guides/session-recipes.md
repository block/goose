---
sidebar_position: 5
title: Create a Recipe from Your Session
sidebar_label: Shareable Recipes
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
      - Edit the **instructions** to clarify its purpose. 
      - Add or remove **activities** as needed.
   4. Click **Save**  
   5. Copy the Recipe URL and use it however you like (e.g., share it with teammates, drop it in documentation, or keep it for yourself)

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
   - `activities`: List the activities that can be performed


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

   You can start a session with a recipe file in two ways:

   - Run the recipe once and exit:

   ```sh
   goose run --recipe recipe.yaml
   ```

   - Run the recipe and enter interactive mode:

   ```sh
   goose run --recipe recipe.yaml --interactive
   ```

   :::info
   Be sure to use the exact filename of the recipe.
   :::

   </TabItem> 
</Tabs>


### What's Included

A Recipe captures:

- AI instructions (goal/purpose)  
- Suggested activities (examples for the user to click)  
- Enabled extensions and their configurations  
- Project folder or file context  
- Initial setup (but not full conversation history)


### What's *Not* Included

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
