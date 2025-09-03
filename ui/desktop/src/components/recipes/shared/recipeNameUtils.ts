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

/**
 * Handle real-time input transformation for recipe name fields
 * This allows users to type normally (including spaces) and transforms the input in real-time
 */
export function handleRecipeNameInput(value: string, onChange: (value: string) => void): void {
  // First, let's be more permissive and allow the space to be typed
  // Then transform it step by step
  let transformed = value;

  // Convert to lowercase
  transformed = transformed.toLowerCase();

  // Replace spaces with dashes (this should happen immediately when space is typed)
  transformed = transformed.replace(/\s/g, '-');

  // Remove invalid characters (but keep letters, numbers, and dashes)
  transformed = transformed.replace(/[^a-z0-9-]/g, '');

  // Clean up multiple dashes
  transformed = transformed.replace(/-+/g, '-');

  // Remove leading/trailing dashes
  transformed = transformed.replace(/^-+|-+$/g, '');

  onChange(transformed);
}
