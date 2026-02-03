/**
 * List command - Display tasks with optional filters
 */

import { ArgumentsCamelCase } from 'yargs';
import { ListCommandArgs, Priority } from '../types';
import { TaskRepository } from '../db/repository';
import { formatTaskList, formatInfo } from '../utils/formatter';

export const command = 'list';
export const describe = 'List all tasks';
export const builder = {
  priority: {
    alias: 'p',
    describe: 'Filter by priority (low, medium, high)',
    type: 'string',
  },
  completed: {
    alias: 'c',
    describe: 'Show only completed tasks',
    type: 'boolean',
  },
  pending: {
    describe: 'Show only pending tasks',
    type: 'boolean',
  },
  overdue: {
    describe: 'Show only overdue tasks',
    type: 'boolean',
  },
};

/**
 * Validate and convert priority string to Priority enum (case-insensitive)
 * @param priorityStr - Priority string from CLI args
 * @returns Priority enum value or null if invalid
 */
function validatePriority(priorityStr: string): Priority | null {
  const normalized = priorityStr.toLowerCase();
  if (normalized === 'low') return Priority.LOW;
  if (normalized === 'medium') return Priority.MEDIUM;
  if (normalized === 'high') return Priority.HIGH;
  return null;
}

export async function handler(argv: ArgumentsCamelCase<ListCommandArgs>): Promise<void> {
  const repository = new TaskRepository();

  try {
    // Build filters object from argv
    const filters: {
      priority?: Priority;
      completed?: boolean;
      pending?: boolean;
      overdue?: boolean;
    } = {};

    // Validate and set priority filter if provided
    if (argv.priority) {
      const validatedPriority = validatePriority(argv.priority);
      if (!validatedPriority) {
        console.log(formatInfo(`Invalid priority "${argv.priority}". Use: low, medium, or high`));
        return;
      }
      filters.priority = validatedPriority;
    }

    // Set completed filter
    if (argv.completed === true) {
      filters.completed = true;
    }

    // Set pending filter
    if (argv.pending === true) {
      filters.pending = true;
    }

    // Set overdue filter
    if (argv.overdue === true) {
      filters.overdue = true;
    }

    // Fetch tasks with filters
    const tasks = repository.findAll(filters);

    // Display results
    if (tasks.length === 0) {
      console.log(formatInfo('No tasks found'));
    } else {
      console.log(formatTaskList(tasks));
    }
  } finally {
    repository.close();
  }
}
