import { z } from 'zod';

/**
 * Validation schema for recipe names
 */
export const recipeNameSchema = z.string().min(3, 'Recipe name must be at least 3 characters');

/**
 * Transform a string to a valid recipe name format:
 * - Convert to lowercase
 * - Replace spaces with dashes
 * - Remove invalid characters
 * - Trim whitespace and dashes
 */
export function transformToRecipeName(input: string): string {
  return input
    .toLowerCase()
    .replace(/[^a-zA-Z0-9\s-]/g, '') // Remove invalid characters
    .replace(/\s+/g, '-') // Replace spaces with dashes
    .replace(/--+/g, '-') // Replace multiple dashes with single dash
    .replace(/^-+|-+$/g, '') // Remove leading/trailing dashes
    .trim();
}

/**
 * Generate a recipe name from a title
 */
export function generateRecipeNameFromTitle(title: string): string {
  if (!title.trim()) {
    return '';
  }
  return transformToRecipeName(title);
}

/**
 * Common placeholder text for recipe name inputs
 */
export const RECIPE_NAME_PLACEHOLDER = 'my-awesome-recipe';
