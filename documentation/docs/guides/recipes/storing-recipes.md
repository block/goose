---
title: Saving Recipes
sidebar_position: 4
sidebar_label: Saving Recipes
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

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
1. Click the gear icon ⚙️ in the top right corner
2. Click **Save recipe**
3. Enter a name for the recipe
4. [Choose to save globally or locally](#recipe-storage-locations) to your current project
5. Click **Save Recipe**

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
1. Click the gear icon ⚙️ in the top right corner
2. Click **Recipe Library**
3. Browse your saved recipes in a list view
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

1. Click the gear icon ⚙️ in the top right corner
2. Click **Recipe Library**
3. Find your recipe in the Recipe Library
4. Choose one of the following:
   - Click **Use Recipe** to run it immediately
   - Click **Preview** to see details first, then click **Load Recipe** to run it

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

Once you've located your recipe file, [run the recipe](/docs/guides/recipes/session-recipes#run-a-recipe).

  </TabItem>
</Tabs>
