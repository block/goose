import { RecipeManifestResponse } from '../../api';
import { ScheduledJob } from '../../schedule';

export type RecipeWithSchedule = RecipeManifestResponse & {
  schedule?: ScheduledJob;
};

/**
 * Merges recipes with their corresponding schedules
 * Matches schedules to recipes based on the schedule's source path
 */
export function mergeRecipesWithSchedules(
  recipes: RecipeManifestResponse[],
  schedules: ScheduledJob[]
): RecipeWithSchedule[] {
  return recipes.map((recipe) => {
    // Find schedule that matches this recipe's path
    // The schedule.source contains the recipe file path
    const schedule = schedules.find((s) => {
      // Extract filename from source path for comparison
      const sourcePath = s.source;
      const recipeId = recipe.id;
      
      // Check if the schedule source contains the recipe ID
      // or if it matches the recipe name/path
      return sourcePath.includes(recipeId) || sourcePath.includes(recipe.name);
    });

    return {
      ...recipe,
      schedule,
    };
  });
}

/**
 * Filters recipes based on schedule status
 */
export function filterRecipesByScheduleStatus(
  recipes: RecipeWithSchedule[],
  filterType: 'all' | 'scheduled' | 'unscheduled'
): RecipeWithSchedule[] {
  switch (filterType) {
    case 'scheduled':
      return recipes.filter((r) => r.schedule !== undefined);
    case 'unscheduled':
      return recipes.filter((r) => r.schedule === undefined);
    case 'all':
    default:
      return recipes;
  }
}
