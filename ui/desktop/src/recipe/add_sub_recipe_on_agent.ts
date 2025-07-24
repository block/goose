import { SubRecipe } from '../api';
import { getApiUrl, getSecretKey } from '../config';

export async function addSubRecipes(subRecipes: SubRecipe[]) {
  const add_sub_recipe_response = await fetch(getApiUrl('/agent/add_sub_recipes'), {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Secret-Key': getSecretKey(),
    },
    body: JSON.stringify({
      sub_recipes: subRecipes,
    }),
  });
  if (!add_sub_recipe_response.ok) {
    console.warn(`Failed to add sub recipes: ${add_sub_recipe_response.statusText}`);
  } else {
    console.log('Added sub recipes');
  }
}
