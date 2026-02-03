/**
 * Add Command - Create a new task
 */

import { ArgumentsCamelCase } from 'yargs';
import { AddCommandArgs, Priority, CreateTaskInput } from '../types';
import { TaskRepository } from '../db/repository';
import { validateAddInput, validatePriority } from '../utils/validation';
import { formatSuccess, formatError, formatTask } from '../utils/formatter';

export const command = 'add';
export const describe = 'Add a new task';
export const builder = {
  title: {
    alias: 't',
    describe: 'Task title (required)',
    type: 'string',
    demandOption: true,
  },
  description: {
    alias: 'd',
    describe: 'Task description',
    type: 'string',
  },
  priority: {
    alias: 'p',
    describe: 'Task priority (low, medium, high)',
    type: 'string',
    default: 'medium',
  },
  due: {
    describe: 'Due date (YYYY-MM-DD format)',
    type: 'string',
  },
};

export async function handler(argv: ArgumentsCamelCase<AddCommandArgs>): Promise<void> {
  // 1. Validate all inputs using validateAddInput
  const validation = validateAddInput({
    title: argv.title,
    priority: argv.priority,
    due: argv.due,
  });

  // 2. If validation fails, print errors and exit with code 1
  if (!validation.valid) {
    for (const error of validation.errors) {
      console.log(formatError(error));
    }
    process.exit(1);
  }

  // Get normalized priority value
  const priorityResult = validatePriority(argv.priority);
  const priority = priorityResult.normalizedValue ?? Priority.MEDIUM;

  // 3. Create the task via TaskRepository
  const repo = new TaskRepository();

  try {
    const input: CreateTaskInput = {
      title: argv.title,
      description: argv.description,
      priority,
      dueDate: argv.due,
    };

    const task = repo.create(input);

    // 4. Print success message with the created task
    console.log(formatSuccess('Task created successfully!'));
    console.log(formatTask(task));
  } catch (error) {
    // 5. Handle any database errors gracefully
    const message = error instanceof Error ? error.message : 'Unknown error';
    console.log(formatError(message));
    process.exit(1);
  } finally {
    repo.close();
  }
}
