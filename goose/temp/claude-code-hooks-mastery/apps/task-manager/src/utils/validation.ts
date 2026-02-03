/**
 * Input validation utilities for the Task Manager CLI
 */

import { ValidationResult, Priority } from '../types';

// =============================================================================
// Title Validation
// =============================================================================

/**
 * Validate task title
 * - Must not be empty or only whitespace
 * - Must be between 1 and 200 characters
 */
export function validateTitle(title: string | undefined): ValidationResult {
  if (title === undefined || title === '') {
    return { valid: false, error: 'Title is required' };
  }

  if (title.trim() === '') {
    return { valid: false, error: 'Title cannot be empty' };
  }

  if (title.length > 200) {
    return { valid: false, error: 'Title must be 200 characters or less' };
  }

  return { valid: true };
}

// =============================================================================
// Priority Validation
// =============================================================================

/**
 * Validate priority
 * - Must be 'low', 'medium', or 'high' (case-insensitive)
 * - Returns normalized lowercase value in result
 */
export function validatePriority(
  priority: string | undefined
): ValidationResult & { normalizedValue?: Priority } {
  // Default to medium if not provided
  if (priority === undefined) {
    return { valid: true, normalizedValue: Priority.MEDIUM };
  }

  const normalizedPriority = priority.toLowerCase();

  if (normalizedPriority === 'low') {
    return { valid: true, normalizedValue: Priority.LOW };
  }

  if (normalizedPriority === 'medium') {
    return { valid: true, normalizedValue: Priority.MEDIUM };
  }

  if (normalizedPriority === 'high') {
    return { valid: true, normalizedValue: Priority.HIGH };
  }

  return { valid: false, error: 'Priority must be low, medium, or high' };
}

// =============================================================================
// Due Date Validation
// =============================================================================

/**
 * Validate due date
 * - Must be valid ISO format (YYYY-MM-DD)
 * - Can optionally require future date
 */
export function validateDueDate(
  date: string | undefined,
  requireFuture?: boolean
): ValidationResult {
  // Optional - valid if not provided
  if (date === undefined) {
    return { valid: true };
  }

  // Check ISO date format YYYY-MM-DD
  const isoDateRegex = /^\d{4}-\d{2}-\d{2}$/;
  if (!isoDateRegex.test(date)) {
    return { valid: false, error: 'Invalid date format. Use YYYY-MM-DD' };
  }

  // Parse and validate the date is real (e.g., not 2024-02-30)
  const parsedDate = new Date(date + 'T00:00:00');
  if (isNaN(parsedDate.getTime())) {
    return { valid: false, error: 'Invalid date format. Use YYYY-MM-DD' };
  }

  // Verify the parsed date matches the input (catches invalid dates like 2024-02-30)
  const [year, month, day] = date.split('-').map(Number);
  if (
    parsedDate.getFullYear() !== year ||
    parsedDate.getMonth() + 1 !== month ||
    parsedDate.getDate() !== day
  ) {
    return { valid: false, error: 'Invalid date format. Use YYYY-MM-DD' };
  }

  // Check if date must be in the future
  if (requireFuture) {
    const today = new Date();
    today.setHours(0, 0, 0, 0);

    if (parsedDate < today) {
      return { valid: false, error: 'Due date must be in the future' };
    }
  }

  return { valid: true };
}

// =============================================================================
// Task ID Validation
// =============================================================================

/**
 * Validate task ID
 * - Must be a positive integer
 */
export function validateId(
  id: number | string | undefined
): ValidationResult & { normalizedValue?: number } {
  if (id === undefined) {
    return { valid: false, error: 'Task ID is required' };
  }

  // Convert string to number if needed
  const numericId = typeof id === 'string' ? parseInt(id, 10) : id;

  // Check if it's a valid positive integer
  if (isNaN(numericId) || !Number.isInteger(numericId) || numericId <= 0) {
    return { valid: false, error: 'Task ID must be a positive number' };
  }

  return { valid: true, normalizedValue: numericId };
}

// =============================================================================
// Combined Validation for Add Command
// =============================================================================

/**
 * Combined validation for add command
 * Validates title, priority, and due date together
 */
export function validateAddInput(args: {
  title?: string;
  priority?: string;
  due?: string;
}): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  // Validate title
  const titleResult = validateTitle(args.title);
  if (!titleResult.valid && titleResult.error) {
    errors.push(titleResult.error);
  }

  // Validate priority
  const priorityResult = validatePriority(args.priority);
  if (!priorityResult.valid && priorityResult.error) {
    errors.push(priorityResult.error);
  }

  // Validate due date (require future date for new tasks)
  const dueDateResult = validateDueDate(args.due, true);
  if (!dueDateResult.valid && dueDateResult.error) {
    errors.push(dueDateResult.error);
  }

  return {
    valid: errors.length === 0,
    errors
  };
}
