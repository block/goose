import { addSubRecipes, SubRecipe } from '../api';

export async function addSubRecipesToAgent(subRecipes: SubRecipe[]) {
  const add_sub_recipe_response = await addSubRecipes({ body: { sub_recipes: subRecipes } });
  if (add_sub_recipe_response.error) {
    console.warn(`Failed to add sub recipes: ${add_sub_recipe_response.error}`);
  } else {
    console.log('Added sub recipes');
  }
}
