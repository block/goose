/**
 * Delete Command - Delete a task by ID
 */

import { ArgumentsCamelCase } from 'yargs';
import { DeleteCommandArgs } from '../types';
import { TaskRepository } from '../db/repository';
import { validateId } from '../utils/validation';
import { formatSuccess, formatError, formatWarning } from '../utils/formatter';

export const command = 'delete <id>';
export const describe = 'Delete a task';
export const builder = {
  id: {
    describe: 'Task ID to delete',
    type: 'number',
    demandOption: true,
  },
  force: {
    alias: 'f',
    describe: 'Skip confirmation prompt',
    type: 'boolean',
    default: false,
  },
};

export async function handler(argv: ArgumentsCamelCase<DeleteCommandArgs>): Promise<void> {
  // 1. Validate ID using validateId
  const idValidation = validateId(argv.id);

  // 2. If invalid, print error and exit with code 1
  if (!idValidation.valid) {
    console.log(formatError(idValidation.error ?? 'Invalid task ID'));
    process.exit(1);
  }

  const taskId = idValidation.normalizedValue!;
  const repo = new TaskRepository();

  try {
    // 3. Check if task exists using findById
    const task = repo.findById(taskId);

    // 4. If not found, print "Task with ID X not found" and exit with code 1
    if (!task) {
      console.log(formatError(`Task with ID ${taskId} not found`));
      process.exit(1);
    }

    // 5. If --force flag not set, show confirmation prompt
    if (!argv.force) {
      console.log(formatWarning(`Are you sure you want to delete "${task.title}"? Use --force to confirm.`));
      process.exit(0);
    }

    // 6. Delete the task using repository.delete
    const deleted = repo.delete(taskId);

    if (deleted) {
      // 7. Print success message
      console.log(formatSuccess(`Task "${task.title}" deleted successfully`));
    } else {
      console.log(formatError(`Failed to delete task with ID ${taskId}`));
      process.exit(1);
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    console.log(formatError(message));
    process.exit(1);
  } finally {
    repo.close();
  }
}
