---
title: Recipe Library
sidebar_position: 15
sidebar_label: Recipe Library
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

The Recipe Library is where you save and find your recipes. It provides different experiences depending on whether you're using Goose Desktop or CLI.

:::info Desktop UI vs CLI
- **Goose Desktop** has a visual Recipe Library for browsing and managing saved recipes
- **Goose CLI** stores recipes as files that you find using file paths or environment variables
:::

## Storing Recipes

<Tabs groupId="interface">
  <TabItem value="desktop" label="Goose Desktop" default>

### Creating Recipes from Chat Sessions
1. To create a recipe from your chat session, see: [Create Recipe from Session](/docs/guides/recipes/session-recipes#create-recipe)
2. Once in the Recipe Editor, click **"Save Recipe"** to save it to your Recipe Library

### From an Active Recipe Session
If you're already using a recipe and want to save a modified version:
1. Click the **"⚙️"** (settings) button in the top right
2. Click **"Save recipe"**
3. Enter a name for the recipe
4. [Choose to save globally or locally](#recipe-storage-locations) to your current project
5. Click **"Save Recipe"**

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

When you [create a recipe](/docs/guides/recipes/recipe-reference), it gets saved to:

    - `./recipe.yaml` by default (current directory)
    - Any path you specify: `/recipe /path/to/my-recipe.yaml`
    - Local project recipes: `/recipe .goose/recipes/my-recipe.yaml`
  </TabItem>
</Tabs>

:::tip
See [Recipe Storage Locations](#recipe-storage-locations) for more details about global vs. local recipe storage.
:::

## Finding and Using Saved Recipes

<Tabs groupId="interface">
  <TabItem value="desktop" label="Goose Desktop" default>

### Accessing Your Recipes
1. Click the **"⚙️"** (settings) button in the top right
2. Click **"Recipe Library"**
3. Browse your saved recipes in a list view
4. Each recipe shows its title, description, and whether it's global or local

### Running a Recipe
1. Click the **"⚙️"** (settings) button in the top right
2. Click **"Recipe Library"**
3. Find your recipe in the Recipe Library
4. Choose one of the following:
   - Click **"Use Recipe"** to run it immediately
   - Click **"Preview"** to see details first, then click **"Load Recipe"** to run it

  </TabItem>
  <TabItem value="cli" label="Goose CLI">

### Accessing Your Recipes
To find your saved recipes, you can:

**Browse recipe directories:**
```bash
# List recipes in default global location
ls ~/.config/goose/recipes/

# List recipes in current project
ls .goose/recipes/

# Search for all recipe files
find . -name "*.md" -path "*/recipes/*"
```

**Set up custom recipe paths** (optional):
```bash
# Add multiple recipe directories (Unix/Linux/macOS)
export GOOSE_RECIPE_PATH="~/.config/goose/recipes:/path/to/project/recipes"

# Add multiple recipe directories (Windows)
set GOOSE_RECIPE_PATH="C:\Users\%USERNAME%\.config\goose\recipes;C:\path\to\project\recipes"
```

### Running a Recipe
Once you know where your recipes are, run them with:

```bash
# Run by recipe name (Goose searches for it automatically)
goose run --recipe my-recipe

# Run from specific file path
goose run --recipe ./recipes/my-recipe.md
goose run --recipe ~/.config/goose/recipes/my-recipe.md
```

Goose searches for recipes in this order:
1. Current directory (`.`)
2. Paths in `GOOSE_RECIPE_PATH` environment variable

  </TabItem>
</Tabs>

## Recipe Storage Locations

| Type | Location | Availability | Best For |
|------|----------|-------------|----------|
| **Global Recipes** | `~/.config/goose/recipes/` | All projects and sessions | Personal workflows, general-purpose recipes |
| **Local Recipes** | `.goose/recipes/` (in project directory) | Only when working in that project | Project-specific workflows, team recipes |

## Common Issues

### Recipe Not Found (CLI)
```bash
# Check your recipe path
echo $GOOSE_RECIPE_PATH

# Verify the recipe file exists
ls -la ~/.config/goose/recipes/my-recipe.md

# Try running from the recipe directory
cd ~/.config/goose/recipes && goose run --recipe my-recipe
```

### Can't See Recipe in Desktop Library
- Check if you saved it globally or locally
- Use the filter options to switch between global and local recipes
- Try refreshing the library view
