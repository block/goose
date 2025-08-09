---
title: Saving Recipes
sidebar_position: 4
sidebar_label: Saving Recipes
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import { PanelLeft, Bot } from 'lucide-react';

This guide covers storing, organizing, and finding Goose recipes when you need to access them again later. 

:::info Desktop UI vs CLI
- **Goose Desktop** has a visual Recipe Library for browsing and managing saved recipes
- **Goose CLI** stores recipes as files that you find using file paths or environment variables
:::

## Understanding Recipe Storage

Before saving recipes, it's important to understand where they can be stored and how this affects their availability.

### Recipe Storage Locations

| Type | Location | Availability | Best For |
|------|----------|-------------|----------|
| **Global** | `~/.config/goose/recipes/` | All projects and sessions | Personal workflows, general-purpose recipes |
| **Local** | `YOUR_WORKING_DIRECTORY/.goose/recipes/` | Only when working in that project | Project-specific workflows, team recipes |

**Choose Global Storage When:**
- You want the recipe available across all projects
- It's a personal workflow or general-purpose recipe
- You're the primary user of the recipe

**Choose Local Storage When:**
- The recipe is specific to a particular project
- You're working with a team and want to share the recipe
- The recipe depends on project-specific files or configurations


## Storing Recipes

<Tabs groupId="interface">
  <TabItem value="desktop" label="Goose Desktop" default>

**Save New Recipe:**

1. To create a recipe from your chat session, see: [Create Recipe](/docs/guides/recipes/session-recipes#create-recipe)
2. Once in the Recipe Editor, click **Save Recipe** to save it to your Recipe Library

**Save Modified Recipe:**

If you're already using a recipe and want to save a modified version:
1. Click the <Bot className="inline" size={16}/> button with your current model at the bottom of the window
2. Click **View Recipe**
3. Make any desired edits to the description, instructions, or initial prompts
5. Click **Save Recipe**

:::note
When you modify and save a recipe with a new name, a new recipe and new link are generated. You can still run the original recipe from the recipe library, or using the original link. If you edit a recipe without changing its name, the version in the recipe library is updated, but you can still run the original recipe via link.
:::

**Import Recipe:**

You can import a [shared recipe](/docs/guides/recipes/session-recipes#share-recipe) into your Recipe Library:
1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
2. Click `Recipes`
3. Click **Import Recipe** and paste in the recipe URL
4. Add a name and choose the [storage location](#recipe-storage-locations)
5. Click **Import Recipe**

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

    When you [create a recipe](/docs/guides/recipes/recipe-reference), it gets saved to:

    * Your working directory by default: `./recipe.yaml`
    * Any path you specify: `/recipe /path/to/my-recipe.yaml`  
    * Local project recipes: `/recipe .goose/recipes/my-recipe.yaml`

  </TabItem>
</Tabs>


## Finding Your Recipes

<Tabs groupId="interface">
  <TabItem value="desktop" label="Goose Desktop" default>

**Access Recipe Library:**
1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
2. Click `Recipes`
3. Browse the list of your saved recipes  
4. Each recipe shows its title, description, and whether it's global or local

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

To find and configure your saved recipes:

**Browse recipe directories:**
```bash
# List recipes in default global location
ls ~/.config/goose/recipes/

# List recipes in current project
ls .goose/recipes/

# Search for all recipe files
find . -name "*.md" -path "*/recipes/*"
```

:::tip
Set up [custom recipe paths](/docs/guides/recipes/session-recipes#configure-recipe-location) to organize recipes in specific directories or access recipes from a shared GitHub repository.
:::

  </TabItem>
</Tabs>

## Using Saved Recipes

<Tabs groupId="interface">
  <TabItem value="desktop" label="Goose Desktop" default>

1. Click the <PanelLeft className="inline" size={16} /> button in the top-left to open the sidebar
2. Click `Recipes`
3. Find your recipe in the Recipe Library
4. Choose one of the following:
   - Click `Use` to run it immediately
   - Click `Preview` to see the recipe details first, then click **Load Recipe** to run it

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

Once you've located your recipe file, [run the recipe](/docs/guides/recipes/session-recipes#run-a-recipe).

:::tip Format Compatibility
The CLI can run recipes saved from Goose Desktop without any conversion. Both CLI-created and Desktop-saved recipes work with all recipe commands.
:::

  </TabItem>
</Tabs>
