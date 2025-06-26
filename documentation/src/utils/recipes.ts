import type { Recipe } from "@site/src/components/recipe-card";

// Webpack context loader for all JSON files in the recipes folder
const recipeFiles = require.context(
  '../pages/recipes/data/recipes',
  false,
  /\.json$/
);

export function getRecipeById(id: string): Recipe | null {
    const allRecipes: Recipe[] = recipeFiles
      .keys()
      .map((key: string) => recipeFiles(key))
      .map((module: any) => module.default || module);
  
    return allRecipes.find((recipe) => recipe.id === id) || null;
}

export async function searchRecipes(query: string): Promise<Recipe[]> {
  const allRecipes: Recipe[] = recipeFiles
    .keys()
    .map((key: string) => recipeFiles(key))
    .map((module: any) => {
      const recipe = module.default || module;

      // Normalize fields for filters
      return {
        ...recipe,
        persona: recipe.persona || null,
        action: recipe.action || null,
        extensions: Array.isArray(recipe.extensions) ? recipe.extensions : [],
      };
    });

  if (query) {
    return allRecipes.filter((r) =>
        r.title.toLowerCase().includes(query.toLowerCase()) ||
        r.description.toLowerCase().includes(query.toLowerCase()) ||
        r.action?.toLowerCase().includes(query.toLowerCase()) ||
        r.activities?.some((activity) =>
            activity.toLowerCase().includes(query.toLowerCase())
        )
    );
  }

  return allRecipes;
}
